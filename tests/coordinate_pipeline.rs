use ra3maps::{
    archive,
    build::{self, Options},
    coordinate_pipeline as pipeline,
    map::{self, MapFile},
    project::{self, Project},
    terrain::Layout,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "ra3-coordinate-tests-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let recipe = root.join("maps/千壑横原");
    fs::create_dir_all(recipe.join("authoring")).unwrap();
    let original = Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in [
        "config.json",
        "version.json",
        "layout.json",
        "authoring/source.zip",
        "authoring/layout.json",
        "authoring/policy.json",
        "authoring/baseline.json",
    ] {
        fs::copy(original.join("maps/千壑横原").join(file), recipe.join(file)).unwrap();
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
    Fixture(root)
}
fn load(root: &Fixture) -> Project {
    Project::load(&root.0, "千壑横原").unwrap()
}
fn source_files(p: &Project) -> std::collections::BTreeMap<String, Vec<u8>> {
    archive::unzip(&fs::read(p.path("authoring/source.zip").unwrap()).unwrap()).unwrap()
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
    let version = fs::read(project::version_path(&root.0, &p.id)).unwrap();
    let built = build::run(&root.0, &p.id, &stages()).unwrap();
    for (n, bytes) in files.iter().filter(|(n, _)| n.as_str() != "layout.json") {
        assert_eq!(*bytes, fs::read(built.assets.join(n)).unwrap(), "{n}");
        assert_eq!(*bytes, fs::read(built.dist.join(n)).unwrap());
    }
    assert_eq!(
        version,
        fs::read(project::version_path(&root.0, &p.id)).unwrap()
    );
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
        !root
            .0
            .join("assets/[CHAI]千壑横原/[CHAI]千壑横原T0.6.6")
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
    assert_eq!(next.version, "T0.6.6");
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
    let id = "千流叩海";
    let dir = root.0.join("maps").join(id);
    fs::create_dir_all(&dir).unwrap();
    for f in ["config.json", "version.json"] {
        fs::copy(original.join("maps").join(id).join(f), dir.join(f)).unwrap();
    }
    fs::copy(
        original.join("templates/rules").join(format!("{id}.json")),
        root.0.join("templates/rules").join(format!("{id}.json")),
    )
    .unwrap();
    let config = dir.join("config.json");
    let mut cfg: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
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
    let names = ra3maps::scripts::load_rules(&p)
        .unwrap()
        .spawn_waypoints
        .into_iter()
        .map(|s| s.name)
        .collect::<Vec<_>>();
    let full_count = partial.objects.len();
    partial.objects.retain(|o| {
        !ra3maps::map::prop(&o.props, "waypointName")
            .and_then(ra3maps::map::Value::text)
            .is_some_and(|s| names.iter().any(|n| n == s))
    });
    assert_eq!(full_count - partial.objects.len(), 31);
    project::write_json(&p.source().join("layout.json"), &partial).unwrap();
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
