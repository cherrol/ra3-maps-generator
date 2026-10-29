//! Static placement evidence from official assets, kept separate from map design policy.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    gameplay::{
        production_facing,
        production_spawns::PlacementPolicy,
        scripts::{self, Node},
    },
    pipeline::project::Project,
    resources::official,
    terrain,
};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};

type Polygon = Vec<[f64; 2]>;
const REVISION: &str = "official-placement-v2-actual-conditional-headings";

/// Explicit mode overrides the compatibility defaults in either direction.
pub fn enforced(p: &Project) -> Result<bool> {
    if p.cfg["OFFICIAL_PLACEMENT"]
        .get("mode")
        .is_some_and(|mode| !mode.is_string())
    {
        return Err(err("OFFICIAL_PLACEMENT.mode must be strict or diagnostic"));
    }
    match p.cfg["OFFICIAL_PLACEMENT"]
        .get("mode")
        .and_then(Json::as_str)
    {
        Some("strict") => Ok(true),
        Some("diagnostic") => Ok(false),
        Some(other) => Err(err(format!("unknown OFFICIAL_PLACEMENT mode {other}"))),
        None if !p.cfg["OFFICIAL_PLACEMENT"].is_null()
            && !p.cfg["OFFICIAL_PLACEMENT"].is_object() =>
        {
            Err(err("OFFICIAL_PLACEMENT must be an object"))
        }
        None => Ok(!p.cfg["COORDINATE_PIPELINE"].is_object()
            && p.string("TEXTURE_PROFILE", "") != "legacy"
            && p.string("RAMP_PROFILE", "") != "legacy"
            && !p.cfg["SPAWN_PLACEMENT_REFERENCE"].is_string()
            && !p.cfg["RAMP_CLEARANCE_REFERENCE"].is_string()),
    }
}

pub fn binding(p: &Project) -> Result<Json> {
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"algorithm":REVISION,"status":"not_applicable","terrain_only":true,"catalog":official::binding()}),
        );
    }
    let common = match std::fs::read(p.root.join("templates/rules/common-T0.5.1.json")) {
        Ok(bytes) => Some(crc32fast::hash(&bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    Ok(json!({"algorithm":REVISION,"enforced":enforced(p)?,
        "settings":p.cfg["OFFICIAL_PLACEMENT"],"teams":p.cfg["BASES"],
        "water_z":p.number("WATER_Z",200.),"cell":p.number("CELL",10.),"z_scale":p.number("Z_SCALE",0.0390625),
        "compatibility":{"frozen":p.cfg["COORDINATE_PIPELINE"].is_object(),"texture_profile":p.cfg["TEXTURE_PROFILE"],"ramp_profile":p.cfg["RAMP_PROFILE"],"spawn_reference":p.cfg["SPAWN_PLACEMENT_REFERENCE"],"ramp_reference":p.cfg["RAMP_CLEARANCE_REFERENCE"]},
        "public_production_policy_crc32":common,"refinery_policy":terrain::ramp_clearance::policy(p)?,"spatial_policy":terrain::spatial::policy(p)?,"catalog":official::binding()}))
}

pub(crate) fn strict_project(p: &Project) -> Project {
    let mut strict = p.clone();
    if !strict.cfg["OFFICIAL_PLACEMENT"].is_object() {
        strict.cfg["OFFICIAL_PLACEMENT"] = json!({});
    }
    strict.cfg["OFFICIAL_PLACEMENT"]["mode"] = json!("strict");
    strict.cfg["OFFICIAL_PLACEMENT"]["scope"] = json!("production_repair");
    strict
}

#[derive(Clone)]
struct Binding {
    waypoint: String,
    kind: String,
    seat: Option<usize>,
    heading: Option<f32>,
}

fn building(kind: &str) -> bool {
    let named = production_facing::gifted_building(kind)
        || kind.contains("Refinery")
        || kind.contains("ConstructionYard")
        || kind.contains("TechStructure")
        || kind == "OilDerrick"
        || kind.starts_with("OreNode")
        || kind.starts_with("EI_Structure_")
        || kind.starts_with("ST_");
    named
        || official::object(kind).is_some_and(|object| {
            let structural_base = object.lineage.iter().any(|base| {
                matches!(
                    base.as_str(),
                    "BaseStructure" | "BaseTechStructure" | "BaseOreNode"
                ) || base.starts_with("BaseCivilianStructure")
            });
            structural_base
                || (object.kind_of.iter().any(|flag| flag == "STRUCTURE")
                    && !object.kind_of.iter().any(|flag| {
                        matches!(
                            flag.as_str(),
                            "SIMPLE_OBJECT_PROP" | "OPTIMIZED_PROP" | "SHRUBBERY" | "TREE"
                        )
                    }))
        })
}

fn instance_pending(object: &MapObject) -> Option<String> {
    match prop(&object.props, "objectPrototypeScale") {
        Some(Value::Float(scale)) if (*scale - 1.).abs() <= 1e-6 => None,
        Some(Value::Int(1)) | None => None,
        Some(value) => Some(format!(
            "pending: native objectPrototypeScale {value:?} requires verified collision-scale semantics"
        )),
    }
}

fn bindings(
    nodes: &[Node],
    out: &mut Vec<Binding>,
    pending: &mut Vec<String>,
    markers: &mut Vec<Json>,
) -> Result<()> {
    for node in nodes {
        if let Some(children) = &node.children {
            bindings(children, out, pending, markers)?;
        } else if node.command.as_deref().is_some_and(|command| {
            command.starts_with("CREATE_") && command.contains("AT_WAYPOINT")
        }) {
            let args = scripts::string_arguments(node)?;
            let value = |kind| {
                args.iter()
                    .find(|(ty, _)| *ty == kind)
                    .map(|(_, value)| value.as_str())
            };
            let (Some(kind), Some(waypoint)) = (value(15), value(7)) else {
                pending.push("creation action has no explicit object type or waypoint".into());
                continue;
            };
            if official::certainly_noncolliding_marker(kind) {
                markers.push(json!({"kind":kind,"waypoint":waypoint,"classification":official::classification(kind),"scope":"consensus noncolliding nonbuilding marker; complete definition/geometry remains unresolved"}));
                continue;
            }
            if official::object(kind).is_some() && !building(kind) {
                continue;
            }
            let heading = scripts::scalar_arguments(node)?
                .iter()
                .find(|arg| arg.kind == 17)
                .and_then(|arg| {
                    scripts::unhex(&node.body)
                        .ok()?
                        .get(arg.real_offset..arg.real_offset + 4)?
                        .try_into()
                        .ok()
                        .map(f32::from_le_bytes)
                });
            let seat = value(3).and_then(|team| production_facing::seat(team).ok());
            out.push(Binding {
                waypoint: waypoint.into(),
                kind: kind.into(),
                seat,
                heading,
            });
        }
    }
    Ok(())
}

pub(crate) type ScriptedFactory = (String, [f64; 2], f32, String);

/// Only explicit typed factory actions and their actual final native headings.
pub(crate) fn scripted_factories(p: &Project, map: &MapFile) -> Result<Vec<ScriptedFactory>> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut found = Vec::new();
    let mut pending = Vec::new();
    bindings(&nodes, &mut found, &mut pending, &mut Vec::new())?;
    let mut seen = BTreeSet::new();
    let mut factories = Vec::new();
    for binding in found {
        let Some(definition) = official::object(&binding.kind) else {
            continue;
        };
        if !definition.kind_of.iter().any(|flag| flag == "FS_FACTORY")
            || definition
                .production_exit
                .as_ref()
                .is_none_or(|exit| exit.no_exit_path)
        {
            continue;
        }
        let heading = binding
            .heading
            .filter(|angle| angle.is_finite())
            .ok_or_else(|| {
                err(format!(
                    "factory {} has no explicit final heading",
                    binding.waypoint
                ))
            })?;
        if !seen.insert((
            binding.kind.clone(),
            binding.waypoint.clone(),
            heading.to_bits(),
        )) {
            continue;
        }
        let matches = objects
            .iter()
            .filter(|object| {
                prop(&object.props, "waypointName").and_then(Value::text)
                    == Some(binding.waypoint.as_str())
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(err(format!(
                "factory {} requires one final native waypoint",
                binding.waypoint
            )));
        }
        let cell = frame.grid([matches[0].position[0] as f64, matches[0].position[1] as f64]);
        factories.push((binding.kind, cell, heading, binding.waypoint));
    }
    Ok(factories)
}

fn original_waypoint(name: &str) -> &str {
    name.strip_prefix("gift-alt-")
        .and_then(|name| name.rsplit_once('-').map(|(original, _)| original))
        .unwrap_or(name)
}

fn transform(frame: &Frame, center: [f64; 2], heading: f32, local: [f64; 2]) -> [f64; 2] {
    frame.local_to_grid(center, heading as f64, local)
}

pub(crate) fn world_offset(
    frame: &Frame,
    cell: [f64; 2],
    heading: f32,
    offset: [f64; 3],
) -> [f64; 2] {
    transform(frame, cell, heading, [offset[0], offset[1]])
}

pub(crate) fn collision_polygons(
    kind: &str,
    frame: &Frame,
    cell: [f64; 2],
    heading: f32,
) -> Result<Vec<Polygon>> {
    if official::object(kind)
        .is_some_and(|object| object.kind_of.iter().any(|flag| flag == "NO_COLLIDE"))
    {
        return Ok(Vec::new());
    }
    let model = Model::load(kind);
    if !model.pending.is_empty() {
        return Err(err(format!(
            "official geometry for {kind} is unresolved: {}",
            model.pending.join("; ")
        )));
    }
    Ok(model.placed(frame, cell, heading, false))
}

fn local_shape(shape: &official::Shape) -> Option<Polygon> {
    if !shape.active {
        return Some(Vec::new());
    }
    if shape
        .front_angle_radians
        .is_some_and(|angle| angle.abs() > 1e-9)
    {
        return None;
    }
    let [x, y, _] = shape.offset;
    match shape.kind.as_str() {
        "BOX" => Some(vec![
            [x - shape.major_radius, y - shape.minor_radius],
            [x + shape.major_radius, y - shape.minor_radius],
            [x + shape.major_radius, y + shape.minor_radius],
            [x - shape.major_radius, y + shape.minor_radius],
        ]),
        "SPHERE" | "CYLINDER" => {
            // Circumscribed polygon keeps the sampled footprint conservative.
            let radius = shape.major_radius / (std::f64::consts::PI / 16.).cos();
            Some(
                (0..16)
                    .map(|i| {
                        let angle = i as f64 * std::f64::consts::TAU / 16.;
                        [x + radius * angle.cos(), y + radius * angle.sin()]
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

fn overlap(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    for polygon in [a, b] {
        for i in 0..polygon.len() {
            let delta = [
                polygon[(i + 1) % polygon.len()][0] - polygon[i][0],
                polygon[(i + 1) % polygon.len()][1] - polygon[i][1],
            ];
            let axis = [-delta[1], delta[0]];
            let extent = |points: &[[f64; 2]]| {
                points
                    .iter()
                    .map(|point| point[0] * axis[0] + point[1] * axis[1])
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                        (low.min(value), high.max(value))
                    })
            };
            let (al, ah) = extent(a);
            let (bl, bh) = extent(b);
            if ah < bl - 1e-9 || bh < al - 1e-9 {
                return false;
            }
        }
    }
    true
}

fn square(center: [f64; 2], radius: f64) -> Polygon {
    vec![
        [center[0] - radius, center[1] - radius],
        [center[0] + radius, center[1] - radius],
        [center[0] + radius, center[1] + radius],
        [center[0] - radius, center[1] + radius],
    ]
}

#[derive(Clone)]
struct Model {
    kind: String,
    source: String,
    placement: Vec<Polygon>,
    collision: Vec<Polygon>,
    exit: Option<([f64; 2], [f64; 2])>,
    water: Option<bool>,
    pending: Vec<String>,
}
impl Model {
    fn load(kind: &str) -> Self {
        let mut model = Self {
            kind: kind.into(),
            source: String::new(),
            placement: Vec::new(),
            collision: Vec::new(),
            exit: None,
            water: None,
            pending: Vec::new(),
        };
        let Some(object) = official::object(kind) else {
            model
                .pending
                .push("official object definition is unavailable".into());
            return model;
        };
        model.source = object.source.clone();
        model.pending = object.pending.clone();
        if object.rotation_anchor != [0., 0.] {
            model
                .pending
                .push("nonzero rotation anchor requires verified engine pivot semantics".into());
        }
        for shape in &object.shapes {
            match local_shape(shape) {
                Some(polygon) if !polygon.is_empty() => model.collision.push(polygon),
                Some(_) => {}
                None => model
                    .pending
                    .push(format!("unsupported collision shape {}", shape.kind)),
            }
        }
        if let Some(shape) = &object.placement_shape {
            match local_shape(shape) {
                Some(polygon) if !polygon.is_empty() => model.placement.push(polygon),
                Some(_) => {}
                None => model
                    .pending
                    .push(format!("unsupported placement shape {}", shape.kind)),
            }
        } else {
            model.placement = model.collision.clone();
        }
        if model.placement.is_empty() {
            model
                .pending
                .push("official placement geometry is unavailable".into());
        }
        let has = |flag: &str| object.kind_of.iter().any(|value| value == flag);
        model.water = if has("CAN_NOT_BUILD_ON_LAND") {
            Some(true)
        } else if !has("CAN_BUILD_ON_WATER") && !has("CAN_BUILD_ON_DEEP_WATER") {
            Some(false)
        } else {
            None
        };
        if let Some(exit) = &object.production_exit
            && !exit.no_exit_path
        {
            model.exit = Some((
                [exit.create[0], exit.create[1]],
                [exit.rally[0], exit.rally[1]],
            ));
        }
        model
    }
    fn placed(&self, frame: &Frame, cell: [f64; 2], heading: f32, placement: bool) -> Vec<Polygon> {
        let shapes = if placement {
            &self.placement
        } else {
            &self.collision
        };
        shapes
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|point| transform(frame, cell, heading, *point))
                    .collect()
            })
            .collect()
    }
}

pub(crate) struct Obstacles {
    polygons: Vec<Polygon>,
    pending: Vec<String>,
}
#[cfg(test)]
impl Obstacles {
    fn new(polygons: Vec<Polygon>) -> Self {
        Self {
            polygons,
            pending: Vec::new(),
        }
    }
}

pub(crate) struct Checker {
    frame: Frame,
    z: Vec<f64>,
    imp: Vec<bool>,
    blocked: Vec<bool>,
    water: f64,
    exit_clearance: f64,
    maximum_step: f64,
    facing: Option<production_facing::Context>,
    bindings: Vec<Binding>,
    script_pending: Vec<String>,
    classified_markers: Vec<Json>,
    models: BTreeMap<String, Model>,
    policy: Option<PlacementPolicy>,
}
impl Checker {
    pub(crate) fn for_candidates(
        p: &Project,
        n: usize,
        z: &[f64],
        imp: &[bool],
        blocked: &[bool],
        nodes: &[Node],
        objects: &[MapObject],
    ) -> Result<Option<Self>> {
        if !enforced(p)? {
            return Ok(None);
        }
        Ok(Some(Self::from_parts(
            p,
            Frame::from_project(p),
            n,
            z,
            imp,
            blocked,
            nodes,
            objects,
        )?))
    }
    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        p: &Project,
        frame: Frame,
        n: usize,
        z: &[f64],
        imp: &[bool],
        blocked: &[bool],
        nodes: &[Node],
        objects: &[MapObject],
    ) -> Result<Self> {
        if frame.width != n
            || z.len() != frame.width * frame.height
            || imp.len() != z.len()
            || blocked.len() != z.len()
        {
            return Err(err("official placement terrain dimensions differ"));
        }
        let positive = |key: &str, default: f64| -> Result<f64> {
            match p.cfg["OFFICIAL_PLACEMENT"].get(key) {
                Some(value) => value
                    .as_f64()
                    .filter(|value| value.is_finite() && *value > 0.)
                    .ok_or_else(|| err(format!("invalid official placement {key}"))),
                None => Ok(default),
            }
        };
        let mut bindings_out = Vec::new();
        let mut script_pending = Vec::new();
        let mut classified_markers = Vec::new();
        bindings(
            nodes,
            &mut bindings_out,
            &mut script_pending,
            &mut classified_markers,
        )?;
        let mut models = BTreeMap::new();
        for kind in bindings_out
            .iter()
            .map(|binding| binding.kind.as_str())
            .chain(
                objects
                    .iter()
                    .filter(|object| {
                        official::object(&object.type_name).is_some()
                            && !object.type_name.contains("Waypoint")
                    })
                    .map(|object| object.type_name.as_str()),
            )
        {
            models
                .entry(kind.into())
                .or_insert_with(|| Model::load(kind));
        }
        let spatial = terrain::spatial::policy(p)?;
        let facing = if !bindings_out
            .iter()
            .any(|binding| production_facing::gifted_building(&binding.kind))
        {
            None
        } else {
            Some(production_facing::Context::from_objects(p, objects)?)
        };
        let policy = if bindings_out.is_empty() {
            None
        } else {
            Some(PlacementPolicy::load(p)?)
        };
        Ok(Self {
            frame,
            z: z.into(),
            imp: imp.into(),
            blocked: blocked.into(),
            water: p.number("WATER_Z", 200.),
            exit_clearance: positive("exit_clearance_cells", spatial.navigation_clearance as f64)?,
            maximum_step: positive("maximum_exit_step", spatial.maximum_walk_step)?,
            facing,
            bindings: bindings_out,
            script_pending,
            classified_markers,
            models,
            policy,
        })
    }
    fn matching(&self, name: &str) -> Vec<&Binding> {
        self.bindings
            .iter()
            .filter(|binding| {
                binding.waypoint == name || binding.waypoint == original_waypoint(name)
            })
            .collect()
    }
    fn binding_heading(&self, binding: &Binding, cell: [f64; 2], planning: bool) -> Result<f32> {
        if planning && production_facing::gifted_building(&binding.kind) {
            let facing = self
                .facing
                .as_ref()
                .ok_or_else(|| err("pending: gift facing teams are unavailable"))?;
            let seat = binding
                .seat
                .ok_or_else(|| err("pending: gift facing owner is unavailable"))?;
            return facing.candidate_heading(&self.frame, seat, cell);
        }
        let heading = binding.heading.ok_or_else(|| {
            err(format!(
                "pending: {} at {} has no explicit creation heading",
                binding.kind, binding.waypoint
            ))
        })?;
        let legal = heading.is_finite()
            && production_facing::ALLOWED_DEGREES.iter().any(|degrees| {
                let delta = f64::from(heading) - f64::from(*degrees).to_radians();
                delta.sin().atan2(delta.cos()).abs()
                    <= crate::gameplay::ore_facing::TOLERANCE_RADIANS
            });
        if !legal {
            return Err(err(format!(
                "illegal explicit creation heading for {} at {}",
                binding.kind, binding.waypoint
            )));
        }
        Ok(heading)
    }

    pub(crate) fn planned_obstacles(&self, objects: &[MapObject]) -> Obstacles {
        self.obstacles_for(objects, true)
    }

    fn obstacles(&self, objects: &[MapObject]) -> Obstacles {
        self.obstacles_for(objects, false)
    }

    fn obstacles_for(&self, objects: &[MapObject], planning: bool) -> Obstacles {
        let mut polygons = Vec::new();
        let mut pending = Vec::new();
        for object in objects {
            if official::object(&object.type_name)
                .is_some_and(|object| object.kind_of.iter().any(|flag| flag == "NO_COLLIDE"))
            {
                continue;
            }
            let cell = self
                .frame
                .grid([object.position[0] as f64, object.position[1] as f64]);
            if object.type_name.contains("Waypoint") {
                if let Some(name) = prop(&object.props, "waypointName").and_then(Value::text) {
                    for binding in self.matching(name) {
                        match (
                            self.models.get(&binding.kind),
                            self.binding_heading(binding, cell, planning),
                        ) {
                            (Some(model), Ok(heading)) => {
                                polygons.extend(model.placed(&self.frame, cell, heading, false))
                            }
                            (_, Err(error)) => pending.push(error.to_string()),
                            (None, _) => pending.push(format!(
                                "pending: obstacle definition {} is unavailable",
                                binding.kind
                            )),
                        }
                    }
                }
            } else if let Some(model) = self.models.get(&object.type_name) {
                polygons.extend(model.placed(&self.frame, cell, object.angle, false));
            }
        }
        Obstacles { polygons, pending }
    }
    pub(crate) fn acceptable(&self, name: &str, cell: [usize; 2], others: &Obstacles) -> bool {
        let bindings = self.matching(name);
        // Unregistered primary creation points must be resolved before strict placement.
        !bindings.is_empty()
            && bindings.into_iter().all(|binding| {
                let point = cell.map(|value| value as f64);
                let Ok(heading) = self.binding_heading(binding, point, true) else {
                    return false;
                };
                self.assess(&binding.kind, point, heading, others, true)
                    .0
                    .is_empty()
            })
    }
    pub(crate) fn acceptable_object(
        &self,
        object: &MapObject,
        cell: [usize; 2],
        others: &Obstacles,
    ) -> bool {
        self.object_rejections(object, cell, others).is_empty()
    }

    pub(crate) fn object_rejections(
        &self,
        object: &MapObject,
        cell: [usize; 2],
        others: &Obstacles,
    ) -> Vec<String> {
        if let Some(name) = prop(&object.props, "waypointName").and_then(Value::text) {
            let bindings = self.matching(name);
            if bindings.is_empty() {
                return vec![format!("pending: no explicit building binding for {name}")];
            }
            let mut findings = Vec::new();
            let point = cell.map(|value| value as f64);
            for binding in bindings {
                match self.binding_heading(binding, point, true) {
                    Ok(heading) => findings.extend(
                        self.assess(&binding.kind, point, heading, others, true)
                            .0
                            .into_iter()
                            .map(|detail| {
                                format!("{}/{}: {detail}", binding.waypoint, binding.kind)
                            }),
                    ),
                    Err(error) => findings.push(error.to_string()),
                }
            }
            return findings;
        }
        self.assess(
            &object.type_name,
            cell.map(|value| value as f64),
            object.angle,
            others,
            true,
        )
        .0
    }
    fn cells(&self, polygon: &[[f64; 2]]) -> Option<Vec<usize>> {
        if polygon.iter().any(|point| !self.frame.playable(*point)) {
            return None;
        }
        let extent = |axis: usize| {
            polygon
                .iter()
                .map(|point| point[axis])
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                })
        };
        let (xmin, xmax) = extent(0);
        let (ymin, ymax) = extent(1);
        let mut cells = Vec::new();
        for y in (ymin - 0.5).floor().max(0.) as usize
            ..=(ymax + 0.5).ceil().min((self.frame.height - 1) as f64) as usize
        {
            for x in (xmin - 0.5).floor().max(0.) as usize
                ..=(xmax + 0.5).ceil().min((self.frame.width - 1) as f64) as usize
            {
                if overlap(polygon, &square([x as f64, y as f64], 0.5)) {
                    cells.push(y * self.frame.width + x);
                }
            }
        }
        Some(cells)
    }
    fn clear_ground(&self, polygon: &[[f64; 2]], water: bool, flat: bool) -> bool {
        let Some(cells) = self.cells(polygon) else {
            return false;
        };
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for cell in cells {
            if self.imp[cell]
                || self.blocked[cell]
                || (if water {
                    self.z[cell] >= self.water - 1.
                } else {
                    self.z[cell] <= self.water
                })
            {
                return false;
            }
            low = low.min(self.z[cell]);
            high = high.max(self.z[cell]);
        }
        !flat || water || high - low <= 1.
    }
    fn assess(
        &self,
        kind: &str,
        cell: [f64; 2],
        heading: f32,
        others: &Obstacles,
        public_reserve: bool,
    ) -> (Vec<String>, Json) {
        let mut findings = others.pending.clone();
        let Some(model) = self.models.get(kind) else {
            return (
                vec!["pending: official definition missing".into()],
                json!({"status":"pending","kind":kind}),
            );
        };
        findings.extend(
            model
                .pending
                .iter()
                .map(|pending| format!("pending: {pending}")),
        );
        if !heading.is_finite() {
            findings.push("non-finite heading".into());
            return (findings, json!({"status":"pending","kind":kind}));
        }
        let water = model.water.unwrap_or_else(|| {
            let index = (cell[1].round().clamp(0., (self.frame.height - 1) as f64) as usize)
                * self.frame.width
                + cell[0].round().clamp(0., (self.frame.width - 1) as f64) as usize;
            self.z[index] < self.water
        });
        let mut placement = model.placed(&self.frame, cell, heading, true);
        if public_reserve
            && let Some((_, padding)) = self
                .policy
                .as_ref()
                .and_then(|policy| crate::gameplay::gift_clearance::kind(kind, policy))
        {
            placement.push(square(cell, padding as f64));
        }
        if placement
            .iter()
            .any(|polygon| !self.clear_ground(polygon, water, public_reserve))
        {
            findings.push("placement footprint crosses blocked, nonflat, wrong-surface or out-of-bounds ground".into());
        }
        let collision = model.placed(&self.frame, cell, heading, false);
        if collision
            .iter()
            .any(|polygon| others.polygons.iter().any(|other| overlap(polygon, other)))
        {
            findings
                .push("official collision geometry overlaps another object or gift branch".into());
        }
        let mut exit_evidence = Json::Null;
        if let Some((create, rally)) = model.exit {
            let start = transform(&self.frame, cell, heading, create);
            let end = transform(&self.frame, cell, heading, rally);
            let length = (end[0] - start[0]).hypot(end[1] - start[1]);
            let steps = (length * 2.).ceil().max(1.) as usize;
            let mut last = None;
            let mut clear = true;
            for step in 0..=steps {
                let t = step as f64 / steps as f64;
                let point = [
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ];
                let sweep = square(point, self.exit_clearance);
                if !self.clear_ground(&sweep, water, false)
                    || others.polygons.iter().any(|other| overlap(&sweep, other))
                {
                    clear = false;
                    break;
                }
                if let Some(cells) = self.cells(&sweep) {
                    let (low, high) = cells
                        .into_iter()
                        .map(|cell| self.z[cell])
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| {
                            (lo.min(z), hi.max(z))
                        });
                    if high - low > self.maximum_step {
                        clear = false;
                        break;
                    }
                }
                let x = point[0].round() as usize;
                let y = point[1].round() as usize;
                let z = self.z[y * self.frame.width + x];
                if last.is_some_and(|previous: f64| (z - previous).abs() > self.maximum_step) {
                    clear = false;
                    break;
                }
                last = Some(z);
            }
            if !clear {
                findings.push(
                    "production exit sweep is obstructed or changes surface/height beyond policy"
                        .into(),
                );
            }
            exit_evidence = json!({"create_cell":start,"rally_cell":end,"clearance_cells":self.exit_clearance,"passed":clear,"scope":"static create-to-rally sweep; runtime navigation is not inferred"});
        }
        (
            findings.clone(),
            json!({"kind":model.kind,"source":model.source,"cell":cell,"heading_radians":heading,"placement_polygons":placement,"collision_polygons":collision,"production_exit":exit_evidence,"pending":model.pending,"passed":findings.is_empty()}),
        )
    }
}

/// Audit the final native scene. Frozen and legacy defaults report without changing content.
pub fn audit(p: &Project, map: &MapFile) -> Result<Json> {
    let enforced = enforced(p)?;
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"schema_version":1,"scope":"terrain-only map has no gameplay placement targets","enforced":false,"status":"not_applicable","passed":null,"raw_passed":null,"pending":[],"findings":[],"entries":[],"uncovered_objects":[],"catalog":official::binding()}),
        );
    }
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut targets = Vec::new();
    let mut unresolved = Vec::new();
    let mut classified_markers = Vec::new();
    bindings(
        &nodes,
        &mut targets,
        &mut unresolved,
        &mut classified_markers,
    )?;
    if !objects.iter().any(|object| building(&object.type_name))
        && targets.is_empty()
        && unresolved.is_empty()
    {
        let uncovered=objects.iter().filter(|object|!object.type_name.contains("Waypoint")&&official::object(&object.type_name).is_none()).map(|object|json!({"kind":object.type_name,"status":"unresolved-decoration-or-object"})).collect::<Vec<_>>();
        return Ok(
            json!({"schema_version":1,"scope":"no applicable native buildings or explicit building creation actions","enforced":false,"status":"not_applicable","passed":null,"raw_passed":null,"pending":[],"findings":[],"entries":[],"uncovered_objects":uncovered,"classified_noncolliding_markers":classified_markers,"catalog":official::binding()}),
        );
    }
    let z = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|value| *value as f64 * frame.z_scale)
        .collect::<Vec<_>>();
    let checker = match Checker::from_parts(
        p,
        frame,
        hm.width,
        &z,
        &terrain::flip(&bt.impassability, hm.width),
        &terrain::flip(&bt.buildability, hm.width),
        &nodes,
        &objects,
    ) {
        Ok(checker) => checker,
        Err(error) => {
            return Ok(
                json!({"schema_version":1,"enforced":enforced,"mode":if enforced{"strict"}else{"diagnostic"},"passed":false,"raw_passed":false,"pending":[error.to_string()],"findings":[{"status":"pending","detail":error.to_string()}],"entries":[],"uncovered_objects":[],"catalog":official::binding()}),
            );
        }
    };
    let mut entries = Vec::new();
    let mut findings = Vec::new();
    let mut diagnostic_findings = Vec::new();
    let mut uncovered = Vec::new();
    let repair_policy = if p.cfg["OFFICIAL_PLACEMENT"]["scope"] == "production_repair" {
        Some(PlacementPolicy::load(p)?)
    } else {
        None
    };
    for pending in &checker.script_pending {
        findings.push(json!({"status":"pending","detail":pending}));
    }
    for (index, object) in objects.iter().enumerate() {
        if object.type_name.contains("Waypoint") {
            continue;
        }
        if !building(&object.type_name) {
            if official::object(&object.type_name).is_none() || instance_pending(object).is_some() {
                uncovered.push(json!({"object_id":index+1,"kind":object.type_name,"status":"unresolved-decoration-or-object","instance_pending":instance_pending(object)}));
            }
            continue;
        }
        let others = objects
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, object)| object.clone())
            .collect::<Vec<_>>();
        let obstacles = checker.obstacles(&others);
        let cell = checker
            .frame
            .grid([object.position[0] as f64, object.position[1] as f64]);
        let (mut problems, mut evidence) =
            checker.assess(&object.type_name, cell, object.angle, &obstacles, false);
        if let Some(pending) = instance_pending(object) {
            problems.push(pending);
        }
        evidence["object_id"] = json!(index + 1);
        let targeted = repair_policy
            .as_ref()
            .is_none_or(|policy| policy.footprint(&object.type_name).is_some());
        evidence["enforced_target"] = json!(targeted);
        for detail in problems {
            let issue = json!({"object_id":index+1,"kind":object.type_name,"detail":detail});
            if targeted {
                findings.push(issue);
            } else {
                diagnostic_findings.push(issue);
            }
        }
        entries.push(evidence);
    }
    let mut seen = BTreeSet::new();
    for binding in &checker.bindings {
        if !seen.insert((
            binding.waypoint.clone(),
            binding.kind.clone(),
            binding.seat,
            binding.heading.map(f32::to_bits),
        )) {
            continue;
        }
        let Some(object) = objects.iter().find(|object| {
            prop(&object.props, "waypointName").and_then(Value::text)
                == Some(binding.waypoint.as_str())
        }) else {
            findings.push(json!({"waypoint":binding.waypoint,"detail":"script waypoint is unavailable","status":"pending"}));
            continue;
        };
        let others = objects
            .iter()
            .filter(|other| {
                prop(&other.props, "waypointName").and_then(Value::text)
                    != Some(binding.waypoint.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        let obstacles = checker.obstacles(&others);
        let cell = checker
            .frame
            .grid([object.position[0] as f64, object.position[1] as f64]);
        let heading = match checker.binding_heading(binding, cell, false) {
            Ok(heading) => heading,
            Err(error) => {
                findings.push(json!({"waypoint":binding.waypoint,"kind":binding.kind,"detail":error.to_string(),"status":if binding.heading.is_none(){"pending"}else{"failed"}}));
                continue;
            }
        };
        let (problems, mut evidence) =
            checker.assess(&binding.kind, cell, heading, &obstacles, true);
        evidence["waypoint"] = json!(binding.waypoint);
        evidence["seat"] = json!(binding.seat);
        for detail in problems {
            findings.push(json!({"waypoint":binding.waypoint,"kind":binding.kind,"detail":detail}));
        }
        entries.push(evidence);
    }
    let pending = findings
        .iter()
        .filter(|finding| {
            finding["status"] == "pending"
                || finding["detail"]
                    .as_str()
                    .is_some_and(|detail| detail.starts_with("pending:"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let passed = findings.is_empty();
    Ok(
        json!({"schema_version":1,"scope":"official building placement/collision and explicit script creation exits; public reserves remain separate; unresolved decorations are uncovered rather than qualified","strict_scope":if p.cfg["OFFICIAL_PLACEMENT"]["scope"]=="production_repair"{"factories/naval yards and explicit building creation actions"}else{"all applicable native buildings and explicit building creation actions"},"enforced":enforced,"mode":if enforced{"strict"}else{"diagnostic"},"passed":passed,"raw_passed":passed&&diagnostic_findings.is_empty(),"pending":pending,"findings":findings,"diagnostic_findings":diagnostic_findings,"entries":entries,"uncovered_objects":uncovered,"classified_noncolliding_markers":checker.classified_markers,"coverage_complete":uncovered.is_empty()&&pending.is_empty(),"catalog":official::binding(),"ore_refinery_evidence":refinery_evidence(p),"runtime_navigation":"not inferred","bone_positions":"not inferred from bone names"}),
    )
}

pub fn require_passed(report: &Json) -> Result<()> {
    if report["enforced"] == true && report["passed"] != true {
        return Err(err(format!(
            "official placement failed: {} findings; see official-placement.json",
            report["findings"].as_array().map_or(0, Vec::len)
        )));
    }
    Ok(())
}

fn refinery_evidence(p: &Project) -> Json {
    let official_objects=["OreNode","AlliedRefinery","SovietRefinery","JapanRefinery"].iter().map(|kind|match official::object(kind){
        Some(object)=>json!({"kind":kind,"source":object.source,"geometry":object.shapes,"placement_shape":object.placement_shape,"assisted_placement":object.assisted_placement,"restricted_area":object.restricted_area,"pending":object.pending}),
        None=>json!({"kind":kind,"status":"pending","detail":"official definition unavailable"})}).collect::<Vec<_>>();
    json!({"scope":"read-only official offsets/restricted regions/three-faction geometry and separate project reservation; no dock-bone coordinates inferred","official":official_objects,"project_reservation":terrain::ramp_clearance::policy(p).ok().flatten()})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_test_blend(map: &mut MapFile, hm: &HeightMap, bt: &BlendTiles) {
        let area = hm.width * hm.height;
        let surface = terrain::surface::Surface {
            chosen: vec![0; area],
            impassable: terrain::flip(&bt.impassability, hm.width),
            build_block: terrain::flip(&bt.buildability, hm.width),
            layer_edge: vec![false; area],
            shore: vec![false; area],
            textures: bt
                .textures
                .iter()
                .map(|name| [name.clone(), String::new(), String::new()])
                .collect(),
        };
        map.get_mut("BlendTileData").unwrap().payload = surface.encode(hm.width, &[]).unwrap();
        map.get_mut("BlendTileData").unwrap().version = 27;
    }

    fn hospital_scene(headings: &[f32]) -> (Project, MapFile, [f64; 2]) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "海南").unwrap();
        let mut map = MapFile::load(root.join("templates/8p_template.map")).unwrap();
        let mut hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
        p.cfg["GRID"] = json!(hm.width);
        p.cfg["GRID_HEIGHT"] = json!(hm.height);
        p.cfg["BORDER"] = json!(hm.border);
        p.cfg["CELL"] = json!(10.);
        p.cfg["Z_SCALE"] = json!(1.);
        p.cfg["BASES"] = Json::Null;
        p.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":"strict"});
        let frame = Frame::from_heightmap(&p, &hm).unwrap();
        let center = [(hm.width / 2) as f64, (hm.height / 2) as f64];
        hm.elevations.fill(212);
        map.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        let mut bt =
            BlendTiles::parse(map.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
        bt.impassability.fill(false);
        bt.buildability.fill(false);
        save_test_blend(&mut map, &hm, &bt);
        let mut waypoint = map::objects(map.get("ObjectsList").unwrap(), &map.table)
            .unwrap()
            .into_iter()
            .find(|object| object.type_name.contains("Waypoint"))
            .unwrap();
        let world = frame.world(center);
        waypoint.position = [world[0] as f32, world[1] as f32, 0.];
        map::set(
            &mut waypoint.props,
            "waypointName",
            Value::Ascii("hospital-site".into()),
        );
        map.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&[waypoint], &map).unwrap();
        let actions = headings
            .iter()
            .map(|heading| {
                scripts::leaf(
                    "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
                    467,
                    &[
                        (15, 0, 0., "HospitalTechStructure"),
                        (3, 0, 0., "Player_1/teamPlayer_1"),
                        (7, 0, 0., "hospital-site"),
                        (17, 0, *heading, ""),
                    ],
                    false,
                    false,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        map.get_mut("PlayerScriptsList").unwrap().payload =
            scripts::encode_nodes(&actions, &mut map).unwrap();
        (p, map, center)
    }

    #[test]
    fn final_audit_checks_distinct_conditional_headings_at_the_same_waypoint() {
        let (p, mut map, center) = hospital_scene(&[45_f32.to_radians(), 135_f32.to_radians()]);
        let hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
        let mut bt =
            BlendTiles::parse(map.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
        // The second official hospital wing reaches this cell at 135 degrees,
        // while neither wing reaches it at the first branch's 45 degrees.
        let (x, y) = (center[0] as usize + 9, center[1] as usize + 4);
        bt.buildability[(hm.height - 1 - y) * hm.width + x] = true;
        save_test_blend(&mut map, &hm, &bt);
        let report = audit(&p, &map).unwrap();
        let entries = report["entries"].as_array().unwrap();
        assert_eq!(
            entries.len(),
            2,
            "both explicit conditional poses must be retained"
        );
        assert_eq!(entries[0]["passed"], true);
        assert_eq!(entries[1]["passed"], false);
        assert!(
            report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["detail"]
                    .as_str()
                    .is_some_and(|detail| detail.contains("placement footprint")))
        );
        assert!(require_passed(&report).is_err());
    }

    #[test]
    fn nongift_obstacles_use_explicit_headings_and_unresolved_angles_fail_closed() {
        let (p, map, center) = hospital_scene(&[135_f32.to_radians()]);
        let hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
        let frame = Frame::from_heightmap(&p, &hm).unwrap();
        let objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
        let nodes =
            scripts::parse_nodes(&map.get("PlayerScriptsList").unwrap().payload, &map.table)
                .unwrap();
        let mut checker = Checker::from_parts(
            &p,
            frame.clone(),
            hm.width,
            &vec![212.; hm.width * hm.height],
            &vec![false; hm.width * hm.height],
            &vec![false; hm.width * hm.height],
            &nodes,
            &objects,
        )
        .unwrap();
        assert!(
            checker.facing.is_none(),
            "a scripted hospital has no opposing-team facing rule"
        );
        let probe = square([center[0] + 9., center[1] + 4.], 0.1);
        for obstacles in [
            checker.obstacles(&objects),
            checker.planned_obstacles(&objects),
        ] {
            assert!(obstacles.pending.is_empty());
            assert!(
                obstacles
                    .polygons
                    .iter()
                    .any(|polygon| overlap(polygon, &probe))
            );
        }
        let model = checker.models.get("HospitalTechStructure").unwrap();
        assert!(
            !model
                .placed(&frame, center, 45_f32.to_radians(), false)
                .iter()
                .any(|polygon| overlap(polygon, &probe))
        );
        checker.bindings[0].heading = None;
        let absent = checker.planned_obstacles(&objects);
        assert!(absent.polygons.is_empty());
        assert!(!absent.pending.is_empty());
        checker.bindings[0].heading = Some(0.);
        let illegal = checker.planned_obstacles(&objects);
        assert!(illegal.polygons.is_empty());
        assert!(
            illegal
                .pending
                .iter()
                .any(|pending| pending.contains("illegal explicit"))
        );
    }

    #[test]
    fn structural_props_are_obstacles_without_becoming_building_placement_targets() {
        let rock = official::object("HA_GardenRock01").unwrap();
        assert!(rock.kind_of.iter().any(|flag| flag == "STRUCTURE"));
        assert!(rock.kind_of.iter().any(|flag| flag == "SIMPLE_OBJECT_PROP"));
        assert!(!building("HA_GardenRock01"));
        assert!(building("HospitalTechStructure"));
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "海南").unwrap();
        for malformed in [json!(true), json!(false), json!(1), Json::Null] {
            p.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":malformed});
            assert!(enforced(&p).is_err());
        }
    }

    #[test]
    fn noncolliding_script_beacons_do_not_poison_building_candidates() {
        let (p, mut map, _) = hospital_scene(&[45_f32.to_radians()]);
        assert!(
            official::object("multiplayerbeacon").is_none(),
            "ambiguous complete definitions are not chosen by filename"
        );
        assert!(official::certainly_noncolliding_marker("multiplayerbeacon"));
        fn beacon(nodes: &[Node]) -> Option<Node> {
            for node in nodes {
                if let Some(children) = &node.children {
                    if let Some(found) = beacon(children) {
                        return Some(found);
                    }
                } else if node.command.as_deref() == Some("CREATE_NAMED_ON_TEAM_AT_WAYPOINT")
                    && scripts::string_arguments(node)
                        .unwrap()
                        .iter()
                        .any(|(kind, name)| {
                            *kind == 15 && name.eq_ignore_ascii_case("multiplayerbeacon")
                        })
                {
                    return Some(node.clone());
                }
            }
            None
        }
        let mut nodes =
            scripts::parse_nodes(&map.get("PlayerScriptsList").unwrap().payload, &map.table)
                .unwrap();
        nodes.push(
            beacon(&scripts::load_rules(&p).unwrap().scripts)
                .expect("real common script contains its unheaded beacon creation"),
        );
        map.get_mut("PlayerScriptsList").unwrap().payload =
            scripts::encode_nodes(&nodes, &mut map).unwrap();
        let report = audit(&p, &map).unwrap();
        assert_eq!(report["entries"].as_array().unwrap().len(), 1);
        assert_eq!(
            report["passed"], true,
            "noncolliding markers are not unknown building obstacles: {report}"
        );
        assert!(report["pending"].as_array().unwrap().is_empty());
        let markers = report["classified_noncolliding_markers"]
            .as_array()
            .unwrap();
        assert_eq!(markers.len(), 1);
        assert_eq!(
            markers[0]["classification"]["equivalent_sources"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    fn checker(water: bool) -> Checker {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "海南").unwrap();
        p.cfg["GRID"] = json!(128);
        p.cfg["GRID_HEIGHT"] = json!(128);
        p.cfg["BORDER"] = json!(4);
        p.cfg["CELL"] = json!(10.);
        p.cfg["Z_SCALE"] = json!(1.);
        p.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":"strict"});
        let z = vec![if water { 130. } else { 212. }; 128 * 128];
        let mut checker = Checker::from_parts(
            &p,
            Frame::from_project(&p),
            128,
            &z,
            &vec![false; 128 * 128],
            &vec![false; 128 * 128],
            &[],
            &[],
        )
        .unwrap();
        for kind in [
            "SovietNavalYard",
            "AlliedWarFactory",
            "HospitalTechStructure",
        ] {
            let model = Model::load(kind);
            assert!(model.pending.is_empty(), "{kind}: {:?}", model.pending);
            checker.models.insert(kind.into(), model);
        }
        checker
    }

    #[test]
    fn rotated_official_shipyard_footprint_detects_ground_beyond_the_old_square() {
        let mut checker = checker(true);
        let clear = Obstacles::new(Vec::new());
        let angle = 45_f32.to_radians();
        assert!(
            checker
                .assess("SovietNavalYard", [64., 64.], angle, &clear, true)
                .0
                .is_empty()
        );
        // This cell is beyond the former eight-cell square but in the rotated
        // official 90-by-45 world-unit placement box.
        checker.blocked[61 * 128 + 73] = true;
        let (findings, _) = checker.assess("SovietNavalYard", [64., 64.], angle, &clear, true);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("placement footprint"))
        );
        assert!(
            checker
                .assess(
                    "SovietNavalYard",
                    [64., 64.],
                    135_f32.to_radians(),
                    &clear,
                    true
                )
                .0
                .is_empty()
        );
    }

    #[test]
    fn factory_exit_obstruction_is_checked_outside_the_rotated_foundation() {
        let mut checker = checker(false);
        let angle = 45_f32.to_radians();
        let clear = Obstacles::new(Vec::new());
        assert!(
            checker
                .assess("AlliedWarFactory", [64., 64.], angle, &clear, true)
                .0
                .is_empty()
        );
        // Allied create(-20,20) -> rally(80,20), rotated to its final heading.
        // The obstruction is outside the 60-by-60 foundation and the old reserve.
        checker.imp[56 * 128 + 68] = true;
        let (findings, evidence) =
            checker.assess("AlliedWarFactory", [64., 64.], angle, &clear, true);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("exit sweep"))
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.contains("placement footprint"))
        );
        assert_eq!(evidence["production_exit"]["passed"], false);
    }

    #[test]
    fn secondary_official_collision_shape_is_not_reduced_to_the_first_box() {
        let checker = checker(false);
        let secondary = Obstacles::new(vec![square([55., 69.], 0.2)]);
        let (findings, _) =
            checker.assess("HospitalTechStructure", [64., 64.], 0., &secondary, false);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains("collision geometry"))
        );
    }

    #[test]
    fn every_conditional_faction_and_fallback_uses_the_same_final_heading() {
        let mut checker = checker(false);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = Project::load(root, "海南").unwrap();
        let map = MapFile::load(root.join("templates/8p_template.map")).unwrap();
        let objects = map::objects(map.get("ObjectsList").unwrap(), &map.table).unwrap();
        checker.facing = Some(production_facing::Context::from_objects(&p, &objects).unwrap());
        checker.policy = Some(PlacementPolicy::load(&p).unwrap());
        checker.bindings = ["AlliedWarFactory", "SovietWarFactory", "JapanWarFactory"]
            .iter()
            .map(|kind| Binding {
                waypoint: "dual-p1-war-factory".into(),
                kind: (*kind).into(),
                seat: Some(1),
                heading: None,
            })
            .collect();
        for kind in ["SovietWarFactory", "JapanWarFactory"] {
            checker.models.insert(kind.into(), Model::load(kind));
        }
        // A larger, otherwise valid branch must participate even when the
        // first faction's building would fit at this exact point.
        checker.models.get_mut("JapanWarFactory").unwrap().placement =
            vec![vec![[-140., -30.], [140., -30.], [140., 30.], [-140., 30.]]];
        let obstacles = Obstacles::new(Vec::new());
        assert!(checker.acceptable("dual-p1-war-factory", [64, 64], &obstacles));
        assert!(checker.acceptable("gift-alt-dual-p1-war-factory-1", [64, 64], &obstacles));
        let heading = checker
            .facing
            .as_ref()
            .unwrap()
            .candidate_heading(&checker.frame, 1, [64., 64.])
            .unwrap();
        let cell = world_offset(&checker.frame, [64., 64.], heading, [120., 0., 0.]);
        checker.blocked[cell[1].round() as usize * 128 + cell[0].round() as usize] = true;
        assert!(
            checker
                .assess("AlliedWarFactory", [64., 64.], heading, &obstacles, true)
                .0
                .is_empty()
        );
        assert!(!checker.acceptable("dual-p1-war-factory", [64, 64], &obstacles));
        assert!(!checker.acceptable("gift-alt-dual-p1-war-factory-1", [64, 64], &obstacles));
    }

    #[test]
    fn unknown_building_geometry_is_pending_and_compatibility_mode_is_explicit() {
        let checker = checker(false);
        assert!(
            !checker
                .assess(
                    "UnknownFactory",
                    [64., 64.],
                    45_f32.to_radians(),
                    &Obstacles::new(Vec::new()),
                    true
                )
                .0
                .is_empty()
        );
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "海南").unwrap();
        assert!(!enforced(&p).unwrap());
        p.cfg["TEXTURE_PROFILE"] = json!("legacy");
        p.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":"strict"});
        assert!(enforced(&p).unwrap());
        assert!(
            require_passed(&json!({"enforced":false,"passed":false,"findings":["inherited"]}))
                .is_ok()
        );
        assert!(
            require_passed(&json!({"enforced":true,"passed":false,"findings":["unknown"]}))
                .is_err()
        );
    }

    #[test]
    fn terrain_only_audit_and_binding_do_not_require_rules_or_gameplay_assets() {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "海南").unwrap();
        let map = MapFile {
            order: Vec::new(),
            table: BTreeMap::new(),
            chunks: Vec::new(),
        };
        p.cfg["TERRAIN_ONLY"] = json!(true);
        p.cfg["BASES"] = Json::Null;
        p.root = std::env::temp_dir().join("ra3-official-placement-absent-root");
        assert_eq!(binding(&p).unwrap()["status"], "not_applicable");
        let report = audit(&p, &map).unwrap();
        assert_eq!(report["status"], "not_applicable");
        require_passed(&report).unwrap();
    }

    #[test]
    fn scene_without_buildings_or_creation_actions_needs_no_facing_teams() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "海南").unwrap();
        let mut map = MapFile::load(root.join("templates/8p_template.map")).unwrap();
        map.get_mut("ObjectsList").unwrap().payload = map::write_objects(&[], &map).unwrap();
        map.get_mut("PlayerScriptsList").unwrap().payload =
            scripts::encode_nodes(&[], &mut map).unwrap();
        p.cfg["BASES"] = Json::Null;
        p.root = std::env::temp_dir().join("ra3-official-placement-absent-root");
        let report = audit(&p, &map).unwrap();
        assert_eq!(report["status"], "not_applicable");
        require_passed(&report).unwrap();
    }
}
