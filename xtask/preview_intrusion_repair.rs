//! Reproducible, scoped repair of manually reviewed preview intrusions.
use ra3maps::{
    Result, err,
    formats::{archive, npy},
    pipeline::project::write_json,
    render::Image,
    terrain_preview,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::Path,
};
const W: usize = 840;
const SNAP: &str = "raw/before-intrusion-repair-2026-10-30";
fn neighbors(i: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (i % W, i / W);
    [
        (x > 20).then(|| i - 1),
        (x < 819).then_some(i + 1),
        (y > 20).then(|| i - W),
        (y < 819).then_some(i + W),
    ]
    .into_iter()
    .flatten()
}
fn disk(r: isize) -> Vec<(isize, isize)> {
    (-r..=r)
        .flat_map(|y| (-r..=r).filter_map(move |x| (x * x + y * y <= r * r).then_some((x, y))))
        .collect()
}
fn opening(a: &[u8], class: u8, b: [usize; 4], r: isize) -> Vec<usize> {
    let d = disk(r);
    let mut eroded = vec![false; a.len()];
    for y in b[1].saturating_sub(r as usize)..(b[3] + r as usize).min(W) {
        for x in b[0].saturating_sub(r as usize)..(b[2] + r as usize).min(W) {
            eroded[y * W + x] = d.iter().all(|&(dx, dy)| {
                let (xx, yy) = (x as isize + dx, y as isize + dy);
                xx >= 0
                    && yy >= 0
                    && xx < W as isize
                    && yy < W as isize
                    && a[yy as usize * W + xx as usize] == class
            });
        }
    }
    let mut out = vec![];
    for y in b[1]..b[3] {
        for x in b[0]..b[2] {
            if a[y * W + x] == class
                && !d.iter().any(|&(dx, dy)| {
                    let (xx, yy) = (x as isize + dx, y as isize + dy);
                    xx >= 0
                        && yy >= 0
                        && xx < W as isize
                        && yy < W as isize
                        && eroded[yy as usize * W + xx as usize]
                })
            {
                out.push(y * W + x);
            }
        }
    }
    out
}
fn merge(a: &mut [u8], cells: &[usize]) -> Result<Value> {
    let mut mask = vec![false; a.len()];
    for &i in cells {
        mask[i] = true;
    }
    let mut groups = vec![];
    for &s in cells {
        if !mask[s] {
            continue;
        }
        let class = a[s];
        let mut q = VecDeque::from([s]);
        mask[s] = false;
        let mut component = vec![];
        let mut votes = [0usize; 6];
        while let Some(i) = q.pop_front() {
            component.push(i);
            for k in neighbors(i) {
                if a[k] != class {
                    votes[a[k] as usize] += 1;
                } else if mask[k] {
                    mask[k] = false;
                    q.push_back(k);
                }
            }
        }
        let target = (0..6)
            .filter(|&l| l != class as usize)
            .max_by_key(|&l| votes[l])
            .unwrap();
        if votes[target] == 0 {
            return Err(err("local merge has no adjoining target"));
        }
        for &i in &component {
            a[i] = target as u8;
        }
        groups.push(json!({"from":class,"to":target,"cells":component.len(),"shared_edges_before":votes,"changed_full_grid_indices":component}));
    }
    Ok(json!(groups))
}
fn shore(a: &[u8]) -> Vec<(Vec<usize>, [usize; 6], bool)> {
    let mut seen = vec![false; a.len()];
    let mut out = vec![];
    for y in 20..820 {
        for x in 20..820 {
            let s = y * W + x;
            if seen[s] || a[s] != 0 {
                continue;
            }
            let mut q = VecDeque::from([s]);
            seen[s] = true;
            let (mut cells, mut votes, mut open) = (vec![], [0; 6], false);
            while let Some(i) = q.pop_front() {
                cells.push(i);
                open |= i % W == 20 || i % W == 819 || i / W == 20 || i / W == 819;
                for k in neighbors(i) {
                    if a[k] > 0 {
                        votes[a[k] as usize] += 1;
                    } else if !seen[k] {
                        seen[k] = true;
                        q.push_back(k);
                    }
                }
            }
            out.push((cells, votes, open));
        }
    }
    out
}
fn detached_fragments(
    old: &[u8],
    now: &mut [u8],
    minimum: usize,
    scopes: &[[usize; 4]],
) -> Result<Value> {
    let mut seen = vec![false; now.len()];
    let mut removed = Vec::new();
    // Only newly detached land fragments adjoining this repair are eligible.
    // Existing small islands and distant components are outside this task.
    for y in 20..820 {
        for x in 20..820 {
            let start = y * W + x;
            if now[start] == 0 || seen[start] {
                continue;
            }
            let class = now[start];
            let mut q = VecDeque::from([start]);
            seen[start] = true;
            let mut cells = Vec::new();
            let mut detached = false;
            while let Some(i) = q.pop_front() {
                cells.push(i);
                for k in neighbors(i) {
                    detached |= old[k] == class && now[k] != class;
                    if now[k] == class && !seen[k] {
                        seen[k] = true;
                        q.push_back(k);
                    }
                }
            }
            let local = cells.iter().all(|&i| {
                scopes
                    .iter()
                    .any(|b| i % W >= b[0] && i % W < b[2] && i / W >= b[1] && i / W < b[3])
            });
            if detached && local && cells.len() < minimum {
                removed.extend(cells);
            }
        }
    }
    merge(now, &removed)
}
fn pack(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    archive::zip(
        &files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>(),
    )
}
fn gallery(root: &Path, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let font = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ]
    .into_iter()
    .find_map(|p| {
        fs::read(p)
            .ok()
            .and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
    });
    let colors = [
        [53, 116, 150],
        [104, 151, 89],
        [160, 178, 99],
        [202, 180, 119],
        [162, 128, 103],
        [218, 221, 211],
    ];
    let mut panels = Vec::new();
    let mut paths = fs::read_dir(root.join("maps"))?.collect::<std::io::Result<Vec<_>>>()?;
    paths.sort_by_key(|p| p.file_name());
    for p in paths {
        let base = p.path().join("data/terrain-preview");
        if !base.join("intrusion-repair-2026-10-30.json").is_file() {
            continue;
        }
        let report: Value =
            serde_json::from_slice(&fs::read(base.join("intrusion-repair-2026-10-30.json"))?)?;
        let before = archive::unzip(&fs::read(base.join(SNAP).join("terrain.npz"))?)?;
        let after = archive::unzip(&fs::read(base.join("terrain.npz"))?)?;
        let old = npy::Npy::parse(&before["layers.npy"])?.numbers()?;
        let now = npy::Npy::parse(&after["layers.npy"])?.numbers()?;
        for site in report["reviewed_sites"].as_array().unwrap() {
            let b = &site["before"]["bounds_full_grid"];
            let cx = (b[0].as_i64().unwrap() + b[2].as_i64().unwrap()) / 2;
            let cy = (b[1].as_i64().unwrap() + b[3].as_i64().unwrap()) / 2;
            let mut tile = Image::new(300, 144, [26, 32, 37]);
            for (index, a) in [&old, &now].iter().enumerate() {
                for y in 0..48 {
                    for x in 0..48 {
                        let (xx, yy) = (cx + x - 24, cy + y - 24);
                        if !(20..820).contains(&xx) || !(20..820).contains(&yy) {
                            continue;
                        }
                        let color = colors[a[yy as usize * W + xx as usize] as usize];
                        for dy in 0..3 {
                            for dx in 0..3 {
                                tile.pixel(
                                    index as isize * 156 + x as isize * 3 + dx,
                                    y as isize * 3 + dy,
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            panels.push((
                format!(
                    "{} #{}  修订前 / 修订后",
                    report["map_id"].as_str().unwrap(),
                    site["candidate_id"]
                ),
                tile,
            ));
        }
    }
    for (batch, panels) in panels.chunks(12).enumerate() {
        let mut sheet = Image::new(640, panels.len().div_ceil(2) * 184, [26, 32, 37]);
        for (i, (label, tile)) in panels.iter().enumerate() {
            let (x, y) = ((i % 2) * 320, (i / 2) * 184);
            sheet.text(
                font.as_ref(),
                x as isize + 5,
                y as isize + 2,
                label,
                14.,
                [240, 240, 240],
            );
            for yy in 0..144 {
                for xx in 0..300 {
                    let j = (yy * 300 + xx) * 3;
                    sheet.pixel(
                        (x + xx + 5) as isize,
                        (y + yy + 28) as isize,
                        tile.rgb[j..j + 3].try_into().unwrap(),
                    );
                }
            }
        }
        sheet.png(&output.join(format!("before-after-{}.png", batch + 1)))?;
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--gallery" {
        return gallery(Path::new(&args[2]), Path::new(&args[3]));
    }
    if args.len() != 3 {
        return Err(err(
            "preview-intrusion-repair <stage-root> <reviewed-sites.json>",
        ));
    }
    let root = Path::new(&args[1]);
    let sites: Vec<Value> = serde_json::from_slice(&fs::read(&args[2])?)?;
    let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for s in sites {
        grouped
            .entry(s["map_id"].as_str().unwrap().into())
            .or_default()
            .push(s);
    }
    for (id, sites) in grouped {
        let base = root.join("maps").join(&id).join("data/terrain-preview");
        let baseline = fs::read(base.join(SNAP).join("terrain.npz"))?;
        let mut files = archive::unzip(&baseline)?;
        let a = npy::Npy::parse(&files["layers.npy"])?;
        if a.shape != [W, W] {
            return Err(err("expected840"));
        }
        let old: Vec<u8> = a.numbers()?.iter().map(|&v| v as u8).collect();
        let mut now = old.clone();
        let report: Value =
            serde_json::from_slice(&fs::read(base.join(SNAP).join("report.json"))?)?;
        let mut params: Value =
            serde_json::from_slice(&fs::read(base.join(SNAP).join("parameters.json"))?)?;
        let candidates: Value =
            serde_json::from_slice(&fs::read(base.join("width-candidates-2026-10-30.json"))?)?;
        let mut records = vec![];
        for s in sites {
            let c = candidates["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == s["candidate_id"])
                .ok_or_else(|| err("candidate missing"))?;
            let b = &s["bounds_full_grid"];
            let b = [
                (b[0].as_u64().unwrap() as usize).saturating_sub(6).max(20),
                (b[1].as_u64().unwrap() as usize).saturating_sub(6).max(20),
                (b[2].as_u64().unwrap() as usize + 6).min(820),
                (b[3].as_u64().unwrap() as usize + 6).min(820),
            ];
            let class = c["class"].as_u64().unwrap() as u8;
            let cells = opening(&now, class, b, 3);
            let changes = merge(&mut now, &cells)?;
            records.push(json!({"candidate_id":s["candidate_id"],"before":c,"scope_xy_exclusive":b,"design_opening_radius_cells":3,"merges":changes}));
        }
        let cleanup_scopes: Vec<[usize; 4]> = records
            .iter()
            .map(|r| {
                let b = &r["scope_xy_exclusive"];
                [
                    (b[0].as_u64().unwrap() as usize).saturating_sub(6).max(20),
                    (b[1].as_u64().unwrap() as usize).saturating_sub(6).max(20),
                    (b[2].as_u64().unwrap() as usize + 6).min(820),
                    (b[3].as_u64().unwrap() as usize + 6).min(820),
                ]
            })
            .collect();
        let fragment_merges = detached_fragments(
            &old,
            &mut now,
            report["processing"]["minimum_component_area_cells"]
                .as_u64()
                .ok_or_else(|| err("missing existing component threshold"))? as usize,
            &cleanup_scopes,
        )?;
        // Reconcile newly exposed shores with their prior dominant surrounding tier.
        let before_bodies = shore(&old);
        let mut enclosure = vec![];
        for (cells, votes, open) in shore(&now) {
            if open || votes.iter().filter(|&&n| n > 0).count() <= 1 {
                continue;
            }
            let first = cells[0];
            let prior = before_bodies
                .iter()
                .find(|(c, _, _)| c.contains(&first))
                .ok_or_else(|| err("new waterbody has no prior owner"))?;
            let owner = (1..6).max_by_key(|&l| prior.1[l]).unwrap() as u8;
            let mut changed = vec![];
            for i in cells {
                for k in neighbors(i) {
                    if now[k] > 0 && now[k] != owner {
                        for (dx, dy) in disk(4) {
                            let (x, y) = (k % W, k / W);
                            let (xx, yy) = (x as isize + dx, y as isize + dy);
                            if xx < 20 || yy < 20 || xx >= 820 || yy >= 820 {
                                continue;
                            }
                            let j = yy as usize * W + xx as usize;
                            if now[j] > 0 && now[j] != owner {
                                now[j] = owner;
                                changed.push(j);
                            }
                        }
                    }
                }
            }
            changed.sort();
            changed.dedup();
            enclosure.push(json!({"owner":owner,"changed_full_grid_indices":changed}));
        }
        let heights: Vec<f32> = serde_json::from_value(report["game_layer_heights"].clone())?;
        let z: Vec<u8> = now
            .iter()
            .flat_map(|&l| {
                if l == 0 {
                    130f32
                } else {
                    heights[l as usize - 1]
                }
                .to_le_bytes()
            })
            .collect();
        files.insert("layers.npy".into(), npy::encode("|u1", &[W, W], &now));
        files.insert(
            "water_mask.npy".into(),
            npy::encode(
                "|u1",
                &[W, W],
                &now.iter().map(|&l| (l == 0) as u8).collect::<Vec<_>>(),
            ),
        );
        files.insert("game_heights.npy".into(), npy::encode("<f4", &[W, W], &z));
        fs::write(base.join("intrusion-reference.npz"), pack(&files)?)?;
        write_json(&base.join("intrusion-reference-report.json"), &report)?;
        params["water_design_land_reference"] = json!({"terrain":"intrusion-reference.npz","report":"intrusion-reference-report.json","use_reference_water_mask":true});
        params["terrain_shape_repair"] = json!({"report":"intrusion-repair-2026-10-30.json","baseline":SNAP,"method":"scoped disk opening and adjacent-terrain merges, followed by same-owner shore reconciliation","combat_routes":"not yet designated"});
        write_json(&base.join("parameters.json"), &params)?;
        let source = root.join(report["source_reference"].as_str().unwrap());
        let current = archive::unzip(&fs::read(base.join("terrain.npz"))?)?;
        if current["layers.npy"] != files["layers.npy"]
            || current["water_mask.npy"] != files["water_mask.npy"]
        {
            terrain_preview::generate(root, &id, &source, None)?;
        }
        let actual = archive::unzip(&fs::read(base.join("terrain.npz"))?)?;
        if actual["layers.npy"] != files["layers.npy"]
            || actual["water_mask.npy"] != files["water_mask.npy"]
        {
            return Err(err("native regeneration differs from design arrays"));
        }
        let changed: Vec<_> = old
            .iter()
            .zip(&now)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        let water = old
            .iter()
            .zip(&now)
            .filter(|(a, b)| (**a == 0) != (**b == 0))
            .count();
        let evidence = json!({"date":"2026-10-30","map_id":id,"baseline":SNAP,"terrain_crc32":crc32fast::hash(&fs::read(base.join("terrain.npz"))?),"policy":serde_json::from_slice::<Value>(&fs::read(root.join("rules/coordinate-pipeline.json"))?)?["terrain_shape"],"reviewed_sites":records,"detached_fragment_merges":fragment_merges,"shore_reconciliation":enclosure,"changed_cells":changed.len(),"changed_water_cells":water,"changed_full_grid_indices":changed,"combat_route_width_verified":false,"gift_placement_applicable":false,"native_playable_data_modified":false});
        write_json(&base.join("intrusion-repair-2026-10-30.json"), &evidence)?;
        println!("{id}: {} changed, {water} water", changed.len());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opening_removes_long_thin_tail_but_preserves_body() {
        let mut a = vec![2; W * W];
        for y in 60..90 {
            for x in 60..90 {
                a[y * W + x] = 1
            }
        }
        for x in 90..110 {
            a[74 * W + x] = 1;
            a[75 * W + x] = 1;
        }
        let removed = opening(&a, 1, [55, 55, 115, 95], 3);
        assert!(removed.contains(&(74 * W + 100)));
        assert!(!removed.contains(&(70 * W + 70)));
        let before = a.clone();
        merge(&mut a, &removed).unwrap();
        assert_eq!(a[74 * W + 100], 2);
        assert_eq!(a[70 * W + 70], 1);
        assert!(
            (0..a.len())
                .filter(|&i| before[i] != a[i])
                .all(|i| removed.contains(&i))
        );
    }
    #[test]
    fn wide_neck_survives_opening() {
        let mut a = vec![2; W * W];
        for y in 60..90 {
            for x in 60..110 {
                a[y * W + x] = 1
            }
        }
        let removed = opening(&a, 1, [65, 65, 105, 85], 3);
        assert!(removed.is_empty());
    }
    #[test]
    fn fragment_cleanup_requires_full_component_inside_local_scope() {
        let mut old = vec![2; W * W];
        for y in 60..80 {
            for x in 60..90 {
                old[y * W + x] = 1;
            }
        }
        for x in 90..99 {
            old[70 * W + x] = 1;
        }
        for y in 69..72 {
            for x in 96..99 {
                old[y * W + x] = 1;
            }
        }
        let mut cut = old.clone();
        for x in 90..96 {
            cut[70 * W + x] = 2;
        }
        let mut limited = cut.clone();
        detached_fragments(&old, &mut limited, 2500, &[[90, 68, 98, 73]]).unwrap();
        assert_eq!(limited, cut);
        detached_fragments(&old, &mut cut, 2500, &[[90, 68, 100, 73]]).unwrap();
        assert_eq!(cut[70 * W + 97], 2);
        assert_eq!(cut[70 * W + 75], 1);
    }
}
