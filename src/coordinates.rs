//! Read-only spatial inventory of the actual saved map, in native grid/world frames.
use crate::{
    Result, archive,
    binary::Reader,
    err,
    map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    project::{self, Project},
    render::Image,
    scripts,
    terrain::{Layout, number},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub border: usize,
    pub cell_size: f64,
    pub z_scale: f64,
}
impl Frame {
    pub fn from_project(p: &Project) -> Self {
        Self {
            width: p.usize("GRID", 840),
            height: p.usize("GRID", 840),
            border: p.usize("BORDER", 20),
            cell_size: p.number("CELL", 10.),
            z_scale: p.number("Z_SCALE", 0.0390625),
        }
    }
    pub fn world(&self, p: [f64; 2]) -> [f64; 2] {
        [
            (p[0] - self.border as f64) * self.cell_size,
            (self.height as f64 - 1. - self.border as f64 - p[1]) * self.cell_size,
        ]
    }
    pub fn grid(&self, p: [f64; 2]) -> [f64; 2] {
        [
            p[0] / self.cell_size + self.border as f64,
            self.height as f64 - 1. - self.border as f64 - p[1] / self.cell_size,
        ]
    }
    pub fn inside(&self, p: [f64; 2]) -> bool {
        p[0] >= 0. && p[1] >= 0. && p[0] < self.width as f64 && p[1] < self.height as f64
    }
    pub fn playable(&self, p: [f64; 2]) -> bool {
        p[0] >= self.border as f64
            && p[1] >= self.border as f64
            && p[0] < (self.width - self.border) as f64
            && p[1] < (self.height - self.border) as f64
    }
}
#[derive(Clone, Debug, Serialize)]
struct Layer {
    id: usize,
    label: String,
    nominal_height: Option<f64>,
    color: [u8; 3],
    cells: usize,
    playable_cells: usize,
    flat_cells: usize,
    slope_cells: usize,
}
#[derive(Debug, Serialize)]
struct Region {
    id: usize,
    layer: usize,
    cells: usize,
    playable_cells: usize,
    flat_cells: usize,
    slope_cells: usize,
    seed: [usize; 2],
    bounds: [usize; 4],
    centroid: [f64; 2],
    height_range: [f64; 2],
}
#[derive(Clone, Debug, Serialize)]
struct Point {
    id: String,
    preview_id: Option<usize>,
    category: String,
    subtype: String,
    name: String,
    object_type: String,
    owner: String,
    owner_seat: Option<usize>,
    neutral: bool,
    grid: [f64; 2],
    world: [f64; 2],
    object_z: Option<f32>,
    surface_z: Option<f64>,
    layer: Option<usize>,
    region: Option<usize>,
    in_grid: bool,
    in_playable: bool,
    angle_radians: Option<f64>,
    waypoint: Option<String>,
    conditional: bool,
    properties: Json,
}
#[derive(Clone, Debug, Serialize)]
struct Boundary {
    id: usize,
    kind: String,
    regions: [usize; 2],
    layers: [usize; 2],
    closed: bool,
    segments: usize,
    playable_segments: usize,
    height_range: [f64; 2],
    points: Vec<[f64; 2]>,
}
#[derive(Clone)]
struct Edge {
    a: (i32, i32),
    b: (i32, i32),
    playable: bool,
    heights: [f64; 2],
}
type BoundaryKey = (usize, usize, u8);
struct Raster {
    frame: Frame,
    raw: Vec<u16>,
    z: Vec<f64>,
    class: Vec<usize>,
    regions: Vec<usize>,
    flags: Vec<u8>,
}
fn flip<T: Clone>(a: &[T], w: usize) -> Vec<T> {
    a.chunks(w)
        .rev()
        .flat_map(|row| row.iter().cloned())
        .collect()
}
fn neighbors(i: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let x = i % w;
    let y = i / w;
    [
        (x > 0).then(|| i - 1),
        (x + 1 < w).then(|| i + 1),
        (y > 0).then(|| i - w),
        (y + 1 < h).then(|| i + w),
    ]
    .into_iter()
    .flatten()
}
fn classify(h: f64, water: f64, peak: f64, levels: &[f64]) -> usize {
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
fn layer_regions(r: &mut Raster, layers: &mut [Layer]) -> Vec<Region> {
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
fn boundary_groups(r: &Raster, cliff_step: f64) -> BTreeMap<BoundaryKey, Vec<Edge>> {
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
fn trace(edges: &[Edge]) -> Vec<Vec<usize>> {
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
fn boundaries(r: &Raster, regions: &[Region], threshold: f64) -> Vec<Boundary> {
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
fn owner_seat(owner: &str) -> Option<usize> {
    match owner
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("team")
    {
        "PlyrCivilian" => Some(7),
        "PlyrCreeps" => Some(8),
        s => s
            .strip_prefix("Player_")
            .and_then(|s| s.parse().ok())
            .filter(|s| (1..=8).contains(s)),
    }
}
fn neutral(owner: &str) -> bool {
    matches!(
        owner.split('/').next().unwrap_or(""),
        "PlyrNeutral" | "SkirmishCivilian" | "SkirmishNeutral" | "_neutral_" | "Neutral"
    )
}
fn building(kind: &str) -> bool {
    let n = kind.to_ascii_lowercase();
    [
        "constructionyard",
        "barracks",
        "warfactory",
        "airfield",
        "navalyard",
        "powerplant",
        "basedefense",
        "basedefenseadvanced",
        "superweapon",
        "superweaponadvanced",
        "techstructure",
    ]
    .iter()
    .any(|s| n.ends_with(s))
        || n.starts_with("ei_structure_")
        || n.starts_with("st_")
}
fn category(kind: &str, waypoint: &str) -> (&'static str, &'static str) {
    if kind.contains("Waypoint") {
        if waypoint.starts_with("Player_") && waypoint.contains("_Start") {
            ("出生点", "玩家出生点")
        } else {
            ("航点", "标记点")
        }
    } else if kind == "OreNode" {
        ("资源", "矿脉")
    } else if kind == "OilDerrick" {
        ("资源", "油井")
    } else if kind.contains("Collapse") {
        ("装饰", "废墟")
    } else if building(kind) {
        if kind.contains("TechStructure") {
            ("建筑", "中立科技建筑")
        } else if kind.starts_with("EI_Structure_") || kind.starts_with("ST_") {
            ("建筑", "民用建筑")
        } else {
            ("建筑", "基地或生产建筑")
        }
    } else if kind.contains("Tree") {
        ("装饰", "树木")
    } else if kind.contains("Bush") {
        ("装饰", "灌木")
    } else if kind.contains("Rock") {
        ("装饰", "石头")
    } else if kind.contains("Sidewalk") {
        ("道路", "人行道")
    } else {
        ("其他", "其他对象")
    }
}
fn sample(r: &Raster, p: [f64; 2]) -> (Option<f64>, Option<usize>, Option<usize>) {
    if !r.frame.inside(p) {
        return (None, None, None);
    }
    let x = p[0].round().clamp(0., (r.frame.width - 1) as f64) as usize;
    let y = p[1].round().clamp(0., (r.frame.height - 1) as f64) as usize;
    let i = y * r.frame.width + x;
    let x0 = p[0].floor() as usize;
    let y0 = p[1].floor() as usize;
    let x1 = (x0 + 1).min(r.frame.width - 1);
    let y1 = (y0 + 1).min(r.frame.height - 1);
    let ax = p[0] - x0 as f64;
    let ay = p[1] - y0 as f64;
    let z = (r.z[y0 * r.frame.width + x0] * (1. - ax) + r.z[y0 * r.frame.width + x1] * ax)
        * (1. - ay)
        + (r.z[y1 * r.frame.width + x0] * (1. - ax) + r.z[y1 * r.frame.width + x1] * ax) * ay;
    (Some(z), Some(r.class[i]), Some(r.regions[i]))
}
fn point_at(
    r: &Raster,
    id: String,
    preview_id: Option<usize>,
    description: (&str, &str, String, String, String),
    p: [f64; 2],
) -> Point {
    let (category, subtype, name, object_type, owner) = description;
    let (z, layer, region) = sample(r, p);
    Point {
        id,
        preview_id,
        category: category.into(),
        subtype: subtype.into(),
        name,
        object_type,
        owner_seat: owner_seat(&owner),
        neutral: neutral(&owner),
        owner,
        grid: p,
        world: r.frame.world(p),
        object_z: None,
        surface_z: z,
        layer,
        region,
        in_grid: r.frame.inside(p),
        in_playable: r.frame.playable(p),
        angle_radians: None,
        waypoint: None,
        conditional: false,
        properties: Json::Null,
    }
}
fn angle(n: &scripts::Node) -> Result<Option<f64>> {
    let bytes = scripts::unhex(&n.body)?;
    let mut reader = Reader::new(&bytes);
    reader.take(8)?;
    for _ in 0..reader.u32()? {
        let ty = reader.u32()?;
        if ty == 16 {
            reader.take(12)?;
            continue;
        }
        reader.i32()?;
        let value = reader.f32()?;
        reader.s16()?;
        if ty == 17 {
            return Ok(Some(value as f64));
        }
    }
    Ok(None)
}
fn points(map: &MapFile, r: &Raster, layout: &Layout) -> Result<Vec<Point>> {
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut result = Vec::new();
    let mut waypoints = BTreeMap::new();
    for (i, o) in objects.iter().enumerate() {
        let wp = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .unwrap_or("");
        let owner = prop(&o.props, "originalOwner")
            .and_then(Value::text)
            .unwrap_or("")
            .to_owned();
        let name = prop(&o.props, "uniqueID")
            .and_then(Value::text)
            .unwrap_or(&o.type_name)
            .to_owned();
        let (c, s) = category(&o.type_name, wp);
        let mut point = point_at(
            r,
            format!("O{}", i + 1),
            Some(i + 1),
            (c, s, name, o.type_name.clone(), owner),
            r.frame.grid([o.position[0] as f64, o.position[1] as f64]),
        );
        point.object_z = Some(o.position[2]);
        point.angle_radians = Some(o.angle as f64);
        point.properties = serde_json::to_value(&o.props)?;
        if !wp.is_empty() {
            point.waypoint = Some(wp.into());
            waypoints.insert(wp.to_owned(), point.grid);
        }
        result.push(point);
    }
    for (i, ramp) in layout.ramps.iter().enumerate() {
        let p = [number(ramp, "x", 0.), number(ramp, "y", 0.)];
        let id = layout.objects.len() + i + 1;
        let mut point = point_at(
            r,
            format!("A{id}"),
            Some(id),
            (
                "斜坡",
                "已定稿斜坡",
                format!("斜坡 {id}"),
                "Ramp".into(),
                String::new(),
            ),
            p,
        );
        point.properties = ramp.clone();
        result.push(point);
    }
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut sites = BTreeMap::<(String, String, String), Option<f64>>::new();
    fn visit(
        nodes: &[scripts::Node],
        sites: &mut BTreeMap<(String, String, String), Option<f64>>,
    ) -> Result<()> {
        for n in nodes {
            if let Some(c) = &n.children {
                visit(c, sites)?;
            } else if n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(n)?;
                let Some(kind) = args
                    .iter()
                    .find(|(t, s)| *t == 15 && building(s))
                    .map(|(_, s)| s.clone())
                else {
                    continue;
                };
                let Some(owner) = args.iter().find(|(t, _)| *t == 3).map(|(_, s)| s.clone()) else {
                    continue;
                };
                let Some(wp) = args.iter().find(|(t, _)| *t == 7).map(|(_, s)| s.clone()) else {
                    continue;
                };
                sites.insert((kind, owner, wp), angle(n)?);
            }
        }
        Ok(())
    }
    visit(&nodes, &mut sites)?;
    for ((kind, owner, wp), angle) in sites {
        let position = *waypoints
            .get(&wp)
            .ok_or_else(|| err(format!("script building has no coordinate waypoint {wp}")))?;
        let id = format!("G{}", result.iter().filter(|p| p.conditional).count() + 1);
        let role = if wp.starts_with("gift-alt-") {
            "备用赠送点"
        } else if wp.starts_with("dual-") {
            "主赠送点"
        } else {
            "条件生成点"
        };
        let mut point = point_at(
            r,
            id,
            None,
            ("脚本建筑候选", role, format!("{kind} @ {wp}"), kind, owner),
            position,
        );
        point.conditional = true;
        point.angle_radians = angle;
        point.waypoint = Some(wp);
        result.push(point);
    }
    Ok(result)
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
fn csv_row(out: &mut String, fields: &[String]) {
    out.push_str(
        &fields
            .iter()
            .map(|s| quoted(s))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
}
fn write_points(path: &Path, points: &[Point], conditional: bool) -> Result<()> {
    let mut out = String::from(
        "\u{feff}id,preview_id,category,subtype,name,object_type,owner,owner_seat,neutral,x,y,world_x,world_y,surface_z,object_z_raw,layer,region,in_grid,in_playable,angle_radians,waypoint\n",
    );
    for p in points.iter().filter(|p| p.conditional == conditional) {
        csv_row(
            &mut out,
            &[
                p.id.clone(),
                p.preview_id.map(|v| v.to_string()).unwrap_or_default(),
                p.category.clone(),
                p.subtype.clone(),
                p.name.clone(),
                p.object_type.clone(),
                p.owner.clone(),
                p.owner_seat.map(|v| v.to_string()).unwrap_or_default(),
                p.neutral.to_string(),
                p.grid[0].to_string(),
                p.grid[1].to_string(),
                p.world[0].to_string(),
                p.world[1].to_string(),
                p.surface_z.map(|v| v.to_string()).unwrap_or_default(),
                p.object_z.map(|v| v.to_string()).unwrap_or_default(),
                p.layer.map(|v| v.to_string()).unwrap_or_default(),
                p.region.map(|v| v.to_string()).unwrap_or_default(),
                p.in_grid.to_string(),
                p.in_playable.to_string(),
                p.angle_radians.map(|v| v.to_string()).unwrap_or_default(),
                p.waypoint.clone().unwrap_or_default(),
            ],
        );
    }
    fs::write(path, out)?;
    Ok(())
}
fn terrain_runs(r: &Raster) -> Vec<Vec<[usize; 6]>> {
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

/// Preserve native ordering and import generated marker waypoints omitted by older caches.
pub fn native_layout(p: &Project, map: &MapFile, saved: &Layout) -> Result<(Layout, Json)> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let mut frame = Frame::from_project(p);
    frame.width = hm.width;
    frame.height = hm.height;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut used = std::collections::BTreeSet::new();
    let mut ids = Vec::new();
    for (i, a) in saved.objects.iter().enumerate() {
        let matching = objects
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                let c = frame.grid([b.position[0] as f64, b.position[1] as f64]);
                a.type_name == b.type_name
                    && a.angle == b.angle
                    && (a.cell[0] - c[0]).hypot(a.cell[1] - c[1]) < 0.001
                    && ["uniqueID", "waypointName"].iter().all(|key| {
                        prop(&a.props, key).and_then(Value::text)
                            == prop(&b.props, key).and_then(Value::text)
                    })
            })
            .map(|(j, _)| j)
            .collect::<Vec<_>>();
        if matching.len() != 1 || !used.insert(matching[0]) {
            return Err(err(
                "saved coordinate layout is stale or ambiguous relative to native objects",
            ));
        }
        ids.push(json!({"old_preview_id":i+1,"preview_id":matching[0]+1,"kind":"object"}));
    }
    let rules = crate::scripts::load_rules(p)?;
    for (i, o) in objects
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
    {
        let name = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .unwrap_or("");
        if !o.type_name.contains("Waypoint")
            || !rules.spawn_waypoints.iter().any(|s| s.name == name)
        {
            return Err(err(format!(
                "native object {} is absent from saved coordinate layout and is not a registered generated waypoint",
                i + 1
            )));
        }
        ids.push(json!({"old_preview_id":null,"preview_id":i+1,"kind":"generated_waypoint"}));
    }
    for i in 0..saved.ramps.len() {
        ids.push(json!({"old_preview_id":saved.objects.len()+i+1,"preview_id":objects.len()+i+1,"kind":"ramp"}));
    }
    let mut layout = saved.clone();
    layout.objects = objects
        .iter()
        .map(|o| crate::terrain::ObjectSpec {
            cell: frame.grid([o.position[0] as f64, o.position[1] as f64]),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    Ok((
        layout,
        json!({"schema_version":1,"map":p.name,"object_id_basis":"native object order","entries":ids}),
    ))
}

pub fn generate(p: &Project) -> Result<Json> {
    let source = p.assets.join(format!("{}.map", p.name));
    let bytes = fs::read(&source)?;
    let map = MapFile::load_bytes(&bytes)?;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let frame = Frame {
        width: hm.width,
        height: hm.height,
        border: hm.border as usize,
        cell_size: p.number("CELL", 10.),
        z_scale: p.number("Z_SCALE", 0.0390625),
    };
    let layout_path = p.source().join("layout.json");
    let saved: Layout = serde_json::from_slice(&fs::read(&layout_path)?)?;
    let (layout, mut id_map) = native_layout(p, &map, &saved)?;
    let authoring = p.path("authoring")?;
    if authoring.is_dir() {
        let mut paths = fs::read_dir(authoring)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        let mut previous = Vec::<Json>::new();
        for path in paths {
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("id-map-") && s.ends_with(".json"))
            {
                previous.push(serde_json::from_slice(&fs::read(path)?)?);
            }
        }
        id_map["previous_versions"] = json!(previous);
    }
    let actual_objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    if layout.objects.len() != actual_objects.len() {
        return Err(err(
            "coordinate report requires complete current source layout",
        ));
    }
    for (a, b) in layout.objects.iter().zip(&actual_objects) {
        let c = frame.grid([b.position[0] as f64, b.position[1] as f64]);
        if a.type_name != b.type_name || (a.cell[0] - c[0]).hypot(a.cell[1] - c[1]) > 0.001 {
            return Err(err(
                "coordinate source layout differs from current native map",
            ));
        }
    }
    let raw = flip(&hm.elevations, hm.width);
    let z: Vec<_> = raw.iter().map(|v| *v as f64 * frame.z_scale).collect();
    let water = p.number("WATER_Z", 200.);
    let peak = water + p.number("PEAK_IMPASSABLE", 235.);
    let mut levels = p.cfg["TERRAIN"]["levels"]
        .as_array()
        .ok_or_else(|| err("coordinate report requires terrain levels"))?
        .iter()
        .filter_map(Json::as_f64)
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    if levels.is_empty() {
        return Err(err("terrain layer list is empty"));
    }
    let palette = [
        [85, 147, 79],
        [156, 177, 92],
        [191, 160, 103],
        [195, 196, 193],
    ];
    let mut layers = vec![Layer {
        id: 0,
        label: "水域".into(),
        nominal_height: None,
        color: [55, 119, 164],
        cells: 0,
        playable_cells: 0,
        flat_cells: 0,
        slope_cells: 0,
    }];
    for (i, h) in levels.iter().enumerate() {
        layers.push(Layer {
            id: i + 1,
            label: format!("第{}层 · {}", i + 1, h),
            nominal_height: Some(*h),
            color: palette[i % palette.len()],
            cells: 0,
            playable_cells: 0,
            flat_cells: 0,
            slope_cells: 0,
        });
    }
    layers.push(Layer {
        id: layers.len(),
        label: format!("高山 · >{peak}"),
        nominal_height: None,
        color: [223, 225, 227],
        cells: 0,
        playable_cells: 0,
        flat_cells: 0,
        slope_cells: 0,
    });
    let imp = flip(&bt.impassability, hm.width);
    let build = flip(&bt.buildability, hm.width);
    let mut class = Vec::new();
    let mut flags = Vec::new();
    for i in 0..z.len() {
        let layer = classify(z[i], water, peak, &levels);
        let x = i % frame.width;
        let y = i / frame.width;
        let gx = (z[y * frame.width + (x + 1).min(frame.width - 1)]
            - z[y * frame.width + x.saturating_sub(1)])
            * 0.5;
        let gy = (z[(y + 1).min(frame.height - 1) * frame.width + x]
            - z[y.saturating_sub(1) * frame.width + x])
            * 0.5;
        let sloped = gx.hypot(gy) > 1.5
            || layers[layer]
                .nominal_height
                .is_some_and(|h| (z[i] - h).abs() > 2.);
        let flat = !sloped && layer > 0 && layer <= levels.len();
        class.push(layer);
        flags.push(
            u8::from(imp[i])
                | u8::from(build[i]) << 1
                | u8::from(sloped) << 2
                | u8::from(flat) << 3,
        );
    }
    let mut raster = Raster {
        frame,
        raw,
        z,
        class,
        regions: vec![0; hm.width * hm.height],
        flags,
    };
    let regions = layer_regions(&mut raster, &mut layers);
    let boundaries = boundaries(&raster, &regions, p.number("SLOPE_IMPASSABLE", 24.));
    let points = points(&map, &raster, &layout)?;
    let rows = terrain_runs(&raster);
    let mut categories = BTreeMap::<String, usize>::new();
    for point in &points {
        *categories.entry(point.category.clone()).or_default() += 1;
    }
    let mut boundary_counts = BTreeMap::from([
        ("shoreline".to_string(), 0usize),
        ("cliff".to_string(), 0),
        ("slope_transition".to_string(), 0),
    ]);
    let mut boundary_lengths = boundary_counts.clone();
    for b in &boundaries {
        *boundary_counts.entry(b.kind.clone()).or_default() += 1;
        *boundary_lengths.entry(b.kind.clone()).or_default() += b.segments;
    }
    let statistics = json!({"map_objects":actual_objects.len(),"ramps":layout.ramps.len(),"script_building_candidates":points.iter().filter(|p|p.conditional).count(),"off_grid_objects":points.iter().filter(|p|p.category!="斜坡"&&!p.conditional&&!p.in_grid).count(),"buildings":points.iter().filter(|p|p.category=="建筑").count(),"neutral_buildings":points.iter().filter(|p|p.category=="建筑"&&p.neutral).count(),"resource_points":points.iter().filter(|p|p.category=="资源").count(),"regions":regions.len(),"boundary_paths":boundaries.len(),"boundary_counts":boundary_counts,"boundary_unit_segments":boundary_lengths,"water_cells":layers[0].cells,"categories":categories,"terrain_grid_points":hm.width*hm.height,"terrain_runs":rows.iter().map(Vec::len).sum::<usize>()});
    let coordinate_system = json!({"origin":"northwest full-grid sample (0,0)","x_direction":"east","y_direction":"south","cell_size_world_units":raster.frame.cell_size,"full_grid_sample_bounds":[[0,0],[hm.width-1,hm.height-1]],"playable_sample_bounds":[[hm.border,hm.border],[hm.width-hm.border as usize-1,hm.height-hm.border as usize-1]],"world_transform":{"x":"(grid_x-border)*cell_size","y":"(height-1-border-grid_y)*cell_size"},"layer_assignment":"nearest configured nominal land height; slopes are flagged separately and ramp properties give actual endpoint levels","flat_rule":"within 2 height units of nominal and central gradient <=1.5","boundary_rule":"complete shared cell edges at half-grid coordinates; no invented shoreline outside terrain","rle_fields":["x_begin","x_end_inclusive","height_raw","layer_id","region_id","flags"],"flags":{"1":"impassable","2":"unbuildable","4":"sloped or transition","8":"flat platform"},"object_z":"raw native object Z; surface_z is bilinear terrain height, null outside raster","in_playable":"sample domain excludes authored border; use world_transform for exact native coordinates"});
    let data = json!({"schema_version":1,"map_id":p.id,"map_name":p.name,"version":p.version,"source_map":source.strip_prefix(&p.root).unwrap_or(&source),"source_crc32":crc32fast::hash(&bytes),"frame":raster.frame,"coordinate_system":coordinate_system,"statistics":statistics,"layers":layers,"regions":regions,"boundaries":boundaries,"points":points,"terrain_rows":rows});
    let dir = p.assets.join("coordinates");
    fs::create_dir_all(&dir)?;
    project::write_json(&dir.join("coordinates.json"), &data)?;
    project::write_json(&dir.join("coordinate-id-map.json"), &id_map)?;
    write_points(&dir.join("objects.csv"), &points, false)?;
    write_points(&dir.join("script-buildings.csv"), &points, true)?;
    let mut csv = String::from(
        "\u{feff}layer_id,label,nominal_height,full_grid_cells,playable_cells,flat_cells,slope_cells\n",
    );
    for l in &layers {
        csv_row(
            &mut csv,
            &[
                l.id.to_string(),
                l.label.clone(),
                l.nominal_height.map(|v| v.to_string()).unwrap_or_default(),
                l.cells.to_string(),
                l.playable_cells.to_string(),
                l.flat_cells.to_string(),
                l.slope_cells.to_string(),
            ],
        );
    }
    fs::write(dir.join("layers.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}region_id,layer_id,cells,playable_cells,flat_cells,slope_cells,min_x,min_y,max_x,max_y,centroid_x,centroid_y,seed_x,seed_y,min_height,max_height\n",
    );
    for r in &regions {
        csv_row(
            &mut csv,
            &[
                r.id.to_string(),
                r.layer.to_string(),
                r.cells.to_string(),
                r.playable_cells.to_string(),
                r.flat_cells.to_string(),
                r.slope_cells.to_string(),
                r.bounds[0].to_string(),
                r.bounds[1].to_string(),
                r.bounds[2].to_string(),
                r.bounds[3].to_string(),
                r.centroid[0].to_string(),
                r.centroid[1].to_string(),
                r.seed[0].to_string(),
                r.seed[1].to_string(),
                r.height_range[0].to_string(),
                r.height_range[1].to_string(),
            ],
        );
    }
    fs::write(dir.join("regions.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}boundary_id,kind,region_a,region_b,layer_a,layer_b,point_index,x,y,world_x,world_y,closed\n",
    );
    for b in &boundaries {
        for (i, pt) in b.points.iter().enumerate() {
            let world = raster.frame.world(*pt);
            csv_row(
                &mut csv,
                &[
                    b.id.to_string(),
                    b.kind.clone(),
                    b.regions[0].to_string(),
                    b.regions[1].to_string(),
                    b.layers[0].to_string(),
                    b.layers[1].to_string(),
                    i.to_string(),
                    pt[0].to_string(),
                    pt[1].to_string(),
                    world[0].to_string(),
                    world[1].to_string(),
                    b.closed.to_string(),
                ],
            );
        }
    }
    fs::write(dir.join("boundary-points.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}y,x_begin,x_end_inclusive,height_raw,height,layer_id,region_id,flags,world_x_begin,world_x_end,world_y\n",
    );
    for (y, row) in rows.iter().enumerate() {
        for run in row {
            let a = raster.frame.world([run[0] as f64, y as f64]);
            let b = raster.frame.world([run[1] as f64, y as f64]);
            csv_row(
                &mut csv,
                &[
                    y.to_string(),
                    run[0].to_string(),
                    run[1].to_string(),
                    run[2].to_string(),
                    (run[2] as f64 * raster.frame.z_scale).to_string(),
                    run[3].to_string(),
                    run[4].to_string(),
                    run[5].to_string(),
                    a[0].to_string(),
                    b[0].to_string(),
                    a[1].to_string(),
                ],
            );
        }
    }
    fs::write(dir.join("terrain-runs.csv"), csv)?;
    let encoded = serde_json::to_string(&data)?.replace('<', "\\u003c");
    fs::write(
        dir.join("index.html"),
        include_str!("coordinates/view.html").replace("__COORDINATE_DATA__", &encoded),
    )?;
    overview(
        &dir.join("overview.png"),
        &raster,
        &layers,
        &boundaries,
        &points,
    )?;
    write_readme(&dir, p, &data)?;
    let mut files = Vec::new();
    for f in [
        "index.html",
        "coordinates.json",
        "coordinate-id-map.json",
        "objects.csv",
        "script-buildings.csv",
        "layers.csv",
        "regions.csv",
        "boundary-points.csv",
        "terrain-runs.csv",
        "overview.png",
        "README.md",
    ] {
        files.push((format!("coordinates/{f}"), fs::read(dir.join(f))?));
    }
    fs::write(p.assets.join("coordinates.zip"), archive::zip(&files)?)?;
    if fs::read(&source)? != bytes {
        return Err(err("native map changed during coordinate generation"));
    }
    println!("坐标报告：{}；{}", dir.display(), statistics);
    Ok(statistics)
}

fn overview(
    path: &Path,
    r: &Raster,
    layers: &[Layer],
    boundaries: &[Boundary],
    points: &[Point],
) -> Result<()> {
    let scale = 3usize;
    let left = 75;
    let top = 90;
    let bottom = points
        .iter()
        .filter(|p| !p.conditional)
        .map(|p| p.grid[1])
        .fold(r.frame.height as f64, f64::max)
        .ceil() as usize
        + 12;
    let mut image = Image::new(
        left + r.frame.width * scale + 360,
        top + bottom * scale + 50,
        [23, 29, 38],
    );
    let font = crate::render::font();
    image.text(
        font.as_ref(),
        20,
        16,
        &format!(
            "地图坐标与地形层 · X向东 / Y向南 · 1格={}游戏单位",
            r.frame.cell_size
        ),
        26.,
        [238, 239, 230],
    );
    for i in 0..r.raw.len() {
        let color = layers[r.class[i]].color;
        for yy in 0..scale {
            for xx in 0..scale {
                image.pixel(
                    (left + (i % r.frame.width) * scale + xx) as isize,
                    (top + (i / r.frame.width) * scale + yy) as isize,
                    color,
                );
            }
        }
    }
    let line = |im: &mut Image, a: [f64; 2], b: [f64; 2], color: [u8; 3]| {
        let ax = left as f64 + a[0] * scale as f64;
        let ay = top as f64 + a[1] * scale as f64;
        let bx = left as f64 + b[0] * scale as f64;
        let by = top as f64 + b[1] * scale as f64;
        let n = (bx - ax).abs().max((by - ay).abs()).ceil().max(1.) as usize;
        for j in 0..=n {
            im.pixel(
                (ax + (bx - ax) * j as f64 / n as f64) as isize,
                (ay + (by - ay) * j as f64 / n as f64) as isize,
                color,
            );
        }
    };
    for b in boundaries {
        let c = if b.kind == "shoreline" {
            [60, 190, 240]
        } else if b.kind == "cliff" {
            [45, 47, 50]
        } else {
            [183, 83, 164]
        };
        for pair in b.points.windows(2) {
            line(&mut image, pair[0], pair[1], c);
        }
    }
    for x in (0..r.frame.width).step_by(50) {
        line(
            &mut image,
            [x as f64, 0.],
            [x as f64, r.frame.height as f64],
            [130, 139, 139],
        );
        image.text(
            font.as_ref(),
            (left + x * scale) as isize,
            top as isize - 28,
            &x.to_string(),
            16.,
            [220, 225, 225],
        );
    }
    for y in (0..r.frame.height).step_by(50) {
        line(
            &mut image,
            [0., y as f64],
            [r.frame.width as f64, y as f64],
            [130, 139, 139],
        );
        image.text(
            font.as_ref(),
            8,
            (top + y * scale) as isize,
            &y.to_string(),
            16.,
            [220, 225, 225],
        );
    }
    for p in points.iter().filter(|p| {
        !p.conditional && matches!(p.category.as_str(), "资源" | "建筑" | "出生点" | "斜坡")
    }) {
        let x = (left as f64 + p.grid[0] * scale as f64) as isize;
        let y = (top as f64 + p.grid[1] * scale as f64) as isize;
        let color = match p.category.as_str() {
            "资源" => [255, 215, 30],
            "建筑" => [190, 60, 210],
            "出生点" => [40, 145, 255],
            _ => [255, 65, 45],
        };
        image.circle(x, y, 5, color);
        if p.in_grid {
            image.text(
                font.as_ref(),
                x + 5,
                y - 5,
                &p.preview_id.unwrap().to_string(),
                13.,
                color,
            );
        }
    }
    let x = (left + r.frame.width * scale + 20) as isize;
    image.text(
        font.as_ref(),
        x,
        100,
        "地形标高 / 全图格点数",
        19.,
        [240, 240, 230],
    );
    for (i, l) in layers.iter().enumerate() {
        image.circle(x + 7, 150 + i as isize * 50, 7, l.color);
        image.text(
            font.as_ref(),
            x + 23,
            139 + i as isize * 50,
            &format!("{}: {}", l.label, l.cells),
            17.,
            [225, 229, 232],
        );
    }
    image.text(
        font.as_ref(),
        x,
        500,
        "深色：陡崖交界",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        535,
        "紫色：缓坡层级过渡",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        570,
        "蓝色：水陆交界",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        625,
        "完整点位请打开 index.html",
        17.,
        [220, 225, 230],
    );
    image.png(path)
}
fn write_readme(dir: &Path, p: &Project, data: &Json) -> Result<()> {
    let s = &data["statistics"];
    let mut out = format!(
        "# {} 坐标资料\n\n打开 `index.html` 查看离线交互地图，可按现有编号、名称、R区域号、B交界号或 `X,Y` 定位。\n\n坐标原点在完整高度网格的西北角；X 向东，Y 向南；1格={}游戏单位。完整采样网格 {}×{}，可玩采样索引为 X={}–{}、Y={}–{}。世界转换：`world_x=(x-border)*cell_size`，`world_y=(height-1-border-y)*cell_size`。对象 Z 原始值与脚下地形高度分列；图外对象的地形高度不猜测。\n\n## 统计\n\n原生对象 {} 个、斜坡 {} 条、脚本建筑候选 {} 条、图外对象 {} 个；建筑 {} 座（其中中立 {}），资源点 {}。水域格点 {}，分层连通区 {} 个，交界折线 {} 条。\n\n",
        p.name,
        data["frame"]["cell_size"],
        data["frame"]["width"],
        data["frame"]["height"],
        data["coordinate_system"]["playable_sample_bounds"][0][0],
        data["coordinate_system"]["playable_sample_bounds"][1][0],
        data["coordinate_system"]["playable_sample_bounds"][0][1],
        data["coordinate_system"]["playable_sample_bounds"][1][1],
        s["map_objects"],
        s["ramps"],
        s["script_building_candidates"],
        s["off_grid_objects"],
        s["buildings"],
        s["neutral_buildings"],
        s["resource_points"],
        s["water_cells"],
        s["regions"],
        s["boundary_paths"]
    );
    out.push_str("## 文件\n\n- `coordinate-id-map.json`：原生对象顺序编号与旧预览编号的对照，含已记录版本的转换历史。\n- `coordinates.json`：完整坐标系统、层统计、区域、交界折线、全部点和逐行地形数据。\n- `objects.csv`：原生对象与斜坡，保留预览编号、归属、格坐标和世界坐标。\n- `script-buildings.csv`：按实际脚本提取的条件生成/主备用候选点，不能当作已生成建筑。\n- `layers.csv`、`regions.csv`：层级及连通区域的格点数量、范围、中心与高度。\n- `boundary-points.csv`：每条层交接或岸线的所有折线顶点，半格精度，无抽稀。\n- `terrain-runs.csv`：每行的完整连续坐标范围，可无损还原所有高度格点、层、区域和通行/建造标记。\n- `overview.png`：带坐标网格的总览。\n\n层着色按最近的配置标高归类，缓坡可能经过多个标高带；`flat_cells` 只计算接近标高且平整的台地，坡面单独标记。实际斜坡连接层以斜坡点的 `pair` 为准。交界是相邻层级带共享格边：陡崖和缓坡过渡分别统计；水陆交界只从实际水域提取。区域为四邻接连通区，`R` 号和 `B` 号在本版报告内有效。\n\n对象编号对应当前地图预览，不等于游戏中的单位编队编号。中立标识区分 `PlyrNeutral`、`SkirmishCivilian` 与固定 AI 的 `PlyrCivilian`。总数含图外对象；统计分别给出完整网格和可玩区范围。\n\n本资料直接读取当前原生地图，不修改游戏文件，不递增地图版本。\n");
    fs::write(dir.join("README.md"), out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(class: Vec<usize>, w: usize) -> (Raster, Vec<Layer>) {
        let h = class.len() / w;
        let z: Vec<_> = class.iter().map(|i| [190., 212., 280.][*i]).collect();
        let layers = (0..3)
            .map(|id| Layer {
                id,
                label: id.to_string(),
                nominal_height: None,
                color: [0; 3],
                cells: 0,
                playable_cells: 0,
                flat_cells: 0,
                slope_cells: 0,
            })
            .collect();
        (
            Raster {
                frame: Frame {
                    width: w,
                    height: h,
                    border: 0,
                    cell_size: 10.,
                    z_scale: 1.,
                },
                raw: z.iter().map(|v| *v as u16).collect(),
                z,
                regions: vec![0; class.len()],
                flags: vec![8; class.len()],
                class,
            },
            layers,
        )
    }

    #[test]
    fn land_edges_do_not_invent_shorelines_and_diagonal_islands_stay_separate() {
        let (mut r, mut layers) = fixture(vec![1, 2, 2, 1], 2);
        let regions = layer_regions(&mut r, &mut layers);
        assert_eq!(regions.len(), 4); // Diagonal contact is not a four-neighbor connection.
        let paths = boundaries(&r, &regions, 24.);
        assert_eq!(paths.iter().map(|p| p.segments).sum::<usize>(), 4);
        assert!(paths.iter().all(|p| p.kind == "cliff"));
        assert_eq!(regions.iter().map(|r| r.cells).sum::<usize>(), 4);
    }

    #[test]
    fn shorelines_form_closed_complete_unit_edge_loops() {
        let (mut r, mut layers) = fixture(vec![1, 1, 1, 1, 0, 1, 1, 1, 1], 3);
        let regions = layer_regions(&mut r, &mut layers);
        let paths = boundaries(&r, &regions, 24.);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].kind, "shoreline");
        assert!(paths[0].closed);
        assert_eq!(paths[0].segments, 4);
        assert_eq!(paths[0].points.first(), paths[0].points.last());
        for pair in paths[0].points.windows(2) {
            assert_eq!(
                (pair[0][0] - pair[1][0]).abs() + (pair[0][1] - pair[1][1]).abs(),
                1.
            );
        }
    }

    #[test]
    fn terrain_runs_preserve_every_field_and_row_boundary() {
        let (mut r, mut layers) = fixture(vec![1, 1, 1, 1, 1, 1, 2, 2], 4);
        r.flags[1] = 4;
        r.raw[2] += 1;
        layer_regions(&mut r, &mut layers);
        let runs = terrain_runs(&r);
        let mut decoded = Vec::new();
        for row in runs {
            assert_eq!(row.first().unwrap()[0], 0);
            assert_eq!(row.last().unwrap()[1], 3);
            for pair in row.windows(2) {
                assert_eq!(pair[0][1] + 1, pair[1][0]);
            }
            for run in row {
                for _ in run[0]..=run[1] {
                    decoded.push([run[2], run[3], run[4], run[5]]);
                }
            }
        }
        for (i, value) in decoded.iter().enumerate() {
            assert_eq!(
                *value,
                [
                    r.raw[i] as usize,
                    r.class[i],
                    r.regions[i],
                    r.flags[i] as usize
                ]
            );
        }
        assert_eq!(flip(&[1, 2, 3, 4, 5, 6], 3), [4, 5, 6, 1, 2, 3]);
    }
    #[test]
    fn grid_world_roundtrip_includes_offmap_coordinates() {
        let f = Frame {
            width: 840,
            height: 840,
            border: 20,
            cell_size: 10.,
            z_scale: 0.0390625,
        };
        for p in [[20., 819.], [781., 411.], [801.8, 871.2], [-1., 15.]] {
            let q = f.grid(f.world(p));
            assert!((p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9);
        }
        assert_eq!(f.world([20., 819.]), [0., 0.]);
        assert!(!f.inside([801., 871.]));
    }
    #[test]
    fn fixed_ai_is_distinguished_from_neutral_buildings() {
        assert_eq!(owner_seat("PlyrCivilian/teamPlyrCivilian"), Some(7));
        assert!(!neutral("PlyrCivilian/teamPlyrCivilian"));
        assert!(neutral("SkirmishCivilian/teamSkirmishCivilian"));
        assert_eq!(
            category("HospitalTechStructure", ""),
            ("建筑", "中立科技建筑")
        );
    }
    #[test]
    fn boundary_traces_cover_every_edge_without_fake_connections() {
        let edges = vec![
            Edge {
                a: (1, 1),
                b: (3, 1),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 1),
                b: (3, 3),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 3),
                b: (1, 3),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (1, 3),
                b: (1, 1),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 1),
                b: (5, 1),
                playable: true,
                heights: [212., 280.],
            },
        ];
        let paths = trace(&edges);
        let all = paths.iter().flatten().copied().collect::<Vec<_>>();
        assert_eq!(all.len(), 5);
        assert_eq!(
            all.iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            5
        );
        assert!(paths.len() > 1);
    }
}
