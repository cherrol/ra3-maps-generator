use ra3maps::{
    formats::{
        binary::Writer,
        map::{self, MapFile},
    },
    gameplay::{
        ore_facing,
        scripts::{self, Node},
    },
    pipeline::{
        build, coordinate_pipeline,
        project::{self, Project},
        release,
    },
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

mod support;

fn package() -> (Vec<(String, Vec<u8>)>, MapFile) {
    let (_, files) = support::current_package();
    let mut native =
        MapFile::load_bytes(&files.iter().find(|(n, _)| n.ends_with(".map")).unwrap().1).unwrap();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    assert!(objects.iter().any(|o| o.type_name == "OreNode"));
    for object in objects.iter_mut().filter(|o| o.type_name == "OreNode") {
        object.angle = 45_f32.to_radians();
    }
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    (files, native)
}

fn encoded(files: &[(String, Vec<u8>)], native: &MapFile) -> Vec<(String, Vec<u8>)> {
    files
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                if name.ends_with(".map") {
                    ra3maps::formats::refpack::pack(&native.encode().unwrap()).unwrap()
                } else {
                    bytes.clone()
                },
            )
        })
        .collect()
}

#[test]
fn illegal_native_ores_are_rejected_by_source_import_and_publication() {
    let (files, mut native) = package();
    release::validate_runtime_files(&encoded(&files, &native)).unwrap();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    let ore_id = objects
        .iter()
        .position(|o| o.type_name == "OreNode")
        .unwrap();
    for angle in [0_f32, 90., -78.75, 45.001] {
        objects[ore_id].angle = angle.to_radians();
        native.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &native).unwrap();
        let before = native.encode().unwrap();
        let report = ore_facing::report(&native).unwrap();
        assert_eq!(report["passed"], false);
        assert_eq!(report["failed_objects"], 1);
        assert_eq!(report["objects"][0]["object_id"], ore_id + 1);
        let output = encoded(&files, &native);
        assert!(release::validate_files(&output).is_err());
        assert!(
            release::validate_runtime_files(&output)
                .unwrap_err()
                .to_string()
                .contains("OreNode facing failed")
        );
        assert_eq!(native.encode().unwrap(), before);
    }
    for (i, object) in objects
        .iter_mut()
        .filter(|o| o.type_name == "OreNode")
        .enumerate()
    {
        object.angle = [45_f32, 135., -45., -135., 225., 315., 405.][i % 7].to_radians();
    }
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    release::validate_runtime_files(&encoded(&files, &native)).unwrap();
}

fn action(command: &str, arguments: &[(u32, &str, f32)], otherwise: bool) -> Node {
    let mut writer = Writer::new();
    writer.u32(467);
    writer.u32(3);
    writer.u32(arguments.len() as u32);
    for (kind, text, angle) in arguments {
        writer.u32(*kind);
        writer.i32(0);
        writer.f32(*angle);
        writer.s16(text).unwrap();
    }
    writer.u32(1);
    Node {
        asset: if otherwise {
            "ScriptActionFalse"
        } else {
            "ScriptAction"
        }
        .into(),
        version: 3,
        prefix: String::new(),
        body: scripts::hex(&writer.data),
        command: Some(command.into()),
        children: None,
    }
}

fn replace_scripts(native: &mut MapFile, nodes: Vec<Node>) {
    let container = Node {
        asset: "ScriptList".into(),
        version: 1,
        prefix: String::new(),
        body: String::new(),
        command: None,
        children: Some(nodes),
    };
    let bytes = scripts::encode_nodes(&[container], native).unwrap();
    native.get_mut("PlayerScriptsList").unwrap().payload = bytes;
}

#[test]
fn creation_and_named_turns_are_checked_in_all_branches_without_waypoint_inference() {
    let (_, mut native) = package();
    let create = action(
        "CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
        &[
            (14, "new-ore", 0.),
            (15, "OreNode", 0.),
            (17, "", 45_f32.to_radians()),
        ],
        false,
    );
    // The name resolves across branches even when the turn precedes creation.
    let turn = action(
        "UNIT_SET_ORIENTATION",
        &[(14, "new-ore", 0.), (17, "", 90_f32.to_radians())],
        true,
    );
    replace_scripts(&mut native, vec![turn, create.clone()]);
    let report = ore_facing::report(&native).unwrap();
    assert_eq!(report["checked_actions"], 2);
    assert_eq!(report["failed_actions"], 1);
    assert_eq!(report["script_actions"][0]["asset"], "ScriptActionFalse");
    assert!(ore_facing::validate(&native).is_err());
    let turn = action(
        "UNIT_SET_ORIENTATION",
        &[(14, "new-ore", 0.), (17, "", (-135_f32).to_radians())],
        true,
    );
    replace_scripts(&mut native, vec![turn, create]);
    ore_facing::validate(&native).unwrap();
    for arguments in [
        vec![(15, "OreNode", 0.)],
        vec![
            (15, "OreNode", 0.),
            (17, "runtime-variable", 45_f32.to_radians()),
        ],
    ] {
        replace_scripts(
            &mut native,
            vec![action(
                "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
                &arguments,
                false,
            )],
        );
        let report = ore_facing::report(&native).unwrap();
        assert_eq!(report["pending_actions"], 1);
        assert_eq!(report["passed"], false);
        assert!(ore_facing::validate(&native).is_err());
    }
    let create = action(
        "CREATE_NAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
        &[
            (14, "new-ore", 0.),
            (15, "OreNode", 0.),
            (17, "", 45_f32.to_radians()),
        ],
        false,
    );
    replace_scripts(
        &mut native,
        vec![
            create,
            action(
                "UNIT_ROTATE",
                &[(14, "new-ore", 0.), (17, "", 45_f32.to_radians())],
                false,
            ),
        ],
    );
    assert_eq!(ore_facing::report(&native).unwrap()["pending_actions"], 1);
}

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn distribution_and_adoption_reject_illegal_ores_before_writing_any_game_file() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let temp = Temp(std::env::temp_dir().join(format!(
        "ra3-ore-facing-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir_all(&temp.0).unwrap();
    let (files, mut native) = package();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    objects
        .iter_mut()
        .find(|o| o.type_name == "OreNode")
        .unwrap()
        .angle = 0.;
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    let name = files
        .iter()
        .find(|(n, _)| n.ends_with(".map"))
        .unwrap()
        .0
        .trim_end_matches(".map")
        .to_owned();
    let version = name
        .char_indices()
        .rev()
        .find_map(|(pos, ch)| {
            ((ch == 'T' || ch == 'V') && project::parse_version(&name[pos..]).is_ok())
                .then_some(name[pos..].to_owned())
        })
        .unwrap();
    let p = Project {
        root: temp.0.clone(),
        id: "ore-test".into(),
        cfg: json!({}),
        state: json!({}),
        version,
        group: "ore-test".into(),
        name: name.clone(),
        assets: temp.0.join("assets"),
        dist: temp.0.join("dist").join(&name),
    };
    fs::create_dir_all(&p.assets).unwrap();
    for (name, bytes) in encoded(&files, &native) {
        fs::write(p.assets.join(name), bytes).unwrap();
    }
    fs::create_dir_all(&p.dist).unwrap();
    fs::write(p.dist.join("preserve.txt"), b"existing release").unwrap();
    let source = p.map_dir().join("authoring/source.zip");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, b"existing frozen source").unwrap();
    assert!(
        build::dist(&p, true)
            .unwrap_err()
            .to_string()
            .contains("OreNode facing failed")
    );
    assert_eq!(
        fs::read(p.dist.join("preserve.txt")).unwrap(),
        b"existing release"
    );
    assert!(!build::zip_path(&p).exists());
    assert!(
        coordinate_pipeline::adopt(&p)
            .unwrap_err()
            .to_string()
            .contains("OreNode facing failed")
    );
    assert_eq!(fs::read(source).unwrap(), b"existing frozen source");
    ore_facing::verify(&p, &native).unwrap_err();
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(p.assets.join("ore-facing.json")).unwrap()).unwrap();
    assert_eq!(saved["passed"], false);
    assert_eq!(saved["failed_objects"], 1);
}
