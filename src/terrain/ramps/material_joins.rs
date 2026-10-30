//! Explicit pair joins on connected native interfaces; no global surface repaint.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::{
        binary::Reader,
        map::{BlendTiles, HeightMap, MapFile},
    },
    pipeline::project::Project,
    terrain::{self, Layout},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Vertex = [usize; 2];
type Record = [u8; 18];

#[derive(Clone)]
struct Texture {
    name: String,
    start: u32,
    count: u32,
    size: u32,
    magic: u32,
}

struct Native {
    width: usize,
    height: usize,
    tail: usize,
    descriptions: usize,
    records: Vec<Record>,
    textures: Vec<Texture>,
    tiles: Vec<u16>,
    families: Vec<usize>,
}

impl Native {
    fn parse(map: &MapFile) -> Result<Self> {
        let hm = HeightMap::parse(map.get("HeightMapData")?)?;
        let chunk = map.get("BlendTileData")?;
        if chunk.version != 27 {
            return Err(err("material joins require native BlendTileData v27"));
        }
        let bt = BlendTiles::parse(chunk, hm.width, hm.height)?;
        let area = hm
            .width
            .checked_mul(hm.height)
            .ok_or_else(|| err("material join area overflow"))?;
        let tail = 4 + 8 * area + 8 * hm.height * hm.width.div_ceil(8) + area;
        let mut reader = Reader::new(
            chunk
                .payload
                .get(tail..)
                .ok_or_else(|| err("truncated material join lookup table"))?,
        );
        let cell_count = reader.u32()?;
        let blend_count = reader.u32()?;
        reader.u32()?;
        let texture_count = reader.u32()? as usize;
        let mut textures = Vec::new();
        let mut expected_start = 0u32;
        for _ in 0..texture_count {
            let start = reader.u32()?;
            let count = reader.u32()?;
            let size = reader.u32()?;
            let magic = reader.u32()?;
            let name = reader.s16()?;
            if size == 0 || size.checked_mul(size) != Some(count) || start != expected_start {
                return Err(err("unsupported material texture phase layout"));
            }
            expected_start = expected_start
                .checked_add(count)
                .ok_or_else(|| err("texture phase range overflow"))?;
            textures.push(Texture {
                name,
                start: start
                    .checked_mul(4)
                    .ok_or_else(|| err("texture tile range overflow"))?,
                count: count
                    .checked_mul(4)
                    .ok_or_else(|| err("texture tile count overflow"))?,
                size,
                magic,
            });
        }
        if expected_start != cell_count
            || blend_count == 0
            || bt.blend_descriptions.len() % 18 != 0
            || blend_count as usize != bt.blend_descriptions.len() / 18 + 1
        {
            return Err(err("unsupported material lookup counts"));
        }
        reader.take(8)?;
        let descriptions = tail + reader.pos;
        let records = bt.blend_descriptions.as_chunks::<18>().0.to_vec();
        if records.len() > u16::MAX as usize {
            return Err(err("material blend table exceeds u16 addressing"));
        }
        let tiles = terrain::flip(&bt.tiles, hm.width);
        let families = tiles
            .iter()
            .map(|&tile| {
                textures
                    .iter()
                    .position(|texture| {
                        (u32::from(tile))
                            .checked_sub(texture.start)
                            .is_some_and(|phase| phase < texture.count)
                    })
                    .ok_or_else(|| {
                        err("material join primary tile is outside its native phase table")
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            width: hm.width,
            height: hm.height,
            tail,
            descriptions,
            records,
            textures,
            tiles,
            families,
        })
    }

    fn game(&self, index: usize) -> usize {
        (self.height - 1 - index / self.width) * self.width + index % self.width
    }
    fn word(&self, data: &[u8], plane: usize, index: usize) -> u16 {
        let offset = 4 + 2 * self.width * self.height * (plane + 1) + 2 * self.game(index);
        u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
    }
    fn ordinary_offset(&self, index: usize) -> usize {
        4 + 2 * self.width * self.height + 2 * self.game(index)
    }
    fn incident(&self, vertex: Vertex) -> impl Iterator<Item = usize> + '_ {
        [(-1, -1), (0, -1), (-1, 0), (0, 0)]
            .into_iter()
            .filter_map(move |(dx, dy)| {
                let x = vertex[0] as isize + dx;
                let y = vertex[1] as isize + dy;
                (x >= 0 && y >= 0 && x < self.width as isize && y < self.height as isize).then_some(
                    (y as usize)
                        .wrapping_mul(self.width)
                        .wrapping_add(x as usize),
                )
            })
    }
    fn record(&self, id: u16) -> Result<Record> {
        self.records
            .get(usize::from(id).wrapping_sub(1))
            .copied()
            .ok_or_else(|| err("ordinary blend references missing native description"))
    }
    fn secondary_family(&self, record: &Record) -> Option<usize> {
        let tile = u32::from_le_bytes(record[..4].try_into().unwrap());
        self.textures.iter().position(|texture| {
            tile.checked_sub(texture.start)
                .is_some_and(|phase| phase < texture.count)
        })
    }
}

#[derive(Clone)]
struct Seed {
    id: usize,
    center: [f64; 2],
    axis: [f64; 2],
    along: f64,
    across: f64,
    core_along: f64,
    core_across: f64,
}
impl Seed {
    fn contains(&self, x: f64, y: f64, core: bool) -> bool {
        let dx = x - self.center[0];
        let dy = y - self.center[1];
        let u = dx * self.axis[0] + dy * self.axis[1];
        let v = -dx * self.axis[1] + dy * self.axis[0];
        u.abs() <= if core { self.core_along } else { self.along }
            && v.abs() <= if core { self.core_across } else { self.across }
    }
}

#[derive(Clone, Copy)]
struct Edge {
    a: Vertex,
    b: Vertex,
    cells: [usize; 2],
}

fn edges(native: &Native, pair: [usize; 2]) -> (Vec<Edge>, BTreeMap<Vertex, Vec<usize>>) {
    let mut edges = Vec::new();
    let mut graph: BTreeMap<Vertex, Vec<usize>> = BTreeMap::new();
    let belongs =
        |a: usize, b: usize| (a == pair[0] && b == pair[1]) || (a == pair[1] && b == pair[0]);
    for y in 0..native.height {
        for x in 0..native.width {
            let i = y * native.width + x;
            for (other, a, b) in [
                (x + 1 < native.width, [x + 1, y], [x + 1, y + 1]),
                (y + 1 < native.height, [x, y + 1], [x + 1, y + 1]),
            ]
            .into_iter()
            .enumerate()
            .filter(|(_, entry)| entry.0)
            .map(|(direction, entry)| {
                (
                    if direction == 0 {
                        i + 1
                    } else {
                        i + native.width
                    },
                    entry.1,
                    entry.2,
                )
            }) {
                if belongs(native.families[i], native.families[other]) {
                    let index = edges.len();
                    edges.push(Edge {
                        a,
                        b,
                        cells: [i, other],
                    });
                    graph.entry(a).or_default().push(index);
                    graph.entry(b).or_default().push(index);
                }
            }
        }
    }
    (edges, graph)
}

fn alpha(direction: u8, flags: u8, two: u8, mut x: f64, mut y: f64) -> f64 {
    if flags != 0 {
        if direction == 0 {
            x = 1. - x
        } else {
            y = 1. - y
        }
    }
    if direction == 0 {
        return x;
    }
    if direction == 1 {
        return y;
    }
    if direction == 2 {
        x = 1. - x;
        y = 1. - y;
    } else {
        y = 1. - y;
    }
    let sum = x + y;
    if two != 0 {
        1. - (sum - 1.).clamp(0., 1.)
    } else {
        (1. - sum).clamp(0., 1.)
    }
}

fn encoding(mask: [u8; 4]) -> Option<(u8, u8, u8)> {
    let mut result = None;
    for direction in 0..4 {
        for flags in 0..2 {
            for two in 0..if direction < 2 { 1 } else { 2 } {
                let corners = [(0., 0.), (1., 0.), (0., 1.), (1., 1.)]
                    .map(|(x, y)| alpha(direction, flags, two, x, y).round() as u8);
                if mask == corners {
                    result = Some((direction, flags, two));
                }
            }
        }
    }
    result
}

fn descriptor(secondary: u32, mask: [u8; 4]) -> Result<Record> {
    let (direction, flags, two) = encoding(mask)
        .ok_or_else(|| err(format!("unsupported ordinary blend corner mask {mask:?}")))?;
    let mut record = [0u8; 18];
    record[..4].copy_from_slice(&secondary.to_le_bytes());
    record[4 + direction as usize] = 1;
    record[8] = flags;
    record[9] = two;
    record[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
    record[14..18].copy_from_slice(&0x7ada0000u32.to_le_bytes());
    Ok(record)
}

fn simple(record: &Record) -> bool {
    record[4..8].iter().all(|&value| value <= 1)
        && record[4..8].iter().filter(|&&value| value != 0).count() == 1
        && record[8] <= 1
        && record[9] <= 1
        && [u32::MAX, 24].contains(&u32::from_le_bytes(record[10..14].try_into().unwrap()))
        && record[14..18] == 0x7ada0000u32.to_le_bytes()
}

#[derive(Clone)]
struct Change {
    desired: Option<Record>,
    old: u16,
    owner: usize,
    secondary: usize,
    mask: [u8; 4],
    components: BTreeSet<u32>,
}

fn weights(
    native: &Native,
    payload: &[u8],
    records: &[Record],
    index: usize,
    x: f64,
    y: f64,
) -> Result<BTreeMap<usize, f64>> {
    let family = native.families[index];
    let id = native.word(payload, 0, index);
    if id == 0 {
        return Ok(BTreeMap::from([(family, 1.)]));
    }
    let record = records
        .get(usize::from(id) - 1)
        .ok_or_else(|| err("missing candidate material blend"))?;
    if !simple(record) {
        return Err(err("unsupported candidate material blend-factor model"));
    }
    let direction = record[4..8].iter().position(|&value| value != 0).unwrap() as u8;
    let factor = alpha(direction, record[8], record[9], x, y);
    let secondary = native
        .secondary_family(record)
        .ok_or_else(|| err("unknown candidate secondary material"))?;
    let mut result = BTreeMap::from([(family, 1. - factor)]);
    *result.entry(secondary).or_default() += factor;
    Ok(result)
}

/// Repair only explicitly named material interfaces seeded by the selected
/// ramp scopes. Connected interface components end naturally at third material
/// junctions; existing foreign terminal blends remain separately reported.
pub fn repair_material_joins(
    p: &Project,
    map: &mut MapFile,
    layout: &Layout,
    ramp_ids: &[usize],
    shoulder_cells: usize,
    texture_pairs: &[[String; 2]],
) -> Result<Value> {
    repair_material_joins_seeded(p, map, layout, ramp_ids, shoulder_cells, texture_pairs, &[])
}

/// Restrict the repair to reviewed interfaces identified by their exact shared
/// grid-edge vertices. Primary tiles retain these seeds after blend edits.
pub fn repair_material_joins_seeded(
    p: &Project,
    map: &mut MapFile,
    layout: &Layout,
    ramp_ids: &[usize],
    shoulder_cells: usize,
    texture_pairs: &[[String; 2]],
    seed_edges: &[[[usize; 2]; 2]],
) -> Result<Value> {
    let mut candidate = map.clone();
    let mut report = repair_inner(
        p,
        &mut candidate,
        layout,
        ramp_ids,
        shoulder_cells,
        texture_pairs,
        seed_edges,
    )?;
    report["reviewed_seed_edges"] = json!(seed_edges);
    *map = candidate;
    Ok(report)
}

fn repair_inner(
    p: &Project,
    map: &mut MapFile,
    layout: &Layout,
    ramp_ids: &[usize],
    shoulder_cells: usize,
    texture_pairs: &[[String; 2]],
    seed_edges: &[[[usize; 2]; 2]],
) -> Result<Value> {
    let native = Native::parse(map)?;
    let mut required_edges = BTreeSet::new();
    for &[a, b] in seed_edges {
        if [a, b]
            .iter()
            .any(|p| p[0] > native.width || p[1] > native.height)
            || a[0].abs_diff(b[0]) + a[1].abs_diff(b[1]) != 1
            || !required_edges.insert(if a < b { [a, b] } else { [b, a] })
        {
            return Err(err(
                "reviewed material seeds require unique adjacent grid-edge vertices",
            ));
        }
    }
    let mut matched_edges = BTreeSet::new();
    if shoulder_cells > native.width.min(native.height) {
        return Err(err("material join shoulder exceeds native dimensions"));
    }
    if ramp_ids.is_empty()
        || ramp_ids.iter().copied().collect::<BTreeSet<_>>().len() != ramp_ids.len()
    {
        return Err(err("material joins require unique explicit ramp IDs"));
    }
    if texture_pairs.is_empty() {
        return Err(err("material joins require explicit ordered texture pairs"));
    }
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let mut seeds = Vec::new();
    for &id in ramp_ids {
        let r = layout
            .ramps
            .get(id.wrapping_sub(1))
            .ok_or_else(|| err(format!("unknown material join ramp {id}")))?;
        let (ux, uy, _, _) = terrain::ramp_axes(r)?;
        let center = [
            terrain::number(r, "x", f64::NAN),
            terrain::number(r, "y", f64::NAN),
        ];
        let span = terrain::number(r, "span", 25.);
        let width = terrain::ramp_width(r)?;
        if !span.is_finite()
            || span <= 0.
            || center.iter().any(|v| !v.is_finite())
            || ux.hypot(uy) < 0.99
        {
            return Err(err("invalid material join ramp geometry"));
        }
        let seed = Seed {
            id,
            center,
            axis: [ux, uy],
            along: span * 0.5 + shoulder_cells as f64,
            across: width * 0.5 + shoulder_cells as f64,
            core_along: span * 0.5,
            core_across: width * 0.5,
        };
        for (a, b) in [(-1., -1.), (-1., 1.), (1., -1.), (1., 1.)] {
            let point = [
                center[0] + a * seed.along * ux - b * seed.across * uy,
                center[1] + a * seed.along * uy + b * seed.across * ux,
            ];
            if !frame.playable(point) {
                return Err(err(
                    "material join seed scope exceeds playable native bounds",
                ));
            }
        }
        seeds.push(seed);
    }
    let source = map.get("BlendTileData")?.payload.clone();
    let mut pairs = Vec::new();
    let mut unique = BTreeSet::new();
    let mut unordered_pairs = BTreeSet::new();
    for pair in texture_pairs {
        let indices = pair
            .iter()
            .map(|name| {
                let matches = native
                    .textures
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.name == *name)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(err(format!(
                        "unknown or ambiguous material join texture {name}"
                    )));
                }
                Ok(matches[0])
            })
            .collect::<Result<Vec<_>>>()?;
        let pair = [indices[0], indices[1]];
        if pair[0] == pair[1]
            || !unique.insert(pair)
            || !unordered_pairs.insert([pair[0].min(pair[1]), pair[0].max(pair[1])])
        {
            return Err(err(
                "material join texture pairs must be distinct and unique",
            ));
        }
        let owner = &native.textures[pair[0]];
        let target = &native.textures[pair[1]];
        if owner.size != target.size
            || ![2, 4].contains(&owner.size)
            || owner.magic != 0
            || target.magic != 0
        {
            return Err(err(
                "material join pair has an unsupported or mismatched native phase shape",
            ));
        }
        pairs.push(pair);
    }
    let mut changes: BTreeMap<usize, Change> = BTreeMap::new();
    let mut components = Vec::new();
    let mut terminals: BTreeMap<(usize, usize, usize), Value> = BTreeMap::new();
    let mut deferred_foreign = Vec::new();
    let mut matched_ramps = BTreeSet::new();
    let mut edge_checks = Vec::new();
    for pair in pairs {
        let component_start = components.len();
        let (edges, graph) = edges(&native, pair);
        let mut visited = BTreeSet::new();
        for (seed_edge, edge) in edges.iter().enumerate() {
            let edge_point = [
                (edge.cells[0] % native.width + edge.cells[1] % native.width) as f64 * 0.5,
                (edge.cells[0] / native.width + edge.cells[1] / native.width) as f64 * 0.5,
            ];
            if visited.contains(&seed_edge)
                || !seeds
                    .iter()
                    .any(|seed| seed.contains(edge_point[0], edge_point[1], false))
            {
                continue;
            }
            let mut queue = VecDeque::from([seed_edge]);
            let mut component = BTreeSet::new();
            let mut vertices = BTreeSet::new();
            let mut edge_cells = BTreeSet::new();
            while let Some(index) = queue.pop_front() {
                if !component.insert(index) {
                    continue;
                }
                visited.insert(index);
                let edge = edges[index];
                edge_cells.extend(edge.cells);
                for vertex in [edge.a, edge.b] {
                    vertices.insert(vertex);
                    for &neighbor in &graph[&vertex] {
                        if !component.contains(&neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
            if !required_edges.is_empty() {
                let found = component
                    .iter()
                    .filter_map(|&index| {
                        let edge = edges[index];
                        let key = if edge.a < edge.b {
                            [edge.a, edge.b]
                        } else {
                            [edge.b, edge.a]
                        };
                        required_edges.contains(&key).then_some(key)
                    })
                    .collect::<BTreeSet<_>>();
                if found.is_empty() {
                    continue;
                }
                matched_edges.extend(found);
            }
            let mut canonical = Vec::new();
            let mut edge_rows = Vec::new();
            for &index in &component {
                let edge = edges[index];
                for vertex in [edge.a, edge.b] {
                    for value in vertex {
                        canonical.extend_from_slice(&(value as u32).to_le_bytes());
                    }
                }
                edge_rows.push([edge.a, edge.b]);
            }
            let component_crc = crc32fast::hash(&canonical);
            edge_checks.extend(
                component
                    .iter()
                    .map(|&index| (component_crc, pair, edges[index])),
            );
            let incident = vertices
                .iter()
                .flat_map(|&v| native.incident(v))
                .collect::<BTreeSet<_>>();
            let endpoints = vertices
                .iter()
                .filter(|v| graph[*v].len() == 1)
                .copied()
                .collect::<BTreeSet<_>>();
            let touched_ramps = seeds
                .iter()
                .filter(|seed| {
                    component.iter().any(|&i| {
                        let edge = edges[i];
                        let a = edge.cells[0];
                        let b = edge.cells[1];
                        seed.contains(
                            (a % native.width + b % native.width) as f64 * 0.5,
                            (a / native.width + b / native.width) as f64 * 0.5,
                            false,
                        )
                    })
                })
                .map(|s| s.id)
                .collect::<Vec<_>>();
            matched_ramps.extend(touched_ramps.iter().copied());
            components.push(json!({"edge_crc32":component_crc,"texture_pair":texture_pairs.iter().find(|p|p[0]==native.textures[pair[0]].name&&p[1]==native.textures[pair[1]].name),"family_pair":pair,"ramp_ids":touched_ramps,"edge_count":component.len(),"edges":edge_rows,"endpoint_vertices":endpoints,"corner_halo_cells":incident.len()}));
            for index in incident {
                let family = native.families[index];
                if !pair.contains(&family) {
                    continue;
                }
                let x = index % native.width;
                let y = index / native.width;
                let old = native.word(&source, 0, index);
                let old_record = if old == 0 {
                    None
                } else {
                    Some(native.record(old)?)
                };
                let adjacent = edge_cells.contains(&index);
                let target = if family == pair[0] { pair[1] } else { pair[0] };
                let corners = [[x, y + 1], [x + 1, y + 1], [x, y], [x + 1, y]];
                let mask = corners.map(|vertex| {
                    u8::from(
                        native
                            .incident(vertex)
                            .any(|cell| native.families[cell] == target),
                    )
                });
                let relevant = mask.iter().any(|&v| v != 0)
                    || old_record
                        .as_ref()
                        .is_some_and(|record| native.secondary_family(record) == Some(target));
                if !relevant {
                    continue;
                }
                let foreign = old_record
                    .as_ref()
                    .is_some_and(|record| native.secondary_family(record) != Some(target));
                let terminal = !seeds
                    .iter()
                    .any(|seed| seed.contains(x as f64, y as f64, true))
                    && corners.iter().any(|vertex| endpoints.contains(vertex));
                if native.word(&source, 1, index) != 0 || native.word(&source, 2, index) != 0 {
                    return Err(err(format!(
                        "material join cell [{x},{y}] has a protected second/cliff blend"
                    )));
                }
                let phase = u32::from(native.tiles[index]) - native.textures[family].start;
                if foreign {
                    let record = old_record.as_ref().unwrap();
                    let other = native.secondary_family(record);
                    let standard = other.is_some_and(|other| {
                        simple(record)
                            && native.textures[other].size == native.textures[family].size
                            && u32::from_le_bytes(record[..4].try_into().unwrap())
                                - native.textures[other].start
                                == phase
                    });
                    let row = json!({"cell":[x,y],"owner_family":family,"declared_pair":pair,"existing_ordinary":old,"existing_secondary_family":other,"component_edge_crc32":component_crc,"actual_pair_boundary_cell":adjacent,"preserved":true,"reason":"foreign ordinary material at an existing non-core degree-one endpoint; not an authorized same-pair rewrite","requires_independent_surface_review":true});
                    deferred_foreign.push((
                        index,
                        pair,
                        family == pair[1],
                        terminal && standard,
                        row,
                    ));
                    continue;
                }
                let desired = if family == pair[0] && mask.iter().any(|&v| v != 0) {
                    Some(descriptor(native.textures[target].start + phase, mask)?)
                } else {
                    None
                };
                if let Some(record) = old_record
                    && desired != Some(record)
                {
                    let actual_phase = u32::from_le_bytes(record[..4].try_into().unwrap())
                        .checked_sub(native.textures[target].start);
                    if foreign || !simple(&record) || actual_phase != Some(phase) {
                        return Err(err(format!(
                            "material join cell [{x},{y}] has an unsupported existing ordinary blend for declared pair {:?}",
                            pair
                        )));
                    }
                }
                if old == 0 && desired.is_none() {
                    continue;
                }
                let change = Change {
                    desired,
                    old,
                    owner: pair[0],
                    secondary: pair[1],
                    mask,
                    components: BTreeSet::from([component_crc]),
                };
                if let Some(previous) = changes.get_mut(&index) {
                    if previous.desired != desired {
                        return Err(err(format!(
                            "two declared material pairs conflict at cell [{x},{y}]"
                        )));
                    }
                    previous.components.insert(component_crc);
                } else {
                    changes.insert(index, change);
                }
            }
        }
        if components.len() == component_start {
            return Err(err(
                "a declared material join pair has no seeded interface component",
            ));
        }
    }
    if matched_ramps.len() != seeds.len() {
        return Err(err(
            "a declared material join ramp has no seeded interface component",
        ));
    }
    if matched_edges != required_edges {
        return Err(err(
            "a reviewed material seed does not match a declared pair inside the selected ramp scope",
        ));
    }
    for (index, pair, target_side, allowed, row) in deferred_foreign {
        if let Some(change) = changes.get(&index) {
            if target_side && change.desired.is_none() {
                continue;
            }
            if change.desired != Some(native.record(native.word(&source, 0, index))?) {
                return Err(err(
                    "declared material pairs conflict with a preserved terminal junction",
                ));
            }
        }
        if !allowed {
            return Err(err(format!(
                "material join cell [{},{}] has a foreign or complex non-terminal ordinary blend",
                index % native.width,
                index / native.width
            )));
        }
        terminals.insert((index, pair[0], pair[1]), row);
    }
    let mut records = native.records.clone();
    let mut cache = BTreeMap::new();
    for (i, record) in records.iter().enumerate() {
        cache.entry(*record).or_insert(u16::try_from(i + 1)?);
    }
    let mut payload = source.clone();
    let mut rows = Vec::new();
    let mut modified = Vec::new();
    for (&index, change) in &changes {
        let id = if let Some(record) = change.desired {
            if change.old != 0 && native.record(change.old)? == record {
                change.old
            } else if let Some(&id) = cache.get(&record) {
                id
            } else {
                let id = u16::try_from(records.len() + 1)
                    .map_err(|_| err("material joins exceed u16 ordinary blend table"))?;
                records.push(record);
                cache.insert(record, id);
                id
            }
        } else {
            0
        };
        let offset = native.ordinary_offset(index);
        if id != change.old {
            if !frame.playable([(index % native.width) as f64, (index / native.width) as f64]) {
                return Err(err(
                    "connected material interface would modify a cell outside playable bounds",
                ));
            }
            payload[offset..offset + 2].copy_from_slice(&id.to_le_bytes());
            modified.push((offset, change.old));
        }
        rows.push(json!({"cell":[index%native.width,index/native.width],"native_index":native.game(index),"ordinary_plane_byte_offset":offset,"base_tile":native.tiles[index],"phase":u32::from(native.tiles[index])-native.textures[native.families[index]].start,"family":native.families[index],"declared_pair":[change.owner,change.secondary],"before_ordinary":change.old,"new_blend_index":id,"changed":id!=change.old,"alpha_corners_SW_SE_NW_NE":change.mask,"record_hex":change.desired.map(|record|record.iter().map(|byte|format!("{byte:02x}")).collect::<String>()),"component_edge_crcs":change.components}));
    }
    let appended = records[native.records.len()..]
        .iter()
        .flat_map(|record| record.iter().copied())
        .collect::<Vec<_>>();
    let old_end = native.descriptions + native.records.len() * 18;
    payload.splice(old_end..old_end, appended.iter().copied());
    payload[native.tail + 4..native.tail + 8]
        .copy_from_slice(&u32::try_from(records.len() + 1)?.to_le_bytes());
    let mut restored = payload.clone();
    restored.drain(old_end..old_end + appended.len());
    restored[native.tail + 4..native.tail + 8]
        .copy_from_slice(&source[native.tail + 4..native.tail + 8]);
    for &(offset, old) in &modified {
        restored[offset..offset + 2].copy_from_slice(&old.to_le_bytes());
    }
    if restored != source {
        return Err(err("material join preservation proof failed"));
    }
    let before_crc = crc32fast::hash(&source);
    let after_crc = crc32fast::hash(&payload);
    let terminal_cells = terminals
        .keys()
        .map(|&(index, _, _)| index)
        .collect::<BTreeSet<_>>();
    let mut terminal_edges = Vec::new();
    let mut pair_samples = 0usize;
    let mut terminal_samples = 0usize;
    for (component, pair, edge) in edge_checks {
        let [a, b] = edge.cells;
        let horizontal = b == a + 1;
        let mut failures = 0;
        let mut example = None;
        for sample in 0..=32 {
            let t = sample as f64 / 32.;
            let wa = weights(
                &native,
                &payload,
                &records,
                a,
                if horizontal { 1. } else { t },
                if horizontal { t } else { 0. },
            )?;
            let wb = weights(
                &native,
                &payload,
                &records,
                b,
                if horizontal { 0. } else { t },
                if horizontal { t } else { 1. },
            )?;
            let families = wa.keys().chain(wb.keys()).copied().collect::<BTreeSet<_>>();
            let delta = families
                .iter()
                .map(|family| {
                    (wa.get(family).copied().unwrap_or(0.) - wb.get(family).copied().unwrap_or(0.))
                        .abs()
                })
                .fold(0f64, f64::max);
            pair_samples += 1;
            if delta > 1e-9 {
                failures += 1;
                if example.is_none() {
                    example = Some(
                        json!({"fraction":t,"weights_a":wa,"weights_b":wb,"maximum_delta":delta}),
                    );
                }
            }
        }
        if failures > 0 {
            if !edge
                .cells
                .iter()
                .any(|index| terminal_cells.contains(index))
            {
                return Err(err(
                    "material join created a non-terminal interface weight discontinuity",
                ));
            }
            terminal_samples += failures;
            terminal_edges.push(json!({"component_edge_crc32":component,"declared_pair":pair,"edge_cells":[[a%native.width,a/native.width],[b%native.width,b/native.width]],"discontinuous_samples":failures,"sample_count":33,"example":example,"reason":"existing third-material endpoint weights preserved; requires independent diffuse review"}));
        }
    }
    map.get_mut("BlendTileData")?.payload = payload;
    BlendTiles::parse(map.get("BlendTileData")?, native.width, native.height)?;
    let all_continuous = terminals.is_empty() && terminal_edges.is_empty();
    Ok(
        json!({"schema_version":1,"operation":"repair-ramp-material-joins","ramp_ids":ramp_ids,"shoulder_cells":shoulder_cells,"texture_pairs":texture_pairs,"scope":"explicit ordered pairs; ramp core plus declared shoulder seeds connected interface components; component corner halo only","source_blend_chunk_crc32":before_crc,"candidate_blend_chunk_crc32":after_crc,"components":components,"cells":rows,"changed_cells":modified.len(),"original_blend_count":native.records.len(),"new_blend_records":records.len()-native.records.len(),"new_blend_count":records.len(),"terminal_material_junctions":terminals.into_values().collect::<Vec<_>>(),"terminal_edge_joins":terminal_edges,"pair_edge_alpha_samples":pair_samples,"terminal_edge_discontinuous_samples":terminal_samples,"all_component_edges_claimed_continuous":all_continuous,"preserved":{"primary_tiles_and_subphases":true,"second_blend_plane":true,"cliff_plane_and_descriptors":true,"all_gameplay_masks_and_unknown_cell_bytes":true,"existing_description_prefix":true,"texture_table_and_unknown_bytes":true,"unrelated_native_chunks":true},"blend_factor_model":"OpenSAGE Terrain.frag ordinary direction/flags/TwoSided corner model; not an engine frame rendering"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{binary::Writer, map::Chunk};

    fn fixture() -> (Project, MapFile, Layout) {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let (width, height) = (96usize, 64usize);
        p.cfg["GRID"] = json!(width);
        p.cfg["GRID_HEIGHT"] = json!(height);
        p.cfg["BORDER"] = json!(4);
        let hm = HeightMap {
            width,
            height,
            border: 4,
            borders: Vec::new(),
            elevations: vec![5376; width * height],
        };
        let area = width * height;
        let tail = 4 + 8 * area + 8 * height * width.div_ceil(8) + area;
        let mut data = vec![0xa5; tail];
        data[..4].copy_from_slice(&(area as u32).to_le_bytes());
        for y in 0..height {
            for x in 0..width {
                let family =
                    if (12..52).contains(&y) && ((8..40).contains(&x) || (56..88).contains(&x)) {
                        let cut = if x < 48 { 24 } else { 72 } + (y - 12) / 6;
                        usize::from(x >= cut)
                    } else {
                        2
                    };
                let phase = (x % 8) + 8 * (y % 8);
                let tile = (family * 64 + phase) as u16;
                let game = (height - 1 - y) * width + x;
                data[4 + 2 * game..6 + 2 * game].copy_from_slice(&tile.to_le_bytes());
                for plane in 0..3 {
                    let offset = 4 + 2 * area * (plane + 1) + 2 * game;
                    data[offset..offset + 2].fill(0);
                }
            }
        }
        let mut lookup = Writer::new();
        lookup.u32(48);
        lookup.u32(2);
        lookup.u32(2);
        lookup.u32(3);
        for (index, name) in ["Owner", "Target", "Third"].iter().enumerate() {
            lookup.u32(index as u32 * 16);
            lookup.u32(16);
            lookup.u32(4);
            lookup.u32(0);
            lookup.s16(name).unwrap();
        }
        lookup.u32(0x12345678);
        lookup.u32(0);
        lookup.bytes(&descriptor(64 + 5, [0, 1, 0, 1]).unwrap());
        lookup.bytes(&[0x7b; 38]);
        data.extend_from_slice(&lookup.data);
        let order = vec![
            ("HeightMapData".to_owned(), 1),
            ("BlendTileData".to_owned(), 2),
            ("untouched".to_owned(), 3),
        ];
        let map = MapFile {
            table: order
                .iter()
                .map(|(name, index)| (*index, name.clone()))
                .collect(),
            order,
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 27,
                    payload: data,
                },
                Chunk {
                    name_idx: 3,
                    version: 19,
                    payload: vec![17, 91, 14, 255],
                },
            ],
        };
        let layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[],
            "ramps":[{"x":24.,"y":32.,"ux":1.,"uy":0.,"span":20.,"width":20.,"pair":[210.,280.]},
                      {"x":72.,"y":32.,"ux":1.,"uy":0.,"span":20.,"width":20.,"pair":[210.,280.]}]})).unwrap();
        (p, map, layout)
    }

    fn append(map: &mut MapFile, record: Record) -> u16 {
        let native = Native::parse(map).unwrap();
        let end = native.descriptions + native.records.len() * 18;
        let data = &mut map.get_mut("BlendTileData").unwrap().payload;
        data.splice(end..end, record);
        let id = native.records.len() + 1;
        data[native.tail + 4..native.tail + 8].copy_from_slice(&(id as u32 + 1).to_le_bytes());
        id as u16
    }
    fn set_word(map: &mut MapFile, plane: usize, point: [usize; 2], id: u16) {
        let native = Native::parse(map).unwrap();
        let index = point[1] * native.width + point[0];
        let offset = 4 + 2 * native.width * native.height * (plane + 1) + 2 * native.game(index);
        map.get_mut("BlendTileData").unwrap().payload[offset..offset + 2]
            .copy_from_slice(&id.to_le_bytes());
    }
    fn pair() -> Vec<[String; 2]> {
        vec![["Owner".into(), "Target".into()]]
    }

    #[test]
    fn rectangular_stair_join_is_continuous_phase_preserving_and_idempotent() {
        let (p, mut map, layout) = fixture();
        // Both sides used to point toward each other. Normalize the owner's
        // known pair record and clear only the target's reverse pair record.
        let native = Native::parse(&map).unwrap();
        let owner = [26, 31];
        let target = [27, 31];
        let a = owner[1] * native.width + owner[0];
        let b = target[1] * native.width + target[0];
        let old_owner = append(
            &mut map,
            descriptor(64 + u32::from(native.tiles[a]), [1, 0, 1, 0]).unwrap(),
        );
        set_word(&mut map, 0, owner, old_owner);
        let reverse = append(
            &mut map,
            descriptor(u32::from(native.tiles[b]) - 64, [0, 1, 0, 1]).unwrap(),
        );
        set_word(&mut map, 0, target, reverse);
        let before = map.clone();
        let old = Native::parse(&map).unwrap();
        let old_data = map.get("BlendTileData").unwrap().payload.clone();
        let report = repair_material_joins(&p, &mut map, &layout, &[1], 0, &pair()).unwrap();
        assert!(report["changed_cells"].as_u64().unwrap() > 30);
        assert_eq!(report["terminal_edge_discontinuous_samples"], 0);
        assert_eq!(report["all_component_edges_claimed_continuous"], true);
        assert!(report["cells"].as_array().unwrap().iter().any(|cell| {
            cell["record_hex"]
                .as_str()
                .is_some_and(|hex| &hex[8..16] != "01000000")
        }));
        let after = Native::parse(&map).unwrap();
        let data = &map.get("BlendTileData").unwrap().payload;
        assert_eq!(old.tiles, after.tiles);
        assert_eq!(
            &old_data[..4 + 2 * old.width * old.height],
            &data[..4 + 2 * old.width * old.height]
        );
        assert_eq!(
            &old_data[4 + 4 * old.width * old.height..old.tail],
            &data[4 + 4 * old.width * old.height..after.tail]
        );
        assert_eq!(
            &old_data[old.descriptions..old.descriptions + old.records.len() * 18],
            &data[after.descriptions..after.descriptions + old.records.len() * 18]
        );
        assert_eq!(
            &old_data[old.descriptions + old.records.len() * 18..],
            &data[after.descriptions + after.records.len() * 18..]
        );
        assert_eq!(
            map.get("HeightMapData").unwrap(),
            before.get("HeightMapData").unwrap()
        );
        assert_eq!(
            map.get("untouched").unwrap(),
            before.get("untouched").unwrap()
        );
        assert_eq!(after.word(data, 0, 31 * after.width + 27), 0);
        // The disconnected interface belonging to the other ramp is untouched.
        assert_eq!(after.word(data, 0, 31 * after.width + 74), 0);
        let bytes = map.encode().unwrap();
        let again = repair_material_joins(&p, &mut map, &layout, &[1], 0, &pair()).unwrap();
        assert_eq!(again["changed_cells"], 0);
        assert_eq!(again["new_blend_records"], 0);
        assert_eq!(map.encode().unwrap(), bytes);
    }

    #[test]
    fn invalid_scope_and_protected_complex_planes_fail_atomically() {
        let (p, map, layout) = fixture();
        for (ids, shoulder, pairs) in [
            (vec![99], 0, pair()),
            (vec![1], usize::MAX, pair()),
            (vec![1], 0, vec![["Owner".into(), "missing".into()]]),
            (
                vec![1],
                0,
                vec![
                    ["Owner".into(), "Target".into()],
                    ["Target".into(), "Owner".into()],
                ],
            ),
        ] {
            let mut candidate = map.clone();
            let before = candidate.encode().unwrap();
            assert!(
                repair_material_joins(&p, &mut candidate, &layout, &ids, shoulder, &pairs).is_err()
            );
            assert_eq!(candidate.encode().unwrap(), before);
        }
        for plane in [1, 2] {
            let mut candidate = map.clone();
            set_word(&mut candidate, plane, [26, 31], 1);
            let before = candidate.encode().unwrap();
            assert!(repair_material_joins(&p, &mut candidate, &layout, &[1], 0, &pair()).is_err());
            assert_eq!(candidate.encode().unwrap(), before);
        }
        let mut candidate = map.clone();
        let mut bad = descriptor(64 + 58, [0, 1, 0, 1]).unwrap();
        bad[5] = 1;
        let index = append(&mut candidate, bad);
        set_word(&mut candidate, 0, [26, 31], index);
        let before = candidate.encode().unwrap();
        assert!(repair_material_joins(&p, &mut candidate, &layout, &[1], 0, &pair()).is_err());
        assert_eq!(candidate.encode().unwrap(), before);
    }

    #[test]
    fn connected_interface_cannot_escape_playable_border() {
        let (p, mut map, layout) = fixture();
        let native = Native::parse(&map).unwrap();
        // Extend the first interface through the border: a valid seed is not
        // permission to write disabled border pixels farther along the seam.
        for y in 0..12 {
            for x in 8..40 {
                let tile = if x < 24 { 5u16 } else { 69u16 };
                let index = (native.height - 1 - y) * native.width + x;
                map.get_mut("BlendTileData").unwrap().payload[4 + 2 * index..6 + 2 * index]
                    .copy_from_slice(&tile.to_le_bytes());
            }
        }
        let before = map.encode().unwrap();
        assert!(repair_material_joins(&p, &mut map, &layout, &[1], 0, &pair()).is_err());
        assert_eq!(map.encode().unwrap(), before);
    }

    #[test]
    fn reviewed_seeds_preserve_an_unselected_complex_interface_in_the_same_scope() {
        let (p, mut map, mut layout) = fixture();
        layout.ramps[0]["x"] = json!(48.);
        layout.ramps[0]["span"] = json!(64.);
        set_word(&mut map, 1, [74, 31], 1);
        let selected = [[[27, 31], [27, 32]]];
        let report =
            repair_material_joins_seeded(&p, &mut map, &layout, &[1], 0, &pair(), &selected)
                .unwrap();
        assert_eq!(report["components"].as_array().unwrap().len(), 1);
        let native = Native::parse(&map).unwrap();
        let data = &map.get("BlendTileData").unwrap().payload;
        assert_eq!(native.word(data, 1, 31 * native.width + 74), 1);
        assert_eq!(native.word(data, 0, 31 * native.width + 74), 0);
        let before = map.encode().unwrap();
        let again =
            repair_material_joins_seeded(&p, &mut map, &layout, &[1], 0, &pair(), &selected)
                .unwrap();
        assert_eq!(again["changed_cells"], 0);
        assert_eq!(map.encode().unwrap(), before);
        assert!(
            repair_material_joins_seeded(
                &p,
                &mut map,
                &layout,
                &[1],
                0,
                &pair(),
                &[[[10, 30], [10, 31]]],
            )
            .is_err()
        );
        assert_eq!(map.encode().unwrap(), before);
    }
}
