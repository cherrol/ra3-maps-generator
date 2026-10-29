use ra3maps::{
    formats::map::{BlendTiles, HeightMap, MapFile},
    pipeline::project::Project,
};
use std::{fs, path::Path};

#[test]
fn four_mountains_block_ground_preserve_navigation_assets_and_ore_foundations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = Project::load(root, "陕西").unwrap();
    let plan: serde_json::Value = serde_json::from_slice(
        &fs::read(p.path("histories/T0.6.7/add-four-mountains.json").unwrap()).unwrap(),
    )
    .unwrap();
    let name = plan["source"].as_str().unwrap();
    let mut map =
        MapFile::load(p.release_assets(name).unwrap().join(format!("{name}.map"))).unwrap();
    let original = map.clone();
    assert!(ra3maps::terrain::ramps::add_mountains(&p, &mut map, "wrong source", &plan).is_err());
    assert_eq!(map.encode().unwrap(), original.encode().unwrap());
    let result = ra3maps::terrain::ramps::add_mountains(&p, &mut map, name, &plan).unwrap();
    assert_eq!(result.report["mountains"].as_array().unwrap().len(), 4);
    assert_eq!(result.report["regressions"], serde_json::json!([]));
    let old = HeightMap::parse(original.get("HeightMapData").unwrap()).unwrap();
    let new = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
    let a = BlendTiles::parse(
        original.get("BlendTileData").unwrap(),
        old.width,
        old.height,
    )
    .unwrap();
    let b = BlendTiles::parse(map.get("BlendTileData").unwrap(), new.width, new.height).unwrap();
    assert_eq!(a.textures, b.textures);
    let mut modified = 0;
    for i in 0..old.elevations.len() {
        if old.elevations[i] != new.elevations[i] {
            modified += 1;
            assert!(new.elevations[i] > old.elevations[i]);
            assert!(b.impassability[i] && b.buildability[i]);
        } else {
            assert_eq!(a.impassability[i], b.impassability[i]);
            assert_eq!(a.buildability[i], b.buildability[i]);
        }
    }
    assert!(modified > 4000);
    let n = old.width;
    // This ore lies inside the user's central box. Its complete foundation
    // must remain flat and retain its original navigation/building flags.
    for y in 388..=427 {
        for x in 384..=423 {
            if (x as f64 - 403.8).hypot(y as f64 - 407.3) > 20. {
                continue;
            }
            let i = (n - 1 - y) * n + x;
            assert_eq!(old.elevations[i], new.elevations[i]);
            assert_eq!(a.impassability[i], b.impassability[i]);
            assert_eq!(a.buildability[i], b.buildability[i]);
        }
    }
    let start = 4 + 8 * n * n;
    let len = n * n.div_ceil(8);
    for mask in [1, 2, 3, 4, 6, 7] {
        let r = start + mask * len..start + (mask + 1) * len;
        assert_eq!(
            original.get("BlendTileData").unwrap().payload[r.clone()],
            map.get("BlendTileData").unwrap().payload[r]
        );
    }
    for c in &original.chunks {
        let name = &original.table[&c.name_idx];
        if !["HeightMapData", "BlendTileData"].contains(&name.as_str()) {
            assert_eq!(c, map.get(name).unwrap());
        }
    }
}
