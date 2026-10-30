use ra3maps::{
    formats::{
        archive,
        binary::Writer,
        map::{BlendTiles, Chunk, HeightMap, MapFile},
        npy, sha256,
    },
    pipeline::project::Project,
    terrain::native_cell_edits,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const W: usize = 16;
const H: usize = 16;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    p: Project,
    map: MapFile,
    dir: PathBuf,
    scope: Value,
    cells: Value,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ra3-native-scope-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let dir = root.join("maps/synthetic");
        fs::create_dir_all(&dir).unwrap();
        let mut raw = vec![0; W * H];
        let mut layers = vec![0u8; W * H];
        let mut water = vec![0u8; W * H];
        for y in 0..H {
            for x in 0..W {
                let level = if x < 8 {
                    1
                } else if x < 12 {
                    2
                } else {
                    0
                };
                layers[y * W + x] = level;
                water[y * W + x] = u8::from(level == 0);
                raw[(H - 1 - y) * W + x] = match level {
                    1 => 210,
                    2 => 280,
                    _ => 130,
                };
            }
        }
        let hm = HeightMap {
            width: W,
            height: H,
            border: 2,
            borders: vec![[0, 0, 12, 12]],
            elevations: raw,
        };
        let n = W * H;
        let mut b = Writer::new();
        b.u32(n as u32);
        for plane in 0..4 {
            for _ in 0..n {
                b.u16(if plane == 0 { 2 } else { 1 });
            }
        }
        for plane in 0..8 {
            b.bytes(&vec![
                if plane % 2 == 0 { 0x55 } else { 0xaa };
                H * W.div_ceil(8)
            ]);
        }
        b.bytes(&vec![0x73; n]);
        b.u32(128);
        b.u32(2);
        b.u32(2);
        b.u32(2);
        for name in ["Grass_NewYork01", "Dirt_Iceland05"] {
            b.bytes(&[0; 16]);
            b.s16(name).unwrap();
        }
        b.bytes(&[0; 8]);
        b.bytes(&[0; 18]);
        b.bytes(&[0; 38]);
        let table: BTreeMap<_, _> = [
            (1, "HeightMapData".to_owned()),
            (2, "BlendTileData".to_owned()),
            (3, "ObjectsList".to_owned()),
            (4, "PlayerScriptsList".to_owned()),
        ]
        .into_iter()
        .collect();
        let map = MapFile {
            order: table.iter().rev().map(|(i, n)| (n.clone(), *i)).collect(),
            table,
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 27,
                    payload: b.data,
                },
                Chunk {
                    name_idx: 3,
                    version: 3,
                    payload: vec![3, 7, 11],
                },
                Chunk {
                    name_idx: 4,
                    version: 1,
                    payload: vec![13, 17, 19],
                },
            ],
        };
        let source = map.encode().unwrap();
        fs::write(dir.join("source.map"), &source).unwrap();
        let design = archive::zip(&[
            ("layers.npy".into(), npy::encode("|u1", &[H, W], &layers)),
            ("water.npy".into(), npy::encode("|u1", &[H, W], &water)),
        ])
        .unwrap();
        fs::write(dir.join("design.npz"), &design).unwrap();
        let mut allowed = vec![0; W * H];
        allowed[7 * W + 7] = 1;
        allowed[7 * W + 8] = 1;
        let mask = npy::encode("|u1", &[H, W], &allowed);
        fs::write(dir.join("allowed.npy"), &mask).unwrap();
        let scope = json!({"schema_version":1,"source":{"path":"source.map","sha256":sha256::hex(&source)},"design":{"path":"design.npz","sha256":sha256::hex(&design),"layer_key":"layers.npy","water_key":"water.npy"},"allowed_cells":{"path":"allowed.npy","sha256":sha256::hex(&mask)},"slopes":[{"id":"R1","layer_pair":[1,2],"raw_range":[210,280],"mask":{"path":"allowed.npy","sha256":sha256::hex(&mask)}}]});
        let p = Project {
            root: root.clone(),
            id: "synthetic".into(),
            cfg: json!({"Z_SCALE":1.,"WATER_Z":200.,"NATIVE_CELL_EDITS":"cells.json","NATIVE_CELL_EDIT_SCOPE":"scope.json"}),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "Synthetic".into(),
            name: "SyntheticT0.0.1".into(),
            assets: root.join("assets"),
            dist: root.join("dist"),
        };
        let cells = json!([{"x":7,"y":7,"before_raw":210,"raw":245,"impassable":false,"build_blocked":true,"tile":null}]);
        Self {
            p,
            map,
            dir,
            scope,
            cells,
        }
    }
    fn write(&self) {
        fs::write(
            self.dir.join("scope.json"),
            serde_json::to_vec(&self.scope).unwrap(),
        )
        .unwrap();
        fs::write(
            self.dir.join("cells.json"),
            serde_json::to_vec(&self.cells).unwrap(),
        )
        .unwrap();
    }
    fn error_unchanged(&mut self, needle: &str) {
        self.write();
        let before = self.map.encode().unwrap();
        let error = native_cell_edits::apply(&self.p, &mut self.map)
            .unwrap_err()
            .to_string();
        assert!(error.contains(needle), "{error}");
        assert_eq!(before, self.map.encode().unwrap());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(self.p.root.clone()).unwrap();
    }
}

#[test]
fn legal_source_bound_slope_is_idempotent_and_preserves_other_planes() {
    let mut f = Fixture::new();
    f.write();
    let before = f.map.clone();
    native_cell_edits::apply(&f.p, &mut f.map).unwrap();
    let after = f.map.encode().unwrap();
    native_cell_edits::apply(&f.p, &mut f.map).unwrap();
    assert_eq!(after, f.map.encode().unwrap());
    let hm = HeightMap::parse(f.map.get("HeightMapData").unwrap()).unwrap();
    assert_eq!(hm.elevations[(H - 1 - 7) * W + 7], 245);
    let old = &before.get("BlendTileData").unwrap().payload;
    let new = &f.map.get("BlendTileData").unwrap().payload;
    let prefix = 4 + W * H * 8;
    let bytes = H * W.div_ceil(8);
    assert_eq!(&old[..prefix], &new[..prefix]);
    for plane in [1, 2, 3, 4, 6, 7] {
        let start = prefix + plane * bytes;
        assert_eq!(&old[start..start + bytes], &new[start..start + bytes]);
    }
    assert_eq!(&old[prefix + 8 * bytes..], &new[prefix + 8 * bytes..]);
    assert_eq!(
        before.get("ObjectsList").unwrap(),
        f.map.get("ObjectsList").unwrap()
    );
}
#[test]
fn scope_source_identity_and_undeclared_legacy_edits_are_rejected() {
    let mut f = Fixture::new();
    f.scope["source"]["sha256"] = json!("0".repeat(64));
    f.error_unchanged("identity mismatch");
    f.p.cfg
        .as_object_mut()
        .unwrap()
        .remove("NATIVE_CELL_EDIT_SCOPE");
    f.error_unchanged("NATIVE_CELL_EDIT_SCOPE is required");
}
#[test]
fn water_boundary_changes_and_height_edits_outside_a_slope_are_rejected() {
    let mut f = Fixture::new();
    f.cells[0]["raw"] = json!(190);
    f.error_unchanged("changes water/land boundary");
    f.cells[0]["raw"] = json!(245);
    f.scope["slopes"] = json!([]);
    f.error_unchanged("outside authorized slope");
}
#[test]
fn out_of_scope_and_duplicate_cells_fail_atomically() {
    let mut f = Fixture::new();
    f.cells[0]["x"] = json!(6);
    f.error_unchanged("outside declared scope");
    f.cells[0]["x"] = json!(7);
    let duplicate = f.cells[0].clone();
    f.cells.as_array_mut().unwrap().push(duplicate);
    f.error_unchanged("duplicate native cell");
}
#[test]
fn mismatching_design_masks_and_existing_candidate_edits_are_rejected() {
    let mut f = Fixture::new();
    f.scope["design"]["sha256"] = json!("0".repeat(64));
    f.error_unchanged("identity mismatch");
    let bytes = fs::read(f.dir.join("design.npz")).unwrap();
    f.scope["design"]["sha256"] = json!(sha256::hex(&bytes));
    let mut hm = HeightMap::parse(f.map.get("HeightMapData").unwrap()).unwrap();
    hm.elevations[(H - 1 - 5) * W + 5] += 1;
    f.map.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
    f.error_unchanged("outside the complete declared patch");
}
#[test]
fn material_edits_require_scope_and_preserve_mixes_unless_explicitly_cleared() {
    let mut f = Fixture::new();
    f.cells[0]["tile"] = json!(66);
    f.error_unchanged("explicit material scope");
    f.scope["materials"] = json!({"mask":f.scope["allowed_cells"].clone(),"clear_blends":false});
    f.write();
    let before = f.map.get("BlendTileData").unwrap().payload.clone();
    native_cell_edits::apply(&f.p, &mut f.map).unwrap();
    let index = (H - 1 - 7) * W + 7;
    let after = &f.map.get("BlendTileData").unwrap().payload;
    for plane in 0..3 {
        let start = 4 + W * H * 2 + plane * W * H * 2 + index * 2;
        assert_eq!(&before[start..start + 2], &after[start..start + 2]);
    }
    f.map = MapFile::load(f.dir.join("source.map")).unwrap();
    f.scope["materials"]["clear_blends"] = json!(true);
    f.write();
    native_cell_edits::apply(&f.p, &mut f.map).unwrap();
    let bt = BlendTiles::parse(f.map.get("BlendTileData").unwrap(), W, H).unwrap();
    assert_eq!(bt.tiles[index], 66);
    for plane in 0..3 {
        let start = 4 + W * H * 2 + plane * W * H * 2 + index * 2;
        assert_eq!(
            &f.map.get("BlendTileData").unwrap().payload[start..start + 2],
            &[0, 0]
        );
    }
}

#[test]
fn a_correct_file_hash_does_not_excuse_wrong_water_design_or_source_scripts() {
    let mut f = Fixture::new();
    let design = fs::read(f.dir.join("design.npz")).unwrap();
    let mut arrays = archive::unzip(&design).unwrap();
    let mut water = vec![0u8; W * H];
    for y in 0..H {
        for x in 12..W {
            water[y * W + x] = 1;
        }
    }
    water[7 * W + 7] = 1;
    arrays.insert("water.npy".into(), npy::encode("|u1", &[H, W], &water));
    let wrong = archive::zip(&arrays.into_iter().collect::<Vec<_>>()).unwrap();
    fs::write(f.dir.join("design.npz"), &wrong).unwrap();
    f.scope["design"]["sha256"] = json!(sha256::hex(&wrong));
    f.error_unchanged("design");
    fs::write(f.dir.join("design.npz"), &design).unwrap();
    f.scope["design"]["sha256"] = json!(sha256::hex(&design));
    f.map.get_mut("PlayerScriptsList").unwrap().payload.push(23);
    f.error_unchanged("source identity differs in PlayerScriptsList");
}
