use ra3maps::{
    archive,
    binary::{Reader, Writer},
    map::{self, Chunk, MapFile, Property, Value},
    project, refpack, scripts,
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

#[test]
fn shared_template_preserves_t051_except_the_named_yard_lookup() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../maps/千壑横原/repack/T0.5.1.rules.json")).unwrap();
    let shared: serde_json::Value =
        serde_json::from_str(include_str!("../templates/rules/common-T0.5.1.json")).unwrap();
    for key in ["SidesList", "Teams", "names"] {
        assert_eq!(source[key], shared[key], "{key}");
    }
    let mut expected: Vec<scripts::Node> =
        serde_json::from_value(source["scripts"].clone()).unwrap();
    fn adapt(nodes: &mut [scripts::Node]) -> usize {
        let mut changes = 0;
        for node in nodes {
            if node.asset == "Script"
                && scripts::name(node).unwrap().as_deref() == Some("Player 8 Set Up")
            {
                for action in node.children.as_mut().unwrap() {
                    if action.command.as_deref()
                        == Some("SET_REF_TO_NEREST_TEAM_OF_UNNAMED_TYPE_OWNED_BY_PLAYER")
                    {
                        let mut bytes = scripts::unhex(&action.body).unwrap();
                        assert_eq!(u32::from_le_bytes(bytes[..4].try_into().unwrap()), 371);
                        bytes[..4].copy_from_slice(&370u32.to_le_bytes());
                        action.body = scripts::hex(&bytes);
                        action.command =
                            Some("SET_REF_TO_NEREST_TEAM_OF_TYPE_OWNED_BY_PLAYER".into());
                        changes += 1;
                    }
                }
            } else if let Some(children) = &mut node.children {
                changes += adapt(children);
            }
        }
        changes
    }
    assert_eq!(adapt(&mut expected), 1);
    assert_eq!(scripts::script_names(&expected).unwrap().len(), 224);
    assert_eq!(serde_json::to_value(expected).unwrap(), shared["scripts"]);
}

#[test]
fn refpack_all_literal_lengths_and_overlapping_matches() {
    for n in 0..260 {
        let raw = (0..n).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        assert_eq!(
            refpack::decompress(&refpack::literals(&raw).unwrap()).unwrap(),
            raw
        );
        assert_eq!(
            refpack::decompress(&refpack::compress(&raw).unwrap()).unwrap(),
            raw
        );
    }
    let raw = vec![b'a'; 300_000];
    let compressed = refpack::compress(&raw).unwrap();
    assert!(compressed.len() < 2000);
    assert_eq!(refpack::decompress(&compressed).unwrap(), raw);
}
#[test]
fn refpack_all_back_reference_forms() {
    let streams = [
        vec![0x10, 0xfb, 0, 0, 6, 0x03, 0x02, b'a', b'b', b'c', 0xfc],
        vec![
            0x10, 0xfb, 0, 0, 8, 0xe0, b'a', b'b', b'c', b'd', 0x80, 0, 3, 0xfc,
        ],
        vec![
            0x10, 0xfb, 0, 0, 9, 0xe0, b'a', b'b', b'c', b'd', 0xc0, 0, 3, 0, 0xfc,
        ],
    ];
    for (stream, expected) in
        streams
            .into_iter()
            .zip([b"abcabc".as_slice(), b"abcdabcd", b"abcdabcda"])
    {
        assert_eq!(refpack::decompress(&stream).unwrap(), expected);
        for n in 0..stream.len() {
            assert!(refpack::decompress(&stream[..n]).is_err(), "truncation {n}");
        }
    }
}
#[test]
fn refpack_legacy_large_and_compressed_length_headers() {
    for marker in [0x80u8, 0x90] {
        let bytes = [marker, 0xfb, 0, 0, 0, 1, 0xfd, b'x'];
        assert_eq!(refpack::decompress(&bytes).unwrap(), b"x");
    }
    for marker in [0x81u8, 0x91] {
        let bytes = [marker, 0xfb, 0, 0, 0, 13, 0, 0, 0, 1, 0xfd, b'x'];
        assert_eq!(refpack::decompress(&bytes).unwrap(), b"x");
    }
    assert_eq!(
        refpack::decompress(&[0x11, 0xfb, 0, 0, 10, 0, 0, 1, 0xfd, b'x']).unwrap(),
        b"x"
    );
}
#[test]
fn refpack_rejects_corrupt_streams_and_ear_length() {
    for bytes in [
        vec![],
        vec![0x10],
        vec![0x10, 0xfc, 0, 0, 0, 0xfc],
        vec![0x10, 0xfb, 0, 0, 3, 0, 0, 0xfc],
        vec![0x10, 0xfb, 0, 0, 0, 0xfd, 0],
        vec![0x10, 0xfb, 0, 0, 0, 0xfc, 0],
    ] {
        assert!(refpack::decompress(&bytes).is_err());
    }
    let plain = b"CkMp\x00\x00\x00\x00";
    let mut ear = refpack::pack(plain).unwrap();
    assert_eq!(refpack::unpack(&ear).unwrap(), plain);
    ear[4] += 1;
    assert!(refpack::unpack(&ear).is_err());
    assert!(refpack::pack(b"wrong").is_err());
}
#[test]
fn deterministic_random_refpack_roundtrips() {
    let mut seed = 0x137ac5u32;
    for n in (0..8000).step_by(97) {
        let data = (0..n)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed % 31) as u8
            })
            .collect::<Vec<_>>();
        let compressed = refpack::compress(&data).unwrap();
        assert_eq!(refpack::decompress(&compressed).unwrap(), data);
        assert_eq!(refpack::compress(&data).unwrap(), compressed);
    }
}
#[test]
fn template_codecs_are_byte_exact() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/8p_template.map");
    let disk = std::fs::read(&path).unwrap();
    let plain = refpack::unpack(&disk).unwrap();
    let m = MapFile::decode(&plain).unwrap();
    assert_eq!(m.encode().unwrap(), plain);
    let objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
    assert_eq!(
        map::write_objects(&objects, &m).unwrap(),
        m.get("ObjectsList").unwrap().payload
    );
    let nodes =
        scripts::parse_nodes(&m.get("PlayerScriptsList").unwrap().payload, &m.table).unwrap();
    let mut linked = m.clone();
    assert_eq!(
        scripts::encode_nodes(&nodes, &mut linked).unwrap(),
        m.get("PlayerScriptsList").unwrap().payload
    );
    assert_eq!(
        refpack::unpack(&refpack::pack(&plain).unwrap()).unwrap(),
        plain
    );
}
#[test]
fn linked_string_table_is_written_in_worldbuilder_order_without_changing_ids() {
    let mut m = MapFile {
        order: vec![("first".into(), 1)],
        table: BTreeMap::from([(1, "first".into())]),
        chunks: vec![],
    };
    assert_eq!(m.id("linked").unwrap(), 2);
    assert!(m.validate_release().is_err());
    let loaded = MapFile::decode(&m.encode().unwrap()).unwrap();
    loaded.validate_release().unwrap();
    assert_eq!(
        loaded.order,
        vec![("linked".into(), 2), ("first".into(), 1)]
    );
    assert_eq!(loaded.table, m.table);
    m.table.remove(&1);
    assert!(m.encode().is_err());
}

#[test]
fn xml_release_path_changes_preserve_resource_asset_ids() {
    let xml = "<GameMap id=\"CHAIT0.5.1\"/><TerrainTextureAtlas id=\"CHAIT0.5.1\"/><MapMetaData FileName=\"data\\maps\\testt0.5.1\\testt0.5.1.map\"/>";
    let next = ra3maps::release::retarget_xml(xml, "T0.5.1", "T0.5.2").unwrap();
    assert!(next.contains("<TerrainTextureAtlas id=\"CHAIT0.5.1\""));
    assert!(next.contains("FileName=\"data\\maps\\testt0.5.2\\testt0.5.2.map\""));
    assert!(ra3maps::release::retarget_xml("FileName=\"oops", "T0.5.1", "T0.5.2").is_err());
}
#[test]
fn properties_preserve_types_order_unicode_and_latin1() {
    let mut m = MapFile {
        order: Vec::new(),
        table: BTreeMap::new(),
        chunks: Vec::new(),
    };
    for key in ["bool", "int", "float", "ascii", "unicode", "unknown"] {
        m.id(key).unwrap();
    }
    let props = [
        Value::Bool(1),
        Value::Int(-923),
        Value::Float(-0.125),
        Value::Ascii("café".into()),
        Value::Unicode("地图 🌊".into()),
        Value::Unknown("GameObject:Tree".into()),
    ]
    .into_iter()
    .zip(["bool", "int", "float", "ascii", "unicode", "unknown"])
    .map(|(value, key)| Property {
        key: key.into(),
        value,
    })
    .collect::<Vec<_>>();
    let mut w = Writer::new();
    map::write_props(&mut w, &props, &m).unwrap();
    let mut r = Reader::new(&w.data);
    assert_eq!(map::read_props(&mut r, &m.table).unwrap(), props);
    r.finish().unwrap();
    assert!(Writer::new().u24(1 << 24).is_err());
    assert!(Writer::new().s16("地图").is_err());
}
#[test]
fn corrupt_map_and_binary_lengths_return_errors() {
    for raw in [
        b"".as_slice(),
        b"CkMp",
        b"CkMp\xff\xff\xff\xff",
        b"CkMp\x01\x00\x00\x00\x80\x80\x80\x80\x10",
    ] {
        assert!(MapFile::decode(raw).is_err());
    }
    let mut r = Reader::new(&[0, 1]);
    assert!(r.take(usize::MAX).is_err());
    r.take(1).unwrap();
    assert!(r.u32().is_err());
    assert_eq!(r.pos, 1);
    assert!(
        map::HeightMap::parse(&Chunk {
            name_idx: 1,
            version: 6,
            payload: vec![0; 36]
        })
        .is_err()
    );
    let mut w = Writer::new();
    w.u32(2);
    w.varstr("same").unwrap();
    w.u32(1);
    w.varstr("duplicate").unwrap();
    w.u32(1);
    let mut data = b"CkMp".to_vec();
    data.extend(w.data);
    assert!(MapFile::decode(&data).is_err());
}
#[test]
fn bitmaps_align_each_row_and_use_lsb_first() {
    let bits = vec![
        true, false, false, false, false, false, false, false, true, false, true, false, false,
        false, false, false, false, false,
    ];
    let mut w = Writer::new();
    map::write_bits(&mut w, &bits, 9);
    assert_eq!(w.data, vec![1, 1, 2, 0]);
    assert_eq!(
        map::read_bits(&mut Reader::new(&w.data), 9, 2).unwrap(),
        bits
    );
}
#[test]
fn zip_unicode_names_crc_and_deterministic_output() {
    let files = vec![
        ("地图/地图.map".into(), vec![17; 5000]),
        ("地图/map.str".into(), b"hello".to_vec()),
    ];
    let zip = archive::zip(&files).unwrap();
    assert_eq!(zip, archive::zip(&files).unwrap());
    let read = archive::unzip(&zip).unwrap();
    for (name, data) in files {
        assert_eq!(read[&name], data);
    }
    let mut corrupt = zip.clone();
    corrupt[45] ^= 1;
    assert!(archive::unzip(&corrupt).is_err());
    assert!(archive::zip(&[("../escape".into(), vec![])]).is_err());
    for cut in [0, 3, 10, zip.len() - 1] {
        assert!(archive::unzip(&zip[..cut]).is_err());
    }
}
#[test]
fn npy_reader_uses_little_endian_shapes_and_scalars() {
    let a = ra3maps::terrain::npy("<u2", &[2, 2], &[1, 0, 0, 1, 255, 255, 2, 0]);
    let a = archive::Npy::parse(&a).unwrap();
    assert_eq!(a.shape, vec![2, 2]);
    assert_eq!(a.numbers().unwrap(), vec![1., 256., 65535., 2.]);
    let scalar = archive::Npy::parse(&ra3maps::terrain::npy(
        "<f8",
        &[],
        &0.0390625f64.to_le_bytes(),
    ))
    .unwrap();
    assert_eq!(scalar.numbers().unwrap(), vec![0.0390625]);
}
#[test]
fn version_carries_promotions_and_history_do_not_mutate_input() {
    let st = json!({"stage":"T","major":0,"minor":9,"patch":9,"history":[]});
    let next = project::bump(&st, false, 1, "调整").unwrap();
    assert_eq!(project::format_version(&next).unwrap(), "T1.0.0");
    assert_eq!(st["history"], json!([]));
    assert_eq!(next["history"].as_array().unwrap().len(), 1);
    assert_eq!(
        project::format_version(&project::bump(&st, true, 0, "").unwrap()).unwrap(),
        "V1.0.0"
    );
    assert_eq!(
        project::format_version(&project::bump(&st, true, 1, "").unwrap()).unwrap(),
        "V1.0.1"
    );
    assert_eq!(
        project::format_version(&project::parse_version(" v1.2.3 ").unwrap()).unwrap(),
        "V1.2.3"
    );
    for bad in ["", "X0.0.1", "T1.2", "T-1.2.3", "T1.2.3.4"] {
        assert!(project::parse_version(bad).is_err());
    }
}
#[test]
fn recipe_ids_cannot_escape_project() {
    for s in ["", "..", "../escape", "a/b", "a\\b", "/tmp"] {
        assert!(project::safe_component(s).is_err());
    }
    assert!(project::safe_component("碧嶂回澜").is_ok());
}
