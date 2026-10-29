//! 栅格分层、连通区域、边界追踪和无损逐行数据。
use super::Frame;
use crate::terrain::grid::neighbors_rect as neighbors;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, Serialize)]
pub(super) struct Layer {
    pub(super) id: usize,
    pub(super) label: String,
    pub(super) nominal_height: Option<f64>,
    pub(super) color: [u8; 3],
    pub(super) cells: usize,
    pub(super) playable_cells: usize,
    pub(super) flat_cells: usize,
    pub(super) slope_cells: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct Region {
    pub(super) id: usize,
    pub(super) layer: usize,
    pub(super) cells: usize,
    pub(super) playable_cells: usize,
    pub(super) flat_cells: usize,
    pub(super) slope_cells: usize,
    pub(super) seed: [usize; 2],
    pub(super) bounds: [usize; 4],
    pub(super) centroid: [f64; 2],
    pub(super) height_range: [f64; 2],
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Boundary {
    pub(super) id: usize,
    pub(super) kind: String,
    pub(super) regions: [usize; 2],
    pub(super) layers: [usize; 2],
    pub(super) closed: bool,
    pub(super) segments: usize,
    pub(super) playable_segments: usize,
    pub(super) height_range: [f64; 2],
    pub(super) points: Vec<[f64; 2]>,
}

#[derive(Clone)]
pub(super) struct Edge {
    pub(super) a: (i32, i32),
    pub(super) b: (i32, i32),
    pub(super) playable: bool,
    pub(super) heights: [f64; 2],
}

pub(super) struct Raster {
    pub(super) frame: Frame,
    pub(super) raw: Vec<u16>,
    pub(super) z: Vec<f64>,
    pub(super) class: Vec<usize>,
    pub(super) regions: Vec<usize>,
    pub(super) flags: Vec<u8>,
}

type BoundaryKey = (usize, usize, u8);

pub(super) fn classify(h: f64, water: f64, peak: f64, levels: &[f64]) -> usize {
    if h <= water {
        0
    } else if h > peak {
        levels.len() + 1
    } else {
        levels
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - h).abs().total_cmp(&(b.1 - h).abs()))
            .map(|(i, _)| i + 1)
            .unwrap()
    }
}
pub(super) fn layer_regions(r: &mut Raster, layers: &mut [Layer]) -> Vec<Region> {
    let f = &r.frame;
    let w = f.width;
    let h = f.height;
    let mut output = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..w * h {
        if r.regions[start] != 0 {
            continue;
        }
        let id = output.len() + 1;
        let layer = r.class[start];
        let mut region = Region {
            id,
            layer,
            cells: 0,
            playable_cells: 0,
            flat_cells: 0,
            slope_cells: 0,
            seed: [start % w, start / w],
            bounds: [start % w, start / w, start % w, start / w],
            centroid: [0., 0.],
            height_range: [f64::INFINITY, f64::NEG_INFINITY],
        };
        r.regions[start] = id;
        queue.push_back(start);
        while let Some(i) = queue.pop_front() {
            let x = i % w;
            let y = i / w;
            region.cells += 1;
            region.playable_cells += usize::from(f.playable([x as f64, y as f64]));
            region.flat_cells += usize::from(r.flags[i] & 8 != 0);
            region.slope_cells += usize::from(r.flags[i] & 4 != 0);
            region.bounds[0] = region.bounds[0].min(x);
            region.bounds[1] = region.bounds[1].min(y);
            region.bounds[2] = region.bounds[2].max(x);
            region.bounds[3] = region.bounds[3].max(y);
            region.centroid[0] += x as f64;
            region.centroid[1] += y as f64;
            region.height_range[0] = region.height_range[0].min(r.z[i]);
            region.height_range[1] = region.height_range[1].max(r.z[i]);
            for next in neighbors(i, w, h) {
                if r.regions[next] == 0 && r.class[next] == layer {
                    r.regions[next] = id;
                    queue.push_back(next);
                }
            }
        }
        region.centroid[0] /= region.cells as f64;
        region.centroid[1] /= region.cells as f64;
        let l = &mut layers[layer];
        l.cells += region.cells;
        l.playable_cells += region.playable_cells;
        l.flat_cells += region.flat_cells;
        l.slope_cells += region.slope_cells;
        output.push(region);
    }
    output
}
pub(super) fn boundary_groups(r: &Raster, cliff_step: f64) -> BTreeMap<BoundaryKey, Vec<Edge>> {
    let mut groups = BTreeMap::<BoundaryKey, Vec<Edge>>::new();
    let w = r.frame.width;
    let h = r.frame.height;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            for (next, a, b) in [
                (
                    if x + 1 < w { Some(i + 1) } else { None },
                    (2 * x as i32 + 1, 2 * y as i32 - 1),
                    (2 * x as i32 + 1, 2 * y as i32 + 1),
                ),
                (
                    if y + 1 < h { Some(i + w) } else { None },
                    (2 * x as i32 - 1, 2 * y as i32 + 1),
                    (2 * x as i32 + 1, 2 * y as i32 + 1),
                ),
            ] {
                let Some(j) = next else { continue };
                if r.class[i] == r.class[j] {
                    continue;
                }
                let kind = if r.class[i] == 0 || r.class[j] == 0 {
                    0
                } else if (r.z[i] - r.z[j]).abs() > cliff_step {
                    1
                } else {
                    2
                };
                let pair = if r.regions[i] < r.regions[j] {
                    [r.regions[i], r.regions[j]]
                } else {
                    [r.regions[j], r.regions[i]]
                };
                groups
                    .entry((pair[0], pair[1], kind))
                    .or_default()
                    .push(Edge {
                        a,
                        b,
                        playable: r.frame.playable([x as f64, y as f64])
                            && r.frame.playable([(j % w) as f64, (j / w) as f64]),
                        heights: [r.z[i], r.z[j]],
                    });
            }
        }
    }
    groups
}
pub(super) fn trace(edges: &[Edge]) -> Vec<Vec<usize>> {
    let mut graph = BTreeMap::<(i32, i32), Vec<usize>>::new();
    for (i, e) in edges.iter().enumerate() {
        graph.entry(e.a).or_default().push(i);
        graph.entry(e.b).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut result = Vec::new();
    let walk = |start: (i32, i32), first: usize, used: &mut [bool]| {
        let mut path = Vec::new();
        let mut v = start;
        let mut edge = first;
        loop {
            if used[edge] {
                break;
            }
            used[edge] = true;
            path.push(edge);
            let e = &edges[edge];
            v = if e.a == v { e.b } else { e.a };
            let adjacent = &graph[&v];
            if adjacent.len() != 2 {
                break;
            }
            let Some(next) = adjacent.iter().copied().find(|i| !used[*i]) else {
                break;
            };
            edge = next;
        }
        path
    };
    for (v, adjacent) in &graph {
        if adjacent.len() == 2 {
            continue;
        }
        for &edge in adjacent {
            if !used[edge] {
                result.push(walk(*v, edge, &mut used));
            }
        }
    }
    for i in 0..edges.len() {
        if !used[i] {
            result.push(walk(edges[i].a, i, &mut used));
        }
    }
    result
}
pub(super) fn boundaries(r: &Raster, regions: &[Region], threshold: f64) -> Vec<Boundary> {
    let mut result = Vec::new();
    for ((a, b, kind), edges) in boundary_groups(r, threshold) {
        for path in trace(&edges) {
            let first = path[0];
            let mut start = edges[first].a;
            if path.len() > 1 {
                let e = &edges[path[1]];
                if e.a == start || e.b == start {
                    start = edges[first].b;
                }
            }
            let mut v = start;
            let mut points = vec![[v.0 as f64 / 2., v.1 as f64 / 2.]];
            let mut heights = [f64::INFINITY, f64::NEG_INFINITY];
            let mut playable = 0;
            for &i in &path {
                let e = &edges[i];
                v = if e.a == v { e.b } else { e.a };
                points.push([v.0 as f64 / 2., v.1 as f64 / 2.]);
                playable += usize::from(e.playable);
                heights[0] = heights[0].min(e.heights[0].min(e.heights[1]));
                heights[1] = heights[1].max(e.heights[0].max(e.heights[1]));
            }
            result.push(Boundary {
                id: result.len() + 1,
                kind: match kind {
                    0 => "shoreline",
                    1 => "cliff",
                    _ => "slope_transition",
                }
                .into(),
                regions: [a, b],
                layers: [regions[a - 1].layer, regions[b - 1].layer],
                closed: v == start,
                segments: path.len(),
                playable_segments: playable,
                height_range: heights,
                points,
            });
        }
    }
    result
}
pub(super) fn terrain_runs(r: &Raster) -> Vec<Vec<[usize; 6]>> {
    let w = r.frame.width;
    let mut rows = Vec::new();
    for y in 0..r.frame.height {
        let mut row = Vec::new();
        let mut x = 0;
        while x < w {
            let i = y * w + x;
            let begin = x;
            x += 1;
            while x < w {
                let j = y * w + x;
                if r.raw[j] != r.raw[i]
                    || r.class[j] != r.class[i]
                    || r.regions[j] != r.regions[i]
                    || r.flags[j] != r.flags[i]
                {
                    break;
                }
                x += 1;
            }
            row.push([
                begin,
                x - 1,
                r.raw[i] as usize,
                r.class[i],
                r.regions[i],
                r.flags[i] as usize,
            ]);
        }
        rows.push(row);
    }
    rows
}
