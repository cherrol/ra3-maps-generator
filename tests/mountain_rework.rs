use ra3maps::{
    formats::archive,
    formats::map::{BlendTiles, HeightMap, MapFile},
    pipeline::build::Options,
    pipeline::project::{self, Project},
    pipeline::release,
    terrain,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
mod support;
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rounded_reference_replaces_old_peaks_preserves_routes_ores_and_extends_paired_atlas() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let original = Project::load(root, "陕西").unwrap();
    let plan: Value = serde_json::from_slice(
        &fs::read(
            original
                .path("histories/T0.6.9/rework-four-mountains-44gg.json")
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let source = plan["source"].as_str().unwrap();
    let restore = plan["restore_source"].as_str().unwrap();
    let temp =
        Temp(std::env::temp_dir().join(format!("ra3-mountain-rework-{}", std::process::id())));
    let recipe = temp.0.join("maps/陕西");
    fs::create_dir_all(recipe.join("authoring")).unwrap();
    fs::create_dir_all(recipe.join("histories/T0.6.9")).unwrap();
    let mut cfg: Value = serde_json::from_slice(
        &fs::read(original.path("histories/T0.6.9/config.json").unwrap()).unwrap(),
    )
    .unwrap();
    cfg["COORDINATE_PIPELINE"]["source_name"] = json!(source);
    cfg["LAYERS"] = json!(original.string("LAYERS", "layers.npz"));
    cfg["LAYOUT"] = json!(original.string("LAYOUT", "layout.json"));
    support::copy_declared_inputs(&original.map_dir(), &recipe, &cfg);
    project::write_json(&recipe.join("config.json"), &cfg).unwrap();
    project::write_json(
        &recipe.join("version.json"),
        &project::parse_version(original.release_version(source).unwrap()).unwrap(),
    )
    .unwrap();
    for file in [
        "authoring/policy.json",
        "histories/T0.6.9/rework-four-mountains-44gg.json",
    ] {
        fs::copy(original.path(file).unwrap(), recipe.join(file)).unwrap();
    }
    for (from, key) in [
        ("histories/T0.6.9/source.zip", "source_package"),
        ("histories/T0.6.9/layout.json", "layout"),
        ("histories/T0.6.9/baseline.json", "baseline"),
    ] {
        let target = recipe.join(cfg["COORDINATE_PIPELINE"][key].as_str().unwrap());
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(original.path(from).unwrap(), target).unwrap();
    }
    if let Some(plan) = cfg["RESOURCE_DISTRIBUTION"].as_str() {
        let target = recipe.join(plan);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(
            original
                .path("histories/T0.6.9/resource-distribution.json")
                .unwrap(),
            target,
        )
        .unwrap();
    }
    for m in plan["mountains"].as_array().unwrap() {
        for key in ["relief_file", "material_file"] {
            let file = m[key].as_str().unwrap();
            fs::copy(original.path(file).unwrap(), recipe.join(file)).unwrap();
        }
    }
    fs::create_dir_all(temp.0.join("templates/rules")).unwrap();
    for file in [
        "common-T0.5.1.json",
        "千壑横原.json",
        "coordinate-pipeline.json",
    ] {
        fs::copy(
            root.join("templates/rules").join(file),
            temp.0.join("templates/rules").join(file),
        )
        .unwrap();
    }
    fs::write(temp.0.join("Cargo.toml"), "# isolated mountain rework").unwrap();
    let source_assets = original.release_assets(source).unwrap();
    let isolated = Project::load(&temp.0, &original.id).unwrap();
    let target = isolated.release_assets(source).unwrap();
    fs::create_dir_all(target.join("source")).unwrap();
    for file in [
        format!("{source}.map"),
        format!("{source}_art.tga"),
        "map.xml".into(),
        "map.str".into(),
        "overrides.xml".into(),
        "NewWorldBuilder.notes.json".into(),
        "source/layout.json".into(),
    ] {
        fs::copy(source_assets.join(&file), target.join(file)).unwrap();
    }
    let restore_dir = isolated.release_assets(restore).unwrap();
    fs::create_dir_all(&restore_dir).unwrap();
    fs::copy(
        original
            .release_assets(restore)
            .unwrap()
            .join(format!("{restore}.map")),
        restore_dir.join(format!("{restore}.map")),
    )
    .unwrap();
    let next = release::add_mountains(
        &temp.0,
        &original.id,
        &original
            .dist
            .parent()
            .unwrap()
            .join(format!("{source}.zip")),
        &recipe.join("histories/T0.6.9/rework-four-mountains-44gg.json"),
        &Options {
            bump: true,
            note: "rounded mountain regression".into(),
            ..Options::default()
        },
    )
    .unwrap();
    let a = MapFile::load(source_assets.join(format!("{source}.map"))).unwrap();
    let b = MapFile::load(next.assets.join(format!("{}.map", next.name))).unwrap();
    assert_eq!(a.table, b.table);
    assert_eq!(a.order, b.order);
    for chunk in &a.chunks {
        let name = &a.table[&chunk.name_idx];
        if !["HeightMapData", "BlendTileData", "WorldInfo"].contains(&name.as_str()) {
            assert_eq!(chunk, b.get(name).unwrap());
        }
    }
    let ah = HeightMap::parse(a.get("HeightMapData").unwrap()).unwrap();
    let bh = HeightMap::parse(b.get("HeightMapData").unwrap()).unwrap();
    let n = ah.width;
    // The original tree stays in place and follows the mountain, rather than
    // forcing a circular lowland cut through the eastern ridge.
    let tree_cell = (n - 1 - 260) * n + 675;
    assert!((bh.elevations[tree_cell] - ah.elevations[tree_cell]) as f64 * 0.0390625 > 40.);
    assert!(bh.elevations.iter().max() < ah.elevations.iter().max());
    let at = BlendTiles::parse(a.get("BlendTileData").unwrap(), n, n).unwrap();
    let bt = BlendTiles::parse(b.get("BlendTileData").unwrap(), n, n).unwrap();
    assert!(bt.textures.starts_with(&at.textures));
    assert_eq!(bt.textures.len(), at.textures.len() + 2);
    let mut lower = 0;
    let mut higher = 0;
    for i in 0..n * n {
        if ah.elevations[i] != bh.elevations[i] {
            let x = i % n;
            let y = n - 1 - i / n;
            assert!(plan["mountains"].as_array().unwrap().iter().any(|m| {
                let q = &m["bounds"];
                x >= q[0].as_u64().unwrap() as usize
                    && x <= q[2].as_u64().unwrap() as usize
                    && y >= q[1].as_u64().unwrap() as usize
                    && y <= q[3].as_u64().unwrap() as usize
            }));
            if bh.elevations[i] < ah.elevations[i] {
                lower += 1;
            } else {
                higher += 1;
            }
        }
    }
    assert!(lower > 4000 && higher > 1000);
    for mask in [1, 2, 3, 4, 6, 7] {
        let len = n * n.div_ceil(8);
        let start = 4 + 8 * n * n + mask * len;
        assert_eq!(
            a.get("BlendTileData").unwrap().payload[start..start + len],
            b.get("BlendTileData").unwrap().payload[start..start + len]
        );
    }
    let distribution: Value = serde_json::from_slice(
        &fs::read(
            original
                .path("authoring/resource-distribution.json")
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    for r in distribution["routes"].as_array().unwrap() {
        for key in ["main_path", "secondary_path"] {
            for c in r[key].as_array().unwrap() {
                let x = c[0].as_u64().unwrap() as usize;
                let y = c[1].as_u64().unwrap() as usize;
                let i = (n - 1 - y) * n + x;
                assert_eq!(ah.elevations[i], bh.elevations[i]);
                assert_eq!(at.impassability[i], bt.impassability[i]);
            }
        }
    }
    let gradient = |hm: &HeightMap| {
        let z: Vec<_> = terrain::flip(&hm.elevations, n)
            .iter()
            .map(|h| *h as f64 * 0.0390625)
            .collect();
        let g = terrain::gradient(&z, n);
        let mut values = Vec::new();
        for m in plan["mountains"].as_array().unwrap() {
            let q = &m["bounds"];
            for y in q[1].as_u64().unwrap() as usize..=q[3].as_u64().unwrap() as usize {
                for x in q[0].as_u64().unwrap() as usize..=q[2].as_u64().unwrap() as usize {
                    values.push(g[y * n + x]);
                }
            }
        }
        values.sort_by(f64::total_cmp);
        values[(values.len() as f64 * 0.95) as usize]
    };
    assert!(gradient(&bh) < gradient(&ah));
    let xml = fs::read_to_string(next.assets.join("map.xml")).unwrap();
    for name in ["Rock_Yucatan03", "Rock_Yucatan02"] {
        assert!(xml.contains(&format!(
            "TextureID=\"{}\"",
            ra3maps::resources::fast_hash(name)
        )));
    }
    let source_xml = fs::read_to_string(source_assets.join("map.xml")).unwrap();
    let atlas_id = source_xml
        .split("<TerrainTextureAtlas")
        .nth(1)
        .unwrap()
        .split("id=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(!xml.contains(&format!("id=\"{atlas_id}\"")));
    ra3maps::resources::validate_terrain_package(&b, &xml, Some(&next.version)).unwrap();
    let props = |m: &MapFile| {
        ra3maps::formats::map::read_props(
            &mut ra3maps::formats::binary::Reader::new(&m.get("WorldInfo").unwrap().payload),
            &m.table,
        )
        .unwrap()
    };
    let old_props = props(&a);
    let new_props = props(&b);
    for old in &old_props {
        if old.key != "terrainTextureStrings" {
            assert_eq!(
                Some(&old.value),
                ra3maps::formats::map::prop(&new_props, &old.key)
            );
        }
    }
    let preload = ra3maps::formats::map::prop(&new_props, "terrainTextureStrings")
        .unwrap()
        .text()
        .unwrap();
    assert!(preload.contains("TRock_Yucatan03;TRock_Yucatan03_nrm;"));
    assert!(preload.contains("TRock_Yucatan02;TRock_Yucatan02_nrm;"));
    let zip =
        archive::unzip(&fs::read(ra3maps::pipeline::build::zip_path(&next)).unwrap()).unwrap();
    for file in ra3maps::pipeline::build::release_files(&next) {
        assert_eq!(
            fs::read(next.assets.join(&file)).unwrap(),
            zip[&format!("{}/{file}", next.name)]
        );
    }
    let report: Value =
        serde_json::from_slice(&fs::read(next.assets.join("mountain-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["profile"], "44gg-rounded-ridge");
    assert_eq!(report["regressions"], json!([]));
    assert_eq!(report["routes"], 528);
    assert_eq!(report["mountains"].as_array().unwrap().len(), 4);
}
