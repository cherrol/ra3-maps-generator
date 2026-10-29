//! 高程平滑、形态处理、连通碎层合并和游戏层级设计。
use super::Frame;
use crate::{Result, err, terrain::grid::neighbors_rect as neighbors};
use serde_json::{Value, json};
use std::collections::VecDeque;

pub(super) fn box_mean(src: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    let mut integral = vec![0.; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut s = 0.;
        for x in 0..w {
            s += src[y * w + x];
            integral[(y + 1) * (w + 1) + x + 1] = integral[y * (w + 1) + x + 1] + s;
        }
    }
    let mut dst = vec![0.; w * h];
    for y in 0..h {
        for x in 0..w {
            let (x0, x1, y0, y1) = (
                x.saturating_sub(r),
                (x + r + 1).min(w),
                y.saturating_sub(r),
                (y + r + 1).min(h),
            );
            dst[y * w + x] = (integral[y1 * (w + 1) + x1]
                - integral[y0 * (w + 1) + x1]
                - integral[y1 * (w + 1) + x0]
                + integral[y0 * (w + 1) + x0])
                / ((x1 - x0) * (y1 - y0)) as f64;
        }
    }
    dst
}
// 3/4 chamfer distance, in thirds of a cell. Uses 8-neighbor diagonals.
pub(super) fn distance(mask: &[bool], w: usize, h: usize, source: bool) -> Vec<u32> {
    let mut d: Vec<u32> = mask
        .iter()
        .map(|&v| if v == source { 0 } else { 1000000 })
        .collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                d[i] = d[i].min(d[i - 1] + 3);
            }
            if y > 0 {
                d[i] = d[i].min(d[i - w] + 3);
                if x > 0 {
                    d[i] = d[i].min(d[i - w - 1] + 4);
                }
                if x + 1 < w {
                    d[i] = d[i].min(d[i - w + 1] + 4);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                d[i] = d[i].min(d[i + 1] + 3);
            }
            if y + 1 < h {
                d[i] = d[i].min(d[i + w] + 3);
                if x > 0 {
                    d[i] = d[i].min(d[i + w - 1] + 4);
                }
                if x + 1 < w {
                    d[i] = d[i].min(d[i + w + 1] + 4);
                }
            }
        }
    }
    d
}
pub(super) fn open(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let d = distance(mask, w, h, false);
    let core: Vec<bool> = d.iter().map(|&d| d >= (r * 3) as u32).collect();
    distance(&core, w, h, true)
        .iter()
        .map(|&d| d <= (r * 3) as u32)
        .collect()
}
pub(super) fn close(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    open(&mask.iter().map(|v| !v).collect::<Vec<_>>(), w, h, r)
        .iter()
        .map(|v| !v)
        .collect()
}
pub(super) fn components(a: &[u8], mask: &[bool], w: usize, h: usize) -> Vec<(u8, Vec<usize>)> {
    let mut seen = vec![false; a.len()];
    let mut out = Vec::new();
    for i in 0..a.len() {
        if !mask[i] || seen[i] || a[i] == 0 {
            continue;
        }
        let mut cells = vec![i];
        seen[i] = true;
        let mut j = 0;
        while j < cells.len() {
            let at = cells[j];
            j += 1;
            for k in neighbors(at, w, h) {
                if mask[k] && !seen[k] && a[k] == a[i] {
                    seen[k] = true;
                    cells.push(k);
                }
            }
        }
        out.push((a[i], cells));
    }
    out
}
pub(super) fn prune(
    a: &mut [u8],
    mask: &[bool],
    f: &Frame,
    area: usize,
    peak_area: usize,
) -> usize {
    let mut changed = 0;
    // Iterative smallest-first merges favor an adjacent height and the longest shared edge.
    for _ in 0..6 {
        let mut cc = components(a, mask, f.w, f.h);
        cc.sort_by_key(|v| v.1.len());
        let mut pass = 0;
        for (layer, cells) in cc {
            if cells.len() >= if layer == 5 { peak_area } else { area } {
                continue;
            }
            let mut edges = [0usize; 6];
            for &i in &cells {
                for k in neighbors(i, f.w, f.h) {
                    if mask[k] && a[k] != layer && a[k] > 0 {
                        edges[a[k] as usize] += 1;
                    }
                }
            }
            let target = (1u8..=5)
                .filter(|&l| edges[l as usize] > 0)
                .max_by_key(|&l| edges[l as usize] * 100 / (1 + layer.abs_diff(l) as usize))
                .unwrap_or(layer);
            if target != layer {
                for i in cells {
                    a[i] = target;
                    pass += 1;
                }
            }
        }
        changed += pass;
        if pass == 0 {
            break;
        }
    }
    changed
}
pub(super) fn layered_terrain(
    dem: &[f64],
    land: &[bool],
    f: &Frame,
    thresholds: &[f64],
    p: &Value,
) -> (Vec<u8>, usize) {
    let param = |key: &str| p[key].as_u64().unwrap() as usize;
    let mut smooth = dem.to_vec();
    for _ in 0..3 {
        smooth = box_mean(&smooth, f.w, f.h, param("dem_box_blur_radius_cells"));
    }
    let mut layers = vec![1u8; dem.len()];
    for (i, &threshold) in thresholds.iter().enumerate() {
        let high: Vec<bool> = smooth.iter().map(|&z| z >= threshold).collect();
        let radius = if i == 3 {
            param("peak_open_radius_cells")
        } else {
            param("cumulative_open_radius_cells")
        };
        let high = close(
            &open(&high, f.w, f.h, radius),
            f.w,
            f.h,
            param("cumulative_close_radius_cells"),
        );
        for k in 0..layers.len() {
            if high[k] {
                layers[k] = (i + 2) as u8;
            }
        }
    }
    for _ in 0..2 {
        let mut best = vec![0.; dem.len()];
        let mut next = layers.clone();
        for layer in 1u8..=5 {
            let votes: Vec<f64> = layers
                .iter()
                .map(|&l| if l == layer { 1. } else { 0. })
                .collect();
            for (i, v) in box_mean(&votes, f.w, f.h, param("majority_radius_cells"))
                .into_iter()
                .enumerate()
            {
                if v > best[i] {
                    best[i] = v;
                    next[i] = layer;
                }
            }
        }
        layers = next;
    }
    let merged = prune(
        &mut layers,
        land,
        f,
        param("minimum_component_area_cells"),
        param("minimum_peak_area_cells"),
    );
    // Remove narrow same-layer necks without altering water or the province silhouette.
    let mut seed_layer = vec![0u8; layers.len()];
    let mut dist = vec![usize::MAX; layers.len()];
    let mut queue = VecDeque::new();
    for l in 1u8..=5 {
        let region: Vec<bool> = layers
            .iter()
            .zip(land)
            .map(|(&a, &m)| a == l && m)
            .collect();
        let d = distance(&region, f.w, f.h, false);
        for i in 0..layers.len() {
            if region[i] && d[i] >= param("minimum_neck_width_cells") as u32 * 3 / 2 {
                seed_layer[i] = l;
                dist[i] = 0;
                queue.push_back(i);
            }
        }
    }
    while let Some(i) = queue.pop_front() {
        for k in neighbors(i, f.w, f.h) {
            if land[k] && dist[k] == usize::MAX {
                dist[k] = dist[i] + 1;
                seed_layer[k] = seed_layer[i];
                queue.push_back(k);
            }
        }
    }
    for i in 0..layers.len() {
        if land[i] && seed_layer[i] > 0 {
            layers[i] = seed_layer[i];
        }
    }
    prune(
        &mut layers,
        land,
        f,
        param("minimum_component_area_cells"),
        param("minimum_peak_area_cells"),
    );
    (layers, merged)
}

pub(super) fn layer_counts(layers: &[u8], land: &[bool]) -> [usize; 6] {
    let mut counts = [0; 6];
    for (&layer, &is_land) in layers.iter().zip(land) {
        if is_land {
            counts[layer as usize] += 1;
        }
    }
    counts
}

/// Keep the national altitude bands unless a plateau or a single surviving band
/// needs a local adjustment. All decisions use the same smoothed DEM as slicing.
pub(super) fn design_layers(
    dem: &[f64],
    land: &[bool],
    f: &Frame,
    mut thresholds: Vec<f64>,
    p: &Value,
) -> Result<(Vec<u8>, Vec<f64>, Value)> {
    let requested = thresholds.clone();
    let mut smoothed = dem.to_vec();
    for _ in 0..3 {
        smoothed = box_mean(
            &smoothed,
            f.w,
            f.h,
            p["dem_box_blur_radius_cells"].as_u64().unwrap() as usize,
        );
    }
    let mut elevations: Vec<f64> = smoothed
        .iter()
        .zip(land)
        .filter_map(|(&z, &m)| m.then_some(z))
        .collect();
    elevations.sort_by(f64::total_cmp);
    let quantile = |q: f64| elevations[((elevations.len() - 1) as f64 * q).round() as usize];
    let mut adjustments = Vec::new();
    // A high, broad plateau is a playable terrace. Reserve the fifth band for
    // the upper tail at least 750 m above the local median, never its whole floor.
    if quantile(0.5) >= 3000. {
        let raised = thresholds[3].max(quantile(0.90)).max(quantile(0.5) + 750.);
        if raised > thresholds[3] {
            thresholds[3] = raised;
            adjustments.push(
                json!({"rule":"plateau_peak_threshold", "threshold_m":raised,
                "median_smoothed_elevation_m":quantile(0.5),
                "reason":"retain broad plateau surfaces as playable terraces"}),
            );
        }
    }
    let (mut layers, _) = layered_terrain(dem, land, f, &thresholds, p);
    let count = |a: &[u8]| {
        layer_counts(a, land)[1..]
            .iter()
            .filter(|&&n| n > 0)
            .count()
    };
    if count(&layers) < 2 {
        // Try central quantiles because a median boundary can run through water
        // or be lost when small regions and narrow necks are cleaned up.
        for q in [0.5, 0.4, 0.6, 0.3, 0.7] {
            let split = quantile(q);
            if split <= elevations[0] || split >= elevations[elevations.len() - 1] {
                continue;
            }
            let upper = elevations[elevations.len() - 1].max(split) + 1.;
            let local = vec![split, upper, upper + 1., upper + 2.];
            let (candidate, _) = layered_terrain(dem, land, f, &local, p);
            if count(&candidate) >= 2 {
                layers = candidate;
                thresholds = local;
                adjustments.push(json!({"rule":"local_two_layer_split", "split_m":split,
                    "quantile":q,"reason":"fewer than two land layers survived cleanup"}));
                break;
            }
        }
    }
    if count(&layers) < 2 {
        return Err(err(
            "真实高程经整理后不足两层，无法按当地高程补分；请提供更完整的真实地形或调整本图平滑/连片参数",
        ));
    }
    let counts = layer_counts(&layers, land);
    let active: Vec<_> = (1..=5).filter(|&l| counts[l] > 0).collect();
    let details = json!({"national_thresholds_m":[200.,500.,1500.,4000.],
        "requested_thresholds_m":requested,"adjustments":adjustments,
        "active_land_layers":active,"land_layer_cell_counts":counts,
        "minimum_land_layers":2,"maximum_land_layers":5,
        "unused_layers_omitted":true,"water_is_separate":true});
    Ok((layers, thresholds, details))
}
