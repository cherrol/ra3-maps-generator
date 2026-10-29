//! 为脚本建筑分配满足地形、占地和间距要求的航点。
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    gameplay::scripts::{self, Node},
    pipeline::project::{self, Project},
    terrain::Terrain,
    terrain::surface::Surface,
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
        None => crate::gameplay::gift_clearance::search_radius(p, seat)?,
    };
    let radius = radius.max(crate::gameplay::gift_clearance::search_radius(p, seat)?);
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
    if x < border || y < border || x + border >= n || y + border >= z.len() / n {
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
    let frame = crate::coordinates::Frame::from_project(p);
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
            for y in by.saturating_sub(radius)..=(by + radius).min(t.height() - 1) {
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
            let position = frame.world([x as f64, y as f64]);
            o.position = [position[0] as f32, position[1] as f32, 0.];
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

/// Rebind an existing placement report after all native waypoint repairs finish.
pub fn refresh_report(p: &Project, map: &MapFile) -> Result<()> {
    let path = p.assets.join("script_waypoints.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut report: serde_json::Value = serde_json::from_slice(&bytes)?;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = crate::coordinates::Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut waypoints = std::collections::BTreeMap::new();
    for object in &objects {
        if object.type_name.contains("Waypoint")
            && let Some(name) = prop(&object.props, "waypointName").and_then(Value::text)
        {
            let cell = frame.grid([object.position[0] as f64, object.position[1] as f64]);
            if cell.iter().any(|value| !value.is_finite()) || waypoints.insert(name, cell).is_some()
            {
                return Err(err(format!("invalid or duplicate report waypoint {name}")));
            }
        }
    }
    for entry in report
        .as_array_mut()
        .ok_or_else(|| err("invalid script waypoint report"))?
    {
        let name = entry["waypoint"]
            .as_str()
            .ok_or_else(|| err("script waypoint report is missing its waypoint"))?;
        let cell = *waypoints
            .get(name)
            .ok_or_else(|| err(format!("script waypoint report target is missing: {name}")))?;
        let seat = entry["seat"]
            .as_u64()
            .filter(|seat| (1..=8).contains(seat))
            .ok_or_else(|| err("invalid script waypoint report seat"))?;
        let ordinary = format!("Player_{seat}_Start");
        let imported = format!("{ordinary} (imported)");
        let base = match (
            waypoints.get(ordinary.as_str()),
            waypoints.get(imported.as_str()),
        ) {
            (Some(base), None) | (None, Some(base)) => *base,
            _ => {
                return Err(err(format!(
                    "script waypoint report requires one start for P{seat}"
                )));
            }
        };
        entry["cell"] = serde_json::json!(cell.map(|value| value.round_ties_even() as usize));
        entry["distance_from_base"] = serde_json::json!(distance(base, cell));
    }
    project::write_json(&path, &report)
}

pub fn validate(
    p: &Project,
    map: &MapFile,
    hm: &HeightMap,
    bt: &BlendTiles,
    objects: &[MapObject],
) -> Result<()> {
    crate::gameplay::production_spawns::validate(p, map, hm, bt, objects)?;
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
    use super::*;
    use serde_json::json;

    struct ReportFixture {
        p: Project,
        map: MapFile,
    }
    impl ReportFixture {
        fn new() -> Self {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let mut p = Project::load(root, "海南").unwrap();
            p.assets = std::env::temp_dir().join(format!(
                "ra3-script-report-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&p.assets).unwrap();
            let map = MapFile::load(root.join("templates/8p_template.map")).unwrap();
            Self { p, map }
        }
    }
    impl Drop for ReportFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.p.assets);
        }
    }

    #[test]
    fn final_waypoint_report_uses_native_positions_and_preserves_other_fields() {
        let mut fixture = ReportFixture::new();
        let hm = HeightMap::parse(fixture.map.get("HeightMapData").unwrap()).unwrap();
        let frame = crate::coordinates::Frame::from_heightmap(&fixture.p, &hm).unwrap();
        let prototype = map::objects(fixture.map.get("ObjectsList").unwrap(), &fixture.map.table)
            .unwrap()
            .into_iter()
            .find(|object| object.type_name.contains("Waypoint"))
            .unwrap();
        let objects =
            [("Player_1_Start", [100., 100.]), ("SWJ1", [130., 140.])].map(|(name, cell)| {
                let mut object = prototype.clone();
                let world = frame.world(cell);
                object.position = [world[0] as f32, world[1] as f32, 0.];
                map::set(&mut object.props, "waypointName", Value::Ascii(name.into()));
                object
            });
        fixture.map.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &fixture.map).unwrap();
        let path = fixture.p.assets.join("script_waypoints.json");
        project::write_json(
            &path,
            &json!([{"waypoint":"SWJ1","seat":1,"cell":[101,102],"padding":5,
                "distance_from_base":2.2,"note":"preserve authored evidence"}]),
        )
        .unwrap();

        refresh_report(&fixture.p, &fixture.map).unwrap();
        let final_report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(final_report[0]["cell"], json!([130, 140]));
        assert_eq!(final_report[0]["distance_from_base"], 50.);
        assert_eq!(final_report[0]["padding"], 5);
        assert_eq!(final_report[0]["note"], "preserve authored evidence");

        let bytes = std::fs::read(&path).unwrap();
        refresh_report(&fixture.p, &fixture.map).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn final_waypoint_report_does_not_invent_missing_evidence() {
        let fixture = ReportFixture::new();
        refresh_report(&fixture.p, &fixture.map).unwrap();
        assert!(!fixture.p.assets.join("script_waypoints.json").exists());
    }

    #[test]
    fn building_footprints_leave_space_at_diagonal_corners() {
        let protected = [([100., 100.], 5.)];
        // Euclidean distance exceeds 12 here, but two 11×11 footprints overlap.
        assert!(!clear([109, 109], 5, &protected));
        assert!(clear([112, 100], 5, &protected));
    }
}
