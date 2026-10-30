use super::*;

fn approach_ramp(
    y: f64,
    policy: &crate::terrain::ramp_clearance::Policy,
) -> crate::terrain::ramp_clearance::Ramp {
    crate::terrain::ramp_clearance::Ramp {
        id: 1,
        center: [128., y],
        unit: [1., 0.],
        half: 12.5,
        width: 25.,
        pair: [210., 280.],
        approach: crate::terrain::ramp_clearance::rectangle(
            [128., y],
            [1., 0.],
            -12.5 - policy.approach_extension,
            12.5 + policy.approach_extension,
            12.5,
        ),
    }
}

#[test]
fn geometry_candidates_reserve_full_refinery_and_end_approaches() {
    let policy = crate::terrain::ramp_clearance::Policy::default();
    let frame = crate::coordinates::Frame {
        width: 256,
        height: 256,
        border: 8,
        cell_size: 7.5,
        z_scale: 1.,
    };
    let old = approach_ramp(100., &policy);
    let candidate = approach_ramp(128., &policy);
    let heights = vec![280.; frame.width * frame.height];
    for (kind, point, angle) in [
        ("OreNode", [190., 160.], std::f64::consts::PI),
        ("OilDerrick", [171., 128.], 0.),
        ("HospitalTechStructure", [171., 128.], 0.),
    ] {
        let position = frame.world(point);
        let object = MapObject {
            position: [position[0] as f32, position[1] as f32, 0.],
            angle: angle as f32,
            version: 1,
            road_type: 0,
            type_name: kind.into(),
            props: vec![],
        };
        let conflicts = crate::terrain::ramp_clearance::conflicts(
            point,
            object.angle as f64,
            kind,
            280.,
            std::slice::from_ref(&candidate),
            &policy,
        );
        assert!(conflicts[0].body_gap > policy.gap);
        if kind == "OreNode" {
            assert!(conflicts[0].approach_gap > policy.gap);
            assert!(conflicts[0].refinery_gap.unwrap() < policy.gap);
        }
        assert!(preserves_resource_approaches(
            std::slice::from_ref(&object),
            &frame,
            (&heights, &heights),
            (&old, &old),
            &policy,
        ));
        assert!(!preserves_resource_approaches(
            std::slice::from_ref(&object),
            &frame,
            (&heights, &heights),
            (&old, &candidate),
            &policy,
        ));
        assert!(preserves_resource_approaches(
            std::slice::from_ref(&object),
            &frame,
            (&heights, &heights),
            (&candidate, &candidate),
            &policy,
        ));
        // Identical geometry cannot inherit a conflict that only becomes
        // applicable after its resource's supporting platform changes.
        let mut previous = heights.clone();
        previous[point[1] as usize * frame.width + point[0] as usize] = 420.;
        assert!(!preserves_resource_approaches(
            std::slice::from_ref(&object),
            &frame,
            (&previous, &heights),
            (&candidate, &candidate),
            &policy,
        ));
    }
}

#[test]
fn repeated_material_edits_reuse_existing_blend_records() {
    let n = 8_usize;
    let area = n * n;
    let mut w = crate::formats::binary::Writer::new();
    w.u32(area as u32);
    w.bytes(&vec![0; 8 * area + 8 * n * n.div_ceil(8) + area]);
    for v in [32, 1, 1, 2] {
        w.u32(v);
    }
    for (i, name) in ["Grass_NewYork01", "Rock_Yucatan01"].iter().enumerate() {
        for v in [i as u32 * 16, 16, 4, 0] {
            w.u32(v);
        }
        w.s16(name).unwrap();
    }
    w.bytes(&[205, 205, 205, 205, 0, 0, 0, 0]);
    let mut m = MapFile {
        order: vec![("BlendTileData".into(), 1)],
        table: BTreeMap::from([(1, "BlendTileData".into())]),
        chunks: vec![map::Chunk {
            name_idx: 1,
            version: 27,
            payload: w.data,
        }],
    };
    let tiles = (0..area)
        .map(|i| if i % n >= n / 2 { 64 } else { 0 })
        .collect::<Vec<_>>();
    for _ in 0..2 {
        let bt = BlendTiles::parse(m.get("BlendTileData").unwrap(), n, n).unwrap();
        patch_blend(
            &mut m,
            &bt,
            &tiles,
            &vec![false; area],
            &vec![false; area],
            &vec![true; area],
            n,
        )
        .unwrap();
    }
    let before = m.get("BlendTileData").unwrap().payload.clone();
    let bt = BlendTiles::parse(m.get("BlendTileData").unwrap(), n, n).unwrap();
    assert_eq!(bt.blend_descriptions.len() / 18, 2);
    patch_blend(
        &mut m,
        &bt,
        &tiles,
        &vec![false; area],
        &vec![false; area],
        &vec![true; area],
        n,
    )
    .unwrap();
    assert_eq!(m.get("BlendTileData").unwrap().payload, before);
}

#[test]
fn removing_an_apron_clears_stale_blends_on_unchanged_neighbor_tiles() {
    use crate::formats::binary::Writer;
    let n = 8_usize;
    let area = n * n;
    let mut w = Writer::new();
    w.u32(area as u32);
    for array in 0..4 {
        for _ in 0..area {
            w.u16(u16::from(array != 0));
        }
    }
    w.bytes(&vec![0; 8 * n * n.div_ceil(8) + area]);
    for value in [0, 2, 1, 1] {
        w.u32(value);
    }
    w.bytes(&[0; 16]);
    w.s16("Grass_CapeCod05").unwrap();
    w.bytes(&[0; 8]);
    w.u32(0);
    w.bytes(&[1, 0, 0, 0, 0, 0]);
    w.u32(u32::MAX);
    w.u32(0x7ada0000);
    let mut m = MapFile {
        order: vec![("BlendTileData".into(), 1)],
        table: BTreeMap::from([(1, "BlendTileData".into())]),
        chunks: vec![map::Chunk {
            name_idx: 1,
            version: 27,
            payload: w.data,
        }],
    };
    let bt = BlendTiles::parse(m.get("BlendTileData").unwrap(), n, n).unwrap();
    let mut changed = vec![false; area];
    changed[4 * n + 4] = true;
    patch_blend(
        &mut m,
        &bt,
        &vec![0; area],
        &bt.impassability,
        &bt.buildability,
        &changed,
        n,
    )
    .unwrap();
    let data = &m.get("BlendTileData").unwrap().payload;
    for array in 0..3 {
        let value = |x: usize, y: usize| {
            let at = 4 + 2 * area + 2 * area * array + 2 * ((n - 1 - y) * n + x);
            u16::from_le_bytes(data[at..at + 2].try_into().unwrap())
        };
        assert_eq!(value(4, 4), 0);
        assert_eq!(value(3, 4), 0, "unchanged neighbor retained a ghost border");
        assert_eq!(value(4, 5), 0);
        assert_eq!(value(0, 0), 1, "unrelated blends must remain intact");
    }
}
#[test]
fn rotated_cut_preserves_outside_and_rejects_protected_ground() {
    let n = 128;
    let mut z = vec![210.; n * n];
    for y in 0..n {
        for x in 64..n {
            z[y * n + x] = 280.;
        }
    }
    let old = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"w":25.,"h":25.,"span":25.,"width":25.,"pair":[210.,280.]});
    let mut new = old.clone();
    new["ux"] = json!(0.9);
    new["uy"] = json!(0.43589);
    let mut locks = vec![false; n * n];
    let patch = sculpt(&z, (&z, &z), n, &old, &new, &locks, 0.0390625).unwrap();
    assert!(!patch.is_empty());
    assert!(
        patch
            .iter()
            .all(|(i, _)| old_foot(&old, (i % n) as f64, (i / n) as f64, 0.)
                || contains(&new, (i % n) as f64, (i / n) as f64, 0.))
    );
    locks[patch[0].0] = true;
    assert!(sculpt(&z, (&z, &z), n, &old, &new, &locks, 0.0390625).is_none());
}
#[test]
fn full_lane_validation_catches_off_center_cliff() {
    let n = 128;
    let r = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"span":25.,"width":25.,"pair":[210.,280.]});
    let z: Vec<_> = (0..n * n)
        .map(|i| 210. + 70. * smooth((i % n) as f64 / 25. - 51.5 / 25.))
        .collect();
    assert!(validate_shape(&z, n, &r, 0).is_some());
    let mut broken = z;
    // This cliff lies outside the old ten-cell-wide validation corridor.
    broken[74 * n + 64] += 40.;
    assert!(validate_shape(&broken, n, &r, 0).is_none());
}
#[test]
fn minimum_width_and_exactly_two_endpoints() {
    let mut r = json!({"pair":[210.,350.],"width":20.});
    assert_eq!(terrain::ramp_width(&r).unwrap(), 20.);
    assert_eq!(terrain::ramp_axes(&r).unwrap().2, 210.);
    r["width"] = json!(19.99);
    assert!(terrain::ramp_width(&r).is_err());
    for pair in [json!([210., 280., 350.]), json!([210., 210.]), json!([])] {
        r["pair"] = pair;
        assert!(terrain::ramp_axes(&r).is_err());
    }
}
#[test]
fn direct_first_to_third_connection_avoids_intermediate_platform() {
    let n = 128;
    let r = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"w":25.,"h":25.,"span":40.,"width":20.,"auto":true,"style":"mountain","surface_style":"mountain","pair":[210.,350.]});
    let base: Vec<_> = (0..n * n)
        .map(|i| if i % n < 64 { 210. } else { 350. })
        .collect();
    let locks = vec![false; n * n];
    let patch = sculpt(&base, (&base, &base), n, &r, &r, &locks, 0.0390625).unwrap();
    let mut slope = base.clone();
    for (i, h) in patch {
        slope[i] = h;
    }
    let metrics = validate_shape(&slope, n, &r, 0).unwrap();
    assert_eq!(metrics["width_cells"], 20.);
    assert_eq!(metrics["parallel_lanes"], 21);
    let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
    let mut carved = base.clone();
    terrain::carve(&mut carved, n, std::slice::from_ref(&r), &p).unwrap();
    // The declared width also controls carving when the old rectangle is
    // smaller; shoulders cannot stand in for the required full slope.
    let mut small_box = r.clone();
    small_box["w"] = json!(8.);
    small_box["h"] = json!(8.);
    let mut wide = base.clone();
    terrain::carve(&mut wide, n, &[small_box], &p).unwrap();
    for y in 54..=74 {
        assert!((wide[y * n + 64] - 280.).abs() < 0.001);
    }
    // A second-level platform in the opening must fail even though 280 is
    // between the two endpoint heights, and even if it is off the axis.
    let mut third = base.clone();
    third[72 * n + 64] = 280.;
    assert!(sculpt(&third, (&third, &third), n, &r, &r, &locks, 0.0390625).is_none());
    assert!(terrain::carve(&mut third, n, &[r], &p).is_err());
}
