//! Shared planning facts and read-only candidates. Final geometry remains owned
//! by the existing spawn, ramp, official-placement and gameplay validators.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile},
    pipeline::{frozen_source::FrozenSource, project::Project},
    terrain::{
        Layout, Terrain, ramp_clearance, resource_routes, spawn_placement, surface::Surface,
    },
};
pub use resource_routes::{Plan, Route, Slot};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct PlanningContext {
    pub frame: Frame,
    pub spawn: spawn_placement::Policy,
    pub resource: resource_routes::Policy,
    pub ramp: ramp_clearance::Policy,
    pub plan: Option<Plan>,
}
impl PlanningContext {
    pub fn load(p: &Project, layout: &Layout, native: Option<&MapFile>) -> Result<Self> {
        let frame = match native {
            Some(m) => Frame::from_heightmap(p, &HeightMap::parse(m.get("HeightMapData")?)?)?,
            None => Frame::from_project(p),
        };
        frame.validate()?;
        if layout
            .base_cells
            .iter()
            .chain(&layout.ore_cells)
            .any(|pt| pt.iter().any(|v| !v.is_finite()) || !frame.inside(*pt))
        {
            return Err(err(
                "planning layout point is non-finite or outside its Frame",
            ));
        }
        Ok(Self {
            frame,
            spawn: spawn_placement::policy(p)?,
            resource: resource_routes::policy(p)?,
            ramp: ramp_clearance::policy(p)?
                .ok_or_else(|| err("planning requires ramp/refinery policy"))?,
            plan: Plan::load(p)?,
        })
    }
}

/// Explicit team membership; a route's authored A/B direction selects the order.
/// Missing BASES may be recovered from complete declared route sides, never from
/// seat-number halves or geographical guesses.
pub fn team_sides(p: &Project, seat_count: usize, plan: Option<&Plan>) -> Result<[Vec<usize>; 2]> {
    if seat_count == 0 {
        return Ok([Vec::new(), Vec::new()]);
    }
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    if let Some(bases) = p.cfg["BASES"]
        .as_array()
        .filter(|bases| bases.len() == seat_count)
    {
        for (i, base) in bases.iter().enumerate() {
            let team = base
                .get(3)
                .and_then(Value::as_str)
                .filter(|team| !team.trim().is_empty())
                .ok_or_else(|| err("planning BASES entries require explicit team identities"))?;
            groups.entry(team.to_owned()).or_default().push(i + 1);
        }
        if groups.len() != 2 || groups.values().any(|group| group.len() * 2 != seat_count) {
            return Err(err(
                "planning requires two equally sized explicitly declared teams",
            ));
        }
        let anchor = plan
            .and_then(|plan| plan.routes.first())
            .map_or(1, |route| route.side_a_seat);
        let mut values = groups.into_values().collect::<Vec<_>>();
        if values[1].contains(&anchor) {
            values.swap(0, 1);
        }
        if !values[0].contains(&anchor) {
            return Err(err("route A seat has no declared team"));
        }
        return Ok([values.remove(0), values.remove(0)]);
    }
    let plan =
        plan.ok_or_else(|| err("planning teams require BASES or a complete explicit route plan"))?;
    let a = plan
        .routes
        .iter()
        .map(|r| r.side_a_seat)
        .collect::<BTreeSet<_>>();
    let b = plan
        .routes
        .iter()
        .map(|r| r.side_b_seat)
        .collect::<BTreeSet<_>>();
    if a.len() * 2 != seat_count
        || b.len() != a.len()
        || !a.is_disjoint(&b)
        || a.union(&b).copied().collect::<BTreeSet<_>>() != (1..=seat_count).collect()
    {
        return Err(err(
            "resource routes do not declare complete disjoint team sides",
        ));
    }
    Ok([a.into_iter().collect(), b.into_iter().collect()])
}

/// Build an in-memory candidate from selected current inputs or CRC-bound native
/// source. This never reads the generated terrain cache for a new map and never
/// relocates or invents script waypoints.
pub fn native_scene(p: &Project, layout: &Layout) -> Result<MapFile> {
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        let (files, source_layout) = FrozenSource::load(p)?.into_parts()?;
        let bytes = files
            .iter()
            .find(|(name, _)| name.ends_with(".map"))
            .ok_or_else(|| err("frozen planning source has no native map"))?;
        let mut m = MapFile::load_bytes(&bytes.1)?;
        let mut objects = map::objects(m.get("ObjectsList")?, &m.table)?;
        let before_objects = objects.clone();
        if source_layout.objects.len() != layout.objects.len()
            || objects.len() != layout.objects.len()
        {
            return Err(err(
                "planning source and declared layout object identities differ",
            ));
        }
        let frame = Frame::from_heightmap(p, &HeightMap::parse(m.get("HeightMapData")?)?)?;
        for ((object, declared), original) in objects
            .iter_mut()
            .zip(&layout.objects)
            .zip(&source_layout.objects)
        {
            if object.type_name != declared.type_name
                || original.type_name != declared.type_name
                || object.props != declared.props
                || object.version != declared.version
                || object.road_type != declared.road_type
            {
                return Err(err(
                    "frozen planning can only change declared object positions and headings",
                ));
            }
            let world = frame.world(declared.cell);
            object.position[0] = world[0] as f32;
            object.position[1] = world[1] as f32;
            object.angle = declared.angle;
        }
        m.get_mut("ObjectsList")?.payload = map::write_objects(&objects, &m)?;
        crate::terrain::native_cell_edits::apply(p, &mut m)?;
        crate::gameplay::gift_clearance::sync_moved_areas(&mut m, &before_objects, &objects)?;
        if before_objects.iter().zip(&objects).any(|(before, after)| {
            before.type_name.contains("Waypoint") && before.position[..2] != after.position[..2]
        }) {
            crate::gameplay::production_facing::apply(p, &mut m)?;
        }
        return Ok(m);
    }
    let t = Terrain::generate(p)?;
    let surface = Surface::generate(&t, p)?;
    let template = p.string("TEMPLATE", "8p_template.map");
    crate::pipeline::project::safe_component(template)?;
    let mut m = MapFile::load(p.root.join("templates").join(template))?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let blend = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
    if !p.flag("TERRAIN_ONLY") {
        crate::gameplay::scripts::apply(&mut m, p)?;
    }
    let objects = layout
        .objects
        .iter()
        .map(|o| o.object(p))
        .collect::<Vec<_>>();
    for object in &objects {
        for property in &object.props {
            m.id(&property.key)?;
        }
    }
    m.get_mut("HeightMapData")?.payload = t.heightmap(p).encode()?;
    m.get_mut("HeightMapData")?.version = 6;
    m.get_mut("BlendTileData")?.payload = surface.encode(t.n, &blend.blend_descriptions)?;
    m.get_mut("BlendTileData")?.version = 27;
    m.get_mut("ObjectsList")?.payload = map::write_objects(&objects, &m)?;
    Ok(m)
}

// Classification mirrors have coordinates rather than IDs. Resolve each group
// against immutable native identities before applying the batch. Type-specific
// matching distinguishes e.g. an oil derrick and a hospital sharing one point.
fn neutral_mirror_matches(group: usize, kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    match group {
        0 => kind == "oilderrick",
        1 => {
            kind.contains("house")
                || kind.contains("civilianbuilding")
                || kind.starts_with("ei_structure_")
        }
        2 => kind.contains("observationpost") || kind.contains("outpost"),
        3 => kind.contains("garage"),
        4 => kind.contains("airport") || kind.contains("airfield"),
        5 => kind.contains("hospital"),
        6 => kind.contains("veterancy") || kind.contains("academy"),
        7 => kind.contains("shipyard") || kind.contains("navalyard"),
        8 => kind.contains("bridge"),
        _ => false,
    }
}
fn neutral_mirrors(layout: &Layout) -> [&[[f64; 2]]; 9] {
    [
        &layout.derrick_cells,
        &layout.house_cells,
        &layout.post_cells,
        &layout.garage_cells,
        &layout.airport_cells,
        &layout.hospital_cells,
        &layout.academy_cells,
        &layout.shipyard_cells,
        &layout.bridge_cells,
    ]
}
fn neutral_mirror_updates(
    frame: &Frame,
    layout: &Layout,
    objects: &[map::MapObject],
    destinations: &BTreeMap<usize, [f64; 2]>,
) -> Result<Vec<(usize, usize, [f64; 2])>> {
    let mut updates = Vec::new();
    for (group, points) in neutral_mirrors(layout).into_iter().enumerate() {
        for (slot, point) in points.iter().enumerate() {
            let colocated = objects
                .iter()
                .enumerate()
                .filter(|(_, object)| {
                    if object.type_name.contains("Waypoint") || object.type_name == "OreNode" {
                        return false;
                    }
                    let old = frame.grid([object.position[0] as f64, object.position[1] as f64]);
                    (old[0] - point[0]).hypot(old[1] - point[1]) < 0.001
                })
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            if !colocated.iter().any(|id| destinations.contains_key(id)) {
                continue;
            }
            let matching = colocated
                .iter()
                .copied()
                .filter(|id| neutral_mirror_matches(group, &objects[*id].type_name))
                .collect::<Vec<_>>();
            let object = match matching.as_slice() {
                [id] => *id,
                [] if colocated.len() == 1 => colocated[0],
                _ => {
                    return Err(err(format!(
                        "ambiguous neutral classification group {group} at ({}, {}); native identities cannot be inferred from a shared coordinate",
                        point[0], point[1]
                    )));
                }
            };
            if let Some(target) = destinations.get(&object) {
                updates.push((group, slot, *target));
            }
        }
    }
    Ok(updates)
}
fn set_neutral_mirror(layout: &mut Layout, group: usize, slot: usize, cell: [f64; 2]) {
    let fields = [
        &mut layout.derrick_cells,
        &mut layout.house_cells,
        &mut layout.post_cells,
        &mut layout.garage_cells,
        &mut layout.airport_cells,
        &mut layout.hospital_cells,
        &mut layout.academy_cells,
        &mut layout.shipyard_cells,
        &mut layout.bridge_cells,
    ];
    fields
        .into_iter()
        .nth(group)
        .expect("resolved classification group")[slot] = cell;
}

/// Apply a complete candidate assignment against immutable original identities.
/// IDs are one-based and headings use native/world radians. Ore mirror indices
/// are resolved before any edit, so swaps cannot be confused by an intermediate
/// duplicate coordinate. Scripts/guards are synchronized once for the final state.
pub fn candidate_batch(
    p: &Project,
    map: &MapFile,
    layout: &Layout,
    edits: &[(usize, [f64; 2], f32)],
) -> Result<(MapFile, Layout)> {
    if edits.is_empty() {
        return Ok((map.clone(), layout.clone()));
    }
    let mut m = map.clone();
    let mut next = layout.clone();
    let frame = Frame::from_heightmap(p, &HeightMap::parse(map.get("HeightMapData")?)?)?;
    let before_objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    if before_objects.len() != layout.objects.len() {
        return Err(err(
            "batch candidate requires the complete original native/layout identity table",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut resolved = Vec::with_capacity(edits.len());
    for &(object_id, cell, angle) in edits {
        if !seen.insert(object_id) {
            return Err(err(
                "batch candidate has duplicate edits for one native object ID",
            ));
        }
        if !frame.playable(cell) || !cell.iter().all(|v| v.is_finite()) || !angle.is_finite() {
            return Err(err(
                "planning candidate is outside the playable Frame or non-finite",
            ));
        }
        let index = object_id
            .checked_sub(1)
            .ok_or_else(|| err("native object ID is one-based"))?;
        let object = before_objects
            .get(index)
            .ok_or_else(|| err("planning candidate object ID is missing"))?;
        let declared = layout
            .objects
            .get(index)
            .ok_or_else(|| err("planning candidate layout object is missing"))?;
        if declared.type_name != object.type_name
            || declared.props != object.props
            || declared.version != object.version
            || declared.road_type != object.road_type
        {
            return Err(err(
                "planning candidate object identity differs from its layout",
            ));
        }
        let ore_index = if object.type_name == "OreNode" {
            let old = frame.grid([object.position[0] as f64, object.position[1] as f64]);
            let found = layout
                .ore_cells
                .iter()
                .enumerate()
                .filter(|(_, pt)| (pt[0] - old[0]).hypot(pt[1] - old[1]) < 0.001)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if found.len() != 1 || layout.ore_cells.len() != layout.ore_angles.len() {
                return Err(err(
                    "candidate ore identity must resolve exactly one original declarative ore",
                ));
            }
            Some(found[0])
        } else {
            None
        };
        let base_index = map::prop(&object.props, "waypointName")
            .and_then(map::Value::text)
            .and_then(|name| name.strip_prefix("Player_"))
            .and_then(|s| s.split_once("_Start"))
            .and_then(|(s, _)| s.parse::<usize>().ok())
            .map(|seat| {
                seat.checked_sub(1)
                    .filter(|i| *i < layout.base_cells.len())
                    .ok_or_else(|| err("candidate start has no declared base"))
            })
            .transpose()?;
        resolved.push((index, ore_index, base_index, cell, angle));
    }
    let destinations = resolved
        .iter()
        .map(|(index, _, _, cell, _)| (*index, *cell))
        .collect::<BTreeMap<_, _>>();
    let neutral_updates = neutral_mirror_updates(&frame, layout, &before_objects, &destinations)?;
    for (group, slot, cell) in neutral_updates {
        set_neutral_mirror(&mut next, group, slot, cell);
    }
    let mut objects = before_objects.clone();
    for (index, ore_index, base_index, cell, angle) in resolved {
        if let Some(i) = ore_index {
            next.ore_cells[i] = cell;
            next.ore_angles[i] = -(angle as f64);
        }
        if let Some(i) = base_index {
            next.base_cells[i] = cell;
        }
        let world = frame.world(cell);
        objects[index].position[0] = world[0] as f32;
        objects[index].position[1] = world[1] as f32;
        objects[index].angle = angle;
        next.objects[index].cell = cell;
        next.objects[index].angle = angle;
    }
    m.get_mut("ObjectsList")?.payload = map::write_objects(&objects, &m)?;
    crate::gameplay::gift_clearance::sync_moved_areas(&mut m, &before_objects, &objects)?;
    if before_objects.iter().zip(&objects).any(|(before, after)| {
        before.type_name.contains("Waypoint") && before.position[..2] != after.position[..2]
    }) {
        crate::gameplay::production_facing::apply(p, &mut m)?;
    }
    Ok((m, next))
}

/// Single-object convenience wrapper around the same atomic batch operation.
pub fn candidate(
    p: &Project,
    map: &MapFile,
    layout: &Layout,
    object_id: usize,
    cell: [f64; 2],
    angle: f32,
) -> Result<(MapFile, Layout)> {
    candidate_batch(p, map, layout, &[(object_id, cell, angle)])
}

pub fn check_placement(
    p: &Project,
    map: &MapFile,
    layout: &Layout,
    object_id: usize,
    cell: [f64; 2],
    angle: f32,
) -> Result<Value> {
    let (native, next) = candidate(p, map, layout, object_id, cell, angle)?;
    crate::terrain::preflight::check_native(p, &native, &next)
}
