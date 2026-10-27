//! Local ramp edits on a finished map. Unrelated chunks and resource IDs stay intact.
use crate::{
    Result, err,
    map::{self, BlendTiles, HeightMap, MapFile, MapObject},
    project::{self, Project},
    render::{self, Image},
    surface::Surface,
    terrain::{self, Layout, ObjectSpec, Terrain, number},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
};

mod relocate;
pub use relocate::relocate;

pub struct Polished {
    pub terrain: Terrain,
    pub report: Value,
    before: Vec<f64>,
    before_tiles: Vec<u16>,
    tiles: Vec<u16>,
    imp: Vec<bool>,
    build: Vec<bool>,
    textures: Vec<String>,
    example: usize,
}
fn smooth(v: f64) -> f64 {
    let v = v.clamp(0., 1.);
    v * v * (3. - 2. * v)
}
fn sample(z: &[f64], n: usize, x: f64, y: f64) -> f64 {
    let x = x.clamp(0., (n - 1) as f64);
    let y = y.clamp(0., (n - 1) as f64);
    let ix = x.floor() as usize;
    let iy = y.floor() as usize;
    let jx = (ix + 1).min(n - 1);
    let jy = (iy + 1).min(n - 1);
    let ax = x - ix as f64;
    let ay = y - iy as f64;
    (z[iy * n + ix] * (1. - ax) + z[iy * n + jx] * ax) * (1. - ay)
        + (z[jy * n + ix] * (1. - ax) + z[jy * n + jx] * ax) * ay
}
fn delta(a: f64, b: f64) -> f64 {
    (a - b + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
fn frame(r: &Value, x: f64, y: f64) -> (f64, f64) {
    let ux = number(r, "ux", 1.);
    let uy = number(r, "uy", 0.);
    let norm = ux.hypot(uy).max(1e-12);
    let dx = x - number(r, "x", 0.);
    let dy = y - number(r, "y", 0.);
    ((dx * ux + dy * uy) / norm, (-dx * uy + dy * ux) / norm)
}
fn half(r: &Value) -> f64 {
    (number(r, "span", 25.) * 0.5).clamp(10., 20.)
}
fn mouth(r: &Value, u: f64) -> f64 {
    terrain::ramp_width(r).expect("validated ramp width")
        * 0.5
        * (1. + 0.25 * (u.abs() / half(r)).clamp(0., 1.).powi(2))
}
fn contains(r: &Value, x: f64, y: f64, pad: f64) -> bool {
    let (u, v) = frame(r, x, y);
    u.abs() <= half(r) + 5. + pad && v.abs() <= mouth(r, u) + 5. + pad
}
fn old_foot(r: &Value, x: f64, y: f64, pad: f64) -> bool {
    let (u, v) = frame(r, x, y);
    let (ux, uy, _, _) = terrain::ramp_axes(r).unwrap();
    let bw = number(r, "w", 25.) * 0.5;
    let bh = number(r, "h", 25.) * 0.5;
    let hv = (-bw * uy + bh * ux)
        .abs()
        .max((-bw * uy - bh * ux).abs())
        .max(4.);
    ((x - number(r, "x", 0.)).abs() <= bw + pad && (y - number(r, "y", 0.)).abs() <= bh + pad)
        || (u.abs() <= half(r) + pad && v.abs() <= hv + pad)
}
fn normal(base: &[f64], n: usize, r: &Value) -> f64 {
    let (ux, uy, lo, hi) = terrain::ramp_axes(r).unwrap();
    let mut gx = 0.;
    let mut gy = 0.;
    for dy in -32..=32 {
        for dx in -32..=32 {
            let d = (dx * dx + dy * dy) as f64;
            if !(225. ..=1024.).contains(&d) {
                continue;
            }
            let z = sample(
                base,
                n,
                number(r, "x", 0.) + dx as f64,
                number(r, "y", 0.) + dy as f64,
            );
            if (z - lo).abs().min((z - hi).abs()) > 12. {
                continue;
            }
            let sign = if (z - lo).abs() < (z - hi).abs() {
                -1.
            } else {
                1.
            };
            let w = (-d / 1152.).exp();
            gx += dx as f64 * sign * w;
            gy += dy as f64 * sign * w;
        }
    }
    let old = uy.atan2(ux);
    if gx.hypot(gy) < 1. {
        old
    } else {
        old + delta(gy.atan2(gx), old).clamp(-65_f64.to_radians(), 65_f64.to_radians())
    }
}
fn candidates(base: &[f64], n: usize, old: &Value) -> Vec<Value> {
    let (_, _, lo, hi) = terrain::ramp_axes(old).unwrap();
    let target = normal(base, n, old);
    let original = number(old, "uy", 0.).atan2(number(old, "ux", 1.));
    let mut list = Vec::new();
    for angle in [
        target - 10_f64.to_radians(),
        target,
        target + 10_f64.to_radians(),
        original,
    ] {
        for du in [-3., 0., 3.] {
            for dv in [-3., 0., 3.] {
                let ux = angle.cos();
                let uy = angle.sin();
                let x = number(old, "x", 0.) + du * ux - dv * uy;
                let y = number(old, "y", 0.) + du * uy + dv * ux;
                let mut score =
                    delta(angle, target).to_degrees().powi(2) * 0.15 + du * du + dv * dv;
                for side in [-1., 1.] {
                    for v in [-6., 0., 6.] {
                        let z = sample(
                            base,
                            n,
                            x + side * (half(old) + 4.) * ux - v * uy,
                            y + side * (half(old) + 4.) * uy + v * ux,
                        );
                        score += (z - if side < 0. { lo } else { hi }).powi(2) * 10.;
                    }
                }
                let mut r = old.clone();
                r["x"] = json!(x);
                r["y"] = json!(y);
                r["ux"] = json!(ux);
                r["uy"] = json!(uy);
                r["style"] = json!("natural");
                list.push((score, r));
            }
        }
    }
    list.sort_by(|a, b| a.0.total_cmp(&b.0));
    list.into_iter().map(|(_, r)| r).collect()
}
fn protected(p: &Project, objects: &[MapObject], n: usize) -> Vec<bool> {
    let mut mask = vec![false; n * n];
    let specs = crate::scripts::load_rules(p)
        .map(|r| r.spawn_waypoints)
        .unwrap_or_default();
    for o in objects {
        // Vegetation and rocks follow the terrain; keep their positions and
        // avoid their collision footprint in the lane, without freezing the
        // entire shoulder under them as if they were flat building foundations.
        if o.type_name.contains("Tree")
            || o.type_name.contains("Bush")
            || o.type_name.contains("Rock")
        {
            continue;
        }
        let name = map::prop(&o.props, "waypointName")
            .and_then(map::Value::text)
            .unwrap_or("");
        let radius = if o.type_name == "OreNode" {
            p.number("ORE_CLEAR_RADIUS", 20.).max(20.)
        } else if name.starts_with("Player_") {
            p.terrain_number("base_min_flat_radius", 24.)
        } else if let Some(s) = specs.iter().find(|s| s.name == name) {
            s.padding as f64 + 2.
        } else if o.type_name.contains("Waypoint") {
            3.
        } else {
            8.
        };
        let cx = o.position[0] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.);
        let cy =
            n as f64 - 1. - p.number("BORDER", 20.) - o.position[1] as f64 / p.number("CELL", 10.);
        for y in ((cy - radius).floor() as isize).max(0)
            ..=((cy + radius).ceil() as isize).min(n as isize - 1)
        {
            for x in ((cx - radius).floor() as isize).max(0)
                ..=((cx + radius).ceil() as isize).min(n as isize - 1)
            {
                if (x as f64 - cx).hypot(y as f64 - cy) <= radius {
                    mask[y as usize * n + x as usize] = true;
                }
            }
        }
    }
    mask
}
fn sculpt(
    z: &[f64],
    ground: (&[f64], &[f64]),
    n: usize,
    old: &Value,
    new: &Value,
    locks: &[bool],
    scale: f64,
) -> Option<Vec<(usize, f64)>> {
    let (base, platforms) = ground;
    let (_, _, lo, hi) = terrain::ramp_axes(new).ok()?;
    let cx = number(new, "x", 0.);
    let cy = number(new, "y", 0.);
    let mut patch = Vec::new();
    let radius = (half(new) + mouth(new, half(new)) + 6.)
        .max(number(old, "w", 25.).max(number(old, "h", 25.)))
        .max(44.)
        .ceil() as isize;
    for y in (cy as isize - radius).max(0)..=(cy as isize + radius).min(n as isize - 1) {
        for x in (cx as isize - radius).max(0)..=(cx as isize + radius).min(n as isize - 1) {
            let i = y as usize * n + x as usize;
            let (u, v) = frame(new, x as f64, y as f64);
            let old_inside = old_foot(old, x as f64, y as f64, 0.);
            let w = (1. - smooth((v.abs() - mouth(new, u)) / 5.))
                * (1. - smooth((u.abs() - half(new)) / 5.));
            if !old_inside && w <= 0. {
                continue;
            }
            if w > 0. && !terrain::ramp_layer_allowed(platforms[i], lo, hi) {
                return None;
            }
            // Do not cut a third level or reshape neighboring terrain features.
            let bg = if old_inside && (base[i] - lo).abs().min((base[i] - hi).abs()) < 3. {
                base[i]
            } else {
                z[i]
            };
            let h = lo + (hi - lo) * smooth((u + half(new)) / (2. * half(new)));
            let value = ((bg * (1. - w) + h * w) / scale).round_ties_even() * scale;
            if (value - z[i]).abs() < scale * 0.5 {
                continue;
            }
            if locks[i] || z[i] < lo - 3. || z[i] > hi + 3. {
                return None;
            }
            patch.push((i, value));
        }
    }
    Some(patch)
}
/// Full-width parallel lanes, complete mouths, monotonicity, longitudinal and
/// transverse steps. Metrics are measured after uint16 height quantization.
fn validate_shape(z: &[f64], n: usize, r: &Value, border: usize) -> Option<Value> {
    let (ux, uy, lo, hi) = terrain::ramp_axes(r).ok()?;
    let cx = number(r, "x", 0.);
    let cy = number(r, "y", 0.);
    let half = half(r);
    let width = terrain::ramp_width(r).ok()?;
    let half_width = width * 0.5;
    let mut along = 0_f64;
    let mut gap = 0_f64;
    let mut cross = 0_f64;
    // Check every lane across the full width, including both outer edges.
    let lanes = width.ceil() as usize;
    for lane in 0..=lanes {
        let v = -half_width + width * lane as f64 / lanes as f64;
        let mut prev = None;
        for j in 0..=((2. * half + 8.).ceil() as usize) {
            let u = -half - 4. + j as f64;
            let x = cx + ux * u - uy * v;
            let y = cy + uy * u + ux * v;
            if x < border as f64 + 3.
                || y < border as f64 + 3.
                || x > n as f64 - 4. - border as f64
                || y > n as f64 - 4. - border as f64
            {
                return None;
            }
            let h = sample(z, n, x, y);
            if let Some(p) = prev {
                let d: f64 = h - p;
                if d < -1. {
                    return None;
                }
                along = along.max(d.abs());
            }
            prev = Some(h);
        }
        for (side, expected) in [(-1., lo), (1., hi)] {
            let h = sample(
                z,
                n,
                cx + ux * side * (half + 4.) - uy * v,
                cy + uy * side * (half + 4.) + ux * v,
            );
            gap = gap.max((h - expected).abs());
        }
    }
    for u in [-half, -half * 0.5, 0., half * 0.5, half] {
        let mut prev = None;
        for lane in 0..=lanes {
            let v = -half_width + width * lane as f64 / lanes as f64;
            let h = sample(
                z,
                n,
                cx + ux * u - uy * v as f64,
                cy + uy * u + ux * v as f64,
            );
            if let Some(p) = prev {
                let d: f64 = h - p;
                cross = cross.max(d.abs());
            }
            prev = Some(h);
        }
    }
    if along > 6.05 || gap > 12. || cross > 10. {
        return None;
    }
    Some(
        json!({"width_cells":width,"parallel_lanes":lanes+1,"max_along_step":along,"max_endpoint_error":gap,"max_cross_step":cross}),
    )
}
fn labels(z: &[f64], imp: &[bool], n: usize, water: f64, sea: bool) -> Vec<usize> {
    let mut labels = vec![0; n * n];
    let mut next = 0;
    let mut q = VecDeque::new();
    for i in 0..n * n {
        if labels[i] != 0 || imp[i] || (z[i] < water) != sea {
            continue;
        }
        next += 1;
        labels[i] = next;
        q.push_back(i);
        while let Some(j) = q.pop_front() {
            for k in crate::build::neighbors(j, n) {
                if labels[k] == 0 && !imp[k] && (z[k] < water) == sea {
                    labels[k] = next;
                    q.push_back(k);
                }
            }
        }
    }
    labels
}
fn preserve_connections(before: &[usize], after: &[usize], keys: &[usize]) -> bool {
    for (i, &a) in keys.iter().enumerate() {
        for &b in &keys[i..] {
            if before[a] != 0 && before[a] == before[b] && (after[a] == 0 || after[a] != after[b]) {
                return false;
            }
        }
    }
    true
}
fn masks(
    z: &[f64],
    old: &[bool],
    n: usize,
    r: &Value,
    touched: &[(usize, f64)],
    p: &Project,
) -> Vec<bool> {
    let mut imp = old.to_vec();
    let slope = terrain::gradient(z, n);
    let water = p.number("WATER_Z", 200.);
    let border = p.usize("BORDER", 20);
    for &(i, _) in touched {
        let x = i % n;
        let y = i / n;
        for yy in y.saturating_sub(1)..=(y + 1).min(n - 1) {
            for xx in x.saturating_sub(1)..=(x + 1).min(n - 1) {
                let j = yy * n + xx;
                let coast = z[j] >= water && crate::build::neighbors(j, n).any(|k| z[k] < water);
                imp[j] = slope[j] > p.number("SLOPE_IMPASSABLE", 24.)
                    || z[j] > water + p.number("PEAK_IMPASSABLE", 235.)
                    || terrain::edge(j, n, border) < 3.
                    || (coast && !contains(r, xx as f64, yy as f64, 0.));
            }
        }
    }
    imp
}
fn cell(p: &Project, o: &MapObject, n: usize) -> [f64; 2] {
    let mut frame = crate::coordinates::Frame::from_project(p);
    frame.width = n;
    frame.height = n;
    frame.grid([o.position[0] as f64, o.position[1] as f64])
}

fn blueprint(p: &Project, n: usize, path: &std::path::Path) -> Result<(Vec<f64>, Vec<f64>)> {
    let archive = crate::archive::unzip(&fs::read(path)?)?;
    let s = crate::archive::Npy::parse(&archive["z_scale.npy"])?.numbers()?[0];
    let base_values = crate::archive::Npy::parse(&archive["base_z.npy"])?.numbers()?;
    if base_values.len() != n * n {
        return Err(err("ramp blueprint dimensions differ"));
    }
    let mut levels = vec![p.terrain_number("sea_flat", 130.)];
    let first_land = if p.string("HEIGHT_PRESET", "") == "standard" || p.terrain_flag("v11_baked") {
        p.cfg["TERRAIN"]["levels"][0].as_f64().unwrap_or(210.)
    } else {
        p.number("LAND_Z", 210.) + 2.
    };
    levels.push(first_land);
    levels.extend(
        p.cfg["TERRAIN"]["levels"]
            .as_array()
            .ok_or_else(|| err("missing terrain levels"))?
            .iter()
            .filter_map(Value::as_f64)
            .filter(|h| *h > first_land),
    );
    let base: Vec<_> = base_values
        .iter()
        .map(|v| {
            let h = v * s;
            *levels
                .iter()
                .min_by(|a, b| (**a - h).abs().total_cmp(&(**b - h).abs()))
                .unwrap()
        })
        .collect();
    // A baked blueprint already contains slopes. Only flat cells at a known
    // level count as platforms; interpolated slope heights are not third levels.
    let source_heights: Vec<_> = base_values.iter().map(|v| v * s).collect();
    let source_gradient = terrain::gradient(&source_heights, n);
    let platforms: Vec<_> = source_heights
        .iter()
        .enumerate()
        .map(|(i, h)| {
            if !p.terrain_flag("v11_baked")
                || ((base[i] - h).abs() <= 3. && source_gradient[i] <= 3.)
            {
                base[i]
            } else {
                f64::NAN
            }
        })
        .collect();
    Ok((base, platforms))
}

pub fn apply(p: &Project, map: &mut MapFile, source_name: &str) -> Result<Polished> {
    let source = map.clone();
    let mut hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err("ramp polishing requires matching square v27 terrain"));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|v| *v as f64 * scale)
        .collect();
    // A prior polish release carries its actual ramp definitions with it.
    let source_layout = p
        .root
        .join("assets")
        .join(&p.group)
        .join(source_name)
        .join("source/layout.json");
    let mut layout = if source_layout.is_file() {
        serde_json::from_slice::<Layout>(&fs::read(source_layout)?)?
    } else {
        Layout::load(p)?
    };
    let (base, platforms) = blueprint(p, n, &p.path("v11_layers.npz")?)?;
    let locks = protected(p, &objects, n);
    let water = p.number("WATER_Z", 200.);
    let mut z = before.clone();
    let old_imp = terrain::flip(&bt.impassability, n);
    let mut imp = old_imp.clone();
    let keys: Vec<_> = objects
        .iter()
        .filter(|o| {
            o.type_name == "OreNode"
                || map::prop(&o.props, "waypointName")
                    .and_then(map::Value::text)
                    .is_some_and(|s| s.starts_with("Player_"))
        })
        .map(|o| cell(p, o, n))
        .filter(|c| c[0] >= 0. && c[1] >= 0. && c[0] < n as f64 && c[1] < n as f64)
        .map(|c| {
            c[1].round().clamp(0., (n - 1) as f64) as usize * n
                + c[0].round().clamp(0., (n - 1) as f64) as usize
        })
        .collect();
    let original_land = labels(&z, &imp, n, water, false);
    let original_sea = labels(&z, &imp, n, water, true);
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    let mut changed = vec![false; n * n];
    let mut rows = Vec::new();
    let mut example = 0;
    let mut largest_turn = 0_f64;
    for index in 0..layout.ramps.len() {
        let old = layout.ramps[index].clone();
        terrain::ramp_axes(&old)?;
        terrain::ramp_width(&old)?;
        let mut accepted = None;
        let mut rejected = [0_usize; 3];
        for candidate in candidates(&base, n, &old) {
            if objects.iter().any(|o| {
                if !o.type_name.contains("Tree")
                    && !o.type_name.contains("Bush")
                    && !o.type_name.contains("Rock")
                {
                    return false;
                }
                let c = cell(p, o, n);
                let (u, v) = frame(&candidate, c[0], c[1]);
                u.abs() < half(&candidate) + 3. && v.abs() < mouth(&candidate, u) + 2.5
            }) {
                rejected[0] += 1;
                continue;
            }
            let Some(patch) = sculpt(&z, (&base, &platforms), n, &old, &candidate, &locks, scale)
            else {
                rejected[0] += 1;
                continue;
            };
            if patch.is_empty() {
                continue;
            }
            let mut trial = z.clone();
            for &(i, h) in &patch {
                trial[i] = h;
            }
            let Some(metrics) = validate_shape(&trial, n, &candidate, p.usize("BORDER", 20)) else {
                rejected[1] += 1;
                continue;
            };
            if layout.ramps[..index].iter().any(|r| {
                r["style"] == "natural"
                    && validate_shape(&trial, n, r, p.usize("BORDER", 20)).is_none()
            }) {
                rejected[1] += 1;
                continue;
            }
            let trial_imp = masks(&trial, &imp, n, &candidate, &patch, p);
            if !preserve_connections(
                &original_land,
                &labels(&trial, &trial_imp, n, water, false),
                &keys,
            ) || !preserve_connections(
                &original_sea,
                &labels(&trial, &trial_imp, n, water, true),
                &keys,
            ) {
                rejected[2] += 1;
                continue;
            }
            accepted = Some((candidate, patch, trial, trial_imp, metrics));
            break;
        }
        if let Some((candidate, patch, trial, trial_imp, metrics)) = accepted {
            let turn = delta(
                number(&candidate, "uy", 0.).atan2(number(&candidate, "ux", 1.)),
                number(&old, "uy", 0.).atan2(number(&old, "ux", 1.)),
            )
            .to_degrees();
            if turn.abs() > largest_turn {
                largest_turn = turn.abs();
                example = index;
            }
            paint(
                &mut tiles,
                &bt.textures,
                &trial,
                n,
                (&old, &candidate),
                water,
                &mut changed,
            );
            for &(i, _) in &patch {
                changed[i] = true;
            }
            rows.push(json!({"ramp_index":index+1,"status":"optimized","rotation_degrees":turn,"changed_height_cells":patch.len(),"before":old,"after":candidate,"validation":metrics}));
            layout.ramps[index] = candidate;
            z = trial;
            imp = trial_imp;
        } else {
            // Even where geometry cannot move safely, remove the old 8x8
            // material apron using the unchanged, proven opening.
            paint(
                &mut tiles,
                &bt.textures,
                &z,
                n,
                (&old, &old),
                water,
                &mut changed,
            );
            rows.push(json!({"ramp_index":index+1,"status":"texture-only","rejected_candidates":{"footprint_or_third_level":rejected[0],"lane_shape_or_boundary":rejected[1],"connectivity":rejected[2]},"reason":"Geometry retained to preserve protected footprints, full-width lane profiles or existing land/sea resource connectivity; materials polished in place.","before":old,"after":old}));
        }
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even().clamp(0., 65535.) as u16)
        .collect();
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    let mut build = terrain::flip(&bt.buildability, n);
    let slope = terrain::gradient(&z, n);
    for i in 0..n * n {
        if changed[i] && !locks[i] {
            build[i] = build[i] || slope[i] > 1.5;
        }
    }
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    // Check preservation on the actual map that will be packaged.
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if name != "HeightMapData" && name != "BlendTileData" && map.get(name)? != chunk {
            return Err(err(format!("ramp edit changed unrelated chunk {name}")));
        }
    }
    let checked_hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let checked_bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    crate::script_spawns::validate(p, map, &checked_hm, &checked_bt, &objects)?;
    for i in 0..n * n {
        if locks[i] && raw[i] != (before[i] / scale).round_ties_even() as u16 {
            return Err(err("ramp edit changed a protected footprint"));
        }
    }
    let script_waypoints = crate::scripts::load_rules(p)?.spawn_waypoints;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !map::prop(&o.props, "waypointName")
                .and_then(map::Value::text)
                .is_some_and(|name| script_waypoints.iter().any(|s| s.name == name))
        })
        .map(|o| ObjectSpec {
            cell: cell(p, o, n),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    let optimized = rows.iter().filter(|r| r["status"] == "optimized").count();
    for row in &mut rows {
        row["preview_id"] =
            json!(layout.objects.len() + row["ramp_index"].as_u64().unwrap() as usize);
    }
    let report = json!({"source":source_name,"ramps":rows,"optimized":optimized,"retained":layout.ramps.len()-optimized,"map_objects":objects.len(),"indexed_layout_objects":layout.objects.len(),"objects_unchanged":true,"scripts_unchanged":true,"resource_assets_unchanged":true,"protected_footprints_unchanged":true,"existing_land_and_sea_resource_connections_preserved":true,"modified_chunks":["HeightMapData","BlendTileData"],"ingame_test":"not run"});
    println!(
        "斜坡地形优化 {optimized}/{}；其余仅整理纹理；对象、脚本和资源清单保持原样",
        layout.ramps.len()
    );
    Ok(Polished {
        terrain: Terrain {
            n,
            raw,
            land: z.iter().map(|h| *h >= water).collect(),
            z,
            slope,
            layout,
        },
        report,
        before,
        before_tiles,
        tiles,
        imp,
        build,
        textures: bt.textures,
        example,
    })
}

fn paint(
    tiles: &mut [u16],
    textures: &[String],
    z: &[f64],
    n: usize,
    ramps: (&Value, &Value),
    water: f64,
    changed: &mut [bool],
) {
    let (old, new) = ramps;
    let idx = |s: &str| textures.iter().position(|t| t == s).map(|i| i as u16);
    let Some(dirt) = idx("Dirt_Yucatan06") else {
        return;
    };
    let gravel = idx("Gravel_Yucatan01").unwrap_or(dirt);
    let rock = idx("Rock_Yucatan01");
    let cx = number(new, "x", 0.);
    let cy = number(new, "y", 0.);
    for y in (cy as isize - 44).max(0)..=(cy as isize + 44).min(n as isize - 1) {
        for x in (cx as isize - 44).max(0)..=(cx as isize + 44).min(n as isize - 1) {
            let i = y as usize * n + x as usize;
            let (u, v) = frame(new, x as f64, y as f64);
            let mut family = tiles[i] / 64;
            if old_foot(old, x as f64, y as f64, 12.) && (family == dirt || family == gravel) {
                let names = if z[i] < water {
                    ["RA3_ShallowSeaFloor", "Gravel_Yucatan01"]
                } else if z[i] < 245. {
                    ["Grass_NewYork01", "Grass_MtRush04"]
                } else if z[i] < 315. {
                    ["Grass_MtRush04", "Grass_NewYork01"]
                } else if z[i] < 385. {
                    ["Dirt_Iceland05", "Grass_MtRush04"]
                } else {
                    ["Pavement_Amsterdam05", "Dirt_Iceland05"]
                };
                family = names.iter().find_map(|s| idx(s)).unwrap_or(family);
            }
            let width = mouth(new, u) * 0.75;
            let noise = ((x * 37 + y * 17).rem_euclid(7) as f64 - 3.) * 0.10;
            if u.abs() < half(new) + 2. && v.abs() < width + noise {
                family = dirt;
            } else if u.abs() < half(new) + 4. && v.abs() < width + 2. + noise {
                family = gravel;
            } else if contains(new, x as f64, y as f64, 1.)
                && let Some(rock) = rock
            {
                let slope_x = (sample(z, n, x as f64 + 1., y as f64)
                    - sample(z, n, x as f64 - 1., y as f64))
                .abs()
                    * 0.5;
                let slope_y = (sample(z, n, x as f64, y as f64 + 1.)
                    - sample(z, n, x as f64, y as f64 - 1.))
                .abs()
                    * 0.5;
                if slope_x.hypot(slope_y) > 24. {
                    family = rock;
                }
            }
            if family != tiles[i] / 64 {
                tiles[i] = family * 64 + tiles[i] % 64;
                changed[i] = true;
            }
        }
    }
}

pub(crate) fn patch_blend(
    map: &mut MapFile,
    bt: &BlendTiles,
    tiles: &[u16],
    imp: &[bool],
    build: &[bool],
    changed: &[bool],
    n: usize,
) -> Result<()> {
    let data = &mut map.get_mut("BlendTileData")?.payload;
    let area = n * n;
    let mask_len = n * n.div_ceil(8);
    let tail_start = 4 + 8 * area + 8 * mask_len + area;
    let mut tail = crate::binary::Reader::new(&data[tail_start..]);
    tail.take(12)?;
    let count = tail.u32()? as usize;
    for _ in 0..count {
        tail.take(16)?;
        tail.s16()?;
    }
    tail.take(8)?;
    let descriptions_start = tail_start + tail.pos;
    let mut descriptions = bt.blend_descriptions.clone();
    let mut cache = BTreeMap::new();
    for y in 0..n {
        for x in 0..n {
            let i = y * n + x;
            let game = (n - 1 - y) * n + x;
            if changed[i] {
                data[4 + game * 2..6 + game * 2].copy_from_slice(&tiles[i].to_le_bytes());
                for array in 0..3 {
                    let o = 4 + 2 * area + 2 * area * array + 2 * game;
                    data[o..o + 2].copy_from_slice(&0_u16.to_le_bytes());
                }
            }
            // Use standard two-texture blend descriptions at the actual per-cell
            // material edge. Existing atlas definitions and IDs are never rebuilt.
            if !changed[i] && !crate::build::neighbors(i, n).any(|k| changed[k]) {
                continue;
            }
            let neighbor = [
                (1_isize, 0_isize, 0_usize, 0_u8),
                (-1, 0, 0, 1),
                (0, -1, 1, 0),
                (0, 1, 1, 1),
            ]
            .into_iter()
            .find_map(|(dx, dy, dir, flags)| {
                let xx = x as isize + dx;
                let yy = y as isize + dy;
                if xx < 0 || yy < 0 || xx >= n as isize || yy >= n as isize {
                    return None;
                }
                let j = yy as usize * n + xx as usize;
                (tiles[j] / 64 != tiles[i] / 64).then_some((
                    tiles[j] / 64 * 64 + tiles[i] % 64,
                    dir,
                    flags,
                ))
            });
            if let Some((secondary, dir, flags)) = neighbor {
                let key = (secondary, dir, flags);
                let blend = if let Some(id) = cache.get(&key) {
                    *id
                } else {
                    let id = u16::try_from(descriptions.len() / 18 + 1)
                        .map_err(|_| err("too many texture blends"))?;
                    descriptions.extend_from_slice(&(secondary as u32).to_le_bytes());
                    let mut direction = [0; 4];
                    direction[dir] = 1;
                    descriptions.extend_from_slice(&direction);
                    descriptions.extend_from_slice(&[flags, 0]);
                    descriptions.extend_from_slice(&u32::MAX.to_le_bytes());
                    descriptions.extend_from_slice(&0x7ada0000_u32.to_le_bytes());
                    cache.insert(key, id);
                    id
                };
                let o = 4 + 2 * area + 2 * game;
                data[o..o + 2].copy_from_slice(&blend.to_le_bytes());
            }
        }
    }
    for (index, mask) in [(0, imp), (5, build)] {
        let offset = 4 + 8 * area + index * mask_len;
        for y in 0..n {
            for x in 0..n {
                let bit = (n - 1 - y) * n.div_ceil(8) + x / 8;
                let b = 1 << (x % 8);
                if mask[y * n + x] {
                    data[offset + bit] |= b;
                } else {
                    data[offset + bit] &= !b;
                }
            }
        }
    }
    data[tail_start + 4..tail_start + 8]
        .copy_from_slice(&u32::try_from(descriptions.len() / 18 + 1)?.to_le_bytes());
    data.splice(
        descriptions_start..descriptions_start + bt.blend_descriptions.len(),
        descriptions,
    );
    Ok(())
}

impl Polished {
    pub fn art_bytes(&self, p: &Project) -> Result<Vec<u8>> {
        let n = self.terrain.n;
        let surface = Surface {
            chosen: self.tiles.iter().map(|v| v / 64).collect(),
            impassable: self.imp.clone(),
            build_block: self.build.clone(),
            layer_edge: vec![false; n * n],
            shore: vec![false; n * n],
            textures: vec![],
        };
        render::art_image(&self.terrain, p, &surface).tga_bytes()
    }
    pub fn write_artifacts(&self, p: &Project) -> Result<()> {
        self.terrain.save(p)?;
        project::write_json(&p.assets.join("ramp-report.json"), &self.report)?;
        let t = &self.terrain;
        let mut edge = vec![false; t.n * t.n];
        let mut shore = edge.clone();
        let water = p.number("WATER_Z", 200.);
        for i in 0..t.n * t.n {
            edge[i] = t.slope[i] > 20.;
            shore[i] = t.z[i] >= water && crate::build::neighbors(i, t.n).any(|k| t.z[k] < water);
        }
        let surface = Surface {
            chosen: self.tiles.iter().map(|v| v / 64).collect(),
            impassable: self.imp.clone(),
            build_block: self.build.clone(),
            layer_edge: edge,
            shore,
            textures: vec![],
        };
        render::previews(t, p, &surface)?;
        if self.report["operation"] == "relocate-ramps" {
            render::art(t, p, &surface)?;
            let source = p
                .root
                .join("assets")
                .join(&p.group)
                .join(self.report["source"].as_str().unwrap());
            for name in [
                "production-spawns.json",
                "production-facing.json",
                "gift-clearance.json",
                "periodic-attack.json",
                "texture-report.json",
            ] {
                if source.join(name).is_file() {
                    fs::copy(source.join(name), p.assets.join(name))?;
                }
            }
        }
        let path = p.assets.join("layout-index.json");
        let mut index: Value = serde_json::from_slice(&fs::read(&path)?)?;
        index["layout"] = json!("source/layout.json");
        index["source_map"] = json!(format!("{}.map", p.name));
        index["provenance"] = if self.report["operation"] == "relocate-ramps" {
            json!(
                "Ramp centers authored at specified source preview positions. Added ramps and object moves are recorded in ramp-report.json; script contents were preserved."
            )
        } else {
            json!(
                "Objects decoded from the source release; polished ramp definitions are in source/layout.json. Original objects and scripts were preserved."
            )
        };
        project::write_json(&path, &index)?;
        self.comparison(p)?;
        if self.report["operation"] == "relocate-ramps" {
            fs::write(
                p.assets.join("README.md"),
                format!(
                    "# {}\n\n基于 {} 按编号增设或移动斜坡，中心固定为来源标记坐标；移动操作恢复旧坡口，阻挡坡道的装饰及经授权的建筑、矿位挪到附近同层平地。\n\n[前后对比](ramps--comparison.png) · [全要素预览](preview--layout.png) · [编辑与验收](ramp-report.json)\n\n地形、纹理及通行位图只作局部更新，其他斜坡、资源身份、脚本与赠送朝向保留。小地图同步更新，实机待复测。\n",
                    p.name,
                    self.report["source"].as_str().unwrap()
                ),
            )?;
            return Ok(());
        }
        fs::write(
            p.assets.join("README.md"),
            format!(
                "# {}\n\n基于 {} 作局部斜坡优化。\n\n[斜坡前后对比](ramps--comparison.png) · [全要素编号](preview--layout.png) · [编号索引](layout-index.json) · [逐坡验收](ramp-report.json)\n\n调整地形 {} 条，其余 {} 条保留原地形并整理纹理。对象、脚本、资源资产与坡外高度保持原样；保留原有陆地/水域出生点与矿位连通关系。尚未进行游戏内实测。\n\n后续增量优化请保留本目录的 `source/layout.json`，以本版 ZIP 继续执行 `polish-ramps`。\n",
                p.name,
                self.report["source"].as_str().unwrap_or(""),
                self.report["optimized"],
                self.report["retained"]
            ),
        )?;
        Ok(())
    }
    fn comparison(&self, p: &Project) -> Result<()> {
        self.comparison_item(p, self.example, "ramps--comparison.png")?;
        if self.report["operation"] == "relocate-ramps" {
            for i in 0..self.report["ramps"].as_array().unwrap().len() {
                self.comparison_item(
                    p,
                    i,
                    &format!(
                        "ramps-{}--comparison.png",
                        self.report["ramps"][i]["preview_id"]
                    ),
                )?;
            }
        }
        Ok(())
    }
    fn comparison_item(&self, p: &Project, example: usize, filename: &str) -> Result<()> {
        let old = &self.report["ramps"][example]["before"];
        let new = &self.report["ramps"][example]["after"];
        let font = render::font();
        let mut board = Image::new(1040, 590, [22, 29, 38]);
        board.text(
            font.as_ref(),
            20,
            8,
            &format!(
                "{} · 斜坡编号 {}",
                p.name, self.report["ramps"][example]["preview_id"]
            ),
            24.,
            [255, 226, 120],
        );
        board.text(
            font.as_ref(),
            20,
            48,
            "左：优化前    右：优化后（高度与纹理类别示意，非游戏截图）",
            18.,
            [215, 220, 230],
        );
        let relocation = self.report["operation"] == "relocate-ramps";
        let cx = if relocation {
            (number(old, "x", 0.) + number(new, "x", 0.)) * 0.5
        } else {
            number(old, "x", 0.)
        };
        let cy = if relocation {
            (number(old, "y", 0.) + number(new, "y", 0.)) * 0.5
        } else {
            number(old, "y", 0.)
        };
        let extent = if relocation {
            88. + (number(old, "x", 0.) - number(new, "x", 0.))
                .abs()
                .max((number(old, "y", 0.) - number(new, "y", 0.)).abs())
        } else {
            78.
        };
        let n = self.terrain.n;
        for (panel, z, tiles) in [
            (0, &self.before, &self.before_tiles),
            (1, &self.terrain.z, &self.tiles),
        ] {
            for py in 0..500 {
                for px in 0..500 {
                    let x = cx + (px as f64 / 499. - 0.5) * extent;
                    let y = cy + (py as f64 / 499. - 0.5) * extent;
                    let ix = x.round().clamp(0., (n - 1) as f64) as usize;
                    let iy = y.round().clamp(0., (n - 1) as f64) as usize;
                    let h = sample(z, n, x, y);
                    let gx = (sample(z, n, x + 1., y) - sample(z, n, x - 1., y)) * 0.5;
                    let gy = (sample(z, n, x, y + 1.) - sample(z, n, x, y - 1.)) * 0.5;
                    let name = &self.textures[tiles[iy * n + ix] as usize / 64];
                    let mut color = if name.contains("Dirt_Yucatan06") {
                        [151., 119., 72.]
                    } else if name.contains("Gravel") {
                        [135., 133., 111.]
                    } else if name.contains("Rock") {
                        [115., 112., 101.]
                    } else if h < p.number("WATER_Z", 200.) {
                        [39., 100., 157.]
                    } else {
                        [110., 145., 65.]
                    };
                    let light = 0.58
                        + ((-gx * 0.18 * (-0.55) - gy * 0.18 * (-0.55) + 0.63)
                            / (1. + (gx * gx + gy * gy) * 0.0324).sqrt())
                        .clamp(-0.25, 1.)
                            * 0.53;
                    for c in &mut color {
                        *c = (*c * light).clamp(0., 255.);
                    }
                    board.pixel(
                        (20 + panel * 510 + px) as isize,
                        (85 + py) as isize,
                        color.map(|c| c as u8),
                    );
                }
            }
        }
        board.png(&p.assets.join(filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let r = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"w":25.,"h":25.,"span":40.,"width":20.,"auto":true,"pair":[210.,350.]});
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
        let p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "碧嶂回澜").unwrap();
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
}
