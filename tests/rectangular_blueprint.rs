use ra3maps::{
    coordinates::Frame,
    formats::{
        archive,
        map::{BlendTiles, Chunk, HeightMap, MapFile},
        npy,
    },
    pipeline::project::{self, Project},
    terrain::{self, Terrain, surface::Surface},
};
use serde_json::json;
use std::{fs, path::Path};

#[test]
fn wide_and_tall_blueprints_roundtrip_native_heights_masks_and_materials() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (width, height) in [(96, 64), (64, 96)] {
        let root =
            std::env::temp_dir().join(format!("ra3-rect-{}-{width}-{height}", std::process::id()));
        fs::create_dir_all(root.join("rules")).unwrap();
        fs::copy(
            repository.join("rules/common-T0.5.1.json"),
            root.join("rules/common-T0.5.1.json"),
        )
        .unwrap();
        let mut rules: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("rules/common-T0.5.1.json")).unwrap())
                .unwrap();
        rules["spawn_waypoints"] = json!([]);
        project::write_json(&root.join("rules/common-T0.5.1.json"), &rules).unwrap();
        let mut cfg: serde_json::Value =
            serde_json::from_slice(&fs::read(repository.join("maps/河南/config.json")).unwrap())
                .unwrap();
        cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
        cfg.as_object_mut().unwrap().remove("RESOURCE_DISTRIBUTION");
        cfg["MAP_NAME"] = json!("Rect");
        cfg["GRID"] = json!(width);
        cfg["GRID_HEIGHT"] = json!(height);
        cfg["BORDER"] = json!(8);
        cfg["LAYERS"] = json!("layers.npz");
        cfg["LAYOUT"] = json!("layout.json");
        cfg["TERRAIN_ONLY"] = json!(true);
        cfg["TERRAIN"]["v11_baked"] = json!(true);
        cfg["TERRAIN"]["base_min_flat_radius"] = json!(0);
        project::write_json(&root.join("maps/Rect/config.json"), &cfg).unwrap();
        project::write_json(&root.join("maps/Rect/layout.json"), &json!({"schema_version":1,"base_cells":vec![[16,16];8],"ore_cells":[],"ore_angles":[],"objects":[]})).unwrap();
        let p = Project::load(&root, "Rect").unwrap();
        // Different north/south heights expose a mistaken square stride or flip.
        let raw: Vec<u16> = (0..width * height)
            .map(|i| if i / width < height / 2 { 5376 } else { 7168 })
            .collect();
        fs::write(
            p.path("layers.npz").unwrap(),
            archive::zip(&[
                (
                    "base_z.npy".into(),
                    npy::encode(
                        "<u2",
                        &[height, width],
                        &raw.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
                    ),
                ),
                (
                    "z_scale.npy".into(),
                    npy::encode("<f8", &[], &0.0390625_f64.to_le_bytes()),
                ),
            ])
            .unwrap(),
        )
        .unwrap();
        let t = Terrain::generate(&p).unwrap();
        t.save(&p).unwrap();
        assert_eq!(Terrain::load(&p).unwrap().raw, raw);
        assert_eq!(Frame::from_project(&p).height, height);
        let s = Surface::generate(&t, &p).unwrap();
        let mut native = MapFile::load(repository.join("templates/template.map")).unwrap();
        let original_hm = HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap();
        let original_bt = BlendTiles::parse(
            native.get("BlendTileData").unwrap(),
            original_hm.width,
            original_hm.height,
        )
        .unwrap();
        let hm = t.heightmap(&p);
        assert_eq!((hm.width, hm.height), (width, height));
        assert_eq!(hm.elevations, terrain::flip(&raw, width));
        assert_eq!(
            hm.borders[0],
            [0, 0, (width - 16) as u32, (height - 16) as u32]
        );
        native.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        native.get_mut("BlendTileData").unwrap().payload =
            s.encode(width, &original_bt.blend_descriptions).unwrap();
        // Exclude template gameplay: this test exercises terrain, and its empty
        // object list cannot satisfy reference-map script waypoint dependencies.
        native.get_mut("ObjectsList").unwrap().payload =
            ra3maps::formats::map::write_objects(&[], &native).unwrap();
        native.get_mut("PlayerScriptsList").unwrap().payload.clear();
        let bt = BlendTiles::parse(native.get("BlendTileData").unwrap(), width, height).unwrap();
        assert_eq!(terrain::flip(&bt.impassability, width), s.impassable);
        assert_eq!(terrain::flip(&bt.buildability, width), s.build_block);
        let gameplay_before = native.get("BlendTileData").unwrap().payload
            [4 + 8 * width * height..4 + 9 * width * height + 8 * height * width.div_ceil(8)]
            .to_vec();
        terrain::texture_polish::apply(&p, &mut native, &p.name).unwrap();
        let bt = BlendTiles::parse(native.get("BlendTileData").unwrap(), width, height).unwrap();
        assert_eq!(bt.tiles.len(), width * height);
        assert_eq!(
            native.get("BlendTileData").unwrap().payload
                [4 + 8 * width * height..4 + 9 * width * height + 8 * height * width.div_ceil(8)],
            gameplay_before
        );
        assert_eq!(
            HeightMap::parse(native.get("HeightMapData").unwrap())
                .unwrap()
                .elevations,
            hm.elevations
        );
        let truncated = Chunk {
            name_idx: 0,
            version: 6,
            payload: hm.encode().unwrap(),
        };
        assert_eq!(HeightMap::parse(&truncated).unwrap().height, height);
        fs::remove_dir_all(root).unwrap();
    }
}
