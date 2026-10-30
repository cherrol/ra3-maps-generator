use ra3maps::{
    coordinates::Frame,
    formats::map::{self, MapFile},
    pipeline::project::{self, Project},
    terrain::resource_routes,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn current_map() -> (Project, MapFile) {
    let p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
    let map = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    (p, map)
}

fn isolated(tag: &str) -> (Project, MapFile, Value) {
    let (mut p, map) = current_map();
    let plan: Value =
        serde_json::from_slice(&resource_routes::input(&p).unwrap().unwrap()).unwrap();
    p.root = std::env::temp_dir().join(format!("ra3-resource-{tag}-{}", std::process::id()));
    p.cfg["RESOURCE_DISTRIBUTION"] = json!("plan.json");
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    (p, map, plan)
}

fn slot(plan: &Value, seat: usize, ordinal: usize) -> usize {
    plan["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["seat"] == seat && s["ordinal"] == ordinal)
        .unwrap()["ore_id"]
        .as_u64()
        .unwrap() as usize
        - 1
}

fn replace(map: &MapFile, target: usize, source: usize) -> MapFile {
    let mut result = map.clone();
    let mut objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    objects[target].position = objects[source].position;
    result.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &result).unwrap();
    result
}

#[test]
fn every_playable_map_requires_a_plan_even_for_unchanged_frozen_or_historical_repack() {
    let (p, map) = current_map();
    for context in ["frozen", "new", "historical"] {
        let mut candidate = p.clone();
        candidate
            .cfg
            .as_object_mut()
            .unwrap()
            .remove("RESOURCE_DISTRIBUTION");
        if context != "frozen" {
            candidate
                .cfg
                .as_object_mut()
                .unwrap()
                .remove("COORDINATE_PIPELINE");
        }
        if context == "historical" {
            candidate.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("historical-reference.json");
        }
        assert!(
            resource_routes::validate(&candidate, &map)
                .unwrap_err()
                .to_string()
                .contains("RESOURCE_DISTRIBUTION")
        );
        assert!(
            resource_routes::validate_transition(&candidate, &map, &map)
                .unwrap_err()
                .to_string()
                .contains("RESOURCE_DISTRIBUTION")
        );
        candidate.cfg["TERRAIN_ONLY"] = json!(true);
        assert!(
            resource_routes::validate(&candidate, &map)
                .unwrap()
                .is_none()
        );
        assert!(
            resource_routes::validate_transition(&candidate, &map, &map)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn public_default_is_two_home_ores_without_an_override_reason() {
    let (mut p, map, mut plan) = isolated("defaults");
    for field in [
        "home_ores_per_seat",
        "home_maximum",
        "home_radius",
        "mine_count",
        "override_reason",
    ] {
        plan.as_object_mut().unwrap().remove(field);
    }
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    let bytes = map.encode().unwrap();
    let checked = resource_routes::validate(&p, &map).unwrap().unwrap();
    assert_eq!(checked["home_ores_per_seat"], 2);
    assert_eq!(checked["home_maximum"], 2);
    assert_eq!(checked["home_counts"], json!([2, 2, 2, 2, 2, 2, 2, 2]));
    assert_eq!(checked["allocation_mode"], "current_two_home_ores");
    assert_eq!(checked["mine_count"], 48);
    assert!(checked["override_reason"].is_null());
    let binding = resource_routes::binding(&p).unwrap();
    assert_eq!(binding["schema_version"], 4);
    assert_eq!(binding["home_ores_per_seat"], 2);
    for obsolete in [
        "legacy_compatibility_source",
        "historical_reference_crc32",
        "historical_reference",
        "frozen",
    ] {
        assert!(binding.get(obsolete).is_none());
    }
    p.cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    p.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("a-reference-cannot-change-resource-rules.json");
    assert_eq!(resource_routes::binding(&p).unwrap(), binding);
    assert_eq!(
        resource_routes::validate(&p, &map).unwrap().unwrap(),
        checked
    );
    // The existing Henan explanation is still readable, but is optional.
    plan["home_ores_per_seat"] = json!(2);
    plan["override_reason"] = json!("2026-10-30 用户要求基地周边保留2矿。");
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    assert!(resource_routes::validate(&p, &map).is_ok());
    assert_ne!(resource_routes::binding(&p).unwrap(), binding);
    assert_eq!(map.encode().unwrap(), bytes);
    fs::remove_dir_all(p.root).unwrap();
}

#[test]
fn three_home_ores_are_rejected_in_every_source_context() {
    let (p, map, mut plan) = isolated("reject-three");
    let mut three = map.clone();
    let mut objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    let frame = Frame::from_project(&p);
    let layout: Value = serde_json::from_slice(
        &fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("maps/河南/authoring/layout.json"))
            .unwrap(),
    )
    .unwrap();
    for seat in 1..=8 {
        let base = &layout["base_cells"][seat - 1];
        let x = base[0].as_f64().unwrap() + if seat <= 4 { 85. } else { -85. };
        let y = base[1].as_f64().unwrap();
        let world = frame.world([x, y]);
        objects[slot(&plan, seat, 3)].position[0] = world[0] as f32;
        objects[slot(&plan, seat, 3)].position[1] = world[1] as f32;
    }
    three.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &three).unwrap();
    for context in ["frozen", "new", "historical"] {
        let mut candidate = p.clone();
        if context != "frozen" {
            candidate
                .cfg
                .as_object_mut()
                .unwrap()
                .remove("COORDINATE_PIPELINE");
        }
        if context == "historical" {
            candidate.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("immutable-reference.json");
        }
        plan.as_object_mut().unwrap().remove("home_ores_per_seat");
        plan.as_object_mut().unwrap().remove("home_maximum");
        project::write_json(&candidate.path("plan.json").unwrap(), &plan).unwrap();
        assert!(
            resource_routes::validate(&candidate, &three)
                .unwrap_err()
                .to_string()
                .contains("maximum 2")
        );
        assert!(resource_routes::validate_transition(&candidate, &three, &three).is_err());
        plan["home_ores_per_seat"] = json!(3);
        plan["override_reason"] = json!("A reason cannot reenable the removed three-ore rule.");
        project::write_json(&candidate.path("plan.json").unwrap(), &plan).unwrap();
        assert!(
            resource_routes::validate(&candidate, &map)
                .unwrap_err()
                .to_string()
                .contains("must be 2")
        );
        assert!(
            resource_routes::binding(&candidate)
                .unwrap_err()
                .to_string()
                .contains("must be 2")
        );
    }
    fs::remove_dir_all(p.root).unwrap();
}

#[test]
fn all_four_expansion_stages_enforce_home_main_hotspot_and_flank_positions() {
    let (p, map, mut plan) = isolated("stages");
    assert!(resource_routes::validate(&p, &map).is_ok());
    for ordinal in [3, 4, 5, 6] {
        let extra_home = replace(&map, slot(&plan, 1, ordinal), slot(&plan, 1, 1));
        assert!(
            resource_routes::validate_transition(&p, &map, &extra_home)
                .unwrap_err()
                .to_string()
                .contains("maximum 2")
        );
    }
    for ordinal in [3, 4] {
        let off_main = replace(&map, slot(&plan, 1, ordinal), slot(&plan, 1, 6));
        assert!(
            resource_routes::validate(&p, &off_main)
                .unwrap_err()
                .to_string()
                .contains("main route")
        );
    }
    let off_hotspot = replace(&map, slot(&plan, 1, 5), slot(&plan, 1, 4));
    assert!(
        resource_routes::validate(&p, &off_hotspot)
            .unwrap_err()
            .to_string()
            .contains("contested hotspot")
    );
    let off_flank = replace(&map, slot(&plan, 1, 6), slot(&plan, 1, 5));
    assert!(
        resource_routes::validate(&p, &off_flank)
            .unwrap_err()
            .to_string()
            .contains("secondary route")
    );
    let third = plan["slots"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["seat"] == 1 && s["ordinal"] == 3)
        .unwrap();
    let route = plan["slots"][third]
        .as_object_mut()
        .unwrap()
        .remove("route_id")
        .unwrap();
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("no route for its seat")
    );
    plan["slots"][third]["route_id"] = route;
    plan["slots"][1]["ore_id"] = plan["slots"][0]["ore_id"].clone();
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    fs::remove_dir_all(p.root).unwrap();
}

#[test]
fn effective_near_ore_radius_and_public_or_plan_changes_invalidate_bindings() {
    let (mut p, map, mut plan) = isolated("binding");
    let binding = resource_routes::binding(&p).unwrap();
    p.cfg["SPAWN_PLACEMENT"] = json!({"near_ore_radius":44.});
    assert_eq!(
        resource_routes::binding(&p).unwrap()["near_ore_radius"],
        44.
    );
    assert_ne!(resource_routes::binding(&p).unwrap(), binding);
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("opening ores")
    );
    p.cfg.as_object_mut().unwrap().remove("SPAWN_PLACEMENT");
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules/coordinate-pipeline.json");
    let mut common: Value = serde_json::from_slice(&fs::read(original).unwrap()).unwrap();
    let public = p.root.join("rules/coordinate-pipeline.json");
    project::write_json(&public, &common).unwrap();
    assert_eq!(resource_routes::binding(&p).unwrap(), binding);
    common["resource_distribution"]["main_route_max_distance"] = json!(1.);
    project::write_json(&public, &common).unwrap();
    assert_ne!(resource_routes::binding(&p).unwrap(), binding);
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("main route")
    );
    common["resource_distribution"]["main_route_max_distance"] = json!(65.);
    for field in ["home_ores_per_seat", "home_maximum"] {
        common["resource_distribution"][field] = json!(3);
        project::write_json(&public, &common).unwrap();
        assert!(resource_routes::binding(&p).is_err());
        common["resource_distribution"][field] = json!(2);
    }
    project::write_json(&public, &common).unwrap();
    plan["home_maximum"] = json!(3);
    project::write_json(&p.path("plan.json").unwrap(), &plan).unwrap();
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("home maximum")
    );
    assert_ne!(resource_routes::binding(&p).unwrap(), binding);
    fs::remove_dir_all(p.root).unwrap();
}
