//! Native spawn orientation and complete, connected starting-resource reservations.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject},
    pipeline::project::Project,
    terrain::{Layout, grid::neighbors_rect, ramp_clearance},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
};

/// Included in placement proofs so a rule change invalidates cached acceptance.
pub const RULES_VERSION: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub base_radius: f64,
    pub ore_radius: f64,
    pub flat_tolerance: f64,
    pub near_ore_radius: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            base_radius: crate::rules::number("spawn_placement", "base_radius"),
            ore_radius: crate::rules::number("spawn_placement", "ore_radius"),
            flat_tolerance: crate::rules::number("spawn_placement", "flat_tolerance"),
            near_ore_radius: crate::rules::number("spawn_placement", "near_ore_radius"),
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        if [
            self.base_radius,
            self.ore_radius,
            self.flat_tolerance,
            self.near_ore_radius,
        ]
        .iter()
        .any(|n| !n.is_finite() || *n <= 0.)
        {
            return Err(err(
                "invalid spawn placement policy: dimensions and tolerances must be finite and positive",
            ));
        }
        Ok(())
    }
    fn effective(mut self, p: &Project) -> Result<Self> {
        if let Some(value) = p.cfg["TERRAIN"].get("base_min_flat_radius") {
            let radius = value
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.)
                .ok_or_else(|| err("invalid configured base footprint radius"))?;
            self.base_radius = self.base_radius.max(radius);
        }
        for value in [
            p.cfg.get("ORE_CLEAR_RADIUS"),
            p.cfg["TERRAIN"].get("ore_mouth_r1"),
        ]
        .into_iter()
        .flatten()
        {
            let value = value
                .as_f64()
                .filter(|n| n.is_finite() && *n > 0.)
                .ok_or_else(|| err("invalid configured ore footprint radius"))?;
            self.ore_radius = self.ore_radius.max(value);
        }
        let mut reserve = None;
        let common = p.root.join("templates/rules/coordinate-pipeline.json");
        match fs::read(common) {
            Ok(bytes) => {
                reserve = serde_json::from_slice::<Value>(&bytes)?
                    .get("ore_reserve_radius")
                    .cloned()
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        if let Some(path) = p.cfg["COORDINATE_PIPELINE"].get("policy") {
            let path = path
                .as_str()
                .ok_or_else(|| err("spatial policy must name a file"))?;
            let overrides: Value = serde_json::from_slice(&fs::read(p.path(path)?)?)?;
            if let Some(value) = overrides.get("ore_reserve_radius") {
                reserve = Some(value.clone());
            }
        }
        if let Some(value) = reserve {
            let radius = value
                .as_f64()
                .filter(|n| n.is_finite() && *n > 0.)
                .ok_or_else(|| err("invalid spatial ore reservation radius"))?;
            self.ore_radius = self.ore_radius.max(radius);
        }
        self.validate()?;
        Ok(self)
    }
}
pub fn policy(p: &Project) -> Result<Policy> {
    let mut fields = serde_json::to_value(Policy::default())?;
    let common: Value = match fs::read(p.root.join("templates/rules/coordinate-pipeline.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.into()),
    };
    if let Some(overrides) = common.get("spawn_placement") {
        for (key, value) in overrides
            .as_object()
            .ok_or_else(|| err("spawn placement overrides must be an object"))?
        {
            fields[key] = value.clone();
        }
    }
    if let Some(radius) = p.cfg["TERRAIN"].get("ore_near_max_r") {
        fields["near_ore_radius"] = radius.clone();
    }
    if let Some(overrides) = p.cfg.get("SPAWN_PLACEMENT") {
        for (key, value) in overrides
            .as_object()
            .ok_or_else(|| err("SPAWN_PLACEMENT must be an object"))?
        {
            fields[key] = value.clone();
        }
    }
    serde_json::from_value::<Policy>(fields)?.effective(p)
}

struct Native {
    frame: Frame,
    z: Vec<f64>,
    imp: Vec<bool>,
    build: Vec<bool>,
    objects: Vec<MapObject>,
    bases: Vec<usize>,
    ores: Vec<usize>,
    water: f64,
}
impl Native {
    fn load(p: &Project, m: &MapFile, layout: &Layout) -> Result<Self> {
        let hm = HeightMap::parse(m.get("HeightMapData")?)?;
        let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
        let frame = Frame::from_heightmap(p, &hm)?;
        if hm.elevations.len() != frame.width * frame.height {
            return Err(err("spawn placement height dimensions differ"));
        }
        let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
        if layout.base_cells.len() != 8 {
            return Err(err("spawn placement requires eight declared starts"));
        }
        let mut bases = Vec::new();
        for seat in 1..=8 {
            let plain = format!("Player_{seat}_Start");
            let imported = format!("{plain} (imported)");
            let found = objects
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.type_name.contains("Waypoint")
                        && map::prop(&o.props, "waypointName")
                            .and_then(map::Value::text)
                            .is_some_and(|name| name == plain || name == imported)
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if found.len() != 1 {
                return Err(err(format!(
                    "spawn placement P{seat}: expected one native start, found {}",
                    found.len()
                )));
            }
            let index = found[0];
            let point = frame.grid([
                objects[index].position[0] as f64,
                objects[index].position[1] as f64,
            ]);
            let declared = layout.base_cells[seat - 1];
            if declared.iter().any(|n| !n.is_finite())
                || !frame.inside(point)
                || point.iter().any(|n| !n.is_finite())
                || (point[0] - declared[0]).hypot(point[1] - declared[1]) > 0.001
            {
                return Err(err(format!(
                    "spawn placement P{seat}: native start and base_cells differ or lie outside the grid"
                )));
            }
            bases.push(index);
        }
        let ores = objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.type_name == "OreNode")
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for &index in bases.iter().chain(&ores) {
            if objects[index].position.iter().any(|n| !n.is_finite())
                || !objects[index].angle.is_finite()
            {
                return Err(err("spawn placement object has non-finite geometry"));
            }
        }
        let flip = |values: &[bool]| {
            values
                .chunks(frame.width)
                .rev()
                .flatten()
                .copied()
                .collect()
        };
        let z = hm
            .elevations
            .chunks(frame.width)
            .rev()
            .flatten()
            .map(|n| *n as f64 * frame.z_scale)
            .collect();
        let imp = flip(&bt.impassability);
        let build = flip(&bt.buildability);
        let water = p.number("WATER_Z", 200.);
        if !water.is_finite() {
            return Err(err("spawn placement WATER_Z must be finite"));
        }
        Ok(Self {
            frame,
            z,
            imp,
            build,
            objects,
            bases,
            ores,
            water,
        })
    }
    fn point(&self, id: usize) -> [f64; 2] {
        let object = &self.objects[id];
        self.frame
            .grid([object.position[0] as f64, object.position[1] as f64])
    }
    fn index(&self, p: [f64; 2]) -> usize {
        p[1].round().clamp(0., (self.frame.height - 1) as f64) as usize * self.frame.width
            + p[0].round().clamp(0., (self.frame.width - 1) as f64) as usize
    }
    fn stamp(&self, ids: &[usize]) -> String {
        ids.iter()
            .map(|&id| {
                let o = &self.objects[id];
                format!(
                    "{}:{}:{:08x}:{:08x}:{:08x}:{}",
                    id,
                    o.type_name,
                    o.position[0].to_bits(),
                    o.position[1].to_bits(),
                    o.angle.to_bits(),
                    json!({"uniqueID":map::prop(&o.props,"uniqueID").and_then(map::Value::text),
                        "waypointName":map::prop(&o.props,"waypointName").and_then(map::Value::text),
                        "owner":map::prop(&o.props,"originalOwner").and_then(map::Value::text)})
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
    fn reachable(&self, base: [f64; 2], tolerance: f64) -> Vec<bool> {
        let start = self.index(base);
        let reference = self.z[start];
        let water = reference < self.water;
        let allowed = |i: usize| {
            let p = [(i % self.frame.width) as f64, (i / self.frame.width) as f64];
            self.frame.playable(p)
                && !self.imp[i]
                && (self.z[i] < self.water) == water
                && (water || (self.z[i] - reference).abs() <= tolerance)
        };
        let mut visited = vec![false; self.z.len()];
        if !allowed(start) {
            return visited;
        }
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        while let Some(i) = queue.pop_front() {
            for next in neighbors_rect(i, self.frame.width, self.frame.height) {
                if !visited[next] && allowed(next) {
                    visited[next] = true;
                    queue.push_back(next);
                }
            }
        }
        visited
    }
}

enum Shape {
    Disk([f64; 2], f64),
    Rectangle(ramp_clearance::Polygon),
}
impl Shape {
    fn bounds(&self) -> [f64; 4] {
        match self {
            Self::Disk(p, r) => [p[0] - r, p[1] - r, p[0] + r, p[1] + r],
            Self::Rectangle(points) => [
                points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
                points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
                points
                    .iter()
                    .map(|p| p[0])
                    .fold(f64::NEG_INFINITY, f64::max),
                points
                    .iter()
                    .map(|p| p[1])
                    .fold(f64::NEG_INFINITY, f64::max),
            ],
        }
    }
    fn covers(&self, p: [f64; 2]) -> bool {
        match self {
            Self::Disk(c, r) => {
                ((p[0] - c[0]).abs() - 0.5)
                    .max(0.)
                    .hypot(((p[1] - c[1]).abs() - 0.5).max(0.))
                    <= *r
            }
            Self::Rectangle(points) => ramp_clearance::intersects(
                points,
                &ramp_clearance::rectangle(p, [1., 0.], -0.5, 0.5, 0.5),
            ),
        }
    }
    fn overlap(&self, other: &Self) -> f64 {
        match (self, other) {
            (Self::Disk(a, ar), Self::Disk(b, br)) => {
                (ar + br - (a[0] - b[0]).hypot(a[1] - b[1])).max(0.)
            }
            (Self::Disk(center, radius), Self::Rectangle(polygon))
            | (Self::Rectangle(polygon), Self::Disk(center, radius)) => {
                let mut distance = f64::INFINITY;
                let mut positive = false;
                let mut negative = false;
                for i in 0..4 {
                    let a = polygon[i];
                    let b = polygon[(i + 1) % 4];
                    let edge = [b[0] - a[0], b[1] - a[1]];
                    let offset = [center[0] - a[0], center[1] - a[1]];
                    let cross = edge[0] * offset[1] - edge[1] * offset[0];
                    positive |= cross > 0.;
                    negative |= cross < 0.;
                    let t = ((offset[0] * edge[0] + offset[1] * edge[1])
                        / (edge[0] * edge[0] + edge[1] * edge[1]))
                        .clamp(0., 1.);
                    distance =
                        distance.min((offset[0] - t * edge[0]).hypot(offset[1] - t * edge[1]));
                }
                if positive && negative {
                    (radius - distance).max(0.)
                } else {
                    radius + distance
                }
            }
            (Self::Rectangle(a), Self::Rectangle(b)) => {
                let mut penetration = f64::INFINITY;
                for polygon in [a, b] {
                    for i in 0..4 {
                        let edge = [
                            polygon[(i + 1) % 4][0] - polygon[i][0],
                            polygon[(i + 1) % 4][1] - polygon[i][1],
                        ];
                        let norm = edge[0].hypot(edge[1]);
                        let axis = [-edge[1] / norm, edge[0] / norm];
                        let project = |points: &ramp_clearance::Polygon| {
                            points
                                .iter()
                                .map(|point| point[0] * axis[0] + point[1] * axis[1])
                                .fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], value| {
                                    [lo.min(value), hi.max(value)]
                                })
                        };
                        let pa = project(a);
                        let pb = project(b);
                        let depth = (pa[1] - pb[0]).min(pb[1] - pa[0]);
                        if depth <= 0. {
                            return 0.;
                        }
                        penetration = penetration.min(depth);
                    }
                }
                penetration
            }
        }
    }
}

struct Occupant {
    object: usize,
    role: String,
    shape: Shape,
    base_seat: Option<usize>,
}
fn obstacle_radius(kind: &str) -> Option<f64> {
    if kind == "OilDerrick" {
        Some(4.)
    } else if kind.contains("TechStructure")
        || kind.contains("ConstructionYard")
        || kind.starts_with("EI_Structure_")
        || [
            "Barracks",
            "WarFactory",
            "Airfield",
            "NavalYard",
            "PowerPlant",
            "Refinery",
            "SuperWeapon",
        ]
        .iter()
        .any(|name| kind.contains(name))
    {
        Some(5.)
    } else if ["Tree", "Bush", "GardenRock"]
        .iter()
        .any(|name| kind.contains(name))
    {
        Some(1.)
    } else {
        None
    }
}
fn occupants(
    scene: &Native,
    policy: &Policy,
    refinery: &ramp_clearance::Policy,
) -> Result<Vec<Occupant>> {
    let mut occupants = Vec::new();
    for (seat, &id) in scene.bases.iter().enumerate() {
        occupants.push(Occupant {
            object: id,
            role: format!("P{}/core", seat + 1),
            shape: Shape::Disk(scene.point(id), policy.base_radius.min(8.)),
            base_seat: Some(seat + 1),
        });
    }
    for &id in &scene.ores {
        let point = scene.point(id);
        let angle = scene.objects[id].angle as f64;
        occupants.push(Occupant {
            object: id,
            role: "ore/disk".into(),
            shape: Shape::Disk(point, policy.ore_radius),
            base_seat: None,
        });
        occupants.push(Occupant {
            object: id,
            role: "ore/refinery".into(),
            shape: Shape::Rectangle(ramp_clearance::rectangle(
                point,
                [angle.cos(), -angle.sin()],
                -refinery.refinery_back,
                refinery.refinery_forward,
                refinery.refinery_half_width,
            )),
            base_seat: None,
        });
    }
    for (id, object) in scene.objects.iter().enumerate() {
        if let Some(radius) = obstacle_radius(&object.type_name) {
            if object.position.iter().any(|v| !v.is_finite()) {
                return Err(err("spawn reservation obstacle has non-finite position"));
            }
            occupants.push(Occupant {
                object: id,
                role: "obstacle".into(),
                shape: Shape::Disk(scene.point(id), radius),
                base_seat: None,
            });
        }
    }
    Ok(occupants)
}
fn own_base_body(scene: &Native, base: &Occupant, other: &Occupant) -> bool {
    let Some(seat) = base.base_seat else {
        return false;
    };
    let object = &scene.objects[other.object];
    if !object.type_name.contains("ConstructionYard") {
        return false;
    }
    let owner = if seat <= 6 {
        format!("Player_{seat}")
    } else if seat == 7 {
        "PlyrCivilian".into()
    } else {
        "PlyrCreeps".into()
    };
    let matches = map::prop(&object.props, "originalOwner")
        .and_then(map::Value::text)
        .is_some_and(|value| value.split('/').next() == Some(owner.as_str()));
    let a = scene.point(base.object);
    let b = scene.point(other.object);
    matches && (a[0] - b[0]).hypot(a[1] - b[1]) <= 8.
}

fn check_occupancy(
    scene: &Native,
    policy: &Policy,
    refinery: &ramp_clearance::Policy,
    seats: &[Value],
    findings: &mut Vec<Value>,
) -> Result<()> {
    let occupants = occupants(scene, policy, refinery)?;
    let mut checked = BTreeSet::new();
    for row in seats {
        let seat = row["seat"].as_u64().unwrap() as usize;
        let ores = row["ore_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_u64().unwrap() as usize - 1)
            .collect::<BTreeSet<_>>();
        for (i, own) in occupants.iter().enumerate().filter(|(_, item)| {
            item.base_seat == Some(seat)
                || (item.role.starts_with("ore/") && ores.contains(&item.object))
        }) {
            for (j, other) in occupants.iter().enumerate() {
                if own.object == other.object
                    || !checked.insert((i.min(j), i.max(j)))
                    || own_base_body(scene, own, other)
                    || own_base_body(scene, other, own)
                {
                    continue;
                }
                let overlap = own.shape.overlap(&other.shape);
                if overlap <= 1e-6 {
                    continue;
                }
                let (a, b) = if i < j { (own, other) } else { (other, own) };
                let key = format!(
                    "overlap/{}/{}/{}",
                    a.role,
                    b.role,
                    scene.stamp(&[a.object, b.object])
                );
                let mut issue = finding(
                    "reservation_overlap",
                    seat,
                    scene.point(own.object),
                    key,
                    overlap,
                    vec![],
                    "complete start/ore/refinery reservation overlaps another reservation or a physical object",
                );
                issue["objects"] = json!([a.object + 1, b.object + 1]);
                issue["roles"] = json!([a.role, b.role]);
                findings.push(issue);
            }
        }
    }
    Ok(())
}

/// Preserve the same complete starting reservations during native terrain edits.
/// The two-cell halo also protects neighboring navigation/buildability updates.
pub(crate) fn protected_cells(p: &Project, m: &MapFile, layout: &Layout) -> Result<Vec<bool>> {
    if p.flag("TERRAIN_ONLY") {
        let hm = HeightMap::parse(m.get("HeightMapData")?)?;
        return Ok(vec![false; hm.width * hm.height]);
    }
    let policy = policy(p)?;
    let refinery = ramp_clearance::policy(p)?.ok_or_else(|| err("missing refinery policy"))?;
    let scene = Native::load(p, m, layout)?;
    let mut shapes = Vec::new();
    for &start in &scene.bases {
        let base = scene.point(start);
        shapes.push(Shape::Disk(base, policy.base_radius));
        for &ore in &scene.ores {
            let point = scene.point(ore);
            if (point[0] - base[0]).hypot(point[1] - base[1]) > policy.near_ore_radius {
                continue;
            }
            shapes.push(Shape::Disk(point, policy.ore_radius));
            let angle = scene.objects[ore].angle as f64;
            shapes.push(Shape::Rectangle(ramp_clearance::rectangle(
                point,
                [angle.cos(), -angle.sin()],
                -refinery.refinery_back,
                refinery.refinery_forward,
                refinery.refinery_half_width,
            )));
        }
    }
    let mut protected = vec![false; scene.z.len()];
    for shape in shapes {
        let bounds = shape.bounds();
        let x0 = (bounds[0] - 0.5).floor().max(0.) as usize;
        let y0 = (bounds[1] - 0.5).floor().max(0.) as usize;
        let x1 = (bounds[2] + 0.5)
            .ceil()
            .clamp(0., (scene.frame.width - 1) as f64) as usize;
        let y1 = (bounds[3] + 0.5)
            .ceil()
            .clamp(0., (scene.frame.height - 1) as f64) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if shape.covers([x as f64, y as f64]) {
                    for yy in y.saturating_sub(2)..=(y + 2).min(scene.frame.height - 1) {
                        for xx in x.saturating_sub(2)..=(x + 2).min(scene.frame.width - 1) {
                            protected[yy * scene.frame.width + xx] = true;
                        }
                    }
                }
            }
        }
    }
    Ok(protected)
}

fn finding(
    rule: &str,
    seat: usize,
    at: [f64; 2],
    key: String,
    measurement: f64,
    cells: Vec<[f64; 3]>,
    detail: &str,
) -> Value {
    json!({"rule":rule,"seat":seat,"at":at,"key":key,"measurement":measurement,"cells":cells,"detail":detail})
}
struct Reservation<'a> {
    scene: &'a Native,
    policy: &'a Policy,
    seat: usize,
    base: [f64; 2],
    key: &'a str,
    reachable: &'a [bool],
}
impl Reservation<'_> {
    fn check(&self, shape: Shape, role: &str, findings: &mut Vec<Value>) {
        let Self {
            scene,
            policy,
            seat,
            base,
            key,
            reachable,
        } = *self;
        let bounds = shape.bounds();
        let edge = [
            scene.frame.border as f64,
            scene.frame.border as f64,
            (scene.frame.width - scene.frame.border - 1) as f64,
            (scene.frame.height - scene.frame.border - 1) as f64,
        ];
        let outside = [
            edge[0] - bounds[0],
            edge[1] - bounds[1],
            bounds[2] - edge[2],
            bounds[3] - edge[3],
        ];
        let overflow = outside.iter().copied().fold(0., f64::max);
        if overflow > 0. {
            let cells = outside
                .iter()
                .enumerate()
                .filter(|(_, n)| **n > 0.)
                .map(|(i, n)| [i as f64, 0., *n])
                .collect();
            findings.push(finding(
                "footprint_bounds",
                seat,
                base,
                format!("{key}/{role}/bounds"),
                overflow,
                cells,
                "complete reservation crosses playable bounds",
            ));
        }
        let lo_x = (bounds[0] - 0.5).floor().max(0.) as usize;
        let lo_y = (bounds[1] - 0.5).floor().max(0.) as usize;
        let hi_x = (bounds[2] + 0.5)
            .ceil()
            .min((scene.frame.width - 1) as f64)
            .max(0.) as usize;
        let hi_y = (bounds[3] + 0.5)
            .ceil()
            .min((scene.frame.height - 1) as f64)
            .max(0.) as usize;
        let reference = scene.z[scene.index(base)];
        let water = reference < scene.water;
        let mut bad = BTreeMap::<&str, Vec<[f64; 3]>>::new();
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                let point = [x as f64, y as f64];
                if !shape.covers(point) {
                    continue;
                }
                let i = y * scene.frame.width + x;
                let mut add = |rule, value| {
                    bad.entry(rule)
                        .or_default()
                        .push([x as f64, y as f64, value])
                };
                if (scene.z[i] < scene.water) != water {
                    add("footprint_domain", 1.);
                }
                if !water && (scene.z[i] - reference).abs() > policy.flat_tolerance {
                    add("footprint_height", (scene.z[i] - reference).abs());
                }
                if scene.imp[i] {
                    add("footprint_impassable", 1.);
                }
                if scene.build[i] {
                    add("footprint_unbuildable", 1.);
                }
                if !reachable[i] {
                    add("same_surface_connection", 1.);
                }
            }
        }
        for (rule, cells) in bad {
            let measurement = if rule == "footprint_height" {
                cells.iter().map(|v| v[2]).fold(0., f64::max)
            } else {
                cells.len() as f64
            };
            let at = [cells[0][0], cells[0][1]];
            findings.push(finding(
            rule,
            seat,
                at,
            format!("{key}/{role}/{rule}"),
            measurement,
            cells,
            "complete reservation must be usable on the start's connected level or water surface",
        ));
        }
    }
}

pub fn report(p: &Project, m: &MapFile, layout: &Layout, policy: &Policy) -> Result<Value> {
    let policy = policy.clone().effective(p)?;
    let refinery =
        ramp_clearance::policy(p)?.ok_or_else(|| err("missing refinery reservation policy"))?;
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"schema_version":1,"passed":true,"raw_passed":true,"status":"terrain_only","findings":[],"regressions":[],"inherited":0,"seats":[],"policy":policy,"refinery_policy":refinery}),
        );
    }
    let scene = Native::load(p, m, layout)?;
    let center = [
        (scene.frame.width - 1) as f64 * 0.5,
        (scene.frame.height - 1) as f64 * 0.5,
    ];
    let positions = scene
        .bases
        .iter()
        .map(|id| scene.point(*id))
        .collect::<Vec<_>>();
    let matches = |axes: &[usize]| {
        positions.iter().enumerate().all(|(i, point)| {
            axes.iter().all(|&axis| {
                if i < 4 {
                    point[axis] > center[axis]
                } else {
                    point[axis] < center[axis]
                }
            })
        })
    };
    let west_east = positions.iter().enumerate().all(|(i, point)| {
        if i < 4 {
            point[0] < center[0]
        } else {
            point[0] > center[0]
        }
    });
    let mut orientations = vec![("west_east", west_east), ("south_north", matches(&[1]))];
    let southwest = positions.iter().enumerate().all(|(i, point)| {
        if i < 4 {
            point[0] < center[0] && point[1] > center[1]
        } else {
            point[0] > center[0] && point[1] < center[1]
        }
    });
    orientations.push(("southwest_northeast", southwest));
    let orientation_exception = p.cfg.get("SPAWN_ORIENTATION_EXCEPTION");
    if let Some(exception) = orientation_exception
        && (exception["layout"] != "west_east"
            || exception["reason"]
                .as_str()
                .is_none_or(|s| s.trim().is_empty()))
    {
        return Err(err(
            "SPAWN_ORIENTATION_EXCEPTION requires layout west_east and an explicit reason",
        ));
    }
    // An existing recorded west/east choice continues to require that layout.
    let accepted = orientations
        .iter()
        .filter(|(name, passed)| {
            *passed && (orientation_exception.is_none() || *name == "west_east")
        })
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    if accepted.is_empty() {
        findings.push(finding(
            "orientation",
            0,
            center,
            format!("orientation/{}", scene.stamp(&scene.bases)),
            1.,
            vec![],
            "P1-P4 must all be west/south/southwest, with P5-P8 east/north/northeast respectively",
        ));
    }
    let mut seats = Vec::new();
    for (i, &id) in scene.bases.iter().enumerate() {
        let seat = i + 1;
        let base = scene.point(id);
        let near = scene
            .ores
            .iter()
            .copied()
            .filter(|&ore| {
                let point = scene.point(ore);
                (point[0] - base[0]).hypot(point[1] - base[1]) <= policy.near_ore_radius
            })
            .collect::<Vec<_>>();
        let mut group = vec![id];
        group.extend(&near);
        let key = format!("P{seat}/{}", scene.stamp(&group));
        if near.len() != 2 {
            findings.push(finding(
                "near_ore_count",
                seat,
                base,
                format!("{key}/count"),
                near.len().abs_diff(2) as f64,
                vec![],
                "exactly two native ore nodes must be in the start's near-ore radius",
            ));
        }
        let reachable = scene.reachable(base, policy.flat_tolerance);
        let reservation = Reservation {
            scene: &scene,
            policy: &policy,
            seat,
            base,
            key: &key,
            reachable: &reachable,
        };
        reservation.check(Shape::Disk(base, policy.base_radius), "base", &mut findings);
        for &ore in &near {
            let point = scene.point(ore);
            reservation.check(
                Shape::Disk(point, policy.ore_radius),
                &format!("ore{ore}/disk"),
                &mut findings,
            );
            // Native angles use world Y north; our grid Y points south.
            let angle = scene.objects[ore].angle as f64;
            let unit = [angle.cos(), -angle.sin()];
            let polygon = ramp_clearance::rectangle(
                point,
                unit,
                -refinery.refinery_back,
                refinery.refinery_forward,
                refinery.refinery_half_width,
            );
            reservation.check(
                Shape::Rectangle(polygon),
                &format!("ore{ore}/refinery"),
                &mut findings,
            );
        }
        seats.push(json!({"seat":seat,"base":base,"start_id":id+1,"water":scene.z[scene.index(base)]<scene.water,
            "height":scene.z[scene.index(base)],"ore_ids":near.iter().map(|i|i+1).collect::<Vec<_>>(),"group_key":key}));
    }
    check_occupancy(&scene, &policy, &refinery, &seats, &mut findings)?;
    let passed = findings.is_empty();
    let mut checked = json!({"schema_version":1,"passed":passed,"raw_passed":passed,"status":"strict","findings":findings,
        "regressions":findings,"inherited":0,"seats":seats,"orientation":{"center":center,"accepted":accepted,"candidates":orientations},
        "frame":scene.frame,"policy":policy,"refinery_policy":refinery});
    if let Some(exception) = orientation_exception {
        checked["orientation"]["explicit_exception"] = exception.clone();
    }
    Ok(checked)
}

fn finish(checked: Value) -> Result<Value> {
    if checked["passed"] != true {
        let count = checked["regressions"].as_array().map_or(0, Vec::len);
        let details=checked["regressions"].as_array().into_iter().flatten().take(8).map(|v|json!({"rule":v["rule"],"seat":v["seat"],"at":v["at"],"measurement":v["measurement"]})).collect::<Vec<_>>();
        return Err(err(format!(
            "spawn placement rejected: {count} strict findings: {}",
            json!(details)
        )));
    }
    Ok(checked)
}

/// Read-only geometry diagnostic. It cannot authorize placement acceptance.
pub fn reference(p: &Project, m: &MapFile, layout: &Layout) -> Result<Value> {
    let policy = policy(p)?;
    let checked = report(p, m, layout, &policy)?;
    let scene = Native::load(p, m, layout)?;
    let selected = scene
        .objects
        .iter()
        .enumerate()
        .filter(|(id, object)| {
            scene.bases.contains(id)
                || scene.ores.contains(id)
                || obstacle_radius(&object.type_name).is_some()
        })
        .map(|(_, object)| object)
        .collect::<Vec<_>>();
    let geometry = json!({"frame":scene.frame,"heights":scene.z,"impassability":scene.imp,"buildability":scene.build,"objects":selected,"water":scene.water});
    Ok(
        json!({"schema_version":2,"map_id":p.id,"geometry_crc32":crc32fast::hash(&serde_json::to_vec(&geometry)?),
        "policy":checked["policy"],"refinery_policy":checked["refinery_policy"],"report_crc32":crc32fast::hash(&serde_json::to_vec(&checked)?)}),
    )
}

pub fn validate(p: &Project, m: &MapFile, layout: &Layout) -> Result<Value> {
    if p.cfg.get("SPAWN_PLACEMENT_REFERENCE").is_some() {
        return Err(err(
            "SPAWN_PLACEMENT_REFERENCE is no longer supported; all maps require current strict spawn placement",
        ));
    }
    finish(report(p, m, layout, &policy(p)?)?)
}

pub fn validate_transition(
    p: &Project,
    _before_map: &MapFile,
    _before_layout: &Layout,
    after_map: &MapFile,
    after_layout: &Layout,
) -> Result<Value> {
    validate(p, after_map, after_layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current() -> (Project, MapFile, Layout) {
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let m = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
        let layout = Layout::load(&p).unwrap();
        (p, m, layout)
    }

    #[test]
    fn removed_reference_option_is_rejected_even_with_an_explicit_policy() {
        let (mut p, m, layout) = current();
        p.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("old-reference.json");
        p.cfg["SPAWN_PLACEMENT"] = json!({});
        assert!(
            reference(&p, &m, &layout).is_ok(),
            "diagnostics remain read-only"
        );
        assert!(
            validate(&p, &m, &layout)
                .unwrap_err()
                .to_string()
                .contains("no longer supported")
        );
        assert!(validate_transition(&p, &m, &layout, &m, &layout).is_err());
    }

    #[test]
    fn identical_source_findings_cannot_be_inherited_by_incremental_edits() {
        let (p, m, layout) = current();
        assert!(
            validate(&p, &m, &layout).unwrap()["raw_passed"]
                .as_bool()
                .unwrap()
        );
        let frame = Frame::from_project(&p);
        let base = layout.base_cells[0];
        let mut objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
        let outside = objects
            .iter_mut()
            .find(|o| {
                let c = frame.grid([o.position[0] as f64, o.position[1] as f64]);
                o.type_name == "OreNode" && (c[0] - base[0]).hypot(c[1] - base[1]) > 100.
            })
            .unwrap();
        let point = frame.world([base[0] + 20., base[1]]);
        outside.position[0] = point[0] as f32;
        outside.position[1] = point[1] as f32;
        let mut conflicting = m.clone();
        conflicting.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &conflicting).unwrap();
        assert!(validate(&p, &conflicting, &layout).is_err());
        assert!(validate_transition(&p, &conflicting, &layout, &conflicting, &layout).is_err());
    }
}
