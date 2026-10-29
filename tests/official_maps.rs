//! Small, source-bound official fragments exercise codecs without the SDK mirror.
//! Heights and BlendTileData below are controlled test data, not official terrain.
use ra3maps::{
    formats::{
        archive,
        map::{BlendTiles, Chunk, HeightMap, MapFile},
        refpack,
    },
    gameplay::scripts::unhex,
    terrain::surface::Surface,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn fixture() -> Value {
    let files = archive::unzip(include_bytes!("fixtures/official-map-fragments.zip")).unwrap();
    serde_json::from_slice(&files["official-map-fragments.json"]).unwrap()
}

fn controlled_map(case: &Value) -> (MapFile, Vec<Chunk>) {
    let order = case["table"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry[1].as_str().unwrap().to_string(),
                entry[0].as_u64().unwrap() as u32,
            )
        })
        .collect::<Vec<_>>();
    let table = order.iter().map(|(name, id)| (*id, name.clone())).collect();
    let mut map = MapFile {
        order,
        table,
        chunks: vec![],
    };
    let opaque = case["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|chunk| {
            let payload = unhex(chunk["payload_hex"].as_str().unwrap()).unwrap();
            assert_eq!(crc32fast::hash(&payload) as u64, chunk["payload_crc32"]);
            Chunk {
                name_idx: chunk["id"].as_u64().unwrap() as u32,
                version: chunk["version"].as_u64().unwrap() as u16,
                payload,
            }
        })
        .collect::<Vec<_>>();
    let (width, height) = (
        case["width"].as_u64().unwrap() as usize,
        case["height"].as_u64().unwrap() as usize,
    );
    let mut heights = unhex(case["heightmap_header_hex"].as_str().unwrap()).unwrap();
    heights.extend(std::iter::repeat_n(210_u16.to_le_bytes(), width * height).flatten());
    map.chunks.push(Chunk {
        name_idx: map.key("HeightMapData").unwrap(),
        version: 6,
        payload: heights,
    });
    let surface = Surface {
        chosen: vec![0; width * height],
        impassable: vec![false; width * height],
        build_block: vec![false; width * height],
        layer_edge: vec![false; width * height],
        shore: vec![false; width * height],
        textures: vec![[
            "ControlledFixtureGrass".into(),
            String::new(),
            String::new(),
        ]],
    };
    map.chunks.push(Chunk {
        name_idx: map.key("BlendTileData").unwrap(),
        version: 27,
        payload: surface.encode(width, &[]).unwrap(),
    });
    map.chunks.extend(opaque.iter().cloned());
    let future_id = map.id("ControlledFutureUnknownChunk").unwrap();
    map.chunks.push(Chunk {
        name_idx: future_id,
        version: 73,
        payload: vec![0, 0xff, 0x7f, 0x80, 3, 0],
    });
    (map, opaque)
}

#[test]
fn committed_official_fragments_bind_four_specific_originals_without_external_data() {
    let data = fixture();
    assert_eq!(data["revision"], "3aad8e412961ab543192ae70f8503dc8e178e095");
    let expected = BTreeMap::from([
        (
            "Map_MP_4_Feasel1.map",
            "f1958605a28365755c17eb71226eaab246805849b6e836f1bca7fd52d51f76ca",
        ),
        (
            "CAMP_J05_DeepOcean_Davis.map",
            "7b31b041cdcdf1acfd8f9c322a09820e3dad06a40e4b8d14a88311651844ded4",
        ),
        (
            "MAP_MP_4_Feasel1_EP1.map",
            "5ab52a1697c32baa43325b7b0ef515b637d0504cbdcc088cc31e9ea320ce748f",
        ),
        (
            "Map_MP_2_Feasel2_Edit.map",
            "43e019ec1f2592f1dc5af8429d2d538034d53b77909964fbbd309d00e75a1ca4",
        ),
    ]);
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected.len());
    for case in cases {
        let name = case["source"].as_str().unwrap().rsplit('/').next().unwrap();
        assert_eq!(case["source_sha256"], expected[name]);
        assert!(case["source_bytes"].as_u64().unwrap() > 500_000);
    }
}

#[test]
fn official_headers_and_opaque_chunks_survive_codec_and_local_height_edits() {
    let mut frames = Vec::new();
    let mut environment_versions = Vec::new();
    for case in fixture()["cases"].as_array().unwrap() {
        let (map, opaque) = controlled_map(case);
        let plain = map.encode().unwrap();
        let packed = refpack::pack(&plain).unwrap();
        let mut roundtrip = MapFile::load_bytes(&packed).unwrap();
        roundtrip.validate_release().unwrap();
        assert_eq!(roundtrip.table, map.table);
        assert_eq!(roundtrip.chunks, map.chunks);
        let mut hm = HeightMap::parse(roundtrip.get("HeightMapData").unwrap()).unwrap();
        let bt = BlendTiles::parse(roundtrip.get("BlendTileData").unwrap(), hm.width, hm.height)
            .unwrap();
        assert_eq!(bt.impassability.len(), hm.width * hm.height);
        assert!(bt.impassability.iter().all(|&value| !value));
        assert_eq!(hm.border as u64, case["border"]);
        frames.push((hm.width, hm.height, hm.border));
        environment_versions.push(roundtrip.get("EnvironmentData").unwrap().version);

        hm.elevations[hm.width * (hm.height - 1)] = 211;
        roundtrip.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        let edited = MapFile::decode(&roundtrip.encode().unwrap()).unwrap();
        assert_eq!(
            HeightMap::parse(edited.get("HeightMapData").unwrap())
                .unwrap()
                .elevations,
            hm.elevations
        );
        for chunk in opaque {
            let name = &map.table[&chunk.name_idx];
            assert_eq!(
                edited.get(name).unwrap(),
                &chunk,
                "opaque chunk changed: {name}"
            );
        }
        assert_eq!(
            edited.get("ControlledFutureUnknownChunk").unwrap(),
            map.get("ControlledFutureUnknownChunk").unwrap()
        );
        assert_eq!(
            edited.get("BlendTileData").unwrap(),
            map.get("BlendTileData").unwrap()
        );
    }
    assert!(frames.contains(&(440, 640, 20)));
    assert!(frames.contains(&(660, 660, 30)));
    assert!(environment_versions.contains(&4));
    assert!(environment_versions.contains(&5));
}
