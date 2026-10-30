//! Targeted gift editing must preserve release state until its candidate is reviewed.
use ra3maps::{
    coordinates::Frame,
    formats::{
        archive,
        map::{self, MapFile, Value as NativeValue, prop},
        refpack,
    },
    pipeline::{build, project::Project, release},
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
mod support;

struct Fixture {
    root: PathBuf,
    project: Project,
    source: PathBuf,
    native: MapFile,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ra3-targeted-gift-release-{}-{label}",
            std::process::id()
        ));
        let mut project = support::copy_current_project(&root);
        let (_, mut files) = support::current_package();
        let mut native = MapFile::load_bytes(
            &files
                .iter()
                .find(|(name, _)| name.ends_with(".map"))
                .unwrap()
                .1,
        )
        .unwrap();
        let before = native.clone();
        let declared = ra3maps::terrain::Layout::load(&project).unwrap();
        let (mut layout, _) =
            ra3maps::coordinates::native_layout(&project, &before, &declared).unwrap();
        let frame = Frame::from_project(&project);
        let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
        let original_objects = objects.clone();
        let allocation: Value = serde_json::from_slice(
            &fs::read(
                project
                    .path(project.cfg["RESOURCE_DISTRIBUTION"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let ore_id = allocation["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|slot| slot["seat"] == 8 && slot["ordinal"] == 1)
            .unwrap()["ore_id"]
            .as_u64()
            .unwrap() as usize;
        let ore = &objects[ore_id - 1];
        assert_eq!(ore.type_name, "OreNode");
        let ore_cell = frame.grid([ore.position[0] as f64, ore.position[1] as f64]);
        let target = objects
            .iter_mut()
            .find(|object| {
                prop(&object.props, "waypointName").and_then(NativeValue::text)
                    == Some("gift-alt-SW83-1")
            })
            .unwrap();
        let position = frame.world([ore_cell[0], ore_cell[1] - 28.]);
        target.position[0] = position[0] as f32;
        target.position[1] = position[1] as f32;
        native.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &native).unwrap();
        ra3maps::gameplay::gift_clearance::sync_moved_areas(
            &mut native,
            &original_objects,
            &objects,
        )
        .unwrap();
        ra3maps::gameplay::production_facing::apply(&project, &mut native).unwrap();
        layout.objects = objects
            .iter()
            .map(|object| ra3maps::terrain::ObjectSpec {
                cell: frame.grid([object.position[0] as f64, object.position[1] as f64]),
                version: object.version,
                angle: object.angle,
                road_type: object.road_type,
                type_name: object.type_name.clone(),
                props: object.props.clone(),
            })
            .collect();
        let audit =
            ra3maps::gameplay::gift_clearance::audit_reservations(&project, &native, &layout)
                .unwrap();
        assert!(
            audit["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["waypoint"] == "gift-alt-SW83-1")
        );
        let native_bytes = refpack::pack(&native.encode().unwrap()).unwrap();
        files
            .iter_mut()
            .find(|(name, _)| name.ends_with(".map"))
            .unwrap()
            .1 = native_bytes.clone();
        let xml = files
            .iter_mut()
            .find(|(name, _)| name == "map.xml")
            .unwrap();
        xml.1 = ra3maps::gameplay::production_spawns::update_xml(
            std::str::from_utf8(&xml.1).unwrap(),
            &before,
            &native,
        )
        .unwrap()
        .into_bytes();
        release::validate_runtime_files(&files).unwrap();
        let layout_bytes = serde_json::to_vec_pretty(&layout).unwrap();
        fs::write(
            project
                .path(
                    project.cfg["COORDINATE_PIPELINE"]["layout"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap(),
            &layout_bytes,
        )
        .unwrap();
        let mut frozen = files.clone();
        frozen.push(("layout.json".into(), layout_bytes));
        fs::write(
            project
                .path(
                    project.cfg["COORDINATE_PIPELINE"]["source_package"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap(),
            archive::zip(&frozen).unwrap(),
        )
        .unwrap();
        project.cfg["COORDINATE_PIPELINE"]["source_crc32"] = json!(crc32fast::hash(&native_bytes));
        fs::write(
            project.map_dir().join("config.json"),
            serde_json::to_vec_pretty(&project.cfg).unwrap(),
        )
        .unwrap();
        let project = Project::load(&root, &project.id).unwrap();
        let source = root.join("input.zip");
        fs::write(
            &source,
            archive::zip(
                &files
                    .into_iter()
                    .map(|(name, bytes)| (format!("{}/{name}", project.name), bytes))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
        Self {
            root,
            project,
            source,
            native,
        }
    }

    fn plan(&self, repairs: Value) -> Value {
        json!({
            "schema_version":1,
            "source_native_decoded_crc32":crc32fast::hash(&self.native.encode().unwrap()),
            "repairs":repairs
        })
    }

    fn run(&self, plan: &Value) -> String {
        let path = self.root.join("repair-plan.json");
        fs::write(&path, serde_json::to_vec_pretty(plan).unwrap()).unwrap();
        let result = release::repair_gifts(
            &self.root,
            &self.project.id,
            &self.source,
            &path,
            &build::Options {
                bump: true,
                ..Default::default()
            },
        );
        match result {
            Ok(_) => panic!("unreviewed or invalid repair must not publish"),
            Err(error) => error.to_string(),
        }
    }

    fn protected(&self) -> Vec<(PathBuf, Vec<u8>)> {
        [
            self.project.map_dir().join("config.json"),
            self.project.map_dir().join("version.json"),
            self.project.map_dir().join("authoring/source.zip"),
            self.source.clone(),
        ]
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn source_mismatch_and_invalid_directives_cannot_commit_or_stage_a_release() {
    let f = Fixture::new("invalid");
    let protected = f.protected();
    let mut mismatch = f.plan(json!([]));
    mismatch["source_native_decoded_crc32"] =
        json!(mismatch["source_native_decoded_crc32"].as_u64().unwrap() ^ 1);
    let error = f.run(&mismatch);
    assert!(error.to_ascii_lowercase().contains("crc"), "{error}");
    let unknown = f.plan(json!([{
        "waypoint":"missing-gift-waypoint", "strategy":"local", "reason":"test scope"
    }]));
    let error = f.run(&unknown);
    assert!(error.contains("missing-gift-waypoint"), "{error}");
    let duplicate = f.plan(json!([
        {"waypoint":"gift-alt-SW83-1", "strategy":"local", "reason":"first"},
        {"waypoint":"gift-alt-SW83-1", "strategy":"local", "reason":"second"}
    ]));
    let error = f.run(&duplicate);
    assert!(error.to_ascii_lowercase().contains("duplicate"), "{error}");
    let mut duplicate_material = f.plan(json!([]));
    duplicate_material["ramp_material_joins"] = json!({"ramp_ids":[21,21]});
    let error = f.run(&duplicate_material);
    assert!(error.contains("unique one-based ramp IDs"), "{error}");
    for (path, before) in protected {
        assert_eq!(fs::read(path).unwrap(), before);
    }
    assert!(!f.root.join("assets").exists());
    assert!(!f.root.join("dist").exists());
    assert!(
        !f.project
            .map_dir()
            .join("data/delivery-candidates")
            .exists()
    );
}

#[test]
fn an_unreviewed_targeted_repair_preserves_state_and_archives_its_plan_and_comparisons() {
    let f = Fixture::new("pending");
    let protected = f.protected();
    let plan = f.plan(json!([]));
    let error = f.run(&plan);
    assert!(error.contains("review candidate"), "{error}");
    for (path, before) in protected {
        assert_eq!(fs::read(path).unwrap(), before);
    }
    assert!(!f.root.join("assets").exists());
    assert!(!f.root.join("dist").exists());
    let candidates = f.project.map_dir().join("data/delivery-candidates");
    let version = fs::read_dir(candidates)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let candidate = fs::read_dir(version)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let saved: Value =
        serde_json::from_slice(&fs::read(candidate.join("gift-repair-plan.json")).unwrap())
            .unwrap();
    for key in ["schema_version", "source_native_decoded_crc32", "repairs"] {
        assert_eq!(saved[key], plan[key]);
    }
    let report: Value =
        serde_json::from_slice(&fs::read(candidate.join("gift-repair.json")).unwrap()).unwrap();
    assert!(report["changes"].is_array());
    let review: Value = serde_json::from_slice(
        &fs::read(candidate.join("delivery-acceptance.pending.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(review["status"], "pending");
    assert!(
        review["items"]
            .as_object()
            .unwrap()
            .values()
            .all(|item| item["status"] == "pending")
    );
    assert!(candidate.join("source/layout.json").is_file());
    let manual = b"{\"reviewer_draft\":true}";
    fs::write(candidate.join("gift-repair.json"), manual).unwrap();
    let protected = f.protected();
    let error = f.run(&plan);
    assert!(
        error.contains("refusing to overwrite changed gift review evidence"),
        "{error}"
    );
    assert_eq!(
        fs::read(candidate.join("gift-repair.json")).unwrap(),
        manual
    );
    for (path, before) in protected {
        assert_eq!(fs::read(path).unwrap(), before);
    }
    assert!(!f.root.join("assets").exists());
    assert!(!f.root.join("dist").exists());
}
