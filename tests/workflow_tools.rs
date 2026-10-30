//! Synthetic workflow regressions: no production maps are built or changed.
use ra3maps::{
    formats::map::MapFile,
    pipeline::{
        evidence,
        project::{self, Project},
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ra3-workflow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        project::write_json(&root.join("maps/Synthetic/config.json"),&json!({"SCHEMA_VERSION":1,"MAP_NAME":"Synthetic","GRID":8,"BORDER":0,"CELL":10.,"Z_SCALE":0.5,"TERRAIN_ONLY":true,"TERRAIN":{"use_v11_layers":true,"v11_baked":true,"base_min_flat_radius":0}})).unwrap();
        project::write_json(
            &root.join("maps/Synthetic/version.json"),
            &project::parse_version("T0.0.1").unwrap(),
        )
        .unwrap();
        Self(root)
    }
    fn project(&self) -> Project {
        Project::load(&self.0, "Synthetic").unwrap()
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_ra3maps"))
            .args(args)
            .arg("--root")
            .arg(&self.0)
            .output()
            .unwrap()
    }
    fn log(&self) -> Value {
        let path = fs::read_dir(self.0.join("work/runs/Synthetic"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn failed_stage_is_timed_without_writing_a_native_map() {
    let f = Fixture::new();
    let cfg = f.0.join("maps/Synthetic/config.json");
    let before = fs::read(&cfg).unwrap();
    let output = f.cli(&["build", "--map", "Synthetic", "--only", "verify"]);
    assert!(!output.status.success());
    let log = f.log();
    assert_eq!(log["status"], "failed");
    assert_eq!(log["stages"][0]["name"], "verify");
    assert_eq!(log["stages"][0]["status"], "failed");
    assert!(log["stages"][0]["elapsed_seconds"].is_number());
    assert!(!f.project().assets.join("SyntheticT0.0.1.map").exists());
    assert_eq!(fs::read(cfg).unwrap(), before);
}

#[test]
fn rejected_search_budget_is_recorded_as_failure_not_success() {
    let f = Fixture::new();
    let output = f.cli(&["plan-layout", "--map", "Synthetic", "--budget", "0"]);
    assert!(!output.status.success());
    assert_eq!(f.log()["command"], "plan-layout");
    assert_eq!(f.log()["status"], "failed");
    assert!(
        !f.project()
            .assets
            .join("planning/plan-layout.json")
            .exists()
    );
}

#[test]
fn evidence_index_distinguishes_matching_stale_and_unbound_reports() {
    let f = Fixture::new();
    let p = f.project();
    fs::create_dir_all(&p.assets).unwrap();
    let map = MapFile {
        order: Vec::new(),
        table: Default::default(),
        chunks: Vec::new(),
    };
    let native = map.encode().unwrap();
    let crc = crc32fast::hash(&native);
    fs::write(p.assets.join(format!("{}.map", p.name)), &native).unwrap();
    for (name, report) in [
        (
            "spawn-placement.json",
            json!({"native_crc32":crc,"passed":true}),
        ),
        (
            "ramp-clearance.json",
            json!({"native_crc32":crc.wrapping_add(1),"passed":true}),
        ),
        ("texture-report.json", json!({"passed":true})),
    ] {
        project::write_json(&p.assets.join(name), &report).unwrap();
    }
    let index = evidence::collect(&p).unwrap();
    let rows = index["reports"].as_array().unwrap();
    for (name, binding) in [
        ("spawn-placement.json", "matches_candidate"),
        ("ramp-clearance.json", "different_candidate"),
        ("texture-report.json", "unbound"),
    ] {
        let row = rows
            .iter()
            .find(|r| r["path"] == format!("reports/{name}"))
            .unwrap();
        assert_eq!(row["candidate_binding"], binding);
    }
    assert_eq!(index["independent_review"]["stored_status"], "missing");
    assert!(p.map_dir().join("histories/T0.0.1/evidence.zip").is_file());
    assert!(!p.assets.join("delivery-acceptance.json").exists());
}
