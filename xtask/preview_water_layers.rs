//! Inspect enclosed preview waterbodies and regenerate previews with one surrounding land tier.
//! This edits design inputs only; native playable maps and their histories are never rebuilt.
use ra3maps::{
    Result, err,
    formats::{archive, npy},
    pipeline::project,
    terrain_preview,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
};

type Arrays = BTreeMap<String, Vec<u8>>;
const SNAPSHOT: &str = "raw/before-water-layer-enclosure";
const REPORT: &str = "water-layer-verification.json";

#[derive(Clone)]
struct Waterbody {
    cells: Vec<usize>,
    shore: BTreeSet<usize>,
    contacts: [usize; 6],
    open: bool,
    bounds: [usize; 4],
}

fn neighbors(i: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (i % w, i / w);
    [
        (x > 0).then(|| i - 1),
        (x + 1 < w).then_some(i + 1),
        (y > 0).then(|| i - w),
        (y + 1 < h).then_some(i + w),
    ]
    .into_iter()
    .flatten()
}

fn playable(i: usize, w: usize, h: usize, b: usize) -> bool {
    let (x, y) = (i % w, i / w);
    x >= b && y >= b && x < w - b && y < h - b
}

fn bodies(layers: &[u8], w: usize, h: usize, b: usize) -> Vec<Waterbody> {
    let mut seen = vec![false; layers.len()];
    let mut out = Vec::new();
    for start in 0..layers.len() {
        if seen[start] || layers[start] != 0 || !playable(start, w, h, b) {
            continue;
        }
        let mut body = Waterbody {
            cells: vec![],
            shore: BTreeSet::new(),
            contacts: [0; 6],
            open: false,
            bounds: [w, h, 0, 0],
        };
        let mut q = VecDeque::from([start]);
        seen[start] = true;
        while let Some(i) = q.pop_front() {
            let (x, y) = (i % w, i / w);
            body.cells.push(i);
            body.bounds[0] = body.bounds[0].min(x);
            body.bounds[1] = body.bounds[1].min(y);
            body.bounds[2] = body.bounds[2].max(x + 1);
            body.bounds[3] = body.bounds[3].max(y + 1);
            body.open |= x == b || y == b || x == w - b - 1 || y == h - b - 1;
            for k in neighbors(i, w, h).filter(|&k| playable(k, w, h, b)) {
                if layers[k] > 0 {
                    body.contacts[layers[k] as usize] += 1;
                    body.shore.insert(k);
                } else if !seen[k] {
                    seen[k] = true;
                    q.push_back(k);
                }
            }
        }
        out.push(body);
    }
    out
}

fn owner(body: &Waterbody) -> Result<u8> {
    let best = *body.contacts[1..].iter().max().unwrap();
    let owners: Vec<_> = (1..=5).filter(|&l| body.contacts[l] == best).collect();
    if best == 0 || owners.len() != 1 {
        return Err(err(format!(
            "岸线接触量无唯一最多层，须先记录局部连续性选择: {:?}",
            body.bounds
        )));
    }
    Ok(owners[0] as u8)
}

// A rounded local band avoids leaving a one-cell painted rim beside a cliff.
// Radius is an explicit batch design choice, never a new public minimum.
fn enclose(
    old: &[u8],
    bodies: &[Waterbody],
    w: usize,
    h: usize,
    b: usize,
    radius: usize,
) -> Result<(Vec<u8>, Vec<bool>)> {
    let mut result = old.to_vec();
    let mut changed_scope = vec![false; old.len()];
    let mut assigned = vec![0; old.len()];
    let r = radius as isize;
    for body in bodies
        .iter()
        .filter(|c| !c.open && c.contacts[1..].iter().filter(|&&n| n > 0).count() > 1)
    {
        let target = owner(body)?;
        for &i in &body.shore {
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        continue;
                    }
                    let k = ny as usize * w + nx as usize;
                    if old[k] == 0 || !playable(k, w, h, b) {
                        continue;
                    }
                    if assigned[k] != 0 && assigned[k] != target {
                        return Err(err("邻近水域岸带归属冲突，须先调整本图局部方案"));
                    }
                    assigned[k] = target;
                    changed_scope[k] = true;
                    result[k] = target;
                }
            }
        }
    }
    Ok((result, changed_scope))
}

fn array(files: &Arrays, key: &str) -> Result<npy::Npy> {
    npy::Npy::parse(
        files
            .get(&format!("{key}.npy"))
            .ok_or_else(|| err(format!("缺少 {key}")))?,
    )
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn load(files: &Arrays) -> Result<(Vec<u8>, usize, usize)> {
    let a = array(files, "layers")?;
    if a.shape.len() != 2 {
        return Err(err("预览分层须为二维"));
    }
    let values = a.numbers()?;
    if values
        .iter()
        .any(|&x| !(0. ..=5.).contains(&x) || x.fract() != 0.)
    {
        return Err(err("无效分层"));
    }
    Ok((
        values.into_iter().map(|v| v as u8).collect(),
        a.shape[1],
        a.shape[0],
    ))
}

fn updated_cache(
    files: &Arrays,
    layers: &[u8],
    w: usize,
    h: usize,
    report: &Value,
) -> Result<Arrays> {
    let mut out = files.clone();
    let heights: Vec<f32> = serde_json::from_value(report["game_layer_heights"].clone())?;
    if heights.len() != 5 {
        return Err(err("须有五个层高"));
    }
    let z: Vec<u8> = layers
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
    out.insert("layers.npy".into(), npy::encode("|u1", &[h, w], layers));
    out.insert("game_heights.npy".into(), npy::encode("<f4", &[h, w], &z));
    Ok(out)
}

fn pack(files: &Arrays) -> Result<Vec<u8>> {
    archive::zip(
        &files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>(),
    )
}

fn counts(layers: &[u8], w: usize, h: usize, b: usize) -> [usize; 6] {
    let mut c = [0; 6];
    for (i, &l) in layers.iter().enumerate() {
        if l > 0 && playable(i, w, h, b) {
            c[l as usize] += 1;
        }
    }
    c
}

fn contact_json(c: &[usize; 6]) -> Value {
    json!(
        (1..=5)
            .filter(|&l| c[l] > 0)
            .map(|l| (
                l.to_string(),
                json!({"shared_edges":c[l],"length_world":c[l]*10})
            ))
            .collect::<BTreeMap<_, _>>()
    )
}

fn verify(
    before: &Arrays,
    after: &Arrays,
    scope: &[bool],
    w: usize,
    h: usize,
    b: usize,
) -> Result<Value> {
    if before.keys().ne(after.keys()) {
        return Err(err("NPZ 数组清单发生变化"));
    }
    for (key, bytes) in before {
        if key != "layers.npy" && key != "game_heights.npy" && after[key] != *bytes {
            return Err(err(format!("受保护数组发生变化: {key}")));
        }
    }
    let (old, _, _) = load(before)?;
    let (now, _, _) = load(after)?;
    if old
        .iter()
        .zip(&now)
        .enumerate()
        .any(|(i, (&a, &c))| (a != c && !scope[i]) || ((a == 0) != (c == 0)))
    {
        return Err(err("岸带以外层级或水域发生变化"));
    }
    let pre = bodies(&old, w, h, b);
    let post = bodies(&now, w, h, b);
    if pre.len() != post.len() {
        return Err(err("水体连通分量发生变化"));
    }
    let mut land_labels = vec![0usize; now.len()];
    let mut anchored = vec![false];
    for seed in 0..now.len() {
        if now[seed] == 0 || land_labels[seed] != 0 || !playable(seed, w, h, b) {
            continue;
        }
        let label = anchored.len();
        anchored.push(false);
        let mut q = VecDeque::from([seed]);
        land_labels[seed] = label;
        while let Some(i) = q.pop_front() {
            anchored[label] |= !scope[i] && old[i] == now[i];
            for k in neighbors(i, w, h).filter(|&k| playable(k, w, h, b) && now[k] == now[i]) {
                if land_labels[k] == 0 {
                    land_labels[k] = label;
                    q.push_back(k);
                }
            }
        }
    }
    let mut records = Vec::new();
    for (index, (a, c)) in pre.iter().zip(&post).enumerate() {
        let applicable = !a.open;
        let target = if applicable { Some(owner(a)?) } else { None };
        if let Some(target) = target {
            if c.contacts[target as usize] != c.contacts.iter().sum::<usize>() {
                return Err(err(format!("水体 {index} 仍接触多个层")));
            }
            // Each outer/inner shore segment must reach unchanged land in its owner tier.
            let groups: BTreeSet<_> = c.shore.iter().map(|&i| land_labels[i]).collect();
            if groups.iter().any(|&g| g == 0 || !anchored[g]) {
                return Err(err(format!("水体 {index} 岸带未接入未改动的归属层主体")));
            }
            records.push(json!({"id":index,"bounds_xy_exclusive":a.bounds,"water_cells":a.cells.len(),
                "share_of_playable_grid":a.cells.len() as f64/((w-2*b)*(h-2*b)) as f64,
                "touches_frame":false,"applicable":true,"contacts_before":contact_json(&a.contacts),
                "owner_layer":target,"contacts_after":contact_json(&c.contacts),"shore_land_cells":c.shore.len(),
                "shore_land_components":groups.len(),"whole_perimeter_single_layer":true,
                "connected_to_unchanged_owner_land":true,"status":"通过"}));
        } else {
            if a.contacts != c.contacts {
                return Err(err("开放水体接岸层发生变化"));
            }
            records.push(json!({"id":index,"bounds_xy_exclusive":a.bounds,"water_cells":a.cells.len(),
                "touches_frame":true,"applicable":false,"contacts_before":contact_json(&a.contacts),
                "contacts_after":contact_json(&c.contacts),"status":"不适用",
                "reason":"frame-clipped/open water; not a fully enclosed small waterbody; no artificial closure"}));
        }
    }
    Ok(json!({"waterbodies":records,"all_applicable_passed":true,
        "changed_land_cells":old.iter().zip(&now).filter(|(a,c)|a!=c).count(),
        "water_and_source_arrays_preserved":true,"outside_recorded_band_layers_preserved":true,
        "playable_passability_verified":false,
        "playable_passability_status":"待核验：纯预览，缓坡与导航在可玩制作阶段验收"}))
}

fn process(root: &Path, id: &str, apply: bool, radius: usize) -> Result<Value> {
    let map = root.join("maps").join(id);
    let base = map.join("data/terrain-preview");
    let current_bytes = fs::read(base.join("terrain.npz"))?;
    let current = archive::unzip(&current_bytes)?;
    let original_report = read_json(&base.join("report.json"))?;
    let original_parameters = read_json(&base.join("parameters.json"))?;
    let snapshot = base.join(SNAPSHOT);
    // Repeated runs always recompute contacts from the retained pre-edit state.
    let baseline_bytes = if snapshot.join("terrain.npz").is_file() {
        fs::read(snapshot.join("terrain.npz"))?
    } else {
        current_bytes.clone()
    };
    let baseline = archive::unzip(&baseline_bytes)?;
    let baseline_report = if snapshot.join("report.json").is_file() {
        read_json(&snapshot.join("report.json"))?
    } else {
        original_report.clone()
    };
    let (old, w, h) = load(&baseline)?;
    let b = baseline_report["border_cells"]
        .as_u64()
        .ok_or_else(|| err("缺少边框"))? as usize;
    if w != 840 || h != 840 || b != 20 {
        return Err(err("本批仅处理 800×800 可玩格、20 格边框的预览"));
    }
    let water = bodies(&old, w, h, b);
    let (layers, scope) = enclose(&old, &water, w, h, b, radius)?;
    let target = updated_cache(&baseline, &layers, w, h, &baseline_report)?;
    let mut audit = verify(&baseline, &target, &scope, w, h, b)?;
    let changed = layers != old;
    if current != baseline && (!snapshot.is_dir() || current != target) {
        return Err(err(format!("{id} 预览在快照后另有修改，停止覆盖")));
    }
    audit["map_id"] = json!(id);
    audit["date"] = json!("2026-10-29");
    audit["rule"] = json!("SHORE.SINGLE_LAYER");
    audit["scope"] = json!(
        "all fully enclosed waterbodies in this batch; open/frame-clipped components listed separately"
    );
    audit["size_basis"] =
        json!("per-map enclosed components listed below; no global lake-area threshold");
    audit["contact_method"] = json!(
        "pre-edit four-neighbor shared edges, 10 world units per edge; diagonals excluded; full inner and outer perimeter"
    );
    audit["shore_band_radius_cells"] = json!(radius);
    audit["shore_band_method"] = json!(
        "union of Euclidean disks around all land-side shore cells, clipped to playable land"
    );
    audit["before_terrain_crc32"] = json!(crc32fast::hash(&baseline_bytes));
    audit["changed"] = json!(changed);
    if !apply {
        return Ok(audit);
    }
    let mut backups = Vec::new();
    for name in [
        "source.png",
        "preview.png",
        "preview--line.png",
        "preview--center--line.png",
        "data/terrain-preview/reference.npz",
        "data/terrain-preview/coordinate-system.json",
        "data/terrain-preview/centers-report.json",
        "data/terrain-preview/water-layer-reference.npz",
        "data/terrain-preview/water-layer-reference-report.json",
        "data/terrain-preview/water-layer-verification.json",
    ] {
        let path = map.join(name);
        let bytes = if path.exists() {
            Some(fs::read(&path)?)
        } else {
            None
        };
        backups.push((path, bytes));
    }
    let result = (|| -> Result<()> {
        if changed {
            fs::create_dir_all(&snapshot)?;
            for (name, bytes) in [
                ("terrain.npz", baseline_bytes.clone()),
                ("report.json", serde_json::to_vec_pretty(&baseline_report)?),
                (
                    "parameters.json",
                    serde_json::to_vec_pretty(&original_parameters)?,
                ),
            ] {
                let path = snapshot.join(name);
                if !path.exists() {
                    fs::write(path, bytes)?;
                }
            }
            let mut reference_report = baseline_report.clone();
            reference_report["layer_policy"]["land_layer_cell_counts"] =
                json!(counts(&layers, w, h, b));
            reference_report["layer_policy"]["active_land_layers"] = json!(
                (1..=5)
                    .filter(|&l| counts(&layers, w, h, b)[l] > 0)
                    .collect::<Vec<_>>()
            );
            reference_report["layer_policy"]["water_layer_enclosure"] = json!({"rule":"SHORE.SINGLE_LAYER","verification":REPORT,
                "before_terrain":format!("{SNAPSHOT}/terrain.npz"),"before_report":format!("{SNAPSHOT}/report.json"),
                "changed_land_cells":audit["changed_land_cells"],"shore_band_radius_cells":radius});
            project::write_json(
                &base.join("water-layer-reference-report.json"),
                &reference_report,
            )?;
            fs::write(base.join("water-layer-reference.npz"), pack(&target)?)?;
            let mut parameters = original_parameters.clone();
            parameters["water_design_land_reference"] = json!({"terrain":"water-layer-reference.npz","report":"water-layer-reference-report.json"});
            parameters["water_policy"]["single_layer_enclosure"] = json!({"rule":"SHORE.SINGLE_LAYER","verification":REPORT,
                "before_terrain":format!("{SNAPSHOT}/terrain.npz"),"shore_band_radius_cells":radius,
                "owner_selection":"largest pre-edit shared shoreline length","full_perimeter":true});
            project::write_json(&base.join("parameters.json"), &parameters)?;
        }
        let input = root.join(
            baseline_report["source_reference"]
                .as_str()
                .ok_or_else(|| err("缺少输入路径"))?,
        );
        // Use the existing native renderer for every image, including center labels.
        terrain_preview::generate(root, id, &input, None)?;
        let produced_bytes = fs::read(base.join("terrain.npz"))?;
        let produced = archive::unzip(&produced_bytes)?;
        verify(&baseline, &produced, &scope, w, h, b)?;
        if produced != target {
            return Err(err(format!("{id} 原生重生成未复现目标数组")));
        }
        let report = read_json(&base.join("report.json"))?;
        if report["bounds_wgs84"] != baseline_report["bounds_wgs84"] {
            return Err(err("游戏地理取景发生变化"));
        }
        for file in report["final_images"]
            .as_array()
            .ok_or_else(|| err("缺少图片清单"))?
        {
            let name = file.as_str().ok_or_else(|| err("无效图片名"))?;
            let hash = crc32fast::hash(&fs::read(map.join(name))?);
            if report["image_crc32"][name] != hash {
                return Err(err("图片校验和不一致"));
            }
        }
        if report["image_crc32"]["source.png"] != baseline_report["image_crc32"]["source.png"] {
            return Err(err("真实高程参考图发生变化，请检查字体或输入"));
        }
        audit["candidate_terrain_crc32"] = json!(crc32fast::hash(&produced_bytes));
        audit["image_crc32"] = report["image_crc32"].clone();
        audit["original_input_crc32"] = report["reference_crc32"].clone();
        audit["source_image_preserved"] = json!(true);
        audit["actual_arrays_and_images_verified"] = json!(true);
        if changed {
            audit["before_snapshot"] = json!(SNAPSHOT);
        }
        project::write_json(&base.join(REPORT), &audit)?;
        Ok(())
    })();
    if let Err(e) = result {
        // Keep a recoverable input state if native regeneration fails.
        project::write_json(&base.join("parameters.json"), &original_parameters)?;
        fs::write(base.join("terrain.npz"), &current_bytes)?;
        project::write_json(&base.join("report.json"), &original_report)?;
        for (path, bytes) in backups {
            if let Some(bytes) = bytes {
                fs::write(path, bytes)?;
            } else if path.is_file() {
                fs::remove_file(path)?;
            }
        }
        return Err(e);
    }
    Ok(audit)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let apply = args.iter().any(|a| a == "--apply");
    let radius = args
        .windows(2)
        .find(|v| v[0] == "--radius")
        .map(|v| v[1].parse::<usize>())
        .transpose()?
        .unwrap_or(12);
    if !(1..=50).contains(&radius) {
        return Err(err("岸带半径须在 1–50 格"));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let selected = args
        .windows(2)
        .find(|v| v[0] == "--map")
        .map(|v| v[1].clone());
    let mut maps = fs::read_dir(root.join("maps"))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    maps.sort();
    let mut reports = Vec::new();
    for map in maps
        .iter()
        .filter(|m| m.join("data/terrain-preview/terrain.npz").is_file())
    {
        let id = map
            .file_name()
            .unwrap()
            .to_str()
            .ok_or_else(|| err("无效图名"))?;
        if selected.as_deref().is_some_and(|s| s != id) {
            continue;
        }
        let report = process(root, id, apply, radius)?;
        println!(
            "{id}: {} land cells changed; {} water components; regenerated={apply}",
            report["changed_land_cells"],
            report["waterbodies"].as_array().unwrap().len()
        );
        reports.push(report);
    }
    if reports.is_empty() {
        return Err(err("未找到预览"));
    }
    let summary = json!({"schema_version":1,"date":"2026-10-29","applied":apply,"maps":reports});
    project::write_json(
        &root.join("work/preview-water-layers/batch-verification.json"),
        &summary,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_edges_before_edit_and_encloses_the_entire_shore() {
        let (w, h) = (15, 15);
        let mut old = vec![2; w * h];
        for y in 0..h {
            for x in 0..5 {
                old[y * w + x] = 1;
            }
        }
        for y in 5..10 {
            for x in 4..9 {
                old[y * w + x] = 0;
            }
        }
        let body = bodies(&old, w, h, 1);
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].contacts[1], 7);
        assert_eq!(body[0].contacts[2], 13);
        assert_eq!(owner(&body[0]).unwrap(), 2);
        let (now, scope) = enclose(&old, &body, w, h, 1, 2).unwrap();
        let after = bodies(&now, w, h, 1);
        assert_eq!(after[0].contacts[2], 20);
        assert_eq!(after[0].contacts[1], 0);
        assert!(
            old.iter()
                .zip(&now)
                .enumerate()
                .all(|(i, (a, b))| a == b || scope[i])
        );
        assert!(old.iter().zip(&now).all(|(&a, &b)| (a == 0) == (b == 0)));
    }
    #[test]
    fn open_water_is_reported_and_left_unchanged() {
        let mut old = vec![1; 49];
        for y in 1..4 {
            old[y * 7 + 3] = 0;
            old[y * 7 + 4] = 2;
        }
        let b = bodies(&old, 7, 7, 1);
        assert!(b[0].open);
        assert_eq!(enclose(&old, &b, 7, 7, 1, 2).unwrap().0, old);
    }
    #[test]
    fn diagonal_land_does_not_vote_and_ties_require_a_local_decision() {
        let mut old = vec![1; 25];
        old[12] = 0;
        old[11] = 2;
        old[13] = 2;
        old[6] = 5;
        let b = bodies(&old, 5, 5, 1);
        assert_eq!(b[0].contacts[5], 0);
        assert_eq!(b[0].contacts[1], 2);
        assert_eq!(b[0].contacts[2], 2);
        assert!(owner(&b[0]).is_err());
    }
}
