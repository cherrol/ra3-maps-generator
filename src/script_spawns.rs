//! Bind shared scripted buildings to safe locations on the selected map.
use crate::{
    Result, err,
    map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    project::{self, Project},
    scripts::{self, Node},
    surface::Surface,
    terrain::Terrain,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpawnWaypoint {
    pub name: String,
    /// One-based seat index, including fixed AI seats 7 and 8.
    pub seat: usize,
    pub padding: usize,
}
#[derive(Clone, Debug, Serialize)]
struct Placed {
    waypoint: String,
    seat: usize,
    cell: [usize; 2],
    padding: usize,
    distance_from_base: f64,
}
fn seat_radius(p: &Project, seat: usize) -> Result<usize> {
    let radius = match p
        .cfg
        .get("SCRIPT_SPAWN_RADIUS_BY_SEAT")
        .and_then(|v| v.get(seat.to_string()))
    {
        Some(v) => v
            .as_u64()
            .and_then(|r| usize::try_from(r).ok())
            .ok_or_else(|| err("invalid script spawn radius"))?,
        None => crate::gift_clearance::search_radius(p, seat)?,
    };
    let radius = radius.max(crate::gift_clearance::search_radius(p, seat)?);
    if !(14..=p.usize("GRID", 840)).contains(&radius) {
        return Err(err("script spawn radius outside map dimensions"));
    }
    Ok(radius)
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
pub(crate) fn to_cell(p: &Project, o: &MapObject) -> [f64; 2] {
    crate::coordinates::Frame::from_project(p).grid([o.position[0] as f64, o.position[1] as f64])
}

fn specs(p: &Project) -> Result<Vec<SpawnWaypoint>> {
    let specs = scripts::load_rules(p)?.spawn_waypoints;
    let mut names = BTreeSet::new();
    for s in &specs {
        if !(1..=8).contains(&s.seat)
            || !(2..=8).contains(&s.padding)
            || !names.insert(s.name.clone())
        {
            return Err(err("invalid or duplicate shared spawn waypoint"));
        }
    }
    Ok(specs)
}
pub(crate) fn keepouts(p: &Project, objects: &[MapObject]) -> Vec<([f64; 2], f64)> {
    objects
        .iter()
        .filter_map(|o| {
            let radius = if o.type_name.contains("Waypoint") {
                let name = prop(&o.props, "waypointName")
                    .and_then(Value::text)
                    .unwrap_or("");
                if name.starts_with("dual-") {
                    3.
                } else if name.starts_with("Player_") {
                    8.
                } else {
                    return None;
                }
            } else if o.type_name == "OreNode" {
                14.
            } else if o.type_name.contains("ConstructionYard") {
                8.
            } else if o.type_name.contains("Tree")
                || o.type_name.contains("Bush")
                || o.type_name.contains("GardenRock")
            {
                2.
            } else if o.type_name.contains("Sidewalk") || o.road_type != 0 {
                return None;
            } else {
                8.
            };
            Some((to_cell(p, o), radius))
        })
        .collect()
}
fn suitable(
    p: &Project,
    n: usize,
    z: &[f64],
    impassable: &[bool],
    blocked: &[bool],
    point: [usize; 2],
    padding: usize,
) -> bool {
    let [x, y] = point;
    let border = p.usize("BORDER", 20) + padding + 2;
    if x < border || y < border || x + border >= n || y + border >= n {
        return false;
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for yy in y - padding..=y + padding {
        for xx in x - padding..=x + padding {
            let i = yy * n + xx;
            // Use land for compatibility with every faction's scripted building.
            if z[i] <= p.number("WATER_Z", 200.) || impassable[i] || blocked[i] {
                return false;
            }
            lo = lo.min(z[i]);
            hi = hi.max(z[i]);
        }
    }
    hi - lo <= 1.0
}
pub(crate) fn clear(point: [usize; 2], padding: usize, protected: &[([f64; 2], f64)]) -> bool {
    let c = [point[0] as f64, point[1] as f64];
    protected
        .iter()
        .all(|&(q, r)| (c[0] - q[0]).abs().max((c[1] - q[1]).abs()) >= padding as f64 + r + 2.)
}
pub fn append(p: &Project, t: &Terrain, s: &Surface, objects: &mut Vec<MapObject>) -> Result<()> {
    if p.flag("TERRAIN_ONLY") {
        return Ok(());
    }
    let specs = specs(p)?;
    if specs.is_empty() {
        return Ok(());
    }
    let mut protected = keepouts(p, objects);
    let prototype = objects
        .iter()
        .find(|o| o.type_name.contains("Waypoint"))
        .ok_or_else(|| err("no waypoint prototype"))?
        .clone();
    let mut next_id = objects
        .iter()
        .filter_map(|o| match prop(&o.props, "waypointID") {
            Some(Value::Int(i)) => Some(*i),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;
    let mut placed = Vec::new();
    for spec in &specs {
        let base = t.layout.base_cells[spec.seat - 1];
        let radius = seat_radius(p, spec.seat)?;
        let existing = objects.iter().find(|o| {
            prop(&o.props, "waypointName").and_then(Value::text) == Some(spec.name.as_str())
        });
        let point = if let Some(o) = existing {
            let c = to_cell(p, o);
            [
                c[0].round_ties_even() as usize,
                c[1].round_ties_even() as usize,
            ]
        } else {
            let [bx, by] = [
                base[0].round_ties_even() as usize,
                base[1].round_ties_even() as usize,
            ];
            let mut candidates = Vec::new();
            for y in by.saturating_sub(radius)..=(by + radius).min(t.n - 1) {
                for x in bx.saturating_sub(radius)..=(bx + radius).min(t.n - 1) {
                    let d = distance(base, [x as f64, y as f64]);
                    if (14.0..=radius as f64).contains(&d) {
                        candidates.push((d, x, y));
                    }
                }
            }
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
            let (_, x, y) = candidates
                .into_iter()
                .find(|&(_, x, y)| {
                    suitable(
                        p,
                        t.n,
                        &t.z,
                        &s.impassable,
                        &s.build_block,
                        [x, y],
                        spec.padding,
                    ) && clear([x, y], spec.padding, &protected)
                })
                .ok_or_else(|| {
                    err(format!(
                        "{}: no flat land for {} within {} cells of P{}",
                        p.id, spec.name, radius, spec.seat
                    ))
                })?;
            let mut o = prototype.clone();
            o.position = [
                ((x as f64 - p.number("BORDER", 20.)) * p.number("CELL", 10.)) as f32,
                ((t.n as f64 - 1. - p.number("BORDER", 20.) - y as f64) * p.number("CELL", 10.))
                    as f32,
                0.,
            ];
            map::set(
                &mut o.props,
                "waypointName",
                Value::Ascii(spec.name.clone()),
            );
            map::set(&mut o.props, "uniqueID", Value::Ascii(spec.name.clone()));
            map::set(&mut o.props, "waypointID", Value::Int(next_id));
            next_id += 1;
            objects.push(o);
            [x, y]
        };
        if !suitable(
            p,
            t.n,
            &t.z,
            &s.impassable,
            &s.build_block,
            point,
            spec.padding,
        ) || !clear(point, spec.padding, &protected)
            || !(14.0..=radius as f64).contains(&distance(base, [point[0] as f64, point[1] as f64]))
        {
            return Err(err(format!("invalid shared spawn point {}", spec.name)));
        }
        protected.push(([point[0] as f64, point[1] as f64], spec.padding as f64));
        placed.push(Placed {
            waypoint: spec.name.clone(),
            seat: spec.seat,
            cell: point,
            padding: spec.padding,
            distance_from_base: distance(base, [point[0] as f64, point[1] as f64]),
        });
    }
    project::write_json(&p.assets.join("script_waypoints.json"), &placed)?;
    println!("共用脚本：按地图地形绑定 {} 个建筑航点", placed.len());
    Ok(())
}
pub fn validate(
    p: &Project,
    map: &MapFile,
    hm: &HeightMap,
    bt: &BlendTiles,
    objects: &[MapObject],
) -> Result<()> {
    crate::production_spawns::validate(p, map, hm, bt, objects)?;
    let specs = specs(p)?;
    if specs.is_empty() {
        return Ok(());
    }
    let n = hm.width;
    let z = crate::terrain::flip(&hm.elevations, n)
        .into_iter()
        .map(|x| x as f64 * p.number("Z_SCALE", 0.0390625))
        .collect::<Vec<_>>();
    let impassable = crate::terrain::flip(&bt.impassability, n);
    let blocked = crate::terrain::flip(&bt.buildability, n);
    let mut protected = keepouts(p, objects);
    let layout = crate::terrain::Layout::load(p)?;
    for spec in &specs {
        let o = objects
            .iter()
            .find(|o| {
                prop(&o.props, "waypointName").and_then(Value::text) == Some(spec.name.as_str())
            })
            .ok_or_else(|| err(format!("missing shared waypoint {}", spec.name)))?;
        let c = to_cell(p, o);
        let point = [
            c[0].round_ties_even() as usize,
            c[1].round_ties_even() as usize,
        ];
        let radius = seat_radius(p, spec.seat)?;
        if !(14.0..=radius as f64).contains(&distance(layout.base_cells[spec.seat - 1], c))
            || !suitable(p, n, &z, &impassable, &blocked, point, spec.padding)
            || !clear(point, spec.padding, &protected)
        {
            return Err(err(format!(
                "shared building footprint is obstructed: {}",
                spec.name
            )));
        }
        protected.push((c, spec.padding as f64));
    }
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut names = BTreeSet::new();
    fn names_of(nodes: &[Node], names: &mut BTreeSet<String>) -> Result<()> {
        for n in nodes {
            if let Some(name) = scripts::name(n)? {
                names.insert(name);
            }
            if let Some(c) = &n.children {
                names_of(c, names)?;
            }
        }
        Ok(())
    }
    names_of(&nodes, &mut names)?;
    let waypoints = objects
        .iter()
        .filter_map(|o| prop(&o.props, "waypointName").and_then(Value::text))
        .collect::<BTreeSet<_>>();
    fn check(nodes: &[Node], names: &BTreeSet<String>, waypoints: &BTreeSet<&str>) -> Result<()> {
        for n in nodes {
            if let Some(c) = &n.children {
                check(c, names, waypoints)?;
            } else {
                if n.command.as_deref() == Some("X") {
                    return Err(err("unlinked command X in shared scripts"));
                }
                for (ty, s) in scripts::string_arguments(n)? {
                    if ty == 7 && !waypoints.contains(s.as_str()) {
                        return Err(err(format!("missing script waypoint {s}")));
                    }
                    if (ty == 2 || ty == 13) && !names.contains(&s) {
                        return Err(err(format!("missing script target {s}")));
                    }
                }
            }
        }
        Ok(())
    }
    check(&nodes, &names, &waypoints)
}

#[cfg(test)]
mod tests {
    use super::clear;

    #[test]
    fn building_footprints_leave_space_at_diagonal_corners() {
        let protected = [([100., 100.], 5.)];
        // Euclidean distance exceeds 12 here, but two 11×11 footprints overlap.
        assert!(!clear([109, 109], 5, &protected));
        assert!(clear([112, 100], 5, &protected));
    }
}
