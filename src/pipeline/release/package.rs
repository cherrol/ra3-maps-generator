//! Complete native package validation and XML release-path retargeting.
use crate::{Result, err, formats::map::MapFile, gameplay::scripts, pipeline::project};

pub fn validate_files(files: &[(String, Vec<u8>)]) -> Result<()> {
    load_source_map(files).map(|_| ())
}

/// Publication gate; historical import stays lenient enough to repair old maps.
pub fn validate_runtime_files(files: &[(String, Vec<u8>)]) -> Result<()> {
    load_runtime_map(files).map(|_| ())
}

pub(crate) fn load_source_map(files: &[(String, Vec<u8>)]) -> Result<MapFile> {
    validate_package(files, false)
}

pub(crate) fn load_runtime_map(files: &[(String, Vec<u8>)]) -> Result<MapFile> {
    validate_package(files, true)
}

fn validate_package(files: &[(String, Vec<u8>)], runtime: bool) -> Result<MapFile> {
    if files.iter().filter(|(n, _)| n.ends_with(".map")).count() != 1 {
        return Err(err("release requires exactly one .map"));
    }
    let data = files.iter().find(|(n, _)| n.ends_with(".map")).unwrap();
    let map = MapFile::load_bytes(&data.1)?;
    let name = data.0.strip_suffix(".map").unwrap();
    for required in [format!("{name}_art.tga"), "map.str".into()] {
        if !files.iter().any(|(n, _)| *n == required) {
            return Err(err(format!(
                "release missing required game file {required}"
            )));
        }
    }
    map.validate_release()?;
    crate::resources::validate_manifest(&map)?;
    scripts::validate(&map)?;
    scripts::validate_waypoints(&map)?;
    let hm = crate::formats::map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt =
        crate::formats::map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    crate::formats::map::objects(map.get("ObjectsList")?, &map.table)?;
    if bt
        .tiles
        .iter()
        .any(|&tile| tile as usize / 64 >= bt.textures.len())
    {
        return Err(err("release terrain references a missing texture"));
    }
    let assets = map.get("AssetList").ok();
    let xml = files.iter().find(|(n, _)| n == "map.xml");
    if assets.is_some() && xml.is_none() {
        return Err(err(
            "WorldBuilder AssetList requires its matching map.xml; keep the complete saved package",
        ));
    }
    if let Some((_, xml)) = xml {
        let xml = std::str::from_utf8(xml)?;
        let declarations = without_xml_comments(xml)?;
        if (declarations.contains("source=\"overrides.xml\"")
            || declarations.contains("source='overrides.xml'"))
            && !files.iter().any(|(n, _)| n == "overrides.xml")
        {
            return Err(err("map.xml includes missing overrides.xml"));
        }
        let atlas_count = count_xml_tags(&declarations, "TerrainTextureAtlas");
        if atlas_count > 0 && assets.is_none() {
            return Err(err(
                "WorldBuilder package has a texture atlas but no AssetList; use its complete saved package",
            ));
        }
        if xml.contains("Scripts are stored in the merged .map PlayerScriptsList.") {
            return Err(err(
                "WorldBuilder script asset declarations were removed; use its complete saved package",
            ));
        }
        if atlas_count > 0 && count_xml_tags(&declarations, "GameScriptList") == 0 {
            return Err(err(
                "WorldBuilder package is missing script asset declarations",
            ));
        }
        if let Some(assets) = assets {
            if assets.payload[..4] == 0_u32.to_le_bytes() {
                return Err(err("WorldBuilder AssetList is empty"));
            }
            if atlas_count != 1 {
                return Err(err(
                    "WorldBuilder package requires exactly one terrain texture atlas",
                ));
            }
            crate::resources::validate_terrain_atlas_order(&declarations)?;
            if count_xml_tags(&declarations, "Tile") != bt.textures.len() {
                return Err(err(
                    "terrain atlas tile count differs from the map texture table",
                ));
            }
            for name in &bt.textures {
                let id = crate::resources::fast_hash(name);
                if !declarations.contains(&format!("TextureID=\"{id}\""))
                    && !declarations.contains(&format!("TextureID='{id}'"))
                {
                    return Err(err(format!(
                        "terrain atlas is missing texture {name} (ID {id})"
                    )));
                }
            }
            let lists =
                scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?.len();
            let declared = count_xml_tags(&declarations, "GameScriptList");
            if declared != lists {
                return Err(err(format!(
                    "WorldBuilder script asset declarations differ: map has {lists} lists, map.xml declares {declared}"
                )));
            }
        }
        if runtime {
            let version = name.char_indices().rev().find_map(|(pos, ch)| {
                ((ch == 'T' || ch == 'V') && project::parse_version(&name[pos..]).is_ok())
                    .then_some(&name[pos..])
            });
            crate::resources::validate_terrain_package(&map, &declarations, version)?;
        }
    }
    Ok(map)
}

// Count declarations in the exported XML, excluding comments and similarly
// named elements (GameScriptListReference is not a GameScriptList).
fn count_xml_tags(xml: &str, name: &str) -> usize {
    xml.split('<')
        .skip(1)
        .filter(|part| {
            let tag = part
                .split(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
                .next()
                .unwrap_or("");
            tag.rsplit(':').next() == Some(name)
        })
        .count()
}

fn without_xml_comments(mut xml: &str) -> Result<String> {
    let mut out = String::with_capacity(xml.len());
    while let Some(start) = xml.find("<!--") {
        out.push_str(&xml[..start]);
        let rest = &xml[start + 4..];
        let end = rest
            .find("-->")
            .ok_or_else(|| err("unterminated map.xml comment"))?;
        xml = &rest[end + 3..];
    }
    out.push_str(xml);
    Ok(out)
}

/// Retarget file paths only. The terrain finalizer separately refreshes GameMap
/// and atlas version IDs; object/script resource identities remain untouched.
pub fn retarget_xml(xml: &str, old_version: &str, version: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = xml;
    let old = old_version.to_ascii_lowercase();
    let replacement = version.to_ascii_lowercase();
    while let Some(pos) = rest.find("FileName=\"") {
        let end_prefix = pos + "FileName=\"".len();
        out.push_str(&rest[..end_prefix]);
        rest = &rest[end_prefix..];
        let end = rest
            .find('"')
            .ok_or_else(|| err("unterminated XML FileName"))?;
        let value = &rest[..end];
        let lower = value.to_ascii_lowercase();
        let mut cursor = 0;
        while let Some(offset) = lower[cursor..].find(&old) {
            let start = cursor + offset;
            out.push_str(&value[cursor..start]);
            out.push_str(&replacement);
            cursor = start + old.len();
        }
        out.push_str(&value[cursor..]);
        rest = &rest[end..];
    }
    out.push_str(rest);
    Ok(out)
}
