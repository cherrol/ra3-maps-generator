use ra3maps::{
    formats::map::{BlendTiles, HeightMap, MapFile},
    pipeline::project::Project,
    terrain,
    terrain::ramps,
};
use serde_json::Value;
use std::{fs, path::Path};

#[test]
fn authored_shoulder_removes_old_spur_preserves_ramp_objects_and_protected_bitmaps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = Project::load(root, "陕西").unwrap();
    let plan: Value = serde_json::from_slice(
        &fs::read(p.path("histories/T0.7.1/repair-ramp-568.json").unwrap()).unwrap(),
    )
    .unwrap();
    let name = plan["source"].as_str().unwrap();
    let a = MapFile::load(p.release_assets(name).unwrap().join(format!("{name}.map"))).unwrap();
    let mut b = a.clone();
    let mut bad = plan.clone();
    bad["source"] = serde_json::json!("wrong version");
    assert!(ramps::relocate(&p, &mut b, name, &bad).is_err());
    assert_eq!(a.encode().unwrap(), b.encode().unwrap());
    let result = ramps::relocate(&p, &mut b, name, &plan).unwrap();
    assert!(
        result.report["spatial_regressions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let metrics = &result.report["ramps"][0]["validation"];
    assert_eq!(metrics["width_cells"], 25.);
    assert_eq!(metrics["parallel_lanes"], 26);
    for c in &a.chunks {
        let name = &a.table[&c.name_idx];
        if !["HeightMapData", "BlendTileData"].contains(&name.as_str()) {
            assert_eq!(c, b.get(name).unwrap());
        }
    }
    let ah = HeightMap::parse(a.get("HeightMapData").unwrap()).unwrap();
    let bh = HeightMap::parse(b.get("HeightMapData").unwrap()).unwrap();
    let n = ah.width;
    let az = terrain::flip(&ah.elevations, n);
    let bz = terrain::flip(&bh.elevations, n);
    // The high rectangular spur is removed, joining the actual middle terrace.
    assert!((az[584 * n + 320] as f64 * 0.0390625 - 350.).abs() < 1.);
    assert!((bz[584 * n + 320] as f64 * 0.0390625 - 280.).abs() < 1.);
    let ab = BlendTiles::parse(a.get("BlendTileData").unwrap(), n, n).unwrap();
    let bb = BlendTiles::parse(b.get("BlendTileData").unwrap(), n, n).unwrap();
    let frame = ra3maps::coordinates::Frame::from_project(&p);
    let objects = ra3maps::formats::map::objects(a.get("ObjectsList").unwrap(), &a.table).unwrap();
    for o in &objects {
        if o.type_name.contains("Tree")
            || o.type_name.contains("Bush")
            || o.type_name.contains("Rock")
        {
            continue;
        }
        let c = frame.grid([o.position[0] as f64, o.position[1] as f64]);
        let radius = if o.type_name == "OreNode" { 20. } else { 8. };
        for y in 0..n {
            for x in 0..n {
                if (x as f64 - c[0]).hypot(y as f64 - c[1]) <= radius {
                    let i = y * n + x;
                    let g = (n - 1 - y) * n + x;
                    assert_eq!(az[i], bz[i], "protected height");
                    assert_eq!(
                        ab.impassability[g], bb.impassability[g],
                        "protected passability"
                    );
                    assert_eq!(
                        ab.buildability[g], bb.buildability[g],
                        "protected buildability"
                    );
                }
            }
        }
    }
    let bounds = plan["bounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect::<Vec<_>>();
    for i in 0..n * n {
        if az[i] != bz[i] {
            assert!(
                i % n >= bounds[0] && i % n < bounds[2] && i / n >= bounds[1] && i / n < bounds[3]
            );
        }
    }
}
