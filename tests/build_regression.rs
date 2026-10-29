use ra3maps::{
    formats::map::MapFile,
    formats::refpack,
    pipeline::build::{self, Options},
    pipeline::project::{self, Project},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
mod support;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ra3maps-test-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &dst.join(e.file_name()));
        } else {
            fs::copy(e.path(), dst.join(e.file_name())).unwrap();
        }
    }
}
// Preserve the original keys of immutable migration fixtures.
fn historical_id(id: &str) -> &str {
    match id {
        "陕西" => "叠岭连原",
        "广西" => "夹川汇潮",
        "广东" => "岭尽生潮",
        "海南" => "环海相望",
        other => other,
    }
}
// Snapshot filenames and map_id values retain the published names they recorded.
fn fixture_title(id: &str) -> &str {
    match id {
        "陕西" => "千壑横原",
        "广西" => "碧嶂回澜",
        "广东" => "千流叩海",
        "海南" => "翠屿浮光",
        other => other,
    }
}
fn fixture_root(id: &str) -> Temp {
    let temp = Temp::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_tree(&source.join("templates"), &temp.0.join("templates"));
    let dir = temp.0.join("maps").join(id);
    fs::create_dir_all(&dir).unwrap();
    for f in ["config.json", "version.json"] {
        fs::copy(source.join("maps").join(id).join(f), dir.join(f)).unwrap();
    }
    fs::write(temp.0.join("maps/.default"), id).unwrap();
    // Historical migration baselines intentionally retain their original skin.
    // The default accepted profile is exercised separately below.
    let config = dir.join("config.json");
    let mut cfg: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    support::copy_declared_inputs(&source.join("maps").join(id), &dir, &cfg);
    cfg["TEXTURE_PROFILE"] = Value::String("legacy".into());
    cfg["RAMP_PROFILE"] = Value::String("legacy".into());
    // These fixtures intentionally reproduce the immutable pre-authoring blueprint.
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    // This immutable blueprint predates the current expansion allocation.
    cfg.as_object_mut().unwrap().remove("RESOURCE_DISTRIBUTION");
    cfg.as_object_mut().unwrap().remove("RAMP_CLEARANCE");
    cfg["RAMP_CLEARANCE_REFERENCE"] = "ramp-clearance-reference.json".into();
    cfg.as_object_mut().unwrap().remove("SPAWN_PLACEMENT");
    cfg["SPAWN_PLACEMENT_REFERENCE"] = "spawn-placement-reference.json".into();
    fs::copy(
        source
            .join("tests/fixtures")
            .join(format!("spawn-placement-{id}.json")),
        dir.join("spawn-placement-reference.json"),
    )
    .unwrap();
    let mut reference: Value = serde_json::from_slice(
        &fs::read(
            source
                .join("tests/fixtures")
                .join(format!("ramp-clearance-{}.json", fixture_title(id))),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(reference["map_id"], fixture_title(id));
    reference["map_id"] = id.into();
    project::write_json(&dir.join("ramp-clearance-reference.json"), &reference).unwrap();
    project::write_json(&config, &cfg).unwrap();
    fs::write(temp.0.join("Cargo.toml"), "# isolated test root").unwrap();
    temp
}

// Undo only the intentional heading overlay when comparing frozen migration
// scripts. Every other byte of each production create action must still match.
fn normalized_facing(p: &Project, nodes: &mut [ra3maps::gameplay::scripts::Node]) {
    use ra3maps::{
        formats::binary::Reader,
        gameplay::scripts::{self, Node},
    };
    fn selected(n: &Node) -> bool {
        n.command.as_deref() == Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION")
            && scripts::string_arguments(n).unwrap().iter().any(|(t, s)| {
                *t == 15
                    && ([
                        "barracks",
                        "warfactory",
                        "airfield",
                        "navalyard",
                        "powerplant",
                    ]
                    .iter()
                    .any(|suffix| s.to_ascii_lowercase().ends_with(suffix))
                        || s.to_ascii_lowercase().contains("superweapon"))
            })
    }
    fn collect(nodes: &[Node], out: &mut Vec<Node>) {
        for n in nodes {
            if selected(n) {
                out.push(n.clone());
            }
            if let Some(c) = &n.children {
                collect(c, out);
            }
        }
    }
    fn offset(n: &Node) -> usize {
        let b = scripts::unhex(&n.body).unwrap();
        let mut r = Reader::new(&b);
        r.take(8).unwrap();
        let count = r.u32().unwrap();
        for _ in 0..count {
            let t = r.u32().unwrap();
            r.i32().unwrap();
            let offset = r.pos;
            r.f32().unwrap();
            r.s16().unwrap();
            if t == 17 {
                return offset;
            }
        }
        panic!("missing production angle")
    }
    fn restore(nodes: &mut [Node], originals: &[Node]) {
        for n in nodes {
            if let Some(c) = &mut n.children {
                restore(c, originals);
                continue;
            }
            if !selected(n) {
                continue;
            }
            let args = scripts::string_arguments(n).unwrap();
            let original = originals
                .iter()
                .find(|x| scripts::string_arguments(x).unwrap() == args)
                .expect("unknown production create action");
            let mut actual = scripts::unhex(&n.body).unwrap();
            let expected = scripts::unhex(&original.body).unwrap();
            let o = offset(n);
            let e = offset(original);
            actual[o..o + 4].copy_from_slice(&expected[e..e + 4]);
            assert_eq!(actual, expected, "only heading may change");
            n.body = scripts::hex(&actual);
        }
    }
    let mut originals = vec![];
    collect(
        &ra3maps::gameplay::scripts::load_rules(p).unwrap().scripts,
        &mut originals,
    );
    restore(nodes, &originals);
}

#[test]
fn ordinary_build_uses_the_accepted_profile_without_changing_gameplay() {
    use ra3maps::{
        formats::map::{BlendTiles, HeightMap},
        terrain::texture_polish,
    };
    let id = "广西";
    let temp = fixture_root(id);
    let p = Project::load(&temp.0, id).unwrap();
    build::run(&temp.0, id, &Options::default()).unwrap();
    let path = p.assets.join(format!("{}.map", p.name));
    let legacy = MapFile::load(&path).unwrap();
    // The standalone entry and the automatic build must share the same rules.
    let mut expected = legacy.clone();
    texture_polish::apply(&p, &mut expected, &p.name).unwrap();
    ra3maps::resources::generate(&mut expected, &p).unwrap();
    ra3maps::resources::terrain_package::compact(&mut expected).unwrap();
    let config = p.map_dir().join("config.json");
    let mut cfg: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    cfg.as_object_mut().unwrap().remove("TEXTURE_PROFILE");
    project::write_json(&config, &cfg).unwrap();
    let original_version = fs::read(project::version_path(&temp.0, id)).unwrap();
    build::run(&temp.0, id, &Options::default()).unwrap();
    let actual = MapFile::load(&path).unwrap();
    assert_eq!(actual.table, expected.table);
    for chunk in &expected.chunks {
        let name = &expected.table[&chunk.name_idx];
        if name != "WorldInfo" {
            assert!(
                actual.get(name).unwrap() == chunk,
                "build and explicit polish differ in {name}"
            );
        }
    }
    // Build finalization synchronizes the newly appended stock resource names.
    let expected_world = ra3maps::formats::map::read_props(
        &mut ra3maps::formats::binary::Reader::new(&expected.get("WorldInfo").unwrap().payload),
        &expected.table,
    )
    .unwrap();
    let actual_world = ra3maps::formats::map::read_props(
        &mut ra3maps::formats::binary::Reader::new(&actual.get("WorldInfo").unwrap().payload),
        &actual.table,
    )
    .unwrap();
    for prop in &expected_world {
        if prop.key != "terrainTextureStrings" {
            assert!(
                actual_world.contains(prop),
                "unrelated WorldInfo property changed: {}",
                prop.key
            );
        }
    }
    let tokens = ra3maps::formats::map::prop(&actual_world, "terrainTextureStrings")
        .unwrap()
        .text()
        .unwrap();
    assert!(tokens.contains("TRock_Iceland02;TRock_Iceland02_nrm;"));
    assert!(tokens.contains("TRock_Yucatan03;TRock_Yucatan03_nrm;"));
    for chunk in &legacy.chunks {
        let name = &legacy.table[&chunk.name_idx];
        if !["BlendTileData", "WorldInfo", "AssetList"].contains(&name.as_str()) {
            assert_eq!(actual.get(name).unwrap(), chunk, "{name}");
        }
    }
    let hm = HeightMap::parse(actual.get("HeightMapData").unwrap()).unwrap();
    let n = hm.width;
    let masks = 4 + 8 * n * n..4 + 8 * n * n + 8 * n * n.div_ceil(8) + n * n;
    assert_eq!(
        legacy.get("BlendTileData").unwrap().payload[masks.clone()],
        actual.get("BlendTileData").unwrap().payload[masks]
    );
    let old = BlendTiles::parse(legacy.get("BlendTileData").unwrap(), n, n).unwrap();
    let new = BlendTiles::parse(actual.get("BlendTileData").unwrap(), n, n).unwrap();
    assert!(new.textures.starts_with(&old.textures));
    for primary in ra3maps::terrain::materials::PRIMARY {
        assert!(new.textures.iter().any(|name| name == primary));
    }
    assert_ne!(old.tiles, new.tiles);
    let report: Value =
        serde_json::from_slice(&fs::read(p.assets.join("texture-report.json")).unwrap()).unwrap();
    assert_eq!(report["profile"], texture_polish::PROFILE);
    assert_eq!(report["reference_version"], "T2.2.9");
    assert_eq!(
        report["palette_revision"],
        ra3maps::terrain::materials::REVISION
    );
    let xml = fs::read_to_string(p.assets.join("map.xml")).unwrap();
    // The directory/CLI rename must retain the original resource namespace.
    assert!(xml.contains(&format!("id=\"ra3maps_ea7772f9_{}\"", p.version)));
    assert!(xml.contains(&format!("DisplayName=\"{}\"", p.name)));
    assert!(xml.contains(&format!(
        "Description=\"{}\"",
        p.string("MAP_DESCRIPTION", &p.group)
    )));
    if !p.string("SERIES_NAME", "").is_empty() {
        let strings = fs::read_to_string(p.assets.join("map.str")).unwrap();
        assert!(strings.contains("Series:Name\n\"九州大地\""));
        assert!(strings.contains(&format!("Map:DisplayName\n\"{}\"", p.name)));
        assert!(strings.contains(p.string("MAP_DESCRIPTION", "")));
    }
    for name in report["excluded_patterned_grass"].as_array().unwrap() {
        assert!(report["after"].get(name.as_str().unwrap()).is_none());
    }
    assert_eq!(
        original_version,
        fs::read(project::version_path(&temp.0, id)).unwrap()
    );
}

#[test]
fn all_four_native_builds_match_the_checked_map_baselines_without_python() {
    let mut baseline: Value =
        serde_json::from_str(include_str!("fixtures/python-baseline.json")).unwrap();
    // Retain the migration baseline and historical fixtures; T0.4.0 explicitly
    // removes cash displays, forced attacks, and scripted team/seat assignment.
    baseline["叠岭连原"] =
        serde_json::from_str(include_str!("fixtures/dieling-T0.4.0-baseline.json")).unwrap();
    let shared: Value =
        serde_json::from_str(include_str!("fixtures/shared-T0.5.1-baseline.json")).unwrap();
    for (id, expected) in shared.as_object().unwrap() {
        baseline[id] = expected.clone();
    }
    for id in ["陕西", "广西", "广东", "海南"] {
        let temp = fixture_root(id);
        let before = fs::read(project::version_path(&temp.0, id)).unwrap();
        // No shell, interpreter or tool on PATH participates in a native build.
        let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
            .args(["build", "--root"])
            .arg(&temp.0)
            .args(["--map", id])
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{id}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let p = Project::load(&temp.0, id).unwrap();
        assert_eq!(p.assets, temp.0.join("assets").join(id).join(&p.name));
        assert_eq!(p.dist, temp.0.join("dist").join(id).join(&p.name));
        assert_eq!(p.release_assets(&p.name).unwrap(), p.assets);
        assert_eq!(
            build::zip_path(&p),
            temp.0.join("dist").join(id).join(format!("{}.zip", p.name))
        );
        assert!(!temp.0.join("assets").join(&p.group).exists());
        assert!(!temp.0.join("dist").join(&p.group).exists());
        assert!(!temp.0.join("assets").join(id).join(&p.version).exists());
        assert!(!temp.0.join("dist").join(id).join(&p.version).exists());
        assert_eq!(
            fs::read(p.path("layers.npz").unwrap()).unwrap(),
            fs::read(p.path(p.string("LAYERS", "layers.npz")).unwrap()).unwrap()
        );
        let bytes = fs::read(p.assets.join(format!("{}.map", p.name))).unwrap();
        let plain = refpack::unpack(&bytes).unwrap();
        let map = MapFile::decode(&plain).unwrap();
        ra3maps::gameplay::production_facing::validate(&p, &map).unwrap();
        let facing: Value =
            serde_json::from_slice(&fs::read(p.assets.join("production-facing.json")).unwrap())
                .unwrap();
        let actions = facing["actions"].as_array().unwrap();
        assert!(!actions.is_empty());
        for action in actions {
            let degrees = action["degrees"].as_i64().unwrap();
            assert!([45, 135, -45, -135].contains(&degrees));
            assert_eq!(
                action["radians"].as_f64().unwrap() as f32,
                (degrees as f32).to_radians()
            );
        }
        if id != "陕西" {
            for kind in ["powerplant", "superweapon"] {
                assert!(actions.iter().any(|a| {
                    a["kind"]
                        .as_str()
                        .unwrap()
                        .to_ascii_lowercase()
                        .contains(kind)
                }));
            }
        }
        assert!(
            actions
                .iter()
                .any(|a| a["waypoint"].as_str().unwrap().starts_with("gift-alt-"))
        );
        for seat in 1..=8 {
            assert!(actions.iter().any(|a| a["seat"] == seat));
        }
        if id == "陕西" {
            let scripts = ra3maps::gameplay::scripts::parse_nodes(
                &map.get("PlayerScriptsList").unwrap().payload,
                &map.table,
            )
            .unwrap();
            ra3maps::gameplay::gift_clearance::validate(&p, &map).unwrap();
            let scripts = ra3maps::gameplay::gift_clearance::authored_scripts(&scripts).unwrap();
            let scripts = ra3maps::gameplay::periodic_attack::authored_scripts(&scripts).unwrap();
            let names = ra3maps::gameplay::scripts::script_names(&scripts).unwrap();
            assert_eq!(names.len(), 191);
            assert!(names.iter().all(|n| {
                !["cash-", "atk-", "side-"]
                    .iter()
                    .any(|prefix| n.starts_with(prefix))
            }));
            let mut teams =
                ra3maps::formats::binary::Reader::new(&map.get("Teams").unwrap().payload);
            for _ in 0..teams.u32().unwrap() {
                let props = ra3maps::formats::map::read_props(&mut teams, &map.table).unwrap();
                let name = ra3maps::formats::map::prop(&props, "teamName")
                    .and_then(ra3maps::formats::map::Value::text)
                    .unwrap();
                assert!(!name.starts_with("team_atk_"));
            }
            teams.finish().unwrap();
        } else {
            verify_shared_rules_and_layout(&p, &map);
        }
        let expected = &baseline[historical_id(id)];
        for chunk in &map.chunks {
            let name = &map.table[&chunk.name_idx];
            if name == "AssetList" {
                assert_eq!(chunk.payload, ra3maps::resources::manifest(&map).unwrap());
                continue;
            }
            // Factories/shipyards may move to satisfy the new water/land rule.
            // Restoring only their positions must reproduce the historical bytes.
            let normalized;
            let payload = if name == "ObjectsList" {
                normalized = normalized_production_objects(&p, &map);
                &normalized
            } else if name == "PlayerScriptsList" {
                let nodes =
                    ra3maps::gameplay::scripts::parse_nodes(&chunk.payload, &map.table).unwrap();
                let authored = ra3maps::gameplay::gift_clearance::authored_scripts(&nodes).unwrap();
                let mut authored =
                    ra3maps::gameplay::periodic_attack::authored_scripts(&authored).unwrap();
                normalized_facing(&p, &mut authored);
                normalized =
                    ra3maps::gameplay::scripts::encode_nodes(&authored, &mut map.clone()).unwrap();
                &normalized
            } else if name == "TriggerAreas" {
                let mut original = map.clone();
                ra3maps::gameplay::gift_clearance::authored_areas(&mut original).unwrap();
                normalized = original.get("TriggerAreas").unwrap().payload.clone();
                &normalized
            } else if name == "Teams" {
                let mut original = map.clone();
                ra3maps::gameplay::periodic_attack::authored_teams(&mut original).unwrap();
                normalized = original.get("Teams").unwrap().payload.clone();
                &normalized
            } else {
                &chunk.payload
            };
            assert_eq!(
                payload.len() as u64,
                expected["chunks"][name]["length"].as_u64().unwrap(),
                "{id}/{name} length"
            );
            assert_eq!(
                crc32fast::hash(payload) as u64,
                expected["chunks"][name]["crc32"].as_u64().unwrap(),
                "{id}/{name} bytes"
            );
        }
        // The chunk baselines above remain byte exact. The old container CRCs
        // include the invalid insertion-order string table; require the actual
        // WorldBuilder order and deterministic compression instead.
        map.validate_release().unwrap();
        assert_eq!(map.encode().unwrap(), plain, "{id} canonical container");
        assert_eq!(
            refpack::pack(&plain).unwrap(),
            bytes,
            "{id} deterministic EAR"
        );
        let zip =
            ra3maps::formats::archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
        assert_eq!(zip.len(), 5);
        assert!(zip.contains_key(&format!("{}/map.xml", p.name)));
        assert!(zip.contains_key(&format!("{}/overrides.xml", p.name)));
        assert!(zip.contains_key(&format!("{}/map.str", p.name)));
        assert_eq!(zip[&format!("{}/{}.map", p.name, p.name)], bytes);
        assert!(p.assets.join("preview.png").is_file());
        verify_layout_preview(&p);
        assert!(p.assets.join("README.md").is_file());
        assert!(p.assets.join("wb/heightmap.xyz").is_file());
        assert!(p.assets.join("wb/chai_objects.txt").is_file());
        assert_eq!(
            fs::read(project::version_path(&temp.0, id)).unwrap(),
            before
        );
        // Repacking preserves deterministic ZIP bytes and installation stays isolated.
        let zip_before = fs::read(build::zip_path(&p)).unwrap();
        let appdata = temp.0.join("profile");
        let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
            .args(["dist", "--root"])
            .arg(&temp.0)
            .args(["--map", id, "--install"])
            .env("APPDATA", &appdata)
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(build::zip_path(&p)).unwrap(), zip_before);
        assert_eq!(
            fs::read(
                appdata
                    .join("Red Alert 3/Maps")
                    .join(&p.name)
                    .join(format!("{}.map", p.name))
            )
            .unwrap(),
            bytes
        );
    }
}

fn verify_layout_preview(p: &Project) {
    let layout: Value =
        serde_json::from_slice(&fs::read(p.source().join("layout.json")).unwrap()).unwrap();
    let index: Value =
        serde_json::from_slice(&fs::read(p.assets.join("layout-index.json")).unwrap()).unwrap();
    let objects = layout["objects"].as_array().unwrap();
    let ramps = layout["ramps"].as_array().unwrap();
    let entries = index["entries"].as_array().unwrap();
    assert_eq!(index["map_id"], p.id);
    assert_eq!(index["version"], p.version);
    assert_eq!(entries.len(), objects.len() + ramps.len());
    // Every authored object, including every individual tree and rock, must
    // round-trip to the exact source entry. Ramps are not game objects.
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry["id"], i + 1);
        let pointer = entry["source_pointer"].as_str().unwrap();
        let source = layout.pointer(pointer).unwrap();
        if i < objects.len() {
            assert_eq!(pointer, format!("/objects/{i}"));
            assert_eq!(entry["cell"], source["cell"]);
            assert_eq!(entry["object_type"], source["type_name"]);
            for position in entry["position_pointers"].as_array().unwrap() {
                assert_eq!(
                    layout.pointer(position.as_str().unwrap()).unwrap(),
                    &source["cell"]
                );
            }
        } else {
            assert_eq!(pointer, format!("/ramps/{}", i - objects.len()));
            assert_eq!(entry["category"], "斜坡");
            assert_eq!(entry["cell"], serde_json::json!([source["x"], source["y"]]));
            for (axis, position) in entry["position_pointers"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                assert_eq!(
                    layout.pointer(position.as_str().unwrap()).unwrap(),
                    &entry["cell"][axis]
                );
            }
        }
    }
    let image = fs::read(p.assets.join("preview--layout.png")).unwrap();
    assert_eq!(&image[..8], b"\x89PNG\r\n\x1a\n");
    let side = (p.usize("GRID", 0) * 4).clamp(2048, 4096) as u32;
    assert_eq!(
        u32::from_be_bytes(image[16..20].try_into().unwrap()),
        side + 400
    );
    assert_eq!(
        u32::from_be_bytes(image[20..24].try_into().unwrap()),
        side + 160
    );
}

fn verify_shared_rules_and_layout(p: &Project, map: &MapFile) {
    use ra3maps::{
        formats::map::{self, BlendTiles, HeightMap},
        gameplay::script_spawns,
        gameplay::scripts,
        terrain::Layout,
    };
    let rules = scripts::load_rules(p).unwrap();
    assert_eq!(p.string("RULESET", ""), "common-T0.5.1");
    let nodes =
        scripts::parse_nodes(&map.get("PlayerScriptsList").unwrap().payload, &map.table).unwrap();
    ra3maps::gameplay::gift_clearance::validate(p, map).unwrap();
    let mut nodes = ra3maps::gameplay::gift_clearance::authored_scripts(&nodes).unwrap();
    normalized_facing(p, &mut nodes);
    let names = scripts::script_names(&nodes).unwrap();
    assert_eq!(names.len(), 224);
    assert!(names.iter().all(|n| {
        !["cash-", "atk-", "side-"]
            .iter()
            .any(|prefix| n.starts_with(prefix))
    }));
    // Includes all condition/action arguments, group order, flags and intervals.
    assert_eq!(
        serde_json::to_value(&nodes).unwrap(),
        serde_json::to_value(&rules.scripts).unwrap()
    );
    assert_eq!(rules.spawn_waypoints.len(), 31);
    let objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    let layout = Layout::load(p).unwrap();
    assert_eq!(
        objects
            .iter()
            .filter(|o| !ra3maps::gameplay::gift_clearance::is_alternate(o))
            .count(),
        layout.objects.len() + 31
    );
    let originals = layout
        .objects
        .iter()
        .map(|o| o.object(p))
        .collect::<Vec<_>>();
    assert_eq!(
        normalized_production_objects(p, map)[..map::write_objects(&originals, map).unwrap().len()],
        map::write_objects(&originals, map).unwrap()
    );
    // Keep the migration baseline to prove terrain, textures and masks are unchanged.
    let old: Value = serde_json::from_str(include_str!("fixtures/python-baseline.json")).unwrap();
    for chunk in ["HeightMapData", "BlendTileData"] {
        assert_eq!(
            crc32fast::hash(&map.get(chunk).unwrap().payload) as u64,
            old[historical_id(&p.id)]["chunks"][chunk]["crc32"]
                .as_u64()
                .unwrap(),
            "{}/{chunk}",
            p.id
        );
    }
    let hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
    let bt = BlendTiles::parse(map.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    // The final-map validator also rejects regressions at a factory footprint edge.
    let factory = objects
        .iter()
        .find(|o| {
            map::prop(&o.props, "waypointName").and_then(map::Value::text) == Some("dual-p1-wf")
        })
        .unwrap();
    let x = (factory.position[0] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.)) as usize;
    let y = (factory.position[1] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.)) as usize;
    let mut flooded = hm.clone();
    flooded.elevations[y * hm.width + x + 6] =
        ((p.number("WATER_Z", 200.) - 10.) / p.number("Z_SCALE", 0.0390625)) as u16;
    assert!(
        ra3maps::gameplay::production_spawns::validate(p, map, &flooded, &bt, &objects)
            .unwrap_err()
            .to_string()
            .contains("requires flat land")
    );
    // A missing binding or an obstruction must fail validation, not silently ship.
    let mut missing = objects.clone();
    missing
        .retain(|o| map::prop(&o.props, "waypointName").and_then(map::Value::text) != Some("SWJ1"));
    assert!(
        script_spawns::validate(p, map, &hm, &bt, &missing)
            .unwrap_err()
            .to_string()
            .contains("missing shared waypoint SWJ1")
    );
    let waypoint = objects
        .iter()
        .find(|o| map::prop(&o.props, "waypointName").and_then(map::Value::text) == Some("SWJ1"))
        .unwrap();
    let x =
        (waypoint.position[0] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.)) as usize;
    let y =
        (waypoint.position[1] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.)) as usize;
    let mut obstructed = bt.clone();
    obstructed.buildability[y * hm.width + x] = true;
    assert!(
        script_spawns::validate(p, map, &hm, &obstructed, &objects)
            .unwrap_err()
            .to_string()
            .contains("footprint is obstructed: SWJ1")
    );
    let text = fs::read_to_string(p.assets.join("map.str")).unwrap();
    assert!(text.contains("Info:GameStart"));
}

fn normalized_production_objects(p: &Project, map: &MapFile) -> Vec<u8> {
    use ra3maps::{
        formats::map,
        gameplay::script_spawns,
        terrain::surface::Surface,
        terrain::{Layout, Terrain},
    };
    let mut objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
    objects.retain(|o| !ra3maps::gameplay::gift_clearance::is_alternate(o));
    let layout = Layout::load(p).unwrap();
    let mut t = Terrain::load(p).unwrap();
    t.layout = layout.clone();
    let s = Surface::generate(&t, p).unwrap();
    let mut originals = layout
        .objects
        .iter()
        .map(|o| o.object(p))
        .collect::<Vec<_>>();
    let mut temporary = p.clone();
    temporary.assets = p.assets.join("baseline-bindings");
    std::fs::create_dir_all(&temporary.assets).unwrap();
    script_spawns::append(&temporary, &t, &s, &mut originals).unwrap();
    assert_eq!(objects.len(), originals.len());
    for (o, original) in objects.iter_mut().zip(&originals) {
        if o.type_name.contains("Waypoint")
            || o.type_name.ends_with("NavalYard")
            || o.type_name == "ShipyardTechStructure"
        {
            o.position = original.position;
        }
    }
    map::write_objects(&objects, map).unwrap()
}

#[test]
fn production_release_repairs_water_factory_and_preserves_all_other_map_chunks() {
    use ra3maps::{
        formats::archive, formats::map, gameplay::production_spawns, pipeline::release,
        terrain::Layout,
    };
    let temp = fixture_root("广西");
    build::run(&temp.0, "广西", &Options::default()).unwrap();
    let p = Project::load(&temp.0, "广西").unwrap();
    let good = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    let mut bad = good.clone();
    let mut objects = map::objects(bad.get("ObjectsList").unwrap(), &bad.table).unwrap();
    let original = Layout::load(&p)
        .unwrap()
        .objects
        .into_iter()
        .find(|o| {
            map::prop(&o.props, "waypointName").and_then(map::Value::text) == Some("dual-p6-wf")
        })
        .unwrap();
    let o = objects
        .iter_mut()
        .find(|o| {
            map::prop(&o.props, "waypointName").and_then(map::Value::text) == Some("dual-p6-wf")
        })
        .unwrap();
    o.position = original.object(&p).position;
    bad.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &bad).unwrap();
    let mut files = archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
    files.insert(
        format!("{}/{}.map", p.name, p.name),
        refpack::pack(&bad.encode().unwrap()).unwrap(),
    );
    let xml_key = format!("{}/map.xml", p.name);
    let xml =
        production_spawns::update_xml(std::str::from_utf8(&files[&xml_key]).unwrap(), &good, &bad)
            .unwrap();
    // The source releases exported by other tools may put whitespace before />.
    files.insert(xml_key, xml.replace("/>", " />").into_bytes());
    let source = temp.0.join("water-factory.zip");
    fs::write(
        &source,
        archive::zip(&files.into_iter().collect::<Vec<_>>()).unwrap(),
    )
    .unwrap();
    let released = release::repair_production(
        &temp.0,
        "广西",
        &source,
        &Options {
            bump: true,
            note: "修复赠送建筑落点".into(),
            ..Options::default()
        },
    )
    .unwrap();
    let repaired = MapFile::load(released.assets.join(format!("{}.map", released.name))).unwrap();
    ra3maps::gameplay::gift_clearance::validate(&released, &repaired).unwrap();
    // A terrain edit must reserve complete primary and fallback foundations,
    // including their corners, rather than only their waypoint markers.
    let mut terrain_edit = repaired.clone();
    ra3maps::terrain::ramps::apply(&released, &mut terrain_edit, &released.name).unwrap();
    ra3maps::gameplay::gift_clearance::validate(&released, &terrain_edit).unwrap();
    ra3maps::gameplay::production_facing::validate(&released, &terrain_edit).unwrap();
    for chunk in &good.chunks {
        let name = &good.table[&chunk.name_idx];
        if !["ObjectsList", "TriggerAreas", "PlayerScriptsList"].contains(&name.as_str()) {
            assert!(
                repaired.get(name).unwrap() == chunk,
                "unrelated chunk changed: {name}"
            );
        }
    }
    let report: Value =
        serde_json::from_slice(&fs::read(released.assets.join("production-spawns.json")).unwrap())
            .unwrap();
    assert_eq!(
        report["placements"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["moved"] == true)
            .count(),
        1
    );
    let strings = fs::read_to_string(released.assets.join("map.str")).unwrap();
    assert!(strings.contains(&format!("版本 {}", released.version)));
    let xml = fs::read_to_string(released.assets.join("map.xml")).unwrap();
    assert!(xml.contains(&format!("DisplayName=\"{}\"", released.name)));
    assert!(xml.contains(&format!("id=\"ra3maps_ea7772f9_{}\"", released.version)));
}
#[test]
fn failed_bump_restores_exact_version_bytes_and_only_removes_new_paths() {
    let temp = fixture_root("广西");
    let version = project::version_path(&temp.0, "广西");
    let original = fs::read(&version).unwrap();
    let options = Options {
        bump: true,
        only: vec!["verify".into()],
        ..Options::default()
    };
    assert!(build::run(&temp.0, "广西", &options).is_err());
    assert_eq!(fs::read(&version).unwrap(), original);
    let st = project::bump(
        &project::load_version(&temp.0, "广西").unwrap(),
        false,
        1,
        "",
    )
    .unwrap();
    project::write_json(&version, &st).unwrap();
    let next = Project::load(&temp.0, "广西").unwrap();
    project::atomic_write(&version, &original).unwrap();
    assert!(!next.assets.exists());
    fs::create_dir_all(&next.assets).unwrap();
    fs::write(next.assets.join("keep.txt"), "existing artifact").unwrap();
    assert!(build::run(&temp.0, "广西", &options).is_err());
    assert_eq!(
        fs::read(next.assets.join("keep.txt")).unwrap(),
        b"existing artifact"
    );
    assert_eq!(fs::read(&version).unwrap(), original);
}

#[test]
fn periodic_attack_update_preserves_finished_map_and_installs_exact_pairs() {
    use ra3maps::{formats::map, gameplay::periodic_attack, gameplay::scripts, pipeline::release};
    let temp = fixture_root("陕西");
    let rules_path = temp.0.join("templates/rules/千壑横原.json");
    let mut enabled: Value = serde_json::from_slice(&fs::read(&rules_path).unwrap()).unwrap();
    enabled["periodic_attack"] = serde_json::json!({
        "interval_seconds": 60,
        "minimum_units": 20,
        "pairs": [[1,5],[2,7],[3,8],[4,6]],
        "unit_types": ["SovietAntiVehicleVehicleTech1"],
        "diagnostics": true
    });
    let mut legacy = enabled.clone();
    legacy.as_object_mut().unwrap().remove("periodic_attack");
    project::write_json(&rules_path, &legacy).unwrap();
    build::run(&temp.0, "陕西", &Options::default()).unwrap();
    let original = Project::load(&temp.0, "陕西").unwrap();
    let before = MapFile::load(original.assets.join(format!("{}.map", original.name))).unwrap();
    project::write_json(&rules_path, &enabled).unwrap();
    let p = release::update_scripts(
        &temp.0,
        "陕西",
        &build::zip_path(&original),
        &Options {
            bump: true,
            note: "每分钟至少20部队按位置进攻".into(),
            ..Options::default()
        },
    )
    .unwrap();
    let after = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    periodic_attack::validate(&p, &after).unwrap();
    for chunk in &before.chunks {
        let name = &before.table[&chunk.name_idx];
        if !["Teams", "PlayerScriptsList"].contains(&name.as_str()) {
            assert!(
                after.get(name).unwrap() == chunk,
                "unrelated chunk {name} changed"
            );
        }
    }
    let nodes = scripts::parse_nodes(
        &after.get("PlayerScriptsList").unwrap().payload,
        &after.table,
    )
    .unwrap();
    let restored = periodic_attack::authored_scripts(&nodes).unwrap();
    let old = scripts::parse_nodes(
        &before.get("PlayerScriptsList").unwrap().payload,
        &before.table,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    let mut stripped = after.clone();
    periodic_attack::authored_teams(&mut stripped).unwrap();
    assert!(stripped.get("Teams").unwrap().payload == before.get("Teams").unwrap().payload);
    let report: Value =
        serde_json::from_slice(&fs::read(p.assets.join("periodic-attack.json")).unwrap()).unwrap();
    assert_eq!(
        report["targets"],
        serde_json::json!([5, 7, 8, 6, 1, 4, 2, 3])
    );
    assert_eq!(report["minimum_units"], 20);
    assert_eq!(report["comparison"], ">=");
    assert_eq!(report["human_player_check"], false);
    assert_eq!(report["attack_teams"], 0);
    assert_eq!(report["ai_retains_control"], true);
    assert_eq!(report["control_transfers"], false);
    assert_eq!(report["request_command"], "PLACE_AI_BEACON_LOCATION");
    assert_eq!(report["scripts_added"], 18);
    let names = scripts::script_names(&nodes).unwrap();
    assert_eq!(
        names.len(),
        scripts::script_names(&old).unwrap().len()
            + report["scripts_added"].as_u64().unwrap() as usize
    );
    let strings = fs::read_to_string(p.assets.join("map.str")).unwrap();
    assert!(strings.contains("COUNTER:minute-attack-round"));
    assert!(strings.contains("P7 AI进攻指令累计"));
    assert!(!strings.contains("发令编队"));
    let mut strings_again = strings.clone();
    periodic_attack::append_strings(&p, &mut strings_again).unwrap();
    assert_eq!(strings, strings_again);
    // Applying the same map-specific rules again is a byte-preserving operation.
    let mut repeated = after.clone();
    periodic_attack::apply(&p, &mut repeated).unwrap();
    assert!(repeated.encode().unwrap() == after.encode().unwrap());
    assert_eq!(
        map::objects(after.get("ObjectsList").unwrap(), &after.table)
            .unwrap()
            .len(),
        map::objects(before.get("ObjectsList").unwrap(), &before.table)
            .unwrap()
            .len()
    );

    // Removing the per-map configuration must also remove the live module from
    // an existing package, without touching any other script or map chunk.
    project::write_json(&rules_path, &legacy).unwrap();
    assert!(periodic_attack::validate(&p, &after).is_err());
    let removed = release::update_scripts(
        &temp.0,
        "陕西",
        &build::zip_path(&p),
        &Options {
            bump: true,
            note: "取消分钟进攻".into(),
            ..Options::default()
        },
    )
    .unwrap();
    let clean = MapFile::load(removed.assets.join(format!("{}.map", removed.name))).unwrap();
    periodic_attack::validate(&removed, &clean).unwrap();
    let clean_nodes = scripts::parse_nodes(
        &clean.get("PlayerScriptsList").unwrap().payload,
        &clean.table,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(clean_nodes).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    for chunk in &after.chunks {
        let name = &after.table[&chunk.name_idx];
        if name != "PlayerScriptsList" {
            assert_eq!(
                clean.get(name).unwrap(),
                chunk,
                "unrelated chunk {name} changed"
            );
        }
    }
    for file in ["map.xml", "map.str"] {
        let text = fs::read_to_string(removed.assets.join(file)).unwrap();
        assert!(
            !text.contains("minute-attack-"),
            "{file} retains removed module"
        );
    }
    let report: Value =
        serde_json::from_slice(&fs::read(removed.assets.join("periodic-attack.json")).unwrap())
            .unwrap();
    assert_eq!(report["scripts_removed"], 18);
    assert_eq!(report["enabled"], false);
    assert!(
        fs::read_to_string(removed.assets.join("README.md"))
            .unwrap()
            .contains("已取消分钟进攻")
    );
    let mut repeated = clean.clone();
    assert!(
        periodic_attack::apply(&removed, &mut repeated)
            .unwrap()
            .is_none()
    );
    assert_eq!(repeated.encode().unwrap(), clean.encode().unwrap());
}
#[test]
fn invalid_stage_is_rejected_before_version_changes() {
    let temp = fixture_root("海南");
    let path = project::version_path(&temp.0, "海南");
    let original = fs::read(&path).unwrap();
    let options = Options {
        bump: true,
        only: vec!["typo".into()],
        ..Options::default()
    };
    assert!(build::run(&temp.0, "海南", &options).is_err());
    assert_eq!(fs::read(path).unwrap(), original);
}
#[test]
fn distribution_validates_all_files_before_touching_existing_package() {
    let temp = fixture_root("广西");
    let p = Project::load(&temp.0, "广西").unwrap();
    fs::create_dir_all(&p.dist).unwrap();
    fs::write(p.dist.join("keep.txt"), "existing package").unwrap();
    assert!(build::dist(&p, true).is_err());
    assert_eq!(
        fs::read(p.dist.join("keep.txt")).unwrap(),
        b"existing package"
    );
    assert!(build::zip_path(&p).ends_with(format!("{}.zip", p.name)));
}

#[test]
fn editing_layout_metadata_without_the_actual_objects_is_rejected() {
    let temp = fixture_root("广西");
    let p = Project::load(&temp.0, "广西").unwrap();
    let path = p.path(p.string("LAYOUT", "layout.json")).unwrap();
    let mut layout: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    layout["base_cells"][0][0] = serde_json::json!(200.);
    project::write_json(&path, &layout).unwrap();
    assert!(
        ra3maps::terrain::Terrain::generate(&p)
            .unwrap_err()
            .to_string()
            .contains("coordinates differ")
    );
}
