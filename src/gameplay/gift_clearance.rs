//! 为赠送建筑保留主备落点，生成占用检查和重试保护。
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{self, MapFile, MapObject, Value, prop},
    gameplay::script_spawns::{keepouts, to_cell},
    gameplay::scripts::{self, Node},
    pipeline::project::Project,
    terrain,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};

const HELPER: &str = "gift-clear-";
const ALT: &str = "gift-alt-";
const AREA: &str = "gift-area-";
#[derive(Clone, Deserialize)]
struct Policy {
    avoid_existing_buildings: bool,
    avoid_gifted_buildings: bool,
    runtime_occupancy_check: bool,
    fallback_slots: usize,
    clearance: usize,
    runtime_blocker_radius: usize,
    search_radius: usize,
}
fn policy(p: &Project) -> Result<Policy> {
    #[derive(Deserialize)]
    struct Root {
        production_placement: Policy,
    }
    let x: Root = serde_json::from_slice(&std::fs::read(
        p.root.join("templates/rules/common-T0.5.1.json"),
    )?)?;
    let q = x.production_placement;
    if !q.avoid_existing_buildings
        || !q.avoid_gifted_buildings
        || !q.runtime_occupancy_check
        || !(2..=4).contains(&q.fallback_slots)
        || !(2..=8).contains(&q.clearance)
        || !(16..=32).contains(&q.runtime_blocker_radius)
        || !(80..=400).contains(&q.search_radius)
    {
        return Err(err("invalid public gift collision policy"));
    }
    Ok(q)
}

pub(crate) fn search_radius(p: &Project, seat: usize) -> Result<usize> {
    let q = policy(p)?;
    Ok(p.cfg["SCRIPT_SPAWN_RADIUS_BY_SEAT"][seat.to_string()]
        .as_u64()
        .map(|v| v as usize)
        .unwrap_or(q.search_radius)
        .max(q.search_radius))
}
#[derive(Clone, Debug, Serialize)]
struct Spec {
    name: String,
    wet: bool,
    padding: usize,
    seat: usize,
}
pub(crate) fn kind(
    name: &str,
    production: &crate::gameplay::production_spawns::PlacementPolicy,
) -> Option<(bool, usize)> {
    if let Some(footprint) = production.footprint(name) {
        return Some(footprint);
    }
    let s = name.to_ascii_lowercase();
    if s.ends_with("airfield") {
        Some((false, 8))
    } else if s.ends_with("barracks") || s.contains("superweapon") {
        Some((false, 6))
    } else if s.ends_with("powerplant") {
        Some((false, 4))
    } else {
        None
    }
}
fn name(o: &MapObject) -> &str {
    prop(&o.props, "waypointName")
        .and_then(Value::text)
        .unwrap_or("")
}
fn specs(p: &Project, nodes: &[Node], objects: &[MapObject]) -> Result<Vec<Spec>> {
    let rules = scripts::load_rules(p)?;
    let production = crate::gameplay::production_spawns::PlacementPolicy::load(p)?;
    let bases = terrain::Layout::load(p)?.base_cells;
    let mut out = BTreeMap::<String, Spec>::new();
    fn visit(
        p: &Project,
        nodes: &[Node],
        objects: &[MapObject],
        rules: &scripts::Rules,
        production: &crate::gameplay::production_spawns::PlacementPolicy,
        bases: &[[f64; 2]],
        out: &mut BTreeMap<String, Spec>,
    ) -> Result<()> {
        for n in nodes {
            if let Some(c) = &n.children {
                visit(p, c, objects, rules, production, bases, out)?;
            } else if n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(n)?;
                let Some((wet, padding)) = args
                    .iter()
                    .find_map(|(t, s)| if *t == 15 { kind(s, production) } else { None })
                else {
                    continue;
                };
                let Some((_, wp)) = args.iter().find(|(t, _)| *t == 7) else {
                    continue;
                };
                if !wp.starts_with("dual-") && !rules.spawn_waypoints.iter().any(|s| s.name == *wp)
                {
                    continue;
                }
                let o = objects
                    .iter()
                    .find(|o| name(o) == wp)
                    .ok_or_else(|| err(format!("missing gift waypoint {wp}")))?;
                let c = to_cell(p, o);
                let seat = rules
                    .spawn_waypoints
                    .iter()
                    .find(|s| s.name == *wp)
                    .map(|s| s.seat)
                    .or_else(|| {
                        wp.strip_prefix("dual-p")
                            .and_then(|s| s.split('-').next()?.parse().ok())
                    })
                    .or_else(|| {
                        if wp.starts_with("dual-civilian-") {
                            Some(7)
                        } else if wp.starts_with("dual-creeps-") {
                            Some(8)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        bases
                            .iter()
                            .enumerate()
                            .min_by(|a, b| {
                                ((a.1[0] - c[0]).hypot(a.1[1] - c[1]))
                                    .total_cmp(&((b.1[0] - c[0]).hypot(b.1[1] - c[1])))
                            })
                            .unwrap()
                            .0
                            + 1
                    });
                let s = out.entry(wp.clone()).or_insert(Spec {
                    name: wp.clone(),
                    wet,
                    padding,
                    seat,
                });
                if s.wet != wet {
                    return Err(err("gift waypoint mixes water and land buildings"));
                }
                s.padding = s.padding.max(padding);
            }
        }
        Ok(())
    }
    visit(p, nodes, objects, &rules, &production, &bases, &mut out)?;
    Ok(out.into_values().collect())
}
fn point(p: &Project, o: &MapObject) -> [usize; 2] {
    to_cell(p, o).map(|v| v.round_ties_even() as usize)
}
pub(crate) struct Reservations {
    pub guarded: bool,
    pub footprints: Vec<([usize; 2], usize)>,
}
/// Preserve the complete primary and fallback foundations during terrain edits.
/// Radii come from the same actual create actions as the gift acceptance check.
pub(crate) fn reservations(p: &Project, map: &MapFile) -> Result<Reservations> {
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let guarded = scripts::script_names(&nodes)?
        .iter()
        .any(|s| s.starts_with(HELPER));
    if guarded {
        validate(p, map)?;
    }
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let authored = authored_scripts(&nodes)?;
    let specs = specs(p, &authored, &objects)?;
    let q = policy(p)?;
    let mut footprints = Vec::new();
    for spec in specs {
        for slot in 0..q.fallback_slots {
            let wp = if slot == 0 {
                spec.name.clone()
            } else {
                format!("{ALT}{}-{slot}", spec.name)
            };
            if let Some(object) = objects.iter().find(|o| name(o) == wp) {
                footprints.push((point(p, object), spec.padding));
            }
        }
    }
    Ok(Reservations {
        guarded,
        footprints,
    })
}
struct Ground<'a> {
    p: &'a Project,
    n: usize,
    z: Vec<f64>,
    imp: Vec<bool>,
    blocked: Vec<bool>,
}
impl Ground<'_> {
    fn suitable(&self, c: [usize; 2], s: &Spec) -> bool {
        let [x, y] = c;
        let r = s.padding;
        let b = self.p.usize("BORDER", 20) + r + 2;
        if x < b || y < b || x >= self.n - b || y >= self.z.len() / self.n - b {
            return false;
        }
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                let i = yy * self.n + xx;
                if self.imp[i]
                    || self.blocked[i]
                    || (if s.wet {
                        self.z[i] >= self.p.number("WATER_Z", 200.) - 1.
                    } else {
                        self.z[i] <= self.p.number("WATER_Z", 200.)
                    })
                {
                    return false;
                }
                lo = lo.min(self.z[i]);
                hi = hi.max(self.z[i]);
            }
        }
        s.wet || hi - lo <= 1.
    }
}
fn clear(c: [usize; 2], r: usize, protected: &[([f64; 2], f64)], gap: usize) -> bool {
    protected.iter().all(|(p, q)| {
        (c[0] as f64 - p[0]).abs().max((c[1] as f64 - p[1]).abs()) > r as f64 + q + gap as f64
    })
}
fn protected(
    p: &Project,
    objects: &[MapObject],
    specs: &[Spec],
    q: &Policy,
) -> Vec<([f64; 2], f64)> {
    let mut result = keepouts(p, objects);
    for (_, r) in &mut result {
        if *r >= 8. {
            *r = r.max(q.runtime_blocker_radius as f64);
        }
    }
    for o in objects {
        if specs.iter().any(|s| s.name == name(o)) || is_alternate(o) {
            result.push((to_cell(p, o), q.runtime_blocker_radius as f64));
        }
    }
    result
}
fn move_to(p: &Project, o: &mut MapObject, c: [usize; 2]) {
    let position = crate::coordinates::Frame::from_project(p).world(c.map(|v| v as f64));
    o.position[0] = position[0] as f32;
    o.position[1] = position[1] as f32;
}
#[derive(Clone)]
struct Trigger {
    name: String,
    id: u32,
    points: Vec<[f32; 2]>,
    layer: String,
}
fn areas(map: &MapFile) -> Result<Vec<Trigger>> {
    let mut r = Reader::new(&map.get("TriggerAreas")?.payload);
    let mut out = Vec::new();
    for _ in 0..r.u32()? {
        let name = r.s16()?;
        let layer = r.s16()?;
        let id = r.u32()?;
        let mut points = Vec::new();
        for _ in 0..r.u32()? {
            points.push([r.f32()?, r.f32()?]);
        }
        if r.u32()? != 0 {
            return Err(err("unsupported trigger trailer"));
        }
        out.push(Trigger {
            name,
            id,
            points,
            layer,
        });
    }
    r.finish()?;
    Ok(out)
}
fn write_areas(map: &mut MapFile, a: &[Trigger]) -> Result<()> {
    let mut w = Writer::new();
    w.u32(a.len() as u32);
    for t in a {
        w.s16(&t.name)?;
        w.s16(&t.layer)?;
        w.u32(t.id);
        w.u32(t.points.len() as u32);
        for p in &t.points {
            w.f32(p[0]);
            w.f32(p[1]);
        }
        w.u32(0);
    }
    map.get_mut("TriggerAreas")?.payload = w.data;
    Ok(())
}
fn leaf(
    command: &str,
    opcode: u32,
    args: &[(u32, &str, i32)],
    condition: bool,
    invert: bool,
) -> Result<Node> {
    let arguments = args
        .iter()
        .map(|(t, s, i)| (*t, *i, 0., *s))
        .collect::<Vec<_>>();
    scripts::leaf(command, opcode, &arguments, condition, invert)
}
fn flag(s: &str, value: bool, condition: bool) -> Result<Node> {
    leaf(
        if condition { "FLAG" } else { "SET_FLAG" },
        if condition { 2 } else { 1 },
        &[(5, s, 0), (8, "", i32::from(value))],
        condition,
        false,
    )
}
fn helper(name: &str, conditions: Vec<Node>, spawn: Node, done: &str) -> Result<Node> {
    let mut w = Writer::new();
    w.s16(name)?;
    for _ in 0..3 {
        w.s16("")?;
    }
    w.bytes(&[1, 1, 1, 1, 1, 0]);
    w.u32(1);
    w.bytes(&[0, 0]);
    w.u32(0);
    w.u8(0);
    w.s16("")?;
    w.s16("X")?;
    Ok(Node {
        asset: "Script".into(),
        version: 4,
        prefix: scripts::hex(&w.data),
        body: String::new(),
        command: None,
        children: Some(vec![
            Node {
                asset: "OrCondition".into(),
                version: 1,
                prefix: String::new(),
                body: String::new(),
                command: None,
                children: Some(conditions),
            },
            spawn,
            flag(done, true, false)?,
        ]),
    })
}
fn replace_wp(n: &Node, wp: &str) -> Result<Node> {
    let bytes = scripts::unhex(&n.body)?;
    let mut r = Reader::new(&bytes);
    let mut w = Writer::new();
    w.bytes(r.take(8)?);
    let cnt = r.u32()?;
    w.u32(cnt);
    for _ in 0..cnt {
        let t = r.u32()?;
        w.u32(t);
        if t == 16 {
            w.bytes(r.take(12)?);
        } else {
            w.bytes(r.take(8)?);
            let s = r.s16()?;
            w.s16(if t == 7 { wp } else { &s })?;
        }
    }
    w.bytes(r.take(r.remaining())?);
    let mut out = n.clone();
    out.body = scripts::hex(&w.data);
    Ok(out)
}

pub fn apply(p: &Project, map: &mut MapFile) -> Result<Json> {
    let q = policy(p)?;
    let mut nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    if scripts::script_names(&nodes)?
        .iter()
        .any(|s| s.starts_with(HELPER))
    {
        if validate(p, map).is_ok()
            && (!crate::gameplay::official_placement::enforced(p)?
                || crate::gameplay::official_placement::audit(p, map)?["passed"] == true)
        {
            return Ok(json!({"already_guarded":true}));
        }
        nodes = authored_scripts(&nodes)?;
    }
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let ground = Ground {
        p,
        n: hm.width,
        z: terrain::flip(&hm.elevations, hm.width)
            .iter()
            .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
            .collect(),
        imp: terrain::flip(&bt.impassability, hm.width),
        blocked: terrain::flip(&bt.buildability, hm.width),
    };
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let specs = specs(p, &nodes, &objects)?;
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        hm.width,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let bases = terrain::Layout::load(p)?.base_cells;
    let mut slots = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut triggers = areas(map)?
        .into_iter()
        .filter(|a| !a.name.starts_with(AREA))
        .collect::<Vec<_>>();
    let mut next_area = triggers.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let mut next_wp = objects
        .iter()
        .filter_map(|o| match prop(&o.props, "waypointID") {
            Some(Value::Int(x)) => Some(*x),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;
    let mut placed = Vec::new();
    for slot in 0..q.fallback_slots {
        for spec in &specs {
            let index = objects.iter().position(|o| name(o) == spec.name).unwrap();
            let origin = point(p, &objects[index]);
            let prototype = objects[index].clone();
            let wp = if slot == 0 {
                spec.name.clone()
            } else {
                format!("{ALT}{}-{slot}", spec.name)
            };
            let existing = objects.iter().position(|o| name(o) == wp);
            let from = existing.map(|i| point(p, &objects[i])).unwrap_or(origin);
            let others: Vec<_> = objects
                .iter()
                .enumerate()
                .filter(|(i, _)| Some(*i) != existing)
                .map(|(_, o)| o.clone())
                .collect();
            let protected = protected(p, &others, &specs, &q);
            let official_obstacles = official
                .as_ref()
                .map(|checker| checker.planned_obstacles(&others));
            let base = bases[spec.seat - 1];
            let radius = search_radius(p, spec.seat)?;
            let radius = p.usize("PRODUCTION_SPAWN_RADIUS", radius).min(ground.n);
            let distance = |c: [usize; 2]| (c[0] as f64 - base[0]).hypot(c[1] as f64 - base[1]);
            let acceptable = |c: [usize; 2]| {
                ground.suitable(c, spec)
                    && clear(c, spec.padding.max(8), &protected, q.clearance)
                    && (14.0..=radius as f64).contains(&distance(c))
                    && official.as_ref().is_none_or(|checker| {
                        checker.acceptable(&wp, c, official_obstacles.as_ref().unwrap())
                    })
            };
            let point = if existing.is_some() && acceptable(from) {
                from
            } else {
                let [bx, by] = base.map(|v| v as usize);
                let mut candidates = Vec::new();
                for y in
                    by.saturating_sub(radius)..=(by + radius).min(ground.z.len() / ground.n - 1)
                {
                    for x in bx.saturating_sub(radius)..=(bx + radius).min(ground.n - 1) {
                        if (14.0..=radius as f64).contains(&distance([x, y])) {
                            candidates.push((
                                (x as f64 - origin[0] as f64).hypot(y as f64 - origin[1] as f64),
                                x,
                                y,
                            ));
                        }
                    }
                }
                candidates
                    .sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
                let (_, x, y) = candidates
                    .into_iter()
                    .find(|(_, x, y)| acceptable([*x, *y]))
                    .ok_or_else(|| err(format!("no clear gift slot {wp} near P{}", spec.seat)))?;
                [x, y]
            };
            let target = if let Some(i) = existing {
                i
            } else {
                let mut o = prototype.clone();
                map::set(&mut o.props, "waypointName", Value::Ascii(wp.clone()));
                map::set(&mut o.props, "uniqueID", Value::Ascii(wp.clone()));
                map::set(&mut o.props, "waypointID", Value::Int(next_wp));
                next_wp += 1;
                objects.push(o);
                objects.len() - 1
            };
            move_to(p, &mut objects[target], point);
            let area = format!("{AREA}{wp}");
            let center = objects[target].position;
            let r = (spec.padding + q.runtime_blocker_radius + q.clearance) as f32
                * p.number("CELL", 10.) as f32;
            triggers.push(Trigger {
                name: area.clone(),
                id: next_area,
                layer: String::new(),
                points: vec![
                    [center[0] - r, center[1] - r],
                    [center[0] + r, center[1] - r],
                    [center[0] + r, center[1] + r],
                    [center[0] - r, center[1] + r],
                ],
            });
            next_area += 1;
            slots
                .entry(spec.name.clone())
                .or_default()
                .push((wp.clone(), area));
            placed.push(json!({"waypoint":wp,"original_waypoint":spec.name,"from":from,"cell":point,"padding":spec.padding,"runtime_guard_radius":r,"moved":from!=point}));
        }
    }
    let players = crate::resources::player_names(map)?
        .into_iter()
        .filter(|s| !s.starts_with("Skirmish") && s != "_neutral_")
        .collect::<Vec<_>>();
    let mut serial = 0;
    fn transform(
        nodes: &mut [Node],
        slots: &BTreeMap<String, Vec<(String, String)>>,
        players: &[String],
        serial: &mut usize,
        helpers: &mut Vec<Node>,
    ) -> Result<()> {
        for n in nodes {
            if let Some(c) = &mut n.children {
                transform(c, slots, players, serial, helpers)?;
            } else if n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(n)?;
                let Some(wp) = args.iter().find(|(t, _)| *t == 7).map(|(_, s)| s.clone()) else {
                    continue;
                };
                let Some(candidates) = slots.get(&wp) else {
                    continue;
                };
                *serial += 1;
                let ready = format!("gift-ready-{serial}");
                let done = format!("gift-done-{serial}");
                for (slot, (target, area)) in candidates.iter().enumerate() {
                    let mut cond = vec![flag(&ready, true, true)?, flag(&done, false, true)?];
                    for player in players {
                        cond.push(leaf(
                            "SKIRMISH_PLAYER_HAS_UNITS_IN_AREA",
                            72,
                            &[(11, player, 0), (9, area, 0)],
                            true,
                            true,
                        )?);
                    }
                    helpers.push(helper(
                        &format!("{HELPER}{serial}-{}", slot + 1),
                        cond,
                        replace_wp(n, target)?,
                        &done,
                    )?);
                }
                *n = flag(&ready, true, false)?;
            }
        }
        Ok(())
    }
    for list in &mut nodes {
        let mut helpers = Vec::new();
        if let Some(c) = &mut list.children {
            transform(c, &slots, &players, &mut serial, &mut helpers)?;
            c.extend(helpers);
        }
    }
    for o in &objects {
        for v in &o.props {
            map.id(&v.key)?;
        }
    }
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    write_areas(map, &triggers)?;
    let payload = scripts::encode_nodes(&nodes, map)?;
    map.get_mut("PlayerScriptsList")?.payload = payload;
    validate(p, map)?;
    Ok(
        json!({"placements":placed,"gift_actions":serial,"slots_per_gift":q.fallback_slots,"runtime_occupancy_check":true,"all_occupied":"retain gift eligibility and retry","ingame_test":"not run"}),
    )
}

/// Keep existing guard identities when coordinate authoring moves a gift slot.
pub fn sync_moved_areas(
    map: &mut MapFile,
    before: &[MapObject],
    after: &[MapObject],
) -> Result<()> {
    let mut triggers = areas(map)?;
    let mut changed = false;
    for (old, new) in before.iter().zip(after) {
        if old.position == new.position || name(old).is_empty() {
            continue;
        }
        if let Some(area) = triggers
            .iter_mut()
            .find(|a| a.name == format!("{AREA}{}", name(old)))
        {
            let dx = new.position[0] - old.position[0];
            let dy = new.position[1] - old.position[1];
            for point in &mut area.points {
                point[0] += dx;
                point[1] += dy;
            }
            changed = true;
        }
    }
    if changed {
        write_areas(map, &triggers)?;
    }
    Ok(())
}

/// Move only slots obstructed by an authored resource change. Script conditions,
/// flags, slot identities and guard IDs stay intact.
pub fn repair_obstructed_slots(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
) -> Result<Json> {
    repair_slots(p, map, layout, false)
}

/// Explicit start relocation: repack every existing gift foundation around its
/// assigned seat while preserving waypoint identities, guards and conditions.
pub fn relocate_for_starts(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
) -> Result<Json> {
    repair_slots(p, map, layout, true)
}

fn repair_slots(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
    relocate: bool,
) -> Result<Json> {
    let q = policy(p)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let original = objects.clone();
    let authored = authored_scripts(&nodes)?;
    let all_specs = specs(p, &authored, &objects)?;
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let ground = Ground {
        p,
        n: hm.width,
        z: terrain::flip(&hm.elevations, hm.width)
            .iter()
            .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
            .collect(),
        imp: terrain::flip(&bt.impassability, hm.width),
        blocked: terrain::flip(&bt.buildability, hm.width),
    };
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        hm.width,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let corridor = crate::terrain::ramp_clearance::Policy::default();
    let ramps = crate::terrain::ramp_clearance::ramps(layout, &corridor)?;
    let ore_fields: Vec<_> = objects
        .iter()
        .filter(|o| o.type_name == "OreNode")
        .map(|o| {
            let a = o.angle as f64;
            crate::terrain::ramp_clearance::rectangle(
                to_cell(p, o),
                [a.cos(), -a.sin()],
                -5.,
                48.,
                18.,
            )
        })
        .collect();
    let mut changes = Vec::new();
    let mut placed = Vec::new();
    let mut seat_candidates = BTreeMap::new();
    for spec in &all_specs {
        for slot in 0..q.fallback_slots {
            let wp = if slot == 0 {
                spec.name.clone()
            } else {
                format!("{ALT}{}-{slot}", spec.name)
            };
            let index = objects
                .iter()
                .position(|o| name(o) == wp)
                .ok_or_else(|| err("missing gift slot during resource repair"))?;
            let old = point(p, &objects[index]);
            let others: Vec<_> = objects
                .iter()
                .enumerate()
                .filter(|(i, o)| {
                    *i != index
                        && (!relocate
                            || (!is_alternate(o) && !all_specs.iter().any(|s| name(o) == s.name)))
                })
                .map(|(_, o)| o.clone())
                .collect();
            let mut protected = protected(p, &others, &all_specs, &q);
            protected.extend(placed.iter().copied());
            let official_obstacles = official
                .as_ref()
                .map(|checker| checker.planned_obstacles(&others));
            let official_clear = |cell| {
                official.as_ref().is_none_or(|checker| {
                    checker.acceptable(&wp, cell, official_obstacles.as_ref().unwrap())
                })
            };
            if !relocate
                && ground.suitable(old, spec)
                && clear(old, spec.padding.max(8), &protected, q.clearance)
                && official_clear(old)
            {
                continue;
            }
            let base = layout.base_cells[spec.seat - 1];
            let radius = search_radius(p, spec.seat)?;
            let center = if relocate {
                base.map(|v| v.round() as usize)
            } else {
                old
            };
            let search = if relocate { radius } else { 120 };
            let make_candidates = || {
                let mut candidates = Vec::new();
                for y in center[1].saturating_sub(search)..=(center[1] + search).min(hm.height - 1)
                {
                    for x in
                        center[0].saturating_sub(search)..=(center[0] + search).min(hm.width - 1)
                    {
                        let c = [x, y];
                        let point = [x as f64, y as f64];
                        let d = (point[0] - base[0]).hypot(point[1] - base[1]);
                        if !(14.0..=radius as f64).contains(&d) {
                            continue;
                        }
                        candidates.push((
                            (x as f64 - center[0] as f64).hypot(y as f64 - center[1] as f64),
                            c,
                        ));
                    }
                }
                candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
                candidates
            };
            let local;
            let candidates = if relocate {
                seat_candidates
                    .entry(spec.seat)
                    .or_insert_with(make_candidates)
            } else {
                local = make_candidates();
                &local
            };
            let target = candidates
                .iter()
                .map(|(_, c)| *c)
                .find(|c| {
                    if !ground.suitable(*c, spec)
                        || !clear(*c, spec.padding.max(8), &protected, q.clearance)
                        || !official_clear(*c)
                    {
                        return false;
                    }
                    let point = c.map(|v| v as f64);
                    let height = ground.z[c[1] * hm.width + c[0]];
                    if !crate::terrain::ramp_clearance::conflicts(
                        point,
                        0.,
                        "HospitalTechStructure",
                        height,
                        &ramps,
                        &corridor,
                    )
                    .is_empty()
                    {
                        return false;
                    }
                    let padding = spec.padding.max(8) as f64;
                    let foundation = crate::terrain::ramp_clearance::rectangle(
                        point,
                        [1., 0.],
                        -padding,
                        padding,
                        padding,
                    );
                    ore_fields.iter().all(|field| {
                        crate::terrain::ramp_clearance::polygon_distance(&foundation, field) >= 3.
                    })
                })
                .ok_or_else(|| err(format!("no clear replacement gift slot {wp}")))?;
            move_to(p, &mut objects[index], target);
            placed.push((target.map(|v| v as f64), q.runtime_blocker_radius as f64));
            changes.push(json!({"id":index+1,"waypoint":wp,"before":old,"after":target}));
        }
    }
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    sync_moved_areas(map, &original, &objects)?;
    crate::gameplay::production_facing::apply(p, map)?;
    validate(p, map)?;
    Ok(
        json!({"changes":changes,"preserved":"guard IDs, scripts, conditions, flags and waypoint identities"}),
    )
}

pub fn validate(p: &Project, map: &MapFile) -> Result<()> {
    let q = policy(p)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let all_areas = areas(map)?
        .into_iter()
        .map(|t| t.name)
        .collect::<BTreeSet<_>>();
    let names = scripts::script_names(&nodes)?;
    if !names.iter().any(|s| s.starts_with(HELPER)) {
        return Err(err("gift runtime occupancy guards missing"));
    }
    fn check(nodes: &[Node], all: &BTreeSet<String>, slots: usize) -> Result<()> {
        for n in nodes {
            if scripts::name(n)?.is_some_and(|s| s.starts_with(HELPER)) {
                let c = n
                    .children
                    .as_ref()
                    .ok_or_else(|| err("gift helper has no body"))?;
                let conditions = c[0]
                    .children
                    .as_ref()
                    .ok_or_else(|| err("gift helper has no conditions"))?;
                let guards = conditions
                    .iter()
                    .filter(|n| n.command.as_deref() == Some("SKIRMISH_PLAYER_HAS_UNITS_IN_AREA"))
                    .collect::<Vec<_>>();
                if guards.len() < 9 || slots < 2 {
                    return Err(err("incomplete gift occupancy checks"));
                }
                for g in guards {
                    let b = scripts::unhex(&g.body)?;
                    if b[b.len() - 4..] != 1u32.to_le_bytes() {
                        return Err(err("gift occupancy check must be inverted"));
                    }
                    let args = scripts::string_arguments(g)?;
                    let a = &args
                        .iter()
                        .find(|(t, _)| *t == 9)
                        .ok_or_else(|| err("missing gift trigger reference"))?
                        .1;
                    if !all.contains(a) {
                        return Err(err(format!("missing gift trigger {a}")));
                    }
                }
            }
            if let Some(c) = &n.children {
                check(c, all, slots)?;
            }
        }
        Ok(())
    }
    check(&nodes, &all_areas, q.fallback_slots)?;
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let ground = Ground {
        p,
        n: hm.width,
        z: terrain::flip(&hm.elevations, hm.width)
            .iter()
            .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
            .collect(),
        imp: terrain::flip(&bt.impassability, hm.width),
        blocked: terrain::flip(&bt.buildability, hm.width),
    };
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let authored = authored_scripts(&nodes)?;
    let all_specs = specs(p, &authored, &objects)?;
    let triggers = areas(map)?;
    for spec in &all_specs {
        for slot in 0..q.fallback_slots {
            let wp = if slot == 0 {
                spec.name.clone()
            } else {
                format!("{ALT}{}-{slot}", spec.name)
            };
            let matches = objects
                .iter()
                .enumerate()
                .filter(|(_, o)| name(o) == wp)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(err(format!("gift slot missing or duplicated: {wp}")));
            }
            let (index, object) = matches[0];
            let c = point(p, object);
            let others = objects
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, o)| o.clone())
                .collect::<Vec<_>>();
            if !ground.suitable(c, spec)
                || !clear(
                    c,
                    spec.padding.max(8),
                    &protected(p, &others, &all_specs, &q),
                    q.clearance,
                )
            {
                return Err(err(format!("gift slot overlaps an obstruction: {wp}")));
            }
            let t = triggers
                .iter()
                .find(|a| a.name == format!("{AREA}{wp}"))
                .ok_or_else(|| err(format!("gift guard area missing: {wp}")))?;
            let r = (spec.padding + q.runtime_blocker_radius + q.clearance) as f32
                * p.number("CELL", 10.) as f32;
            let [x, y, _] = object.position;
            if t.points
                != vec![
                    [x - r, y - r],
                    [x + r, y - r],
                    [x + r, y + r],
                    [x - r, y + r],
                ]
            {
                return Err(err(format!(
                    "gift guard area does not cover footprint: {wp}"
                )));
            }
        }
    }
    Ok(())
}

/// Recover the exact authored scripts for regression comparisons.
pub fn authored_scripts(nodes: &[Node]) -> Result<Vec<Node>> {
    let mut creates = BTreeMap::new();
    fn collect(nodes: &[Node], out: &mut BTreeMap<String, Node>) -> Result<()> {
        for n in nodes {
            if let Some(s) = scripts::name(n)?
                && s.starts_with(HELPER)
                && s.ends_with("-1")
            {
                let id = s.strip_prefix(HELPER).unwrap().strip_suffix("-1").unwrap();
                out.insert(
                    format!("gift-ready-{id}"),
                    n.children.as_ref().unwrap()[1].clone(),
                );
            }
            if let Some(c) = &n.children {
                collect(c, out)?;
            }
        }
        Ok(())
    }
    collect(nodes, &mut creates)?;
    fn restore(nodes: &[Node], creates: &BTreeMap<String, Node>) -> Result<Vec<Node>> {
        let mut out = Vec::new();
        for n in nodes {
            if scripts::name(n)?.is_some_and(|s| s.starts_with(HELPER)) {
                continue;
            }
            let mut n = n.clone();
            if let Some(c) = &n.children {
                n.children = Some(restore(c, creates)?);
            } else if n.command.as_deref() == Some("SET_FLAG") {
                let args = scripts::string_arguments(&n)?;
                if let Some((_, s)) = args.iter().find(|(t, _)| *t == 5)
                    && let Some(create) = creates.get(s)
                {
                    n = create.clone();
                }
            }
            out.push(n);
        }
        Ok(out)
    }
    restore(nodes, &creates)
}

pub fn authored_areas(map: &mut MapFile) -> Result<()> {
    let a = areas(map)?
        .into_iter()
        .filter(|t| !t.name.starts_with(AREA))
        .collect::<Vec<_>>();
    write_areas(map, &a)
}
pub fn is_alternate(o: &MapObject) -> bool {
    name(o).starts_with(ALT)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gift_footprints_follow_the_shared_production_policy() {
        let production: crate::gameplay::production_spawns::PlacementPolicy =
            serde_json::from_value(json!({
                "full_footprint":true,
                "war_factory":{"surface":"land","padding":10},
                "naval_yard":{"surface":"water","padding":12}
            }))
            .unwrap();
        assert_eq!(kind("AlliedWarFactory", &production), Some((false, 10)));
        assert_eq!(kind("SovietNavalYard", &production), Some((true, 12)));
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "海南").unwrap();
        let mut ground = Ground {
            p: &p,
            n: 128,
            z: vec![212.; 128 * 128],
            imp: vec![false; 128 * 128],
            blocked: vec![false; 128 * 128],
        };
        let (wet, padding) = kind("AlliedWarFactory", &production).unwrap();
        let spec = Spec {
            name: "test".into(),
            wet,
            padding,
            seat: 1,
        };
        assert!(ground.suitable([64, 64], &spec));
        ground.blocked[64 * 128 + 73] = true;
        assert!(!ground.suitable([64, 64], &spec));
        let smaller = Spec { padding: 6, ..spec };
        assert!(ground.suitable([64, 64], &smaller));
    }

    fn arguments(n: &Node) -> Vec<(u32, i32, String)> {
        let b = scripts::unhex(&n.body).unwrap();
        let mut r = Reader::new(&b);
        r.take(8).unwrap();
        let count = r.u32().unwrap();
        (0..count)
            .map(|_| {
                let t = r.u32().unwrap();
                let i = r.i32().unwrap();
                r.f32().unwrap();
                (t, i, r.s16().unwrap())
            })
            .collect()
    }
    fn run(
        nodes: &[Node],
        flags: &mut BTreeMap<String, bool>,
        occupied: &mut BTreeSet<String>,
    ) -> Vec<String> {
        let mut spawned = Vec::new();
        for script in nodes {
            let c = script.children.as_ref().unwrap();
            let pass = c[0].children.as_ref().unwrap().iter().all(|n| {
                let a = arguments(n);
                match n.command.as_deref().unwrap() {
                    "FLAG" => *flags.get(&a[0].2).unwrap_or(&false) == (a[1].1 != 0),
                    "SKIRMISH_PLAYER_HAS_UNITS_IN_AREA" => {
                        let b = scripts::unhex(&n.body).unwrap();
                        let inverted = b[b.len() - 4..] == 1u32.to_le_bytes();
                        occupied.contains(&a[1].2) != inverted
                    }
                    _ => panic!("unexpected condition"),
                }
            });
            if pass {
                for action in &c[1..] {
                    let a = arguments(action);
                    match action.command.as_deref().unwrap() {
                        "SET_FLAG" => {
                            flags.insert(a[0].2.clone(), a[1].1 != 0);
                        }
                        "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION" => {
                            let wp = a.iter().find(|a| a.0 == 7).unwrap().2.clone();
                            occupied.insert(wp.clone());
                            spawned.push(wp);
                        }
                        _ => panic!("unexpected action"),
                    }
                }
            }
        }
        spawned
    }
    fn candidates() -> Vec<Node> {
        ["primary", "backup"]
            .iter()
            .map(|slot| {
                let conditions = vec![
                    flag("ready", true, true).unwrap(),
                    flag("done", false, true).unwrap(),
                    leaf(
                        "SKIRMISH_PLAYER_HAS_UNITS_IN_AREA",
                        72,
                        &[(11, "Player_1", 0), (9, slot, 0)],
                        true,
                        true,
                    )
                    .unwrap(),
                ];
                let spawn = leaf(
                    "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
                    467,
                    &[(15, "sovietwarfactory", 0), (7, slot, 0)],
                    false,
                    false,
                )
                .unwrap();
                helper(slot, conditions, spawn, "done").unwrap()
            })
            .collect()
    }
    #[test]
    fn occupied_primary_uses_backup_once_and_does_not_spawn_before_trigger() {
        let nodes = candidates();
        let mut flags = BTreeMap::new();
        let mut occupied = BTreeSet::from(["primary".into()]);
        assert!(run(&nodes, &mut flags, &mut occupied).is_empty());
        flags.insert("ready".into(), true);
        assert_eq!(run(&nodes, &mut flags, &mut occupied), ["backup"]);
        assert!(run(&nodes, &mut flags, &mut occupied).is_empty());
    }
    #[test]
    fn all_occupied_retries_without_losing_or_duplicating_the_gift() {
        let nodes = candidates();
        let mut flags = BTreeMap::from([("ready".into(), true)]);
        let mut occupied = BTreeSet::from(["primary".into(), "backup".into()]);
        assert!(run(&nodes, &mut flags, &mut occupied).is_empty());
        assert!(!flags.contains_key("done"));
        occupied.remove("primary");
        occupied.remove("backup");
        assert_eq!(run(&nodes, &mut flags, &mut occupied), ["primary"]);
        assert_eq!(flags.get("done"), Some(&true));
        assert!(run(&nodes, &mut flags, &mut occupied).is_empty());
    }
}
