use ra3maps::{
    coordinates::Frame,
    formats::{
        archive,
        map::{self, BlendTiles, HeightMap},
    },
    terrain::{
        Layout,
        planning_constraints::{self, Plan, Route},
        preflight, resource_routes,
    },
};
use serde_json::json;
use std::fs;
mod support;

#[test]
fn route_direction_uses_declared_sides_and_plan_identity_not_seat_halves() {
    let route = Route {
        id: 1,
        side_a_seat: 7,
        side_b_seat: 2,
        main_path: vec![[0., 0.], [10., 0.]],
        secondary_path: vec![[0., 10.], [10., 10.]],
    };
    let q = resource_routes::Policy::default();
    assert_eq!(q.progress_band(&route, 7, 3).unwrap(), q.fourth_progress);
    assert_eq!(
        q.progress_band(&route, 2, 3).unwrap(),
        [1. - q.fourth_progress[1], 1. - q.fourth_progress[0]]
    );
    let plan:Plan=serde_json::from_value(json!({"routes":[route],"slots":[{"seat":2,"ordinal":1,"ore_id":91},{"seat":7,"ordinal":6,"ore_id":23,"route_id":1}]})).unwrap();
    assert_eq!(plan.slot_for_ore(91).unwrap().seat, 2);
    assert_eq!(
        plan.route_for_slot(plan.slot_for_ore(23).unwrap())
            .unwrap()
            .side_a_seat,
        7
    );
}

#[test]
fn configured_team_membership_can_interleave_seat_numbers() {
    let (mut p, _) = support::current_map();
    for (i, base) in p.cfg["BASES"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        base[3] = json!(if i % 2 == 0 { "odd-team" } else { "even-team" });
    }
    let plan:Plan=serde_json::from_value(json!({"routes":[
        {"id":1,"side_a_seat":7,"side_b_seat":2,"main_path":[[0,0],[1,0]],"secondary_path":[[0,1],[1,1]]},
        {"id":2,"side_a_seat":1,"side_b_seat":4,"main_path":[[0,0],[1,0]],"secondary_path":[[0,1],[1,1]]},
        {"id":3,"side_a_seat":3,"side_b_seat":6,"main_path":[[0,0],[1,0]],"secondary_path":[[0,1],[1,1]]},
        {"id":4,"side_a_seat":5,"side_b_seat":8,"main_path":[[0,0],[1,0]],"secondary_path":[[0,1],[1,1]]}
    ],"slots":[]})).unwrap();
    assert_eq!(
        planning_constraints::team_sides(&p, 8, Some(&plan)).unwrap(),
        [vec![1, 3, 5, 7], vec![2, 4, 6, 8]]
    );
}

#[test]
fn candidates_preserve_original_ids_and_sync_shuffled_declarative_ore_tables() {
    let (p, m) = support::current_map();
    let declared = Layout::load(&p).unwrap();
    let (mut layout, _) = ra3maps::coordinates::native_layout(&p, &m, &declared).unwrap();
    layout.ore_cells.reverse();
    layout.ore_angles.reverse();
    let objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
    let i = objects
        .iter()
        .position(|o| o.type_name == "OreNode")
        .unwrap();
    let frame = Frame::from_heightmap(
        &p,
        &HeightMap::parse(m.get("HeightMapData").unwrap()).unwrap(),
    )
    .unwrap();
    let old = frame.grid([objects[i].position[0] as f64, objects[i].position[1] as f64]);
    let index = layout.ore_cells.iter().position(|pt| *pt == old).unwrap();
    let cell = [old[0] + 1., old[1]];
    let angle = std::f32::consts::FRAC_PI_4;
    let original = m.encode().unwrap();
    let (native, next) =
        planning_constraints::candidate(&p, &m, &layout, i + 1, cell, angle).unwrap();
    assert_eq!(m.encode().unwrap(), original);
    assert_eq!(next.ore_cells[index], cell);
    assert_eq!(next.ore_angles[index], -(angle as f64));
    let actual = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    assert_eq!(actual[i].props, objects[i].props);
    assert_eq!(actual.len(), objects.len());
    assert_eq!(
        native.get("HeightMapData").unwrap().payload,
        m.get("HeightMapData").unwrap().payload
    );
}

#[test]
fn complete_native_reports_both_conflicting_ore_identities_without_mutating_map() {
    let (p, m) = support::current_map();
    let declared = Layout::load(&p).unwrap();
    let (layout, _) = ra3maps::coordinates::native_layout(&p, &m, &declared).unwrap();
    let objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
    let ores = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.type_name == "OreNode")
        .map(|(i, _)| i)
        .take(2)
        .collect::<Vec<_>>();
    let frame = Frame::from_heightmap(
        &p,
        &HeightMap::parse(m.get("HeightMapData").unwrap()).unwrap(),
    )
    .unwrap();
    let target = frame.grid([
        objects[ores[1]].position[0] as f64,
        objects[ores[1]].position[1] as f64,
    ]);
    let (candidate, layout) = planning_constraints::candidate(
        &p,
        &m,
        &layout,
        ores[0] + 1,
        target,
        objects[ores[0]].angle,
    )
    .unwrap();
    let bytes = candidate.encode().unwrap();
    let report = preflight::check_native(&p, &candidate, &layout).unwrap();
    assert_eq!(candidate.encode().unwrap(), bytes);
    assert_eq!(report["passed"], false);
    let conflicts = report["checks"]["spawn"]["findings"].as_array().unwrap();
    let conflict = conflicts
        .iter()
        .find(|r| {
            r["rule"] == "reservation_overlap"
                && r["objects"].as_array().is_some_and(|ids| {
                    ids.contains(&json!(ores[0] + 1)) && ids.contains(&json!(ores[1] + 1))
                })
        })
        .unwrap();
    assert!(conflict["a"]["cell"].is_array());
    assert!(conflict["b"]["cell"].is_array());
    assert!(report["checks"].get("official_placement").is_some());
    assert!(report["checks"].get("production_access").is_some());
    assert!(preflight::require_native(&report).is_err());
}

#[test]
fn frozen_scene_uses_actual_native_navigation_without_generated_cache_or_writes() {
    let (mut p, mut native) = support::current_map();
    let layout = Layout::load(&p).unwrap();
    let hm = HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap();
    let before =
        BlendTiles::parse(native.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    let (x, y) = (100usize, 100usize);
    let index = (hm.height - 1 - y) * hm.width + x;
    let byte = 4
        + hm.width * hm.height * 8
        + (index / hm.width) * hm.width.div_ceil(8)
        + (index % hm.width) / 8;
    native.get_mut("BlendTileData").unwrap().payload[byte] ^= 1 << (x % 8);
    let root = std::env::temp_dir().join(format!("ra3-frozen-planning-{}", std::process::id()));
    let dir = root.join("maps").join(&p.id);
    fs::create_dir_all(dir.join("authoring")).unwrap();
    let source_name = p.cfg["COORDINATE_PIPELINE"]["source_name"]
        .as_str()
        .unwrap()
        .to_string();
    let bytes = native.encode().unwrap();
    let package = archive::zip(&[
        (format!("{source_name}.map"), bytes.clone()),
        ("layout.json".into(), serde_json::to_vec(&layout).unwrap()),
    ])
    .unwrap();
    fs::write(dir.join("authoring/source.zip"), &package).unwrap();
    p.root = root.clone();
    p.cfg.as_object_mut().unwrap().remove("NATIVE_CELL_EDITS");
    p.assets = root.join("absent-generated-assets");
    p.cfg["COORDINATE_PIPELINE"]["source_package"] = json!("authoring/source.zip");
    p.cfg["COORDINATE_PIPELINE"]["source_crc32"] = json!(crc32fast::hash(&bytes));
    let scene = planning_constraints::native_scene(&p, &layout).unwrap();
    let after =
        BlendTiles::parse(scene.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    assert_eq!(after.impassability[index], !before.impassability[index]);
    assert_eq!(fs::read(dir.join("authoring/source.zip")).unwrap(), package);
    assert!(!p.assets.exists());
    assert!(!dir.join("coordinate-system.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_rectangular_preflight_is_read_only_and_marks_incomplete_coverage() {
    use ra3maps::formats::npy;
    use std::collections::BTreeMap;
    fn files(
        dir: &std::path::Path,
    ) -> BTreeMap<std::path::PathBuf, (Vec<u8>, std::time::SystemTime)> {
        let mut out = BTreeMap::new();
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                out.extend(files(&e.path()));
            } else {
                out.insert(
                    e.path(),
                    (
                        fs::read(e.path()).unwrap(),
                        e.metadata().unwrap().modified().unwrap(),
                    ),
                );
            }
        }
        out
    }
    let (mut p, _) = support::current_map();
    let repository = p.root.clone();
    let root = std::env::temp_dir().join(format!("ra3-readonly-preflight-{}", std::process::id()));
    p.root = root.clone();
    p.id = "RectReadOnly".into();
    p.assets = root.join("assets/RectReadOnly/Test");
    p.dist = root.join("dist/RectReadOnly/Test");
    p.cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    p.cfg
        .as_object_mut()
        .unwrap()
        .remove("RESOURCE_DISTRIBUTION");
    p.cfg.as_object_mut().unwrap().remove("NATIVE_CELL_EDITS");
    p.cfg["GRID"] = json!(64);
    p.cfg["GRID_HEIGHT"] = json!(96);
    p.cfg["BORDER"] = json!(8);
    p.cfg["TERRAIN_ONLY"] = json!(true);
    p.cfg["LAYERS"] = json!("layers.npz");
    p.cfg["LAYOUT"] = json!("layout.json");
    p.cfg["TERRAIN"]["v11_baked"] = json!(true);
    p.cfg["TERRAIN"]["base_min_flat_radius"] = json!(0);
    fs::create_dir_all(p.map_dir()).unwrap();
    fs::create_dir_all(root.join("templates")).unwrap();
    fs::copy(
        repository.join("templates/template.map"),
        root.join("templates/template.map"),
    )
    .unwrap();
    let raw = vec![5376u16; 64 * 96];
    fs::write(
        p.path("layers.npz").unwrap(),
        archive::zip(&[
            (
                "base_z.npy".into(),
                npy::encode(
                    "<u2",
                    &[96, 64],
                    &raw.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>(),
                ),
            ),
            (
                "z_scale.npy".into(),
                npy::encode("<f8", &[], &0.0390625f64.to_le_bytes()),
            ),
        ])
        .unwrap(),
    )
    .unwrap();
    ra3maps::pipeline::project::write_json(&p.path("layout.json").unwrap(),&json!({"schema_version":1,"base_cells":vec![[16,16];8],"ore_cells":[],"ore_angles":[],"objects":[]})).unwrap();
    let before = files(&root);
    let report = preflight::run(&p).unwrap();
    assert_eq!(report["frame"]["width"], 64);
    assert_eq!(report["frame"]["height"], 96);
    assert_eq!(report["terrain"]["cells"], 64 * 96);
    assert_eq!(report["coverage"]["complete"], false);
    assert_eq!(report["search"]["unsatisfiable_proven"], false);
    assert_eq!(files(&root), before);
    assert!(!p.assets.exists());
    assert!(!p.map_dir().join("coordinate-system.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn batch_ore_swap_resolves_original_ids_before_moving_any_coordinate() {
    let (p, mut native) = support::current_map();
    let frame = Frame::from_heightmap(
        &p,
        &HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap(),
    )
    .unwrap();
    // Keep the native codec/table, but construct a two-object synthetic layout.
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table)
        .unwrap()
        .into_iter()
        .filter(|o| o.type_name == "OreNode")
        .take(2)
        .collect::<Vec<_>>();
    let points = [[100., 100.], [200., 100.]];
    for (object, point) in objects.iter_mut().zip(points) {
        let world = frame.world(point);
        object.position[0] = world[0] as f32;
        object.position[1] = world[1] as f32;
    }
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    let specs = objects
        .iter()
        .zip(points)
        .map(|(o, cell)| ra3maps::terrain::ObjectSpec {
            cell,
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect::<Vec<_>>();
    // Classification order is deliberately the reverse of native object order.
    let layout: Layout = serde_json::from_value(json!({"schema_version":1,
        "base_cells":vec![[40.,40.];8],"ore_cells":[points[1],points[0]],
        "ore_angles":[-(objects[1].angle as f64),-(objects[0].angle as f64)],
        "ore_kind":[1,2],"ore_owner":[23,17],"objects":specs}))
    .unwrap();
    let before = native.encode().unwrap();
    let first_angle = std::f32::consts::FRAC_PI_4;
    let second_angle = -std::f32::consts::FRAC_PI_4;
    let edits = [(1, points[1], first_angle), (2, points[0], second_angle)];
    let (swapped, snapshot) =
        planning_constraints::candidate_batch(&p, &native, &layout, &edits).unwrap();
    assert_eq!(native.encode().unwrap(), before);
    let actual = map::objects(swapped.get("ObjectsList").unwrap(), &swapped.table).unwrap();
    for (i, point) in [points[1], points[0]].into_iter().enumerate() {
        assert_eq!(
            frame.grid([actual[i].position[0] as f64, actual[i].position[1] as f64]),
            point
        );
        assert_eq!(actual[i].props, objects[i].props);
    }
    assert_eq!(snapshot.ore_cells, vec![points[0], points[1]]);
    assert_eq!(
        snapshot.ore_angles,
        vec![-(second_angle as f64), -(first_angle as f64)]
    );
    assert_eq!(snapshot.ore_owner, layout.ore_owner);
    assert_eq!(snapshot.ore_kind, layout.ore_kind);
    assert_eq!(
        swapped.get("HeightMapData").unwrap(),
        native.get("HeightMapData").unwrap()
    );
    assert_eq!(
        swapped.get("BlendTileData").unwrap(),
        native.get("BlendTileData").unwrap()
    );
    let (reverse_order, reverse_snapshot) =
        planning_constraints::candidate_batch(&p, &native, &layout, &[edits[1], edits[0]]).unwrap();
    assert_eq!(swapped.encode().unwrap(), reverse_order.encode().unwrap());
    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        serde_json::to_value(reverse_snapshot).unwrap()
    );
    assert!(
        planning_constraints::candidate_batch(&p, &native, &layout, &[edits[0], edits[0]]).is_err()
    );
    assert_eq!(native.encode().unwrap(), before);
}

#[test]
fn batch_neutral_swaps_sync_all_mirrors_and_disambiguate_colocated_categories() {
    let (p, source) = support::current_map();
    let frame = Frame::from_heightmap(
        &p,
        &HeightMap::parse(source.get("HeightMapData").unwrap()).unwrap(),
    )
    .unwrap();
    let prototypes = map::objects(source.get("ObjectsList").unwrap(), &source.table)
        .unwrap()
        .into_iter()
        .filter(|o| o.type_name == "OreNode")
        .take(3)
        .collect::<Vec<_>>();
    let a = [100., 100.];
    let b = [200., 100.];
    for (key, kind) in [
        ("derrick_cells", "OilDerrick"),
        ("house_cells", "CivilianHouse01"),
        ("post_cells", "ObservationPostTechStructure"),
        ("garage_cells", "GarageTechStructure"),
        ("airport_cells", "AirportTechStructure"),
        ("hospital_cells", "HospitalTechStructure"),
        ("academy_cells", "VeterancyTechStructure"),
        ("shipyard_cells", "AlliedNavalYard"),
        ("bridge_cells", "BridgeSegment"),
    ] {
        let (other_key, other_kind) = if kind == "OilDerrick" {
            ("hospital_cells", "HospitalTechStructure")
        } else {
            ("derrick_cells", "OilDerrick")
        };
        let mut native = source.clone();
        let mut objects = prototypes.clone();
        for (i, object) in objects.iter_mut().enumerate() {
            object.type_name = if i < 2 { kind } else { other_kind }.into();
            native.id(&object.type_name).unwrap();
            let world = frame.world(if i == 1 { b } else { a });
            object.position[0] = world[0] as f32;
            object.position[1] = world[1] as f32;
        }
        native.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &native).unwrap();
        let specs = objects
            .iter()
            .enumerate()
            .map(|(i, o)| ra3maps::terrain::ObjectSpec {
                cell: if i == 1 { b } else { a },
                version: o.version,
                angle: o.angle,
                road_type: o.road_type,
                type_name: o.type_name.clone(),
                props: o.props.clone(),
            })
            .collect::<Vec<_>>();
        let mut value = json!({"schema_version":1,"base_cells":vec![[40.,40.];8],"ore_cells":[],"ore_angles":[],"objects":specs});
        value[key] = json!([b, a]); // The category order differs from native IDs.
        value[other_key] = json!([a]); // A different, unmoved neutral shares A.
        let layout: Layout = serde_json::from_value(value).unwrap();
        let original = native.encode().unwrap();
        let edits = [
            (1, b, std::f32::consts::FRAC_PI_4),
            (2, a, -std::f32::consts::FRAC_PI_4),
        ];
        let (swapped, next) =
            planning_constraints::candidate_batch(&p, &native, &layout, &edits).unwrap();
        let snapshot = serde_json::to_value(&next).unwrap();
        assert_eq!(snapshot[key], json!([a, b]), "{key}");
        assert_eq!(
            snapshot[other_key],
            json!([a]),
            "colocated {other_key} must not follow {key}"
        );
        assert_eq!(next.objects[2].cell, a);
        assert_eq!(next.objects[2].props, layout.objects[2].props);
        let actual = map::objects(swapped.get("ObjectsList").unwrap(), &swapped.table).unwrap();
        for i in 0..3 {
            assert_eq!(actual[i].props, objects[i].props);
        }
        assert_eq!(actual[2].position, objects[2].position);
        assert_eq!(
            swapped.get("HeightMapData").unwrap(),
            native.get("HeightMapData").unwrap()
        );
        assert_eq!(
            swapped.get("BlendTileData").unwrap(),
            native.get("BlendTileData").unwrap()
        );
        assert_eq!(native.encode().unwrap(), original);
        let (reversed, reversed_layout) =
            planning_constraints::candidate_batch(&p, &native, &layout, &[edits[1], edits[0]])
                .unwrap();
        assert_eq!(swapped.encode().unwrap(), reversed.encode().unwrap());
        assert_eq!(snapshot, serde_json::to_value(reversed_layout).unwrap());
    }
}
