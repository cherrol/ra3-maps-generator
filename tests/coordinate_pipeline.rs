use ra3maps::{
    formats::{
        archive,
        map::{self, MapFile},
    },
    pipeline::{
        build::{self, Options},
        coordinate_pipeline as pipeline,
        project::{self, Project},
    },
    terrain::Layout,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
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
    let f = Fixture(std::env::temp_dir().join(format!(
        "ra3-current-coordinate-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    support::copy_current_project(&f.0);
    f
}
fn load(f: &Fixture) -> Project {
    Project::load(&f.0, "河南").unwrap()
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
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        ..Options::default()
    }
}

#[test]
fn current_source_rebuild_preserves_files_but_requires_its_independent_review_to_publish() {
    let f = fixture();
    let p = load(&f);
    let before = source_files(&p);
    let layout: Layout = serde_json::from_slice(
        &fs::read(p.path(p.cfg["LAYOUT"].as_str().unwrap()).unwrap()).unwrap(),
    )
    .unwrap();
    let version = fs::read(p.map_dir().join("version.json")).unwrap();
    let built = build::run(&f.0, &p.id, &stages()).unwrap();
    for (n, b) in before.iter().filter(|(n, _)| n.as_str() != "layout.json") {
        assert_eq!(*b, fs::read(built.assets.join(n)).unwrap(), "{n}");
    }
    let error = build::dist(&built, true).unwrap_err().to_string();
    assert!(error.contains("independent delivery acceptance"), "{error}");
    assert!(!built.dist.exists());
    assert!(!build::zip_path(&built).exists());
    assert_eq!(version, fs::read(p.map_dir().join("version.json")).unwrap());
    let r: Value =
        serde_json::from_slice(&fs::read(p.assets.join("spatial/report.json")).unwrap()).unwrap();
    assert_eq!(r["passed"], true);
    assert_eq!(r["counts"]["objects"], 182);
    assert_eq!(r["counts"]["ramps"], layout.ramps.len());
    assert_eq!(r["counts"]["routes"], 496);
    assert_eq!(
        r["resource_distribution"]["home_counts"],
        json!([2, 2, 2, 2, 2, 2, 2, 2])
    );
    assert!(built.assets.join("coordinates/index.html").is_file());
    let exported: Value = serde_json::from_slice(
        &fs::read(built.assets.join("coordinates/coordinates.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(exported["statistics"]["ramps"], layout.ramps.len());
    assert_eq!(
        exported["points"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|point| point["category"] == "斜坡")
            .count(),
        layout.ramps.len()
    );
}

#[test]
fn changed_plan_and_common_policy_cannot_reuse_successful_spatial_cache() {
    let f = fixture();
    let p = load(&f);
    pipeline::audit(&p).unwrap();
    let path = p
        .path(p.cfg["RESOURCE_DISTRIBUTION"].as_str().unwrap())
        .unwrap();
    let original = fs::read(&path).unwrap();
    let mut plan: Value = serde_json::from_slice(&original).unwrap();
    let third = plan["slots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["ordinal"] == 3)
        .unwrap();
    third["route_id"] = json!(999);
    project::write_json(&path, &plan).unwrap();
    assert!(pipeline::audit(&p).is_err());
    fs::write(&path, original).unwrap();
    pipeline::audit(&p).unwrap();
    let common = f.0.join("rules/coordinate-pipeline.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&common).unwrap()).unwrap();
    policy["resource_distribution"]["main_route_max_distance"] = json!(1.);
    project::write_json(&common, &policy).unwrap();
    assert!(pipeline::audit(&p).is_err());
}

#[test]
fn strict_ramp_policy_rejects_source_conflicts_instead_of_inheriting_them() {
    let f = fixture();
    let p = load(&f);
    pipeline::audit(&p).unwrap();
    let common = f.0.join("rules/coordinate-pipeline.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&common).unwrap()).unwrap();
    policy["ramp_clearance"]["gap"] = json!(1000.);
    project::write_json(&common, &policy).unwrap();
    assert!(pipeline::audit(&p).is_err());
}

#[test]
fn source_geometry_metadata_and_native_assets_remain_guarded() {
    let f = fixture();
    let p = load(&f);
    let path = p.path("authoring/layout.json").unwrap();
    let original = fs::read(&path).unwrap();
    let mut l: Layout = serde_json::from_slice(&original).unwrap();
    l.ramps[0]["x"] = json!(5.);
    project::write_json(&path, &l).unwrap();
    assert!(pipeline::prepare(&p).is_err());
    fs::write(&path, &original).unwrap();
    pipeline::assemble(&p).unwrap();
    let file = p.assets.join(format!("{}.map", p.name));
    let mut native = MapFile::load(&file).unwrap();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    objects[0].angle += 0.2;
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    native.save(&file).unwrap();
    let damaged = fs::read(&file).unwrap();
    assert!(build::run(&f.0, &p.id, &stages()).is_err());
    assert_eq!(damaged, fs::read(file).unwrap());
}

#[test]
fn initial_adoption_requires_complete_current_plan_and_keeps_failed_inputs_intact() {
    let f = fixture();
    let p = load(&f);
    pipeline::assemble(&p).unwrap();
    let config = p.map_dir().join("config.json");
    let mut cfg = p.cfg.clone();
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    project::write_json(&config, &cfg).unwrap();
    let source = p.path("authoring/source.zip").unwrap();
    fs::remove_file(&source).unwrap();
    let path = p
        .path(p.cfg["RESOURCE_DISTRIBUTION"].as_str().unwrap())
        .unwrap();
    let good = fs::read(&path).unwrap();
    let mut plan: Value = serde_json::from_slice(&good).unwrap();
    plan["home_ores_per_seat"] = json!(3);
    project::write_json(&path, &plan).unwrap();
    let before = fs::read(&config).unwrap();
    assert!(pipeline::adopt(&load(&f)).is_err());
    assert!(!source.exists());
    assert_eq!(before, fs::read(&config).unwrap());
    fs::write(&path, good).unwrap();
    let error = pipeline::adopt(&load(&f)).unwrap_err().to_string();
    assert!(error.contains("independent delivery acceptance"), "{error}");
    assert!(!source.exists());
    assert_eq!(before, fs::read(&config).unwrap());
}

#[test]
fn colliding_coordinate_edit_rolls_back_version_and_new_release() {
    let f = fixture();
    let p = load(&f);
    let version = fs::read(p.map_dir().join("version.json")).unwrap();
    let path = p.path("authoring/layout.json").unwrap();
    let mut l: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let ids: Vec<_> = l
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.type_name.contains("TechStructure"))
        .map(|(i, _)| i)
        .collect();
    l.objects[ids[1]].cell = l.objects[ids[0]].cell;
    project::write_json(&path, &l).unwrap();
    let mut options = stages();
    options.bump = true;
    assert!(build::run(&f.0, &p.id, &options).is_err());
    assert_eq!(version, fs::read(p.map_dir().join("version.json")).unwrap());
    let next = project::format_version(&project::bump(&p.state, false, 1, "").unwrap()).unwrap();
    assert!(
        !p.release_assets(&p.release_name(&next).unwrap())
            .unwrap()
            .exists()
    );
}

#[test]
fn incomplete_native_package_cannot_replace_the_current_source() {
    let f = fixture();
    let p = load(&f);
    pipeline::assemble(&p).unwrap();
    let file = p.assets.join(format!("{}.map", p.name));
    let mut native = MapFile::load(&file).unwrap();
    native.get_mut("AssetList").unwrap().payload = 0u32.to_le_bytes().to_vec();
    native.save(&file).unwrap();
    let source = fs::read(p.path("authoring/source.zip").unwrap()).unwrap();
    let baseline = fs::read(p.path("authoring/baseline.json").unwrap()).unwrap();
    assert!(pipeline::adopt(&p).is_err());
    assert_eq!(
        source,
        fs::read(p.path("authoring/source.zip").unwrap()).unwrap()
    );
    assert_eq!(
        baseline,
        fs::read(p.path("authoring/baseline.json").unwrap()).unwrap()
    );
}

#[test]
fn removed_compatibility_flags_cannot_reuse_a_successful_spatial_cache() {
    let f = fixture();
    let p = load(&f);
    pipeline::audit(&p).unwrap();
    for key in ["SPAWN_PLACEMENT_REFERENCE", "RAMP_CLEARANCE_REFERENCE"] {
        let mut changed = p.clone();
        changed.cfg[key] = json!("missing-old-reference.json");
        assert!(
            pipeline::audit(&changed)
                .unwrap_err()
                .to_string()
                .contains("no longer supported")
        );
    }
    for key in ["RAMP_PROFILE", "TEXTURE_PROFILE"] {
        let mut changed = p.clone();
        changed.cfg[key] = json!("legacy");
        assert!(
            pipeline::audit(&changed)
                .unwrap_err()
                .to_string()
                .contains("no longer supported")
        );
    }
}
