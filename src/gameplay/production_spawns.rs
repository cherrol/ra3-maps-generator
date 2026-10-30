//! 检查和修正赠送建筑的水陆、平坦度、完整占地及配套 XML。
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    gameplay::script_spawns::{clear, keepouts, to_cell},
    gameplay::scripts::{self, Node},
    pipeline::project::Project,
    terrain,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

const POLICY_SOURCE: &str = "templates/rules/common-T0.5.1.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FootprintRule {
    surface: String,
    padding: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PlacementPolicy {
    full_footprint: bool,
    war_factory: FootprintRule,
    naval_yard: FootprintRule,
}

impl PlacementPolicy {
    /// Every ruleset, including the map-specific legacy scripts, shares this policy.
    pub(crate) fn load(p: &Project) -> Result<Self> {
        #[derive(Deserialize)]
        struct CommonRules {
            production_placement: PlacementPolicy,
        }
        let common: CommonRules =
            serde_json::from_slice(&std::fs::read(p.root.join(POLICY_SOURCE))?)?;
        let policy = common.production_placement;
        if !policy.full_footprint
            || policy.war_factory.surface != "land"
            || policy.naval_yard.surface != "water"
            || !(6..=16).contains(&policy.war_factory.padding)
            || !(8..=16).contains(&policy.naval_yard.padding)
        {
            return Err(err(
                "common production placement requires full land factory and water shipyard footprints",
            ));
        }
        Ok(policy)
    }

    fn padding(&self, wet: bool) -> usize {
        if wet {
            self.naval_yard.padding
        } else {
            self.war_factory.padding
        }
    }

    pub(crate) fn footprint(&self, name: &str) -> Option<(bool, usize)> {
        water_only(name).map(|wet| (wet, self.padding(wet)))
    }
}

fn water_only(name: &str) -> Option<bool> {
    let name = name.to_ascii_lowercase();
    if name.ends_with("navalyard") || name == "shipyardtechstructure" {
        Some(true)
    } else if super::placement::is_production(&name) {
        Some(false)
    } else {
        None
    }
}

/// Resolve actual create actions rather than assuming a faction or waypoint name.
pub(crate) fn bindings(nodes: &[Node], objects: &[MapObject]) -> Result<BTreeMap<usize, bool>> {
    let mut out = BTreeMap::new();
    for (i, o) in objects.iter().enumerate() {
        if let Some(wet) = water_only(&o.type_name) {
            out.insert(i, wet);
        }
    }
    fn visit(nodes: &[Node], objects: &[MapObject], out: &mut BTreeMap<usize, bool>) -> Result<()> {
        for node in nodes {
            if let Some(children) = &node.children {
                visit(children, objects, out)?;
            } else if node
                .command
                .as_deref()
                .is_some_and(|c| c.starts_with("CREATE_") && c.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(node)?;
                let Some(wet) = args
                    .iter()
                    .find_map(|(ty, name)| (*ty == 15).then(|| water_only(name)).flatten())
                else {
                    continue;
                };
                let name = args
                    .iter()
                    .find(|(ty, _)| *ty == 7)
                    .ok_or_else(|| err("production create action has no waypoint"))?
                    .1
                    .as_str();
                let indices: Vec<_> = objects
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| {
                        prop(&o.props, "waypointName").and_then(Value::text) == Some(name)
                    })
                    .map(|(i, _)| i)
                    .collect();
                if indices.len() != 1 {
                    return Err(err(format!(
                        "production waypoint missing or duplicated: {name}"
                    )));
                }
                if out.insert(indices[0], wet).is_some_and(|old| old != wet) {
                    return Err(err(format!("factory and shipyard share waypoint {name}")));
                }
            }
        }
        Ok(())
    }
    visit(nodes, objects, &mut out)?;
    Ok(out)
}

/// A seat comes from the actual create action's team, never a nearby base.
pub(crate) fn owners(nodes: &[Node], objects: &[MapObject]) -> Result<BTreeMap<usize, usize>> {
    let mut seats = BTreeMap::new();
    fn visit(
        nodes: &[Node],
        objects: &[MapObject],
        seats: &mut BTreeMap<usize, usize>,
    ) -> Result<()> {
        for node in nodes {
            if let Some(children) = &node.children {
                visit(children, objects, seats)?;
                continue;
            }
            if !node.command.as_deref().is_some_and(|command| {
                command.starts_with("CREATE_") && command.contains("AT_WAYPOINT")
            }) {
                continue;
            }
            let args = scripts::string_arguments(node)?;
            if !args
                .iter()
                .any(|(kind, value)| *kind == 15 && water_only(value).is_some())
            {
                continue;
            }
            let team = args
                .iter()
                .find(|(kind, _)| *kind == 3)
                .ok_or_else(|| err("production create action has no owning team"))?;
            let seat = super::production_facing::seat(&team.1)?;
            let name = args
                .iter()
                .find(|(kind, _)| *kind == 7)
                .ok_or_else(|| err("production create action has no waypoint"))?;
            let indices: Vec<_> = objects
                .iter()
                .enumerate()
                .filter(|(_, object)| {
                    prop(&object.props, "waypointName").and_then(Value::text)
                        == Some(name.1.as_str())
                })
                .map(|(index, _)| index)
                .collect();
            if indices.len() != 1 {
                return Err(err("production owner requires a unique waypoint"));
            }
            if seats
                .insert(indices[0], seat)
                .is_some_and(|old| old != seat)
            {
                return Err(err("production waypoint has multiple owning seats"));
            }
        }
        Ok(())
    }
    visit(nodes, objects, &mut seats)?;
    // Native player-owned production can use its explicit player identity.
    // Neutral/native objects with no verified seat retain their original site.
    for (index, object) in objects.iter().enumerate() {
        if super::placement::is_production(&object.type_name)
            && let Some(seat) = prop(&object.props, "playerName")
                .and_then(Value::text)
                .and_then(|name| super::production_facing::seat(name).ok())
        {
            seats.insert(index, seat);
        }
    }
    Ok(seats)
}

pub(crate) fn reservations(
    p: &Project,
    nodes: &[Node],
    objects: &[MapObject],
) -> Result<Vec<([f64; 2], f64)>> {
    let policy = PlacementPolicy::load(p)?;
    Ok(bindings(nodes, objects)?
        .into_iter()
        .filter(|(index, _)| {
            !prop(&objects[*index].props, "waypointName")
                .and_then(Value::text)
                .is_some_and(|name| name.starts_with("gift-alt-"))
        })
        .map(|(index, wet)| (to_cell(p, &objects[index]), policy.padding(wet) as f64))
        .collect())
}

struct Ground<'a> {
    p: &'a Project,
    n: usize,
    z: &'a [f64],
    impassable: &'a [bool],
    blocked: &'a [bool],
    policy: PlacementPolicy,
}
impl Ground<'_> {
    fn suitable(&self, c: [usize; 2], wet: bool) -> bool {
        let [x, y] = c;
        let padding = self.policy.padding(wet);
        let border = self.p.usize("BORDER", 20) + padding + 2;
        if x < border
            || y < border
            || x >= self.n.saturating_sub(border)
            || y >= (self.z.len() / self.n).saturating_sub(border)
        {
            return false;
        }
        let water = self.p.number("WATER_Z", 200.);
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for yy in y - padding..=y + padding {
            for xx in x - padding..=x + padding {
                let i = yy * self.n + xx;
                if (if wet {
                    self.z[i] >= water - 1.
                } else {
                    self.z[i] <= water
                }) || self.impassable[i]
                    || self.blocked[i]
                {
                    return false;
                }
                lo = lo.min(self.z[i]);
                hi = hi.max(self.z[i]);
            }
        }
        wet || hi - lo <= 1.
    }
}
fn cell(p: &Project, o: &MapObject) -> [usize; 2] {
    to_cell(p, o).map(|v| v.round_ties_even() as usize)
}

fn clear_functional_reservations(
    reservations: &super::gift_clearance::FixedReservations,
    official: &super::official_placement::Checker,
    object: &MapObject,
    point: [usize; 2],
    height: f64,
) -> Result<bool> {
    let polygons = official.object_reservation_polygons(object, point)?;
    Ok(reservations.clear_polygons_at(point, height, &polygons))
}

#[allow(clippy::too_many_arguments)]
pub fn repair(
    p: &Project,
    n: usize,
    z: &[f64],
    impassable: &[bool],
    blocked: &[bool],
    nodes: &[Node],
    objects: &mut [MapObject],
) -> Result<Json> {
    repair_with_layout(
        p,
        n,
        z,
        impassable,
        blocked,
        nodes,
        objects,
        &terrain::Layout::load(p)?,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn repair_with_layout(
    p: &Project,
    n: usize,
    z: &[f64],
    impassable: &[bool],
    blocked: &[bool],
    nodes: &[Node],
    objects: &mut [MapObject],
    layout: &terrain::Layout,
) -> Result<Json> {
    use super::placement::{AccessDistances, is_decoration};
    use std::collections::BTreeSet;
    let frame = crate::coordinates::Frame::from_project(p);
    let ground = Ground {
        p,
        n,
        z,
        impassable,
        blocked,
        policy: PlacementPolicy::load(p)?,
    };
    let targets: BTreeMap<_, _> = bindings(nodes, objects)?
        .into_iter()
        .filter(|(index, _)| {
            !prop(&objects[*index].props, "waypointName")
                .and_then(Value::text)
                .is_some_and(|name| name.starts_with("gift-alt-"))
        })
        .collect();
    let reservations = if !targets.is_empty() {
        Some(super::gift_clearance::FixedReservations::new(
            p, layout, objects, z, n,
        )?)
    } else {
        None
    };
    let owners = owners(nodes, objects)?;
    let lower_waypoints: BTreeSet<_> = scripts::load_rules(p)?
        .spawn_waypoints
        .into_iter()
        .map(|spec| spec.name)
        .collect();
    let moving: BTreeSet<_> = targets
        .keys()
        .filter(|index| owners.contains_key(index))
        .copied()
        .collect();
    let fixed: Vec<_> = objects
        .iter()
        .enumerate()
        .filter(|(index, object)| {
            !moving.contains(index)
                && !is_decoration(object)
                && !prop(&object.props, "waypointName")
                    .and_then(Value::text)
                    .is_some_and(|name| name.starts_with("gift-alt-"))
                && !prop(&object.props, "waypointName")
                    .and_then(Value::text)
                    .is_some_and(|name| lower_waypoints.contains(name))
        })
        .map(|(_, object)| object.clone())
        .collect();
    let fixed_keepouts = keepouts(p, &fixed);
    let official = crate::gameplay::official_placement::Checker::for_candidates(
        p, n, z, impassable, blocked, nodes, objects,
    )?;
    if reservations.is_some() && official.is_none() {
        return Err(err(
            "production resource reservations require resolved official geometry",
        ));
    }
    let mut access = BTreeMap::new();
    type Candidate = (u32, f64, f64, usize, usize);
    struct Plan {
        index: usize,
        wet: bool,
        seat: Option<usize>,
        base: Option<[f64; 2]>,
        from: [usize; 2],
        candidates: Vec<Candidate>,
    }
    let mut plans = Vec::new();
    for (index, wet) in targets {
        let from = cell(p, &objects[index]);
        let seat = owners.get(&index).copied();
        let mut candidates = Vec::new();
        let base = if let Some(seat) = seat {
            let ordinary = format!("Player_{seat}_Start");
            let imported = format!("{ordinary} (imported)");
            let starts: Vec<_> = objects
                .iter()
                .filter(|object| {
                    prop(&object.props, "waypointName")
                        .and_then(Value::text)
                        .is_some_and(|name| name == ordinary || name == imported)
                })
                .collect();
            if starts.len() != 1 {
                return Err(err(format!(
                    "production placement requires one start for P{seat}"
                )));
            }
            let base = to_cell(p, starts[0]);
            if let std::collections::btree_map::Entry::Vacant(entry) = access.entry(seat) {
                entry.insert(AccessDistances::new(p, n, z, impassable, &fixed, seat)?);
            }
            let radius = p
                .usize(
                    "PRODUCTION_SPAWN_RADIUS",
                    crate::gameplay::gift_clearance::search_radius(p, seat)?,
                )
                .min(n.max(z.len() / n));
            let center = base.map(|value| value.round_ties_even() as usize);
            for y in center[1].saturating_sub(radius)..=(center[1] + radius).min(z.len() / n - 1) {
                for x in center[0].saturating_sub(radius)..=(center[0] + radius).min(n - 1) {
                    let distance = (x as f64 - base[0]).hypot(y as f64 - base[1]);
                    if distance > radius as f64
                        || !ground.suitable([x, y], wet)
                        || !clear([x, y], ground.policy.padding(wet), &fixed_keepouts)
                        || reservations.as_ref().is_some_and(|reserves| {
                            !clear_functional_reservations(
                                reserves,
                                official.as_ref().unwrap(),
                                &objects[index],
                                [x, y],
                                z[y * n + x],
                            )
                            .unwrap_or(false)
                        })
                    {
                        continue;
                    }
                    let steps = if let Some(checker) = &official {
                        checker
                            .object_production_access_targets(&objects[index], [x, y])
                            .ok()
                            .and_then(|targets| {
                                if targets.is_empty() {
                                    access[&seat].score([x, y], wet, ground.policy.padding(wet))
                                } else {
                                    targets
                                        .into_iter()
                                        .map(|(wet, point)| access[&seat].score_point(point, wet))
                                        .collect::<Option<Vec<_>>>()
                                        .and_then(|values| values.into_iter().max())
                                }
                            })
                    } else {
                        access[&seat].score([x, y], wet, ground.policy.padding(wet))
                    };
                    if let Some(steps) = steps {
                        candidates.push((
                            steps,
                            distance,
                            (x as f64 - from[0] as f64).hypot(y as f64 - from[1] as f64),
                            x,
                            y,
                        ));
                    }
                }
            }
            Some(base)
        } else {
            // Do not infer ownership from proximity or silently relocate a
            // neutral/native building whose owner has no verified seat.
            if ground.suitable(from, wet) {
                candidates.push((0, 0., 0., from[0], from[1]));
            }
            None
        };
        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.total_cmp(&b.2))
                .then(a.4.cmp(&b.4))
                .then(a.3.cmp(&b.3))
        });
        plans.push(Plan {
            index,
            wet,
            seat,
            base,
            from,
            candidates,
        });
    }
    // Fewer terrain/access candidates are harder to place. Resolve those first;
    // native object identity is only the final deterministic tie breaker.
    plans.sort_by_key(|plan| {
        (
            plan.candidates.len(),
            std::cmp::Reverse(ground.policy.padding(plan.wet)),
            std::cmp::Reverse(plan.wet),
            plan.index,
        )
    });
    let mut placed = Vec::new();
    let mut placements = Vec::new();
    for plan in plans {
        let others: Vec<_> = objects
            .iter()
            .enumerate()
            .filter(|(index, object)| {
                *index != plan.index
                    && (!moving.contains(index) || placed.contains(index))
                    && !is_decoration(object)
                    && !prop(&object.props, "waypointName")
                        .and_then(Value::text)
                        .is_some_and(|name| name.starts_with("gift-alt-"))
                    && !prop(&object.props, "waypointName")
                        .and_then(Value::text)
                        .is_some_and(|name| {
                            lower_waypoints.contains(name) && !placed.contains(index)
                        })
            })
            .map(|(_, object)| object.clone())
            .collect();
        let protected = keepouts(p, &others);
        let official_obstacles = official
            .as_ref()
            .map(|checker| checker.planned_obstacles(&others));
        let mut rejected = None;
        let target = plan.candidates.iter().find(|candidate| {
            let point = [candidate.3, candidate.4];
            if !clear(point, ground.policy.padding(plan.wet), &protected) { return false; }
            if reservations.as_ref().is_some_and(|reserves| {
                !clear_functional_reservations(reserves, official.as_ref().unwrap(),
                    &objects[plan.index], point, z[point[1] * n + point[0]])
                    .unwrap_or(false)
            }) { return false; }
            if official.as_ref().is_none_or(|checker| {
                checker.acceptable_object(&objects[plan.index], point, official_obstacles.as_ref().unwrap())
            }) { return true; }
            if rejected.is_none() {
                rejected = official.as_ref().map(|checker| checker.object_rejections(
                    &objects[plan.index], point, official_obstacles.as_ref().unwrap())
                    .into_iter().take(3).collect::<Vec<_>>());
            }
            false
        }).ok_or_else(|| err(format!(
            "no safe {} production site for object {} {:?}; owning seat {:?}, original {:?}, candidates {}, first official rejection {:?}",
            if plan.wet { "water" } else { "land" }, plan.index + 1,
            prop(&objects[plan.index].props, "waypointName").and_then(Value::text),
            plan.seat, plan.from, plan.candidates.len(), rejected)))?;
        let point = [target.3, target.4];
        let position = frame.world(point.map(|value| value as f64));
        objects[plan.index].position[0] = position[0] as f32;
        objects[plan.index].position[1] = position[1] as f32;
        placed.push(plan.index);
        placements.push(json!({"object_index":plan.index,
            "waypoint":prop(&objects[plan.index].props,"waypointName").and_then(Value::text),
            "type":objects[plan.index].type_name,"surface":if plan.wet {"water"} else {"land"},
            "padding":ground.policy.padding(plan.wet),"seat":plan.seat,"from":plan.from,
            "cell":point,"moved":plan.from!=point,"candidate_count":plan.candidates.len(),
            "distance_from_base":plan.base.map(|base| (point[0] as f64-base[0]).hypot(point[1] as f64-base[1])),
            "static_access_steps":plan.seat.map(|_|target.0)}));
    }
    Ok(json!({"placements":placements,"policy":ground.policy,
        "policy_source":format!("{POLICY_SOURCE}#/production_placement"),
        "candidate_order":"verified owner base access, base distance, displacement, y/x; fewer candidates first",
        "access_model":"static same-domain four-neighbor paths to every actual rotated official rally target, worst branch first; mixed domain ranks reachable shoreline and a separate second leg; verified airfields without a ground rally and explicit NoExitPath use foundation access; official placement checks full rotated exit sweeps",
        "ingame_test":"not run"}))
}

pub fn validate(
    p: &Project,
    map: &MapFile,
    hm: &HeightMap,
    bt: &BlendTiles,
    objects: &[MapObject],
) -> Result<()> {
    validate_with_layout(p, map, hm, bt, objects, &terrain::Layout::load(p)?)
}

pub(crate) fn validate_final_with_layout(
    p: &Project,
    map: &MapFile,
    layout: &terrain::Layout,
) -> Result<()> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    validate_with_layout(p, map, &hm, &bt, &objects, layout)
}

pub(crate) fn validate_with_layout(
    p: &Project,
    map: &MapFile,
    hm: &HeightMap,
    bt: &BlendTiles,
    objects: &[MapObject],
    layout: &terrain::Layout,
) -> Result<()> {
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let impassable = terrain::flip(&bt.impassability, hm.width);
    let blocked = terrain::flip(&bt.buildability, hm.width);
    let ground = Ground {
        p,
        n: hm.width,
        z: &z,
        impassable: &impassable,
        blocked: &blocked,
        policy: PlacementPolicy::load(p)?,
    };
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let targets = bindings(&nodes, objects)?;
    let functional_reservations = if targets.is_empty() {
        None
    } else {
        let reservations =
            super::gift_clearance::FixedReservations::new(p, layout, objects, &z, hm.width)?;
        let official = super::official_placement::Checker::for_candidates(
            p,
            hm.width,
            &z,
            &impassable,
            &blocked,
            &nodes,
            objects,
        )?
        .ok_or_else(|| err("production validation requires resolved official geometry"))?;
        Some((reservations, official))
    };
    for (i, wet) in targets {
        if !ground.suitable(cell(p, &objects[i]), wet) {
            return Err(err(format!(
                "production footprint requires {}: {}",
                if wet { "water" } else { "flat land" },
                prop(&objects[i].props, "waypointName")
                    .and_then(Value::text)
                    .unwrap_or(&objects[i].type_name)
            )));
        }
        if let Some((reservations, official)) = &functional_reservations {
            let point = cell(p, &objects[i]);
            let polygons = official.actual_reservation_polygons(&objects[i])?;
            let findings = reservations.findings_polygons_at(
                point,
                z[point[1] * hm.width + point[0]],
                &polygons,
            );
            if !findings.is_empty() {
                return Err(err(format!(
                    "production object {} {} {:?} enters complete ore/refinery or ramp reservations: {}",
                    i + 1,
                    objects[i].type_name,
                    prop(&objects[i].props, "waypointName").and_then(Value::text),
                    serde_json::to_string(&findings)?
                )));
            }
        }
    }
    Ok(())
}

pub fn apply(p: &Project, map: &mut MapFile) -> Result<Json> {
    let mut candidate = map.clone();
    let report = apply_inner(p, &mut candidate)?;
    *map = candidate;
    Ok(report)
}

fn apply_inner(p: &Project, map: &mut MapFile) -> Result<Json> {
    let strict = crate::gameplay::official_placement::strict_project(p);
    let p = &strict;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let original_objects = objects.clone();
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let report = repair(
        p,
        hm.width,
        &z,
        &terrain::flip(&bt.impassability, hm.width),
        &terrain::flip(&bt.buildability, hm.width),
        &nodes,
        &mut objects,
    )?;
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    // The production prepass may move primary waypoints on an already guarded
    // map. Preserve their guard centres before the shared gift planner reads it.
    crate::gameplay::gift_clearance::sync_moved_areas(map, &original_objects, &objects)?;
    let collision = crate::gameplay::gift_clearance::apply(p, map)?;
    let facing = crate::gameplay::production_facing::apply(p, map)?;
    let native_decoded_crc32 = crc32fast::hash(&map.encode()?);
    let recorded_access = collision.get("final_production_access").or_else(|| {
        collision
            .get("planning")
            .and_then(|report| report.get("final_production_access"))
    });
    let final_access = if let Some(report) = recorded_access.filter(|report| {
        report["passed"] == true && report["native_decoded_crc32"] == json!(native_decoded_crc32)
    }) {
        report.clone()
    } else {
        super::placement::validate_final_access(p, map)?
    };
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    validate(p, map, &hm, &bt, &objects)?;
    let mut report = report;
    refresh_report(p, map, &mut report)?;
    report["collision_avoidance"] = collision;
    report["facing"] = facing;
    report["final_production_access"] = final_access;
    let official = crate::gameplay::official_placement::audit(p, map)?;
    crate::gameplay::official_placement::require_passed(&official)?;
    report["official_placement"] = official;
    Ok(report)
}

pub fn refresh_report(p: &Project, map: &MapFile, report: &mut Json) -> Result<()> {
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    for entry in report["placements"]
        .as_array_mut()
        .ok_or_else(|| err("invalid spawn report"))?
    {
        let index = entry["object_index"]
            .as_u64()
            .ok_or_else(|| err("missing spawn index"))? as usize;
        let c = cell(p, &objects[index]);
        entry["cell"] = json!(c);
        entry["moved"] = json!(entry["from"] != json!(c));
    }
    Ok(())
}

/// Find one physical object's XML position, tolerating WorldBuilder decimal precision.
fn position_span(xml: &str, position: [f32; 3], kind: &str) -> Result<(usize, usize)> {
    let mut matches = Vec::new();
    for (start, _) in xml.match_indices("<Position ") {
        let Some(end) = xml[start..].find("/>").map(|i| start + i + 2) else {
            continue;
        };
        let tag = &xml[start..end];
        // A start waypoint and its preplaced construction yard deliberately
        // share coordinates. Match the physical object's type as well, so a
        // moved waypoint cannot make the yard update ambiguous.
        let Some(object_start) = xml[..start].rfind('<') else {
            continue;
        };
        if !xml[object_start..start].contains(&format!("ThingTemplate=\"{kind}\"")) {
            continue;
        }
        let coordinates = ["x", "y", "z"].map(|axis| {
            let marker = format!("{axis}=\"");
            let part = tag.split_once(&marker)?.1;
            part.split('"').next()?.parse::<f64>().ok()
        });
        // Original WB XML stores decimal coordinates at slightly different
        // precision than its native float32 map. Refuse ambiguous matches.
        if coordinates
            .iter()
            .zip(position)
            .all(|(a, b)| a.is_some_and(|a| (a - b as f64).abs() <= 0.002))
        {
            matches.push((start, end));
        }
    }
    if matches.len() != 1 {
        return Err(err(format!(
            "source XML has missing or ambiguous object position: {position:?}"
        )));
    }
    Ok(matches[0])
}
fn sync_object_heading(xml: &mut String, object: &MapObject) -> Result<()> {
    let (position, _) = position_span(xml, object.position, &object.type_name)?;
    let start = xml[..position]
        .rfind('<')
        .ok_or_else(|| err("missing object XML opening tag"))?;
    let tag = &xml[start..position];
    if !tag.contains(&format!("ThingTemplate=\"{}\"", object.type_name)) {
        return Err(err("object XML heading does not match native type"));
    }
    let value = tag
        .find("Angle=\"")
        .ok_or_else(|| err("missing object XML Angle"))?
        + "Angle=\"".len();
    let end = value
        + tag[value..]
            .find('"')
            .ok_or_else(|| err("unterminated XML Angle"))?;
    let previous: f32 = tag[value..end].parse()?;
    if !previous.is_finite() || (previous - object.angle).abs() > 0.000001 {
        xml.replace_range(start + value..start + end, &object.angle.to_string());
    }
    Ok(())
}
pub fn update_xml(xml: &str, before: &MapFile, after: &MapFile) -> Result<String> {
    let old = map::objects(before.get("ObjectsList")?, &before.table)?;
    let new = map::objects(after.get("ObjectsList")?, &after.table)?;
    if old.len() > new.len() {
        return Err(err("production repair removed objects"));
    }
    let mut xml = xml.to_owned();
    for (a, b) in old.iter().zip(&new) {
        // Frozen packages can already contain stale ore headings. Reconcile all
        // ore XML against the native result, including angle-only edits.
        let sync_heading = a.angle != b.angle || b.type_name == "OreNode";
        if a.position == b.position && !sync_heading {
            continue;
        }
        if let Some(name) = prop(&a.props, "waypointName").and_then(Value::text) {
            let marker = format!("<Waypoint id=\"{name}\"");
            let start = xml
                .find(&marker)
                .ok_or_else(|| err(format!("source XML missing waypoint {name}")))?;
            let end = start
                + xml[start..]
                    .find("</Waypoint>")
                    .ok_or_else(|| err("unterminated waypoint XML"))?;
            let position_start = start
                + xml[start..end]
                    .find("<Position ")
                    .ok_or_else(|| err("missing waypoint XML position"))?;
            let position_end = position_start
                + xml[position_start..end]
                    .find("/>")
                    .ok_or_else(|| err("unterminated waypoint XML position"))?
                + 2;
            xml.replace_range(
                position_start..position_end,
                &format!(
                    "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
                    b.position[0], b.position[1], b.position[2]
                ),
            );
            continue;
        }
        let pos = |o: &MapObject| {
            format!(
                "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
                o.position[0], o.position[1], o.position[2]
            )
        };
        if a.position != b.position {
            let (start, end) = position_span(&xml, a.position, &a.type_name)?;
            xml.replace_range(start..end, &pos(b));
        }
        if sync_heading {
            sync_object_heading(&mut xml, b)?;
        }
    }
    let mut extra = String::new();
    for o in &new[old.len()..] {
        let name = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .ok_or_else(|| err("unexpected new non-waypoint object"))?;
        extra.push_str(&format!(
            "<Waypoint id=\"{name}\"><Position x=\"{}\" y=\"{}\" z=\"{}\"/></Waypoint>",
            o.position[0], o.position[1], o.position[2]
        ));
    }
    if !extra.is_empty() {
        xml = crate::resources::xml_structure::insert_waypoints(&xml, &extra)?;
    }
    crate::resources::refresh_script_xml(&xml, after)
}

pub fn write_artifacts(p: &Project, source_name: &str, map: &MapFile, report: &Json) -> Result<()> {
    use crate::{
        pipeline::project,
        render,
        terrain::surface::Surface,
        terrain::{Layout, ObjectSpec, Terrain},
    };
    use std::fs;
    let source_dir = p.release_assets(source_name)?;
    let mut layout = Layout::load_release(p, source_name)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let rules = scripts::load_rules(p)?;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !prop(&o.props, "waypointName")
                .and_then(Value::text)
                .is_some_and(|name| rules.spawn_waypoints.iter().any(|s| s.name == name))
        })
        .map(|o| ObjectSpec {
            cell: to_cell(p, o),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    layout.shipyard_cells = objects
        .iter()
        .filter(|o| water_only(&o.type_name) == Some(true))
        .map(|o| to_cell(p, o))
        .collect();
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, hm.height)?;
    let raw = terrain::flip(&hm.elevations, n);
    let z: Vec<_> = raw
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let t = Terrain {
        n,
        land: z.iter().map(|v| *v > p.number("WATER_Z", 200.)).collect(),
        slope: terrain::gradient(&z, n),
        raw,
        z,
        layout,
    };
    t.save(p)?;
    let s = Surface {
        chosen: terrain::flip(&bt.tiles, n).iter().map(|v| v / 64).collect(),
        impassable: terrain::flip(&bt.impassability, n),
        build_block: terrain::flip(&bt.buildability, n),
        layer_edge: vec![false; n * hm.height],
        shore: vec![false; n * hm.height],
        textures: vec![],
    };
    render::previews(&t, p, &s)?;
    let index_path = p.assets.join("layout-index.json");
    let mut index: Json = serde_json::from_slice(&fs::read(&index_path)?)?;
    index["layout"] = json!("source/layout.json");
    index["source_map"] = json!(format!("{}.map", p.name));
    project::write_json(&index_path, &index)?;
    project::write_json(&p.assets.join("production-spawns.json"), report)?;
    if let Some(collision) = report.get("collision_avoidance") {
        if collision["already_guarded"] == true && source_dir.join("gift-clearance.json").is_file()
        {
            fs::copy(
                source_dir.join("gift-clearance.json"),
                p.assets.join("gift-clearance.json"),
            )?;
        } else {
            project::write_json(&p.assets.join("gift-clearance.json"), collision)?;
        }
    }
    if let Some(facing) = report.get("facing") {
        project::write_json(&p.assets.join("production-facing.json"), facing)?;
    }
    if source_dir.join("periodic-attack.json").is_file() {
        fs::copy(
            source_dir.join("periodic-attack.json"),
            p.assets.join("periodic-attack.json"),
        )?;
    }
    for name in [
        "ramp-report.json",
        "ramps--comparison.png",
        "texture-report.json",
    ] {
        if source_dir.join(name).is_file() {
            fs::copy(source_dir.join(name), p.assets.join(name))?;
        }
    }
    fs::write(
        p.assets.join("README.md"),
        format!(
            "# {}\n\n基于 {} 更新公共赠送建筑规则：兵营、重工、机场、船厂、电厂和超武从各自实际主备落点朝向对方队伍出生区域中心，吸附到最近的 45、135、-45、-135 度。仅改变生成动作的朝向，不改变触发条件或归属。重工占地只允许平坦陆地，船厂占地只允许水域；运行时占用检查及主备避让继续保留。高度、纹理、通行及建造位图和资源身份保持原样。\n\n[朝向报告](production-facing.json) · [水陆与避让报告](production-spawns.json) · [全要素编号](preview--layout.png) · [发布核对](release-report.json)\n\n游戏内效果尚待实测。\n",
            p.name, source_name
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn physical_xml_position_disambiguates_a_colocated_start_waypoint() {
        let xml = "<Object ThingTemplate=\"SovietConstructionYard\" Angle=\"0\"><Position x=\"330\" y=\"3690\" z=\"0\"/></Object><Waypoint id=\"Player_3_Start\"><Position x=\"330\" y=\"3690\" z=\"0\"/></Waypoint>";
        let (start, end) =
            super::position_span(xml, [330., 3690., 0.], "SovietConstructionYard").unwrap();
        assert_eq!(&xml[start..end], "<Position x=\"330\" y=\"3690\" z=\"0\"/>");
        assert!(start < xml.find("</Object>").unwrap());
        assert!(super::position_span(xml, [330., 3690., 0.], "JapanConstructionYard").is_err());
        assert!(
            super::position_span(
                &format!("{xml}{xml}"),
                [330., 3690., 0.],
                "SovietConstructionYard"
            )
            .is_err()
        );
    }

    use super::*;
    use crate::{formats::binary::Writer, terrain::Layout};
    #[test]
    fn wb_decimal_coordinates_match_native_float_precision_but_duplicates_fail() {
        let tag = "<Position x=\"4642.596482\" y=\"3106.560427\" z=\"0\" />";
        let opening = "<Object ThingTemplate=\"OreNode\">";
        let xml = format!("{opening}{tag}</Object>");
        let position = [4642.596_f32, 3106.5603_f32, 0.];
        assert_eq!(
            position_span(&xml, position, "OreNode").unwrap(),
            (opening.len(), opening.len() + tag.len())
        );
        assert!(position_span(&format!("{xml}{xml}"), position, "OreNode").is_err());
        assert!(position_span(&xml, [4642.6, 3106.56, 0.], "OreNode").is_err());
    }

    #[test]
    fn angle_only_edits_and_stale_ore_xml_follow_native_headings_idempotently() {
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let before = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
        let mut objects = map::objects(before.get("ObjectsList").unwrap(), &before.table).unwrap();
        let ore = objects
            .iter_mut()
            .find(|o| o.type_name == "OreNode")
            .unwrap();
        let position = ore.position;
        ore.angle = std::f32::consts::FRAC_PI_4;
        let mut after = before.clone();
        after.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &after).unwrap();
        // Keep the actual package's script XML and physical objects, but
        // deliberately stale the first ore's angle.
        let xml = std::fs::read_to_string(p.assets.join("map.xml")).unwrap();
        let mut stale = xml.clone();
        let object = objects
            .iter()
            .find(|o| o.position == position && o.type_name == "OreNode")
            .unwrap();
        let mut wrong = object.clone();
        wrong.angle = -std::f32::consts::FRAC_PI_4;
        sync_object_heading(&mut stale, &wrong).unwrap();
        let edited = update_xml(&stale, &before, &after).unwrap();
        let repaired = update_xml(&stale, &after, &after).unwrap();
        assert_eq!(edited, repaired);
        assert_eq!(update_xml(&repaired, &after, &after).unwrap(), repaired);
        // Inspect the exact opening tag at this ore's unchanged native position.
        let (at, _) = position_span(&repaired, position, "OreNode").unwrap();
        let start = repaired[..at].rfind('<').unwrap();
        assert!(repaired[start..at].contains("Angle=\"0.7853982\""));
        let (old_start, old_end) = position_span(&xml, position, "OreNode").unwrap();
        let (_, end) = position_span(&repaired, position, "OreNode").unwrap();
        assert_eq!(&repaired[at..end], &xml[old_start..old_end]);
    }

    fn project() -> Project {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        p.cfg["GRID"] = json!(220);
        p.cfg["GRID_HEIGHT"] = json!(220);
        p
    }
    fn action(kind: &str, waypoint: &str) -> Node {
        let mut w = Writer::new();
        w.u32(467);
        w.u32(3);
        w.u32(4);
        for (ty, s) in [
            (15, kind),
            (3, "Player_1/teamPlayer_1"),
            (7, waypoint),
            (17, ""),
        ] {
            w.u32(ty);
            w.u32(0);
            w.f32(if ty == 17 {
                std::f32::consts::FRAC_PI_4
            } else {
                0.
            });
            w.s16(s).unwrap();
        }
        w.u32(1);
        Node {
            asset: "ScriptAction".into(),
            version: 3,
            prefix: String::new(),
            body: scripts::hex(&w.data),
            command: Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION".into()),
            children: None,
        }
    }

    #[test]
    fn every_ruleset_reads_the_public_policy_and_rejects_wrong_surfaces() {
        let directory =
            std::env::temp_dir().join(format!("ra3-public-placement-{}", std::process::id()));
        let rules = directory.join("templates/rules");
        std::fs::create_dir_all(&rules).unwrap();
        let path = rules.join("common-T0.5.1.json");
        let mut common = json!({"production_placement": {
            "full_footprint":true,
            "war_factory":{"surface":"land","padding":7},
            "naval_yard":{"surface":"water","padding":9}
        }});
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        for ruleset in ["common-T0.5.1", "another-script-ruleset"] {
            let mut p = project();
            p.cfg["RULESET"] = json!(ruleset);
            p.root = directory.clone();
            let policy = PlacementPolicy::load(&p).unwrap();
            assert_eq!(policy.padding(false), 7);
            assert_eq!(policy.padding(true), 9);
        }
        let mut p = project();
        p.root = directory.clone();
        common["production_placement"]["war_factory"]["surface"] = json!("water");
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        assert!(PlacementPolicy::load(&p).is_err());
        common["production_placement"]["war_factory"]["surface"] = json!("land");
        common["production_placement"]["full_footprint"] = json!(false);
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        assert!(PlacementPolicy::load(&p).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn entire_footprint_must_match_surface_and_be_clear() {
        let p = project();
        let n = 220;
        let mut z = vec![210.; n * n];
        for y in 0..n {
            for x in 110..n {
                z[y * n + x] = 130.;
            }
        }
        let mut blocked = vec![false; n * n];
        let impassable = vec![false; n * n];
        let ground = Ground {
            p: &p,
            n,
            z: &z,
            impassable: &impassable,
            blocked: &blocked,
            policy: PlacementPolicy::load(&p).unwrap(),
        };
        assert!(ground.suitable([80, 100], false));
        assert!(!ground.suitable([80, 100], true));
        assert!(ground.suitable([140, 100], true));
        assert!(!ground.suitable([140, 100], false));
        assert!(!ground.suitable([105, 100], false));
        assert!(!ground.suitable([115, 100], true));
        assert!(!ground.suitable([10, 100], true));
        blocked[100 * n + 80] = true;
        let ground = Ground {
            p: &p,
            n,
            z: &z,
            impassable: &impassable,
            blocked: &blocked,
            policy: PlacementPolicy::load(&p).unwrap(),
        };
        assert!(!ground.suitable([80, 100], false));
    }

    #[test]
    fn native_factory_geometry_outside_its_public_padding_preserves_mine_clearance() {
        let p = project();
        let original =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let mut layout = Layout::load(&original).unwrap();
        layout.ramps.clear();
        let mut ore = layout
            .objects
            .iter()
            .find(|object| object.type_name == "OreNode")
            .unwrap()
            .clone();
        ore.cell = [141., 110.];
        ore.angle = std::f32::consts::FRAC_PI_4;
        let ore = ore.object(&p);
        let mut factory = ore.clone();
        factory.type_name = "AlliedWarFactory".into();
        factory.props.clear();
        let frame = crate::coordinates::Frame::from_project(&p);
        let world = frame.world([110., 110.]);
        factory.position[0] = world[0] as f32;
        factory.position[1] = world[1] as f32;
        let objects = vec![factory.clone(), ore];
        let n = p.usize("GRID", 220);
        let z = vec![210.; n * n];
        let flags = vec![false; n * n];
        let reservations =
            super::super::gift_clearance::FixedReservations::new(&p, &layout, &objects, &z, n)
                .unwrap();
        let official = super::super::official_placement::Checker::for_candidates(
            &p,
            n,
            &z,
            &flags,
            &flags,
            &[],
            &objects,
        )
        .unwrap()
        .unwrap();
        // The 6-cell policy square clears the 20-cell ore disk. The real
        // diagonal EA foundation and production access extend farther.
        assert!(reservations.clear_at([110, 110], 6, 210.));
        assert!(
            !clear_functional_reservations(&reservations, &official, &factory, [110, 110], 210.,)
                .unwrap()
        );
        assert!(
            clear_functional_reservations(&reservations, &official, &factory, [104, 110], 210.,)
                .unwrap()
        );
    }

    #[test]
    fn custom_scripted_production_without_a_native_factory_checks_full_mine_reservations() {
        let p = project();
        let original =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let mut layout = Layout::load(&original).unwrap();
        layout.ramps.clear();
        let mut ore = layout
            .objects
            .iter()
            .find(|object| object.type_name == "OreNode")
            .unwrap()
            .clone();
        ore.cell = [141., 110.];
        ore.angle = std::f32::consts::FRAC_PI_4;
        let mut marker = layout
            .objects
            .iter()
            .find(|object| object.type_name.contains("Waypoint"))
            .unwrap()
            .clone();
        let mut objects = vec![ore.object(&p)];
        for (name, point) in std::iter::once(("custom-production".to_owned(), [110., 110.])).chain(
            (1..=8).map(|seat| {
                (
                    format!("Player_{seat}_Start"),
                    [
                        if seat <= 4 { 40. } else { 180. },
                        35. + ((seat - 1) % 4) as f64 * 50.,
                    ],
                )
            }),
        ) {
            marker.cell = point;
            map::set(&mut marker.props, "waypointName", Value::Ascii(name));
            objects.push(marker.object(&p));
        }
        assert!(
            objects
                .iter()
                .all(|object| !super::super::placement::is_production(&object.type_name))
        );
        let nodes = vec![action("AlliedWarFactory", "custom-production")];
        let mut map =
            MapFile::load(original.assets.join(format!("{}.map", original.name))).unwrap();
        map.get_mut("PlayerScriptsList").unwrap().payload =
            scripts::encode_nodes(&nodes, &mut map).unwrap();
        let n = p.usize("GRID", 220);
        let hm = HeightMap {
            width: n,
            height: n,
            border: 20,
            borders: Vec::new(),
            elevations: vec![5376; n * n],
        };
        let bt = BlendTiles {
            tiles: vec![0; n * n],
            impassability: vec![false; n * n],
            buildability: vec![false; n * n],
            textures: Vec::new(),
            tail: Vec::new(),
            blend_descriptions: Vec::new(),
        };
        let error = validate_with_layout(&p, &map, &hm, &bt, &objects, &layout)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("custom-production") && error.contains("ore_disk"),
            "{error}"
        );
        let frame = crate::coordinates::Frame::from_project(&p);
        let world = frame.world([104., 110.]);
        objects[1].position[0] = world[0] as f32;
        objects[1].position[1] = world[1] as f32;
        validate_with_layout(&p, &map, &hm, &bt, &objects, &layout).unwrap();
    }

    #[test]
    fn repairs_scripted_factories_and_shipyards_without_changing_actions() {
        let p = project();
        let n = 220;
        let original_p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let mut layout = Layout::load(&original_p).unwrap();
        // This synthetic 220-cell scene declares no ramps. Its fixtures must
        // not accidentally inherit the real 800-cell map's slope geometry.
        layout.ramps.clear();
        let mut prototype = layout.objects.remove(
            layout
                .objects
                .iter()
                .position(|o| o.type_name.contains("Waypoint"))
                .unwrap(),
        );
        let mut objects = Vec::new();
        for (name, point) in [("factory", [140., 80.]), ("shipyard", [80., 140.])] {
            prototype.cell = point;
            map::set(
                &mut prototype.props,
                "waypointName",
                Value::Ascii(name.into()),
            );
            objects.push(prototype.object(&p));
        }
        // Strict official placement resolves every scripted gift's native
        // owner and facing target from all eight starts, even in a local test.
        for seat in 1..=8 {
            prototype.cell = [
                if seat <= 4 { 40. } else { 180. },
                35. + ((seat - 1) % 4) as f64 * 50.,
            ];
            map::set(
                &mut prototype.props,
                "waypointName",
                Value::Ascii(format!("Player_{seat}_Start")),
            );
            objects.push(prototype.object(&p));
        }
        let mut z = vec![210.; n * n];
        for y in 0..n {
            for x in 110..n {
                z[y * n + x] = 130.;
            }
        }
        let flags = vec![false; n * n];
        for faction in ["allied", "soviet", "japan"] {
            let nodes = vec![
                action(&format!("{faction}warfactory"), "factory"),
                action(&format!("{faction}navalyard"), "shipyard"),
            ];
            let original = serde_json::to_value(&nodes).unwrap();
            let mut repaired = objects.clone();
            let report =
                repair_with_layout(&p, n, &z, &flags, &flags, &nodes, &mut repaired, &layout)
                    .unwrap();
            assert!(
                report["placements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v["moved"] == true)
            );
            let ground = Ground {
                p: &p,
                n,
                z: &z,
                impassable: &flags,
                blocked: &flags,
                policy: PlacementPolicy::load(&p).unwrap(),
            };
            assert!(ground.suitable(cell(&p, &repaired[0]), false));
            assert!(ground.suitable(cell(&p, &repaired[1]), true));
            assert_eq!(serde_json::to_value(&nodes).unwrap(), original);
            let repeated =
                repair_with_layout(&p, n, &z, &flags, &flags, &nodes, &mut repaired, &layout)
                    .unwrap();
            assert!(
                repeated["placements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v["moved"] == false)
            );
        }
        // A legal but remote point is also reconsidered against the script's
        // owning seat; proximity to some other start does not change ownership.
        let mut remote = objects.clone();
        let world = crate::coordinates::Frame::from_project(&p).world([82., 140.]);
        remote[0].position = [world[0] as f32, world[1] as f32, 0.];
        let old_distance = (82_f64 - 40.).hypot(140_f64 - 35.);
        let report = repair_with_layout(
            &p,
            n,
            &z,
            &flags,
            &flags,
            &[action("alliedwarfactory", "factory")],
            &mut remote,
            &layout,
        )
        .unwrap();
        let placement = &report["placements"][0];
        assert_eq!(placement["seat"], 1);
        assert_eq!(placement["moved"], true);
        assert!(placement["distance_from_base"].as_f64().unwrap() < old_distance);
        assert!(placement["static_access_steps"].as_u64().is_some());
        assert!(bindings(&[action("sovietwarfactory", "missing")], &objects).is_err());
        assert!(
            bindings(
                &[
                    action("sovietwarfactory", "factory"),
                    action("sovietnavalyard", "factory")
                ],
                &objects
            )
            .is_err()
        );
        assert!(
            repair_with_layout(
                &p,
                n,
                &vec![130.; n * n],
                &flags,
                &flags,
                &[action("sovietwarfactory", "factory")],
                &mut objects,
                &layout,
            )
            .is_err()
        );
    }
}
