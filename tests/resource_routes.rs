use ra3maps::{
    formats::map::{self, MapFile},
    pipeline::project::{self, Project},
    terrain::resource_routes,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn current_map() -> (Project, MapFile) {
    let p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "陕西").unwrap();
    let map = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    (p, map)
}

fn rebase_with_policy(p: &mut Project, root: &Path) {
    let sidecar = p.cfg["COORDINATE_PIPELINE"]["policy"]
        .as_str()
        .map(|path| (path.to_owned(), fs::read(p.path(path).unwrap()).unwrap()));
    p.root = root.into();
    if let Some((path, bytes)) = sidecar {
        let target = p.path(&path).unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
}

#[test]
fn common_rule_requires_new_plans_but_preserves_unchanged_frozen_releases() {
    let (mut p, map) = current_map();
    p.cfg
        .as_object_mut()
        .unwrap()
        .remove("RESOURCE_DISTRIBUTION");
    assert!(resource_routes::validate(&p, &map).unwrap().is_none());
    assert!(
        resource_routes::validate_transition(&p, &map, &map)
            .unwrap()
            .is_none()
    );
    let mut changed = map.clone();
    let mut objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    let mine = objects
        .iter()
        .position(|o| o.type_name == "OreNode")
        .unwrap();
    objects[mine].angle += 0.1;
    changed.get_mut("ObjectsList").unwrap().payload =
        map::write_objects(&objects, &changed).unwrap();
    assert!(
        resource_routes::validate_transition(&p, &map, &changed)
            .unwrap()
            .is_none()
    );
    objects[mine].position[0] += 1.;
    changed.get_mut("ObjectsList").unwrap().payload =
        map::write_objects(&objects, &changed).unwrap();
    assert!(
        resource_routes::validate_transition(&p, &map, &changed)
            .unwrap_err()
            .to_string()
            .contains("RESOURCE_DISTRIBUTION")
    );
    let mut starts = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    let imported = starts
        .iter_mut()
        .find(|o| {
            map::prop(&o.props, "waypointName").and_then(map::Value::text)
                == Some("Player_7_Start (imported)")
        })
        .unwrap();
    imported.position[1] += 1.;
    changed.get_mut("ObjectsList").unwrap().payload =
        map::write_objects(&starts, &changed).unwrap();
    assert!(
        resource_routes::validate_transition(&p, &map, &changed)
            .unwrap_err()
            .to_string()
            .contains("RESOURCE_DISTRIBUTION")
    );
    p.cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("RESOURCE_DISTRIBUTION")
    );
    p.cfg["TERRAIN_ONLY"] = json!(true);
    assert!(resource_routes::validate(&p, &map).unwrap().is_none());
}

#[test]
fn common_allocation_uses_the_effective_spawn_near_ore_radius() {
    let (mut p, map) = current_map();
    p.cfg["SPAWN_PLACEMENT"] = json!({"near_ore_radius":90.});
    assert_eq!(
        resource_routes::binding(&p).unwrap()["near_ore_radius"],
        90.
    );
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("third ore")
    );
}

#[test]
fn common_defaults_generic_route_sides_and_policy_changes_are_enforced() {
    let (mut p, map) = current_map();
    let mut plan: Value =
        serde_json::from_slice(&resource_routes::input(&p).unwrap().unwrap()).unwrap();
    for key in ["home_radius", "home_maximum", "mine_count"] {
        plan.as_object_mut().unwrap().remove(key);
    }
    for route in plan["routes"].as_array_mut().unwrap() {
        for (old, new) in [("north_seat", "side_a_seat"), ("south_seat", "side_b_seat")] {
            let v = route.as_object_mut().unwrap().remove(old).unwrap();
            route[new] = v;
        }
    }
    let root = std::env::temp_dir().join(format!("ra3-common-ore-defaults-{}", std::process::id()));
    let mut common: Value = serde_json::from_slice(
        &fs::read(p.root.join("templates/rules/coordinate-pipeline.json")).unwrap(),
    )
    .unwrap();
    rebase_with_policy(&mut p, &root);
    p.cfg["RESOURCE_DISTRIBUTION"] = json!("plan.json");
    let path = p.path("plan.json").unwrap();
    let public = root.join("templates/rules/coordinate-pipeline.json");
    project::write_json(&path, &plan).unwrap();
    project::write_json(&public, &common).unwrap();
    let report = resource_routes::validate(&p, &map).unwrap().unwrap();
    assert_eq!(report["mine_count"], 48);
    assert_eq!(report["home_counts"], json!([3, 3, 3, 3, 3, 3, 3, 3]));
    let binding = resource_routes::binding(&p).unwrap();
    common["resource_distribution"]["main_route_max_distance"] = json!(1.);
    project::write_json(&public, &common).unwrap();
    assert_ne!(binding, resource_routes::binding(&p).unwrap());
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("main route")
    );
    common["resource_distribution"]["main_route_max_distance"] = json!(65.);
    project::write_json(&public, &common).unwrap();
    plan["home_maximum"] = json!(4);
    project::write_json(&path, &plan).unwrap();
    assert!(resource_routes::validate(&p, &map).is_err());
    plan.as_object_mut().unwrap().remove("home_maximum");
    plan["mine_count"] = json!(24);
    project::write_json(&path, &plan).unwrap();
    assert!(resource_routes::validate(&p, &map).is_err());
    plan.as_object_mut().unwrap().remove("mine_count");
    let first = plan["slots"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["seat"] == 1 && s["ordinal"] == 1)
        .unwrap();
    let third = plan["slots"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["seat"] == 1 && s["ordinal"] == 3)
        .unwrap();
    plan["slots"][first]["ordinal"] = json!(3);
    plan["slots"][third]["ordinal"] = json!(1);
    project::write_json(&path, &plan).unwrap();
    assert!(
        resource_routes::validate(&p, &map)
            .unwrap_err()
            .to_string()
            .contains("third ore")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expansion_stages_reject_extra_home_ores_wrong_main_hotspot_and_side_placements() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = Project::load(root, "陕西").unwrap();
    let map = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    let report = resource_routes::validate(&p, &map).unwrap().unwrap();
    assert_eq!(report["home_counts"], json!([3, 3, 3, 3, 3, 3, 3, 3]));
    let plan: Value =
        serde_json::from_slice(&resource_routes::input(&p).unwrap().unwrap()).unwrap();
    let slot = |ordinal| {
        plan["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["seat"] == 1 && s["ordinal"] == ordinal)
            .unwrap()["ore_id"]
            .as_u64()
            .unwrap() as usize
            - 1
    };
    let objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    let replace = |target: usize, source: usize| {
        let mut m = map.clone();
        let mut v = objects.clone();
        v[target].position = v[source].position;
        m.get_mut("ObjectsList").unwrap().payload = map::write_objects(&v, &m).unwrap();
        m
    };
    let extra_home = replace(slot(4), slot(1));
    assert!(
        resource_routes::validate(&p, &extra_home)
            .unwrap_err()
            .to_string()
            .contains("maximum 3")
    );
    let off_main = replace(slot(4), slot(6));
    assert!(
        resource_routes::validate(&p, &off_main)
            .unwrap_err()
            .to_string()
            .contains("main route")
    );
    let back_hotspot = replace(slot(5), slot(4));
    assert!(
        resource_routes::validate(&p, &back_hotspot)
            .unwrap_err()
            .to_string()
            .contains("contested hotspot")
    );
    let off_secondary = replace(slot(6), slot(5));
    assert!(
        resource_routes::validate(&p, &off_secondary)
            .unwrap_err()
            .to_string()
            .contains("secondary route")
    );
    let mut plan = plan;
    plan["slots"][1]["ore_id"] = plan["slots"][0]["ore_id"].clone();
    let temp = std::env::temp_dir().join(format!("ra3-resource-policy-{}", std::process::id()));
    fs::create_dir_all(temp.join("maps/陕西")).unwrap();
    let mut other = p.clone();
    rebase_with_policy(&mut other, &temp);
    other.cfg["RESOURCE_DISTRIBUTION"] = json!("plan.json");
    project::write_json(&other.path("plan.json").unwrap(), &plan).unwrap();
    let error = resource_routes::validate(&other, &map)
        .unwrap_err()
        .to_string();
    fs::remove_dir_all(temp).unwrap();
    assert!(error.contains("duplicate"));
}
