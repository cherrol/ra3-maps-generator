//! Move eligible scenery out of finalized functional reservations, preserving identity.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    gameplay::{
        placement, production_facing,
        scripts::{self, Node},
    },
    pipeline::project::Project,
    resources::official,
    terrain::{
        self, Layout,
        ramp_clearance::{self, Polygon},
    },
};
use serde_json::{Value as Json, json};

fn cell(frame: &Frame, object: &MapObject) -> [f64; 2] {
    frame.grid([object.position[0] as f64, object.position[1] as f64])
}

fn square(center: [f64; 2], radius: f64) -> Polygon {
    ramp_clearance::rectangle(center, [1., 0.], -radius, radius, radius)
}

// Include official placement and collision extents. A conservative enclosing
// square is intentional here; final official placement still checks exact shapes.
fn extent(kind: &str, frame: &Frame, minimum: f64) -> f64 {
    let Some(definition) = official::object(kind) else {
        return minimum;
    };
    definition
        .shapes
        .iter()
        .chain(definition.placement_shape.iter())
        .filter(|shape| shape.active)
        .map(|shape| {
            (shape.offset[0].hypot(shape.offset[1])
                + shape.major_radius.hypot(shape.minor_radius)
                + definition.rotation_anchor[0].hypot(definition.rotation_anchor[1]))
                / frame.cell_size
        })
        .fold(minimum, f64::max)
}

fn add_building(
    frame: &Frame,
    reserves: &mut Vec<Polygon>,
    kind: &str,
    point: [f64; 2],
    angle: f32,
    minimum: f64,
) {
    reserves.push(square(point, extent(kind, frame, minimum)));
    if let Some(exit) = official::object(kind)
        .and_then(|definition| definition.production_exit.as_ref())
        .filter(|exit| !exit.no_exit_path)
    {
        let create = frame.local_to_grid(point, angle as f64, [exit.create[0], exit.create[1]]);
        let rally = frame.local_to_grid(point, angle as f64, [exit.rally[0], exit.rally[1]]);
        let lo = [
            create[0].min(rally[0]) - minimum,
            create[1].min(rally[1]) - minimum,
        ];
        let hi = [
            create[0].max(rally[0]) + minimum,
            create[1].max(rally[1]) + minimum,
        ];
        reserves.push([lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]]);
    }
}

fn script_reserves(
    nodes: &[Node],
    objects: &[MapObject],
    frame: &Frame,
    reserves: &mut Vec<Polygon>,
    production: &super::production_spawns::PlacementPolicy,
    clearance: usize,
) -> Result<()> {
    for node in nodes {
        if let Some(children) = &node.children {
            script_reserves(children, objects, frame, reserves, production, clearance)?;
            continue;
        }
        if !node.command.as_deref().is_some_and(|command| {
            command.starts_with("CREATE_") && command.contains("AT_WAYPOINT")
        }) {
            continue;
        }
        let args = scripts::string_arguments(node)?;
        let kind = args
            .iter()
            .find(|(ty, _)| *ty == 15)
            .map(|(_, value)| value.as_str());
        let waypoint = args
            .iter()
            .find(|(ty, _)| *ty == 7)
            .map(|(_, value)| value.as_str());
        let (Some(kind), Some(waypoint)) = (kind, waypoint) else {
            continue;
        };
        if !production_facing::gifted_building(kind) && !placement::is_production(kind) {
            continue;
        }
        let object = objects
            .iter()
            .find(|object| {
                prop(&object.props, "waypointName").and_then(Value::text) == Some(waypoint)
            })
            .ok_or_else(|| err(format!("missing functional waypoint {waypoint}")))?;
        let bytes = scripts::unhex(&node.body)?;
        let angle = scripts::scalar_arguments(node)?
            .iter()
            .find(|arg| arg.kind == 17)
            .and_then(|arg| bytes.get(arg.real_offset..arg.real_offset + 4))
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
            .ok_or_else(|| {
                err(format!(
                    "functional reservation requires explicit heading for {waypoint}"
                ))
            })?;
        let padding = super::gift_clearance::kind(kind, production)
            .map(|(_, padding)| padding)
            .unwrap_or(8)
            .max(8)
            + clearance;
        add_building(
            frame,
            reserves,
            kind,
            cell(frame, object),
            angle,
            padding as f64,
        );
    }
    Ok(())
}

struct SceneryGround<'a> {
    frame: &'a Frame,
    heights: &'a [f64],
    impassable: &'a [bool],
    water: f64,
    tolerance: f64,
    radius: f64,
    ramps: &'a [ramp_clearance::Ramp],
}

impl SceneryGround<'_> {
    fn suitable(&self, point: [usize; 2], original_height: f64) -> bool {
        let margin = self.frame.border + self.radius.ceil() as usize;
        let [x, y] = point;
        if x < margin
            || y < margin
            || x + margin >= self.frame.width
            || y + margin >= self.frame.height
        {
            return false;
        }
        let radius = self.radius.ceil() as usize;
        (y - radius..=y + radius).all(|yy| {
            (x - radius..=x + radius).all(|xx| {
                let index = yy * self.frame.width + xx;
                !self.impassable[index]
                    && (self.heights[index] > self.water) == (original_height > self.water)
                    && (self.heights[index] - original_height).abs() <= self.tolerance
            })
        })
    }
}

fn rehome(
    objects: &mut [MapObject],
    movable: &[bool],
    reserves: &[Polygon],
    ground: &SceneryGround<'_>,
) -> Result<Json> {
    let mut candidate = objects.to_vec();
    let mut changes = Vec::new();
    for index in 0..candidate.len() {
        if !movable[index] {
            continue;
        }
        let from = cell(ground.frame, &candidate[index]);
        let functional_clear = |point: [f64; 2]| {
            let footprint = square(point, ground.radius);
            let x = (point[0].round().max(0.) as usize).min(ground.frame.width - 1);
            let y = (point[1].round().max(0.) as usize).min(ground.frame.height - 1);
            let height = ground.heights[y * ground.frame.width + x];
            reserves
                .iter()
                .all(|reserve| ramp_clearance::polygon_distance(&footprint, reserve) > 0.)
                && ground.ramps.iter().all(|ramp| {
                    let delta = [point[0] - ramp.center[0], point[1] - ramp.center[1]];
                    let along = delta[0] * ramp.unit[0] + delta[1] * ramp.unit[1];
                    let expected = if along >= 0. {
                        ramp.pair[1]
                    } else {
                        ramp.pair[0]
                    };
                    (along.abs() > ramp.half && (height - expected).abs() > 12.)
                        || height < ramp.pair[0] - 12.
                        || height > ramp.pair[1] + 12.
                        || ramp_clearance::polygon_distance(&footprint, &ramp.approach) > 0.
                })
        };
        let clear = |point: [f64; 2], others: &[MapObject]| {
            functional_clear(point)
                && others.iter().enumerate().all(|(other, object)| {
                    other == index
                        || !movable[other]
                        || (cell(ground.frame, object)[0] - point[0])
                            .hypot(cell(ground.frame, object)[1] - point[1])
                            >= 2. * ground.radius
                })
        };
        // Existing scenery clusters are outside this edit's scope. Only a
        // functional conflict authorizes movement; spacing applies to new sites.
        if functional_clear(from) {
            continue;
        }
        let origin = [from[0].round() as usize, from[1].round() as usize];
        if origin[0] >= ground.frame.width || origin[1] >= ground.frame.height {
            return Err(err("cannot relocate out-of-frame scenery"));
        }
        let height = ground.heights[origin[1] * ground.frame.width + origin[0]];
        let mut target = None;
        for distance in 1..ground.frame.width.max(ground.frame.height) {
            let mut ring = Vec::new();
            for y in origin[1].saturating_sub(distance)
                ..=(origin[1] + distance).min(ground.frame.height - 1)
            {
                for x in origin[0].saturating_sub(distance)
                    ..=(origin[0] + distance).min(ground.frame.width - 1)
                {
                    if x.abs_diff(origin[0]).max(y.abs_diff(origin[1])) == distance
                        && ground.suitable([x, y], height)
                        && clear([x as f64, y as f64], &candidate)
                    {
                        ring.push((
                            x.abs_diff(origin[0]).pow(2) + y.abs_diff(origin[1]).pow(2),
                            y,
                            x,
                        ));
                    }
                }
            }
            if let Some((_, y, x)) = ring.into_iter().min() {
                target = Some([x, y]);
                break;
            }
        }
        let to = target.ok_or_else(|| {
            err(format!(
                "no legal scenery replacement for object {}",
                index + 1
            ))
        })?;
        let world = ground.frame.world(to.map(|value| value as f64));
        candidate[index].position[0] = world[0] as f32;
        candidate[index].position[1] = world[1] as f32;
        changes.push(
            json!({"object_id":index+1,"kind":candidate[index].type_name,"before":from,"after":to}),
        );
    }
    objects.clone_from_slice(&candidate);
    Ok(
        json!({"algorithm":"functional-reservations-before-eligible-scenery-v1","changes":changes,
        "scope":"only eligible scenery intersecting final functional reservations; identity and terrain remain unchanged"}),
    )
}

pub fn yield_to_functional(p: &Project, map: &mut MapFile, layout: &Layout) -> Result<Json> {
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    if !objects.iter().any(placement::is_decoration) {
        return Ok(json!({"changes":[],"scope":"no eligible movable scenery"}));
    }
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let spatial = terrain::spatial::policy(p)?;
    let corridor =
        ramp_clearance::policy(p)?.ok_or_else(|| err("missing ramp reservation policy"))?;
    let ramps = ramp_clearance::ramps(layout, &corridor)?;
    let mut reserves = Vec::new();
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let production = super::production_spawns::PlacementPolicy::load(p)?;
    let clearance = super::gift_clearance::decoration_clearance(p)?;
    script_reserves(
        &nodes,
        &objects,
        &frame,
        &mut reserves,
        &production,
        clearance,
    )?;
    let movable = objects
        .iter()
        .map(placement::is_decoration)
        .collect::<Vec<_>>();
    for (index, object) in objects.iter().enumerate() {
        if movable[index] {
            continue;
        }
        let point = cell(&frame, object);
        if object.type_name.contains("Waypoint") {
            let name = prop(&object.props, "waypointName")
                .and_then(Value::text)
                .unwrap_or("");
            if name.starts_with("Player_") {
                reserves.push(square(point, spatial.building_reserve_radius));
            }
            continue;
        }
        let radius =
            ramp_clearance::radius(&object.type_name).unwrap_or(spatial.building_reserve_radius);
        add_building(
            &frame,
            &mut reserves,
            &object.type_name,
            point,
            object.angle,
            radius,
        );
        if object.type_name == "OreNode" {
            let angle = object.angle as f64;
            reserves.push(ramp_clearance::rectangle(
                point,
                [angle.cos(), -angle.sin()],
                -corridor.refinery_back,
                corridor.refinery_forward,
                corridor.refinery_half_width,
            ));
        }
    }
    let heights = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|value| *value as f64 * frame.z_scale)
        .collect::<Vec<_>>();
    let impassable = terrain::flip(&bt.impassability, hm.width);
    let report = rehome(
        &mut objects,
        &movable,
        &reserves,
        &SceneryGround {
            frame: &frame,
            heights: &heights,
            impassable: &impassable,
            water: p.number("WATER_Z", 200.),
            tolerance: spatial.flat_tolerance,
            radius: spatial.decoration_reserve_radius,
            ramps: &ramps,
        },
    )?;
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Frame, Vec<MapObject>) {
        let frame = Frame {
            width: 32,
            height: 24,
            border: 1,
            cell_size: 10.,
            z_scale: 1.,
        };
        let world = frame.world([12., 12.]);
        let object = MapObject {
            version: 1,
            position: [world[0] as f32, world[1] as f32, 0.],
            angle: 0.7,
            road_type: 0,
            type_name: "HA_GardenRock01".into(),
            props: vec![],
        };
        (frame, vec![object])
    }
    #[test]
    fn scenery_yields_without_changing_identity_or_functional_reservations() {
        let (frame, mut objects) = fixture();
        let original = objects[0].clone();
        let reserves = [square([12., 12.], 4.)];
        let heights = vec![210.; frame.width * frame.height];
        let impassable = vec![false; heights.len()];
        let ground = SceneryGround {
            frame: &frame,
            heights: &heights,
            impassable: &impassable,
            water: 200.,
            tolerance: 3.,
            radius: 1.,
            ramps: &[],
        };
        let report = rehome(&mut objects, &[true], &reserves, &ground).unwrap();
        assert_eq!(report["changes"].as_array().unwrap().len(), 1);
        assert!(
            ramp_clearance::polygon_distance(&square(cell(&frame, &objects[0]), 1.), &reserves[0])
                > 0.
        );
        assert_eq!(objects[0].type_name, original.type_name);
        assert_eq!(objects[0].props, original.props);
        assert_eq!(objects[0].angle, original.angle);
        assert!(
            rehome(&mut objects, &[true], &reserves, &ground).unwrap()["changes"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn unrelated_scenery_clusters_and_other_level_ramp_projections_stay_unchanged() {
        let (frame, mut objects) = fixture();
        let mut neighbor = objects[0].clone();
        neighbor.position[0] += frame.cell_size as f32;
        objects.push(neighbor);
        let before = objects.clone();
        let heights = vec![210.; frame.width * frame.height];
        let impassable = vec![false; heights.len()];
        let ramps = [ramp_clearance::Ramp {
            id: 3,
            center: [12., 12.],
            unit: [1., 0.],
            half: 4.,
            width: 8.,
            pair: [280., 350.],
            approach: square([12., 12.], 8.),
        }];
        let ground = SceneryGround {
            frame: &frame,
            heights: &heights,
            impassable: &impassable,
            water: 200.,
            tolerance: 3.,
            radius: 2.,
            ramps: &ramps,
        };
        rehome(&mut objects, &[true, true], &[], &ground).unwrap();
        assert_eq!(objects, before);
    }
    #[test]
    fn impossible_relocation_is_atomic_and_protected_objects_stay_put() {
        let (frame, mut objects) = fixture();
        let before = objects.clone();
        let heights = vec![210.; frame.width * frame.height];
        let impassable = vec![true; heights.len()];
        let ground = SceneryGround {
            frame: &frame,
            heights: &heights,
            impassable: &impassable,
            water: 200.,
            tolerance: 3.,
            radius: 1.,
            ramps: &[],
        };
        let reserves = [square([12., 12.], 4.)];
        assert!(rehome(&mut objects, &[true], &reserves, &ground).is_err());
        assert_eq!(objects, before);
        rehome(&mut objects, &[false], &reserves, &ground).unwrap();
        assert_eq!(objects, before);
    }
}
