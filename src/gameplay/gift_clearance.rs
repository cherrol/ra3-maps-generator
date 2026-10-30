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
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

const HELPER: &str = "gift-clear-";
const ALT: &str = "gift-alt-";
const AREA: &str = "gift-area-";

/// An explicit repair decision belongs to one decoded native source. Unlisted
/// obstructed slots receive local repair; valid slots remain unchanged unless
/// an individually justified production base reselection explicitly includes them.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepairPlan {
    pub schema_version: u32,
    pub source_native_decoded_crc32: u32,
    pub repairs: Vec<RepairDirective>,
    #[serde(default)]
    pub allowed_regions: Vec<AllowedRepairRegion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedRepairRegion {
    pub id: String,
    pub height: f64,
    /// Grid coordinates: xmin, ymin, xmax, ymax. Complete candidate geometry
    /// must fit one region; a clear centre alone never establishes coverage.
    pub bounds: [f64; 4],
    /// Optional authored flat terrain support. It is not an entity footprint
    /// and does not enlarge resource gaps or obstruction walls.
    #[serde(default)]
    pub support_half_extent: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepairDirective {
    pub waypoint: String,
    pub strategy: RepairStrategy,
    pub reason: String,
    /// Explicitly include a valid production slot in a justified base
    /// reselection. This never turns its existing geometry into a fake defect.
    #[serde(default)]
    pub reselect_unobstructed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    Local,
    Base,
}

fn permits_unobstructed_reselection(directive: &RepairDirective, production: bool) -> Result<bool> {
    if !directive.reselect_unobstructed {
        return Ok(false);
    }
    if !production
        || directive.strategy != RepairStrategy::Base
        || directive.reason.trim().is_empty()
    {
        return Err(err(format!(
            "unobstructed reselection requires a production point, base strategy and nonempty reason: {}",
            directive.waypoint
        )));
    }
    Ok(true)
}

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

pub(crate) fn decoration_clearance(p: &Project) -> Result<usize> {
    Ok(policy(p)?.clearance)
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
    production: bool,
}
pub(crate) fn kind(
    name: &str,
    production: &crate::gameplay::production_spawns::PlacementPolicy,
) -> Option<(bool, usize)> {
    let s = name.to_ascii_lowercase();
    // Airfields keep their own public gift reservation. The shared production
    // classifier also recognizes them, but its land-factory padding is smaller.
    if s.ends_with("airfield") {
        return Some((false, 8));
    }
    if let Some(footprint) = production.footprint(name) {
        return Some(footprint);
    }
    if s.ends_with("barracks") || s.contains("superweapon") {
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
    let mut out = BTreeMap::<String, Spec>::new();
    fn visit(
        nodes: &[Node],
        objects: &[MapObject],
        rules: &scripts::Rules,
        production: &crate::gameplay::production_spawns::PlacementPolicy,
        out: &mut BTreeMap<String, Spec>,
    ) -> Result<()> {
        for n in nodes {
            if let Some(c) = &n.children {
                visit(c, objects, rules, production, out)?;
            } else if n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(n)?;
                let Some((building, (wet, padding))) = args.iter().find_map(|(t, s)| {
                    if *t == 15 {
                        kind(s, production).map(|footprint| (s, footprint))
                    } else {
                        None
                    }
                }) else {
                    continue;
                };
                let Some((_, wp)) = args.iter().find(|(t, _)| *t == 7) else {
                    continue;
                };
                if !wp.starts_with("dual-") && !rules.spawn_waypoints.iter().any(|s| s.name == *wp)
                {
                    continue;
                }
                objects
                    .iter()
                    .find(|object| name(object) == wp)
                    .ok_or_else(|| err(format!("missing gift waypoint {wp}")))?;
                let team = args
                    .iter()
                    .find(|(kind, _)| *kind == 3)
                    .map(|(_, value)| value.as_str())
                    .ok_or_else(|| err(format!("gift action has no recipient team: {wp}")))?;
                let seat = crate::gameplay::production_facing::seat(team)?;
                let s = out.entry(wp.clone()).or_insert(Spec {
                    name: wp.clone(),
                    wet,
                    padding,
                    seat,
                    production: crate::gameplay::placement::is_production(building),
                });
                if s.wet != wet {
                    return Err(err("gift waypoint mixes water and land buildings"));
                }
                if s.seat != seat {
                    return Err(err(format!(
                        "gift waypoint has conflicting recipient seats: {wp}, P{} and P{seat}",
                        s.seat
                    )));
                }
                s.padding = s.padding.max(padding);
                s.production |= crate::gameplay::placement::is_production(building);
            }
        }
        Ok(())
    }
    visit(nodes, objects, &rules, &production, &mut out)?;
    Ok(out.into_values().collect())
}

fn slot_name(spec: &Spec, slot: usize) -> String {
    if slot == 0 {
        spec.name.clone()
    } else {
        format!("{ALT}{}-{slot}", spec.name)
    }
}

fn is_planned_slot(waypoint: &str, specs: &[Spec]) -> bool {
    specs
        .iter()
        .any(|spec| waypoint == spec.name || waypoint.starts_with(&format!("{ALT}{}-", spec.name)))
}

/// A deterministic terrain sample estimates scarcity for ordering only. Every
/// selected point still receives the complete foundation and official checks.
fn planning_order(
    p: &Project,
    specs: &[Spec],
    ground: &Ground<'_>,
    bases: &[[f64; 2]],
    objects: &[MapObject],
) -> Result<Vec<Spec>> {
    let mut ranked = Vec::new();
    for spec in specs {
        let base = bases[spec.seat - 1];
        let radius = p
            .usize("PRODUCTION_SPAWN_RADIUS", search_radius(p, spec.seat)?)
            .min(ground.n);
        let center = base.map(|v| v.round_ties_even() as usize);
        let stride = (2 * spec.padding).max(8);
        let mut candidates = 0usize;
        for y in center[1].saturating_sub(radius)
            ..=(center[1] + radius).min(ground.z.len() / ground.n - 1)
        {
            if !y.is_multiple_of(stride) {
                continue;
            }
            for x in center[0].saturating_sub(radius)..=(center[0] + radius).min(ground.n - 1) {
                if x.is_multiple_of(stride)
                    && (14.0..=radius as f64)
                        .contains(&(x as f64 - base[0]).hypot(y as f64 - base[1]))
                    && ground.suitable([x, y], spec)
                {
                    candidates += 1;
                }
            }
        }
        let identity = objects
            .iter()
            .position(|object| name(object) == spec.name)
            .ok_or_else(|| err("missing gift waypoint while ordering candidates"))?;
        ranked.push((
            !spec.production,
            candidates,
            std::cmp::Reverse(spec.padding),
            identity,
            spec.clone(),
        ));
    }
    ranked.sort_by_key(|entry| (entry.0, entry.1, entry.2, entry.3));
    Ok(ranked.into_iter().map(|entry| entry.4).collect())
}

fn planning_objects(
    objects: &[MapObject],
    specs: &[Spec],
    selected: &BTreeSet<String>,
    fixed: &BTreeSet<String>,
    current: usize,
) -> Vec<MapObject> {
    objects
        .iter()
        .enumerate()
        .filter(|(index, object)| {
            *index != current
                && !crate::gameplay::placement::is_decoration(object)
                && (!is_planned_slot(name(object), specs)
                    || selected.contains(name(object))
                    || fixed.contains(name(object)))
        })
        .map(|(_, object)| object.clone())
        .collect()
}

pub(crate) struct FixedReservations {
    policy: terrain::ramp_clearance::Policy,
    mine_gap: f64,
    ore_radius: f64,
    flat_tolerance: f64,
    ramps: Vec<terrain::ramp_clearance::Ramp>,
    ores: Vec<([f64; 2], terrain::ramp_clearance::Polygon, f64)>,
    ore_ids: Vec<usize>,
}
impl FixedReservations {
    pub(crate) fn new(
        p: &Project,
        layout: &terrain::Layout,
        objects: &[MapObject],
        z: &[f64],
        width: usize,
    ) -> Result<Self> {
        if width == 0 || z.is_empty() || !z.len().is_multiple_of(width) {
            return Err(err("invalid height grid for functional reservations"));
        }
        let spawn = terrain::spawn_placement::policy(p)?;
        let mine_gap = decoration_clearance(p)? as f64;
        let policy = terrain::ramp_clearance::policy(p)?
            .ok_or_else(|| err("gift planning requires the ramp clearance policy"))?;
        let ramps = terrain::ramp_clearance::ramps(layout, &policy)?;
        let ores = objects
            .iter()
            .filter(|object| object.type_name == "OreNode")
            .map(|object| {
                let angle = object.angle as f64;
                let center = to_cell(p, object);
                let x = center[0].round_ties_even() as usize;
                let y = center[1].round_ties_even() as usize;
                if center.iter().any(|value| !value.is_finite() || *value < 0.)
                    || x >= width
                    || y >= z.len() / width
                {
                    return Err(err(
                        "ore outside height grid while reserving functional space",
                    ));
                }
                Ok((
                    center,
                    terrain::ramp_clearance::rectangle(
                        center,
                        [angle.cos(), -angle.sin()],
                        -policy.refinery_back,
                        policy.refinery_forward,
                        policy.refinery_half_width,
                    ),
                    z[y * width + x],
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let ore_ids = objects
            .iter()
            .enumerate()
            .filter(|(_, object)| object.type_name == "OreNode")
            .map(|(index, _)| index + 1)
            .collect();
        Ok(Self {
            policy,
            mine_gap,
            ore_radius: spawn.ore_radius,
            flat_tolerance: spawn.flat_tolerance,
            ramps,
            ores,
            ore_ids,
        })
    }

    #[cfg(test)]
    fn clear(&self, ground: &Ground<'_>, cell: [usize; 2], spec: &Spec) -> bool {
        self.clear_at(cell, spec.padding, ground.z[cell[1] * ground.n + cell[0]])
    }

    fn clear_candidate(
        &self,
        ground: &Ground<'_>,
        cell: [usize; 2],
        _spec: &Spec,
        checker: Option<&crate::gameplay::official_placement::Checker>,
        waypoint: &str,
    ) -> bool {
        // The resolved union already includes the public core; checking the
        // same square first would repeat every resource/approach calculation.
        candidate_reservation_polygons(checker, waypoint, cell).is_ok_and(|polygons| {
            self.clear_polygons_at(cell, ground.z[cell[1] * ground.n + cell[0]], &polygons)
        })
    }

    #[cfg(test)]
    pub(crate) fn clear_at(&self, cell: [usize; 2], padding: usize, height: f64) -> bool {
        let center = cell.map(|value| value as f64);
        // This is the actual public foundation, not the larger generic runtime
        // keepout radius used to separate gift slots from one another.
        let padding = padding as f64;
        let foundation =
            terrain::ramp_clearance::rectangle(center, [1., 0.], -padding, padding, padding);
        for ramp in &self.ramps {
            let delta = [center[0] - ramp.center[0], center[1] - ramp.center[1]];
            let along = delta[0] * ramp.unit[0] + delta[1] * ramp.unit[1];
            let expected = if along >= 0. {
                ramp.pair[1]
            } else {
                ramp.pair[0]
            };
            if (along.abs() > ramp.half && (height - expected).abs() > 12.)
                || height < ramp.pair[0] - 12.
                || height > ramp.pair[1] + 12.
            {
                continue;
            }
            if terrain::ramp_clearance::polygon_distance(&foundation, &ramp.approach)
                < self.policy.gap
            {
                return false;
            }
        }
        self.ores.iter().all(|(ore, field, ore_height)| {
            if (height - ore_height).abs() > self.flat_tolerance {
                return true;
            }
            let dx = ((center[0] - ore[0]).abs() - padding).max(0.);
            let dy = ((center[1] - ore[1]).abs() - padding).max(0.);
            dx.hypot(dy) - self.ore_radius >= self.mine_gap
                && terrain::ramp_clearance::polygon_distance(&foundation, field) >= self.mine_gap
        })
    }

    #[cfg(test)]
    fn findings_at(&self, cell: [usize; 2], padding: usize, height: f64) -> Vec<Json> {
        let center = cell.map(|value| value as f64);
        let padding = padding as f64;
        let foundation =
            terrain::ramp_clearance::rectangle(center, [1., 0.], -padding, padding, padding);
        self.findings_polygons_at(cell, height, &[foundation.to_vec()])
    }

    /// Complete resolved geometry supplements the public core reservation.
    /// The same world-height filters and independent ore/ramp gaps apply.
    pub(crate) fn clear_polygons_at(
        &self,
        cell: [usize; 2],
        height: f64,
        polygons: &[Vec<[f64; 2]>],
    ) -> bool {
        self.visit_polygon_deficits(cell, height, polygons, |_| false)
    }

    pub(crate) fn findings_polygons_at(
        &self,
        cell: [usize; 2],
        height: f64,
        polygons: &[Vec<[f64; 2]>],
    ) -> Vec<Json> {
        let mut findings = Vec::new();
        self.visit_polygon_deficits(cell, height, polygons, |finding| {
            findings.push(finding);
            true
        });
        findings
    }

    fn visit_polygon_deficits(
        &self,
        cell: [usize; 2],
        height: f64,
        polygons: &[Vec<[f64; 2]>],
        mut record: impl FnMut(Json) -> bool,
    ) -> bool {
        if polygons.is_empty()
            || polygons.iter().any(|polygon| {
                polygon.len() < 3 || polygon.iter().flatten().any(|value| !value.is_finite())
            })
        {
            record(json!({"kind":"unresolved_functional_geometry"}));
            return false;
        }
        let center = cell.map(|value| value as f64);
        let bounds = polygons
            .iter()
            .map(|polygon| polygon_bounds(polygon))
            .collect::<Vec<_>>();
        let mut clear = true;
        for ramp in &self.ramps {
            let delta = [center[0] - ramp.center[0], center[1] - ramp.center[1]];
            let along = delta[0] * ramp.unit[0] + delta[1] * ramp.unit[1];
            let expected = if along >= 0. {
                ramp.pair[1]
            } else {
                ramp.pair[0]
            };
            if (along.abs() > ramp.half && (height - expected).abs() > 12.)
                || height < ramp.pair[0] - 12.
                || height > ramp.pair[1] + 12.
            {
                continue;
            }
            let approach_bounds = polygon_bounds(&ramp.approach);
            let gap = polygons
                .iter()
                .zip(&bounds)
                .filter(|(_, bounds)| bounds_distance(**bounds, approach_bounds) <= self.policy.gap)
                .map(|(polygon, _)| convex_polygon_distance(polygon, &ramp.approach))
                .fold(f64::INFINITY, f64::min);
            if gap < self.policy.gap {
                clear = false;
                if !record(
                    json!({"kind":"ramp_approach","ramp_id":ramp.id,"gap_cells":gap,"required_gap_cells":self.policy.gap}),
                ) {
                    return false;
                }
            }
        }
        for (index, (ore, field, ore_height)) in self.ores.iter().enumerate() {
            if (height - ore_height).abs() > self.flat_tolerance {
                continue;
            }
            let disk_gap = polygons
                .iter()
                .zip(&bounds)
                .filter(|(_, bounds)| {
                    bounds_distance(**bounds, [ore[0], ore[1], ore[0], ore[1]])
                        <= self.ore_radius + self.mine_gap
                })
                .map(|(polygon, _)| convex_polygon_distance(polygon, &[*ore]) - self.ore_radius)
                .fold(f64::INFINITY, f64::min);
            let field_bounds = polygon_bounds(field);
            let field_gap = polygons
                .iter()
                .zip(&bounds)
                .filter(|(_, bounds)| bounds_distance(**bounds, field_bounds) <= self.mine_gap)
                .map(|(polygon, _)| convex_polygon_distance(polygon, field))
                .fold(f64::INFINITY, f64::min);
            let ore_id = self.ore_ids[index];
            for (kind, gap) in [("ore_disk", disk_gap), ("rotated_refinery", field_gap)] {
                if gap < self.mine_gap {
                    clear = false;
                    if !record(
                        json!({"kind":kind,"ore_id":ore_id,"ore_cell":ore,"gap_cells":gap,"required_gap_cells":self.mine_gap}),
                    ) {
                        return false;
                    }
                }
            }
        }
        clear
    }
}

fn polygon_bounds(polygon: &[[f64; 2]]) -> [f64; 4] {
    polygon.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |bounds, point| {
            [
                bounds[0].min(point[0]),
                bounds[1].min(point[1]),
                bounds[2].max(point[0]),
                bounds[3].max(point[1]),
            ]
        },
    )
}

/// Axis-aligned enclosing boxes give a lower bound, so only pairs already
/// farther than the unchanged rule gap skip the exact convex calculation.
fn bounds_distance(a: [f64; 4], b: [f64; 4]) -> f64 {
    (a[0] - b[2])
        .max(b[0] - a[2])
        .max(0.)
        .hypot((a[1] - b[3]).max(b[1] - a[3]).max(0.))
}

fn convex_polygon_distance(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let separated = [a, b].into_iter().any(|polygon| {
        (0..polygon.len()).any(|index| {
            let p = polygon[index];
            let q = polygon[(index + 1) % polygon.len()];
            let axis = [q[1] - p[1], p[0] - q[0]];
            let range = |points: &[[f64; 2]]| {
                points
                    .iter()
                    .map(|point| dot(*point, axis))
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), value| {
                        (lo.min(value), hi.max(value))
                    })
            };
            let aa = range(a);
            let bb = range(b);
            aa.1 < bb.0 - 1e-8 || bb.1 < aa.0 - 1e-8
        })
    });
    if !separated {
        return 0.;
    }
    let mut best = f64::INFINITY;
    for (points, edges) in [(a, b), (b, a)] {
        for point in points {
            for index in 0..edges.len() {
                let start = edges[index];
                let end = edges[(index + 1) % edges.len()];
                let edge = [end[0] - start[0], end[1] - start[1]];
                let relative = [point[0] - start[0], point[1] - start[1]];
                let length = dot(edge, edge);
                let t = if length == 0. {
                    0.
                } else {
                    (dot(relative, edge) / length).clamp(0., 1.)
                };
                best = best.min((relative[0] - t * edge[0]).hypot(relative[1] - t * edge[1]));
            }
        }
    }
    best
}

fn candidate_reservation_polygons(
    checker: Option<&crate::gameplay::official_placement::Checker>,
    waypoint: &str,
    cell: [usize; 2],
) -> Result<Vec<Vec<[f64; 2]>>> {
    checker
        .ok_or_else(|| err("complete gift reservation checking requires strict official geometry"))?
        .candidate_reservation_polygons(waypoint, cell)
}

struct Selection {
    base: [f64; 2],
    origin: [f64; 2],
    radius: usize,
    local: bool,
}

fn production_access_score(
    field: Option<&crate::gameplay::placement::AccessDistances>,
    checker: Option<&crate::gameplay::official_placement::Checker>,
    waypoint: &str,
    cell: [usize; 2],
    spec: &Spec,
) -> Option<u32> {
    let field = field?;
    if let Some(checker) = checker {
        let targets = checker.production_access_targets(waypoint, cell).ok()?;
        if !targets.is_empty() {
            let mut worst = 0;
            for (wet, point) in targets {
                worst = worst.max(field.score_point(point, wet)?);
            }
            return Some(worst);
        }
    }
    field.score(cell, spec.wet, spec.padding)
}

fn choose_point(
    ground: &Ground<'_>,
    spec: &Spec,
    selection: &Selection,
    access: Option<&dyn Fn([usize; 2]) -> Option<u32>>,
    acceptable: impl Fn([usize; 2]) -> bool,
) -> Option<[usize; 2]> {
    let eligible = |cell: [usize; 2]| {
        (14.0..=selection.radius as f64).contains(
            &(cell[0] as f64 - selection.base[0]).hypot(cell[1] as f64 - selection.base[1]),
        ) && acceptable(cell)
    };
    let origin_sample = selection
        .origin
        .map(|value| value.round_ties_even() as usize);
    if !spec.production && eligible(origin_sample) {
        return Some(origin_sample);
    }
    let center = if selection.local {
        origin_sample
    } else {
        selection.base.map(|value| value.round_ties_even() as usize)
    };
    let search = if selection.local {
        120
    } else {
        selection.radius
    };
    let mut candidates = Vec::new();
    for y in
        center[1].saturating_sub(search)..=(center[1] + search).min(ground.z.len() / ground.n - 1)
    {
        for x in center[0].saturating_sub(search)..=(center[0] + search).min(ground.n - 1) {
            let cell = [x, y];
            if !(14.0..=selection.radius as f64)
                .contains(&(x as f64 - selection.base[0]).hypot(y as f64 - selection.base[1]))
            {
                continue;
            }
            let score = if spec.production {
                let Some(score) = access.and_then(|score| score(cell)) else {
                    continue;
                };
                score
            } else {
                0
            };
            let displacement =
                (x as f64 - selection.origin[0]).hypot(y as f64 - selection.origin[1]);
            let base_distance = if spec.production {
                (x as f64 - selection.base[0]).hypot(y as f64 - selection.base[1])
            } else {
                0.
            };
            candidates.push((score, base_distance, displacement, y, x));
        }
    }
    candidates.sort_by(|a, b| {
        (if selection.local {
            a.2.total_cmp(&b.2)
                .then(a.0.cmp(&b.0))
                .then(a.1.total_cmp(&b.1))
        } else {
            a.0.cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.total_cmp(&b.2))
        })
        .then(a.3.cmp(&b.3))
        .then(a.4.cmp(&b.4))
    });
    candidates
        .into_iter()
        .map(|(_, _, _, y, x)| [x, y])
        .find(|cell| eligible(*cell))
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
        let Some(x_limit) = self.n.checked_sub(b) else {
            return false;
        };
        let Some(y_limit) = (self.z.len() / self.n).checked_sub(b) else {
            return false;
        };
        if x < b || y < b || x >= x_limit || y >= y_limit {
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
fn actual_displacement(before: [f64; 2], after: [f64; 2]) -> Option<f64> {
    (before != after).then(|| (after[0] - before[0]).hypot(after[1] - before[1]))
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

fn edit_candidate(
    map: &mut MapFile,
    edit: impl FnOnce(&mut MapFile) -> Result<Json>,
) -> Result<Json> {
    let mut candidate = map.clone();
    let report = edit(&mut candidate)?;
    *map = candidate;
    Ok(report)
}

pub fn apply(p: &Project, map: &mut MapFile) -> Result<Json> {
    edit_candidate(map, |candidate| apply_inner(p, candidate))
}

fn apply_inner(p: &Project, map: &mut MapFile) -> Result<Json> {
    let strict = crate::gameplay::official_placement::strict_project(p);
    let p = &strict;
    let q = policy(p)?;
    let mut nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    if scripts::script_names(&nodes)?
        .iter()
        .any(|s| s.starts_with(HELPER))
    {
        let mut layout = terrain::Layout::load(p)?;
        layout.base_cells = terrain::Layout::native_base_cells(p, map)?;
        let placements = repair_slots(p, map, &layout, RepairMode::Optimize, None)?;
        return Ok(json!({"already_guarded":true,"planning":placements}));
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
    let mut layout = terrain::Layout::load(p)?;
    layout.base_cells = terrain::Layout::native_base_cells(p, map)?;
    let specs = planning_order(
        p,
        &specs(p, &nodes, &objects)?,
        &ground,
        &layout.base_cells,
        &objects,
    )?;
    let reservations = FixedReservations::new(p, &layout, &objects, &ground.z, ground.n)?;
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        hm.width,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let bases = &layout.base_cells;
    let mut selected = BTreeSet::new();
    let fixed = BTreeSet::new();
    let access_objects = planning_objects(&objects, &specs, &selected, &fixed, usize::MAX);
    let mut access = BTreeMap::new();
    for spec in specs.iter().filter(|spec| spec.production) {
        if let std::collections::btree_map::Entry::Vacant(entry) = access.entry(spec.seat) {
            entry.insert(crate::gameplay::placement::AccessDistances::new(
                p,
                hm.width,
                &ground.z,
                &ground.imp,
                &access_objects,
                spec.seat,
            )?);
        }
    }
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
            let wp = slot_name(spec, slot);
            let existing = objects.iter().position(|o| name(o) == wp);
            let from = existing.map(|i| point(p, &objects[i])).unwrap_or(origin);
            let others = planning_objects(
                &objects,
                &specs,
                &selected,
                &fixed,
                existing.unwrap_or(usize::MAX),
            );
            let protected = protected(p, &others, &specs, &q);
            let official_obstacles = official
                .as_ref()
                .map(|checker| checker.planned_obstacles(&others));
            let base = bases[spec.seat - 1];
            let radius = search_radius(p, spec.seat)?;
            let radius = p.usize("PRODUCTION_SPAWN_RADIUS", radius).min(ground.n);
            let acceptable = |c: [usize; 2]| {
                ground.suitable(c, spec)
                    && clear(c, spec.padding.max(8), &protected, q.clearance)
                    && reservations.clear_candidate(&ground, c, spec, official.as_ref(), &wp)
                    && official.as_ref().is_none_or(|checker| {
                        checker.acceptable(&wp, c, official_obstacles.as_ref().unwrap())
                    })
            };
            let access_score = |cell| {
                production_access_score(access.get(&spec.seat), official.as_ref(), &wp, cell, spec)
            };
            let point = choose_point(
                &ground,
                spec,
                &Selection {
                    base,
                    origin: from.map(|value|value as f64),
                    radius,
                    local: false,
                },
                Some(&access_score),
                acceptable,
            )
            .ok_or_else(|| err(format!("no clear gift slot {wp} near P{}: no candidate satisfies base access, complete foundation, ramp approaches, ore/refinery reservations and official exits", spec.seat)))?;
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
            selected.insert(wp.clone());
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
    crate::gameplay::production_facing::apply(p, map)?;
    let decoration_yield = crate::gameplay::decoration::yield_to_functional(p, map, &layout)?;
    validate_with_layout(p, map, &layout)?;
    let final_production_access = crate::gameplay::placement::validate_final_access(p, map)?;
    Ok(
        json!({"placements":placed,"gift_actions":serial,"slots_per_gift":q.fallback_slots,"runtime_occupancy_check":true,"all_occupied":"retain gift eligibility and retry","planning_order":"all primary points before fallbacks; production first, sampled terrain scarcity, larger foundations, native identity","access_model":"static same-domain path or separately connected shore legs; all conditional production branches must have reachable official rally targets, ranked by their worst distance; explicit NoExitPath uses foundation access; official checks validate rotated exits","final_production_access":final_production_access,"decoration_yield":decoration_yield,"ingame_test":"not run"}),
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
    repair_slots(p, map, layout, RepairMode::Obstructed, None)
}

/// Repair obstructed points with source-bound, individually explained choices.
/// A base strategy is an explicit reselection, never an implicit consequence of
/// a small path-score improvement during local repair.
pub fn repair_obstructed_slots_with_plan(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
    plan: &RepairPlan,
) -> Result<Json> {
    if plan.schema_version != 1 {
        return Err(err("unsupported gift repair plan schema_version"));
    }
    if plan.source_native_decoded_crc32 != crc32fast::hash(&map.encode()?) {
        return Err(err(
            "gift repair plan decoded native CRC32 does not match its source",
        ));
    }
    let mut seen = BTreeSet::new();
    for directive in &plan.repairs {
        if directive.waypoint.trim().is_empty() || directive.reason.trim().is_empty() {
            return Err(err(
                "gift repair plan requires nonempty waypoints and reasons",
            ));
        }
        if !seen.insert(directive.waypoint.as_str()) {
            return Err(err(format!(
                "duplicate gift repair plan waypoint: {}",
                directive.waypoint
            )));
        }
    }
    let mut regions = BTreeSet::new();
    let native_extent = if plan
        .allowed_regions
        .iter()
        .any(|region| region.support_half_extent > 0)
    {
        let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
        Some(hm.width.min(hm.height) / 2)
    } else {
        None
    };
    for region in &plan.allowed_regions {
        if region.id.trim().is_empty()
            || !regions.insert(region.id.as_str())
            || !region.height.is_finite()
            || region.bounds.iter().any(|value| !value.is_finite())
            || region.bounds[0] >= region.bounds[2]
            || region.bounds[1] >= region.bounds[3]
            || region.support_half_extent as f64 * 2.
                > (region.bounds[2] - region.bounds[0]).min(region.bounds[3] - region.bounds[1])
            || native_extent.is_some_and(|extent| region.support_half_extent >= extent)
        {
            return Err(err(
                "gift repair plan requires unique finite, nonempty allowed_regions",
            ));
        }
    }
    repair_slots(p, map, layout, RepairMode::Obstructed, Some(plan))
}

fn reservation_findings(
    ground: &Ground<'_>,
    objects: &[MapObject],
    specs: &[Spec],
    q: &Policy,
    reservations: &FixedReservations,
    checker: Option<&crate::gameplay::official_placement::Checker>,
) -> Result<Vec<Json>> {
    let mut findings = Vec::new();
    for spec in specs {
        for slot in 0..q.fallback_slots {
            let wp = slot_name(spec, slot);
            let matching = objects
                .iter()
                .enumerate()
                .filter(|(_, object)| name(object) == wp)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return Err(err(format!("gift slot missing or duplicated: {wp}")));
            }
            let (index, object) = matching[0];
            let cell = point(ground.p, object);
            if cell[0] >= ground.n || cell[1] >= ground.z.len() / ground.n {
                return Err(err(format!("gift slot outside native height grid: {wp}")));
            }
            let polygons = checker
                .ok_or_else(|| {
                    err("complete gift reservation checking requires strict official geometry")
                })?
                .actual_reservation_polygons(object)?;
            let conflicts = reservations.findings_polygons_at(
                cell,
                ground.z[cell[1] * ground.n + cell[0]],
                &polygons,
            );
            if !conflicts.is_empty() {
                findings.push(json!({"object_id":index+1,"waypoint":wp,"seat":spec.seat,
                    "slot":slot,"production":spec.production,"cell":to_cell(ground.p,object),"navigation_sample_cell":cell,
                    "padding_cells":spec.padding,"conflicts":conflicts}));
            }
        }
    }
    Ok(findings)
}

/// Enumerate every same-layer ore disk, rotated refinery and ramp approach
/// deficit. This read-only report does not certify other delivery requirements.
pub fn audit_reservations(p: &Project, map: &MapFile, layout: &terrain::Layout) -> Result<Json> {
    let strict = crate::gameplay::official_placement::strict_project(p);
    let p = &strict;
    let q = policy(p)?;
    let source_native_decoded_crc32 = crc32fast::hash(&map.encode()?);
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
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
    let reservations = FixedReservations::new(p, layout, &objects, &ground.z, ground.n)?;
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        ground.n,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let findings = reservation_findings(
        &ground,
        &objects,
        &all_specs,
        &q,
        &reservations,
        official.as_ref(),
    )?;
    Ok(
        json!({"schema_version":1,"native_decoded_crc32":source_native_decoded_crc32,
        "checked_slots":all_specs.len()*q.fallback_slots,"passed":findings.is_empty(),
        "scope":"all gift primary/fallback foundations against complete same-layer ore/refinery and ramp approach reservations",
        "findings":findings}),
    )
}

/// Explicit start relocation: repack every existing gift foundation around its
/// assigned seat while preserving waypoint identities, guards and conditions.
pub fn relocate_for_starts(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
) -> Result<Json> {
    repair_slots(p, map, layout, RepairMode::Starts, None)
}

#[derive(Clone, Copy, PartialEq)]
enum RepairMode {
    Obstructed,
    Optimize,
    Starts,
}

fn repair_slots(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
    mode: RepairMode,
    plan: Option<&RepairPlan>,
) -> Result<Json> {
    edit_candidate(map, |candidate| {
        repair_slots_inner(p, candidate, layout, mode, plan)
    })
}

fn repair_slots_inner(
    p: &Project,
    map: &mut MapFile,
    layout: &terrain::Layout,
    mode: RepairMode,
    plan: Option<&RepairPlan>,
) -> Result<Json> {
    let strict = crate::gameplay::official_placement::strict_project(p);
    let p = &strict;
    let q = policy(p)?;
    let source_native_decoded_crc32 = crc32fast::hash(&map.encode()?);
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let original = objects.clone();
    let authored = authored_scripts(&nodes)?;
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
    let all_specs = planning_order(
        p,
        &specs(p, &authored, &objects)?,
        &ground,
        &layout.base_cells,
        &objects,
    )?;
    let reservations = FixedReservations::new(p, layout, &objects, &ground.z, ground.n)?;
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        hm.width,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let original_reservation_findings = reservation_findings(
        &ground,
        &objects,
        &all_specs,
        &q,
        &reservations,
        official.as_ref(),
    )?;
    let mut fixed = BTreeSet::new();
    let mut original_findings = BTreeMap::<String, Vec<Json>>::new();
    if mode == RepairMode::Obstructed {
        // A resource-only edit preserves every unaffected gift. Its repair set
        // is determined before placing anything, so this cannot silently turn
        // into a global optimisation of the frozen map.
        for spec in &all_specs {
            for slot in 0..q.fallback_slots {
                let wp = slot_name(spec, slot);
                let index = objects
                    .iter()
                    .position(|object| name(object) == wp)
                    .ok_or_else(|| err("missing gift slot during resource repair"))?;
                let old = point(p, &objects[index]);
                let others = objects
                    .iter()
                    .enumerate()
                    .filter(|(i, object)| {
                        *i != index && !crate::gameplay::placement::is_decoration(object)
                    })
                    .map(|(_, object)| object.clone())
                    .collect::<Vec<_>>();
                let protected = protected(p, &others, &all_specs, &q);
                let official_obstacles = official
                    .as_ref()
                    .map(|checker| checker.planned_obstacles(&others));
                let mut findings = if old[0] < ground.n && old[1] < ground.z.len() / ground.n {
                    reservations.findings_polygons_at(
                        old,
                        ground.z[old[1] * ground.n + old[0]],
                        &official.as_ref().ok_or_else(||err("complete gift reservation checking requires strict official geometry"))?
                            .actual_reservation_polygons(&objects[index])?,
                    )
                } else {
                    Vec::new()
                };
                if !ground.suitable(old, spec) {
                    findings.push(json!({"kind":"ground","detail":"full public foundation crosses blocked, nonflat, wrong-surface or out-of-bounds ground"}));
                }
                if !clear(old, spec.padding.max(8), &protected, q.clearance) {
                    findings.push(json!({"kind":"object_clearance","detail":"existing functional object or another gift reservation blocks the slot"}));
                }
                if let Some(checker) = &official {
                    findings.extend(
                        checker
                            .object_rejections(
                                &objects[index],
                                old,
                                official_obstacles.as_ref().unwrap(),
                            )
                            .into_iter()
                            .map(|detail| json!({"kind":"official_geometry","detail":detail})),
                    );
                }
                if findings.is_empty() {
                    fixed.insert(wp);
                } else {
                    original_findings.insert(wp, findings);
                }
            }
        }
    }
    let directives = plan
        .map(|plan| {
            plan.repairs
                .iter()
                .map(|directive| (directive.waypoint.as_str(), directive))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let initial_obstructed_slots = original_findings.len();
    let mut explicit_base_reselections = Vec::new();
    for (wp, directive) in &directives {
        let matched = all_specs
            .iter()
            .find(|spec| (0..q.fallback_slots).any(|slot| slot_name(spec, slot) == *wp))
            .ok_or_else(|| err(format!("unknown gift repair plan waypoint: {wp}")))?;
        let permit_reselection = permits_unobstructed_reselection(directive, matched.production)?;
        if fixed.contains(*wp) || !original_findings.contains_key(*wp) {
            if !permit_reselection {
                return Err(err(format!(
                    "gift repair plan waypoint is not obstructed: {wp}"
                )));
            }
            fixed.remove(*wp);
            original_findings.insert(
                (*wp).to_owned(),
                vec![json!({"kind":"explicit_base_reselection",
                "detail":directive.reason,"resource_deficit":false})],
            );
            explicit_base_reselections.push((*wp).to_owned());
        }
        if directive.strategy == RepairStrategy::Base && !matched.production {
            return Err(err(format!(
                "base repair strategy requires a production building: {wp}"
            )));
        }
    }
    let mut selected = BTreeSet::new();
    let access_objects = planning_objects(&objects, &all_specs, &selected, &fixed, usize::MAX);
    let mut access = BTreeMap::new();
    for spec in all_specs.iter().filter(|spec| spec.production) {
        if let std::collections::btree_map::Entry::Vacant(entry) = access.entry(spec.seat) {
            entry.insert(crate::gameplay::placement::AccessDistances::new(
                p,
                hm.width,
                &ground.z,
                &ground.imp,
                &access_objects,
                spec.seat,
            )?);
        }
    }
    let mut changes = Vec::new();
    // All primaries are settled before any fallback in every editing mode.
    for slot in 0..q.fallback_slots {
        for spec in &all_specs {
            let wp = slot_name(spec, slot);
            if fixed.contains(&wp) {
                continue;
            }
            let index = objects
                .iter()
                .position(|object| name(object) == wp)
                .ok_or_else(|| err("missing gift slot during resource repair"))?;
            let old = point(p, &objects[index]);
            let actual_before = to_cell(p, &objects[index]);
            let others = planning_objects(&objects, &all_specs, &selected, &fixed, index);
            let protected = protected(p, &others, &all_specs, &q);
            let official_obstacles = official
                .as_ref()
                .map(|checker| checker.planned_obstacles(&others));
            let base = layout.base_cells[spec.seat - 1];
            let radius = p
                .usize("PRODUCTION_SPAWN_RADIUS", search_radius(p, spec.seat)?)
                .min(ground.n);
            let origin = if mode == RepairMode::Starts && !spec.production {
                base
            } else {
                actual_before
            };
            let access_score = |cell| {
                production_access_score(access.get(&spec.seat), official.as_ref(), &wp, cell, spec)
            };
            let allowed_regions = plan
                .map(|plan| plan.allowed_regions.as_slice())
                .unwrap_or_default();
            let region_cache = RefCell::new(BTreeMap::<[usize; 2], Option<&str>>::new());
            let containing_region = |cell: [usize; 2]| {
                if let Some(result) = region_cache.borrow().get(&cell) {
                    return *result;
                }
                let result = (|| {
                    if cell[0] >= ground.n || cell[1] >= ground.z.len() / ground.n {
                        return None;
                    }
                    let height = ground.z[cell[1] * ground.n + cell[0]];
                    // Reject wrong-height, unsupported or out-of-region cells
                    // before constructing all conditional official polygons.
                    let possible = allowed_regions
                        .iter()
                        .filter(|region| {
                            let support = region.support_half_extent as f64;
                            if (region.height - height).abs() > reservations.flat_tolerance
                                || cell[0] as f64 - support < region.bounds[0]
                                || cell[1] as f64 - support < region.bounds[1]
                                || cell[0] as f64 + support > region.bounds[2]
                                || cell[1] as f64 + support > region.bounds[3]
                            {
                                return false;
                            }
                            let support_spec = Spec {
                                padding: spec.padding.max(region.support_half_extent),
                                ..spec.clone()
                            };
                            ground.suitable(cell, &support_spec)
                        })
                        .collect::<Vec<_>>();
                    if possible.is_empty() {
                        return None;
                    }
                    let bounds = official
                        .as_ref()?
                        .candidate_reservation_bounds(&wp, cell, q.clearance as f64)
                        .ok()?;
                    possible
                        .into_iter()
                        .find(|region| {
                            bounds[0] >= region.bounds[0]
                                && bounds[1] >= region.bounds[1]
                                && bounds[2] <= region.bounds[2]
                                && bounds[3] <= region.bounds[3]
                        })
                        .map(|region| region.id.as_str())
                })();
                region_cache.borrow_mut().insert(cell, result);
                result
            };
            let candidate_cache = RefCell::new(BTreeMap::<[usize; 2], bool>::new());
            let acceptable = |cell| {
                if let Some(result) = candidate_cache.borrow().get(&cell) {
                    return *result;
                }
                let result = ground.suitable(cell, spec)
                    && clear(cell, spec.padding.max(8), &protected, q.clearance)
                    && (allowed_regions.is_empty() || containing_region(cell).is_some())
                    && reservations.clear_candidate(&ground, cell, spec, official.as_ref(), &wp)
                    && official.as_ref().is_none_or(|checker| {
                        checker.acceptable(&wp, cell, official_obstacles.as_ref().unwrap())
                    });
                candidate_cache.borrow_mut().insert(cell, result);
                result
            };
            let choose = |local| {
                choose_point(
                    &ground,
                    spec,
                    &Selection {
                        base,
                        origin,
                        radius,
                        local,
                    },
                    Some(&access_score),
                    acceptable,
                )
            };
            let directive = directives.get(wp.as_str()).copied();
            let strategy = directive
                .map(|directive| directive.strategy)
                .unwrap_or(RepairStrategy::Local);
            let (local_candidate, base_candidate, target) = if mode == RepairMode::Obstructed {
                let local = choose(true);
                let base = spec.production.then(|| choose(false)).flatten();
                (
                    local,
                    base,
                    match strategy {
                        RepairStrategy::Local => local,
                        RepairStrategy::Base => base,
                    },
                )
            } else {
                (None, None, choose(false))
            };
            let target = target
            .ok_or_else(|| err(format!("no clear replacement gift slot {wp} near P{}: no candidate satisfies base access, complete foundation, ramp approaches, ore/refinery reservations and official exits", spec.seat)))?;
            let candidate_info = |cell: Option<[usize; 2]>| {
                cell.map(|cell|json!({
                "cell":cell,"displacement_cells":(cell[0] as f64-actual_before[0]).hypot(cell[1] as f64-actual_before[1]),
                "base_straight_distance_cells":(cell[0] as f64-base[0]).hypot(cell[1] as f64-base[1]),
                "planning_base_access_steps":if spec.production {access_score(cell)}else{None},
                "allowed_region_id":if allowed_regions.is_empty(){None}else{containing_region(cell)}
            }))
            };
            move_to(p, &mut objects[index], target);
            let actual_after = to_cell(p, &objects[index]);
            selected.insert(wp.clone());
            if let Some(displacement) = actual_displacement(actual_before, actual_after) {
                changes.push(json!({"id":index+1,"waypoint":wp,"seat":spec.seat,"slot":slot,
                    "production":spec.production,"before":actual_before,"after":actual_after,
                    "displacement_cells":displacement,
                    "strategy":if mode==RepairMode::Obstructed{json!(strategy)}else{json!("replan")},
                    "reason":directive.map(|directive|directive.reason.as_str()).unwrap_or(
                        if mode==RepairMode::Obstructed{"local repair of an obstructed slot; minimum displacement among reachable legal candidates"}
                        else{"explicit whole-layout or start relocation"}),
                    "original_findings":original_findings.get(&wp),
                    "original_candidate":{"cell":actual_before,"navigation_sample_cell":old,
                        "displacement_cells":0.,"base_straight_distance_cells":(actual_before[0]-base[0]).hypot(actual_before[1]-base[1]),
                        "planning_base_access_steps":if spec.production {access_score(old)}else{None}},
                    "local_candidate":candidate_info(local_candidate),
                    "base_candidate":candidate_info(base_candidate),
                    "selected_candidate":candidate_info(Some(target))}));
            }
        }
    }
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    sync_moved_areas(map, &original, &objects)?;
    crate::gameplay::production_facing::apply(p, map)?;
    let decoration_yield = crate::gameplay::decoration::yield_to_functional(p, map, layout)?;
    validate_with_layout(p, map, layout)?;
    let final_production_access = crate::gameplay::placement::validate_final_access(p, map)?;
    Ok(json!({
        "changes":changes,
        "source_native_decoded_crc32":source_native_decoded_crc32,
        "initial_obstructed_slots":initial_obstructed_slots,
        "explicit_base_reselections":explicit_base_reselections,
        "original_reservation_audit":{"native_decoded_crc32":source_native_decoded_crc32,
            "checked_slots":all_specs.len()*q.fallback_slots,
            "passed":original_reservation_findings.is_empty(),"findings":original_reservation_findings},
        "repair_plan":plan,
        "local_ranking":"minimum displacement, then reachable static base access and base straight distance; hard constraints remain mandatory",
        "base_ranking":"explicit production reselection by static base access, base straight distance and displacement",
        "allowed_regions_enforced":plan.is_some_and(|plan|!plan.allowed_regions.is_empty()),
        "allowed_regions_scope":"complete geometry of moved candidates only; unchanged slots retain separate independent review",
        "allowed_region_support_scope":"optional authored flat terrain support applies to moved candidates only; it does not enlarge entity, resource-clearance or navigation footprints",
        "planning_order":"all primary points before fallbacks; production first, sampled terrain scarcity, larger foundations, native identity",
        "access_model":"static same-domain path or separately connected shore legs; all conditional production branches must have reachable official rally targets, ranked by their worst distance; explicit NoExitPath uses foundation access; official checks validate rotated exits",
        "final_production_access":final_production_access,
        "decoration_yield":decoration_yield,
        "preserved":"guard IDs, scripts, conditions, flags and waypoint identities"
    }))
}

pub fn validate(p: &Project, map: &MapFile) -> Result<()> {
    validate_with_layout(p, map, &terrain::Layout::load(p)?)
}

pub(crate) fn validate_with_layout(
    p: &Project,
    map: &MapFile,
    layout: &terrain::Layout,
) -> Result<()> {
    let strict = crate::gameplay::official_placement::strict_project(p);
    let p = &strict;
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
    let reservations = FixedReservations::new(p, layout, &objects, &ground.z, ground.n)?;
    let authored = authored_scripts(&nodes)?;
    let all_specs = specs(p, &authored, &objects)?;
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p,
        ground.n,
        &ground.z,
        &ground.imp,
        &ground.blocked,
        &nodes,
        &objects,
    )?;
    let deficits = reservation_findings(
        &ground,
        &objects,
        &all_specs,
        &q,
        &reservations,
        official.as_ref(),
    )?;
    if let Some(first) = deficits.first() {
        let waypoint = first["waypoint"].as_str().unwrap();
        return Err(err(format!(
            "gift slot enters a complete ramp approach or ore/refinery reservation: {waypoint} at {}; {} affected gift slots; complete findings: {}",
            first["cell"],
            deficits.len(),
            serde_json::to_string(&deficits)?
        )));
    }
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
            // Complete resource/approach geometry was checked above at the
            // saved fractional position and explicit branch heading. Reusing
            // an integer candidate here would silently snap the source audit.
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
        assert_eq!(kind("AlliedAirfield", &production), Some((false, 8)));
        assert_eq!(kind("SovietAirField", &production), Some((false, 8)));
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let public = crate::gameplay::production_spawns::PlacementPolicy::load(&p).unwrap();
        // Recognizing every producer must not reduce the established airport
        // footprint or invalidate an unchanged 270-world-unit runtime guard.
        let (_, airport_padding) = kind("AlliedAirfield", &public).unwrap();
        let q = policy(&p).unwrap();
        assert_eq!(airport_padding, 8);
        assert_eq!(
            (airport_padding + q.runtime_blocker_radius + q.clearance) as f64
                * p.number("CELL", 10.),
            270.
        );
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
            production: true,
        };
        assert!(ground.suitable([64, 64], &spec));
        ground.blocked[64 * 128 + 73] = true;
        assert!(!ground.suitable([64, 64], &spec));
        let smaller = Spec { padding: 6, ..spec };
        assert!(ground.suitable([64, 64], &smaller));
    }

    fn planning_project() -> Project {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        p.cfg["GRID"] = json!(160);
        p.cfg["GRID_HEIGHT"] = json!(160);
        p
    }

    fn waypoint(p: &Project, waypoint: &str, cell: [f64; 2]) -> MapObject {
        let position = crate::coordinates::Frame::from_project(p).world(cell);
        let mut object = MapObject {
            version: 3,
            position: [position[0] as f32, position[1] as f32, 0.],
            angle: 0.,
            road_type: 0,
            type_name: "Waypoint".into(),
            props: Vec::new(),
        };
        map::set(
            &mut object.props,
            "waypointName",
            Value::Ascii(waypoint.into()),
        );
        object
    }

    #[test]
    fn failed_gift_edit_does_not_leak_native_mutations() {
        let mut map = MapFile {
            order: vec![("ObjectsList".into(), 1)],
            table: BTreeMap::from([(1, "ObjectsList".into())]),
            chunks: vec![map::Chunk {
                name_idx: 1,
                version: 1,
                payload: vec![0],
            }],
        };
        let original = map.clone();
        let result = edit_candidate(&mut map, |candidate| {
            candidate.get_mut("ObjectsList")?.payload = vec![1, 2, 3];
            candidate.id("NewGiftIdentity")?;
            Err(err("injected failure after native edits"))
        });
        assert!(result.is_err());
        assert_eq!(map.chunks, original.chunks);
        assert_eq!(map.table, original.table);
        assert_eq!(map.order, original.order);
        edit_candidate(&mut map, |candidate| {
            candidate.get_mut("ObjectsList")?.payload = vec![4];
            Ok(json!({"passed": true}))
        })
        .unwrap();
        assert_eq!(map.get("ObjectsList").unwrap().payload, [4]);
    }

    #[test]
    fn valid_point_reselection_requires_explicit_production_base_authorization() {
        let default:RepairDirective=serde_json::from_value(json!({"waypoint":"production-site",
            "strategy":"base","reason":"independent comparison identifies a better base connection"})).unwrap();
        assert!(!permits_unobstructed_reselection(&default, true).unwrap());
        let allowed = RepairDirective {
            reselect_unobstructed: true,
            ..default
        };
        assert!(permits_unobstructed_reselection(&allowed, true).unwrap());
        assert!(permits_unobstructed_reselection(&allowed, false).is_err());
        assert!(
            permits_unobstructed_reselection(
                &RepairDirective {
                    strategy: RepairStrategy::Local,
                    ..allowed.clone()
                },
                true
            )
            .is_err()
        );
        assert!(
            permits_unobstructed_reselection(
                &RepairDirective {
                    reason: " ".into(),
                    ..allowed
                },
                true
            )
            .is_err()
        );
    }

    #[test]
    fn gift_recipient_comes_from_each_actual_create_action_not_the_waypoint_name() {
        let p = planning_project();
        let wp = "dual-p2-misleading-name";
        let objects = vec![waypoint(&p, wp, [120., 120.])];
        let create = |team: &str| {
            scripts::leaf(
                "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION",
                467,
                &[
                    (15, 0, 0., "AlliedWarFactory"),
                    (3, 0, 0., team),
                    (7, 0, 0., wp),
                    (17, 0, std::f32::consts::FRAC_PI_4, ""),
                ],
                false,
                false,
            )
            .unwrap()
        };
        for (team, expected) in [
            ("Player_1/teamPlayer_1", 1),
            ("PlyrCivilian/teamPlyrCivilian", 7),
            ("PlyrCreeps/teamPlyrCreeps", 8),
        ] {
            let actual = specs(&p, &[create(team)], &objects).unwrap();
            assert_eq!(actual[0].seat, expected);
        }
        let error = specs(
            &p,
            &[
                create("Player_1/teamPlayer_1"),
                create("Player_2/teamPlayer_2"),
            ],
            &objects,
        )
        .unwrap_err();
        assert!(error.to_string().contains("conflicting recipient seats"));
        assert!(specs(&p, &[create("Unknown/teamUnknown")], &objects).is_err());
    }

    #[test]
    fn power_plant_refinery_gap_uses_actual_foundation_and_gift_clearance() {
        let p = planning_project();
        let policy = terrain::ramp_clearance::Policy::default();
        let spawn = terrain::spawn_placement::policy(&p).unwrap();
        let angle = -2.356_194_5_f64;
        let center = [724., 82.];
        let field = terrain::ramp_clearance::rectangle(
            center,
            [angle.cos(), -angle.sin()],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        );
        let mut reservations = FixedReservations {
            policy,
            mine_gap: decoration_clearance(&p).unwrap() as f64,
            ore_radius: spawn.ore_radius,
            flat_tolerance: spawn.flat_tolerance,
            ramps: Vec::new(),
            ores: vec![(center, field, 210.)],
            ore_ids: vec![1],
        };
        // A four-cell power foundation has six cells of actual ore clearance.
        // The larger slot keepout and the ramp's gap are different policies.
        assert_eq!(reservations.mine_gap, 3.);
        assert_eq!(reservations.policy.gap, 8.);
        assert!(reservations.clear_at([696, 68], 4, 210.));
        assert!(!reservations.clear_at([696, 68], 8, 210.));
        reservations.ores[0].2 = 280.;
        assert!(reservations.clear_at([696, 68], 8, 210.));
    }

    #[test]
    fn complete_ore_disk_rejects_a_real_one_cell_superweapon_clearance_deficit() {
        let p = planning_project();
        let policy = terrain::ramp_clearance::Policy::default();
        let spawn = terrain::spawn_placement::policy(&p).unwrap();
        let angle = -2.356_194_5_f64;
        let ore = [764., 460.];
        let field = terrain::ramp_clearance::rectangle(
            ore,
            [angle.cos(), -angle.sin()],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        );
        let reservations = FixedReservations {
            policy,
            mine_gap: decoration_clearance(&p).unwrap() as f64,
            ore_radius: spawn.ore_radius,
            flat_tolerance: spawn.flat_tolerance,
            ramps: Vec::new(),
            ores: vec![(ore, field, 210.)],
            ore_ids: vec![1],
        };
        // Existing SW83 backup: 28 center distance - 6 foundation - 20 ore = 2,
        // one cell less than the public gift clearance. Keep rejecting it.
        assert!(!reservations.clear_at([764, 432], 6, 210.));
        assert!(reservations.clear_at([764, 431], 6, 210.));
    }

    #[test]
    fn production_prefers_actual_base_access_over_a_closer_point_across_a_wall() {
        let p = planning_project();
        let mut ground = Ground {
            p: &p,
            n: 160,
            z: vec![212.; 160 * 160],
            imp: vec![false; 160 * 160],
            blocked: vec![false; 160 * 160],
        };
        for y in 0..110 {
            ground.imp[y * ground.n + 60] = true;
        }
        let objects = vec![waypoint(&p, "Player_1_Start", [40., 64.])];
        let access = crate::gameplay::placement::AccessDistances::new(
            &p,
            ground.n,
            &ground.z,
            &ground.imp,
            &objects,
            1,
        )
        .unwrap();
        let spec = Spec {
            name: "factory".into(),
            wet: false,
            padding: 6,
            seat: 1,
            production: true,
        };
        let across_wall = [75, 64];
        let accessible = [40, 105];
        let base = [40., 64.];
        let direct_distance =
            |cell: [usize; 2]| (cell[0] as f64 - base[0]).hypot(cell[1] as f64 - base[1]);
        assert!(direct_distance(across_wall) < direct_distance(accessible));
        assert!(access.score(across_wall, false, 6) > access.score(accessible, false, 6));
        let selected = choose_point(
            &ground,
            &spec,
            &Selection {
                base: [40., 64.],
                origin: across_wall.map(|value| value as f64),
                radius: 120,
                local: false,
            },
            Some(&|cell| access.score(cell, false, 6)),
            |cell| [across_wall, accessible].contains(&cell) && ground.suitable(cell, &spec),
        );
        assert_eq!(selected, Some(accessible));
    }

    #[test]
    fn local_resource_repair_minimizes_displacement_without_accepting_unreachable_points() {
        let p = planning_project();
        let ground = Ground {
            p: &p,
            n: 160,
            z: vec![212.; 160 * 160],
            imp: vec![false; 160 * 160],
            blocked: vec![false; 160 * 160],
        };
        let spec = Spec {
            name: "factory".into(),
            wet: false,
            padding: 6,
            seat: 1,
            production: true,
        };
        let nearby = [99, 100];
        let base_side = [60, 64];
        let choose = |local: bool, nearby_reachable: bool| {
            choose_point(
                &ground,
                &spec,
                &Selection {
                    base: [40., 64.],
                    origin: [100., 100.],
                    radius: 120,
                    local,
                },
                Some(&|cell| {
                    if cell == nearby {
                        nearby_reachable.then_some(300)
                    } else {
                        Some(1)
                    }
                }),
                |cell| [nearby, base_side].contains(&cell),
            )
        };
        assert_eq!(choose(true, true), Some(nearby));
        assert_eq!(choose(false, true), Some(base_side));
        assert_eq!(choose(true, false), Some(base_side));
    }

    #[test]
    fn fractional_local_origin_controls_distance_and_same_sample_moves_are_recorded() {
        let p = planning_project();
        let mut object = waypoint(&p, "fractional-site", [100.25, 100.]);
        let before = to_cell(&p, &object);
        let sample = point(&p, &object);
        move_to(&p, &mut object, sample);
        assert_eq!(point(&p, &object), sample);
        assert_eq!(
            actual_displacement(before, to_cell(&p, &object)),
            Some(0.25)
        );
        let ground = Ground {
            p: &p,
            n: 160,
            z: vec![212.; 160 * 160],
            imp: vec![false; 160 * 160],
            blocked: vec![false; 160 * 160],
        };
        let spec = Spec {
            name: "fractional-site".into(),
            wet: false,
            padding: 6,
            seat: 1,
            production: true,
        };
        let west = [99, 100];
        let east = [101, 100];
        let selected = choose_point(
            &ground,
            &spec,
            &Selection {
                base: [40., 64.],
                origin: [100.49, 100.1],
                radius: 120,
                local: true,
            },
            Some(&|cell| Some(if cell == west { 1 } else { 2 })),
            |cell| [west, east].contains(&cell),
        );
        // Rounded origin would tie the displacements and prefer the west
        // point's access score. Actual source distance prefers the east point.
        assert_eq!(selected, Some(east));
    }

    #[test]
    fn reservation_diagnostics_enumerate_every_same_layer_conflict() {
        let policy = terrain::ramp_clearance::Policy::default();
        let make_field = |center| {
            terrain::ramp_clearance::rectangle(
                center,
                [0., -1.],
                -policy.refinery_back,
                policy.refinery_forward,
                policy.refinery_half_width,
            )
        };
        let ore_a = [80., 80.];
        let ore_b = [87., 80.];
        let reservations = FixedReservations {
            policy: policy.clone(),
            mine_gap: 3.,
            ore_radius: 20.,
            flat_tolerance: 3.,
            ramps: Vec::new(),
            ores: vec![
                (ore_a, make_field(ore_a), 212.),
                (ore_b, make_field(ore_b), 212.),
            ],
            ore_ids: vec![21, 22],
        };
        let findings = reservations.findings_at([80, 54], 6, 212.);
        let ids = findings
            .iter()
            .filter(|finding| finding["kind"] == "ore_disk")
            .map(|finding| finding["ore_id"].as_u64().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids, BTreeSet::from([21, 22]));
        assert!(!reservations.clear_at([80, 54], 6, 212.));
        assert!(reservations.findings_at([80, 54], 6, 280.).is_empty());
        assert!(reservations.clear_at([80, 54], 6, 280.));
    }

    #[test]
    fn complete_geometry_can_fail_when_the_public_core_alone_clears_resources() {
        let policy = terrain::ramp_clearance::Policy::default();
        let ore = [80., 80.];
        let field = terrain::ramp_clearance::rectangle(
            ore,
            [1., 0.],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        );
        let reservations = FixedReservations {
            policy,
            mine_gap: 3.,
            ore_radius: 20.,
            flat_tolerance: 3.,
            ramps: Vec::new(),
            ores: vec![(ore, field, 212.)],
            ore_ids: vec![17],
        };
        let cell = [80, 50];
        let public = terrain::ramp_clearance::rectangle([80., 50.], [1., 0.], -4., 4., 4.).to_vec();
        let official =
            terrain::ramp_clearance::rectangle([80., 50.], [1., 0.], -8., 8., 8.).to_vec();
        assert!(reservations.clear_at(cell, 4, 212.));
        assert!(!reservations.clear_polygons_at(cell, 212., &[public.clone(), official.clone()]));
        let findings = reservations.findings_polygons_at(cell, 212., &[public, official]);
        assert!(findings.iter().any(|finding| finding["kind"] == "ore_disk"
            && finding["ore_id"] == 17
            && finding["gap_cells"] == 2.));
    }

    #[test]
    fn fractional_existing_geometry_does_not_round_a_resource_deficit_into_a_pass() {
        let policy = terrain::ramp_clearance::Policy::default();
        let ore = [80., 80.];
        let field = terrain::ramp_clearance::rectangle(
            ore,
            [1., 0.],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        );
        let reservations = FixedReservations {
            policy,
            mine_gap: 3.,
            ore_radius: 20.,
            flat_tolerance: 3.,
            ramps: Vec::new(),
            ores: vec![(ore, field, 212.)],
            ore_ids: vec![17],
        };
        // The navigation sample stays at y=51. Its integer public square has
        // gap 3, but the saved source geometry at y=51.25 has only 2.75.
        assert!(reservations.clear_at([80, 51], 6, 212.));
        let actual =
            terrain::ramp_clearance::rectangle([80., 51.25], [1., 0.], -6., 6., 6.).to_vec();
        assert!(!reservations.clear_polygons_at([80, 51], 212., std::slice::from_ref(&actual)));
        let findings = reservations.findings_polygons_at([80, 51], 212., &[actual]);
        assert!(
            findings
                .iter()
                .any(|finding| finding["kind"] == "ore_disk" && finding["gap_cells"] == 2.75)
        );
    }

    #[test]
    fn convex_distance_handles_points_and_diagonal_exit_sweeps_without_box_inflation() {
        let diagonal = vec![
            [10., 9.],
            [11., 9.],
            [21., 19.],
            [21., 21.],
            [19., 21.],
            [9., 11.],
        ];
        assert_eq!(convex_polygon_distance(&diagonal, &[[15., 15.]]), 0.);
        assert!(convex_polygon_distance(&diagonal, &[[10., 20.]]) > 5.);
        let a = terrain::ramp_clearance::rectangle([30., 30.], [1., 0.], -4., 4., 4.);
        let b = terrain::ramp_clearance::rectangle([46., 30.], [1., 0.], -4., 4., 4.);
        assert_eq!(convex_polygon_distance(&a, &b), 8.);
    }

    #[test]
    fn deferred_backup_does_not_reserve_space_before_another_primary() {
        let p = planning_project();
        let spec = Spec {
            name: "factory".into(),
            wet: false,
            padding: 6,
            seat: 1,
            production: true,
        };
        let other = Spec {
            name: "other-factory".into(),
            ..spec.clone()
        };
        let backup = slot_name(&other, 1);
        let objects = vec![
            waypoint(&p, &spec.name, [60., 64.]),
            waypoint(&p, &backup, [60., 64.]),
        ];
        let q = policy(&p).unwrap();
        let fixed = BTreeSet::new();
        let selected = BTreeSet::new();
        let specs = [spec.clone(), other];
        let others = planning_objects(&objects, &specs, &selected, &fixed, 0);
        assert!(clear(
            [60, 64],
            8,
            &protected(&p, &others, &specs, &q),
            q.clearance
        ));
        let selected = BTreeSet::from([backup]);
        let others = planning_objects(&objects, &specs, &selected, &fixed, 0);
        assert!(!clear(
            [60, 64],
            8,
            &protected(&p, &others, &specs, &q),
            q.clearance
        ));
    }

    #[test]
    fn scarce_shipyard_primary_is_ordered_before_abundant_land_and_nonproduction() {
        let p = planning_project();
        let mut ground = Ground {
            p: &p,
            n: 160,
            z: vec![212.; 160 * 160],
            imp: vec![false; 160 * 160],
            blocked: vec![false; 160 * 160],
        };
        for y in 0..160 {
            for x in 105..160 {
                ground.z[y * 160 + x] = 130.;
            }
        }
        let spec = Spec {
            name: "a-power".into(),
            wet: false,
            padding: 4,
            seat: 1,
            production: false,
        };
        let land = Spec {
            name: "b-factory".into(),
            padding: 6,
            production: true,
            ..spec.clone()
        };
        let water = Spec {
            name: "z-shipyard".into(),
            padding: 8,
            wet: true,
            production: true,
            ..spec.clone()
        };
        let specs = [spec, land, water];
        let objects = specs
            .iter()
            .map(|spec| waypoint(&p, &spec.name, [40., 64.]))
            .collect::<Vec<_>>();
        let ordered = planning_order(&p, &specs, &ground, &[[40., 64.]], &objects).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|spec| spec.name.as_str())
                .collect::<Vec<_>>(),
            ["z-shipyard", "b-factory", "a-power"]
        );
    }

    #[test]
    fn complete_approach_and_rotated_refinery_reservations_remain_hard_constraints() {
        let p = planning_project();
        let ground = Ground {
            p: &p,
            n: 160,
            z: vec![212.; 160 * 160],
            imp: vec![false; 160 * 160],
            blocked: vec![false; 160 * 160],
        };
        let spec = Spec {
            name: "factory".into(),
            wet: false,
            padding: 6,
            seat: 1,
            production: true,
        };
        let policy = terrain::ramp_clearance::Policy::default();
        let ramp = terrain::ramp_clearance::Ramp {
            id: 1,
            center: [80., 80.],
            unit: [1., 0.],
            half: 10.,
            width: 20.,
            pair: [212., 280.],
            approach: terrain::ramp_clearance::rectangle([80., 80.], [1., 0.], -35., 35., 10.),
        };
        let reservations = FixedReservations {
            policy: policy.clone(),
            mine_gap: 3.,
            ore_radius: 20.,
            flat_tolerance: 3.,
            ramps: vec![ramp],
            ores: Vec::new(),
            ore_ids: Vec::new(),
        };
        assert!(!reservations.clear(&ground, [44, 80], &spec));
        assert!(reservations.clear(&ground, [30, 40], &spec));
        let field = terrain::ramp_clearance::rectangle(
            [80., 80.],
            [0., -1.],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        );
        let reservations = FixedReservations {
            policy,
            mine_gap: 3.,
            ore_radius: 20.,
            flat_tolerance: 3.,
            ramps: Vec::new(),
            ores: vec![([80., 80.], field, 212.)],
            ore_ids: vec![1],
        };
        assert!(!reservations.clear(&ground, [80, 40], &spec));
        assert!(reservations.clear(&ground, [40, 80], &spec));
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
