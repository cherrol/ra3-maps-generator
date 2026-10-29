use ra3maps::{
    formats::map::MapFile, pipeline::project::Project, terrain::Layout, terrain::ramp_clearance,
};
use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn frozen_report_reproduces_approach_conflicts_and_current_release_has_none() {
    let p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "陕西").unwrap();
    let old = p.release_assets("[CHAI]千壑横原T0.7.7").unwrap();
    let original = MapFile::load(old.join("[CHAI]千壑横原T0.7.7.map")).unwrap();
    let layout: Layout =
        serde_json::from_slice(&fs::read(old.join("source/layout.json")).unwrap()).unwrap();
    let report = ramp_clearance::report(&p, &original, &layout, &Default::default()).unwrap();
    let objects: BTreeSet<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["object_id"].as_u64().unwrap())
        .collect();
    assert_eq!(report["checked_objects"], 75);
    assert_eq!(objects.len(), 24);
    assert!(objects.contains(&203) && objects.contains(&207) && objects.contains(&196));
    // Configuration must enforce the new clearance instead of inheriting the
    // old report's successful baseline comparison.
    let mut enforcing = p.clone();
    enforcing.cfg["RAMP_CLEARANCE"] =
        serde_json::to_value(ramp_clearance::Policy::default()).unwrap();
    assert!(ramp_clearance::validate(&enforcing, &original, &layout).is_err());
    let current = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    let layout = Layout::load(&p).unwrap();
    let report = ramp_clearance::validate(&enforcing, &current, &layout)
        .unwrap()
        .unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["checked_objects"], 75);
    assert_eq!(report["ramps"], 27);
    let mut guarded = current.clone();
    let before = guarded.encode().unwrap();
    let gifts =
        ra3maps::gameplay::gift_clearance::repair_obstructed_slots(&p, &mut guarded, &layout)
            .unwrap();
    assert!(gifts["changes"].as_array().unwrap().is_empty());
    assert_eq!(
        guarded.encode().unwrap(),
        before,
        "published gift reservations must not move on rebuild"
    );
}
