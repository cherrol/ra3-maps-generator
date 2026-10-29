//! Diagnostic unit/state-specific static routes. This is not the engine pathfinder.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::{
        binary::Reader,
        map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    },
    gameplay::official_placement::{collision_polygons, world_offset},
    pipeline::project::Project,
    resources::official,
    terrain::{self, grid::neighbors_rect, spatial},
};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const REVISION: &str = "unit-state-static-routes-v1";
const REPRESENTATIVES: [&str; 4] = [
    "AlliedAntiVehicleVehicleTech1",
    "AlliedAntiNavalScout",
    "AlliedMCV",
    "AlliedMiner",
];
const START_ACCESS_RADIUS: f64 = 12.;

pub fn binding(p: &Project) -> Result<Json> {
    let policy = spatial::policy(p)?;
    Ok(json!({"algorithm":REVISION,"catalog":official::binding(),
        "water_depths":official::water_depths(),"representative_units":REPRESENTATIVES,
        "maximum_walk_step":policy.maximum_walk_step,
        "navigation_clearance":policy.navigation_clearance,
        "start_access_radius_cells":START_ACCESS_RADIUS,
        "collision_model":"conservative square around official collision bounding circle"}))
}

#[derive(Debug)]
struct WaterArea {
    name: String,
    level: f64,
    polygon: Vec<[f64; 2]>,
}

fn water_areas(map: &MapFile) -> Result<Vec<WaterArea>> {
    let chunk = map.get("StandingWaterAreas")?;
    if chunk.version != 2 {
        return Err(err("unsupported StandingWaterAreas version"));
    }
    let mut reader = Reader::new(&chunk.payload);
    let count = reader.u32()? as usize;
    if count > 4096 {
        return Err(err("too many standing water areas"));
    }
    let mut areas = Vec::new();
    for _ in 0..count {
        let _id = reader.u32()?;
        let name = reader.s16()?;
        let _unknown = reader.s16()?;
        let _scroll = reader.f32()?;
        let _additive = reader.u8()?;
        let _bump = reader.s16()?;
        let _sky = reader.s16()?;
        let points = reader.u32()? as usize;
        if !(3..=4096).contains(&points) {
            return Err(err("invalid standing-water polygon"));
        }
        let mut polygon = Vec::with_capacity(points);
        for _ in 0..points {
            let point = [reader.f32()? as f64, reader.f32()? as f64];
            if point.iter().any(|v| !v.is_finite()) {
                return Err(err("nonfinite water polygon"));
            }
            polygon.push(point);
        }
        let level = reader.u32()? as f64;
        let _fx = reader.s16()?;
        let _depth_colors = reader.s16()?;
        areas.push(WaterArea {
            name,
            level,
            polygon,
        });
    }
    reader.finish()?;
    Ok(areas)
}

fn inside(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let mut result = false;
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        let cross = (point[0] - a[0]) * (b[1] - a[1]) - (point[1] - a[1]) * (b[0] - a[0]);
        if cross.abs() <= 1e-7
            && (point[0] - a[0]) * (point[0] - b[0]) <= 1e-7
            && (point[1] - a[1]) * (point[1] - b[1]) <= 1e-7
        {
            return true;
        }
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            result = !result;
        }
    }
    result
}

struct NativeGrid {
    frame: Frame,
    z: Vec<f64>,
    water: Vec<Option<f64>>,
    imp: Vec<bool>,
}
impl NativeGrid {
    fn surface_allowed(&self, i: usize, surfaces: &[String], wade: f64, deep: f64) -> bool {
        let has = |name| surfaces.iter().any(|s| s == name);
        let depth = self.water[i].map_or(0., |level| (level - self.z[i]).max(0.));
        if depth <= wade && has("GROUND") {
            return true;
        }
        if depth <= 0. {
            return false;
        }
        if depth >= deep {
            has("DEEP_WATER")
        } else {
            has("WATER")
        }
    }
    fn navigation_z(&self, i: usize) -> f64 {
        self.water[i].map_or(self.z[i], |level| level.max(self.z[i]))
    }
    fn index(&self, point: [f64; 2]) -> Option<usize> {
        if !self.frame.inside(point) {
            return None;
        }
        Some(
            point[1].round().min((self.frame.height - 1) as f64) as usize * self.frame.width
                + point[0].round().min((self.frame.width - 1) as f64) as usize,
        )
    }
}

#[derive(Clone)]
struct Obstacle {
    index: usize,
    cell: [f64; 2],
    radius: f64,
}

fn unscaled(object: &MapObject) -> bool {
    match prop(&object.props, "objectPrototypeScale") {
        None => true,
        Some(Value::Float(value)) => *value == 1.,
        Some(Value::Int(value)) => *value == 1,
        _ => false,
    }
}

fn radius(kind: &str, frame: &Frame, cell: [f64; 2], heading: f32) -> Result<f64> {
    let polygons = collision_polygons(kind, frame, cell, heading)?;
    if polygons.is_empty() {
        return Err(err("no supported collision shape"));
    }
    Ok(polygons
        .iter()
        .flatten()
        .map(|p| (p[0] - cell[0]).hypot(p[1] - cell[1]))
        .fold(0., f64::max))
}

fn obstacles(objects: &[MapObject], frame: &Frame) -> (Vec<Obstacle>, BTreeSet<String>) {
    let mut result = Vec::new();
    let mut pending = BTreeSet::new();
    for (index, object) in objects.iter().enumerate() {
        let definition = official::object(&object.type_name);
        let static_object = definition.is_some_and(|d| {
            d.kind_of
                .iter()
                .any(|k| matches!(k.as_str(), "STRUCTURE" | "IMMOBILE" | "TREE" | "SHRUBBERY"))
        }) || matches!(object.type_name.as_str(), "OreNode" | "OilDerrick")
            || ["TechStructure", "Tree", "Bush", "GardenRock"]
                .iter()
                .any(|s| object.type_name.contains(s));
        if !static_object || object.type_name.contains("Waypoint") {
            continue;
        }
        if !unscaled(object) {
            pending.insert(format!(
                "{}: unsupported objectPrototypeScale",
                object.type_name
            ));
            continue;
        }
        let cell = frame.grid([object.position[0] as f64, object.position[1] as f64]);
        match radius(&object.type_name, frame, cell, object.angle) {
            Ok(radius) => result.push(Obstacle {
                index,
                cell,
                radius,
            }),
            Err(_) => {
                pending.insert(object.type_name.clone());
            }
        }
    }
    (result, pending)
}

fn disk(point: [f64; 2], radius: f64, frame: &Frame) -> Vec<usize> {
    let mut cells = Vec::new();
    for y in (point[1] - radius).floor().max(0.) as usize
        ..=((point[1] + radius).ceil().max(0.) as usize).min(frame.height - 1)
    {
        for x in (point[0] - radius).floor().max(0.) as usize
            ..=((point[0] + radius).ceil().max(0.) as usize).min(frame.width - 1)
        {
            if (x as f64 - point[0]).hypot(y as f64 - point[1]) <= radius {
                cells.push(y * frame.width + x);
            }
        }
    }
    cells
}

struct Graph {
    open: Vec<bool>,
    radius: usize,
    surfaces: Vec<String>,
}

struct Components {
    labels: Vec<usize>,
    sizes: Vec<usize>,
}
impl Components {
    fn reachable_cells(&self, index: usize) -> usize {
        self.sizes[self.labels[index]]
    }
}

impl Graph {
    fn new(
        grid: &NativeGrid,
        surfaces: &[String],
        radius: usize,
        obstacles: &[Obstacle],
        wade: f64,
        deep: f64,
    ) -> Self {
        let mut blocked = (0..grid.z.len())
            .map(|i| {
                grid.imp[i]
                    || !grid.surface_allowed(i, surfaces, wade, deep)
                    || !grid
                        .frame
                        .playable([(i % grid.frame.width) as f64, (i / grid.frame.width) as f64])
            })
            .collect::<Vec<_>>();
        for obstacle in obstacles {
            for i in disk(obstacle.cell, obstacle.radius, &grid.frame) {
                blocked[i] = true;
            }
        }
        let blocked = terrain::surface::dilate(&blocked, grid.frame.width, radius);
        let open = blocked
            .iter()
            .enumerate()
            .map(|(i, blocked)| {
                let (x, y) = (i % grid.frame.width, i / grid.frame.width);
                !blocked
                    && x >= grid.frame.border + radius
                    && y >= grid.frame.border + radius
                    && x + grid.frame.border + radius < grid.frame.width
                    && y + grid.frame.border + radius < grid.frame.height
            })
            .collect();
        Self {
            open,
            radius,
            surfaces: surfaces.to_vec(),
        }
    }
    fn can_step(&self, grid: &NativeGrid, from: usize, to: usize, maximum_step: f64) -> bool {
        self.open[from]
            && self.open[to]
            && (grid.navigation_z(from) - grid.navigation_z(to)).abs() <= maximum_step
    }

    fn components(&self, grid: &NativeGrid, maximum_step: f64) -> Components {
        let mut labels = vec![0; self.open.len()];
        let mut sizes = vec![0];
        let mut queue = VecDeque::new();
        for start in 0..self.open.len() {
            if !self.open[start] || labels[start] != 0 {
                continue;
            }
            let label = sizes.len();
            let mut size = 0;
            labels[start] = label;
            queue.push_back(start);
            while let Some(i) = queue.pop_front() {
                size += 1;
                for j in neighbors_rect(i, grid.frame.width, grid.frame.height) {
                    if labels[j] == 0 && self.can_step(grid, i, j, maximum_step) {
                        labels[j] = label;
                        queue.push_back(j);
                    }
                }
            }
            sizes.push(size);
        }
        Components { labels, sizes }
    }

    fn distances(&self, grid: &NativeGrid, start: usize, maximum_step: f64) -> Vec<u32> {
        let mut distances = vec![u32::MAX; self.open.len()];
        if !self.open[start] {
            return distances;
        }
        distances[start] = 0;
        let mut queue = VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            for j in neighbors_rect(i, grid.frame.width, grid.frame.height) {
                if distances[j] == u32::MAX && self.can_step(grid, i, j, maximum_step) {
                    distances[j] = distances[i] + 1;
                    queue.push_back(j);
                }
            }
        }
        distances
    }
    fn access(&self, grid: &NativeGrid, point: [f64; 2], radius: f64) -> Option<usize> {
        disk(point, radius, &grid.frame)
            .into_iter()
            .filter(|&i| self.open[i])
            .min_by(|&a, &b| {
                let distance = |i: usize| {
                    ((i % grid.frame.width) as f64 - point[0])
                        .hypot((i / grid.frame.width) as f64 - point[1])
                };
                distance(a).total_cmp(&distance(b)).then(a.cmp(&b))
            })
    }
}

fn starts(objects: &[MapObject], frame: &Frame) -> Vec<(usize, [f64; 2])> {
    objects
        .iter()
        .filter_map(|object| {
            let name = prop(&object.props, "waypointName").and_then(Value::text)?;
            if !(name.ends_with("_Start") || name.ends_with("_Start (imported)")) {
                return None;
            }
            let seat = name
                .strip_prefix("Player_")?
                .split('_')
                .next()?
                .parse::<usize>()
                .ok()?;
            Some((
                seat,
                frame.grid([object.position[0] as f64, object.position[1] as f64]),
            ))
        })
        .collect()
}

struct ExitProbe {
    object: MapObject,
    native_index: Option<usize>,
    waypoint: Option<String>,
}

fn factory_probes(
    p: &Project,
    map: &MapFile,
    objects: &[MapObject],
    frame: &Frame,
) -> (Vec<ExitProbe>, Vec<String>) {
    let mut probes = objects
        .iter()
        .enumerate()
        .filter(|(_, object)| {
            official::object(&object.type_name).is_some_and(|definition| {
                definition
                    .production_exit
                    .as_ref()
                    .is_some_and(|exit| !exit.no_exit_path)
            })
        })
        .map(|(index, object)| ExitProbe {
            object: object.clone(),
            native_index: Some(index),
            waypoint: None,
        })
        .collect::<Vec<_>>();
    let mut pending = Vec::new();
    match crate::gameplay::official_placement::scripted_factories(p, map) {
        Ok(factories) => {
            for (kind, cell, heading, waypoint) in factories {
                let world = frame.world(cell);
                probes.push(ExitProbe {
                    object: MapObject {
                        version: 3,
                        position: [world[0] as f32, world[1] as f32, 0.],
                        angle: heading,
                        road_type: 0,
                        type_name: kind,
                        props: vec![],
                    },
                    native_index: None,
                    waypoint: Some(waypoint),
                });
            }
        }
        Err(error) => pending.push(error.to_string()),
    }
    (probes, pending)
}

fn exit_rows(
    grid: &NativeGrid,
    graph: &Graph,
    probes: &[ExitProbe],
    obstacles: &[Obstacle],
    maximum_step: f64,
    wade: f64,
    deep: f64,
) -> Vec<Json> {
    if probes.is_empty() {
        return Vec::new();
    }
    // The movement graph is undirected. All rally-point reachability counts
    // share one component inventory instead of flooding the map per factory.
    let components = graph.components(grid, maximum_step);
    let mut rows = Vec::new();
    for probe in probes {
        let object = &probe.object;
        let object_id = probe.native_index.map(|index| index + 1);
        let Some(definition) = official::object(&object.type_name) else {
            continue;
        };
        let Some(exit) = &definition.production_exit else {
            continue;
        };
        if !unscaled(object) {
            rows.push(
                json!({"object_id":object_id,"waypoint":probe.waypoint,"object_type":object.type_name,"status":"unsupported",
                "reason":"objectPrototypeScale collision/exit semantics are not verified"}),
            );
            continue;
        }
        if exit.no_exit_path {
            rows.push(json!({"object_id":object_id,"waypoint":probe.waypoint,"object_type":object.type_name,"status":"not_applicable","reason":"official NoExitPath"}));
            continue;
        }
        let center = grid
            .frame
            .grid([object.position[0] as f64, object.position[1] as f64]);
        if let Err(error) = radius(&object.type_name, &grid.frame, center, object.angle) {
            rows.push(
                json!({"object_id":object_id,"waypoint":probe.waypoint,"object_type":object.type_name,"status":"unsupported",
                "reason":error.to_string()}),
            );
            continue;
        }
        let create = world_offset(&grid.frame, center, object.angle, exit.create);
        let rally = world_offset(&grid.frame, center, object.angle, exit.rally);
        let steps = ((create[0] - rally[0]).hypot(create[1] - rally[1]) * 2.)
            .ceil()
            .max(1.) as usize;
        let mut blocked = 0;
        let mut previous: Option<f64> = None;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let point = [
                create[0] + (rally[0] - create[0]) * t,
                create[1] + (rally[1] - create[1]) * t,
            ];
            let Some(i) = grid.index(point) else {
                blocked += 1;
                continue;
            };
            let z = grid.navigation_z(i);
            let body_inside = point[0] - graph.radius as f64 >= grid.frame.border as f64
                && point[1] - graph.radius as f64 >= grid.frame.border as f64
                && point[0] + (graph.radius as f64) < (grid.frame.width - grid.frame.border) as f64
                && point[1] + (graph.radius as f64)
                    < (grid.frame.height - grid.frame.border) as f64;
            let terrain_ok = body_inside
                && disk(point, graph.radius as f64, &grid.frame)
                    .iter()
                    .all(|&i| {
                        grid.frame.playable([
                            (i % grid.frame.width) as f64,
                            (i / grid.frame.width) as f64,
                        ]) && !grid.imp[i]
                            && grid.surface_allowed(i, &graph.surfaces, wade, deep)
                    });
            let obstacles_ok = obstacles
                .iter()
                .filter(|o| Some(o.index) != probe.native_index)
                .all(|o| {
                    (o.cell[0] - point[0]).hypot(o.cell[1] - point[1])
                        > o.radius + graph.radius as f64
                });
            if !terrain_ok
                || !obstacles_ok
                || previous.is_some_and(|before| (z - before).abs() > maximum_step)
            {
                blocked += 1;
            }
            previous = Some(z);
        }
        let access = graph.access(grid, rally, graph.radius as f64 + 2.);
        let reachable = access.map(|i| components.reachable_cells(i));
        rows.push(json!({"object_id":object_id,"waypoint":probe.waypoint,"object_type":object.type_name,"source":definition.source,
            "origin":if probe.native_index.is_some(){"native_object"}else{"scripted_creation"},
            "create_grid":create,"rally_grid":rally,"blocked_samples":blocked,
            "scope":"2D static exit projection","vertical_offsets":[exit.create[2],exit.rally[2]],
            "status":if blocked>0 {"blocked_static_exit"} else if access.is_some(){"connected_static_exit"}else{"no_rally_access"},
            "rally_reachable_cells":reachable,"probe_only":true,
            "rally_network_scope":if probe.native_index.is_some(){"current native map"}else{"current native terrain and obstacles before the hypothetical factory is created"},
            "hypothetical_factory_collision_in_rally_network":probe.native_index.is_some(),
            "production_roster_checked":false,"automatic_state_switch_inferred":false}));
    }
    rows
}

/// Produce additive diagnostics; unsupported metadata never grants a gameplay pass.
pub fn analyse(p: &Project, map: &MapFile) -> Result<Json> {
    let binding = binding(p)?;
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"binding":binding,"scope":"diagnostic static routes","status":"not_applicable",
            "reason":"TERRAIN_ONLY has no gameplay mobility requirement","profiles":[]}),
        );
    }
    let policy = spatial::policy(p)?;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let starts = starts(&objects, &frame);
    let targets = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            matches!(o.type_name.as_str(), "OreNode" | "OilDerrick")
                || o.type_name.ends_with("TechStructure")
        })
        .collect::<Vec<_>>();
    let (probes, script_pending) = factory_probes(p, map, &objects, &frame);
    if (starts.is_empty() || targets.is_empty()) && probes.is_empty() && script_pending.is_empty() {
        return Ok(
            json!({"binding":binding,"scope":"diagnostic static routes","status":"not_applicable",
            "reason":"no start-to-resource pairs or applicable factory exits","profiles":[]}),
        );
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let areas = match water_areas(map) {
        Ok(areas) => areas,
        Err(error) => {
            return Ok(
                json!({"binding":binding,"scope":"diagnostic static routes","status":"unsupported",
            "coverage":{"native_water":false},"reason":error.to_string(),"profiles":[]}),
            );
        }
    };
    let mut limitations = vec![
        "Static geometry only; no dynamic collision, turning simulation, engine pathfinder or AI decision prediction.",
        "Each locomotor and condition is analysed separately. No state transition or ability activation is inferred.",
        "Factory exits are capability probes for actual native factories and explicitly typed scripted creation branches; production roster and automatic state changes are not inferred.",
        "Terrain steps use the project spatial policy, not an inferred engine SlopeLimitIndex table.",
        "Crushable obstacles remain solid. AIR, CLIFF and other special locomotor surfaces are not modelled.",
        "Resource routes end in a bounded approach region; capture, harvesting and weapon permissions are not inferred.",
    ];
    let river_unsupported = match map.get("RiverAreas") {
        Ok(chunk) => Reader::new(&chunk.payload)
            .u32()
            .map_or(true, |count| count > 0),
        Err(_) => false,
    };
    if river_unsupported {
        limitations.push("Nonempty or malformed RiverAreas are not modelled.");
    }
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .into_iter()
        .map(|v| v as f64 * frame.z_scale)
        .collect();
    let water = (0..z.len())
        .map(|i| {
            let world = frame.world([(i % hm.width) as f64, (i / hm.width) as f64]);
            areas
                .iter()
                .filter(|area| inside(world, &area.polygon))
                .map(|area| area.level)
                .max_by(f64::total_cmp)
        })
        .collect::<Vec<_>>();
    let depths = official::water_depths();
    let mut counts = BTreeMap::from([("ground", 0usize), ("water", 0), ("deep_water", 0)]);
    for (i, level) in water.iter().enumerate() {
        let depth = level.map_or(0., |level| (level - z[i]).max(0.));
        *counts
            .get_mut(if depth <= 0. {
                "ground"
            } else if depth >= depths.deep {
                "deep_water"
            } else {
                "water"
            })
            .unwrap() += 1;
    }
    let grid = NativeGrid {
        frame,
        z,
        water,
        imp: terrain::flip(&bt.impassability, hm.width),
    };
    let (obstacles, unknown_obstacles) = obstacles(&objects, &grid.frame);
    let mut profiles = Vec::new();
    for unit in REPRESENTATIVES {
        let Some(definition) = official::object(unit) else {
            profiles.push(json!({"unit":unit,"status":"unsupported","reason":"unit absent from embedded catalog"}));
            continue;
        };
        let unit_radius = radius(unit, &grid.frame, [0., 0.], 0.);
        if definition.locomotors.is_empty() {
            profiles.push(json!({"unit":unit,"status":"unsupported","reason":"unit has no resolved locomotor bindings"}));
        }
        for state in &definition.locomotors {
            let Some(locomotor) = official::locomotor(&state.locomotor) else {
                profiles.push(json!({"unit":unit,"condition":state.condition,"locomotor":state.locomotor,"status":"unsupported","reason":"locomotor absent from embedded catalog"}));
                continue;
            };
            let unit_radius = match &unit_radius {
                Ok(radius) => *radius,
                Err(error) => {
                    profiles.push(json!({"unit":unit,"condition":state.condition,"locomotor":state.locomotor,"status":"unsupported","reason":error.to_string()}));
                    continue;
                }
            };
            if !locomotor
                .surfaces
                .iter()
                .any(|s| matches!(s.as_str(), "GROUND" | "WATER" | "DEEP_WATER"))
            {
                profiles.push(json!({"unit":unit,"condition":state.condition,"locomotor":state.locomotor,"status":"unsupported","reason":"no modelled surface"}));
                continue;
            }
            let radius = unit_radius.ceil() as usize + policy.navigation_clearance;
            let unmodelled_surfaces = locomotor
                .surfaces
                .iter()
                .filter(|surface| !matches!(surface.as_str(), "GROUND" | "WATER" | "DEEP_WATER"))
                .collect::<Vec<_>>();
            let graph = Graph::new(
                &grid,
                &locomotor.surfaces,
                radius,
                &obstacles,
                depths.wade,
                depths.deep,
            );
            let mut routes = Vec::new();
            for (seat, start) in &starts {
                let access = graph.access(&grid, *start, START_ACCESS_RADIUS);
                let distances = access.map(|i| graph.distances(&grid, i, policy.maximum_walk_step));
                for (index, target) in &targets {
                    let cell = grid
                        .frame
                        .grid([target.position[0] as f64, target.position[1] as f64]);
                    let target_radius = obstacles
                        .iter()
                        .find(|o| o.index == *index)
                        .map_or(4., |o| o.radius)
                        + radius as f64
                        + 2.;
                    let steps = distances.as_ref().and_then(|distances| {
                        disk(cell, target_radius, &grid.frame)
                            .into_iter()
                            .filter_map(|i| (distances[i] != u32::MAX).then_some(distances[i]))
                            .min()
                    });
                    routes.push(json!({"seat":seat,"target_id":index+1,"target_type":target.type_name,
                        "status":if access.is_none(){"no_start_access"}else if steps.is_some(){"reachable_static"}else{"unreachable_static"},
                        "path_steps":steps,"world_distance":steps.map(|steps|steps as f64*grid.frame.cell_size),
                        "start_access":access.map(|i|[i%hm.width,i/hm.width])}));
                }
            }
            profiles.push(json!({"unit":unit,"unit_source":definition.source,"condition":state.condition,
                "locomotor":state.locomotor,"locomotor_source":locomotor.source,"surfaces":locomotor.surfaces,
                "speed":state.speed,"status":if unknown_obstacles.is_empty()&&!river_unsupported&&locomotor.pending.is_empty()&&unmodelled_surfaces.is_empty()&&script_pending.is_empty(){"analysed_static"}else{"partial"},
                "collision_radius_cells":unit_radius,"conservative_clearance_cells":radius,
                "pending":locomotor.pending,"unmodelled_surfaces":unmodelled_surfaces,"open_cells":graph.open.iter().filter(|&&open|open).count(),"routes":routes,
                "factory_exits":exit_rows(&grid,&graph,&probes,&obstacles,policy.maximum_walk_step,depths.wade,depths.deep)}));
        }
    }
    let unsupported_profiles = profiles
        .iter()
        .filter(|profile| profile["status"] == "unsupported")
        .count();
    let partial_profiles = profiles
        .iter()
        .filter(|profile| profile["status"] == "partial")
        .count();
    Ok(
        json!({"binding":binding,"scope":"additive diagnostic; does not replace existing resource-route gates",
        "status":if river_unsupported||!unknown_obstacles.is_empty()||unsupported_profiles>0||partial_profiles>0{"partial"}else{"analysed_static"},
        "native_water_areas":areas.iter().map(|area|json!({"name":area.name,"height":area.level,"world_polygon":area.polygon})).collect::<Vec<_>>(),
        "coverage":{"water_source":"native StandingWaterAreas v2","river_areas_modelled":!river_unsupported,
            "unknown_static_obstacle_types":unknown_obstacles,"water_depth_counts":counts,"scripted_factory_pending":script_pending,
            "factory_probes":probes.len(),"mutually_exclusive_script_candidates_combined_as_obstacles":false,
            "unsupported_profiles":unsupported_profiles,"partial_profiles":partial_profiles},
        "limitations":limitations,"profiles":profiles}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{binary::Writer, map::Chunk};

    fn water_payload(level: u32) -> Vec<u8> {
        let mut w = Writer::new();
        w.u32(1);
        w.u32(2);
        w.s16("Native lake").unwrap();
        w.s16("").unwrap();
        w.f32(0.06);
        w.u8(0);
        w.s16("bump").unwrap();
        w.s16("sky").unwrap();
        w.u32(4);
        for [x, y] in [
            [-1000., -1000.],
            [-1000., 2000.],
            [2000., 2000.],
            [2000., -1000.],
        ] {
            w.f32(x);
            w.f32(y);
        }
        w.u32(level);
        w.s16("fx").unwrap();
        w.s16("depth").unwrap();
        w.data
    }
    fn grid(width: usize, height: usize) -> NativeGrid {
        NativeGrid {
            frame: Frame {
                width,
                height,
                border: 0,
                cell_size: 10.,
                z_scale: 1.,
            },
            z: vec![130.; width * height],
            water: vec![Some(200.); width * height],
            imp: vec![false; width * height],
        }
    }
    #[test]
    fn normal_deep_water_and_exiting_shallow_water_remain_distinct() {
        let mut grid = grid(9, 5);
        for y in 0..5 {
            grid.z[y * 9 + 4] = 190.;
        }
        let normal = Graph::new(&grid, &["DEEP_WATER".into()], 0, &[], 0., 35.);
        let exiting = Graph::new(
            &grid,
            &["WATER".into(), "DEEP_WATER".into()],
            0,
            &[],
            0.,
            35.,
        );
        assert_eq!(normal.distances(&grid, 2 * 9 + 1, 12.)[2 * 9 + 7], u32::MAX);
        assert_eq!(exiting.distances(&grid, 2 * 9 + 1, 12.)[2 * 9 + 7], 6);
        // Changing the sea floor does not create a slope for a sea-level ship.
        grid.z[2 * 9 + 2] = 0.;
        assert_eq!(exiting.distances(&grid, 2 * 9 + 1, 12.)[2 * 9 + 7], 6);
    }
    #[test]
    fn land_water_capabilities_do_not_implicitly_switch_and_rectangular_routes_work() {
        for (width, height) in [(5, 11), (11, 5)] {
            let mut grid = grid(width, height);
            for y in 0..height {
                for x in 0..width / 2 {
                    grid.z[y * width + x] = 210.;
                }
            }
            let land = Graph::new(&grid, &["GROUND".into()], 0, &[], 0., 35.);
            let water = Graph::new(
                &grid,
                &["WATER".into(), "DEEP_WATER".into()],
                0,
                &[],
                0.,
                35.,
            );
            assert!(land.open[width / 2 - 1]);
            assert!(!water.open[width / 2 - 1]);
            assert!(!land.open[width - 1]);
            assert!(water.open[width - 1]);
            assert_eq!(
                land.distances(&grid, 0, 12.)[(height - 1) * width],
                (height - 1) as u32
            );
        }
    }

    #[test]
    fn component_sizes_equal_distance_floods_for_every_start_and_step_limit() {
        for (width, height) in [(5, 11), (11, 5)] {
            for water in [false, true] {
                let mut grid = grid(width, height);
                if water {
                    for y in 0..height {
                        grid.z[y * width + width / 2] = 190.;
                    }
                } else {
                    grid.water.fill(None);
                    grid.z.fill(210.);
                    grid.z[width + 1] = 280.;
                }
                grid.imp[0] = true;
                for y in 0..height - 1 {
                    grid.imp[y * width + width - 2] = true;
                }
                let surfaces = if water {
                    vec!["DEEP_WATER".into()]
                } else {
                    vec!["GROUND".into()]
                };
                let graph = Graph::new(&grid, &surfaces, 0, &[], 0., 35.);
                for maximum_step in [0.5, 12., 100.] {
                    let components = graph.components(&grid, maximum_step);
                    for start in 0..grid.z.len() {
                        let expected = graph
                            .distances(&grid, start, maximum_step)
                            .iter()
                            .filter(|&&distance| distance != u32::MAX)
                            .count();
                        assert_eq!(
                            components.reachable_cells(start),
                            expected,
                            "{width}x{height}, water={water}, step={maximum_step}, start={start}"
                        );
                        for next in neighbors_rect(start, width, height) {
                            assert_eq!(
                                graph.can_step(&grid, start, next, maximum_step),
                                graph.can_step(&grid, next, start, maximum_step)
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn official_water_polygon_parser_uses_native_height_and_rejects_unknown_versions() {
        use crate::formats::{binary::Writer, map::Chunk};
        let mut w = Writer::new();
        w.u32(1);
        w.u32(2);
        w.s16("Native lake").unwrap();
        w.s16("").unwrap();
        w.f32(0.06);
        w.u8(0);
        w.s16("bump").unwrap();
        w.s16("sky").unwrap();
        w.u32(4);
        for [x, y] in [[0., 0.], [0., 100.], [100., 100.], [100., 0.]] {
            w.f32(x);
            w.f32(y);
        }
        w.u32(315);
        w.s16("fx").unwrap();
        w.s16("depth").unwrap();
        let mut map = MapFile {
            order: vec![],
            table: BTreeMap::from([(1, "StandingWaterAreas".into())]),
            chunks: vec![Chunk {
                name_idx: 1,
                version: 2,
                payload: w.data,
            }],
        };
        let areas = water_areas(&map).unwrap();
        assert_eq!(areas[0].level, 315.);
        assert!(inside([50., 50.], &areas[0].polygon));
        assert!(!inside([150., 50.], &areas[0].polygon));
        map.chunks[0].version = 3;
        assert!(water_areas(&map).is_err());
    }

    #[test]
    fn parser_accepts_the_exact_water_bytes_from_all_four_official_fragments() {
        let zip = crate::formats::archive::unzip(include_bytes!(
            "../../tests/fixtures/official-map-fragments.zip"
        ))
        .unwrap();
        let fixtures: Json = serde_json::from_slice(&zip["official-map-fragments.json"]).unwrap();
        for case in fixtures["cases"].as_array().unwrap() {
            let chunk = case["chunks"]
                .as_array()
                .unwrap()
                .iter()
                .find(|chunk| chunk["name"] == "StandingWaterAreas")
                .unwrap();
            let payload =
                crate::gameplay::scripts::unhex(chunk["payload_hex"].as_str().unwrap()).unwrap();
            let map = MapFile {
                order: vec![],
                table: BTreeMap::from([(1, "StandingWaterAreas".into())]),
                chunks: vec![Chunk {
                    name_idx: 1,
                    version: chunk["version"].as_u64().unwrap() as u16,
                    payload,
                }],
            };
            let areas = water_areas(&map).unwrap();
            assert_eq!(areas.len(), 1, "{}", case["source"]);
            assert_eq!(areas[0].level, 200., "{}", case["source"]);
        }
    }

    #[test]
    fn full_report_uses_native_water_and_keeps_real_unit_conditions_and_catalog_binding() {
        use std::path::PathBuf;
        let p = Project {
            root: PathBuf::from("/tmp/ra3-mobility-embedded-defaults-no-project"),
            id: "mobility-test".into(),
            cfg: json!({"GRID":64,"GRID_HEIGHT":40,"BORDER":2,"CELL":10.,"Z_SCALE":1.,"WATER_Z":999.}),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "mobility-test".into(),
            name: "mobility-testT0.0.1".into(),
            assets: PathBuf::new(),
            dist: PathBuf::new(),
        };
        let mut map = MapFile {
            order: vec![],
            table: BTreeMap::new(),
            chunks: vec![],
        };
        for name in [
            "HeightMapData",
            "BlendTileData",
            "ObjectsList",
            "StandingWaterAreas",
            "RiverAreas",
            "Object",
            "waypointName",
            "PlayerScriptsList",
        ] {
            map.id(name).unwrap();
        }
        let frame = Frame::from_project(&p);
        let mut elevations = vec![130; 64 * 40];
        for y in 0..40 {
            for x in 24..=28 {
                elevations[y * 64 + x] = 190;
            }
        }
        let hm = HeightMap {
            width: 64,
            height: 40,
            border: 2,
            borders: vec![[0, 0, 60, 36]],
            elevations: terrain::flip(&elevations, 64),
        };
        let surface = terrain::surface::Surface {
            chosen: vec![0; 64 * 40],
            impassable: vec![false; 64 * 40],
            build_block: vec![false; 64 * 40],
            layer_edge: vec![false; 64 * 40],
            shore: vec![false; 64 * 40],
            textures: vec![["Test".into(), String::new(), String::new()]],
        };
        let point = |cell| {
            let world = frame.world(cell);
            [world[0] as f32, world[1] as f32, 0.]
        };
        let objects = vec![
            MapObject {
                version: 3,
                position: point([8., 20.]),
                angle: 0.,
                road_type: 0,
                type_name: "Waypoint".into(),
                props: vec![map::Property {
                    key: "waypointName".into(),
                    value: Value::Ascii("Player_1_Start".into()),
                }],
            },
            MapObject {
                version: 3,
                position: point([52., 20.]),
                angle: 0.,
                road_type: 0,
                type_name: "OilDerrick".into(),
                props: vec![],
            },
        ];
        for (name, version, payload) in [
            ("HeightMapData", 6, hm.encode().unwrap()),
            ("BlendTileData", 27, surface.encode(64, &[]).unwrap()),
            (
                "ObjectsList",
                1,
                map::write_objects(&objects, &map).unwrap(),
            ),
            ("StandingWaterAreas", 2, water_payload(200)),
            ("RiverAreas", 3, 0u32.to_le_bytes().to_vec()),
            ("PlayerScriptsList", 1, vec![]),
        ] {
            map.chunks.push(Chunk {
                name_idx: map.key(name).unwrap(),
                version,
                payload,
            });
        }
        let report = analyse(&p, &map).unwrap();
        assert_eq!(
            report["binding"]["catalog"]["revision"],
            official::binding().revision
        );
        assert_eq!(
            report["coverage"]["water_source"],
            "native StandingWaterAreas v2"
        );
        let profiles = report["profiles"].as_array().unwrap();
        assert_eq!(profiles.iter().filter(|profile|profile["unit"]=="AlliedMCV"&&profile["condition"]=="NORMAL").count(),2);
        let dolphin = |condition: &str| {
            profiles
                .iter()
                .find(|profile| {
                    profile["unit"] == "AlliedAntiNavalScout" && profile["condition"] == condition
                })
                .unwrap()
        };
        assert_eq!(
            dolphin("NORMAL")["routes"][0]["status"],
            "unreachable_static"
        );
        assert_eq!(
            dolphin("EXITING_PRODUCTION_STRUCTURE")["routes"][0]["status"],
            "reachable_static"
        );
        assert!(
            report.get("passed").is_none(),
            "diagnostic routes must not replace legacy gameplay gates"
        );
        let mut scaled = objects.clone();
        scaled[1].props.push(map::Property {
            key: "objectPrototypeScale".into(),
            value: Value::Float(2.),
        });
        map.id("objectPrototypeScale").unwrap();
        map.get_mut("ObjectsList").unwrap().payload = map::write_objects(&scaled, &map).unwrap();
        let scaled_report = analyse(&p, &map).unwrap();
        assert_eq!(scaled_report["status"], "partial");
        assert!(
            scaled_report["coverage"]["unknown_static_obstacle_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value
                    .as_str()
                    .is_some_and(|text| text.contains("objectPrototypeScale")))
        );
        map.get_mut("StandingWaterAreas").unwrap().version = 99;
        let unsupported = analyse(&p, &map).unwrap();
        assert_eq!(unsupported["status"], "unsupported");
        assert_eq!(unsupported["coverage"]["native_water"], false);
        let mut terrain_only = p.clone();
        terrain_only.cfg["TERRAIN_ONLY"] = json!(true);
        let empty = MapFile {
            order: vec![],
            table: BTreeMap::new(),
            chunks: vec![],
        };
        assert_eq!(
            analyse(&terrain_only, &empty).unwrap()["status"],
            "not_applicable"
        );
        map.get_mut("ObjectsList").unwrap().payload.clear();
        let no_targets = analyse(&p, &map).unwrap();
        assert_eq!(no_targets["status"], "not_applicable");
        assert_eq!(no_targets["profiles"], json!([]));
    }

    #[test]
    fn scripted_factory_exit_probe_uses_final_heading_and_detects_a_blocked_corridor() {
        let mut grid = grid(64, 40);
        grid.z.fill(210.);
        grid.water.fill(None);
        let graph = Graph::new(&grid, &["GROUND".into()], 1, &[], 0., 35.);
        let center = [32., 20.];
        let heading = 45f32.to_radians();
        let world = grid.frame.world(center);
        let mut probes = vec![ExitProbe {
            object: MapObject {
                version: 3,
                position: [world[0] as f32, world[1] as f32, 0.],
                angle: heading,
                road_type: 0,
                type_name: "AlliedWarFactory".into(),
                props: vec![],
            },
            native_index: None,
            waypoint: Some("gift-alt-test-factory-1".into()),
        }];
        probes.push(ExitProbe {
            object: probes[0].object.clone(),
            native_index: Some(7),
            waypoint: None,
        });
        probes.push(ExitProbe {
            object: probes[0].object.clone(),
            native_index: None,
            waypoint: Some("gift-primary-test-factory".into()),
        });
        let rows = exit_rows(&grid, &graph, &probes, &[], 12., 0., 35.);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["origin"], "scripted_creation");
        assert_eq!(rows[0]["waypoint"], "gift-alt-test-factory-1");
        assert_eq!(rows[0]["status"], "connected_static_exit");
        assert_eq!(rows[1]["origin"], "native_object");
        assert_eq!(rows[1]["object_id"], 8);
        assert_eq!(rows[2]["waypoint"], "gift-primary-test-factory");
        let exit = official::object("AlliedWarFactory")
            .unwrap()
            .production_exit
            .as_ref()
            .unwrap();
        let create = world_offset(&grid.frame, center, heading, exit.create);
        let rally = world_offset(&grid.frame, center, heading, exit.rally);
        assert_eq!(rows[0]["create_grid"], json!(create));
        assert_eq!(rows[0]["rally_grid"], json!(rally));
        let access = graph
            .access(&grid, rally, graph.radius as f64 + 2.)
            .unwrap();
        let count = graph
            .distances(&grid, access, 12.)
            .iter()
            .filter(|&&distance| distance != u32::MAX)
            .count();
        for row in &rows {
            assert_eq!(row["rally_reachable_cells"], count);
        }
        let obstacles = vec![Obstacle {
            index: 999,
            cell: [(create[0] + rally[0]) * 0.5, (create[1] + rally[1]) * 0.5],
            radius: 1.,
        }];
        assert_eq!(
            exit_rows(&grid, &graph, &probes, &obstacles, 12., 0., 35.)[0]["status"],
            "blocked_static_exit"
        );
    }
}
