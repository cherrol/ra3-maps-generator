//! Source-bound, explicit native edits. Coordinates and masks use north-up cells.
use crate::{
    Result, err,
    formats::{
        archive,
        binary::Reader,
        map::{BlendTiles, HeightMap, MapFile},
        npy::Npy,
        sha256,
    },
    pipeline::project::Project,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    x: usize,
    y: usize,
    before_raw: u16,
    raw: u16,
    impassable: bool,
    build_blocked: bool,
    tile: Option<u16>,
}
/// Every file is map-relative, pinned by SHA-256; an optional ZIP member is exact.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: String,
    sha256: String,
    #[serde(default)]
    member: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Design {
    path: String,
    sha256: String,
    layer_key: String,
    water_key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Slope {
    id: String,
    layer_pair: [u16; 2],
    raw_range: [u16; 2],
    mask: Input,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Materials {
    mask: Input,
    #[serde(default)]
    clear_blends: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    schema_version: u32,
    source: Input,
    design: Design,
    allowed_cells: Input,
    #[serde(default)]
    slopes: Vec<Slope>,
    #[serde(default)]
    materials: Option<Materials>,
}

fn read_file(p: &Project, path: &str, expected: &str) -> Result<Vec<u8>> {
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err(format!("native edit scope needs a SHA-256 for {path}")));
    }
    let file = p.path(path)?;
    if !file
        .canonicalize()?
        .starts_with(p.map_dir().canonicalize()?)
    {
        return Err(err(format!(
            "native edit scope input escapes map directory: {path}"
        )));
    }
    let bytes = fs::read(file)?;
    if !sha256::hex(&bytes).eq_ignore_ascii_case(expected) {
        return Err(err(format!(
            "native edit scope source identity mismatch: {path}"
        )));
    }
    Ok(bytes)
}
fn read_input(p: &Project, input: &Input) -> Result<Vec<u8>> {
    let bytes = read_file(p, &input.path, &input.sha256)?;
    match &input.member {
        None => Ok(bytes),
        Some(member) => archive::unzip(&bytes)?.remove(member).ok_or_else(|| {
            err(format!(
                "native edit scope missing member {}#{member}",
                input.path
            ))
        }),
    }
}
fn unsigned_array(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u16>> {
    let a = Npy::parse(bytes)?;
    if a.shape != [height, width] || !matches!(a.dtype.as_str(), "|u1" | "|b1" | "<u2" | "=u2") {
        return Err(err(
            "native edit scope masks must be full-grid unsigned NPY arrays",
        ));
    }
    Ok(a.numbers()?.into_iter().map(|v| v as u16).collect())
}
fn bool_array(bytes: &[u8], width: usize, height: usize) -> Result<Vec<bool>> {
    unsigned_array(bytes, width, height)?
        .into_iter()
        .map(|v| match v {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(err(
                "native edit scope boolean mask contains values other than 0/1",
            )),
        })
        .collect()
}
fn bitmap_offsets(
    chunk: &crate::formats::map::Chunk,
    w: usize,
    h: usize,
) -> Result<(usize, usize, usize)> {
    let v = chunk.version;
    if v < 24 {
        return Err(err("native cell patch requires buildability planes"));
    }
    let mut reader = Reader::new(&chunk.payload);
    let n = reader.u32()? as usize;
    if w.checked_mul(h) != Some(n) {
        return Err(err("native cell patch dimensions mismatch"));
    }
    let plane = if (14..24).contains(&v) { 4 } else { 2 };
    reader.take(n * (2 + plane * 3))?;
    let bytes = h * w.div_ceil(8);
    let imp = reader.pos;
    reader.take(bytes * 3)?;
    if (14..25).contains(&v) {
        reader.take(bytes)?;
    }
    if v >= 15 {
        reader.take(bytes)?;
    }
    if (16..25).contains(&v) {
        reader.take(n)?;
    }
    if v >= 17 {
        reader.take(bytes)?;
    }
    let build = reader.pos;
    reader.take(bytes)?;
    Ok((imp, build, plane))
}
fn set_bit(payload: &mut [u8], offset: usize, index: usize, width: usize, value: bool) {
    let byte = offset + (index / width) * width.div_ceil(8) + (index % width) / 8;
    let mask = 1 << (index % width % 8);
    if value {
        payload[byte] |= mask;
    } else {
        payload[byte] &= !mask;
    }
}

pub fn apply(p: &Project, m: &mut MapFile) -> Result<()> {
    let Some(value) = p.cfg.get("NATIVE_CELL_EDITS") else {
        return Ok(());
    };
    let path = value
        .as_str()
        .ok_or_else(|| err("NATIVE_CELL_EDITS must be a map-relative file path"))?;
    let scope_path = p.cfg.get("NATIVE_CELL_EDIT_SCOPE").and_then(|v| v.as_str()).ok_or_else(||
        err("NATIVE_CELL_EDIT_SCOPE is required: legacy unscoped native edits are not applied; declare the pinned source, design masks and authorized slope/material cells"))?;
    let scope: Scope = serde_json::from_slice(&fs::read(p.path(scope_path)?)?)?;
    if scope.schema_version != 1 {
        return Err(err("unsupported NATIVE_CELL_EDIT_SCOPE schema"));
    }
    let cells: Vec<Cell> = serde_json::from_slice(&fs::read(p.path(path)?)?)?;
    let source = MapFile::load_bytes(&read_input(p, &scope.source)?)?;
    // Coordinate projection may already have moved declared objects/areas, but
    // it must not silently swap the source scripts, water or other native chunks.
    if m.table != source.table || m.chunks.len() != source.chunks.len() {
        return Err(err(
            "native edit candidate source identity/table differs from pinned source",
        ));
    }
    for (original, current) in source.chunks.iter().zip(&m.chunks) {
        let name = source
            .table
            .get(&original.name_idx)
            .map(String::as_str)
            .unwrap_or("");
        if original.name_idx != current.name_idx
            || (!matches!(
                name,
                "HeightMapData" | "BlendTileData" | "ObjectsList" | "TriggerAreas"
            ) && original != current)
        {
            return Err(err(format!(
                "native edit candidate source identity differs in {name}"
            )));
        }
    }
    let mut hm = HeightMap::parse(source.get("HeightMapData")?)?;
    let (w, h) = (hm.width, hm.height);
    let current_hm = HeightMap::parse(m.get("HeightMapData")?)?;
    if (w, h, hm.border) != (current_hm.width, current_hm.height, current_hm.border) {
        return Err(err("native cell scope source frame differs from candidate"));
    }
    let chunk = source.get("BlendTileData")?;
    let bt = BlendTiles::parse(chunk, w, h)?;
    let (imp, build, plane) = bitmap_offsets(chunk, w, h)?;
    let n = w * h;
    if cells.len() > n / 10 {
        return Err(err("native cell patch extent exceeds local edit limit"));
    }
    let design_files = archive::unzip(&read_file(p, &scope.design.path, &scope.design.sha256)?)?;
    let layers = unsigned_array(
        design_files
            .get(&scope.design.layer_key)
            .ok_or_else(|| err("native edit scope design layer array is missing"))?,
        w,
        h,
    )?;
    let water = bool_array(
        design_files
            .get(&scope.design.water_key)
            .ok_or_else(|| err("native edit scope design water array is missing"))?,
        w,
        h,
    )?;
    let allowed = bool_array(&read_input(p, &scope.allowed_cells)?, w, h)?;
    let mut slopes: BTreeMap<usize, Vec<[u16; 2]>> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for slope in scope.slopes {
        if slope.id.trim().is_empty()
            || !ids.insert(slope.id.clone())
            || slope.layer_pair[0] == 0
            || slope.layer_pair[1] == 0
            || slope.layer_pair[0] == slope.layer_pair[1]
            || slope.raw_range[0] >= slope.raw_range[1]
        {
            return Err(err(
                "native edit scope slope IDs, layer pairs and raw ranges must be explicit and distinct",
            ));
        }
        let mask = bool_array(&read_input(p, &slope.mask)?, w, h)?;
        if !mask.iter().any(|v| *v) {
            return Err(err(format!(
                "native edit slope {} has an empty mask",
                slope.id
            )));
        }
        for (i, yes) in mask.into_iter().enumerate().filter(|(_, v)| *v) {
            if !yes || !allowed[i] || water[i] || !slope.layer_pair.contains(&layers[i]) {
                return Err(err(format!(
                    "native edit slope {} crosses its allowed design layers at {},{}",
                    slope.id,
                    i % w,
                    i / w
                )));
            }
            slopes.entry(i).or_default().push(slope.raw_range);
        }
    }
    let materials = scope
        .materials
        .map(|v| {
            let mask = bool_array(&read_input(p, &v.mask)?, w, h)?;
            if mask
                .iter()
                .zip(&allowed)
                .any(|(material, allowed)| *material && !allowed)
            {
                return Err(err("native material mask exceeds authorized cells"));
            }
            Ok::<_, crate::Error>((mask, v.clear_blends))
        })
        .transpose()?;
    let scale = p.number("Z_SCALE", 0.0390625);
    let water_z = p.number("WATER_Z", 200.);
    if !scale.is_finite() || scale <= 0. || !water_z.is_finite() {
        return Err(err("invalid native edit height scale or water level"));
    }
    let border = hm.border as usize;
    for y in border..h - border {
        for x in border..w - border {
            let i = y * w + x;
            let native = (h - 1 - y) * w + x;
            if water[i] != (f64::from(hm.elevations[native]) * scale < water_z)
                || water[i] != (layers[i] == 0)
            {
                return Err(err(format!(
                    "native scope design water/layer mask disagrees with pinned source at {x},{y}"
                )));
            }
        }
    }
    let mut payload = chunk.payload.clone();
    let mut seen = BTreeSet::new();
    for c in cells {
        if c.x < border || c.y < border || c.x >= w - border || c.y >= h - border {
            return Err(err(format!(
                "native cell patch outside playable bounds: {},{}",
                c.x, c.y
            )));
        }
        if !seen.insert((c.x, c.y)) {
            return Err(err(format!(
                "duplicate native cell edit coordinate: {},{}",
                c.x, c.y
            )));
        }
        let i = c.y * w + c.x;
        let native = (h - 1 - c.y) * w + c.x;
        if !allowed[i] {
            return Err(err(format!(
                "native cell edit outside declared scope: {},{}",
                c.x, c.y
            )));
        }
        if hm.elevations[native] != c.before_raw {
            return Err(err(format!(
                "native cell patch source height mismatch at {},{}",
                c.x, c.y
            )));
        }
        if (f64::from(c.raw) * scale < water_z) != water[i] {
            return Err(err(format!(
                "native cell edit changes water/land boundary at {},{}",
                c.x, c.y
            )));
        }
        if c.raw != c.before_raw
            && !slopes.get(&i).is_some_and(|ranges| {
                ranges
                    .iter()
                    .any(|range| (range[0]..=range[1]).contains(&c.raw))
            })
        {
            return Err(err(format!(
                "native height edit outside authorized slope or raw range at {},{}",
                c.x, c.y
            )));
        }
        hm.elevations[native] = c.raw;
        set_bit(&mut payload, imp, native, w, c.impassable);
        set_bit(&mut payload, build, native, w, c.build_blocked);
        if let Some(tile) = c.tile {
            if usize::from(tile) / 64 >= bt.textures.len() {
                return Err(err("native edit tile references an unknown texture"));
            }
            let Some((mask, clear)) = &materials else {
                return Err(err("native tile edits require an explicit material scope"));
            };
            if !mask[i] {
                return Err(err(format!(
                    "native tile edit outside declared material scope at {},{}",
                    c.x, c.y
                )));
            }
            payload[4 + native * 2..6 + native * 2].copy_from_slice(&tile.to_le_bytes());
            if *clear {
                for layer in 0..3 {
                    let offset = 4 + n * 2 + layer * n * plane + native * plane;
                    payload[offset..offset + plane].fill(0);
                }
            }
        }
    }
    let height_payload = hm.encode()?;
    let current_height = m.get("HeightMapData")?;
    let current_blend = m.get("BlendTileData")?;
    let original_height = source.get("HeightMapData")?;
    let original = current_height == original_height && current_blend == chunk;
    let already_applied = current_height.version == original_height.version
        && current_height.payload == height_payload
        && current_blend.version == chunk.version
        && current_blend.payload == payload;
    if !original && !already_applied {
        return Err(err(
            "native edit source geometry differs from pinned source outside the complete declared patch",
        ));
    }
    let mut candidate = m.clone();
    candidate.get_mut("HeightMapData")?.payload = height_payload;
    candidate.get_mut("BlendTileData")?.payload = payload;
    *m = candidate;
    Ok(())
}
