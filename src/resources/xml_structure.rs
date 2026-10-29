//! Structural subset of EA's GameMap and TerrainTextureAtlas schemas.
//! Frozen packages are diagnosed without rewriting their source XML.
use crate::{
    Result, err,
    formats::xml::{Document, Element},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const NAMESPACE: &str = "uri:ea.com:eala:asset";
const GROUPS: [&str; 12] = [
    "Waypoint",
    "Structure",
    "Unit",
    "Prop",
    "Audio",
    "Unknown",
    "Road",
    "MapMetaData",
    "EnvironmentData",
    "WorldDict",
    "PostEffect",
    "MapAnalysis",
];

fn rank(name: &str) -> Option<usize> {
    GROUPS.iter().position(|group| *group == name)
}

pub fn asset_document(xml: &str) -> Result<Document<'_>> {
    let document = Document::parse(xml)?;
    let root = &document.elements[document.root];
    if root.local_name() != "AssetDeclaration" || root.namespace != NAMESPACE {
        return Err(err("map XML requires the EA AssetDeclaration namespace"));
    }
    Ok(document)
}
fn game_map(document: &Document<'_>) -> Result<usize> {
    let maps = document.elements[document.root]
        .children
        .iter()
        .copied()
        .filter(|i| document.elements[*i].local_name() == "GameMap")
        .collect::<Vec<_>>();
    if maps.len() != 1 {
        return Err(err("map XML requires exactly one GameMap"));
    }
    if document.elements[maps[0]].namespace != NAMESPACE {
        return Err(err("GameMap requires the EA asset namespace"));
    }
    Ok(maps[0])
}
fn finding(findings: &mut Vec<Value>, rule: &str, node: &Element<'_>, detail: &str) {
    findings.push(json!({"rule":rule,"element":node.local_name(),
        "id":node.attribute("id"),"byte_offset":node.range.start,"detail":detail}));
}

pub fn analyse(xml: &str) -> Value {
    let mut findings = Vec::new();
    let mut project_diagnostics = Vec::new();
    let parsed = (|| -> Result<(Document<'_>, usize)> {
        let doc = asset_document(xml)?;
        let map = game_map(&doc)?;
        Ok((doc, map))
    })();
    let (document, map) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            return json!({"schema_version":1,"passed":false,
            "scope":"EA XML structural subset; not complete XSD or engine validation",
            "findings":[{"rule":"xml_document","detail":error.to_string()}]});
        }
    };
    let mut maximum_rank = 0;
    let mut singletons = BTreeSet::new();
    let mut waypoints = BTreeSet::new();
    for &index in &document.elements[map].children {
        let child = &document.elements[index];
        let name = child.local_name();
        if child.namespace != NAMESPACE {
            finding(
                &mut findings,
                "xml_namespace",
                child,
                "GameMap child has a different namespace",
            );
        }
        if let Some(current) = rank(name) {
            if current < maximum_rank {
                finding(
                    &mut findings,
                    "game_map_order",
                    child,
                    "child precedes an earlier schema group",
                );
            }
            maximum_rank = maximum_rank.max(current);
        } else {
            finding(
                &mut findings,
                "game_map_element",
                child,
                "child is absent from the GameMap schema",
            );
        }
        if matches!(
            name,
            "MapMetaData" | "EnvironmentData" | "WorldDict" | "MapAnalysis"
        ) && !singletons.insert(name)
        {
            finding(
                &mut findings,
                "game_map_singleton",
                child,
                "duplicate singleton element",
            );
        }
        if matches!(
            name,
            "Waypoint" | "Structure" | "Unit" | "Prop" | "Audio" | "Unknown"
        ) {
            let positions = child
                .children
                .iter()
                .map(|i| &document.elements[*i])
                .filter(|node| node.local_name() == "Position" && node.namespace == NAMESPACE)
                .collect::<Vec<_>>();
            if positions.len() != 1 {
                finding(
                    &mut findings,
                    "map_object_position",
                    child,
                    "object requires exactly one Position",
                );
            } else if ["x", "y", "z"].iter().any(|key| {
                positions[0]
                    .attribute(key)
                    .and_then(|value| value.parse::<f64>().ok())
                    .is_none_or(|value| !value.is_finite())
            }) {
                finding(
                    &mut findings,
                    "map_object_position",
                    child,
                    "Position requires finite x/y/z values",
                );
            }
            if child.attribute("Angle").is_some_and(|value| {
                value
                    .parse::<f64>()
                    .ok()
                    .is_none_or(|value| !value.is_finite())
            }) {
                finding(
                    &mut findings,
                    "map_object_angle",
                    child,
                    "Angle must be finite",
                );
            }
        }
        if name == "Waypoint"
            && child
                .attribute("id")
                .is_none_or(|id| id.is_empty() || !waypoints.insert(id))
        {
            finding(
                &mut project_diagnostics,
                "waypoint_identity",
                child,
                "waypoint id is missing or duplicated",
            );
        }
    }
    for &index in &document.elements[document.root].children {
        let atlas = &document.elements[index];
        if atlas.local_name() != "TerrainTextureAtlas" {
            continue;
        }
        if atlas.namespace != NAMESPACE {
            finding(
                &mut findings,
                "terrain_atlas_namespace",
                atlas,
                "atlas requires the EA asset namespace",
            );
        }
        let mut ids = BTreeSet::new();
        for &tile in &atlas.children {
            let tile = &document.elements[tile];
            if tile.namespace != NAMESPACE || tile.local_name() != "Tile" {
                finding(
                    &mut findings,
                    "terrain_atlas_element",
                    tile,
                    "atlas requires Tile elements",
                );
                continue;
            }
            if tile
                .attribute("TextureID")
                .and_then(|id| id.parse::<u32>().ok())
                .is_none_or(|id| !ids.insert(id))
            {
                finding(
                    &mut findings,
                    "terrain_atlas_id",
                    tile,
                    "TextureID is invalid or duplicated",
                );
            }
            if ["BaseTexture", "NormalTexture"]
                .iter()
                .any(|key| tile.attribute(key).is_none_or(str::is_empty))
            {
                finding(
                    &mut findings,
                    "terrain_atlas_path",
                    tile,
                    "Tile requires BaseTexture and NormalTexture",
                );
            }
        }
    }
    json!({"schema_version":1,"passed":findings.is_empty(),
        "authority":["Schemas/AssetTypeGameMap.xsd","Schemas/AssetTypeTerrainTextureAtlas.xsd"],
        "scope":"EA XML structural subset; not complete XSD or engine validation",
        "object_group_order":GROUPS,"findings":findings,
        "project_diagnostics":project_diagnostics,
        "project_diagnostics_scope":"Waypoint uniqueness is a project native/script-linking rule, not an EA XSD uniqueness constraint"})
}

pub fn validate_generated(xml: &str) -> Result<()> {
    let report = analyse(xml);
    if report["passed"] != true {
        return Err(err(format!(
            "generated XML structure rejected: {}",
            report["findings"]
        )));
    }
    Ok(())
}

/// Only use for newly generated XML; source packages retain their original order.
pub fn canonicalize_generated(xml: &str) -> Result<String> {
    let document = asset_document(xml)?;
    let map = &document.elements[game_map(&document)?];
    let mut fragments = Vec::new();
    let mut cursor = map.content.start;
    for &index in &map.children {
        let child = &document.elements[index];
        let group =
            rank(child.local_name()).ok_or_else(|| err("unknown generated GameMap group"))?;
        fragments.push((group, &xml[cursor..child.range.end]));
        cursor = child.range.end;
    }
    fragments.sort_by_key(|(group, _)| *group);
    let mut result = xml[..map.content.start].to_owned();
    for (_, fragment) in fragments {
        result.push_str(fragment);
    }
    result.push_str(&xml[cursor..]);
    validate_generated(&result)?;
    Ok(result)
}

/// Insert newly created markers at the waypoint boundary, preserving source bytes.
pub fn insert_waypoints(xml: &str, fragment: &str) -> Result<String> {
    let document = asset_document(xml)?;
    let map = &document.elements[game_map(&document)?];
    if xml[map.range.clone()].ends_with("/>") {
        return Err(err("cannot insert markers into a self-closing GameMap"));
    }
    let end = map
        .children
        .iter()
        .map(|i| &document.elements[*i])
        .filter(|node| node.local_name() == "Waypoint")
        .map(|node| node.range.end)
        .max()
        .unwrap_or(map.content.start);
    let mut result = xml.to_owned();
    result.insert_str(end, fragment);
    asset_document(&result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: &str = "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><GameMap id='map'>";
    const END: &str = "</GameMap></AssetDeclaration>";
    #[test]
    fn groups_generated_objects_and_keeps_attributes_and_comments() {
        let xml = format!(
            "{START}<Structure ThingTemplate='test' id='house'><Position x='1' y='2' z='0'/></Structure><!--marker--><Waypoint id='wp'><Position x='3' y='4' z='0'/></Waypoint>{END}"
        );
        assert_eq!(analyse(&xml)["passed"], false);
        let ordered = canonicalize_generated(&xml).unwrap();
        assert!(ordered.find("<Waypoint").unwrap() < ordered.find("<Structure").unwrap());
        assert!(ordered.contains("<!--marker-->"));
        assert!(ordered.contains("ThingTemplate='test' id='house'"));
        assert_eq!(ordered, canonicalize_generated(&ordered).unwrap());
    }
    #[test]
    fn inserting_new_waypoints_does_not_rewrite_a_frozen_source() {
        let xml = format!(
            "{START}<Structure id='house'><Position x='1' y='2' z='0'/></Structure><MapAnalysis/>{END}"
        );
        let fragment = "<Waypoint id='new'><Position x='3' y='4' z='0'/></Waypoint>";
        let updated = insert_waypoints(&xml, fragment).unwrap();
        assert_eq!(updated.replace(fragment, ""), xml);
        validate_generated(&updated).unwrap();
    }
    #[test]
    fn validates_actual_structure_instead_of_comments_or_tag_substrings() {
        for content in [
            "<Waypoint id='w'/>",
            "<Waypoint id='w'><Position x='NaN' y='0' z='0'/></Waypoint>",
            "<MapAnalysis/><MapAnalysis/>",
            "<SomethingElse/>",
        ] {
            assert_eq!(analyse(&format!("{START}{content}{END}"))["passed"], false);
        }
        assert_eq!(
            analyse(&format!("{START}<!-- <Unknown/> -->{END}"))["passed"],
            true
        );
        assert_eq!(
            analyse("<AssetDeclaration><GameMap/></AssetDeclaration>")["passed"],
            false
        );
        let wrong_atlas = "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><GameMap/><TerrainTextureAtlas xmlns='wrong'><Tile xmlns='uri:ea.com:eala:asset' TextureID='1' BaseTexture='x' NormalTexture='y'/></TerrainTextureAtlas></AssetDeclaration>";
        assert_eq!(analyse(wrong_atlas)["passed"], false);
    }

    #[test]
    fn duplicate_editor_waypoints_are_distinguished_from_schema_structure() {
        let xml = format!(
            "{START}<Waypoint id='editor'><Position x='1' y='2' z='0'/></Waypoint><Waypoint id='editor'><Position x='3' y='4' z='0'/></Waypoint>{END}"
        );
        let report = analyse(&xml);
        assert_eq!(report["passed"], true);
        assert_eq!(report["project_diagnostics"].as_array().unwrap().len(), 1);
        assert!(report["findings"].as_array().unwrap().is_empty());
    }
}
