//! Shared native-coordinate geometry, occupancy, routing and material inventory.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::binary::Reader,
    formats::map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    pipeline::project::Project,
    terrain::{self, Layout, number},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Policy {
    pub bucket_size: f64,
    pub ore_reserve_radius: f64,
    pub building_reserve_radius: f64,
    pub oil_reserve_radius: f64,
    pub decoration_reserve_radius: f64,
    pub flat_tolerance: f64,
    pub maximum_walk_step: f64,
    pub maximum_ramp_step: f64,
    pub maximum_landing_error: f64,
    pub navigation_clearance: usize,
}

/// Resolve the same common policy for new blueprints and frozen native sources.
pub fn policy(p: &Project) -> Result<Policy> {
    let path = p.root.join("rules/coordinate-pipeline.json");
    let mut data: Json = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            serde_json::from_str(include_str!("../../rules/coordinate-pipeline.json"))?
        }
        Err(error) => return Err(error.into()),
    };
    if let Some(value) = p.cfg["COORDINATE_PIPELINE"].get("policy")
        && !value.is_null()
    {
        let name = value
            .as_str()
            .ok_or_else(|| err("spatial policy path must be a string"))?;
        let overrides: Json = serde_json::from_slice(&fs::read(p.path(name)?)?)?;
        let fields = overrides
            .as_object()
            .ok_or_else(|| err("spatial policy overrides must be an object"))?;
        let common = data
            .as_object_mut()
            .ok_or_else(|| err("common spatial policy must be an object"))?;
        common.extend(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    let policy: Policy = serde_json::from_value(data)?;
    policy.validate()?;
    Ok(policy)
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        if [
            self.bucket_size,
            self.ore_reserve_radius,
            self.building_reserve_radius,
            self.oil_reserve_radius,
            self.decoration_reserve_radius,
            self.flat_tolerance,
            self.maximum_walk_step,
            self.maximum_ramp_step,
            self.maximum_landing_error,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || self.bucket_size > 256.
            || self.navigation_clearance > 4
        {
            return Err(err("invalid spatial policy"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Issue {
    pub key: String,
    pub rule: String,
    pub ids: Vec<usize>,
    pub at: [f64; 2],
    pub metric: f64,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Entity {
    pub id: usize,
    pub kind: String,
    pub class: String,
    pub point: [f64; 2],
    pub reserve: f64,
    pub physical: f64,
    pub in_playable: bool,
}
impl Entity {
    fn stamp(&self) -> String {
        format!(
            "{}:{}@{:.6},{:.6}",
            self.id, self.kind, self.point[0], self.point[1]
        )
    }
}
#[derive(Default)]
pub struct Index {
    bins: BTreeMap<(i32, i32), Vec<usize>>,
    size: f64,
}
impl Index {
    pub fn new(entities: &[Entity], size: f64) -> Self {
        let mut index = Self {
            bins: BTreeMap::new(),
            size,
        };
        for (i, e) in entities.iter().enumerate() {
            if e.reserve == 0. {
                continue;
            }
            for y in ((e.point[1] - e.reserve) / size).floor() as i32
                ..=((e.point[1] + e.reserve) / size).floor() as i32
            {
                for x in ((e.point[0] - e.reserve) / size).floor() as i32
                    ..=((e.point[0] + e.reserve) / size).floor() as i32
                {
                    index.bins.entry((x, y)).or_default().push(i);
                }
            }
        }
        index
    }
    pub fn query(&self, point: [f64; 2], radius: f64) -> BTreeSet<usize> {
        let mut ids = BTreeSet::new();
        for y in ((point[1] - radius) / self.size).floor() as i32
            ..=((point[1] + radius) / self.size).floor() as i32
        {
            for x in ((point[0] - radius) / self.size).floor() as i32
                ..=((point[0] + radius) / self.size).floor() as i32
            {
                if let Some(bin) = self.bins.get(&(x, y)) {
                    ids.extend(bin);
                }
            }
        }
        ids
    }
}
pub struct Scene {
    pub frame: Frame,
    pub z: Vec<f64>,
    pub imp: Vec<bool>,
    pub build: Vec<bool>,
    pub entities: Vec<Entity>,
    pub index: Index,
    pub policy: Policy,
    water: f64,
}
fn ramp_geometry(r: &Json) -> Json {
    let mut geometry = r.clone();
    if let Some(fields) = geometry.as_object_mut() {
        // Cosmetic material selection cannot turn an identical, measured
        // source defect into a new geometric defect. Keep every geometry field.
        fields.remove("surface_style");
    }
    geometry
}
fn start_name(s: &str) -> Option<usize> {
    s.strip_prefix("Player_")?
        .split('_')
        .next()?
        .parse()
        .ok()
        .filter(|i| (1..=8).contains(i))
}
impl Scene {
    pub fn load(p: &Project, m: &MapFile, policy: Policy) -> Result<Self> {
        policy.validate()?;
        let hm = HeightMap::parse(m.get("HeightMapData")?)?;
        if hm.height != p.usize("GRID_HEIGHT", p.usize("GRID", 0))
            || hm.width != p.usize("GRID", 0)
            || hm.border as usize != p.usize("BORDER", 0)
        {
            return Err(err("spatial frame differs from native map"));
        }
        let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
        let frame = Frame::from_heightmap(p, &hm)?;
        let entities = map::objects(m.get("ObjectsList")?, &m.table)?
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let name = o.type_name.as_str();
                let (class, reserve, physical) = if name == "OreNode" {
                    ("ore", policy.ore_reserve_radius, 3.)
                } else if name == "OilDerrick" {
                    ("oil", policy.oil_reserve_radius, 4.)
                } else if name.contains("TechStructure")
                    || name.contains("ConstructionYard")
                    || name.starts_with("EI_Structure_")
                {
                    ("building", policy.building_reserve_radius, 5.)
                } else if ["Tree", "Bush", "GardenRock"]
                    .iter()
                    .any(|s| name.contains(s))
                {
                    ("decoration", policy.decoration_reserve_radius, 1.)
                } else {
                    ("marker", 0., 0.)
                };
                let point = frame.grid([o.position[0] as f64, o.position[1] as f64]);
                Entity {
                    id: i + 1,
                    kind: name.into(),
                    class: class.into(),
                    point,
                    reserve,
                    physical,
                    in_playable: frame.playable(point),
                }
            })
            .collect::<Vec<_>>();
        let index = Index::new(&entities, policy.bucket_size);
        Ok(Self {
            z: terrain::flip(&hm.elevations, hm.width)
                .iter()
                .map(|h| *h as f64 * frame.z_scale)
                .collect(),
            imp: terrain::flip(&bt.impassability, hm.width),
            build: terrain::flip(&bt.buildability, hm.width),
            frame,
            entities,
            index,
            policy,
            water: p.number("WATER_Z", 200.),
        })
    }
    fn idx(&self, p: [f64; 2]) -> Option<usize> {
        if !self.frame.inside(p) {
            return None;
        }
        Some(
            p[1].round().min((self.frame.height - 1) as f64) as usize * self.frame.width
                + p[0].round().min((self.frame.width - 1) as f64) as usize,
        )
    }
    fn height_at(&self, p: [f64; 2]) -> Option<f64> {
        self.idx(p)?;
        let w = self.frame.width;
        let x = p[0].floor() as usize;
        let y = p[1].floor() as usize;
        let xx = (x + 1).min(w - 1);
        let yy = (y + 1).min(self.frame.height - 1);
        let ax = p[0] - x as f64;
        let ay = p[1] - y as f64;
        Some(
            (self.z[y * w + x] * (1. - ax) + self.z[y * w + xx] * ax) * (1. - ay)
                + (self.z[yy * w + x] * (1. - ax) + self.z[yy * w + xx] * ax) * ay,
        )
    }
    fn disk(&self, p: [f64; 2], radius: f64) -> Vec<usize> {
        let w = self.frame.width;
        let h = self.frame.height;
        let mut out = Vec::new();
        for y in (p[1] - radius).floor().max(0.) as usize
            ..=((p[1] + radius).ceil().max(0.) as usize).min(h - 1)
        {
            for x in (p[0] - radius).floor().max(0.) as usize
                ..=((p[0] + radius).ceil().max(0.) as usize).min(w - 1)
            {
                if (x as f64 - p[0]).hypot(y as f64 - p[1]) <= radius {
                    out.push(y * w + x);
                }
            }
        }
        out
    }
    fn issue(rule: &str, entities: &[&Entity], metric: f64, detail: String) -> Issue {
        Issue {
            key: format!(
                "{rule}:{}",
                entities
                    .iter()
                    .map(|e| e.stamp())
                    .collect::<Vec<_>>()
                    .join("/")
            ),
            rule: rule.into(),
            ids: entities.iter().map(|e| e.id).collect(),
            at: entities[0].point,
            metric,
            detail,
        }
    }
    pub fn inspect(&self, layout: &Layout) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        for e in &self.entities {
            if !e.in_playable && e.reserve > 0. {
                issues.push(Self::issue(
                    "outside_playable",
                    &[e],
                    1.,
                    "原生对象在可玩区外；保留来源记录，不推断脚下地形".into(),
                ));
                continue;
            }
            if e.reserve == 0. {
                continue;
            }
            let disk = self.disk(e.point, e.reserve);
            if disk.is_empty() {
                continue;
            }
            if e.class == "ore" || e.class == "building" || e.class == "oil" {
                let low = disk
                    .iter()
                    .map(|i| self.z[*i])
                    .fold(f64::INFINITY, f64::min);
                let high = disk
                    .iter()
                    .map(|i| self.z[*i])
                    .fold(f64::NEG_INFINITY, f64::max);
                if high - low > self.policy.flat_tolerance {
                    issues.push(Self::issue(
                        "uneven_footprint",
                        &[e],
                        high - low,
                        format!("完整预留范围高差 {:.3}", high - low),
                    ));
                }
                if e.class == "ore" {
                    let blocked = disk
                        .iter()
                        .filter(|i| self.imp[**i] || self.build[**i])
                        .count();
                    if blocked > 0 {
                        issues.push(Self::issue(
                            "ore_build_disk",
                            &[e],
                            blocked as f64,
                            format!("矿场预留圆盘中 {blocked} 个格点阻挡"),
                        ));
                    }
                }
            }
            for j in self.index.query(e.point, e.reserve) {
                let b = &self.entities[j];
                if b.id <= e.id || !b.in_playable {
                    continue;
                }
                let overlap = e.reserve + b.reserve
                    - (e.point[0] - b.point[0]).hypot(e.point[1] - b.point[1]);
                if overlap > 0. {
                    issues.push(Self::issue(
                        "reserve_overlap",
                        &[e, b],
                        overlap,
                        format!("保守占地/预留空间重叠 {:.3} 格", overlap),
                    ));
                }
            }
            for (j, r) in layout.ramps.iter().enumerate() {
                let (ux, uy, _, _) = terrain::ramp_axes(r)?;
                let norm = ux.hypot(uy);
                let dx = e.point[0] - number(r, "x", 0.);
                let dy = e.point[1] - number(r, "y", 0.);
                let u = (dx * ux + dy * uy) / norm;
                let v = (-dx * uy + dy * ux) / norm;
                let along = number(r, "span", 35.) / 2. + number(r, "landing", 0.);
                let across = terrain::ramp_width(r)? / 2.;
                let separation = (u.abs() - along).max(0.).hypot((v.abs() - across).max(0.));
                if separation < e.reserve {
                    let mut issue = Self::issue(
                        "ramp_keepout",
                        &[e],
                        e.reserve - separation,
                        format!(
                            "侵入斜坡 {} 的坡面或端口预留范围",
                            layout.objects.len() + j + 1
                        ),
                    );
                    issue
                        .key
                        .push_str(&format!("/ramp{}:{}", j + 1, ramp_geometry(r)));
                    issue.ids.push(layout.objects.len() + j + 1);
                    issues.push(issue);
                }
            }
        }
        Ok(issues)
    }
    fn walkable(&self, water: bool) -> Vec<bool> {
        let w = self.frame.width;
        let h = self.frame.height;
        let mut open = (0..self.z.len())
            .map(|i| {
                !self.imp[i]
                    && ((self.z[i] <= self.water) == water)
                    && self.frame.playable([(i % w) as f64, (i / w) as f64])
            })
            .collect::<Vec<_>>();
        for e in &self.entities {
            if e.physical > 0. && e.in_playable {
                for i in self.disk(e.point, e.physical) {
                    open[i] = false;
                }
            }
        }
        if self.policy.navigation_clearance > 0 {
            let before = open.clone();
            let radius = self.policy.navigation_clearance;
            for i in 0..open.len() {
                if !before[i] {
                    continue;
                }
                let x = i % w;
                let y = i / w;
                open[i] = (y.saturating_sub(radius)..=(y + radius).min(h - 1)).all(|yy| {
                    (x.saturating_sub(radius)..=(x + radius).min(w - 1))
                        .all(|xx| before[yy * w + xx])
                });
            }
        }
        open
    }
    pub fn ramp_profiles(&self, layout: &Layout) -> Result<(Vec<Json>, Vec<Issue>)> {
        let mut profiles = Vec::new();
        let mut issues = Vec::new();
        for (index, r) in layout.ramps.iter().enumerate() {
            let (ux, uy, lo, hi) = terrain::ramp_axes(r)?;
            let half = number(r, "span", 35.) / 2.;
            let landing = number(r, "landing", 0.);
            let width = terrain::ramp_width(r)?;
            let center = [number(r, "x", 0.), number(r, "y", 0.)];
            let radius = half + landing;
            let mut max_step = 0f64;
            let mut endpoint_error = 0f64;
            let mut blocked = 0usize;
            let mut samples = 0usize;
            for side in 0..=width.ceil() as usize {
                let v = -width / 2. + side as f64 * width / width.ceil();
                let mut previous: Option<f64> = None;
                let length = 2. * radius;
                for step in 0..=length.ceil() as usize {
                    let u = -radius + step as f64 * length / length.ceil();
                    let point = [center[0] + ux * u - uy * v, center[1] + uy * u + ux * v];
                    samples += 1;
                    if let Some(i) = self.idx(point) {
                        blocked += usize::from(self.imp[i]);
                        let z = self.height_at(point).expect("checked native sample");
                        if let Some(before) = previous {
                            max_step = max_step.max((z - before).abs());
                        }
                        previous = Some(z);
                        if step == 0 {
                            endpoint_error = endpoint_error.max((z - lo).abs());
                        }
                        if step == length.ceil() as usize {
                            endpoint_error = endpoint_error.max((z - hi).abs());
                        }
                    } else {
                        blocked += 1;
                    }
                }
            }
            let id = layout.objects.len() + index + 1;
            let e = Entity {
                id,
                kind: "Ramp".into(),
                class: "ramp".into(),
                point: center,
                reserve: width / 2.,
                physical: 0.,
                in_playable: true,
            };
            for (rule, metric, limit) in [
                ("ramp_lane_step", max_step, self.policy.maximum_ramp_step),
                (
                    "ramp_landing",
                    endpoint_error,
                    self.policy.maximum_landing_error,
                ),
                ("ramp_blocked_samples", blocked as f64, 0.),
            ] {
                if metric > limit {
                    let mut issue = Self::issue(
                        rule,
                        &[&e],
                        metric,
                        format!("全宽采样：{metric:.3}，阈值 {limit:.3}"),
                    );
                    issue.key.push_str(&format!("/{}", ramp_geometry(r)));
                    issues.push(issue);
                }
            }
            profiles.push(json!({"id":id,"center":center,"pair":[lo,hi],"width":width,"samples":samples,"blocked_samples":blocked,"maximum_step":max_step,"endpoint_error":endpoint_error}));
        }
        Ok((profiles, issues))
    }
    fn distances(&self, start: usize, open: &[bool]) -> Vec<u32> {
        let mut dist = vec![u32::MAX; open.len()];
        let mut q = VecDeque::from([start]);
        dist[start] = 0;
        while let Some(i) = q.pop_front() {
            for j in crate::terrain::grid::neighbors_rect(i, self.frame.width, self.frame.height) {
                if open[j]
                    && dist[j] == u32::MAX
                    && (self.z[i] - self.z[j]).abs() <= self.policy.maximum_walk_step
                {
                    dist[j] = dist[i] + 1;
                    q.push_back(j);
                }
            }
        }
        dist
    }
    fn access(&self, p: [f64; 2], radius: f64, open: &[bool]) -> Vec<usize> {
        let Some(i) = self.idx(p) else {
            return Vec::new();
        };
        let z = self.z[i];
        self.disk(p, radius)
            .into_iter()
            .filter(|i| open[*i] && (self.z[*i] - z).abs() <= 3.)
            .collect()
    }
    pub fn routes(&self, m: &MapFile) -> Result<(Vec<Json>, Vec<Issue>)> {
        let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
        let mut starts = BTreeMap::new();
        for (i, o) in objects.iter().enumerate() {
            if let Some(name) = prop(&o.props, "waypointName").and_then(Value::text)
                && (name.ends_with("_Start") || name.ends_with("_Start (imported)"))
                && let Some(seat) = start_name(name)
            {
                starts.insert(seat, i);
            }
        }
        if starts.len() != 8 {
            return Err(err("spatial routing requires eight actual start waypoints"));
        }
        let land_open = self.walkable(false);
        let water_open = self.walkable(true);
        let mut rows = Vec::new();
        let mut issues = Vec::new();
        for (seat, source) in starts {
            let e = &self.entities[source];
            let source_water = self.idx(e.point).is_some_and(|i| self.z[i] <= self.water);
            let open = if source_water {
                &water_open
            } else {
                &land_open
            };
            let candidates = self.access(e.point, 12., open);
            let start = candidates.into_iter().min_by(|a, b| {
                let d = |i: usize| {
                    ((i % self.frame.width) as f64 - e.point[0])
                        .hypot((i / self.frame.width) as f64 - e.point[1])
                };
                d(*a).total_cmp(&d(*b)).then(a.cmp(b))
            });
            let dist = start.map(|i| self.distances(i, open));
            for target in self.entities.iter().filter(|t| {
                t.in_playable
                    && matches!(t.class.as_str(), "ore" | "oil" | "building")
                    && !t.kind.contains("ConstructionYard")
                    && !t.kind.starts_with("EI_")
            }) {
                let target_water = self
                    .idx(target.point)
                    .is_some_and(|i| self.z[i] <= self.water);
                let mixed = source_water != target_water;
                let domain = if mixed {
                    "mixed"
                } else if source_water {
                    "water"
                } else {
                    "land"
                };
                let access = self.access(
                    target.point,
                    target.physical + self.policy.navigation_clearance as f64 + 3.,
                    open,
                );
                let best = dist.as_ref().filter(|_| !mixed).and_then(|d| {
                    access
                        .iter()
                        .copied()
                        .filter(|i| d[*i] != u32::MAX)
                        .min_by_key(|i| d[*i])
                });
                let steps = best.and_then(|i| dist.as_ref().map(|d| d[i]));
                rows.push(json!({"seat":seat,"start_id":e.id,"target_id":target.id,"target_type":target.kind,"target":target.point,"domain":domain,"route_status":if mixed {"requires_transport"} else if best.is_some(){"reachable"}else{"unreachable"},"reachable":if mixed {None}else{Some(best.is_some())},"path_steps":steps,"world_distance":steps.map(|v|v as f64*self.frame.cell_size),"straight_distance_world":(e.point[0]-target.point[0]).hypot(e.point[1]-target.point[1])*self.frame.cell_size,"start_access":start.map(|i|[i%self.frame.width,i/self.frame.width]),"target_access":best.map(|i|[i%self.frame.width,i/self.frame.width])}));
                if !mixed && best.is_none() {
                    let mut issue = Self::issue(
                        "resource_unreachable",
                        &[e, target],
                        1.,
                        format!("P{seat} 到资源 {} 未找到静态 {} 路线", target.id, domain),
                    );
                    issue.key.push_str(&format!("/P{seat}"));
                    issues.push(issue);
                }
            }
        }
        Ok((rows, issues))
    }
    pub fn near_ores(&self, p: &Project, layout: &Layout) -> Result<(Vec<Json>, Vec<Issue>)> {
        let limit = crate::terrain::spawn_placement::policy(p)?.near_ore_radius;
        let expected = 2;
        let mut rows = Vec::new();
        let mut issues = Vec::new();
        for (seat, base) in layout.base_cells.iter().enumerate() {
            let start = self
                .entities
                .iter()
                .find(|e| {
                    e.kind.contains("Waypoint")
                        && (e.point[0] - base[0]).hypot(e.point[1] - base[1]) < 0.001
                })
                .ok_or_else(|| err("near-ore quota has no native start"))?;
            let ores = self
                .entities
                .iter()
                .filter(|e| {
                    e.class == "ore" && (e.point[0] - base[0]).hypot(e.point[1] - base[1]) <= limit
                })
                .collect::<Vec<_>>();
            rows.push(json!({"seat":seat+1,"base":base,"expected":expected,"radius":limit,"ore_ids":ores.iter().map(|e|e.id).collect::<Vec<_>>() }));
            if ores.len() != expected {
                let mut issue = Self::issue(
                    "near_ore_quota",
                    &[start],
                    ores.len().abs_diff(expected) as f64,
                    format!(
                        "P{} 的 {:.1} 格范围内有 {} 矿，要求 {}",
                        seat + 1,
                        limit,
                        ores.len(),
                        expected
                    ),
                );
                issue.key.push_str(&format!("/P{}", seat + 1));
                issues.push(issue);
            }
        }
        Ok((rows, issues))
    }
}
pub fn regressions(issues: &[Issue], baseline: &[Issue]) -> Vec<Issue> {
    let old = baseline
        .iter()
        .map(|i| (i.key.as_str(), i.metric))
        .collect::<BTreeMap<_, _>>();
    issues
        .iter()
        .filter(|i| old.get(i.key.as_str()).is_none_or(|v| i.metric > *v + 1e-6))
        .cloned()
        .collect()
}

/// Export all four native material arrays; descriptor bytes retain their original indexes.
pub fn materials(p: &Project, m: &MapFile) -> Result<Json> {
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let chunk = m.get("BlendTileData")?;
    let bt = BlendTiles::parse(chunk, hm.width, hm.height)?;
    let mut reader = Reader::new(&chunk.payload);
    let count = reader.u32()? as usize;
    let tile = (0..count)
        .map(|_| reader.u16().map(u32::from))
        .collect::<Result<Vec<_>>>()?;
    let size = if (14..24).contains(&chunk.version) {
        4
    } else {
        2
    };
    let mut arrays = vec![tile];
    for _ in 0..3 {
        arrays.push(
            (0..count)
                .map(|_| {
                    if size == 2 {
                        reader.u16().map(u32::from)
                    } else {
                        reader.u32()
                    }
                })
                .collect::<Result<Vec<_>>>()?,
        );
    }
    for a in &mut arrays {
        *a = terrain::flip(a, hm.width);
    }
    let mut rows = Vec::new();
    let mut histogram = BTreeMap::<String, usize>::new();
    for y in 0..hm.height {
        let mut runs = Vec::new();
        let mut x = 0;
        while x < hm.width {
            let start = x;
            let i = y * hm.width + x;
            let values = [arrays[0][i], arrays[1][i], arrays[2][i], arrays[3][i]];
            x += 1;
            while x < hm.width && (0..4).all(|k| arrays[k][y * hm.width + x] == values[k]) {
                x += 1;
            }
            runs.push([
                start as u32,
                (x - 1) as u32,
                values[0],
                values[1],
                values[2],
                values[3],
            ]);
            let name = bt
                .textures
                .get(values[0] as usize / 64)
                .ok_or_else(|| err("invalid spatial texture index"))?;
            *histogram.entry(name.clone()).or_default() += x - start;
        }
        rows.push(runs);
    }
    let data = json!({"schema_version":1,"width":hm.width,"height":hm.height,"origin":"northwest","fields":["x_begin","x_end_inclusive","tile_id","blend_index","three_way_blend_index","cliff_texture_index"],"textures":bt.textures,"tile_encoding":"texture_index=tile_id/64; sub_tile=tile_id%64","blend_descriptions_hex":crate::gameplay::scripts::hex(&bt.blend_descriptions),"rows":rows,"histogram":histogram});
    let dir = p.assets.join("spatial");
    fs::create_dir_all(&dir)?;
    crate::pipeline::project::atomic_write(
        &dir.join("texture-coordinates.json"),
        &serde_json::to_vec(&data)?,
    )?;
    let mut csv = String::from(
        "\u{feff}y,x_begin,x_end_inclusive,tile_id,texture_index,texture_name,blend_index,three_way_blend_index,cliff_texture_index\n",
    );
    for (y, row) in rows.iter().enumerate() {
        for r in row {
            csv.push_str(&format!(
                "{y},{},{},{},{},\"{}\",{},{},{}\n",
                r[0],
                r[1],
                r[2],
                r[2] / 64,
                bt.textures[r[2] as usize / 64].replace('"', "\"\""),
                r[3],
                r[4],
                r[5]
            ));
        }
    }
    fs::write(dir.join("texture-coordinates.csv"), csv)?;
    Ok(
        json!({"grid_points":count,"material_runs":rows.iter().map(Vec::len).sum::<usize>(),"histogram":histogram}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_policy_loader_handles_blueprints_overrides_and_invalid_types() {
        use std::path::PathBuf;
        let root = std::env::temp_dir().join(format!("ra3-spatial-policy-{}", std::process::id()));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let mut p = Project {
            root,
            id: "policy-test".into(),
            cfg: json!({}),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "policy-test".into(),
            name: "policy-testT0.0.1".into(),
            assets: PathBuf::new(),
            dist: PathBuf::new(),
        };
        let defaults = policy(&p).unwrap();
        assert_eq!(defaults.navigation_clearance, 1);
        fs::create_dir_all(p.map_dir()).unwrap();
        p.cfg["COORDINATE_PIPELINE"] = json!({"policy":"policy.json"});
        fs::write(
            p.map_dir().join("policy.json"),
            br#"{"navigation_clearance":2,"maximum_walk_step":4.5}"#,
        )
        .unwrap();
        let selected = policy(&p).unwrap();
        assert_eq!(selected.navigation_clearance, 2);
        assert_eq!(selected.maximum_walk_step, 4.5);
        assert_eq!(selected.ore_reserve_radius, defaults.ore_reserve_radius);
        for invalid in [
            br#"{"navigation_clearance":"two"}"#.as_slice(),
            b"[]",
            br#"{"maximum_walk_step":-1}"#,
        ] {
            fs::write(p.map_dir().join("policy.json"), invalid).unwrap();
            assert!(policy(&p).is_err());
        }
        p.cfg["COORDINATE_PIPELINE"]["policy"] = json!(false);
        assert!(policy(&p).is_err());
    }
    fn scene(n: usize) -> Scene {
        let policy = Policy {
            bucket_size: 4.,
            ore_reserve_radius: 3.,
            building_reserve_radius: 2.,
            oil_reserve_radius: 2.,
            decoration_reserve_radius: 1.,
            flat_tolerance: 12.,
            maximum_walk_step: 12.,
            maximum_ramp_step: 6.1,
            maximum_landing_error: 12.,
            navigation_clearance: 0,
        };
        Scene {
            frame: Frame {
                width: n,
                height: n,
                border: 0,
                cell_size: 10.,
                z_scale: 1.,
            },
            z: vec![212.; n * n],
            imp: vec![false; n * n],
            build: vec![false; n * n],
            entities: Vec::new(),
            index: Index::default(),
            policy,
            water: 200.,
        }
    }
    #[test]
    fn near_ore_audit_uses_the_spawn_policy_override_and_always_requires_two() {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        p.cfg["TERRAIN"]["ore_near_max_r"] = json!(62.);
        p.cfg["TERRAIN"]["base_min_ore"] = json!(1);
        p.cfg["SPAWN_PLACEMENT"] = json!({"near_ore_radius":75.});
        let mut s = scene(200);
        for (id, class, point) in [
            (1, "waypoint", [100., 100.]),
            (2, "ore", [150., 100.]),
            (3, "ore", [170., 100.]),
        ] {
            s.entities.push(Entity {
                id,
                kind: if class == "ore" {
                    "OreNode"
                } else {
                    "Waypoint"
                }
                .into(),
                class: class.into(),
                point,
                reserve: 1.,
                physical: 1.,
                in_playable: true,
            });
        }
        let layout: Layout = serde_json::from_value(json!({
            "schema_version":1,"base_cells":[[100.,100.]],"ore_cells":[],
            "ore_angles":[],"objects":[]
        }))
        .unwrap();
        let (rows, issues) = s.near_ores(&p, &layout).unwrap();
        assert!(issues.is_empty());
        assert_eq!(rows[0]["radius"], 75.);
        assert_eq!(rows[0]["expected"], 2);
        assert_eq!(rows[0]["ore_ids"], json!([2, 3]));

        s.entities.pop();
        let (_, issues) = s.near_ores(&p, &layout).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].rule, "near_ore_quota");
    }
    #[test]
    fn spatial_buckets_preserve_intersections_across_negative_and_bucket_edges() {
        let entities = (0..40)
            .map(|id| Entity {
                id,
                kind: "test".into(),
                class: "building".into(),
                point: [(id % 8) as f64 * 3. - 4., (id / 8) as f64 * 7. - 3.],
                reserve: 1. + (id % 4) as f64,
                physical: 1.,
                in_playable: true,
            })
            .collect::<Vec<_>>();
        let index = Index::new(&entities, 4.);
        for e in &entities {
            let candidates = index.query(e.point, e.reserve);
            for (j, b) in entities.iter().enumerate() {
                if (e.point[0] - b.point[0]).hypot(e.point[1] - b.point[1]) < e.reserve + b.reserve
                {
                    assert!(candidates.contains(&j));
                }
            }
        }
    }
    #[test]
    fn routing_uses_real_opening_and_rejects_a_closed_wall() {
        let mut s = scene(9);
        for y in 0..8 {
            s.imp[y * 9 + 4] = true;
        }
        let open = s.walkable(false);
        let d = s.distances(2 * 9 + 2, &open);
        assert_eq!(d[2 * 9 + 6], 16);
        assert!(d[2 * 9 + 6] > 4);
        s.imp[8 * 9 + 4] = true;
        let d = s.distances(2 * 9 + 2, &s.walkable(false));
        assert_eq!(d[2 * 9 + 6], u32::MAX);
    }
    #[test]
    fn rectangular_routes_reach_the_last_row_without_wrapping_or_leaving_the_grid() {
        for (width, height) in [(5, 11), (11, 5)] {
            let mut s = scene(1);
            s.frame.width = width;
            s.frame.height = height;
            s.z = vec![212.; width * height];
            s.imp = vec![false; width * height];
            s.build = vec![false; width * height];
            for y in 0..height - 1 {
                s.imp[y * width + width / 2] = true;
            }
            let distances = s.distances(0, &s.walkable(false));
            assert_eq!(
                distances[width - 1],
                (2 * (height - 1) + width - 1) as u32,
                "{width}x{height}: the only opening is in the final row"
            );
            s.imp[(height - 1) * width + width / 2] = true;
            assert_eq!(s.distances(0, &s.walkable(false))[width - 1], u32::MAX);
        }
    }
    #[test]
    fn rectangular_clearance_checks_the_full_height_for_tall_and_wide_maps() {
        for (width, height) in [(5, 11), (11, 5)] {
            let mut s = scene(1);
            s.frame.width = width;
            s.frame.height = height;
            s.z = vec![212.; width * height];
            s.imp = vec![false; width * height];
            s.build = vec![false; width * height];
            s.policy.navigation_clearance = 1;
            s.imp[(height - 2) * width + 2] = true;
            let open = s.walkable(false);
            assert!(!open[(height - 1) * width + 2]);
            assert!(!open[(height - 3) * width + 2]);
            assert!(open[width - 1]);
        }
    }
    #[test]
    fn routing_does_not_cross_unflagged_height_cliffs_or_cut_diagonal_corners() {
        let mut s = scene(7);
        for y in 0..7 {
            for x in 4..7 {
                s.z[y * 7 + x] = 280.;
            }
        }
        let d = s.distances(3 * 7 + 2, &s.walkable(false));
        assert_eq!(d[3 * 7 + 5], u32::MAX);
        let mut s = scene(3);
        s.imp[1] = true;
        s.imp[3] = true;
        let d = s.distances(0, &s.walkable(false));
        assert_eq!(d[4], u32::MAX);
    }
    #[test]
    fn full_resource_reservation_detects_a_cliff_away_from_its_center() {
        let mut s = scene(20);
        s.entities.push(Entity {
            id: 1,
            kind: "OreNode".into(),
            class: "ore".into(),
            point: [10., 10.],
            reserve: 3.,
            physical: 1.,
            in_playable: true,
        });
        s.index = Index::new(&s.entities, 4.);
        s.z[10 * 20 + 13] = 280.;
        s.build[11 * 20 + 10] = true;
        let layout: Layout = serde_json::from_value(
            json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[]}),
        )
        .unwrap();
        let issues = s.inspect(&layout).unwrap();
        assert!(issues.iter().any(|i| i.rule == "uneven_footprint"));
        assert!(issues.iter().any(|i| i.rule == "ore_build_disk"));
    }
    #[test]
    fn inherited_issues_never_exempt_new_coordinates_or_greater_overlap() {
        let e = Entity {
            id: 3,
            kind: "Tree".into(),
            class: "decoration".into(),
            point: [10., 10.],
            reserve: 2.,
            physical: 1.,
            in_playable: true,
        };
        let old = Scene::issue("reserve_overlap", &[&e], 1., "test".into());
        assert!(regressions(std::slice::from_ref(&old), std::slice::from_ref(&old)).is_empty());
        let mut worse = old.clone();
        worse.metric = 2.;
        assert_eq!(regressions(&[worse], std::slice::from_ref(&old)).len(), 1);
        let mut moved = e;
        moved.point = [11., 10.];
        let fresh = Scene::issue("reserve_overlap", &[&moved], 0.5, "test".into());
        assert_eq!(regressions(&[fresh], &[old]).len(), 1);
    }
    #[test]
    fn ramp_materials_preserve_issue_identity_but_geometry_and_worsening_do_not() {
        let mut s = scene(60);
        let mut layout: Layout = serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[],"ramps":[{"x":30.,"y":30.,"ux":1.,"uy":0.,"width":20.,"span":25.,"pair":[210.,280.]}]})).unwrap();
        let before = s.ramp_profiles(&layout).unwrap().1;
        assert!(!before.is_empty());
        layout.ramps[0]["surface_style"] = json!("mountain");
        let painted = s.ramp_profiles(&layout).unwrap().1;
        assert!(regressions(&painted, &before).is_empty());
        s.z[30 * 60 + 30] += 200.;
        assert!(!regressions(&s.ramp_profiles(&layout).unwrap().1, &before).is_empty());
        layout.ramps[0]["x"] = json!(31.);
        assert!(!regressions(&s.ramp_profiles(&layout).unwrap().1, &before).is_empty());
    }
    #[test]
    fn ramp_validation_covers_off_center_lanes() {
        let mut s = scene(60);
        for y in 0..60 {
            for x in 0..60 {
                s.z[y * 60 + x] = 212. + 68. * ((x as f64 - 15.) / 30.).clamp(0., 1.);
            }
        }
        let layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[],"ramps":[{"x":30.,"y":30.,"ux":1.,"uy":0.,"width":20.,"span":30.,"pair":[212.,280.]}]})).unwrap();
        assert!(s.ramp_profiles(&layout).unwrap().1.is_empty());
        s.z[34 * 60 + 30] += 100.;
        assert!(
            s.ramp_profiles(&layout)
                .unwrap()
                .1
                .iter()
                .any(|i| i.rule == "ramp_lane_step")
        );
    }
}
