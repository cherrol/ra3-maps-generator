//! Independent review records bind the exact candidate and cannot replace audits.
use ra3maps::{
    formats::map::{Chunk, HeightMap, MapFile},
    pipeline::{acceptance, project::Project},
    terrain::Layout,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    project: Project,
    layout: Layout,
    files: Vec<(String, Vec<u8>)>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.project.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("work/placement-optimization-temp")
            .join(format!(
                "delivery-gate-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let id = "synthetic".to_owned();
        let name = "SyntheticT0.0.1".to_owned();
        fs::create_dir_all(root.join("templates/rules")).unwrap();
        fs::write(
            root.join("templates/rules/common-T0.5.1.json"),
            json!({"production_placement":{"full_footprint":true}}).to_string(),
        )
        .unwrap();
        let project = Project {
            assets: root.join("assets").join(&id).join(&name),
            dist: root.join("dist").join(&id).join(&name),
            root,
            id,
            name,
            version: "T0.0.1".to_owned(),
            group: "Synthetic".to_owned(),
            state: json!({}),
            cfg: json!({"SCHEMA_VERSION":1,"GRID":16,"BORDER":0,"CELL":10.0,"Z_SCALE":0.0390625}),
        };
        let layout: Layout = serde_json::from_value(json!({
            "schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[]
        }))
        .unwrap();
        let hm = HeightMap {
            width: 16,
            height: 16,
            border: 0,
            borders: vec![],
            elevations: vec![10000; 256],
        };
        let table = BTreeMap::from([
            (1, "HeightMapData".to_owned()),
            (2, "ObjectsList".to_owned()),
        ]);
        let map = MapFile {
            order: vec![
                ("ObjectsList".to_owned(), 2),
                ("HeightMapData".to_owned(), 1),
            ],
            table,
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 1,
                    payload: vec![],
                },
            ],
        };
        let files = vec![
            (format!("{}.map", project.name), map.encode().unwrap()),
            (
                "map.xml".to_owned(),
                b"synthetic XML evidence input".to_vec(),
            ),
            ("map.str".to_owned(), b"synthetic map strings".to_vec()),
        ];
        fs::create_dir_all(project.source()).unwrap();
        fs::write(
            project.source().join("layout.json"),
            serde_json::to_vec(&layout).unwrap(),
        )
        .unwrap();
        Self {
            project,
            layout,
            files,
        }
    }

    fn record(&self) -> Value {
        let mut record =
            acceptance::pending_record(&self.project, &self.files, &self.layout).unwrap();
        let parent = acceptance::report_path(&self.project).unwrap();
        let parent = parent.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        let bytes = b"Synthetic reviewer measurements for an isolated acceptance-schema fixture.";
        fs::write(parent.join("measurements.txt"), bytes).unwrap();
        record["status"] = json!("passed");
        record["reviewer"] = json!("synthetic-test-reviewer");
        record["reviewed_at"] = json!("2026-10-30T09:00:00+08:00");
        for (_, item) in record["items"].as_object_mut().unwrap() {
            item["status"] = json!("passed");
            item["decision"] = json!("Synthetic independent decision; not a real-map approval.");
            item["evidence"] = json!([{
                "path":"measurements.txt","crc32":crc32fast::hash(bytes),"bytes":bytes.len()
            }]);
        }
        record
    }

    fn write(&self, record: &Value) {
        let path = acceptance::report_path(&self.project).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(record).unwrap()).unwrap();
    }

    fn checked(&self) -> ra3maps::Result<()> {
        acceptance::require_delivery_with_layout(&self.project, &self.files, &self.layout)
    }
}

#[test]
fn exact_review_passes_both_memory_and_saved_candidate_paths() {
    let f = Fixture::new();
    f.write(&f.record());
    f.checked().unwrap();
    acceptance::require_delivery(&f.project, &f.files).unwrap();
    let mut reordered = f.files.clone();
    reordered.reverse();
    acceptance::require_delivery(&f.project, &reordered).unwrap();
}

#[test]
fn missing_review_and_generated_pending_review_never_authorize_publication() {
    let f = Fixture::new();
    assert!(f.checked().unwrap_err().to_string().contains("required at"));
    let record = acceptance::pending_record(&f.project, &f.files, &f.layout).unwrap();
    assert_eq!(record["status"], "pending");
    assert!(
        record["items"]
            .as_object()
            .unwrap()
            .values()
            .all(|item| item["status"] == "pending")
    );
    f.write(&record);
    assert!(
        f.checked()
            .unwrap_err()
            .to_string()
            .contains("failed or pending")
    );
}

#[test]
fn full_item_matrix_and_current_revision_are_required() {
    let f = Fixture::new();
    let valid = f.record();
    for (id, _) in acceptance::ITEMS {
        let mut record = valid.clone();
        record["items"].as_object_mut().unwrap().remove(id);
        f.write(&record);
        assert!(f.checked().is_err(), "missing {id}");
    }
    for field in [
        "schema_version",
        "rules_revision",
        "map_id",
        "map",
        "version",
    ] {
        let mut record = valid.clone();
        record[field] = json!("obsolete");
        f.write(&record);
        assert!(f.checked().is_err(), "wrong {field}");
    }
    let mut record = valid.clone();
    record["items"]["unknown"] = record["items"]["ramp_priority"].clone();
    f.write(&record);
    assert!(f.checked().is_err());
}

#[test]
fn review_before_terrain_transition_requirements_cannot_authorize_publication() {
    let f = Fixture::new();
    let mut record = f.record();
    let old_revision = json!("independent-delivery-v1-placement-priority");
    record["rules_revision"] = old_revision.clone();
    record["binding"]["rules_revision"] = old_revision;
    f.write(&record);
    assert!(
        f.checked()
            .unwrap_err()
            .to_string()
            .contains("obsolete schema, rule revision or release identity")
    );
}

#[test]
fn failed_pending_unknown_and_unsupported_not_applicable_items_fail() {
    let f = Fixture::new();
    let valid = f.record();
    for status in ["failed", "pending", "inherited", "通过", "not_applicable"] {
        let mut record = valid.clone();
        record["items"]["ore_distribution"]["status"] = json!(status);
        record["items"]["ore_distribution"]["reason"] =
            json!("Cannot exempt playable ore allocation.");
        f.write(&record);
        assert!(f.checked().is_err(), "{status}");
    }
    let mut record = valid.clone();
    record["items"]["ramp_priority"]["status"] = json!("not_applicable");
    f.write(&record);
    assert!(f.checked().is_err());
    record["items"]["ramp_priority"]["reason"] =
        json!("This synthetic flat fixture declares no ramps.");
    f.write(&record);
    f.checked().unwrap();
    record["items"]["ramp_priority"]["evidence"] = json!([]);
    f.write(&record);
    assert!(f.checked().is_err());
}

#[test]
fn terrain_only_still_requires_the_applicable_design_check() {
    let mut f = Fixture::new();
    f.project.cfg["TERRAIN_ONLY"] = json!(true);
    let mut record = f.record();
    for (id, _) in acceptance::ITEMS {
        record["items"][id]["status"] = json!("not_applicable");
        record["items"][id]["reason"] =
            json!("Synthetic terrain-only fixture applicability evidence.");
    }
    f.write(&record);
    assert!(f.checked().is_err());
    record["items"]["route_width_intrusions"]["status"] = json!("passed");
    f.write(&record);
    f.checked().unwrap();
}

#[test]
fn candidate_layout_policy_and_plan_changes_invalidate_the_review() {
    let mut f = Fixture::new();
    let valid = f.record();
    f.write(&valid);
    f.files[2].1.push(b'!');
    assert!(f.checked().is_err());
    f.files[2].1.pop();
    f.checked().unwrap();
    f.layout.ramps.push(json!({"id":"synthetic-changed-ramp"}));
    assert!(f.checked().is_err());
    f.layout.ramps.clear();
    f.project.cfg["WATER_Z"] = json!(190.0);
    assert!(f.checked().is_err());
    f.project.cfg.as_object_mut().unwrap().remove("WATER_Z");
    fs::write(
        f.project.root.join("templates/rules/common-T0.5.1.json"),
        json!({"production_placement":{"full_footprint":false}}).to_string(),
    )
    .unwrap();
    assert!(f.checked().is_err());
    fs::write(
        f.project.root.join("templates/rules/common-T0.5.1.json"),
        json!({"production_placement":{"full_footprint":true}}).to_string(),
    )
    .unwrap();
    fs::create_dir_all(f.project.map_dir()).unwrap();
    fs::write(f.project.map_dir().join("plan.json"), "{\"routes\":[]}").unwrap();
    f.project.cfg["RESOURCE_DISTRIBUTION"] = json!("plan.json");
    f.write(&f.record());
    fs::write(f.project.map_dir().join("plan.json"), "{\"routes\":[1]}").unwrap();
    assert!(f.checked().is_err());
}

#[test]
fn formatting_and_source_pointer_updates_do_not_change_candidate_identity() {
    let mut f = Fixture::new();
    f.write(&f.record());
    fs::write(
        f.project.root.join("templates/rules/common-T0.5.1.json"),
        "{\n  \"production_placement\": { \"full_footprint\": true }\n}\n",
    )
    .unwrap();
    f.checked().unwrap();
    f.project.cfg["COORDINATE_PIPELINE"] = json!({
        "source_package":"authoring/source.zip","source_name":f.project.name,
        "source_crc32":123,"layout":"authoring/layout.json","baseline":"authoring/baseline.json"
    });
    f.checked().unwrap();
}

#[test]
fn evidence_is_required_content_bound_and_cannot_escape_the_project() {
    let f = Fixture::new();
    let valid = f.record();
    let mut record = valid.clone();
    record["items"]["ramp_priority"]["decision"] = json!("");
    f.write(&record);
    assert!(f.checked().is_err());
    let mut record = valid.clone();
    record["items"]["ramp_priority"]["evidence"][0]["path"] = json!("missing.txt");
    f.write(&record);
    assert!(f.checked().is_err());
    let mut record = valid.clone();
    record["items"]["ramp_priority"]["evidence"][0]["path"] = json!("/etc/passwd");
    f.write(&record);
    assert!(f.checked().is_err());
    f.write(&valid);
    fs::write(
        f.project.assets.join("measurements.txt"),
        "changed evidence",
    )
    .unwrap();
    assert!(f.checked().is_err());
}

#[test]
fn map_relative_record_is_supported_but_stale_blueprint_is_not_a_snapshot() {
    let mut f = Fixture::new();
    f.project.cfg["DELIVERY_ACCEPTANCE"] = json!("authoring/delivery-review.json");
    f.write(&f.record());
    f.checked().unwrap();
    acceptance::require_delivery(&f.project, &f.files).unwrap();
    fs::remove_file(f.project.source().join("layout.json")).unwrap();
    assert!(acceptance::require_delivery(&f.project, &f.files).is_err());
    f.project.cfg["DELIVERY_ACCEPTANCE"] = json!("../other-map/review.json");
    assert!(acceptance::report_path(&f.project).is_err());
}

#[test]
fn duplicate_package_names_are_not_hidden_by_the_binding_map() {
    let mut f = Fixture::new();
    f.files.push(f.files[2].clone());
    assert!(acceptance::binding(&f.project, &f.files, &f.layout).is_err());
}

#[test]
fn staged_candidates_preserve_manual_review_and_do_not_publish() {
    let mut f = Fixture::new();
    let original_layout = fs::read(f.project.source().join("layout.json")).unwrap();
    let pending = acceptance::stage_candidate(&f.project, &f.files, &f.layout).unwrap();
    assert!(pending.starts_with(f.project.map_dir().join("data/delivery-candidates")));
    assert!(!f.project.dist.exists());
    assert!(!f.project.map_dir().join("version.json").exists());
    assert_eq!(
        fs::read(f.project.source().join("layout.json")).unwrap(),
        original_layout
    );
    for (name, bytes) in &f.files {
        assert_eq!(
            fs::read(pending.parent().unwrap().join(name)).unwrap(),
            *bytes
        );
    }
    let mut reviewed: Value = serde_json::from_slice(&fs::read(&pending).unwrap()).unwrap();
    reviewed["reviewer"] = json!("review-in-progress");
    let human = serde_json::to_vec_pretty(&reviewed).unwrap();
    fs::write(&pending, &human).unwrap();
    assert_eq!(
        acceptance::stage_candidate(&f.project, &f.files, &f.layout).unwrap(),
        pending
    );
    assert_eq!(fs::read(&pending).unwrap(), human);
    f.files[2].1.push(b'!');
    let next = acceptance::stage_candidate(&f.project, &f.files, &f.layout).unwrap();
    assert_ne!(next, pending);
    assert_eq!(fs::read(&pending).unwrap(), human);
    let staged_file = next.parent().unwrap().join("map.str");
    fs::write(&staged_file, "human-edited-candidate").unwrap();
    assert!(acceptance::stage_candidate(&f.project, &f.files, &f.layout).is_err());
    assert_eq!(fs::read(staged_file).unwrap(), b"human-edited-candidate");
}

#[test]
fn existing_water_and_unresolved_entities_cannot_be_marked_inapplicable() {
    use ra3maps::terrain::ObjectSpec;
    let mut f = Fixture::new();
    let mut native = MapFile::load_bytes(&f.files[0].1).unwrap();
    let id = native.id("Object").unwrap();
    assert!(id > 0);
    let object = ObjectSpec {
        cell: [8., 8.],
        version: 3,
        angle: 0.,
        road_type: 0,
        type_name: "UnresolvedSyntheticBuilding".into(),
        props: vec![],
    };
    f.layout.objects.push(object.clone());
    native.get_mut("ObjectsList").unwrap().payload =
        ra3maps::formats::map::write_objects(&[object.object(&f.project)], &native).unwrap();
    f.files[0].1 = native.encode().unwrap();
    let mut record = f.record();
    record["items"]["neutral_building_siting"]["status"] = json!("not_applicable");
    record["items"]["neutral_building_siting"]["reason"] =
        json!("Unsupported assertion of absence.");
    f.write(&record);
    assert!(f.checked().is_err());
    record["items"]["neutral_building_siting"]["status"] = json!("passed");
    f.write(&record);
    f.checked().unwrap();
    let water = native.id("StandingWaterAreas").unwrap();
    native.chunks.push(Chunk {
        name_idx: water,
        version: 2,
        payload: 1_u32.to_le_bytes().to_vec(),
    });
    f.files[0].1 = native.encode().unwrap();
    let mut record = f.record();
    record["items"]["shore_perimeter"]["status"] = json!("not_applicable");
    record["items"]["shore_perimeter"]["reason"] = json!("Unsupported assertion of no shoreline.");
    f.write(&record);
    assert!(f.checked().is_err());
    f.project.cfg["TERRAIN_ONLY"] = json!(true);
    assert!(acceptance::binding(&f.project, &f.files, &f.layout).is_err());
}

#[test]
fn actual_or_unresolved_building_creations_cannot_be_marked_inapplicable() {
    use ra3maps::{formats::binary::Writer, gameplay::scripts};
    for building in ["AlliedBarracks", "UnresolvedSyntheticFactory"] {
        let mut f = Fixture::new();
        let mut record = f.record();
        for id in ["gift_intrusion_exclusion", "production_base_proximity"] {
            record["items"][id]["status"] = json!("not_applicable");
            record["items"][id]["reason"] = json!("Synthetic map initially contains no creations.");
        }
        f.write(&record);
        f.checked().unwrap();
        let mut native = MapFile::load_bytes(&f.files[0].1).unwrap();
        let mut body = Writer::new();
        body.u32(1);
        body.u32(3);
        body.u32(2);
        for (kind, text) in [(15, building), (7, "synthetic-waypoint")] {
            body.u32(kind);
            body.i32(0);
            body.f32(0.);
            body.s16(text).unwrap();
        }
        body.u32(1);
        let action = scripts::Node {
            asset: "ScriptAction".into(),
            version: 3,
            prefix: String::new(),
            body: scripts::hex(&body.data),
            command: Some("CREATE_NAMED_ON_TEAM_AT_WAYPOINT".into()),
            children: None,
        };
        let payload = scripts::encode_nodes(&[action], &mut native).unwrap();
        let name_idx = native.id("PlayerScriptsList").unwrap();
        native.chunks.push(Chunk {
            name_idx,
            version: 1,
            payload,
        });
        f.files[0].1 = native.encode().unwrap();
        record["binding"] = acceptance::binding(&f.project, &f.files, &f.layout).unwrap();
        f.write(&record);
        assert!(f.checked().is_err(), "{building}");
        f.project.cfg["TERRAIN_ONLY"] = json!(true);
        assert!(acceptance::binding(&f.project, &f.files, &f.layout).is_err());
    }
}

#[cfg(unix)]
#[test]
fn evidence_and_review_symlinks_must_stay_inside_the_project() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let valid = f.record();
    let outside = f.project.root.with_extension("outside-evidence");
    fs::write(&outside, b"outside-project evidence").unwrap();
    symlink(&outside, f.project.assets.join("outside.txt")).unwrap();
    let mut record = valid.clone();
    record["items"]["ramp_priority"]["evidence"][0] = json!({
        "path":"outside.txt","crc32":crc32fast::hash(b"outside-project evidence"),"bytes":24
    });
    f.write(&record);
    assert!(f.checked().is_err());
    let record_path = acceptance::report_path(&f.project).unwrap();
    fs::remove_file(&record_path).unwrap();
    fs::write(&outside, serde_json::to_vec(&valid).unwrap()).unwrap();
    symlink(&outside, record_path).unwrap();
    assert!(f.checked().is_err());
    fs::remove_file(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn candidate_staging_rejects_a_symlinked_data_directory_before_writing_outside() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.project.root.with_extension("outside-candidate");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(f.project.map_dir()).unwrap();
    symlink(&outside, f.project.map_dir().join("data")).unwrap();
    assert!(acceptance::stage_candidate(&f.project, &f.files, &f.layout).is_err());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_dir(outside).unwrap();
}
