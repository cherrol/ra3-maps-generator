//! Read-only comparison against the selected PNG. It never generates playable terrain.
use ra3maps::{
    Result, err,
    formats::{
        archive,
        map::{self, HeightMap, MapFile},
        npy, sha256,
    },
    pipeline::project::write_json,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, fs, io::Read, path::Path};
const N: usize = 840;
const B: usize = 20;
const COLORS: [[u8; 3]; 5] = [
    [53, 116, 150],
    [104, 151, 89],
    [160, 178, 99],
    [202, 180, 119],
    [162, 128, 103],
];
fn png() -> Result<(Vec<u8>, usize, usize)> {
    let bytes = fs::read("maps/广西/preview.png")?;
    let mut pos = 8;
    let mut compressed = Vec::new();
    let (mut w, mut h) = (0, 0);
    while pos < bytes.len() {
        let n = u32::from_be_bytes(bytes[pos..pos + 4].try_into()?) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let d = &bytes[pos + 8..pos + 8 + n];
        if kind == b"IHDR" {
            w = u32::from_be_bytes(d[..4].try_into()?) as usize;
            h = u32::from_be_bytes(d[4..8].try_into()?) as usize;
            if d[8..] != [8, 2, 0, 0, 0] {
                return Err(err("expected RGB8 PNG"));
            }
        }
        if kind == b"IDAT" {
            compressed.extend(d);
        }
        pos += n + 12;
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&compressed[..]).read_to_end(&mut raw)?;
    let stride = w * 3;
    let mut pixels = vec![0u8; stride * h];
    for y in 0..h {
        let filter = raw[y * (stride + 1)];
        for x in 0..stride {
            let a = if x >= 3 {
                pixels[y * stride + x - 3]
            } else {
                0
            };
            let b = if y > 0 {
                pixels[(y - 1) * stride + x]
            } else {
                0
            };
            let c = if x >= 3 && y > 0 {
                pixels[(y - 1) * stride + x - 3]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => {
                    let p = a as i32 + b as i32 - c as i32;
                    let ds = [
                        (p - a as i32).abs(),
                        (p - b as i32).abs(),
                        (p - c as i32).abs(),
                    ];
                    if ds[0] <= ds[1] && ds[0] <= ds[2] {
                        a
                    } else if ds[1] <= ds[2] {
                        b
                    } else {
                        c
                    }
                }
                _ => return Err(err("unknown filter")),
            };
            pixels[y * stride + x] = raw[y * (stride + 1) + 1 + x].wrapping_add(predictor);
        }
    }
    Ok((pixels, w, h))
}
fn initial() -> Result<Vec<u8>> {
    let (rgb, w, h) = png()?;
    assert_eq!((w, h), (1104, 1002));
    let mut a = vec![0; N * N];
    for y in 0..N {
        for x in 0..N {
            let px = 28 + x.clamp(B, N - B - 1) - B;
            let py = 112 + y.clamp(B, N - B - 1) - B;
            let c = &rgb[(py * w + px) * 3..(py * w + px) * 3 + 3];
            let mut best = (i32::MAX, 0u8);
            for (k, col) in COLORS.iter().enumerate() {
                for shade in [1.0, 0.77] {
                    let d = (0..3)
                        .map(|i| (c[i] as i32 - (col[i] as f64 * shade) as i32).pow(2))
                        .sum();
                    if d < best.0 {
                        best = (d, k as u8);
                    }
                }
            }
            a[y * N + x] = best.1;
        }
    }
    Ok(a)
}
fn neighbors(i: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (i % N, i / N);
    [
        (x > B).then(|| i - 1),
        (x + 1 < N - B).then_some(i + 1),
        (y > B).then(|| i - N),
        (y + 1 < N - B).then_some(i + N),
    ]
    .into_iter()
    .flatten()
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let file = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("assets/广西/[CHAI]碧嶂回澜T0.0.1/[CHAI]碧嶂回澜T0.0.1.map");
    let config = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("maps/广西/config.json");
    let out = Path::new("maps/广西/data/authoring-handoff/2026-10-30");
    fs::create_dir_all(out)?;
    let (a, known, counts) = write_reference(out)?;
    if args.get(1).is_some_and(|s| s == "--reference-only") {
        println!(
            "Locked reference recreated from the unchanged PNG; unknown geometry remains unknown."
        );
        return Ok(());
    }
    let cfg: Value = serde_json::from_slice(&fs::read(config)?)?;
    let m = MapFile::load(file)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    assert_eq!((hm.width, hm.height, hm.border), (840, 840, 20));
    let scale = cfg["Z_SCALE"].as_f64().unwrap();
    let cell_size = cfg["CELL"].as_f64().unwrap();
    let water_z = cfg["WATER_Z"].as_f64().unwrap();
    let z: Vec<_> = ra3maps::terrain::flip(&hm.elevations, N)
        .into_iter()
        .map(|v| v as f64 * scale)
        .collect();
    let layout: Value = serde_json::from_slice(&fs::read(
        Path::new(file).parent().unwrap().join("source/layout.json"),
    )?)?;
    let ramps = layout["ramps"].as_array().unwrap();
    let mut changes = Vec::new();
    let mut by_ramp = vec![0usize; ramps.len()];
    let mut other_land_changes = 0;
    let mut water_differences = Vec::new();
    let mut max_delta = 0f64;
    let (mut water_min, mut water_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for y in B..N - B {
        for x in B..N - B {
            let i = y * N + x;
            if (a[i] == 0) != (z[i] < water_z) {
                water_differences.push([x, y]);
            }
            if a[i] == 0 {
                water_min = water_min.min(z[i]);
                water_max = water_max.max(z[i]);
                continue;
            }
            let d = z[i] - known[i];
            if d.abs() > 0.05 {
                max_delta = max_delta.max(d.abs());
                let r = ramps
                    .iter()
                    .position(|r| ra3maps::terrain::ramp_cover(i, N, r, 0.));
                if let Some(r) = r {
                    by_ramp[r] += 1;
                } else {
                    other_land_changes += 1;
                }
                changes.push(json!({"cell":[x,y],"preview_layer":a[i],"preview_height":known[i],"candidate_height":z[i],"delta":d,"candidate_ramp":r.map(|r|r+1)}));
            }
        }
    }
    let mut comp = vec![0usize; N * N];
    let mut regions = Vec::new();
    let mut id = 0;
    for y in B..N - B {
        for x in B..N - B {
            let i = y * N + x;
            if comp[i] != 0 {
                continue;
            }
            id += 1;
            let l = a[i];
            let mut q = VecDeque::new();
            comp[i] = id;
            q.push_back(i);
            let (mut n, mut west, mut east) = (0, 0, 0);
            let (mut lo, mut hi) = ([x, y], [x, y]);
            while let Some(j) = q.pop_front() {
                n += 1;
                if j % N < N / 2 {
                    west += 1;
                } else {
                    east += 1;
                }
                lo[0] = lo[0].min(j % N);
                lo[1] = lo[1].min(j / N);
                hi[0] = hi[0].max(j % N);
                hi[1] = hi[1].max(j / N);
                for k in neighbors(j) {
                    if comp[k] == 0 && a[k] == l {
                        comp[k] = id;
                        q.push_back(k);
                    }
                }
            }
            regions.push(json!({"id":id,"layer":l,"cells":n,"west_cells":west,"east_cells":east,"bounds":[lo,hi]}));
        }
    }
    let bases = layout["base_cells"].as_array().unwrap();
    let bidx: Vec<_> = bases
        .iter()
        .map(|b| {
            b[1].as_f64().unwrap().round() as usize * N + b[0].as_f64().unwrap().round() as usize
        })
        .collect();
    let plan: Value = serde_json::from_slice(&fs::read(
        Path::new(config)
            .parent()
            .unwrap()
            .join("authoring/resource-distribution.json"),
    )?)?;
    let routes=plan["routes"].as_array().unwrap().iter().map(|r|{let sa=r["side_a_seat"].as_u64().unwrap()as usize;let sb=r["side_b_seat"].as_u64().unwrap()as usize;json!({"route":r["id"],"seats":[sa,sb],"start_layer":a[bidx[sa-1]],"end_layer":a[bidx[sb-1]],"start_component":comp[bidx[sa-1]],"end_component":comp[bidx[sb-1]],"same_declared_plateau_component":comp[bidx[sa-1]]==comp[bidx[sb-1]],"ground_connection_status":"not established by selected preview; no slope geometry adopted"})}).collect::<Vec<_>>();
    let groups: Value = serde_json::from_slice(&fs::read(
        Path::new(config)
            .parent()
            .unwrap()
            .join("histories/T0.0.1/gift-road-planning.json"),
    )?)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let mut disconnected_gifts = Vec::new();
    for row in groups["moves"].as_array().unwrap() {
        let name = row["waypoint"].as_str().unwrap();
        let seat = row["seat"].as_u64().unwrap() as usize;
        let o = objects
            .iter()
            .find(|o| map::prop(&o.props, "waypointName").and_then(map::Value::text) == Some(name))
            .unwrap();
        let p = [
            o.position[0] as f64 / cell_size + B as f64,
            (N - 1 - B) as f64 - o.position[1] as f64 / cell_size,
        ];
        let i = p[1].round() as usize * N + p[0].round() as usize;
        if comp[i] != comp[bidx[seat - 1]] {
            disconnected_gifts.push(json!({"waypoint":name,"seat":seat,"cell":p,"base_layer":a[bidx[seat-1]],"gift_layer":a[i],"base_component":comp[bidx[seat-1]],"gift_component":comp[i]}));
        }
    }
    write_json(
        &out.join("candidate-terrain-diff.json"),
        &json!({"schema_version":1,"candidate":file,"candidate_native_crc32":crc32fast::hash(&m.encode()?),"candidate_file_sha256":sha256::hex(&fs::read(file)?),"status":"withdrawn_terrain_mismatch","passed":false,"reason":"All 13 ramps and shoreline grading were introduced during playable authoring, rather than handed off in the selected preview. Earlier checks that whitelisted these changes do not establish preview conformance.","new_ramp_count":ramps.len(),"new_ramps":ramps,"changed_known_land_height_cells":changes.len(),"changed_cells_by_candidate_ramp":by_ramp,"changed_land_cells_outside_candidate_ramps":other_land_changes,"maximum_land_height_delta":max_delta,"water_footprint_difference_cells":water_differences,"candidate_water_heights":[water_min,water_max],"candidate_water_plane":water_z,"water_height_design_status":"invented in candidate; no corresponding depth design in selected preview","changed_land_samples":changes,"existing_layout_routes_on_unmodified_reference":routes,"gifts_in_different_source_components_from_base":disconnected_gifts,"source_regions":regions,"rule":"docs/spec/preview_terrain_lock.md","earlier_machine_passes_are_not_delivery_approval":true}),
    )?;
    println!(
        "PNG locked: {} land cells; {} water cells with undefined depth",
        counts[1..].iter().sum::<usize>(),
        counts[0]
    );
    println!(
        "Withdraw candidate: {} added ramps; {} modified land samples ({} outside ramps); {} cross-component gift points",
        ramps.len(),
        changes.len(),
        other_land_changes,
        disconnected_gifts.len()
    );
    Ok(())
}

fn write_reference(out: &Path) -> Result<(Vec<u8>, Vec<f64>, [usize; 5])> {
    let a = initial()?;
    let mut classes = vec![255u8; N * N];
    let mut valid = vec![0u8; N * N];
    let mut wet = vec![0u8; N * N];
    let mut known = vec![f64::NAN; N * N];
    let mut counts = [0usize; 5];
    for y in B..N - B {
        for x in B..N - B {
            let i = y * N + x;
            classes[i] = a[i];
            valid[i] = 1;
            wet[i] = u8::from(a[i] == 0);
            counts[a[i] as usize] += 1;
            known[i] = match a[i] {
                1 => 210.,
                2 => 280.,
                3 => 350.,
                4 => 420.,
                _ => f64::NAN,
            };
        }
    }
    let pack = archive::zip(&[
        (
            "layer_class.npy".into(),
            npy::encode("|u1", &[N, N], &classes),
        ),
        (
            "source_cell_valid.npy".into(),
            npy::encode("|u1", &[N, N], &valid),
        ),
        ("water_mask.npy".into(), npy::encode("|u1", &[N, N], &wet)),
        (
            "known_land_height.npy".into(),
            npy::encode(
                "<f8",
                &[N, N],
                &known
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            ),
        ),
    ])?;
    fs::write(out.join("locked-reference.npz"), &pack)?;
    let report: Value =
        serde_json::from_slice(&fs::read("maps/广西/data/terrain-preview/report.json")?)?;
    let mut images = Vec::new();
    for name in [
        "preview.png",
        "preview--line.png",
        "preview--center--line.png",
        "source.png",
    ] {
        let bytes = fs::read(Path::new("maps/广西").join(name))?;
        let crc = crc32fast::hash(&bytes);
        images.push(json!({"path":format!("maps/广西/{name}"),"crc32":crc,"sha256":sha256::hex(&bytes),"bytes":bytes.len(),"matches_existing_preview_image_identity":report["image_crc32"][name]==crc}));
    }
    write_json(
        &out.join("preview-lock.json"),
        &json!({"schema_version":1,"map":"广西","status":"awaiting_same_version_terrain_design","selected_preview":"maps/广西/preview.png","images":images,"import":"fresh palette and boundary-shade classification from the current PNG; no NPZ cache, historical native source or earlier layout used as terrain input","pixel_crop":[28,112,800,800],"grid_offset":[20,20],"full_grid":[840,840],"playable_grid":[800,800],"layer_counts_in_playable_frame":counts,"known_land_heights":[210.,280.,350.,420.],"water_depth":"unknown in the selected PNG; not inferred from public defaults","ignored_cached_height_arrays":"The existing terrain.npz contains layer-mapped game_heights, including a generator default for water. Per the task, these caches are not terrain input; they do not supply completed ramp or shoreline geometry.","slopes_and_shore_geometry":"not provided by the selected PNG or completed same-version design","reference_npz":{"path":"locked-reference.npz","sha256":sha256::hex(&pack),"does_not_contain_buildable_base_z":true,"unknown_values":"NaN known_land_height; layer_class=255 outside the visible frame","valid_cells":640000,"known_land_height_cells":counts[1..].iter().sum::<usize>(),"unknown_water_height_cells":counts[0]}}),
    )?;

    assert_eq!(counts.iter().sum::<usize>(), 640000);
    assert_eq!(counts, [34967, 178545, 196018, 217912, 12558]);
    Ok((a, known, counts))
}
