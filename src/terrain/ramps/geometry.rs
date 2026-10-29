//! Ramp profiles, protected footprints and navigation preservation.
use super::*;

pub(super) fn smooth(v: f64) -> f64 {
    let v = v.clamp(0., 1.);
    v * v * (3. - 2. * v)
}
pub(super) fn delta(a: f64, b: f64) -> f64 {
    (a - b + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
pub(super) fn frame(r: &Value, x: f64, y: f64) -> (f64, f64) {
    let ux = number(r, "ux", 1.);
    let uy = number(r, "uy", 0.);
    let norm = ux.hypot(uy).max(1e-12);
    let dx = x - number(r, "x", 0.);
    let dy = y - number(r, "y", 0.);
    ((dx * ux + dy * uy) / norm, (-dx * uy + dy * ux) / norm)
}
pub(super) fn half(r: &Value) -> f64 {
    (number(r, "span", 25.) * 0.5).clamp(10., 20.)
}
pub(super) fn mouth(r: &Value, u: f64) -> f64 {
    terrain::ramp_width(r).expect("validated ramp width")
        * 0.5
        * (1. + 0.25 * (u.abs() / half(r)).clamp(0., 1.).powi(2))
}
pub(super) fn contains(r: &Value, x: f64, y: f64, pad: f64) -> bool {
    let (u, v) = frame(r, x, y);
    u.abs() <= half(r) + 5. + pad && v.abs() <= mouth(r, u) + 5. + pad
}
pub(super) fn old_foot(r: &Value, x: f64, y: f64, pad: f64) -> bool {
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
pub(super) fn normal(base: &[f64], n: usize, r: &Value) -> f64 {
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
pub(super) fn candidates(base: &[f64], n: usize, old: &Value) -> Vec<Value> {
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
                r["style"] = json!("mountain");
                r["surface_style"] = json!("mountain");
                list.push((score, r));
            }
        }
    }
    list.sort_by(|a, b| a.0.total_cmp(&b.0));
    list.into_iter().map(|(_, r)| r).collect()
}

pub(super) fn preserves_resource_approaches(
    objects: &[MapObject],
    frame: &crate::coordinates::Frame,
    heights: (&[f64], &[f64]),
    ramps: (
        &crate::terrain::ramp_clearance::Ramp,
        &crate::terrain::ramp_clearance::Ramp,
    ),
    policy: &crate::terrain::ramp_clearance::Policy,
) -> bool {
    let (old, candidate) = ramps;
    let unchanged = old.center == candidate.center
        && old.unit == candidate.unit
        && old.half == candidate.half
        && old.width == candidate.width
        && old.pair == candidate.pair;
    for object in objects {
        if crate::terrain::ramp_clearance::radius(&object.type_name).is_none() {
            continue;
        }
        let point = frame.grid([object.position[0] as f64, object.position[1] as f64]);
        if !frame.playable(point) {
            continue;
        }
        // Match the public report's actual terrain sample, including the
        // object's level relative to the two ramp endpoints.
        let sample = point[1].round() as usize * frame.width + point[0].round() as usize;
        let conflicts = |ramp, height| {
            crate::terrain::ramp_clearance::conflicts(
                point,
                object.angle as f64,
                &object.type_name,
                height,
                std::slice::from_ref(ramp),
                policy,
            )
        };
        let current = conflicts(candidate, heights.1[sample]);
        let Some(current) = current.first() else {
            continue;
        };
        // A moved or turned opening has a new identity in the public
        // transition check and cannot inherit an old resource conflict.
        if !unchanged {
            return false;
        }
        let previous = conflicts(old, heights.0[sample]);
        let Some(previous) = previous.first() else {
            return false;
        };
        if current.body_gap + 1e-6 < previous.body_gap
            || current.approach_gap + 1e-6 < previous.approach_gap
            || match (current.refinery_gap, previous.refinery_gap) {
                (Some(a), Some(b)) => a + 1e-6 < b,
                (None, None) => false,
                _ => true,
            }
        {
            return false;
        }
    }
    true
}

pub(super) fn protected(p: &Project, objects: &[MapObject], n: usize) -> Vec<bool> {
    let mut mask = vec![false; n * n];
    let specs = crate::gameplay::scripts::load_rules(p)
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
        let [cx, cy] = cell(p, o, n);
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
pub(super) fn sculpt(
    z: &[f64],
    ground: (&[f64], &[f64]),
    n: usize,
    old: &Value,
    new: &Value,
    locks: &[bool],
    scale: f64,
) -> Option<Vec<(usize, f64)>> {
    if new["style"] == "mountain" {
        return mountain::sculpt(z, ground, n, old, new, locks, scale);
    }
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
pub(super) fn validate_shape(z: &[f64], n: usize, r: &Value, border: usize) -> Option<Value> {
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
pub(super) fn labels(z: &[f64], imp: &[bool], n: usize, water: f64, sea: bool) -> Vec<usize> {
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
            for k in crate::terrain::grid::neighbors(j, n) {
                if labels[k] == 0 && !imp[k] && (z[k] < water) == sea {
                    labels[k] = next;
                    q.push_back(k);
                }
            }
        }
    }
    labels
}
pub(super) fn preserve_connections(before: &[usize], after: &[usize], keys: &[usize]) -> bool {
    for (i, &a) in keys.iter().enumerate() {
        for &b in &keys[i..] {
            if before[a] != 0 && before[a] == before[b] && (after[a] == 0 || after[a] != after[b]) {
                return false;
            }
        }
    }
    true
}
pub(super) fn masks(
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
                let coast =
                    z[j] >= water && crate::terrain::grid::neighbors(j, n).any(|k| z[k] < water);
                imp[j] = slope[j] > p.number("SLOPE_IMPASSABLE", 24.)
                    || z[j] > water + p.number("PEAK_IMPASSABLE", 235.)
                    || terrain::edge(j, n, border) < 3.
                    || (coast && !contains(r, xx as f64, yy as f64, 0.));
            }
        }
    }
    imp
}
pub(super) fn cell(p: &Project, o: &MapObject, n: usize) -> [f64; 2] {
    let mut frame = crate::coordinates::Frame::from_project(p);
    frame.width = n;
    frame.height = n;
    frame.grid([o.position[0] as f64, o.position[1] as f64])
}

pub(super) fn blueprint(
    p: &Project,
    n: usize,
    path: &std::path::Path,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let archive = crate::formats::archive::unzip(&fs::read(path)?)?;
    let s = crate::formats::npy::Npy::parse(&archive["z_scale.npy"])?.numbers()?[0];
    let base_values = crate::formats::npy::Npy::parse(&archive["base_z.npy"])?.numbers()?;
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
