//! Review and apply the user's explicit T0.0.2 spawn regions to the frozen map.
use ra3maps::{
    Result,
    coordinates::Frame,
    formats::{
        archive,
        map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    },
    gameplay::{gift_clearance, script_spawns},
    pipeline::project::{self, Project},
    terrain::{self, Layout, surface::Surface},
};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let original = Project::load(root, "河南")?;
    let dir = original.path("histories/T0.0.2")?;
    let plan: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("spawn-plan.json"))?)?;
    let package = archive::unzip(&fs::read(original.path("histories/T0.0.1/source.zip")?)?)?;
    let bytes = &package[&format!("{}.map", plan["source_name"].as_str().unwrap())];
    assert_eq!(
        crc32fast::hash(bytes),
        plan["source_crc32"].as_u64().unwrap() as u32
    );
    let mut native = MapFile::load_bytes(bytes)?;
    if std::env::args().any(|v| v == "--audit") {
        return audit(&original, &native, &dir, &plan);
    }
    let mut layout: Layout = serde_json::from_slice(&package["layout.json"])?;
    let before = layout.clone();
    let frame = Frame::from_project(&original);
    layout.base_cells = serde_json::from_value(plan["base_cells"].clone())?;
    let ores: Vec<Vec<[f64; 2]>> = serde_json::from_value(plan["near_ore_cells"].clone())?;
    let angles: Vec<Vec<f64>> = serde_json::from_value(plan["near_ore_angles"].clone())?;
    let mut ore_index = 0;
    let mut moved_trees = Vec::new();
    for o in &mut layout.objects {
        let name = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .unwrap_or("");
        if let Some(seat) = name
            .strip_prefix("Player_")
            .and_then(|n| n.split('_').next())
            .and_then(|n| n.parse::<usize>().ok())
        {
            o.cell = layout.base_cells[seat - 1];
        } else if o.type_name == "SovietConstructionYard" {
            o.cell = layout.base_cells[6];
        } else if o.type_name == "JapanConstructionYard" {
            o.cell = layout.base_cells[7];
        }
        if o.type_name == "OreNode" {
            if ore_index < 16 {
                o.cell = ores[ore_index / 2][ore_index % 2];
                o.angle = -angles[ore_index / 2][ore_index % 2] as f32;
            } else {
                for movement in plan["other_resource_moves"].as_array().unwrap() {
                    if o.cell == terrain::point(&movement[0])? {
                        o.cell = terrain::point(&movement[1])?;
                        break;
                    }
                }
            }
            ore_index += 1;
        }
        if o.type_name.contains("Tree") || o.type_name.contains("Bush") {
            for movement in plan["grove_moves"].as_array().unwrap() {
                let from = terrain::point(&movement[0])?;
                let to = terrain::point(&movement[1])?;
                if (o.cell[0] - from[0]).hypot(o.cell[1] - from[1]) < 25. {
                    let old = o.cell;
                    o.cell = [o.cell[0] + to[0] - from[0], o.cell[1] + to[1] - from[1]];
                    moved_trees.push((old, o.cell));
                    break;
                }
            }
        }
    }
    layout.ore_cells = layout
        .objects
        .iter()
        .filter(|o| o.type_name == "OreNode")
        .map(|o| o.cell)
        .collect();
    layout.ore_angles = layout
        .objects
        .iter()
        .filter(|o| o.type_name == "OreNode")
        .map(|o| -f64::from(o.angle))
        .collect();
    project::write_json(&dir.join("candidate-layout.json"), &layout)?;
    let mut p = original.clone();
    p.cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    p.cfg["LAYOUT"] = json!("histories/T0.0.2/candidate-layout.json");
    p.cfg["NATIVE_CELL_EDITS"] = plan["terrain_patch"].clone();
    for (i, b) in p.cfg["BASES"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        b[3] = json!(if i < 4 { "S" } else { "N" });
    }
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(native.get("BlendTileData")?, hm.width, hm.height)?;
    let mut patches: BTreeMap<(usize, usize), serde_json::Value> =
        serde_json::from_slice::<Vec<serde_json::Value>>(&fs::read(
            p.path(plan["terrain_patch"].as_str().unwrap())?,
        )?)?
        .into_iter()
        .map(|v| {
            (
                (
                    v["x"].as_u64().unwrap() as usize,
                    v["y"].as_u64().unwrap() as usize,
                ),
                v,
            )
        })
        .collect();
    for (old, new) in moved_trees {
        for (point, blocked) in [(old, false), (new, true)] {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let x = (point[0].round() as isize + dx) as usize;
                    let y = (point[1].round() as isize + dy) as usize;
                    let i = (frame.height - 1 - y) * frame.width + x;
                    let patch=patches.entry((x,y)).or_insert_with(||json!({"x":x,"y":y,"before_raw":hm.elevations[i],"raw":hm.elevations[i],"impassable":bt.impassability[i],"build_blocked":bt.buildability[i]}));
                    patch["build_blocked"] = json!(blocked);
                }
            }
        }
    }
    project::write_json(
        &p.path(plan["terrain_patch"].as_str().unwrap())?,
        &patches.into_values().collect::<Vec<_>>(),
    )?;
    terrain::native_cell_edits::apply(&p, &mut native)?;
    let objects: Vec<_> = layout.objects.iter().map(|o| o.object(&p)).collect();
    native.get_mut("ObjectsList")?.payload = map::write_objects(&objects, &native)?;
    let spawns = terrain::spawn_placement::report(
        &p,
        &native,
        &layout,
        &terrain::spawn_placement::policy(&p)?,
    )?;
    let clearance = terrain::ramp_clearance::validate(&p, &native, &layout);
    let policy = serde_json::from_slice(&fs::read(
        root.join("templates/rules/coordinate-pipeline.json"),
    )?)?;
    let scene = terrain::spatial::Scene::load(&p, &native, policy)?;
    let mut issues = scene.inspect(&layout)?;
    let (profiles, ramp_issues) = scene.ramp_profiles(&layout)?;
    issues.extend(ramp_issues);
    let (routes, route_issues) = scene.routes(&native)?;
    issues.extend(route_issues);
    project::write_json(
        &dir.join("spawn-review.json"),
        &json!({"spawn_placement":spawns,"ramp_clearance":clearance.as_ref().map_err(|e|e.to_string()),"issues":issues,"ramp_profiles":profiles,"routes":routes}),
    )?;
    println!(
        "preflight: {} spawn findings, {} spatial findings, clearance {}",
        spawns["findings"].as_array().unwrap().len(),
        issues.len(),
        clearance.is_ok()
    );
    let render = |p: &Project, native: &MapFile, layout: Layout| -> Result<()> {
        let hm = HeightMap::parse(native.get("HeightMapData")?)?;
        let bt = BlendTiles::parse(native.get("BlendTileData")?, hm.width, hm.height)?;
        let raw = terrain::flip(&hm.elevations, hm.width);
        let z: Vec<_> = raw.iter().map(|v| f64::from(*v) * frame.z_scale).collect();
        let t = terrain::Terrain {
            n: hm.width,
            land: z.iter().map(|v| *v >= 200.).collect(),
            slope: terrain::gradient(&z, hm.width),
            raw,
            z,
            layout,
        };
        let s = Surface {
            chosen: terrain::flip(&bt.tiles, hm.width)
                .iter()
                .map(|v| v / 64)
                .collect(),
            impassable: terrain::flip(&bt.impassability, hm.width),
            build_block: terrain::flip(&bt.buildability, hm.width),
            layer_edge: t.slope.iter().map(|v| *v > 24.).collect(),
            shore: vec![false; t.raw.len()],
            textures: vec![],
        };
        fs::create_dir_all(p.source())?;
        ra3maps::render::previews(&t, p, &s)
    };
    let mut review = p.clone();
    review.assets = root.join("work/henan-spawn-review");
    review.cfg["PREVIEW_SUBTITLE"] = json!("按用户红圈：南 P1–P4 / 北 P5–P8；右下局部拓宽二层台地");
    render(&review, &native, layout.clone())?;
    if !std::env::args().any(|v| v == "--write") {
        return Ok(());
    }
    assert!(
        spawns["passed"] == true && issues.is_empty() && clearance.is_ok(),
        "review findings must be resolved before adoption"
    );
    let gifts = gift_clearance::relocate_for_starts(&p, &mut native, &layout)?;
    let actual = map::objects(native.get("ObjectsList")?, &native.table)?;
    assert_eq!(actual.len(), layout.objects.len());
    for (o, s) in actual.iter().zip(&mut layout.objects) {
        s.cell = frame.grid([f64::from(o.position[0]), f64::from(o.position[1])]);
        s.angle = o.angle;
    }
    project::write_json(&dir.join("candidate-layout.json"), &layout)?;
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(native.get("BlendTileData")?, hm.width, hm.height)?;
    script_spawns::validate(&p, &native, &hm, &bt, &actual)?;
    assert_eq!(
        layout
            .objects
            .iter()
            .map(|o| (&o.type_name, &o.props))
            .collect::<Vec<_>>(),
        before
            .objects
            .iter()
            .map(|o| (&o.type_name, &o.props))
            .collect::<Vec<_>>()
    );
    project::write_json(&dir.join("gift-relocation.json"), &gifts)?;
    project::write_json(&original.path("authoring/layout.json")?, &layout)?;
    let mut cfg = original.cfg.clone();
    cfg["NATIVE_CELL_EDITS"] = plan["terrain_patch"].clone();
    cfg["BASES"] = p.cfg["BASES"].clone();
    cfg["PREVIEW_SUBTITLE"] = json!("800×768 · 南4人 VS 北2人+2个固定AI（苏联、帝国）");
    project::write_json(&original.path("config.json")?, &cfg)?;
    println!("Applied reviewed coordinates and native local patch; run build --map 河南 --bump.");
    Ok(())
}

fn audit(p: &Project, before: &MapFile, dir: &Path, plan: &serde_json::Value) -> Result<()> {
    use ra3maps::gameplay::scripts;
    let path = p.assets.join(format!("{}.map", p.name));
    let after = MapFile::load(&path)?;
    let a = HeightMap::parse(before.get("HeightMapData")?)?;
    let b = HeightMap::parse(after.get("HeightMapData")?)?;
    assert_eq!((a.width, a.height, a.border), (b.width, b.height, b.border));
    let patches: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(p.path(plan["terrain_patch"].as_str().unwrap())?)?)?;
    let allowed: std::collections::BTreeSet<_> = patches
        .iter()
        .map(|v| {
            (
                v["x"].as_u64().unwrap() as usize,
                v["y"].as_u64().unwrap() as usize,
            )
        })
        .collect();
    let mut heights = 0;
    for (i, (old, new)) in a.elevations.iter().zip(&b.elevations).enumerate() {
        assert_eq!(
            f64::from(*old) * 0.0390625 < 200.,
            f64::from(*new) * 0.0390625 < 200.,
            "water mask"
        );
        if old != new {
            assert!(allowed.contains(&(i % a.width, a.height - 1 - i / a.width)));
            heights += 1;
        }
    }
    let bt = BlendTiles::parse(before.get("BlendTileData")?, a.width, a.height)?;
    let actual_bt = BlendTiles::parse(after.get("BlendTileData")?, a.width, a.height)?;
    assert_eq!(bt.textures, actual_bt.textures);
    let n = a.width * a.height;
    let old = &before.get("BlendTileData")?.payload;
    let new = &after.get("BlendTileData")?.payload;
    for i in 0..n {
        let point = (i % a.width, a.height - 1 - i / a.width);
        if !allowed.contains(&point) {
            for plane in 0..4 {
                assert_eq!(
                    old[4 + plane * 2 * n + 2 * i..6 + plane * 2 * n + 2 * i],
                    new[4 + plane * 2 * n + 2 * i..6 + plane * 2 * n + 2 * i],
                    "material outside approved patch"
                );
            }
        }
    }
    let mask_len = a.height * a.width.div_ceil(8);
    let mut bitmap_changes = [0; 8];
    for (plane, count) in bitmap_changes.iter_mut().enumerate() {
        for i in 0..n {
            let offset = 4
                + 8 * n
                + plane * mask_len
                + (i / a.width) * a.width.div_ceil(8)
                + (i % a.width) / 8;
            let mask = 1 << ((i % a.width) % 8);
            if old[offset] & mask != new[offset] & mask {
                assert!([0, 5].contains(&plane));
                assert!(allowed.contains(&(i % a.width, a.height - 1 - i / a.width)));
                *count += 1;
            }
        }
    }
    let original = map::objects(before.get("ObjectsList")?, &before.table)?;
    let actual = map::objects(after.get("ObjectsList")?, &after.table)?;
    assert_eq!(original.len(), actual.len());
    let mut moved = 0;
    for (old, new) in original.iter().zip(&actual) {
        assert_eq!(old.props, new.props);
        assert_eq!(old.type_name, new.type_name);
        moved += usize::from(old.position != new.position || old.angle != new.angle);
    }
    fn normalize(nodes: &mut [scripts::Node]) -> Result<()> {
        for node in nodes {
            if let Some(children) = &mut node.children {
                normalize(children)?;
            }
            if node.command.as_deref()
                == Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION")
            {
                let mut body = scripts::unhex(&node.body)?;
                for arg in scripts::scalar_arguments(node)? {
                    if arg.kind == 17 {
                        body[arg.real_offset..arg.real_offset + 4].fill(0);
                    }
                }
                node.body = scripts::hex(&body);
            }
        }
        Ok(())
    }
    let mut old_scripts =
        scripts::parse_nodes(&before.get("PlayerScriptsList")?.payload, &before.table)?;
    let mut new_scripts =
        scripts::parse_nodes(&after.get("PlayerScriptsList")?.payload, &after.table)?;
    normalize(&mut old_scripts)?;
    normalize(&mut new_scripts)?;
    assert_eq!(
        serde_json::to_value(old_scripts)?,
        serde_json::to_value(new_scripts)?
    );
    let mut chunks = Vec::new();
    for chunk in &before.chunks {
        let name = &before.table[&chunk.name_idx];
        if after.get(name)? != chunk {
            chunks.push(name.clone());
            assert!(
                [
                    "ObjectsList",
                    "PlayerScriptsList",
                    "TriggerAreas",
                    "HeightMapData",
                    "BlendTileData"
                ]
                .contains(&name.as_str()),
                "unrelated native chunk {name}"
            );
        }
    }
    gift_clearance::validate(p, &after)?;
    let report = json!({"source":plan["source_name"],"map":p.name,"passed":true,"modified_chunks":chunks,"changed_height_samples":heights,"bitmap_changed_samples":bitmap_changes,"moved_or_rotated_objects":moved,"object_count":actual.len(),"object_identities_properties_and_owners_unchanged":true,"gameplay_conditions_flags_and_trigger_references_unchanged":true,"gift_headings_only_updated_script_arguments":true,"water_mask_unchanged":true,"material_planes_unchanged_outside_approved_cells":true});
    project::write_json(&dir.join("native-edit-diff.json"), &report)?;
    project::write_json(&p.assets.join("native-edit-diff.json"), &report)?;
    println!(
        "Native edit audit: {moved} moved/rotated objects, {heights} local height samples; gameplay conditions and all water samples unchanged."
    );
    Ok(())
}
