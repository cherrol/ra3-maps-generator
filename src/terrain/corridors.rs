//! Read-only final functional-reservation corridor measurement on native fields.
//! Width is a lower bound from exact Euclidean obstacle-centre distance minus
//! the occupied cell's half diagonal, never an average or a centre-only reach test.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile},
    pipeline::project::Project,
    terrain::{
        self, Layout,
        grid::{exact_squared_distance, neighbors_rect},
        ramp_clearance, resource_routes, spatial, spawn_placement,
    },
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
};

const HALF_CELL_DIAGONAL: f64 = std::f64::consts::FRAC_1_SQRT_2;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Domain {
    Land,
    Water,
}
impl Domain {
    fn name(self) -> &'static str {
        match self {
            Self::Land => "land",
            Self::Water => "water",
        }
    }
}
struct Raster {
    frame: Frame,
    occupied: Vec<u32>,
    own_base: Vec<BTreeMap<usize, u32>>,
}
impl Raster {
    fn polygon(&self, polygon: &[[f64; 2]]) -> Vec<usize> {
        let Some((x0, y0, x1, y1)) = bounds(&self.frame, polygon) else {
            return Vec::new();
        };
        let mut cells = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if cell_intersects(polygon, [x as f64, y as f64]) {
                    cells.push(y * self.frame.width + x);
                }
            }
        }
        cells
    }
    fn reserve(&mut self, cells: BTreeSet<usize>, seat: Option<usize>) {
        for i in cells {
            self.occupied[i] += 1;
            if let Some(seat) = seat {
                *self.own_base[seat].entry(i).or_default() += 1;
            }
        }
    }
    fn disk(&self, center: [f64; 2], radius: f64) -> BTreeSet<usize> {
        let extent = [
            [center[0] - radius, center[1] - radius],
            [center[0] + radius, center[1] + radius],
        ];
        let Some((x0, y0, x1, y1)) = bounds(&self.frame, &extent) else {
            return BTreeSet::new();
        };
        let mut cells = BTreeSet::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = ((x as f64 - center[0]).abs() - 0.5).max(0.);
                let dy = ((y as f64 - center[1]).abs() - 0.5).max(0.);
                if dx * dx + dy * dy <= radius * radius {
                    cells.insert(y * self.frame.width + x);
                }
            }
        }
        cells
    }
}
fn bounds(f: &Frame, points: &[[f64; 2]]) -> Option<(usize, usize, usize, usize)> {
    let lo = [
        points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min) - 0.5,
        points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min) - 0.5,
    ];
    let hi = [
        points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
            + 0.5,
        points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max)
            + 0.5,
    ];
    if hi[0] < 0. || hi[1] < 0. || lo[0] > (f.width - 1) as f64 || lo[1] > (f.height - 1) as f64 {
        return None;
    }
    Some((
        lo[0].ceil().max(0.) as usize,
        lo[1].ceil().max(0.) as usize,
        hi[0].floor().min((f.width - 1) as f64) as usize,
        hi[1].floor().min((f.height - 1) as f64) as usize,
    ))
}
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn cell_intersects(poly: &[[f64; 2]], cell: [f64; 2]) -> bool {
    let square = [
        [cell[0] - 0.5, cell[1] - 0.5],
        [cell[0] + 0.5, cell[1] - 0.5],
        [cell[0] + 0.5, cell[1] + 0.5],
        [cell[0] - 0.5, cell[1] + 0.5],
    ];
    for shape in [poly, &square[..]] {
        for i in 0..shape.len() {
            let a = shape[i];
            let b = shape[(i + 1) % shape.len()];
            let axis = [b[1] - a[1], a[0] - b[0]];
            let project = |p: [f64; 2]| p[0] * axis[0] + p[1] * axis[1];
            let range = |points: &[[f64; 2]]| {
                points
                    .iter()
                    .map(|p| project(*p))
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(v), hi.max(v))
                    })
            };
            let a = range(poly);
            let b = range(&square);
            if a.1 < b.0 - 1e-9 || b.1 < a.0 - 1e-9 {
                return false;
            }
        }
    }
    true
}
fn hull(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    let mut out = Vec::new();
    for p in &points {
        while out.len() >= 2 && cross(out[out.len() - 2], out[out.len() - 1], *p) <= 0. {
            out.pop();
        }
        out.push(*p);
    }
    let split = out.len();
    for p in points.iter().rev().skip(1) {
        while out.len() > split && cross(out[out.len() - 2], out[out.len() - 1], *p) <= 0. {
            out.pop();
        }
        out.push(*p);
    }
    out.pop();
    out
}
fn point(value: &Value) -> Result<[f64; 2]> {
    let point: [f64; 2] = serde_json::from_value(value.clone())?;
    if point.iter().any(|v| !v.is_finite()) {
        return Err(err("corridor geometry contains non-finite coordinates"));
    }
    Ok(point)
}
fn polygon(value: &Value) -> Result<Vec<[f64; 2]>> {
    let points: Vec<[f64; 2]> = serde_json::from_value(value.clone())?;
    if points.len() < 3 || points.iter().flatten().any(|v| !v.is_finite()) {
        return Err(err("corridor foundation polygon is incomplete"));
    }
    let convex = hull(points.clone());
    if convex.len() != points.len() {
        return Err(err(
            "corridor foundation polygon must be nondegenerate and convex",
        ));
    }
    Ok(convex)
}
fn index(f: &Frame, point: [f64; 2]) -> Option<usize> {
    if !f.playable(point) {
        return None;
    }
    let x = point[0].round() as usize;
    let y = point[1].round() as usize;
    (x < f.width - f.border && y < f.height - f.border).then_some(y * f.width + x)
}
fn coords(i: usize, width: usize) -> [usize; 2] {
    [i % width, i / width]
}
fn radius(d2: f64) -> f64 {
    (d2.sqrt() - HALF_CELL_DIAGONAL).max(0.)
}
fn polyline_midpoint(points: &[[f64; 2]]) -> [f64; 2] {
    let mut remaining = points
        .windows(2)
        .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
        .sum::<f64>()
        * 0.5;
    for segment in points.windows(2) {
        let length = (segment[1][0] - segment[0][0]).hypot(segment[1][1] - segment[0][1]);
        if length > 0. && remaining <= length {
            let t = remaining / length;
            return [
                segment[0][0] + t * (segment[1][0] - segment[0][0]),
                segment[0][1] + t * (segment[1][1] - segment[0][1]),
            ];
        }
        remaining -= length;
    }
    points[points.len() / 2]
}
fn maximum_deviation(path: &[usize], line: &[[f64; 2]], width: usize) -> ([usize; 2], f64) {
    path.iter()
        .map(|&i| {
            let p = [(i % width) as f64, (i / width) as f64];
            let distance = line
                .windows(2)
                .map(|segment| {
                    let a = segment[0];
                    let delta = [segment[1][0] - a[0], segment[1][1] - a[1]];
                    let length = delta[0] * delta[0] + delta[1] * delta[1];
                    let t = if length > 0. {
                        ((p[0] - a[0]) * delta[0] + (p[1] - a[1]) * delta[1]) / length
                    } else {
                        0.
                    };
                    let t = t.clamp(0., 1.);
                    (p[0] - a[0] - t * delta[0]).hypot(p[1] - a[1] - t * delta[1])
                })
                .fold(f64::INFINITY, f64::min);
            (coords(i, width), distance)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a corridor path is nonempty")
}
fn failure(
    rule: &str,
    detail: impl Into<String>,
    route: Option<&resource_routes::Route>,
    objects: &[usize],
) -> Value {
    json!({"rule":rule,"detail":detail.into(),"route_id":route.map(|r|r.id),"seats":route.map(|r|[r.side_a_seat,r.side_b_seat]),"objects":objects})
}
struct Network<'a> {
    frame: &'a Frame,
    z: &'a [f64],
    terrain_bad: Vec<bool>,
    bad: Vec<bool>,
    d2: Vec<f64>,
    maximum_step: f64,
    domain: Domain,
    water: f64,
}
impl Network<'_> {
    fn step(&self, a: usize, b: usize) -> bool {
        self.domain == Domain::Water || (self.z[a] - self.z[b]).abs() <= self.maximum_step
    }
    fn wide(&self, i: usize, half: f64) -> bool {
        !self.bad[i] && radius(self.d2[i]) + 1e-9 >= half
    }
    fn path(&self, starts: &[usize], goals: &BTreeSet<usize>, half: f64) -> Option<Vec<usize>> {
        let mut previous = vec![usize::MAX; self.z.len()];
        let mut q = VecDeque::new();
        for &i in starts {
            if self.wide(i, half) {
                previous[i] = i;
                q.push_back(i);
            }
        }
        let mut end = None;
        while let Some(i) = q.pop_front() {
            if goals.contains(&i) {
                end = Some(i);
                break;
            }
            for j in neighbors_rect(i, self.frame.width, self.frame.height) {
                if previous[j] == usize::MAX && self.wide(j, half) && self.step(i, j) {
                    previous[j] = i;
                    q.push_back(j);
                }
            }
        }
        let mut i = end?;
        let mut path = vec![i];
        while previous[i] != i {
            i = previous[i];
            path.push(i);
        }
        path.reverse();
        Some(path)
    }
}
struct Access {
    base: [f64; 2],
    native_id: usize,
    entrances: Vec<usize>,
    previous: BTreeMap<usize, usize>,
}
impl Access {
    fn connector(&self, mut i: usize) -> Vec<usize> {
        let mut out = vec![i];
        while self.previous[&i] != i {
            i = self.previous[&i];
            out.push(i);
        }
        out.reverse();
        out
    }
}
fn access(
    network: &Network<'_>,
    raster: &Raster,
    base: [f64; 2],
    native_id: usize,
    seat: usize,
    base_radius: f64,
    half: f64,
) -> Result<Access> {
    let start =
        index(network.frame, base).ok_or_else(|| err("base anchor is outside playable frame"))?;
    let own = &raster.own_base[seat];
    let free = |i: usize| {
        !network.terrain_bad[i] && raster.occupied[i] <= own.get(&i).copied().unwrap_or(0)
    };
    if !free(start) {
        return Err(err(
            "base anchor is blocked by another foundation, resource or native terrain",
        ));
    }
    let mut previous = BTreeMap::from([(start, start)]);
    let mut q = VecDeque::from([start]);
    let mut entrances = Vec::new();
    // Bound access to the actual public starting-platform reserve and the required half-width.
    let limit = base_radius + half + HALF_CELL_DIAGONAL;
    while let Some(i) = q.pop_front() {
        if network.wide(i, half) {
            entrances.push(i);
        }
        for j in neighbors_rect(i, network.frame.width, network.frame.height) {
            let p = [
                (j % network.frame.width) as f64,
                (j / network.frame.width) as f64,
            ];
            if !previous.contains_key(&j)
                && free(j)
                && network.step(i, j)
                && (p[0] - base[0]).hypot(p[1] - base[1]) <= limit
            {
                previous.insert(j, i);
                q.push_back(j);
            }
        }
    }
    if entrances.is_empty() {
        return Err(err(
            "no full-width corridor entrance is connected to the base inside its public platform access area",
        ));
    }
    Ok(Access {
        base,
        native_id,
        entrances,
        previous,
    })
}
fn rows_for_routes(
    network: &Network<'_>,
    raster: &Raster,
    layout: &Layout,
    native_bases: &BTreeMap<usize, (usize, [f64; 2])>,
    routes: &[resource_routes::Route],
    base_radius: f64,
    width: f64,
) -> (Vec<Value>, Vec<Value>) {
    let half = width / 2.;
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut access_cache = BTreeMap::new();
    for route in routes {
        let seats = [route.side_a_seat, route.side_b_seat];
        let ids = seats
            .iter()
            .filter_map(|s| native_bases.get(s).map(|v| v.0))
            .collect::<Vec<_>>();
        let mut valid = true;
        for seat in seats {
            if seat == 0 || seat > layout.base_cells.len() {
                failures.push(failure(
                    "corridor_route_seat",
                    "resource route names an absent base seat",
                    Some(route),
                    &ids,
                ));
                valid = false;
                continue;
            }
            let Some(&(id, base)) = native_bases.get(&seat) else {
                failures.push(failure(
                    "corridor_base_identity",
                    format!("P{seat} has no unique actual native start waypoint"),
                    Some(route),
                    &ids,
                ));
                valid = false;
                continue;
            };
            let Some(i) = index(network.frame, base) else {
                valid = false;
                failures.push(failure(
                    "corridor_base_bounds",
                    format!("P{seat} is outside the playable frame"),
                    Some(route),
                    &ids,
                ));
                continue;
            };
            let domain = if network.z[i] < network.water {
                Domain::Water
            } else {
                Domain::Land
            };
            if domain != network.domain {
                valid = false;
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = access_cache.entry(seat) {
                entry.insert(access(
                    network,
                    raster,
                    base,
                    id,
                    seat - 1,
                    base_radius,
                    half,
                ));
            }
            if let Err(error) = &access_cache[&seat] {
                failures.push(failure(
                    "corridor_base_access",
                    format!("P{seat}: {error}"),
                    Some(route),
                    &ids,
                ));
                valid = false;
            }
        }
        if !valid {
            continue;
        }
        let a = access_cache[&seats[0]]
            .as_ref()
            .expect("validated base access");
        let b = access_cache[&seats[1]]
            .as_ref()
            .expect("validated base access");
        let goals = b.entrances.iter().copied().collect::<BTreeSet<_>>();
        for (kind, declared) in [
            ("main_path", &route.main_path),
            ("secondary_path", &route.secondary_path),
        ] {
            if declared.len() < 2
                || declared
                    .iter()
                    .any(|p| p.iter().any(|v| !v.is_finite()) || !network.frame.playable(*p))
            {
                let mut f = failure(
                    "corridor_declared_route",
                    format!("{kind} is incomplete or outside the playable frame"),
                    Some(route),
                    &ids,
                );
                f["kind"] = json!(kind);
                failures.push(f);
                continue;
            }
            let path = if kind == "main_path" {
                network.path(&a.entrances, &goals, half)
            } else {
                let mid = polyline_midpoint(declared);
                index(network.frame, mid)
                    .filter(|i| network.wide(*i, half))
                    .and_then(|mid| {
                        network
                            .path(&a.entrances, &BTreeSet::from([mid]), half)
                            .and_then(|first| {
                                network.path(&[mid], &goals, half).map(|second| {
                                    first
                                        .into_iter()
                                        .chain(second.into_iter().skip(1))
                                        .collect()
                                })
                            })
                    })
            };
            let declared_measurements=declared.iter().map(|p| {let i=index(network.frame,*p).expect("validated authored point");json!({"cell":p,"width_lower_bound_cells":2.*radius(network.d2[i]),"blocked":network.bad[i]})}).collect::<Vec<_>>();
            let Some(path) = path else {
                let mut f = failure(
                    "corridor_width_or_connectivity",
                    format!(
                        "{kind}: no continuous route meets the {width}-cell width lower bound and native step policy"
                    ),
                    Some(route),
                    &ids,
                );
                f["kind"] = json!(kind);
                f["declared_measurements"] = json!(declared_measurements);
                failures.push(f);
                continue;
            };
            let minimum = path
                .iter()
                .map(|&i| 2. * radius(network.d2[i]))
                .fold(f64::INFINITY, f64::min);
            let begin = *path.first().expect("path is nonempty");
            let end = *path.last().expect("path is nonempty");
            let (deviation_at, maximum_deviation) =
                maximum_deviation(&path, declared, network.frame.width);
            let start_connector = a.connector(begin);
            let mut end_connector = b.connector(end);
            end_connector.reverse();
            rows.push(json!({"route_id":route.id,"kind":kind,"seats":seats,"base_object_ids":[a.native_id,b.native_id],"bases":[a.base,b.base],"domain":network.domain.name(),"start_access":coords(begin,network.frame.width),"end_access":coords(end,network.frame.width),"start_connector":start_connector.iter().map(|&i|coords(i,network.frame.width)).collect::<Vec<_>>(),"end_connector":end_connector.iter().map(|&i|coords(i,network.frame.width)).collect::<Vec<_>>(),"path":path.iter().map(|&i|coords(i,network.frame.width)).collect::<Vec<_>>(),"path_steps":path.len()-1,"minimum_width_lower_bound_cells":minimum,"width_samples":path.iter().map(|&i|2.*radius(network.d2[i])).collect::<Vec<_>>(),"declared_measurements":declared_measurements,"maximum_deviation_from_declared_polyline_cells":maximum_deviation,"maximum_deviation_at":deviation_at,"passed":minimum+1e-9>=width}));
        }
    }
    (rows, failures)
}

/// Read configured policy/route inputs, inspect only the supplied native candidate, and return facts.
/// No map, preview, layout, report or source file is written.
pub fn check(p: &Project, m: &MapFile, layout: &Layout, official: &Value) -> Result<Value> {
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"schema_version":1,"kind":"final_functional_corridors","status":"not_applicable","reason":"terrain-only design has no authored combat pairings","passed":true,"authorizes_delivery":false,"rows":[],"findings":[]}),
        );
    }
    let Some(plan) = resource_routes::Plan::load(p)? else {
        return Ok(
            json!({"schema_version":1,"kind":"final_functional_corridors","status":"pending","passed":false,"rows":[],"findings":[failure("corridor_resource_plan","a formal resource plan is required for actual route pairings",None,&[])],"authorizes_delivery":false}),
        );
    };
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let n = frame.width * frame.height;
    let bt = BlendTiles::parse(m.get("BlendTileData")?, frame.width, frame.height)?;
    let z = terrain::flip(&hm.elevations, frame.width)
        .into_iter()
        .map(|v| f64::from(v) * frame.z_scale)
        .collect::<Vec<_>>();
    let imp = terrain::flip(&bt.impassability, frame.width);
    let spatial = spatial::policy(p)?;
    let resource = resource_routes::policy(p)?;
    let ramp = ramp_clearance::policy(p)?
        .ok_or_else(|| err("corridor check requires complete refinery reservation policy"))?;
    let spawn = spawn_placement::policy(p)?;
    let common = match fs::read(p.root.join("rules/coordinate-pipeline.json")) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_json::from_str(include_str!("../../rules/coordinate-pipeline.json"))?
        }
        Err(e) => return Err(e.into()),
    };
    let width = common["terrain_shape"]["combat_route_minimum_width_cells"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.)
        .ok_or_else(|| err("invalid public corridor minimum width"))?;
    let water = p.number("WATER_Z", 200.);
    if !water.is_finite() {
        return Err(err("invalid corridor water plane"));
    }
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let mut bases = BTreeMap::new();
    let mut failures = Vec::new();
    for (seat, expected) in layout.base_cells.iter().enumerate() {
        let name = format!("Player_{}_Start", seat + 1);
        let matches = objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                map::prop(&o.props, "waypointName").and_then(map::Value::text)
                    == Some(name.as_str())
            })
            .collect::<Vec<_>>();
        if let [(i, o)] = matches.as_slice() {
            let at = frame.grid([o.position[0] as f64, o.position[1] as f64]);
            if (at[0] - expected[0]).hypot(at[1] - expected[1]) < 0.001 {
                bases.insert(seat + 1, (*i + 1, at));
            }
        }
    }
    let mut raster = Raster {
        frame: frame.clone(),
        occupied: vec![0; n],
        own_base: vec![BTreeMap::new(); layout.base_cells.len()],
    };
    let mut coverage = Vec::new();
    if official["coverage_complete"] != true
        || official["passed"] != true
        || official["raw_passed"] != true
    {
        failures.push(failure("corridor_official_coverage","current complete official placement evidence is required; unresolved/failed geometry is not omitted",None,&[]));
    }
    let entries = official["entries"]
        .as_array()
        .ok_or_else(|| err("official corridor input has no entries"))?;
    for (entry_id, entry) in entries.iter().enumerate() {
        let kind = entry["kind"].as_str().unwrap_or("");
        if kind.eq_ignore_ascii_case("OreNode") {
            continue;
        }
        let identity = entry["object_id"].as_u64().map(|i| i as usize);
        let ids = identity.into_iter().collect::<Vec<_>>();
        let polys = entry["placement_polygons"]
            .as_array()
            .filter(|v| !v.is_empty());
        let Some(polys) = polys else {
            failures.push(failure(
                "corridor_foundation_missing",
                format!("official entry {entry_id} {kind} has no complete foundation"),
                None,
                &ids,
            ));
            continue;
        };
        let mut cells = BTreeSet::new();
        for poly in polys {
            cells.extend(raster.polygon(&polygon(poly)?));
        }
        let mut exit_clearance = None;
        if !entry["production_exit"].is_null() {
            let exit = &entry["production_exit"];
            let a = point(&exit["create_cell"])?;
            let b = point(&exit["rally_cell"])?;
            let clearance = exit["clearance_cells"]
                .as_f64()
                .filter(|v| v.is_finite() && *v > 0.)
                .ok_or_else(|| {
                    err("official production exit has no explicit positive sweep clearance")
                })?;
            let sweep = hull(
                [a, b]
                    .into_iter()
                    .flat_map(|p| {
                        [
                            [-clearance, -clearance],
                            [clearance, -clearance],
                            [clearance, clearance],
                            [-clearance, clearance],
                        ]
                        .map(|d| [p[0] + d[0], p[1] + d[1]])
                    })
                    .collect(),
            );
            cells.extend(raster.polygon(&sweep));
            exit_clearance = Some(clearance);
        }
        let seat = entry["waypoint"]
            .as_str()
            .and_then(|name| {
                layout.base_cells.iter().enumerate().find_map(|(seat, _)| {
                    (name == format!("Player_{}_Start", seat + 1)).then_some(seat)
                })
            })
            .or_else(|| {
                kind.to_ascii_lowercase()
                    .contains("constructionyard")
                    .then(|| entry["cell"].as_array())
                    .flatten()
                    .and_then(|_| point(&entry["cell"]).ok())
                    .and_then(|at| {
                        layout
                            .base_cells
                            .iter()
                            .position(|p| (p[0] - at[0]).hypot(p[1] - at[1]) < 0.001)
                    })
            });
        coverage.push(json!({"official_entry":entry_id,"kind":kind,"object_id":entry["object_id"],"waypoint":entry["waypoint"],"seat":entry["seat"],"cell":entry["cell"],"heading_radians":entry["heading_radians"],"foundation_polygons":polys.len(),"reserved_cells":cells.len(),"production_exit_clearance_cells":exit_clearance}));
        raster.reserve(cells, seat);
    }
    let mut ore_rows = Vec::new();
    for (i, o) in objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.type_name == "OreNode")
    {
        let at = frame.grid([o.position[0] as f64, o.position[1] as f64]);
        let angle = f64::from(o.angle);
        let radius = ramp_clearance::radius("OreNode")
            .expect("ore radius is a public fixed model")
            .max(spawn.ore_radius)
            .max(spatial.ore_reserve_radius);
        let mut cells = raster.disk(at, radius);
        cells.extend(raster.polygon(&ramp_clearance::rectangle(
            at,
            [angle.cos(), -angle.sin()],
            -ramp.refinery_back,
            ramp.refinery_forward,
            ramp.refinery_half_width,
        )));
        ore_rows.push(json!({"object_id":i+1,"cell":at,"radius_cells":radius,"angle_radians":angle,"reserved_cells":cells.len()}));
        raster.reserve(cells, None);
    }
    let mut route_ids = BTreeSet::new();
    for route in &plan.routes {
        if !route_ids.insert(route.id) {
            failures.push(failure(
                "corridor_duplicate_route",
                "route IDs must be unique",
                Some(route),
                &[],
            ));
        }
    }
    if plan.routes.len() != resource.route_count {
        failures.push(failure(
            "corridor_route_count",
            format!(
                "resource plan has {} routes; public policy requires {}",
                plan.routes.len(),
                resource.route_count
            ),
            None,
            &[],
        ));
    }
    let mut domains = BTreeMap::<Domain, Vec<resource_routes::Route>>::new();
    for route in &plan.routes {
        let domains_for_pair = [route.side_a_seat, route.side_b_seat].map(|seat| {
            bases
                .get(&seat)
                .and_then(|(_, at)| index(&frame, *at))
                .map(|i| {
                    if z[i] < water {
                        Domain::Water
                    } else {
                        Domain::Land
                    }
                })
        });
        match domains_for_pair { [Some(a),Some(b)] if a==b=>domains.entry(a).or_default().push(route.clone()),[Some(_),Some(_)]=>failures.push(failure("corridor_cross_domain","paired bases occupy different movement domains; no amphibious/transport ability may be inferred",Some(route),&[bases[&route.side_a_seat].0,bases[&route.side_b_seat].0])),_=>failures.push(failure("corridor_base_identity","paired native start waypoint is missing or differs from layout",Some(route),&[])) }
    }
    let geometry_complete = !failures.iter().any(|f| {
        matches!(
            f["rule"].as_str(),
            Some("corridor_official_coverage" | "corridor_foundation_missing")
        )
    });
    let mut rows = Vec::new();
    for (domain, routes) in domains {
        let terrain_bad = (0..n)
            .map(|i| {
                imp[i]
                    || !frame.playable([(i % frame.width) as f64, (i / frame.width) as f64])
                    || (z[i] < water) != (domain == Domain::Water)
                    || (domain == Domain::Land
                        && neighbors_rect(i, frame.width, frame.height)
                            .any(|j| (z[i] - z[j]).abs() > spatial.maximum_walk_step))
            })
            .collect::<Vec<_>>();
        let bad = terrain_bad
            .iter()
            .enumerate()
            .map(|(i, blocked)| *blocked || raster.occupied[i] > 0)
            .collect::<Vec<_>>();
        let mut d2 = exact_squared_distance(&bad, frame.width)?;
        // A zero authored border still has a finite map exterior. Account for
        // virtual blocked sample cells immediately outside every grid edge.
        for (i, squared) in d2.iter_mut().enumerate() {
            let x = i % frame.width;
            let y = i / frame.width;
            let edge = [x + 1, frame.width - x, y + 1, frame.height - y]
                .into_iter()
                .min()
                .expect("four frame edges") as f64;
            *squared = squared.min(edge * edge);
        }
        let network = Network {
            frame: &frame,
            z: &z,
            terrain_bad,
            bad,
            d2,
            maximum_step: spatial.maximum_walk_step,
            domain,
            water,
        };
        let (found, issues) = rows_for_routes(
            &network,
            &raster,
            layout,
            &bases,
            &routes,
            spawn.base_radius,
            width,
        );
        rows.extend(found);
        failures.extend(issues);
    }
    let passed = failures.is_empty() && rows.len() == plan.routes.len() * 2;
    Ok(
        json!({"schema_version":1,"kind":"final_functional_corridors","native_decoded_crc32":crc32fast::hash(&m.encode()?),"official_report_crc32":crc32fast::hash(&serde_json::to_vec(official)?),"frame":frame,"status":if passed{"passed"}else{"failed"},"passed":passed,"rows":rows,"findings":failures,"failures":failures,
        "model":{"minimum_width_cells":width,"distance":"exact squared Euclidean lower-envelope transform on blocked cell centres","radius_lower_bound":"max(0,sqrt(distance_squared)-sqrt(0.5))","occupied_cell_half_diagonal":HALF_CELL_DIAGONAL,"polygon_rasterization":"all grid squares intersecting a complete convex foundation or continuous square exit sweep are reserved","neighbours":4,"outer_grid_boundary":"virtual blocked cells immediately outside the full grid, including BORDER=0","maximum_land_height_step":spatial.maximum_walk_step,"land_step_barriers":"both incident grid cells of every over-limit adjacent height edge are reserved before width measurement","water_movement_height":"horizontal WATER_Z plane; no land crossing inferred","base_access":"connected within public spawn base_radius plus required corridor half-width; own base foundation excluded only in the virtual anchor connector, other foundations and resources remain blocked","base_radius_cells":spawn.base_radius,"refinery":ramp,"route_selection":"main: full-width path between actual paired bases; secondary: also through the declared secondary polyline half-length point; actual paths and declared-path widths retained for independent direction/detour review"},
        "coverage":{"official_entries":entries.len(),"building_branches":coverage,"ore_reservations":ore_rows,"complete_official_report":official["coverage_complete"],"all_primary_fallback_conditional_foundations":geometry_complete,"continuous_production_exit_sweeps":geometry_complete},"authorizes_delivery":false,"limitations":["Static clearance lower bound, not runtime engine pathfinding or a proof that a failed conservative search has no narrower/alternate route.","Attack direction, unnecessary detours, full interface appearance and independent delivery judgment remain separate."]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        formats::map::{Chunk, Property, Value as NativeValue},
        terrain::{ObjectSpec, surface::Surface},
    };
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture {
        p: Project,
        map: MapFile,
        layout: Layout,
        official: Value,
        root: PathBuf,
        width: usize,
        height: usize,
    }
    impl Fixture {
        fn new(gap: Option<usize>) -> Self {
            let root = std::env::temp_dir().join(format!(
                "ra3-corridors-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mapdir = root.join("maps/synthetic");
            fs::create_dir_all(&mapdir).unwrap();
            let (width, height) = (96, 64);
            let bases = [[12., 32.], [84., 32.]];
            let p = Project {
                root: root.clone(),
                id: "synthetic".into(),
                cfg: json!({"GRID":width,"GRID_HEIGHT":height,"BORDER":2,"CELL":10.,"Z_SCALE":1.,"WATER_Z":200.,"RESOURCE_DISTRIBUTION":"resources.json"}),
                state: json!({}),
                version: "T0.0.1".into(),
                group: "Synthetic".into(),
                name: "SyntheticT0.0.1".into(),
                assets: root.join("assets"),
                dist: root.join("dist"),
            };
            let objects = bases
                .iter()
                .enumerate()
                .map(|(i, base)| ObjectSpec {
                    cell: *base,
                    angle: 0.,
                    version: 3,
                    road_type: 0,
                    type_name: "Waypoint".into(),
                    props: vec![Property {
                        key: "waypointName".into(),
                        value: NativeValue::Ascii(format!("Player_{}_Start", i + 1)),
                    }],
                })
                .collect::<Vec<_>>();
            let layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":bases,"ore_cells":[],"ore_angles":[],"objects":objects,"ramps":[]})).unwrap();
            let mut map = MapFile {
                order: Vec::new(),
                table: BTreeMap::new(),
                chunks: Vec::new(),
            };
            let hm = HeightMap {
                width,
                height,
                border: 2,
                borders: vec![[0, 0, (width - 4) as u32, (height - 4) as u32]],
                elevations: vec![210; width * height],
            };
            let mut imp = vec![false; width * height];
            if let Some(gap) = gap {
                let lo = 32 - gap / 2;
                for y in 0..height {
                    if y < lo || y >= lo + gap {
                        imp[y * width + 48] = true;
                    }
                }
            }
            let surface = Surface {
                chosen: vec![0; width * height],
                impassable: imp,
                build_block: vec![false; width * height],
                layer_edge: vec![false; width * height],
                shore: vec![false; width * height],
                textures: vec![[
                    "Grass_NewYork01".into(),
                    "TGrass_NewYork01".into(),
                    "TGrass_NewYork01_NRM".into(),
                ]],
            };
            for (name, version, payload) in [
                ("HeightMapData", 6, hm.encode().unwrap()),
                ("BlendTileData", 27, surface.encode(width, &[]).unwrap()),
            ] {
                let id = map.id(name).unwrap();
                map.chunks.push(Chunk {
                    name_idx: id,
                    version,
                    payload,
                });
            }
            map.id("waypointName").unwrap();
            map.id("Object").unwrap();
            let objects = layout
                .objects
                .iter()
                .map(|o| o.object(&p))
                .collect::<Vec<_>>();
            let id = map.id("ObjectsList").unwrap();
            map.chunks.push(Chunk {
                name_idx: id,
                version: 3,
                payload: map::write_objects(&objects, &map).unwrap(),
            });
            let routes=(1..=4).map(|id|json!({"id":id,"side_a_seat":1,"side_b_seat":2,"main_path":[bases[0],[48.,32.],bases[1]],"secondary_path":[bases[0],[48.,32.],bases[1]]})).collect::<Vec<_>>();
            fs::write(
                mapdir.join("resources.json"),
                serde_json::to_vec(&json!({"routes":routes,"slots":[]})).unwrap(),
            )
            .unwrap();
            let official =
                json!({"coverage_complete":true,"passed":true,"raw_passed":true,"entries":[]});
            Self {
                p,
                map,
                layout,
                official,
                root,
                width,
                height,
            }
        }
        fn check(&self) -> Value {
            check(&self.p, &self.map, &self.layout, &self.official).unwrap()
        }
        fn entry(&mut self, name: &str, poly: Value, exit: Value) {
            self.official["entries"].as_array_mut().unwrap().push(json!({"kind":"AlliedBarracks","waypoint":name,"seat":1,"cell":[46.,26.],"heading_radians":std::f64::consts::FRAC_PI_4,"placement_polygons":[poly],"production_exit":exit,"passed":true,"pending":[]}));
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
    #[test]
    fn rectangular_native_map_returns_all_pair_paths_and_per_sample_lower_bounds() {
        let f = Fixture::new(None);
        let before = f.map.encode().unwrap();
        let report = f.check();
        assert_eq!(report["passed"], true, "{}", report["findings"]);
        assert_eq!(report["rows"].as_array().unwrap().len(), 8);
        for row in report["rows"].as_array().unwrap() {
            assert!(row["minimum_width_lower_bound_cells"].as_f64().unwrap() >= 10.);
            assert_eq!(row["base_object_ids"], json!([1, 2]));
            assert!(!row["start_connector"].as_array().unwrap().is_empty());
            assert!(
                row["width_samples"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v.as_f64().unwrap() >= 10.)
            );
        }
        assert_eq!(before, f.map.encode().unwrap());
        assert_eq!(f.width, 96);
        assert_eq!(f.height, 64);
    }
    #[test]
    fn a_continuous_narrow_open_path_does_not_pass_full_width() {
        let f = Fixture::new(Some(8));
        let report = f.check();
        assert_eq!(report["passed"], false);
        assert!(
            report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["rule"] == "corridor_width_or_connectivity")
        );
        let wider = Fixture::new(Some(16));
        assert_eq!(wider.check()["passed"], true);
    }
    #[test]
    fn a_fallback_foundation_and_continuous_exit_sweep_are_both_reserved() {
        let mut f = Fixture::new(Some(16));
        assert_eq!(f.check()["passed"], true);
        f.entry(
            "gift-alt-blocking",
            json!([[46., 24.], [50., 24.], [50., 39.], [46., 39.]]),
            Value::Null,
        );
        let report = f.check();
        assert_eq!(report["passed"], false);
        assert_eq!(
            report["coverage"]["building_branches"][0]["waypoint"],
            "gift-alt-blocking"
        );
        f.official["entries"] = json!([]);
        f.entry("gift-main-with-exit",json!([[43.,24.],[45.,24.],[45.,26.],[43.,26.]]),json!({"create_cell":[48.,25.],"rally_cell":[48.,38.],"clearance_cells":0.5,"passed":true}));
        let report = f.check();
        assert_eq!(report["passed"], false);
        assert_eq!(
            report["coverage"]["building_branches"][0]["production_exit_clearance_cells"],
            0.5
        );
    }
    #[test]
    fn nearby_wide_ground_does_not_excuse_blocked_base_anchor_or_an_unwalkable_step() {
        let mut f = Fixture::new(None);
        f.entry(
            "unrelated-at-base",
            json!([[10., 30.], [14., 30.], [14., 34.], [10., 34.]]),
            Value::Null,
        );
        let report = f.check();
        assert_eq!(report["passed"], false);
        assert!(
            report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["rule"] == "corridor_base_access")
        );
        f.official["entries"] = json!([]);
        let mut hm = HeightMap::parse(f.map.get("HeightMapData").unwrap()).unwrap();
        for y in 0..f.height {
            hm.elevations[(f.height - 1 - y) * f.width + 48] = 280;
        }
        f.map.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        assert_eq!(f.check()["passed"], false);
    }
    #[test]
    fn refinery_rectangle_is_reserved_in_native_heading_and_not_reduced_to_ore_disc() {
        let frame = Frame {
            width: 128,
            height: 96,
            border: 2,
            cell_size: 10.,
            z_scale: 1.,
        };
        let raster = Raster {
            occupied: vec![0; 128 * 96],
            own_base: vec![],
            frame,
        };
        let ore = [22., 70.];
        let target = 48 * 128 + 64;
        assert!(!raster.disk(ore, 20.).contains(&target));
        let angle = std::f64::consts::FRAC_PI_4;
        let rectangle = ramp_clearance::rectangle(ore, [angle.cos(), -angle.sin()], -5., 48., 18.);
        assert!(raster.polygon(&rectangle).contains(&target));
    }
    #[test]
    fn missing_plan_and_incomplete_official_coverage_are_not_passes() {
        let mut f = Fixture::new(None);
        f.p.cfg
            .as_object_mut()
            .unwrap()
            .remove("RESOURCE_DISTRIBUTION");
        let report = f.check();
        assert_eq!(report["status"], "pending");
        assert_eq!(report["passed"], false);
        f.p.cfg["RESOURCE_DISTRIBUTION"] = json!("resources.json");
        f.official["coverage_complete"] = json!(false);
        assert_eq!(f.check()["passed"], false);
        f.p.cfg["TERRAIN_ONLY"] = json!(true);
        assert_eq!(f.check()["status"], "not_applicable");
    }
    #[test]
    fn conservative_polygon_rasterization_includes_cell_edges_and_all_square_sweep_positions() {
        let f = Frame {
            width: 16,
            height: 16,
            border: 1,
            cell_size: 10.,
            z_scale: 1.,
        };
        let r = Raster {
            occupied: vec![0; 256],
            own_base: vec![],
            frame: f,
        };
        let polygon = [[4.4, 4.4], [4.6, 4.4], [4.6, 4.6], [4.4, 4.6]];
        assert_eq!(r.polygon(&polygon).len(), 4);
        let swept = hull(vec![
            [2., 2.],
            [4., 2.],
            [4., 4.],
            [2., 4.],
            [8., 6.],
            [10., 6.],
            [10., 8.],
            [8., 8.],
        ]);
        assert!(r.polygon(&swept).contains(&(5 * 16 + 6)));
    }
}
