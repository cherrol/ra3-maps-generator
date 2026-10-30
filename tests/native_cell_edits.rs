use ra3maps::{
    formats::map::{BlendTiles, HeightMap},
    terrain::native_cell_edits,
};
use serde_json::json;
use std::fs;
mod support;

#[test]
fn local_cell_edits_are_idempotent_and_preserve_other_navigation_planes() {
    let (mut p, mut map) = support::current_map();
    let original = map.clone();
    let hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
    let bt = BlendTiles::parse(map.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    let (x, y) = (300, 300);
    let index = (hm.height - 1 - y) * hm.width + x;
    let raw = hm.elevations[index];
    let temp = std::env::temp_dir().join(format!("ra3-native-cells-{}", std::process::id()));
    let directory = temp.join("maps/河南");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("cells.json"),serde_json::to_vec(&json!([{"x":x,"y":y,"before_raw":raw,"raw":raw+1,"impassable":!bt.impassability[index],"build_blocked":!bt.buildability[index],"tile":null}])).unwrap()).unwrap();
    p.root = temp.clone();
    p.cfg["NATIVE_CELL_EDITS"] = json!("cells.json");
    native_cell_edits::apply(&p, &mut map).unwrap();
    let after = map.encode().unwrap();
    native_cell_edits::apply(&p, &mut map).unwrap();
    assert_eq!(after, map.encode().unwrap());
    let changed =
        BlendTiles::parse(map.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    assert_eq!(changed.impassability[index], !bt.impassability[index]);
    assert_eq!(changed.buildability[index], !bt.buildability[index]);
    for i in 0..bt.tiles.len() {
        if i != index {
            assert_eq!(changed.impassability[i], bt.impassability[i]);
            assert_eq!(changed.buildability[i], bt.buildability[i]);
        }
    }
    assert_eq!(bt.tiles, changed.tiles);
    assert_eq!(bt.textures, changed.textures);
    assert_eq!(map.get("BlendTileData").unwrap().version, 27);
    let bytes = hm.height * hm.width.div_ceil(8);
    let prefix = 4 + hm.width * hm.height * 8;
    for plane in [1, 2, 3, 4, 6, 7] {
        let start = prefix + plane * bytes;
        assert_eq!(
            &original.get("BlendTileData").unwrap().payload[start..start + bytes],
            &map.get("BlendTileData").unwrap().payload[start..start + bytes]
        );
    }
    let mut bad = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
    bad.elevations[index] = raw + 2;
    map.get_mut("HeightMapData").unwrap().payload = bad.encode().unwrap();
    let before = map.encode().unwrap();
    assert!(native_cell_edits::apply(&p, &mut map).is_err());
    assert_eq!(before, map.encode().unwrap());
    fs::remove_dir_all(temp).unwrap();
}
