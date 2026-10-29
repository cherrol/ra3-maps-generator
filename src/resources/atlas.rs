//! 地形图集、加载清单和版本身份的同步与校验。
use super::{escape, fast_hash, world};
use crate::{
    Result, err,
    formats::binary::Writer,
    formats::map::{self, MapFile, Value},
    pipeline::project::Project,
    terrain::surface,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn terrain_catalog(p: &Project) -> Result<BTreeMap<String, [String; 3]>> {
    let mut catalog: BTreeMap<_, _> = surface::textures(p)?
        .into_iter()
        .map(|row| (row[0].clone(), row))
        .collect();
    // Explicit polish-textures also uses the current palette on legacy recipes.
    // Definitions alone do not add unused assets to a historical map's atlas.
    for row in crate::terrain::materials::STOCK {
        catalog
            .entry(row[0].to_owned())
            .or_insert_with(|| row.map(str::to_owned));
    }
    Ok(catalog)
}

/// Extend the existing terrain atlas in place, keeping its ID, original tiles,
/// object declarations and script declarations unchanged.
fn extend_terrain_atlas(p: &Project, xml: &str, map: &MapFile) -> Result<String> {
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let catalog = terrain_catalog(p)?;
    let mut extra = String::new();
    for name in &bt.textures {
        let id = fast_hash(name);
        if xml.contains(&format!("TextureID=\"{id}\""))
            || xml.contains(&format!("TextureID='{id}'"))
        {
            continue;
        }
        let row = catalog
            .get(name)
            .ok_or_else(|| err(format!("missing terrain resource {name}")))?;
        extra.push_str(&format!("<Tile BaseTexture=\"ART:Terrain/{}.tga\" NormalTexture=\"ART:Terrain/{}.tga\" TextureID=\"{id}\"/>", escape(&row[1]), escape(&row[2])));
    }
    if xml.matches("</TerrainTextureAtlas>").count() != 1 {
        return Err(err(
            "mountain materials require exactly one source terrain atlas",
        ));
    }
    sort_terrain_atlas(&xml.replace(
        "</TerrainTextureAtlas>",
        &format!("{extra}</TerrainTextureAtlas>"),
    ))
}

// WorldBuilder emits the atlas in ascending unsigned TextureID order. Keep
// every declaration verbatim, including the atlas ID and texture filenames;
// only move Tile elements into their canonical slots.
fn atlas_tiles(xml: &str) -> Result<Vec<(std::ops::Range<usize>, u32)>> {
    let start = xml
        .find("<TerrainTextureAtlas")
        .ok_or_else(|| err("missing terrain atlas"))?;
    let end = start
        + xml[start..]
            .find("</TerrainTextureAtlas>")
            .ok_or_else(|| err("unterminated terrain atlas"))?;
    let mut cursor = start
        + xml[start..end]
            .find('>')
            .ok_or_else(|| err("unterminated terrain atlas opening tag"))?
        + 1;
    let mut tiles = Vec::new();
    while cursor < end {
        let remaining = &xml[cursor..end];
        if remaining.starts_with("<!--") {
            cursor += remaining
                .find("-->")
                .ok_or_else(|| err("unterminated atlas comment"))?
                + 3;
        } else if remaining.starts_with("<Tile")
            && remaining
                .as_bytes()
                .get(5)
                .is_some_and(u8::is_ascii_whitespace)
        {
            let stop = cursor
                + remaining
                    .find("/>")
                    .ok_or_else(|| err("unterminated atlas tile"))?
                + 2;
            let tag = &xml[cursor..stop];
            let attr = tag
                .find("TextureID=")
                .ok_or_else(|| err("atlas tile has no TextureID"))?
                + 10;
            let quote = tag
                .as_bytes()
                .get(attr)
                .copied()
                .ok_or_else(|| err("invalid TextureID"))?;
            if quote != b'\'' && quote != b'"' {
                return Err(err("unquoted TextureID"));
            }
            let value = &tag[attr + 1..];
            let value = &value[..value
                .find(quote as char)
                .ok_or_else(|| err("unterminated TextureID"))?];
            tiles.push((cursor..stop, value.parse::<u32>()?));
            cursor = stop;
        } else {
            let c = remaining.chars().next().unwrap();
            if !c.is_whitespace() {
                return Err(err("unexpected content in terrain atlas"));
            }
            cursor += c.len_utf8();
        }
    }
    let mut ids = BTreeSet::new();
    if tiles.iter().any(|(_, id)| !ids.insert(*id)) {
        return Err(err("duplicate terrain atlas TextureID"));
    }
    Ok(tiles)
}

pub(crate) fn sort_terrain_atlas(xml: &str) -> Result<String> {
    let tiles = atlas_tiles(xml)?;
    let mut ordered = tiles.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(_, id)| *id);
    let mut result = String::with_capacity(xml.len());
    let mut cursor = 0;
    for ((slot, _), (source, _)) in tiles.iter().zip(ordered) {
        result.push_str(&xml[cursor..slot.start]);
        result.push_str(&xml[source.clone()]);
        cursor = slot.end;
    }
    result.push_str(&xml[cursor..]);
    Ok(result)
}

pub(crate) fn validate_terrain_atlas_order(xml: &str) -> Result<()> {
    let tiles = atlas_tiles(xml)?;
    if tiles.windows(2).any(|pair| pair[0].1 >= pair[1].1) {
        return Err(err(
            "terrain atlas TextureIDs must be strictly ascending; repair-terrain-atlas before release",
        ));
    }
    Ok(())
}

fn attribute(tag: &str, name: &str) -> Result<std::ops::Range<usize>> {
    let b = tag.as_bytes();
    let mut pos = tag.find(char::is_whitespace).unwrap_or(tag.len());
    while pos < b.len() {
        while b.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        if b.get(pos).is_none_or(|v| *v == b'/' || *v == b'>') {
            break;
        }
        let start = pos;
        while b
            .get(pos)
            .is_some_and(|v| !v.is_ascii_whitespace() && *v != b'=')
        {
            pos += 1;
        }
        let key = &tag[start..pos];
        while b.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        if b.get(pos) != Some(&b'=') {
            return Err(err("invalid XML attribute"));
        }
        pos += 1;
        while b.get(pos).is_some_and(u8::is_ascii_whitespace) {
            pos += 1;
        }
        let quote = *b
            .get(pos)
            .ok_or_else(|| err("missing XML attribute quote"))?;
        if quote != b'\'' && quote != b'"' {
            return Err(err("unquoted XML attribute"));
        }
        pos += 1;
        let start = pos;
        while b.get(pos).is_some_and(|v| *v != quote) {
            pos += 1;
        }
        if pos == b.len() {
            return Err(err("unterminated XML attribute"));
        }
        if key == name {
            return Ok(start..pos);
        }
        pos += 1;
    }
    Err(err(format!("missing XML attribute {name}")))
}

fn texture_tokens(map: &MapFile, xml: &str) -> Result<String> {
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let atlas = atlas_tiles(xml)?
        .into_iter()
        .map(|(range, id)| (id, &xml[range]))
        .collect::<BTreeMap<_, _>>();
    let mut tokens = String::new();
    for name in &bt.textures {
        let tag = atlas
            .get(&fast_hash(name))
            .ok_or_else(|| err(format!("terrain atlas missing {name}")))?;
        for attr in ["BaseTexture", "NormalTexture"] {
            let range = attribute(tag, attr)?;
            let filename = tag[range]
                .strip_prefix("ART:Terrain/")
                .and_then(|v| v.strip_suffix(".tga"))
                .ok_or_else(|| err(format!("invalid terrain resource path for {name}")))?;
            tokens.push_str(filename);
            tokens.push(';');
        }
    }
    Ok(tokens)
}

fn world_tokens(map: &MapFile) -> Result<String> {
    let props = world(map)?;
    if props
        .iter()
        .filter(|p| p.key == "terrainTextureStrings")
        .count()
        != 1
    {
        return Err(err(
            "WorldInfo requires exactly one terrainTextureStrings loading list",
        ));
    }
    map::prop(&props, "terrainTextureStrings")
        .and_then(Value::text)
        .map(str::to_owned)
        .ok_or_else(|| err("WorldInfo has no terrainTextureStrings loading list"))
}

fn xml_world_token_range(xml: &str) -> Result<Option<std::ops::Range<usize>>> {
    let a = xml
        .find("<WorldDict>")
        .ok_or_else(|| err("missing WorldDict"))?;
    let end = a + xml[a..]
        .find("</WorldDict>")
        .ok_or_else(|| err("unterminated WorldDict"))?;
    let mut cursor = a;
    let mut found = None;
    while let Some(pos) = xml[cursor..end].find("<StringProperty") {
        let start = cursor + pos;
        let stop = start
            + xml[start..end]
                .find("/>")
                .ok_or_else(|| err("unterminated StringProperty"))?
            + 2;
        let tag = &xml[start..stop];
        if &tag[attribute(tag, "Key")?] == "terrainTextureStrings" {
            let r = attribute(tag, "Value")?;
            if found.is_some() {
                return Err(err("duplicate XML terrainTextureStrings loading list"));
            }
            found = Some(start + r.start..start + r.end);
        }
        cursor = stop;
    }
    Ok(found)
}

fn refresh_terrain_asset_ids(xml: &str, version: &str) -> Result<String> {
    let mut out = xml.to_owned();
    for tag in ["GameMap", "TerrainTextureAtlas"] {
        let start = out
            .find(&format!("<{tag}"))
            .ok_or_else(|| err(format!("missing {tag}")))?;
        let stop = start
            + out[start..]
                .find('>')
                .ok_or_else(|| err("unterminated terrain asset"))?
            + 1;
        let r = attribute(&out[start..stop], "id")?;
        let r = start + r.start..start + r.end;
        let id = &out[r.clone()];
        let prefix = id.char_indices().rev().find_map(|(pos, ch)| {
            if (ch == 'T' || ch == 'V')
                && crate::pipeline::project::parse_version(&id[pos..]).is_ok()
            {
                Some(&id[..pos])
            } else {
                None
            }
        });
        let new = match prefix {
            Some(p) => format!("{p}{version}"),
            None => format!("{id}_{version}"),
        };
        out.replace_range(r, &new);
    }
    Ok(out)
}

#[derive(Clone, Copy)]
pub(crate) enum TerrainFinalization {
    PreserveLookups,
    CompactLookups,
}

pub(crate) struct FinalizedTerrain {
    pub xml: String,
    pub compaction: Option<crate::resources::terrain_package::Report>,
}

/// The common last step after terrain edits. Stage all native changes, refresh
/// every loading declaration, then validate before replacing the caller's map.
pub(crate) fn finalize_terrain_package(
    p: &Project,
    map: &mut MapFile,
    xml: &str,
    version: &str,
    mode: TerrainFinalization,
) -> Result<FinalizedTerrain> {
    let mut staged = map.clone();
    let compaction = match mode {
        TerrainFinalization::CompactLookups => {
            Some(crate::resources::terrain_package::compact(&mut staged)?)
        }
        TerrainFinalization::PreserveLookups => None,
    };
    let xml = sync_terrain_package(p, &mut staged, xml, version)?;
    validate_terrain_package(&staged, &xml, Some(version))?;
    *map = staged;
    Ok(FinalizedTerrain { xml, compaction })
}

/// Synchronize the native preload list, XML WorldDict and atlas as one unit.
/// Updating only Tile elements leaves new rocks undeclared to the game loader.
fn sync_terrain_package(
    p: &Project,
    map: &mut MapFile,
    xml: &str,
    version: &str,
) -> Result<String> {
    let xml = extend_terrain_atlas(p, xml, map)?;
    let tokens = texture_tokens(map, &xml)?;
    let mut props = world(map)?;
    if props
        .iter()
        .filter(|p| p.key == "terrainTextureStrings")
        .count()
        > 1
    {
        return Err(err(
            "duplicate WorldInfo terrainTextureStrings loading list",
        ));
    }
    if let Some(prop) = props
        .iter_mut()
        .find(|pr| pr.key == "terrainTextureStrings")
    {
        prop.value = Value::Ascii(tokens.clone());
    } else {
        map.id("terrainTextureStrings")?;
        props.push(map::Property {
            key: "terrainTextureStrings".into(),
            value: Value::Ascii(tokens.clone()),
        });
    }
    let mut w = Writer::new();
    map::write_props(&mut w, &props, map)?;
    map.get_mut("WorldInfo")?.payload = w.data;
    let mut xml = xml;
    match xml_world_token_range(&xml)? {
        Some(r) => xml.replace_range(r, &escape(&tokens)),
        None => {
            if !xml.contains("</WorldDict>") {
                return Err(err("missing XML WorldDict"));
            }
            xml = xml.replace(
                "</WorldDict>",
                &format!(
                    "<StringProperty Key=\"terrainTextureStrings\" Value=\"{}\"/></WorldDict>",
                    escape(&tokens)
                ),
            );
        }
    }
    refresh_terrain_asset_ids(&xml, version)
}

/// Runtime checks are stricter than historical source import. Reject an
/// incomplete saved package even if the map decoder and overview accept it.
pub fn validate_terrain_package(map: &MapFile, xml: &str, version: Option<&str>) -> Result<()> {
    crate::resources::terrain_package::validate_references(map)?;
    let expected = texture_tokens(map, xml)?;
    if world_tokens(map)? != expected {
        return Err(err(
            "WorldInfo terrainTextureStrings differs from terrain atlas; synchronize the complete terrain package",
        ));
    }
    let xml_range = xml_world_token_range(xml)?
        .ok_or_else(|| err("XML WorldDict has no terrainTextureStrings loading list"))?;
    if xml[xml_range] != expected {
        return Err(err(
            "XML terrainTextureStrings differs from native terrain loading list",
        ));
    }
    if let Some(version) = version {
        for tag in ["GameMap", "TerrainTextureAtlas"] {
            let start = xml
                .find(&format!("<{tag}"))
                .ok_or_else(|| err(format!("missing {tag}")))?;
            let stop = start
                + xml[start..]
                    .find('>')
                    .ok_or_else(|| err("unterminated asset"))?
                + 1;
            let id = &xml[start..stop][attribute(&xml[start..stop], "id")?];
            if !id.ends_with(version) {
                return Err(err(format!(
                    "stale {tag} resource ID {id}; expected version {version}"
                )));
            }
        }
    }
    Ok(())
}
