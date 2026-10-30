use ra3maps::{
    formats::map::{self, BlendTiles, HeightMap},
    gameplay::{gift_clearance, script_spawns, scripts},
    pipeline::project::{self, Project},
    terrain::Layout,
};
use std::{fs, path::Path};
mod support;

fn without_headings(nodes: &mut [scripts::Node]) {
    for node in nodes {
        if let Some(children) = &mut node.children {
            without_headings(children);
        }
        if node.command.as_deref() == Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION") {
            let mut body = scripts::unhex(&node.body).unwrap();
            for argument in scripts::scalar_arguments(node).unwrap() {
                if argument.kind == 17 {
                    body[argument.real_offset..argument.real_offset + 4].fill(0);
                }
            }
            node.body = scripts::hex(&body);
        }
    }
}

#[test]
fn current_spawn_relocation_preserves_conditions_identities_terrain_and_resource_objects() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = std::env::temp_dir().join(format!("ra3-spawn-relocation-{}", std::process::id()));
    let dir = root.join("maps/河南");
    fs::create_dir_all(dir.join("authoring")).unwrap();
    fs::create_dir_all(root.join("rules")).unwrap();
    for file in ["common-T0.5.1.json", "coordinate-pipeline.json"] {
        fs::copy(repo.join("rules").join(file), root.join("rules").join(file)).unwrap();
    }
    let (current, mut native) = support::current_map();
    let source = current.map_dir();
    fs::copy(
        source.join("authoring/layout.json"),
        dir.join("authoring/layout.json"),
    )
    .unwrap();
    let mut cfg = current.cfg.clone();
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    project::write_json(&dir.join("config.json"), &cfg).unwrap();
    let p = Project::load(&root, "河南").unwrap();
    let before = native.clone();
    let original = map::objects(before.get("ObjectsList").unwrap(), &before.table).unwrap();
    let layout = Layout::load(&p).unwrap();
    let report = gift_clearance::relocate_for_starts(&p, &mut native, &layout).unwrap();
    assert!(!report["changes"].as_array().unwrap().is_empty());
    let actual = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    assert_eq!(original.len(), actual.len());
    for (old, new) in original.iter().zip(&actual) {
        assert_eq!(old.type_name, new.type_name);
        assert_eq!(old.props, new.props);
        assert_eq!(old.angle, new.angle);
        if !old.type_name.contains("Waypoint") {
            assert_eq!(old, new);
        }
    }
    for chunk in &before.chunks {
        let name = &before.table[&chunk.name_idx];
        if !["ObjectsList", "PlayerScriptsList", "TriggerAreas"].contains(&name.as_str()) {
            assert_eq!(native.get(name).unwrap(), chunk, "{name}");
        }
    }
    let mut original = scripts::parse_nodes(
        &before.get("PlayerScriptsList").unwrap().payload,
        &before.table,
    )
    .unwrap();
    let mut actual_scripts = scripts::parse_nodes(
        &native.get("PlayerScriptsList").unwrap().payload,
        &native.table,
    )
    .unwrap();
    without_headings(&mut original);
    without_headings(&mut actual_scripts);
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(actual_scripts).unwrap()
    );
    let hm = HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap();
    let bt = BlendTiles::parse(native.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    script_spawns::validate(&p, &native, &hm, &bt, &actual).unwrap();
    gift_clearance::validate(&p, &native).unwrap();
    fs::remove_dir_all(root).unwrap();
}
