//! 运行时资源识别、哈希和原生清单校验。
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{self, MapFile, MapObject, Value},
    gameplay::scripts::{self, Node},
};
use std::collections::BTreeSet;

const TEXTURE: u32 = 0x21e727da;
const MUSIC: u32 = 0x8f7dc19b;
const OBJECT: u32 = 0x942fff2d;
const ROAD: u32 = 0xd19d90c6;

/// RA3's case-insensitive ASCII FastHash, including its SAGE final avalanche.
/// Compared with MapCoreLib.FashHash and every entry in the verified manifest.
pub fn fast_hash(name: &str) -> u32 {
    let bytes: Vec<_> = name.bytes().map(|c| c.to_ascii_lowercase()).collect();
    let get16 = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]) as u32;
    let mut h = bytes.len() as u32;
    let mut offset = 0;
    for _ in 0..bytes.len() / 4 {
        h = h.wrapping_add(get16(offset));
        let temp = (get16(offset + 2) << 11) ^ h;
        h = (h << 16) ^ temp;
        h = h.wrapping_add(h >> 11);
        offset += 4;
    }
    match bytes.len() % 4 {
        3 => {
            h = h.wrapping_add(get16(offset));
            h ^= h << 16;
            h ^= (bytes[offset + 2] as u32) << 18;
            h = h.wrapping_add(h >> 11);
        }
        2 => {
            h = h.wrapping_add(get16(offset));
            h ^= h << 11;
            h = h.wrapping_add(h >> 17);
        }
        1 => {
            h = h.wrapping_add(bytes[offset] as u32);
            h ^= h << 10;
            h = h.wrapping_add(h >> 1);
        }
        _ => {}
    }
    h ^= h << 3;
    h = h.wrapping_add(h >> 5);
    h ^= h << 2;
    h = h.wrapping_add(h >> 15);
    h ^= h << 10;
    h
}
pub(super) struct Environment {
    pub(super) macro_tex: String,
    pub(super) cloud: String,
    pub(super) environment: String,
}
pub(super) fn environment(map: &MapFile) -> Result<Environment> {
    let c = map.get("EnvironmentData")?;
    let mut r = Reader::new(&c.payload);
    if c.version >= 3 {
        r.take(8)?;
    }
    if c.version < 5 {
        r.u8()?;
    }
    let macro_tex = r.s16()?;
    let cloud = r.s16()?;
    let environment = if c.version >= 4 {
        r.s16()?
    } else {
        "EVDefault".into()
    };
    Ok(Environment {
        macro_tex,
        cloud,
        environment,
    })
}
pub(super) fn objects(map: &MapFile) -> Result<Vec<MapObject>> {
    map::objects(map.get("ObjectsList")?, &map.table)
}
pub(super) fn world(map: &MapFile) -> Result<Vec<map::Property>> {
    map::read_props(&mut Reader::new(&map.get("WorldInfo")?.payload), &map.table)
}
pub(super) fn text<'a>(props: &'a [map::Property], key: &str, default: &'a str) -> &'a str {
    map::prop(props, key)
        .and_then(Value::text)
        .unwrap_or(default)
}
fn music(map: &MapFile) -> Result<String> {
    let props = world(map)?;
    let name = text(&props, "musicZone", "MusicPalette:MusicPalette_NotSet");
    Ok(name.rsplit(':').next().unwrap_or(name).into())
}

pub(super) fn runtime_objects(map: &MapFile) -> Result<BTreeSet<String>> {
    let global: Vec<String> = serde_json::from_str(include_str!(
        "../../templates/resources/stock-global-objects.json"
    ))?;
    let global: BTreeSet<_> = global.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut names: BTreeSet<_> = objects(map)?
        .into_iter()
        .filter(|o| !o.type_name.contains("Waypoint") && o.road_type == 0)
        .map(|o| o.type_name.to_ascii_lowercase())
        .collect();
    fn collect(nodes: &[Node], names: &mut BTreeSet<String>) -> Result<()> {
        for node in nodes {
            if node.asset == "ScriptAction" || node.asset == "ScriptActionFalse" {
                for (ty, value) in scripts::string_arguments(node)? {
                    if ty == 15 && !value.is_empty() {
                        names.insert(value.to_ascii_lowercase());
                    }
                }
            }
            if let Some(children) = &node.children {
                collect(children, names)?;
            }
        }
        Ok(())
    }
    collect(
        &scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?,
        &mut names,
    )?;
    names.retain(|name| !global.contains(name));
    Ok(names)
}

pub fn manifest(map: &MapFile) -> Result<Vec<u8>> {
    let mut assets = BTreeSet::new();
    let env = environment(map)?;
    for name in [&env.macro_tex, &env.cloud, &env.environment] {
        if !name.is_empty() {
            assets.insert((TEXTURE, fast_hash(name)));
        }
    }
    let music = music(map)?;
    if !music.is_empty() {
        assets.insert((MUSIC, fast_hash(&music)));
    }
    for name in runtime_objects(map)? {
        assets.insert((OBJECT, fast_hash(&name)));
    }
    for o in objects(map)? {
        if o.road_type != 0 {
            assets.insert((ROAD, fast_hash(&o.type_name)));
        }
    }
    let mut w = Writer::new();
    w.u32(u32::try_from(assets.len())?);
    for (ty, id) in assets {
        w.u32(ty);
        w.u32(id);
    }
    Ok(w.data)
}
pub fn validate_manifest(map: &MapFile) -> Result<()> {
    let c=map.get("AssetList").map_err(|_|err("missing runtime AssetList; run repair-resources or rebuild the complete resource package"))?;
    let mut r = Reader::new(&c.payload);
    let n = r.u32()?;
    let mut actual = BTreeSet::new();
    for _ in 0..n {
        actual.insert((r.u32()?, r.u32()?));
    }
    r.finish()?;
    if actual.is_empty() {
        return Err(err("WorldBuilder AssetList is empty"));
    }
    let expected = manifest(map)?;
    let mut r = Reader::new(&expected);
    let n = r.u32()?;
    for _ in 0..n {
        let key = (r.u32()?, r.u32()?);
        if !actual.contains(&key) {
            return Err(err(format!(
                "AssetList missing runtime resource {:08x}:{:08x}",
                key.0, key.1
            )));
        }
    }
    Ok(())
}
