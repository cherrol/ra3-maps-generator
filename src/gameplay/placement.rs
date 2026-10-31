//! Shared deterministic placement priorities and static base-access estimates.
use crate::{
    Result, err,
    formats::map::{MapObject, Value, prop},
    pipeline::project::Project,
    resources::official,
    terrain::{grid::neighbors_rect, spatial},
};
use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, VecDeque},
};

pub(crate) fn is_production(kind: &str) -> bool {
    official::object(kind).is_some_and(|definition| {
        definition.kind_of.iter().any(|flag| flag == "STRUCTURE")
            && !definition
                .kind_of
                .iter()
                .any(|flag| flag == "CONSTRUCTION_YARD")
            && definition.kind_of.iter().any(|flag| {
                matches!(
                    flag.as_str(),
                    "FS_BARRACKS" | "FS_WAR_FACTORY" | "FS_AIR_FIELD"
                )
            })
    }) || ["barracks", "warfactory", "airfield", "navalyard"]
        .iter()
        .any(|suffix| kind.to_ascii_lowercase().ends_with(suffix))
}

pub(crate) fn ground_rally_omission(kind: &str) -> Option<&'static str> {
    let definition = official::object(kind)?;
    if !definition.pending.is_empty() {
        return None;
    }
    if definition
        .production_exit
        .as_ref()
        .is_some_and(|exit| exit.no_exit_path)
    {
        Some("explicit_no_exit_path")
    } else if definition.production_exit.is_none()
        && definition.kind_of.iter().any(|flag| flag == "FS_AIR_FIELD")
    {
        Some("airfield_without_ground_rally")
    } else {
        None
    }
}

/// Only anonymous neutral props with a resolved official identity may yield.
/// Named props remain protected because their identity may be a script target.
pub(crate) fn is_decoration(object: &MapObject) -> bool {
    if !["Tree", "Bush", "GardenRock"]
        .iter()
        .any(|part| object.type_name.contains(part))
        || ["uniqueID", "objectName", "name", "waypointName"]
            .iter()
            .any(|key| {
                prop(&object.props, key)
                    .and_then(Value::text)
                    .is_some_and(|v| !v.is_empty())
            })
        || ["playerName", "teamName"].iter().any(|key| {
            prop(&object.props, key)
                .and_then(Value::text)
                .is_some_and(|value| !value.is_empty() && value != "PlyrCivilian")
        })
    {
        return false;
    }
    official::object(&object.type_name).is_some_and(|definition| {
        definition.pending.is_empty()
            && definition
                .kind_of
                .iter()
                .any(|flag| flag == "SIMPLE_OBJECT_PROP")
            && !definition
                .kind_of
                .iter()
                .any(|flag| matches!(flag.as_str(), "FS_FACTORY" | "TECH_BUILDING" | "REPAIR_PAD"))
    })
}

/// Same-domain four-neighbor paths use the effective navigation step/clearance.
/// The opposite domain is ranked using a reachable shore and a separate second
/// leg; this is an access estimate, not a claim that land units traverse water.
pub(crate) struct AccessDistances {
    width: usize,
    height: usize,
    land: Vec<u32>,
    water: Vec<u32>,
    clearance: usize,
    score_cache: RefCell<BTreeMap<(bool, usize), Vec<u32>>>,
}

impl AccessDistances {
    pub(crate) fn new(
        p: &Project,
        width: usize,
        z: &[f64],
        impassable: &[bool],
        objects: &[MapObject],
        seat: usize,
    ) -> Result<Self> {
        if width == 0
            || z.is_empty()
            || !z.len().is_multiple_of(width)
            || z.len() != impassable.len()
            || !(1..=8).contains(&seat)
        {
            return Err(err("invalid placement access grid or seat"));
        }
        let height = z.len() / width;
        let policy = spatial::policy(p)?;
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
                "placement access requires one actual start for P{seat}"
            )));
        }
        let base = super::script_spawns::to_cell(p, starts[0]);
        if base.iter().any(|v| !v.is_finite() || *v < 0.)
            || base[0] >= width as f64
            || base[1] >= height as f64
        {
            return Err(err("placement base outside native grid"));
        }
        let base_index = base[1].round().min((height - 1) as f64) as usize * width
            + base[0].round().min((width - 1) as f64) as usize;
        let water_height = p.number("WATER_Z", 200.);
        let domain: Vec<_> = z.iter().map(|value| *value <= water_height).collect();
        let mut blocked = impassable.to_vec();
        let frame = crate::coordinates::Frame::from_project(p);
        for object in objects {
            if object.type_name.contains("Waypoint")
                || is_decoration(object)
                || official::object(&object.type_name).is_some_and(|definition| {
                    definition.kind_of.iter().any(|flag| flag == "NO_COLLIDE")
                })
            {
                continue;
            }
            let Some(definition) = official::object(&object.type_name) else {
                continue;
            };
            let radius = definition
                .shapes
                .iter()
                .filter(|shape| shape.active)
                .map(|shape| {
                    shape.major_radius.hypot(shape.minor_radius)
                        + shape.offset[0].hypot(shape.offset[1])
                })
                .fold(0., f64::max)
                / frame.cell_size;
            if radius == 0. {
                continue;
            }
            let center = super::script_spawns::to_cell(p, object);
            for y in (center[1] - radius).floor().max(0.) as usize
                ..=((center[1] + radius).ceil().max(0.) as usize).min(height - 1)
            {
                for x in (center[0] - radius).floor().max(0.) as usize
                    ..=((center[0] + radius).ceil().max(0.) as usize).min(width - 1)
                {
                    if (x as f64 - center[0]).hypot(y as f64 - center[1]) <= radius {
                        blocked[y * width + x] = true;
                    }
                }
            }
        }
        let border = p.usize("BORDER", 20);
        let raw = blocked.clone();
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                let margin = policy.navigation_clearance;
                blocked[i] = x < border + margin
                    || y < border + margin
                    || x + border + margin >= width
                    || y + border + margin >= height
                    || (y.saturating_sub(margin)..=(y + margin).min(height - 1)).any(|yy| {
                        (x.saturating_sub(margin)..=(x + margin).min(width - 1))
                            .any(|xx| raw[yy * width + xx])
                    });
            }
        }
        let primary_domain = domain[base_index];
        let mut seeds = Vec::new();
        let access_radius = crate::terrain::spawn_placement::policy(p)?.base_radius
            + policy.navigation_clearance as f64
            + 3.;
        for y in (base[1] - access_radius).floor().max(0.) as usize
            ..=((base[1] + access_radius).ceil() as usize).min(height - 1)
        {
            for x in (base[0] - access_radius).floor().max(0.) as usize
                ..=((base[0] + access_radius).ceil() as usize).min(width - 1)
            {
                let i = y * width + x;
                if !blocked[i]
                    && domain[i] == primary_domain
                    && (z[i] - z[base_index]).abs() <= policy.flat_tolerance
                    && (x as f64 - base[0]).hypot(y as f64 - base[1]) <= access_radius
                {
                    seeds.push((0, i));
                }
            }
        }
        // A single verified access point cannot jump across a wall inside the
        // base reservation by seeding disconnected cells on both sides.
        seeds.sort_by(|a, b| {
            let distance =
                |i: usize| ((i % width) as f64 - base[0]).hypot((i / width) as f64 - base[1]);
            distance(a.1).total_cmp(&distance(b.1)).then(a.1.cmp(&b.1))
        });
        seeds.truncate(1);
        let primary = distances(
            width,
            height,
            z,
            &blocked,
            &domain,
            primary_domain,
            policy.maximum_walk_step,
            seeds,
        );
        let mut shore = Vec::new();
        for (i, value) in primary
            .iter()
            .enumerate()
            .filter(|(_, value)| **value != u32::MAX)
        {
            for j in neighbors_rect(i, width, height) {
                if !blocked[j] && domain[j] != primary_domain {
                    shore.push((value.saturating_add(1), j));
                }
            }
        }
        let secondary = distances(
            width,
            height,
            z,
            &blocked,
            &domain,
            !primary_domain,
            policy.maximum_walk_step,
            shore,
        );
        let (land, water) = if primary_domain {
            (secondary, primary)
        } else {
            (primary, secondary)
        };
        Ok(Self {
            width,
            height,
            land,
            water,
            clearance: policy.navigation_clearance,
            score_cache: RefCell::new(BTreeMap::new()),
        })
    }

    /// Actual rotated rally targets use the distance field directly, so an
    /// accessible foundation cannot hide a disconnected production exit.
    pub(crate) fn score_point(&self, point: [f64; 2], wet: bool) -> Option<u32> {
        if point.iter().any(|value| !value.is_finite() || *value < 0.)
            || point[0] > (self.width - 1) as f64
            || point[1] > (self.height - 1) as f64
        {
            return None;
        }
        let distances = if wet { &self.water } else { &self.land };
        let value = distances[point[1].round() as usize * self.width + point[0].round() as usize];
        (value != u32::MAX).then_some(value)
    }

    pub(crate) fn score(&self, cell: [usize; 2], wet: bool, padding: usize) -> Option<u32> {
        let [x, y] = cell;
        if x >= self.width || y >= self.height {
            return None;
        }
        let mut cache = self.score_cache.borrow_mut();
        let values = cache.entry((wet, padding)).or_insert_with(|| {
            let distances = if wet { &self.water } else { &self.land };
            let radius = (padding + self.clearance + 1) as isize;
            let pad = padding as isize;
            // Four disjoint rectangles cover the exact access ring. Sliding
            // minima cost O(grid), so candidate queries remain O(1).
            let mut values = vec![u32::MAX; distances.len()];
            for (xl, xh, yl, yh) in [
                (-radius, radius, -radius, -pad),
                (-radius, radius, pad, radius),
                (-radius, -pad, -pad + 1, pad - 1),
                (pad, radius, -pad + 1, pad - 1),
            ] {
                if xl > xh || yl > yh {
                    continue;
                }
                let rectangle =
                    rectangle_minimum(distances, self.width, self.height, (xl, xh), (yl, yh));
                for (value, candidate) in values.iter_mut().zip(rectangle) {
                    *value = (*value).min(candidate);
                }
            }
            values
        });
        let best = values[y * self.width + x];
        (best != u32::MAX).then_some(best)
    }
}

fn window_minimum(values: &[u32], low: isize, high: isize) -> Vec<u32> {
    let mut result = vec![u32::MAX; values.len()];
    let mut queue = VecDeque::<usize>::new();
    let mut next = 0;
    for (center, out) in result.iter_mut().enumerate() {
        let right = (center as isize + high).min(values.len() as isize - 1);
        while right >= 0 && next <= right as usize {
            while queue
                .back()
                .is_some_and(|index| values[*index] >= values[next])
            {
                queue.pop_back();
            }
            queue.push_back(next);
            next += 1;
        }
        let left = (center as isize + low).max(0);
        while queue.front().is_some_and(|index| (*index as isize) < left) {
            queue.pop_front();
        }
        if left <= right
            && let Some(index) = queue.front()
        {
            *out = values[*index];
        }
    }
    result
}

fn rectangle_minimum(
    values: &[u32],
    width: usize,
    height: usize,
    x: (isize, isize),
    y: (isize, isize),
) -> Vec<u32> {
    let mut rows = Vec::with_capacity(values.len());
    for row in values.chunks_exact(width) {
        rows.extend(window_minimum(row, x.0, x.1));
    }
    let mut output = vec![u32::MAX; values.len()];
    for column in 0..width {
        let values: Vec<_> = (0..height).map(|row| rows[row * width + column]).collect();
        for (row, value) in window_minimum(&values, y.0, y.1).into_iter().enumerate() {
            output[row * width + column] = value;
        }
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn distances(
    width: usize,
    height: usize,
    z: &[f64],
    blocked: &[bool],
    water: &[bool],
    domain: bool,
    maximum_step: f64,
    seeds: Vec<(u32, usize)>,
) -> Vec<u32> {
    let mut values = vec![u32::MAX; z.len()];
    let mut queue = BinaryHeap::new();
    for (distance, index) in seeds {
        if distance < values[index] {
            values[index] = distance;
            queue.push(Reverse((distance, index)));
        }
    }
    while let Some(Reverse((distance, i))) = queue.pop() {
        if distance != values[i] {
            continue;
        }
        for j in neighbors_rect(i, width, height) {
            let next = distance.saturating_add(1);
            if !blocked[j]
                && water[j] == domain
                && next < values[j]
                && (z[i] - z[j]).abs() <= maximum_step
            {
                values[j] = next;
                queue.push(Reverse((next, j)));
            }
        }
    }
    values
}

/// Recheck the finished placement after every primary/fallback/branch has been
/// overlaid. Initial candidate distance fields cannot authorize final access.
pub(crate) fn validate_final_access(
    p: &Project,
    map: &crate::formats::map::MapFile,
) -> Result<serde_json::Value> {
    use crate::formats::map::{self, BlendTiles, HeightMap};
    use serde_json::json;
    if p.flag("TERRAIN_ONLY") {
        return Ok(
            json!({"status":"not_applicable","reason":"terrain-only input has no gameplay production"}),
        );
    }
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let frame = crate::coordinates::Frame::from_heightmap(p, &hm)?;
    let z: Vec<_> = crate::terrain::flip(&hm.elevations, hm.width)
        .into_iter()
        .map(|value| value as f64 * frame.z_scale)
        .collect();
    let impassable = crate::terrain::flip(&bt.impassability, hm.width);
    let blocked = crate::terrain::flip(&bt.buildability, hm.width);
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let nodes = super::scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let targets = super::production_spawns::bindings(&nodes, &objects)?;
    let owners = super::production_spawns::owners(&nodes, &objects)?;
    if targets.is_empty() {
        return Ok(
            json!({"passed":true,"entries":[],"native_decoded_crc32":crc32fast::hash(&map.encode()?)}),
        );
    }
    let checker = super::official_placement::Checker::for_candidates(
        p,
        hm.width,
        &z,
        &impassable,
        &blocked,
        &nodes,
        &objects,
    )?
    .ok_or_else(|| err("final production access requires strict official geometry"))?;
    let mut counts = vec![0_u32; z.len()];
    let mut footprints = Vec::with_capacity(objects.len());
    for object in &objects {
        let cells = checker.navigation_cells(std::slice::from_ref(object))?;
        for index in &cells {
            counts[*index] += 1;
        }
        footprints.push(cells);
    }
    let complete: Vec<_> = impassable
        .iter()
        .zip(&counts)
        .map(|(blocked, count)| *blocked || *count != 0)
        .collect();
    let markers: Vec<_> = objects
        .iter()
        .filter(|object| object.type_name.contains("Waypoint"))
        .cloned()
        .collect();
    let mut fields = BTreeMap::new();
    let policy = spatial::policy(p)?;
    let production = super::production_spawns::PlacementPolicy::load(p)?;
    let mut entries = Vec::new();
    for (index, wet) in targets {
        let object = &objects[index];
        // A neutral technology shipyard is a resource, not a player production
        // gift. No nearest-base ownership is invented for that resource.
        if !object.type_name.contains("Waypoint") && !is_production(&object.type_name) {
            continue;
        }
        let seat = owners.get(&index).copied().ok_or_else(|| {
            err(format!(
                "final production object {} requires a verified owning seat",
                index + 1
            ))
        })?;
        if let std::collections::btree_map::Entry::Vacant(entry) = fields.entry(seat) {
            entry.insert(AccessDistances::new(
                p, hm.width, &z, &complete, &markers, seat,
            )?);
        }
        let field = &fields[&seat];
        let point =
            super::script_spawns::to_cell(p, object).map(|value| value.round_ties_even() as usize);
        let rally = checker.object_production_access_targets(object, point)?;
        if rally.is_empty() {
            let padding = production
                .footprint(&object.type_name)
                .map(|(_, padding)| padding)
                .unwrap_or(8);
            let steps = field.score(point, wet, padding).ok_or_else(|| {
                err(format!(
                    "final production object {} has no base access after complete placement",
                    index + 1
                ))
            })?;
            entries.push(json!({"object_id":index+1,"seat":seat,"status":"foundation_access_only",
                "ground_rally":"not_applicable","basis":checker.production_ground_access_basis(object),
                "foundation_access_steps":steps}));
        }
        let mut excluded_field = None;
        for (domain, target) in rally {
            let direct = reachable_without_self(
                field,
                target,
                domain,
                &z,
                &impassable,
                &counts,
                &footprints[index],
                &policy,
                frame.border,
                p.number("WATER_Z", 200.),
            );
            // Removing one's own foundation can expose the only usable shore.
            // Rebuild that object's two-leg seeds only when the shared field
            // and local same-domain reconnection cannot establish access.
            let steps = if direct.is_some() { direct } else {
                if excluded_field.is_none() {
                    excluded_field = Some(field_without_self(p,hm.width,&z,&impassable,
                        &counts,&footprints[index],&markers,seat)?);
                }
                excluded_field.as_ref().and_then(|field| field.score_point(target,domain))
            }
                .ok_or_else(|| err(format!("final production exit of object {} {:?} is disconnected from P{seat} after all primary/fallback footprints", index+1,
                    prop(&object.props,"waypointName").and_then(Value::text))))?;
            entries.push(json!({"object_id":index+1,"seat":seat,"waypoint":prop(&object.props,"waypointName").and_then(Value::text),
                "domain":if domain {"water"}else{"land"},"rally":target,"access_steps":steps,"passed":true}));
        }
    }
    Ok(
        json!({"passed":true,"native_decoded_crc32":crc32fast::hash(&map.encode()?),"entries":entries,
        "model":"all final native and conditional primary/fallback placement/collision foundations; effective clearance; actual rally targets; own footprint excluded only for its own access; separate shore legs for mixed-domain access"}),
    )
}

#[allow(clippy::too_many_arguments)]
fn field_without_self(
    p: &Project,
    width: usize,
    z: &[f64],
    impassable: &[bool],
    counts: &[u32],
    own: &[usize],
    markers: &[MapObject],
    seat: usize,
) -> Result<AccessDistances> {
    let complete: Vec<_> = impassable
        .iter()
        .enumerate()
        .map(|(index, blocked)| {
            *blocked || counts[index] > u32::from(own.binary_search(&index).is_ok())
        })
        .collect();
    AccessDistances::new(p, width, z, &complete, markers, seat)
}

#[allow(clippy::too_many_arguments)]
fn reachable_without_self(
    field: &AccessDistances,
    point: [f64; 2],
    wet: bool,
    z: &[f64],
    impassable: &[bool],
    counts: &[u32],
    own: &[usize],
    policy: &spatial::Policy,
    border: usize,
    water: f64,
) -> Option<u32> {
    if let Some(steps) = field.score_point(point, wet) {
        return Some(steps);
    }
    if point.iter().any(|value| !value.is_finite() || *value < 0.)
        || point[0] > (field.width - 1) as f64
        || point[1] > (field.height - 1) as f64
    {
        return None;
    }
    let start = point[1].round() as usize * field.width + point[0].round() as usize;
    let open = |index: usize| {
        let x = index % field.width;
        let y = index / field.width;
        let margin = policy.navigation_clearance;
        x >= border + margin
            && y >= border + margin
            && x + border + margin < field.width
            && y + border + margin < field.height
            && (z[index] <= water) == wet
            && !(y - margin..=y + margin).any(|yy| {
                (x - margin..=x + margin).any(|xx| {
                    let index = yy * field.width + xx;
                    impassable[index]
                        || counts[index] > u32::from(own.binary_search(&index).is_ok())
                })
            })
    };
    if !open(start) {
        return None;
    }
    let mut seen = vec![false; z.len()];
    seen[start] = true;
    let mut queue = VecDeque::from([(start, 0_u32)]);
    while let Some((index, distance)) = queue.pop_front() {
        if let Some(steps) = field.score_point(
            [(index % field.width) as f64, (index / field.width) as f64],
            wet,
        ) {
            return Some(distance.saturating_add(steps));
        }
        for next in neighbors_rect(index, field.width, field.height) {
            if !seen[next] && open(next) && (z[index] - z[next]).abs() <= policy.maximum_walk_step {
                seen[next] = true;
                queue.push_back((next, distance + 1));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_access_ring_matches_exhaustive_queries_on_rectangular_grids() {
        for (width, height) in [(3, 7), (7, 3), (8, 6)] {
            let values: Vec<_> = (0..width * height)
                .map(|i| {
                    if i % 4 == 0 {
                        u32::MAX
                    } else {
                        ((i * 19) % 37) as u32
                    }
                })
                .collect();
            for clearance in 0..=2 {
                let access = AccessDistances {
                    width,
                    height,
                    land: values.clone(),
                    water: vec![u32::MAX; values.len()],
                    clearance,
                    score_cache: RefCell::new(BTreeMap::new()),
                };
                for padding in 0..=3 {
                    let radius = padding + clearance + 1;
                    for y in 0..height {
                        for x in 0..width {
                            let expected = (0..height)
                                .flat_map(|yy| {
                                    let values = &values;
                                    (0..width).filter_map(move |xx| {
                                        let distance = xx.abs_diff(x).max(yy.abs_diff(y));
                                        (distance >= padding && distance <= radius)
                                            .then_some(values[yy * width + xx])
                                    })
                                })
                                .min()
                                .filter(|value| *value != u32::MAX);
                            assert_eq!(access.score([x, y], false, padding), expected);
                            assert_eq!(access.score([x, y], true, padding), None);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn static_routes_respect_walls_height_steps_and_rectangular_axes() {
        let width = 11;
        let height = 7;
        let mut blocked = vec![false; width * height];
        for y in 0..height {
            blocked[y * width + 5] = true;
        }
        let mut z = vec![210.; blocked.len()];
        let domain = vec![false; blocked.len()];
        let start = 3 * width + 2;
        let target = 3 * width + 8;
        assert_eq!(
            distances(
                width,
                height,
                &z,
                &blocked,
                &domain,
                false,
                3.,
                vec![(0, start)]
            )[target],
            u32::MAX
        );
        blocked[width + 5] = false;
        let route = distances(
            width,
            height,
            &z,
            &blocked,
            &domain,
            false,
            3.,
            vec![(0, start)],
        );
        assert_eq!(route[target], 10);
        z[width + 5] = 230.;
        assert_eq!(
            distances(
                width,
                height,
                &z,
                &blocked,
                &domain,
                false,
                3.,
                vec![(0, start)]
            )[target],
            u32::MAX
        );
    }

    #[test]
    fn named_or_player_owned_props_never_yield_as_decoration() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let native =
            crate::formats::map::MapFile::load(root.join("templates/template.map")).unwrap();
        let mut object =
            crate::formats::map::objects(native.get("ObjectsList").unwrap(), &native.table)
                .unwrap()
                .remove(0);
        object.type_name = "CC_Tree01".into();
        object.props.clear();
        assert!(is_decoration(&object));
        crate::formats::map::set(
            &mut object.props,
            "uniqueID",
            Value::Ascii("scripted-tree".into()),
        );
        assert!(!is_decoration(&object));
        object.props.clear();
        crate::formats::map::set(
            &mut object.props,
            "playerName",
            Value::Ascii("Player_1".into()),
        );
        assert!(!is_decoration(&object));
        object.props.clear();
        object.type_name = "UnknownTreeFactory".into();
        assert!(!is_decoration(&object));
    }

    #[test]
    fn construction_yards_and_mobile_cores_are_never_relocated_as_production_gifts() {
        assert!(is_production("AlliedWarFactory"));
        assert!(is_production("SovietAirfield"));
        assert!(is_production("JapanBarracks"));
        assert!(is_production("AlliedNavalYard"));
        assert!(!is_production("SovietConstructionYard"));
        assert!(!is_production("AlliedMCV"));
        assert!(!is_production("JapanWarFactoryEgg"));
        assert!(!is_production("AlliedPowerPlantTurbine"));
    }

    #[test]
    fn nearby_foundation_does_not_hide_a_disconnected_official_rally_target() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "河南").unwrap();
        p.cfg["GRID"] = serde_json::json!(128);
        p.cfg["GRID_HEIGHT"] = serde_json::json!(128);
        p.cfg["BORDER"] = serde_json::json!(4);
        let frame = crate::coordinates::Frame::from_project(&p);
        let native =
            crate::formats::map::MapFile::load(root.join("templates/template.map")).unwrap();
        let mut start =
            crate::formats::map::objects(native.get("ObjectsList").unwrap(), &native.table)
                .unwrap()
                .into_iter()
                .find(|object| object.type_name.contains("Waypoint"))
                .unwrap();
        start.props.clear();
        let world = frame.world([40., 80.]);
        start.position = [world[0] as f32, world[1] as f32, 0.];
        crate::formats::map::set(
            &mut start.props,
            "waypointName",
            Value::Ascii("Player_1_Start".into()),
        );
        let mut factory = start.clone();
        factory.props.clear();
        factory.type_name = "AlliedWarFactory".into();
        factory.angle = 45_f32.to_radians();
        let world = frame.world([64., 68.]);
        factory.position = [world[0] as f32, world[1] as f32, 0.];
        let z = vec![210.; 128 * 128];
        let mut impassable = vec![false; z.len()];
        impassable[61 * 128 + 68] = true;
        let access = AccessDistances::new(&p, 128, &z, &impassable, &[start.clone()], 1).unwrap();
        assert!(access.score([64, 68], false, 6).is_some());
        let checker = super::super::official_placement::Checker::for_candidates(
            &p,
            128,
            &z,
            &impassable,
            &vec![false; z.len()],
            &[],
            &[start.clone(), factory.clone()],
        )
        .unwrap()
        .unwrap();
        let targets = checker
            .object_production_access_targets(&factory, [64, 68])
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(access.score_point(targets[0].1, targets[0].0), None);
        let findings =
            checker.object_rejections(&factory, [64, 68], &checker.planned_obstacles(&[start]));
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
    }

    #[test]
    fn later_foundation_cannot_cut_an_earlier_exit_or_be_erased_as_someone_elses_self() {
        let width = 11;
        let height = 7;
        let z = vec![210.; width * height];
        let water = vec![false; z.len()];
        let mut impassable = vec![false; z.len()];
        for row in 0..height {
            impassable[row * width + 5] = row != 3;
        }
        let start = 3 * width + 2;
        let target = [8., 3.];
        let initial = distances(
            width,
            height,
            &z,
            &impassable,
            &water,
            false,
            3.,
            vec![(0, start)],
        );
        assert_eq!(initial[3 * width + 8], 6);
        let later_foundation = 3 * width + 5;
        let mut counts = vec![0; z.len()];
        counts[later_foundation] = 1;
        let complete: Vec<_> = impassable
            .iter()
            .zip(&counts)
            .map(|(blocked, count)| *blocked || *count != 0)
            .collect();
        let field = AccessDistances {
            width,
            height,
            land: distances(
                width,
                height,
                &z,
                &complete,
                &water,
                false,
                3.,
                vec![(0, start)],
            ),
            water: vec![u32::MAX; z.len()],
            clearance: 0,
            score_cache: RefCell::new(BTreeMap::new()),
        };
        let mut policy: spatial::Policy =
            serde_json::from_str(include_str!("../../rules/coordinate-pipeline.json")).unwrap();
        policy.navigation_clearance = 0;
        assert_eq!(
            reachable_without_self(
                &field,
                target,
                false,
                &z,
                &impassable,
                &counts,
                &[],
                &policy,
                0,
                200.
            ),
            None
        );
        assert_eq!(
            reachable_without_self(
                &field,
                target,
                false,
                &z,
                &impassable,
                &counts,
                &[later_foundation],
                &policy,
                0,
                200.
            ),
            Some(6)
        );
        counts[later_foundation] = 2;
        assert_eq!(
            reachable_without_self(
                &field,
                target,
                false,
                &z,
                &impassable,
                &counts,
                &[later_foundation],
                &policy,
                0,
                200.
            ),
            None
        );
    }

    #[test]
    fn excluding_a_shipyard_can_reopen_its_only_shore_access() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "河南").unwrap();
        p.cfg["GRID"] = serde_json::json!(128);
        p.cfg["GRID_HEIGHT"] = serde_json::json!(128);
        p.cfg["BORDER"] = serde_json::json!(4);
        let frame = crate::coordinates::Frame::from_project(&p);
        let native =
            crate::formats::map::MapFile::load(root.join("templates/template.map")).unwrap();
        let mut start =
            crate::formats::map::objects(native.get("ObjectsList").unwrap(), &native.table)
                .unwrap()
                .into_iter()
                .find(|object| object.type_name.contains("Waypoint"))
                .unwrap();
        start.props.clear();
        crate::formats::map::set(
            &mut start.props,
            "waypointName",
            Value::Ascii("Player_1_Start".into()),
        );
        let world = frame.world([40., 80.]);
        start.position = [world[0] as f32, world[1] as f32, 0.];
        let mut z = vec![210.; 128 * 128];
        let mut impassable = vec![true; z.len()];
        for y in 70..=90 {
            for x in 20..70 {
                impassable[y * 128 + x] = false;
            }
        }
        for y in 78..=82 {
            for x in 70..=110 {
                impassable[y * 128 + x] = false;
                z[y * 128 + x] = 130.;
            }
        }
        let mut own = Vec::new();
        let mut counts = vec![0; z.len()];
        for y in 70..=90 {
            for x in 68..=77 {
                let index = y * 128 + x;
                own.push(index);
                counts[index] = 1;
            }
        }
        let complete: Vec<_> = impassable
            .iter()
            .zip(&counts)
            .map(|(blocked, count)| *blocked || *count != 0)
            .collect();
        let field =
            AccessDistances::new(&p, 128, &z, &complete, std::slice::from_ref(&start), 1).unwrap();
        assert!(field.water.iter().all(|value| *value == u32::MAX));
        assert_eq!(field.score_point([90., 80.], true), None);
        let reopened =
            field_without_self(&p, 128, &z, &impassable, &counts, &own, &[start], 1).unwrap();
        assert!(reopened.score_point([90., 80.], true).is_some());
    }
}
