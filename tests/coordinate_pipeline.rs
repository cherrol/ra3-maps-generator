use ra3maps::{
    formats::archive,
    formats::map::{self, MapFile},
    pipeline::build::{self, Options},
    pipeline::coordinate_pipeline as pipeline,
    pipeline::project::{self, Project},
    terrain::Layout,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
mod support;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "ra3-coordinate-tests-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let recipe = root.join("maps/陕西");
    fs::create_dir_all(recipe.join("authoring")).unwrap();
    let original = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in [
        "config.json",
        "version.json",
        "authoring/source.zip",
        "authoring/layout.json",
        "authoring/policy.json",
        "authoring/baseline.json",
    ] {
        fs::copy(original.join("maps/陕西").join(file), recipe.join(file)).unwrap();
    }
    fs::create_dir_all(root.join("templates/rules")).unwrap();
    for file in [
        "common-T0.5.1.json",
        "千壑横原.json",
        "coordinate-pipeline.json",
    ] {
        fs::copy(
            original.join("templates/rules").join(file),
            root.join("templates/rules").join(file),
        )
        .unwrap();
    }
    fs::write(root.join("Cargo.toml"), "# isolated coordinate pipeline").unwrap();
    let cfg: Value =
        serde_json::from_slice(&fs::read(recipe.join("config.json")).unwrap()).unwrap();
    support::copy_declared_inputs(&original.join("maps/陕西"), &recipe, &cfg);
    // Exercise the frozen release, independently of a newer version currently
    // being authored in the shared workspace but not yet adopted.
    let source_name = cfg["COORDINATE_PIPELINE"]["source_name"].as_str().unwrap();
    let source_version = source_name
        .char_indices()
        .rev()
        .find_map(|(i, c)| {
            ((c == 'T' || c == 'V') && project::parse_version(&source_name[i..]).is_ok())
                .then_some(&source_name[i..])
        })
        .unwrap();
    project::write_json(
        &recipe.join("version.json"),
        &project::parse_version(source_version).unwrap(),
    )
    .unwrap();
    if let Some(plan) = cfg["RESOURCE_DISTRIBUTION"].as_str() {
        let target = recipe.join(plan);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(original.join("maps/陕西").join(plan), target).unwrap();
    }
    Fixture(root)
}
fn load(root: &Fixture) -> Project {
    Project::load(&root.0, "陕西").unwrap()
}

#[test]
fn changed_expansion_policy_cannot_reuse_a_successful_spatial_cache() {
    let root = fixture();
    let p = load(&root);
    let Some(plan) = p.cfg["RESOURCE_DISTRIBUTION"].as_str() else {
        return;
    };
    pipeline::audit(&p).unwrap();
    let path = p.path(plan).unwrap();
    let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["home_maximum"] = json!(2);
    project::write_json(&path, &data).unwrap();
    assert!(
        pipeline::audit(&p)
            .unwrap_err()
            .to_string()
            .contains("maximum 2")
    );
}

#[test]
fn changed_common_ore_policy_invalidates_an_existing_spatial_cache() {
    let root = fixture();
    let p = load(&root);
    pipeline::audit(&p).unwrap();
    let path = root.0.join("templates/rules/coordinate-pipeline.json");
    let mut common: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    common["resource_distribution"]["main_route_max_distance"] = json!(1.);
    project::write_json(&path, &common).unwrap();
    assert!(
        pipeline::audit(&p)
            .unwrap_err()
            .to_string()
            .contains("main route")
    );
}
fn next_version(p: &Project) -> String {
    project::format_version(&project::bump(&p.state, false, 1, "").unwrap()).unwrap()
}
fn source_files(p: &Project) -> std::collections::BTreeMap<String, Vec<u8>> {
    archive::unzip(&fs::read(p.path("authoring/source.zip").unwrap()).unwrap()).unwrap()
}

#[test]
fn initial_adoption_checks_common_clearance_and_spawns_before_creating_a_frozen_source() {
    let root = fixture();
    let p = load(&root);
    pipeline::assemble(&p).unwrap();
    let path = p.map_dir().join("config.json");
    let mut cfg = p.cfg.clone();
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    cfg.as_object_mut().unwrap().remove("RAMP_CLEARANCE");
    project::write_json(&path, &cfg).unwrap();
    fs::remove_file(p.path("authoring/source.zip").unwrap()).unwrap();
    let fresh = load(&root);
    let before = fs::read(&path).unwrap();
    assert!(
        pipeline::adopt(&fresh)
            .unwrap_err()
            .to_string()
            .contains("spawn placement rejected")
    );
    assert!(!fresh.path("authoring/source.zip").unwrap().exists());
    assert_eq!(fs::read(&path).unwrap(), before);
    // Preserve this existing-map import regression with a fixed historical
    // expectation; initial adoption cannot silently invent a passing baseline.
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/spawn-placement-frozen-陕西.json"),
        fresh.path("spawn-placement-reference.json").unwrap(),
    )
    .unwrap();
    cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("spawn-placement-reference.json");
    project::write_json(&path, &cfg).unwrap();
    let fresh = load(&root);
    pipeline::adopt(&fresh).unwrap();
    let adopted = load(&root);
    assert!(pipeline::enabled(&adopted));
    assert!(adopted.cfg.get("RAMP_CLEARANCE").is_none());
    assert_eq!(
        adopted.cfg["COORDINATE_PIPELINE"]["source_name"],
        adopted.name
    );
    pipeline::verify(&adopted).unwrap();
}

#[test]
fn adopting_a_cache_with_missing_ramps_cannot_erase_the_source_guard() {
    let root = fixture();
    let p = load(&root);
    pipeline::assemble(&p).unwrap();
    let before = fs::read(p.path("authoring/source.zip").unwrap()).unwrap();
    let path = p.source().join("layout.json");
    let mut cached: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    cached.ramps.clear();
    project::write_json(&path, &cached).unwrap();
    assert!(
        pipeline::adopt(&p)
            .unwrap_err()
            .to_string()
            .contains("changed ramp definitions")
    );
    assert_eq!(
        before,
        fs::read(p.path("authoring/source.zip").unwrap()).unwrap()
    );
}

#[test]
fn map_only_rejects_incomplete_frozen_terrain_package_before_writing_game_files() {
    let root = fixture();
    let p = load(&root);
    let mut files = source_files(&p);
    let xml = String::from_utf8(files["map.xml"].clone()).unwrap();
    let bad = xml.replace("TRock_Yucatan03;TRock_Yucatan03_nrm;", "");
    assert_ne!(bad, xml);
    files.insert("map.xml".into(), bad.into_bytes());
    let zip = archive::zip(&files.into_iter().collect::<Vec<_>>()).unwrap();
    let source_path = p.path("authoring/source.zip").unwrap();
    fs::write(&source_path, &zip).unwrap();
    let error = pipeline::assemble(&p).unwrap_err().to_string();
    assert!(error.contains("XML terrainTextureStrings"), "{error}");
    for n in [format!("{}.map", p.name), "map.xml".into()] {
        assert!(!p.assets.join(n).exists());
    }
    assert_eq!(fs::read(source_path).unwrap(), zip);
}

#[test]
fn adopting_incomplete_terrain_package_preserves_existing_source_and_baseline() {
    let root = fixture();
    let p = load(&root);
    pipeline::assemble(&p).unwrap();
    let source = p.path("authoring/source.zip").unwrap();
    let source_bytes = fs::read(&source).unwrap();
    let baseline = p.path("authoring/baseline.json").unwrap();
    let baseline_bytes = fs::read(&baseline).unwrap();
    let config = p.map_dir().join("config.json");
    let config_bytes = fs::read(&config).unwrap();
    let path = p.assets.join("map.xml");
    let xml = fs::read_to_string(&path).unwrap();
    let bad = xml.replace("TRock_Yucatan03;TRock_Yucatan03_nrm;", "");
    assert_ne!(bad, xml);
    fs::write(&path, &bad).unwrap();
    assert!(
        pipeline::adopt(&p)
            .unwrap_err()
            .to_string()
            .contains("XML terrainTextureStrings")
    );
    assert_eq!(fs::read(source).unwrap(), source_bytes);
    assert_eq!(fs::read(baseline).unwrap(), baseline_bytes);
    assert_eq!(fs::read(config).unwrap(), config_bytes);
    assert_eq!(fs::read_to_string(path).unwrap(), bad);
}
fn stages() -> Options {
    Options {
        only: [
            "terrain",
            "map",
            "verify",
            "spatial",
            "coordinates",
            "accept",
            "dist",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        ..Options::default()
    }
}
#[test]
fn finished_source_build_is_byte_exact_and_exports_complete_routes_and_materials() {
    let root = fixture();
    let p = load(&root);
    let files = source_files(&p);
    let previous = json!({"schema_version":1,"map":"historical index","entries":[]});
    project::write_json(
        &p.path("histories/T0.0.1/coordinate-id-map.json").unwrap(),
        &previous,
    )
    .unwrap();
    project::write_json(
        &p.path("histories/v11/coordinate-id-map.json").unwrap(),
        &json!({"ignored":true}),
    )
    .unwrap();
    let version = fs::read(project::version_path(&root.0, &p.id)).unwrap();
    let built = build::run(&root.0, &p.id, &stages()).unwrap();
    assert_eq!(
        fs::read(p.path("layers.npz").unwrap()).unwrap(),
        fs::read(p.path(p.string("LAYERS", "layers.npz")).unwrap()).unwrap()
    );
    let exported_ids: Value = serde_json::from_slice(
        &fs::read(built.assets.join("coordinates/coordinate-id-map.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(exported_ids["previous_versions"], json!([previous]));
    for (n, bytes) in files.iter().filter(|(n, _)| n.as_str() != "layout.json") {
        assert_eq!(*bytes, fs::read(built.assets.join(n)).unwrap(), "{n}");
        assert_eq!(*bytes, fs::read(built.dist.join(n)).unwrap());
    }
    assert_eq!(
        version,
        fs::read(project::version_path(&root.0, &p.id)).unwrap()
    );
    let coordinate_system = fs::read(built.source().join("coordinate-system.json")).unwrap();
    assert_eq!(
        coordinate_system,
        fs::read(built.assets.join("coordinates/coordinate-system.json")).unwrap()
    );
    let archive = archive::unzip(&fs::read(built.assets.join("coordinates.zip")).unwrap()).unwrap();
    assert_eq!(
        archive["coordinates/coordinate-system.json"],
        coordinate_system
    );
    let system: Value = serde_json::from_slice(&coordinate_system).unwrap();
    assert_eq!(system["map_id"], p.id);
    assert_eq!(system["version"], p.version);
    assert_eq!(system["frame"]["width"], 840);
    assert_eq!(system["frame"]["border"], p.cfg["BORDER"]);
    assert_eq!(system["frame"]["cell_size"], p.cfg["CELL"]);
    let report: Value =
        serde_json::from_slice(&fs::read(p.assets.join("spatial/report.json")).unwrap()).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["counts"]["objects"], 544);
    assert_eq!(report["counts"]["ramps"], 27);
    assert_eq!(report["counts"]["routes"], 528);
    assert_eq!(report["counts"]["reachable_routes"], 528);
    let material: Value = serde_json::from_slice(
        &fs::read(p.assets.join("spatial/texture-coordinates.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(material["rows"].as_array().unwrap().len(), 840);
    assert_eq!(
        material["histogram"]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_u64().unwrap())
            .sum::<u64>(),
        705600
    );
    let expected = fs::read(p.assets.join("spatial/texture-coordinates.csv")).unwrap();
    fs::write(p.assets.join("spatial/texture-coordinates.csv"), b"corrupt").unwrap();
    pipeline::audit(&p).unwrap();
    assert_eq!(
        expected,
        fs::read(p.assets.join("spatial/texture-coordinates.csv")).unwrap()
    );
}
#[test]
fn new_collision_blocks_bumped_build_and_restores_exact_version() {
    let root = fixture();
    let p = load(&root);
    let version = fs::read(project::version_path(&root.0, &p.id)).unwrap();
    let path = p.path("authoring/layout.json").unwrap();
    let mut layout: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let hospital = layout.objects[206].cell;
    layout.objects[209].cell = hospital;
    project::write_json(&path, &layout).unwrap();
    let mut options = stages();
    options.bump = true;
    let error = build::run(&root.0, &p.id, &options)
        .unwrap_err()
        .to_string();
    assert!(error.contains("new/worsened"), "{error}");
    assert_eq!(
        version,
        fs::read(project::version_path(&root.0, &p.id)).unwrap()
    );
    assert!(
        !p.assets
            .parent()
            .unwrap()
            .join(p.release_name(&next_version(&p)).unwrap())
            .exists()
    );
}
#[test]
fn metadata_follows_object_id_and_stale_native_source_cannot_be_overwritten() {
    let root = fixture();
    let p = load(&root);
    pipeline::prepare(&p).unwrap();
    pipeline::assemble(&p).unwrap();
    let path = p.path("authoring/layout.json").unwrap();
    let mut layout: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    layout.objects[199].cell = [750., 470.];
    layout.objects[206].cell = [505., 531.];
    project::write_json(&path, &layout).unwrap();
    let derived = Layout::load(&p).unwrap();
    assert!(derived.ore_cells.contains(&[750., 470.]));
    assert!(derived.hospital_cells.contains(&[505., 531.]));
    let map_path = p.assets.join(format!("{}.map", p.name));
    let mut native = MapFile::load(&map_path).unwrap();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    objects[0].angle += 0.2;
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    native.save(&map_path).unwrap();
    let bytes = fs::read(&map_path).unwrap();
    assert!(
        build::run(&root.0, &p.id, &stages())
            .unwrap_err()
            .to_string()
            .contains("adopt-coordinate-source")
    );
    assert_eq!(bytes, fs::read(map_path).unwrap());
}
#[test]
fn unsupported_ramp_metadata_and_moved_external_oil_are_rejected_before_output() {
    let root = fixture();
    let p = load(&root);
    let path = p.path("authoring/layout.json").unwrap();
    let bytes = fs::read(&path).unwrap();
    let mut l: Value = serde_json::from_slice(&bytes).unwrap();
    l["ramps"][0]["x"] = json!(5);
    project::write_json(&path, &l).unwrap();
    assert!(
        pipeline::prepare(&p)
            .unwrap_err()
            .to_string()
            .contains("ramp geometry")
    );
    fs::write(&path, &bytes).unwrap();
    let mut l: Value = serde_json::from_slice(&bytes).unwrap();
    l["objects"][33]["cell"][1] = json!(900);
    project::write_json(&path, &l).unwrap();
    assert!(
        pipeline::prepare(&p)
            .unwrap_err()
            .to_string()
            .contains("out-of-grid")
    );
}

#[test]
fn safe_coordinate_edit_publishes_new_objects_and_keeps_native_terrain_and_scripts() {
    let root = fixture();
    let p = load(&root);
    let files = source_files(&p);
    let before = MapFile::load_bytes(&files[&format!("{}.map", p.name)]).unwrap();
    let path = p.path("authoring/layout.json").unwrap();
    let mut layout: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let index = layout
        .objects
        .iter()
        .position(|o| o.type_name.contains("Tree"))
        .unwrap();
    layout.objects[index].cell = [450., 200.];
    project::write_json(&path, &layout).unwrap();
    assert!(
        pipeline::assemble(&p)
            .unwrap_err()
            .to_string()
            .contains("--bump")
    );
    assert!(!p.assets.join(format!("{}.map", p.name)).exists());
    let mut options = stages();
    options.bump = true;
    let next = build::run(&root.0, &p.id, &options).unwrap();
    assert_eq!(next.version, next_version(&p));
    let after = MapFile::load(next.assets.join(format!("{}.map", next.name))).unwrap();
    for chunk in [
        "HeightMapData",
        "BlendTileData",
        "PlayerScriptsList",
        "Teams",
        "AssetList",
    ] {
        assert_eq!(
            before.get(chunk).unwrap(),
            after.get(chunk).unwrap(),
            "{chunk}"
        );
    }
    let objects = map::objects(after.get("ObjectsList").unwrap(), &after.table).unwrap();
    assert_eq!(objects.len(), 544);
    assert!((objects[index].position[0] - 4300.).abs() < 0.001);
    assert!((objects[index].position[1] - 6190.).abs() < 0.001);
    assert_ne!(
        files["map.xml"],
        fs::read(next.assets.join("map.xml")).unwrap()
    );
}

#[test]
fn common_pipeline_adopts_generated_waypoints_and_distinguishes_water_and_land_routes() {
    let root = fixture();
    let original = Path::new(env!("CARGO_MANIFEST_DIR"));
    let id = "广东";
    let dir = root.0.join("maps").join(id);
    fs::create_dir_all(&dir).unwrap();
    for f in ["config.json", "version.json"] {
        fs::copy(original.join("maps").join(id).join(f), dir.join(f)).unwrap();
    }
    let config = dir.join("config.json");
    let mut cfg: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    support::copy_declared_inputs(&original.join("maps").join(id), &dir, &cfg);
    let ruleset = cfg["RULESET"].as_str().unwrap();
    fs::copy(
        original
            .join("templates/rules")
            .join(format!("{ruleset}.json")),
        root.0
            .join("templates/rules")
            .join(format!("{ruleset}.json")),
    )
    .unwrap();
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    // This deliberately imports an unchanged historical native map. Its
    // existing approach findings are bound to an immutable geometry snapshot.
    cfg["RAMP_CLEARANCE_REFERENCE"] = json!("ramp-clearance-reference.json");
    cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("spawn-placement-reference.json");
    fs::copy(
        original.join("tests/fixtures/spawn-placement-frozen-广东.json"),
        dir.join("spawn-placement-reference.json"),
    )
    .unwrap();
    let mut reference: Value = serde_json::from_slice(
        &fs::read(original.join("tests/fixtures/ramp-clearance-frozen-千流叩海.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(reference["map_id"], "千流叩海");
    reference["map_id"] = id.into();
    project::write_json(&dir.join("ramp-clearance-reference.json"), &reference).unwrap();
    project::write_json(&config, &cfg).unwrap();
    let p = Project::load(&root.0, id).unwrap();
    p.ensure_dirs().unwrap();
    let source: Value = serde_json::from_slice(
        &fs::read(original.join("maps").join(id).join("config.json")).unwrap(),
    )
    .unwrap();
    let zip = archive::unzip(
        &fs::read(
            original.join("maps").join(id).join(
                source["COORDINATE_PIPELINE"]["source_package"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    for (n, b) in zip.iter().filter(|(n, _)| n.as_str() != "layout.json") {
        fs::write(p.assets.join(n), b).unwrap();
    }
    let mut partial: Layout = serde_json::from_slice(&zip["layout.json"]).unwrap();
    let names = ra3maps::gameplay::scripts::load_rules(&p)
        .unwrap()
        .spawn_waypoints
        .into_iter()
        .map(|s| s.name)
        .collect::<Vec<_>>();
    let full_count = partial.objects.len();
    partial.objects.retain(|o| {
        !ra3maps::formats::map::prop(&o.props, "waypointName")
            .and_then(ra3maps::formats::map::Value::text)
            .is_some_and(|s| names.iter().any(|n| n == s))
    });
    assert_eq!(full_count - partial.objects.len(), 31);
    project::write_json(&p.source().join("layout.json"), &partial).unwrap();
    // Historical packages retain an earlier GameMap/atlas ID. Migrate that
    // metadata through the real release path before treating the map as a new
    // publishable frozen source; native gameplay bytes remain exact.
    let legacy = build::release_files(&p)
        .into_iter()
        .map(|n| {
            let b = fs::read(p.assets.join(&n)).unwrap();
            (n, b)
        })
        .collect::<Vec<_>>();
    assert!(
        ra3maps::pipeline::release::validate_runtime_files(&legacy)
            .unwrap_err()
            .to_string()
            .contains("stale GameMap")
    );
    let input = root.0.join("historical-source.zip");
    fs::write(
        &input,
        archive::zip(
            &legacy
                .iter()
                .map(|(n, b)| (format!("{}/{n}", p.name), b.clone()))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    let migrated = ra3maps::pipeline::release::run(
        &root.0,
        id,
        &input,
        &Options {
            bump: true,
            note: "migrate historical terrain asset IDs".into(),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        fs::read(migrated.assets.join(format!("{}.map", migrated.name))).unwrap(),
        legacy.iter().find(|(n, _)| n.ends_with(".map")).unwrap().1
    );
    fs::create_dir_all(migrated.source()).unwrap();
    project::write_json(&migrated.source().join("layout.json"), &partial).unwrap();
    let p = migrated;
    pipeline::adopt(&p).unwrap();
    let p = Project::load(&root.0, id).unwrap();
    assert!(pipeline::enabled(&p));
    let canonical = Layout::load(&p).unwrap();
    assert_eq!(canonical.objects.len(), full_count);
    let id_map: Value = serde_json::from_slice(
        &fs::read(p.path("authoring/coordinate-id-map.json").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read(
            p.path(&format!("histories/{}/coordinate-id-map.json", p.version))
                .unwrap()
        )
        .unwrap(),
        fs::read(p.path("authoring/coordinate-id-map.json").unwrap()).unwrap()
    );
    assert!(
        !p.path(&format!("authoring/id-map-{}.json", p.version))
            .unwrap()
            .exists()
    );
    assert!(!p.path(&p.version).unwrap().exists());
    assert_eq!(
        id_map["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "generated_waypoint")
            .count(),
        31
    );
    build::run(&root.0, id, &stages()).unwrap();
    let report: Value =
        serde_json::from_slice(&fs::read(p.assets.join("spatial/report.json")).unwrap()).unwrap();
    assert!(
        report["counts"]["requires_transport_routes"]
            .as_u64()
            .unwrap()
            > 0
    );
    let routes: Value =
        serde_json::from_slice(&fs::read(p.assets.join("spatial/routes.json")).unwrap()).unwrap();
    for r in routes
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["domain"] == "mixed")
    {
        assert!(r["reachable"].is_null());
        assert!(r["world_distance"].is_null());
        assert_eq!(r["route_status"], "requires_transport");
    }
    let coordinates: Value =
        serde_json::from_slice(&fs::read(p.assets.join("coordinates/coordinates.json")).unwrap())
            .unwrap();
    assert_eq!(coordinates["statistics"]["map_objects"], full_count);
    assert!(coordinates["statistics"]["water_cells"].as_u64().unwrap() > 0);
    let mut policy_cfg = p.cfg.clone();
    policy_cfg["COORDINATE_PIPELINE"]["policy"] = json!("authoring/override.json");
    project::write_json(&config, &policy_cfg).unwrap();
    project::write_json(
        &p.path("authoring/override.json").unwrap(),
        &json!({"bucket_size":64}),
    )
    .unwrap();
    let p = Project::load(&root.0, id).unwrap();
    let merged = pipeline::audit(&p).unwrap();
    assert_eq!(merged["policy"]["bucket_size"], 64.);
    assert_eq!(merged["policy"]["ore_reserve_radius"], 20.);
}
