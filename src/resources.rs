//! Rebuild the complete runtime resource package, rather than dropping AssetList.
//! Hash and manifest ordering are checked against the verified WorldBuilder save.
use crate::{
    Result,
    binary::{Reader, Writer},
    err,
    map::{self, Chunk, MapFile, MapObject, Value},
    project::Project,
    scripts::{self, Node},
    surface,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

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
struct Environment {
    macro_tex: String,
    cloud: String,
    environment: String,
}
fn environment(map: &MapFile) -> Result<Environment> {
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
fn objects(map: &MapFile) -> Result<Vec<MapObject>> {
    map::objects(map.get("ObjectsList")?, &map.table)
}
fn world(map: &MapFile) -> Result<Vec<map::Property>> {
    map::read_props(&mut Reader::new(&map.get("WorldInfo")?.payload), &map.table)
}
fn text<'a>(props: &'a [map::Property], key: &str, default: &'a str) -> &'a str {
    map::prop(props, key)
        .and_then(Value::text)
        .unwrap_or(default)
}
fn music(map: &MapFile) -> Result<String> {
    let props = world(map)?;
    let name = text(&props, "musicZone", "MusicPalette:MusicPalette_NotSet");
    Ok(name.rsplit(':').next().unwrap_or(name).into())
}

fn runtime_objects(map: &MapFile) -> Result<BTreeSet<String>> {
    let global: Vec<String> = serde_json::from_str(include_str!(
        "../templates/resources/stock-global-objects.json"
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
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn position(out: &mut String, p: [f32; 3]) {
    out.push_str(&format!(
        "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
        p[0], p[1], p[2]
    ));
}
pub(crate) fn player_names(map: &MapFile) -> Result<Vec<String>> {
    let c = map.get("SidesList")?;
    let mut r = Reader::new(&c.payload);
    if c.version >= 6 {
        r.u8()?;
    }
    let count = r.u32()?;
    let mut names = Vec::new();
    for _ in 0..count {
        let props = map::read_props(&mut r, &map.table)?;
        let name = text(&props, "playerName", "");
        names.push(if name.is_empty() {
            "_neutral_".into()
        } else {
            name.into()
        });
        for _ in 0..r.u32()? {
            r.s16()?;
            r.s16()?;
            r.take(16)?;
            r.take(2)?;
            r.u32()?;
            r.s16()?;
            r.take(7)?;
        }
    }
    r.finish()?;
    Ok(names)
}
fn script_xml(out: &mut String, nodes: &[Node]) -> Result<()> {
    for node in nodes {
        match node.asset.as_str() {
            "Script" | "ScriptGroup" => {
                let name = escape(&scripts::name(node)?.unwrap_or_default());
                out.push_str(&format!("<{} id=\"{name}\">", node.asset));
                script_xml(out, node.children.as_deref().unwrap_or_default())?;
                out.push_str(&format!("</{}>", node.asset));
            }
            "ScriptAction" | "ScriptActionFalse" => {
                let command = escape(
                    node.command
                        .as_deref()
                        .ok_or_else(|| err("action has no resource command"))?,
                );
                out.push_str(&format!("<Action Type=\"{command}\"><ParameterList>"));
                for (ty, value) in scripts::string_arguments(node)? {
                    if ty == 15 {
                        out.push_str(&format!("<ObjectType Object=\"{}\"/>", escape(&value)));
                    } else if ty == 61 {
                        out.push_str("<ObjectTypeList/>");
                    }
                }
                out.push_str("</ParameterList></Action>");
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn refresh_script_xml(xml: &str, map: &MapFile) -> Result<String> {
    let mut out = xml.to_owned();
    while let Some(start) = out.find("<GameScriptList ") {
        let tag_end = start
            + out[start..]
                .find('>')
                .ok_or_else(|| err("unterminated script XML"))?
            + 1;
        let end = if out[start..tag_end].trim_end().ends_with("/>") {
            tag_end
        } else {
            tag_end
                + out[tag_end..]
                    .find("</GameScriptList>")
                    .ok_or_else(|| err("unterminated script list XML"))?
                + "</GameScriptList>".len()
        };
        out.replace_range(start..end, "");
    }
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut declarations = String::new();
    for (player, node) in player_names(map)?.iter().zip(&nodes) {
        declarations.push_str(&format!(
            "<GameScriptList id=\"{}\"><ScriptSet>",
            escape(player)
        ));
        script_xml(
            &mut declarations,
            node.children.as_deref().unwrap_or_default(),
        )?;
        declarations.push_str("</ScriptSet></GameScriptList>");
    }
    let end = out
        .rfind("</AssetDeclaration>")
        .ok_or_else(|| err("missing XML asset declaration"))?;
    out.insert_str(end, &declarations);
    Ok(out)
}
pub struct Package {
    pub xml: Vec<u8>,
    pub overrides: Vec<u8>,
}

/// Resource-only repairs keep the previous release's actual terrain/layout
/// snapshot, so the next incremental edit cannot fall back to older ramps.
pub fn write_repair_artifacts(p: &Project, source_name: &str, map: &MapFile) -> Result<()> {
    let source_dir = p.root.join("assets").join(&p.group).join(source_name);
    if source_dir.join("source/terrain.npz").is_file()
        && source_dir.join("source/layout.json").is_file()
    {
        let mut source = p.clone();
        source.assets = source_dir.clone();
        let t = crate::terrain::Terrain::load(&source)?;
        let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
        if t.heightmap(p).elevations != hm.elevations {
            return Err(err(
                "source preview cache does not match repaired map terrain",
            ));
        }
        t.save(p)?;
        let bt = map::BlendTiles::parse(map.get("BlendTileData")?, t.n, t.n)?;
        let water = p.number("WATER_Z", 200.);
        let surface = crate::surface::Surface {
            chosen: crate::terrain::flip(&bt.tiles, t.n)
                .iter()
                .map(|v| v / 64)
                .collect(),
            impassable: crate::terrain::flip(&bt.impassability, t.n),
            build_block: crate::terrain::flip(&bt.buildability, t.n),
            layer_edge: t.slope.iter().map(|v| *v > 20.).collect(),
            shore: (0..t.n * t.n)
                .map(|i| t.z[i] >= water && crate::build::neighbors(i, t.n).any(|k| t.z[k] < water))
                .collect(),
            textures: Vec::new(),
        };
        crate::render::previews(&t, p, &surface)?;
        let path = p.assets.join("layout-index.json");
        let mut index: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        index["layout"] = serde_json::json!("source/layout.json");
        index["source_map"] = serde_json::json!(format!("{}.map", p.name));
        crate::project::write_json(&path, &index)?;
        for name in ["ramp-report.json", "ramps--comparison.png"] {
            if source_dir.join(name).is_file() {
                fs::copy(source_dir.join(name), p.assets.join(name))?;
            }
        }
    }
    fs::write(
        p.assets.join("README.md"),
        format!(
            "# {}\n\n基于 {} 修复完整资源封装。高度、通行与建造位图、斜坡纹理、对象及脚本保持原样，重新生成资源清单、纹理图集、脚本资产与地图 XML。\n\n[编号预览](preview--layout.png) · [编号索引](layout-index.json) · [发布核对](release-report.json)\n\n若附有斜坡报告和对比图，它们沿用上一版优化记录。游戏内效果仍需实机确认。\n",
            p.name, source_name
        ),
    )?;
    Ok(())
}

pub fn generate(map: &mut MapFile, p: &Project) -> Result<Package> {
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let props = world(map)?;
    let objects = objects(map)?;
    let env = environment(map)?;
    let mut sources: BTreeMap<String, Option<String>> = serde_json::from_str(include_str!(
        "../templates/resources/stock-object-sources.json"
    ))?;
    if let Some(extra) = p.cfg["OBJECT_RESOURCE_SOURCES"].as_object() {
        for (k, v) in extra {
            sources.insert(k.clone(), v.as_str().map(str::to_owned));
        }
    }
    let sources: BTreeMap<_, _> = sources
        .into_iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v))
        .collect();
    let mut includes = BTreeSet::new();
    for name in runtime_objects(map)? {
        let source = sources.get(&name).ok_or_else(|| {
            err(format!(
                "no registered resource source for {}; declare OBJECT_RESOURCE_SOURCES",
                name
            ))
        })?;
        if let Some(path) = source {
            includes.insert(path.clone());
        }
    }
    let resource_seed = match p.cfg.get("RESOURCE_ID_SEED") {
        Some(value) => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| err("RESOURCE_ID_SEED must be a uint32"))?,
        None => crc32fast::hash(p.id.as_bytes()),
    };
    let id = format!("ra3maps_{:08x}_{}", resource_seed, p.version);
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AssetDeclaration xmlns=\"uri:ea.com:eala:asset\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><Tags/><Includes>",
    );
    for path in ["DATA:static.xml", "DATA:global.xml", "DATA:audio.xml"] {
        xml.push_str(&format!("<Include source=\"{path}\" type=\"reference\"/>"));
    }
    let mut textures = BTreeSet::new();
    for name in [&env.environment, &env.cloud, &env.macro_tex] {
        if !name.is_empty() {
            textures.insert(name);
        }
    }
    for name in textures {
        xml.push_str(&format!(
            "<Include source=\"ART:{}.xml\" type=\"instance\"/>",
            escape(name)
        ));
    }
    xml.push_str("<Include source=\"DATA:GlobalData/roads.xml\" type=\"instance\"/>");
    for path in includes {
        xml.push_str(&format!(
            "<Include source=\"{}\" type=\"instance\"/>",
            escape(&path)
        ));
    }
    xml.push_str("<Include source=\"DATA:sounds/musicscript_unknownzone.xml\" type=\"instance\"/><Include source=\"overrides.xml\" type=\"all\"/></Includes>");
    xml.push_str(&format!("<GameMap id=\"{id}\">"));
    let mut roads = BTreeSet::new();
    for o in &objects {
        if o.road_type != 0 {
            if roads.insert(o.type_name.clone()) {
                xml.push_str(&format!("<Road id=\"{}\"/>", escape(&o.type_name)));
            }
            continue;
        }
        if o.type_name.contains("Waypoint") {
            let name = text(&o.props, "waypointName", "");
            if !name.is_empty() {
                xml.push_str(&format!("<Waypoint id=\"{}\">", escape(name)));
                position(&mut xml, o.position);
                xml.push_str("</Waypoint>");
            }
            continue;
        }
        let tag = if o.type_name.contains("Tree")
            || o.type_name.contains("Bush")
            || o.type_name.contains("Rock")
            || o.type_name.contains("Collapse")
        {
            "Prop"
        } else {
            "Structure"
        };
        let owner = text(&o.props, "originalOwner", "PlyrNeutral/teamPlyrNeutral");
        let team = owner.split('/').next().unwrap_or("PlyrNeutral");
        let name = text(&o.props, "objectName", "");
        let name = if name.is_empty() { &o.type_name } else { name };
        let health = match map::prop(&o.props, "objectInitialHealth") {
            Some(Value::Int(v)) => *v,
            _ => 100,
        };
        xml.push_str(&format!("<{tag} Angle=\"{}\" EventList=\"\" Faction=\"{}\" Health=\"{health}\" Team=\"{}\" ThingTemplate=\"{}\" id=\"{}\">",o.angle,escape(owner),escape(team),escape(&o.type_name),escape(name)));
        position(&mut xml, o.position);
        xml.push_str(&format!("</{tag}>"));
    }
    let starts: Vec<_> = objects
        .iter()
        .filter(|o| {
            text(&o.props, "waypointName", "")
                .strip_prefix("Player_")
                .and_then(|s| s.strip_suffix("_Start"))
                .is_some_and(|n| n.parse::<usize>().is_ok())
        })
        .collect();
    let multiplayer =
        matches!(map::prop(&props,"isScenarioMultiplayer"),Some(Value::Bool(v)) if *v!=0);
    xml.push_str(&format!("<MapMetaData BorderSize=\"{}\" CRC=\"0\" Description=\"{}\" DisplayName=\"{}\" FileName=\"data\\maps\\{}\\{}.map\" Height=\"{}\" Width=\"{}\" IsMultiplayer=\"{multiplayer}\" IsOfficial=\"false\" NumPlayers=\"{}\">",hm.border,escape(p.string("MAP_DESCRIPTION", &p.group)),escape(&p.name),escape(&p.name),escape(&p.name),hm.height,hm.width,starts.len()));
    xml.push_str("<StartPosition Name=\"InitialCameraPosition\"><Position x=\"0\" y=\"0\" z=\"0\"/></StartPosition>");
    for o in &objects {
        let name = text(&o.props, "waypointName", "");
        if starts.iter().any(|start| std::ptr::eq(*start, o)) {
            xml.push_str(&format!("<StartPosition Name=\"{}\">", escape(name)));
            position(&mut xml, o.position);
            xml.push_str("</StartPosition>");
        }
    }
    xml.push_str(&format!(
        "</MapMetaData><EnvironmentData Cloud=\"{}\" Environment=\"{}\" Macro=\"{}\"/><WorldDict>",
        escape(&env.cloud),
        escape(&env.environment),
        escape(&env.macro_tex)
    ));
    for prop in &props {
        let (tag, value) = match &prop.value {
            Value::Bool(v) => ("BoolProperty", (*v != 0).to_string()),
            Value::Int(v) => ("IntProperty", v.to_string()),
            Value::Float(v) => ("RealProperty", v.to_string()),
            Value::Unknown(v) => ("AssetIdProperty", v.rsplit(':').next().unwrap_or(v).into()),
            Value::Ascii(v) | Value::Unicode(v) => ("StringProperty", v.clone()),
        };
        xml.push_str(&format!(
            "<{tag} Key=\"{}\" Value=\"{}\"/>",
            escape(&prop.key),
            escape(&value)
        ));
    }
    let water_areas = Reader::new(&map.get("StandingWaterAreas")?.payload).u32()?;
    let garrisons = objects
        .iter()
        .filter(|o| {
            o.type_name.starts_with("ST_")
                || ["EI_Structure_01", "EI_Structure_02"].contains(&o.type_name.as_str())
        })
        .count();
    let tech = objects
        .iter()
        .filter(|o| {
            o.type_name == "OilDerrick"
                || (o.type_name.contains("TechStructure") && !o.type_name.contains("Collapse"))
        })
        .count();
    xml.push_str(&format!("</WorldDict><MapAnalysis CellsExamined=\"{}\" GarrisonableStructures=\"{garrisons}\" TechStructures=\"{tech}\" WaterAreas=\"{water_areas}\" WaterCells=\"{}\">",hm.width*hm.height,if water_areas>0{(hm.width-2*hm.border as usize)*(hm.height-2*hm.border as usize)}else{0}));
    for _ in 0..water_areas {
        xml.push_str("<WaterAreaType>EXTERIOR</WaterAreaType>");
    }
    xml.push_str("</MapAnalysis></GameMap>");
    let catalog: BTreeMap<_, _> = surface::textures(p)?
        .into_iter()
        .map(|r| (r[0].clone(), r))
        .collect();
    let mut tiles = Vec::new();
    if bt.textures.len() > 64 {
        return Err(err(
            "2048 terrain atlas supports at most 64 stock texture families",
        ));
    }
    for name in &bt.textures {
        let row = catalog.get(name).ok_or_else(|| {
            err(format!(
                "terrain atlas lacks resource definition for {name}"
            ))
        })?;
        tiles.push((fast_hash(name), &row[1], &row[2]));
    }
    tiles.sort_by_key(|r| r.0);
    xml.push_str(&format!(
        "<TerrainTextureAtlas AllowLossyCompression=\"true\" AtlasSize=\"2048\" id=\"{id}\">"
    ));
    for (texture_id, diffuse, normal) in tiles {
        xml.push_str(&format!("<Tile BaseTexture=\"ART:Terrain/{}.tga\" NormalTexture=\"ART:Terrain/{}.tga\" TextureID=\"{texture_id}\"/>",escape(diffuse),escape(normal)));
    }
    xml.push_str("</TerrainTextureAtlas>");
    let players = player_names(map)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    if players.len() != nodes.len() {
        return Err(err(
            "player/script list count differs during resource export",
        ));
    }
    for (player, node) in players.iter().zip(&nodes) {
        xml.push_str(&format!(
            "<GameScriptList id=\"{}\"><ScriptSet>",
            escape(player)
        ));
        script_xml(&mut xml, node.children.as_deref().unwrap_or_default())?;
        xml.push_str("</ScriptSet></GameScriptList>");
    }
    xml.push_str("</AssetDeclaration>\n");
    let payload = manifest(map)?;
    let name_idx = map.id("AssetList")?;
    let chunk = Chunk {
        name_idx,
        version: 1,
        payload,
    };
    if let Ok(c) = map.get_mut("AssetList") {
        *c = chunk;
    } else {
        map.chunks.insert(0, chunk);
    }
    Ok(Package{xml:xml.into_bytes(),overrides:b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><AssetDeclaration xmlns=\"uri:ea.com:eala:asset\"><Tags/><Includes/></AssetDeclaration>\n".to_vec()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_matches_installed_library_and_verified_terrain_ids() {
        for (name, hash) in [
            ("TSCloudMed", 0x8387f17c),
            ("EVDefault", 0xa5dc3710),
            ("MusicPalette_NotSet", 0xb5b5cf19),
            ("CC_Tree01", 0x66121f89),
            ("OilDerrick", 0xa3e28bbe),
            ("StPetersburgSidewalk03", 0xd6de96f2),
            ("Gravel_Yucatan01", 63961087),
            ("Grass_NewYork01", 3344070337),
            ("Dirt_Yucatan06", 272543819),
        ] {
            assert_eq!(fast_hash(name), hash);
            assert_eq!(fast_hash(&name.to_ascii_uppercase()), hash);
        }
    }
}
