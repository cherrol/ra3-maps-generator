//! Common six-stage ore allocation with map-authored routes and native ore IDs.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, MapFile},
    pipeline::project::Project,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, io::ErrorKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Policy {
    pub mines_per_seat: usize,
    pub home_radius: f64,
    pub home_maximum: usize,
    pub route_count: usize,
    pub main_route_max_distance: f64,
    pub fourth_progress: [f64; 2],
    pub hotspot_progress: [f64; 2],
    pub secondary_max_distance: f64,
    pub secondary_min_main_distance: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            mines_per_seat: 6,
            home_radius: 100.,
            home_maximum: 3,
            route_count: 4,
            main_route_max_distance: 65.,
            fourth_progress: [0.10, 0.35],
            hotspot_progress: [0.33, 0.67],
            secondary_max_distance: 75.,
            secondary_min_main_distance: 35.,
        }
    }
}
pub fn policy(p: &Project) -> Result<Policy> {
    let path = p.root.join("templates/rules/coordinate-pipeline.json");
    let common: Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(e) if e.kind() == ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.into()),
    };
    let policy: Policy = serde_json::from_value(
        common
            .get("resource_distribution")
            .cloned()
            .unwrap_or_else(|| json!({})),
    )?;
    if policy.mines_per_seat != 6
        || policy.route_count != 4
        || policy.home_maximum == 0
        || [
            policy.home_radius,
            policy.main_route_max_distance,
            policy.secondary_max_distance,
            policy.secondary_min_main_distance,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || [policy.fourth_progress, policy.hotspot_progress]
            .iter()
            .any(|r| r.iter().any(|v| !v.is_finite()) || r[0] < 0. || r[0] >= r[1] || r[1] > 1.)
    {
        return Err(err("invalid common resource distribution policy"));
    }
    Ok(policy)
}

/// Bind both the shared policy and the map's actual plan to cached proofs.
pub fn binding(p: &Project) -> Result<Value> {
    Ok(json!({"schema_version":1,"policy":policy(p)?,
        "near_ore_radius":crate::terrain::spawn_placement::policy(p)?.near_ore_radius,
        "plan_crc32":input(p)?.as_ref().map(|v|crc32fast::hash(v)),
        "terrain_only":p.flag("TERRAIN_ONLY"),
        "frozen":p.cfg["COORDINATE_PIPELINE"].is_object(),
        "historical_reference":p.cfg.get("SPAWN_PLACEMENT_REFERENCE")}))
}

#[derive(Deserialize)]
struct Plan {
    home_radius: Option<f64>,
    home_maximum: Option<usize>,
    mine_count: Option<usize>,
    routes: Vec<Route>,
    slots: Vec<Slot>,
}
#[derive(Deserialize)]
struct Route {
    id: usize,
    #[serde(alias = "north_seat")]
    side_a_seat: usize,
    #[serde(alias = "south_seat")]
    side_b_seat: usize,
    main_path: Vec<[f64; 2]>,
    secondary_path: Vec<[f64; 2]>,
}
#[derive(Deserialize)]
struct Slot {
    seat: usize,
    ordinal: usize,
    ore_id: usize,
    #[serde(default)]
    route_id: Option<usize>,
}
pub fn input(p: &Project) -> Result<Option<Vec<u8>>> {
    p.cfg
        .get("RESOURCE_DISTRIBUTION")
        .map(|v| {
            let path = v
                .as_str()
                .ok_or_else(|| err("RESOURCE_DISTRIBUTION must name a plan file"))?;
            Ok(fs::read(p.path(path)?)?)
        })
        .transpose()
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn projection(point: [f64; 2], path: &[[f64; 2]]) -> (f64, f64) {
    let (i, d) = path
        .iter()
        .enumerate()
        .map(|(i, p)| (i, distance(point, *p)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("validated route");
    (d, i as f64 / (path.len() - 1) as f64)
}
pub fn validate(p: &Project, m: &MapFile) -> Result<Option<Value>> {
    let policy = policy(p)?;
    if p.flag("TERRAIN_ONLY") {
        return Ok(None);
    }
    let Some(bytes) = input(p)? else {
        if p.cfg["COORDINATE_PIPELINE"].is_object()
            || p.cfg["SPAWN_PLACEMENT_REFERENCE"].is_string()
        {
            // Existing unplanned releases remain reproducible. This is not a
            // statement that their allocation satisfies the new common rule.
            return Ok(None);
        }
        return Err(err(
            "common ore allocation requires RESOURCE_DISTRIBUTION: six ores per seat, three home ores, main/hotspot/secondary routes",
        ));
    };
    let plan: Plan = serde_json::from_slice(&bytes)?;
    let home_radius = plan.home_radius.unwrap_or(policy.home_radius);
    let home_maximum = plan.home_maximum.unwrap_or(policy.home_maximum);
    let near = crate::terrain::spawn_placement::policy(p)?.near_ore_radius;
    let mine_count = 8 * policy.mines_per_seat;
    if !home_radius.is_finite()
        || home_radius <= near
        || (home_radius - policy.home_radius).abs() > 1e-6
        || home_maximum == 0
        || home_maximum > policy.home_maximum
        || plan.mine_count.is_some_and(|count| count != mine_count)
        || plan.routes.len() != policy.route_count
    {
        return Err(err("invalid resource expansion plan"));
    }
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let frame = Frame::from_project(p);
    let mut bases = vec![None; 8];
    for o in &objects {
        let name = map::prop(&o.props, "waypointName")
            .and_then(map::Value::text)
            .unwrap_or("");
        for (i, base) in bases.iter_mut().enumerate() {
            if name == format!("Player_{}_Start", i + 1)
                || name == format!("Player_{}_Start (imported)", i + 1)
            {
                if base.is_some() {
                    return Err(err("duplicate resource-plan start"));
                }
                *base = Some(frame.grid([o.position[0] as f64, o.position[1] as f64]));
            }
        }
    }
    let bases: Vec<_> = bases
        .into_iter()
        .map(|b| b.ok_or_else(|| err("resource-plan start missing")))
        .collect::<Result<_>>()?;
    let ores: Vec<_> = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.type_name == "OreNode")
        .map(|(i, o)| {
            (
                i + 1,
                frame.grid([o.position[0] as f64, o.position[1] as f64]),
            )
        })
        .collect();
    if ores.len() != mine_count || plan.slots.len() != ores.len() {
        return Err(err(
            "resource plan must cover every actual ore exactly once",
        ));
    }
    let mut route_ids = BTreeSet::new();
    let mut seats = BTreeSet::new();
    for r in &plan.routes {
        if !route_ids.insert(r.id)
            || !(1..=4).contains(&r.side_a_seat)
            || !(5..=8).contains(&r.side_b_seat)
            || !seats.insert(r.side_a_seat)
            || !seats.insert(r.side_b_seat)
            || [&r.main_path, &r.secondary_path].iter().any(|line| {
                line.len() < 2
                    || line
                        .iter()
                        .any(|c| !c.iter().all(|v| v.is_finite()) || !frame.inside(*c))
            })
        {
            return Err(err("invalid or duplicate resource route"));
        }
    }
    let mut seen = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    let mut rows = Vec::new();
    let home_counts: Vec<_> = bases
        .iter()
        .map(|b| {
            ores.iter()
                .filter(|(_, c)| distance(*c, *b) <= home_radius)
                .count()
        })
        .collect();
    for (i, count) in home_counts.iter().enumerate() {
        if *count > home_maximum {
            return Err(err(format!(
                "P{} has {count} ores in its home area; maximum {}",
                i + 1,
                home_maximum
            )));
        }
    }
    for slot in &plan.slots {
        if !(1..=8).contains(&slot.seat)
            || !(1..=policy.mines_per_seat).contains(&slot.ordinal)
            || !seen.insert(slot.ore_id)
            || !ordinals.insert((slot.seat, slot.ordinal))
        {
            return Err(err("invalid or duplicate resource expansion slot"));
        }
        let point = ores
            .iter()
            .find(|(id, _)| *id == slot.ore_id)
            .ok_or_else(|| err("expansion slot is not an actual ore"))?
            .1;
        let home = distance(point, bases[slot.seat - 1]);
        if slot.ordinal <= 2 && home > near {
            return Err(err("opening ores must remain inside the near-ore radius"));
        }
        if slot.ordinal == 3 && home <= near {
            return Err(err(
                "third ore must lie beyond the two opening ores' radius",
            ));
        }
        let mut row = json!({"seat":slot.seat,"ordinal":slot.ordinal,"ore_id":slot.ore_id,"cell":point,"home_distance":home});
        if slot.ordinal <= 3 {
            if home > home_radius {
                return Err(err(format!(
                    "P{} ore {} left the home area",
                    slot.seat, slot.ordinal
                )));
            }
        } else {
            if bases.iter().any(|b| distance(point, *b) <= home_radius) {
                return Err(err("fourth and later ores must leave all home areas"));
            }
            let route = plan
                .routes
                .iter()
                .find(|r| {
                    Some(r.id) == slot.route_id
                        && (r.side_a_seat == slot.seat || r.side_b_seat == slot.seat)
                })
                .ok_or_else(|| err("expansion slot has no route for its seat"))?;
            let (main, progress) = projection(point, &route.main_path);
            let (secondary, _) = projection(point, &route.secondary_path);
            match slot.ordinal {
                4 => {
                    let expected = if slot.seat == route.side_a_seat {
                        policy.fourth_progress[0]..=policy.fourth_progress[1]
                    } else {
                        (1. - policy.fourth_progress[1])..=(1. - policy.fourth_progress[0])
                    };
                    if main > policy.main_route_max_distance || !expected.contains(&progress) {
                        return Err(err(format!(
                            "P{} fourth ore does not lead into its main route",
                            slot.seat
                        )));
                    }
                }
                5 => {
                    if main > policy.main_route_max_distance
                        || !(policy.hotspot_progress[0]..=policy.hotspot_progress[1])
                            .contains(&progress)
                    {
                        return Err(err(format!(
                            "P{} fifth ore is outside the contested hotspot",
                            slot.seat
                        )));
                    }
                }
                6 => {
                    if secondary > policy.secondary_max_distance
                        || main < policy.secondary_min_main_distance
                    {
                        return Err(err(format!(
                            "P{} sixth ore is not on a distinct secondary route",
                            slot.seat
                        )));
                    }
                }
                _ => unreachable!(),
            }
            row["route_id"] = json!(route.id);
            row["main_route_distance"] = json!(main);
            row["main_progress"] = json!(progress);
            row["secondary_route_distance"] = json!(secondary);
        }
        rows.push(row);
    }
    for seat in 1..=8 {
        for ordinal in 1..=policy.mines_per_seat {
            if !ordinals.contains(&(seat, ordinal)) {
                return Err(err("every seat needs all six resource stages"));
            }
        }
    }
    Ok(Some(
        json!({"policy":policy,"home_radius":home_radius,"home_maximum":home_maximum,"home_counts":home_counts,"mine_count":ores.len(),"routes":plan.routes.len(),"slots":rows,"passed":true,"ai_behavior":"Resource incentives only; no runtime AI behavior inferred."}),
    ))
}

/// Unplanned frozen maps can be rebuilt, but moving mines or starts requires
/// completing the common route plan before publishing a changed allocation.
pub fn validate_transition(
    p: &Project,
    before: &MapFile,
    after: &MapFile,
) -> Result<Option<Value>> {
    if !p.flag("TERRAIN_ONLY") && input(p)?.is_none() {
        let geometry = |m: &MapFile| -> Result<Vec<(String, [f32; 3])>> {
            Ok(map::objects(m.get("ObjectsList")?, &m.table)?
                .into_iter()
                .filter_map(|o| {
                    let start = map::prop(&o.props, "waypointName")
                        .and_then(map::Value::text)
                        .filter(|n| n.starts_with("Player_") && n.contains("_Start"));
                    if o.type_name == "OreNode" || start.is_some() {
                        Some((start.unwrap_or("OreNode").to_owned(), o.position))
                    } else {
                        None
                    }
                })
                .collect())
        };
        if geometry(before)? != geometry(after)? {
            return Err(err(
                "changed ore/start allocation requires RESOURCE_DISTRIBUTION under the common six-stage ore rule",
            ));
        }
        // A byte-preserving legacy repack is not creation of a new allocation.
        policy(p)?;
        return Ok(None);
    }
    validate(p, after)
}
