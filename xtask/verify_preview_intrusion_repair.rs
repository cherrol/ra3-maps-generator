//! Independent NPZ/PNG readback for the reviewed regional intrusion repairs.
//!
//! This verifier deliberately re-implements the ZIP directory, NPY headers, PNG
//! chunks, CRCs, water components and pixel comparison instead of calling the
//! pipeline that produced them. Decompression uses the same zlib backend as the
//! writer; every check below is an independent second implementation.
use ra3maps::formats::sha256;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

const W: usize = 840;
const H: usize = 840;
const BORDER: usize = 20;
const PIXELS: usize = 640_000;
const DATE: &str = "2026-10-30";
const REPORT: &str = "intrusion-repair-2026-10-30.json";
const COLORS: [[u8; 3]; 6] = [
    [53, 116, 150],
    [104, 151, 89],
    [160, 178, 99],
    [202, 180, 119],
    [162, 128, 103],
    [218, 221, 211],
];

type Fallible<T> = Result<T, String>;

fn dark(color: &[u8; 3]) -> [u8; 3] {
    [
        (f64::from(color[0]) * 0.77) as u8,
        (f64::from(color[1]) * 0.77) as u8,
        (f64::from(color[2]) * 0.77) as u8,
    ]
}

// ---------------------------------------------------------------------------
// CRC-32 and ZIP directory: re-implemented, not the workspace writers.
// ---------------------------------------------------------------------------
const CRC_TABLE: [u32; 256] = crc_table();
const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut value = index as u32;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 != 0 {
                0xedb8_8320 ^ (value >> 1)
            } else {
                value >> 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc = CRC_TABLE[((crc ^ u32::from(*byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

fn u16_at(data: &[u8], offset: usize) -> Fallible<u16> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or_else(|| "truncated ZIP structure".to_string())?
            .try_into()
            .map_err(|_| "truncated ZIP structure".to_string())?,
    ))
}
fn u32_at(data: &[u8], offset: usize) -> Fallible<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or_else(|| "truncated ZIP structure".to_string())?
            .try_into()
            .map_err(|_| "truncated ZIP structure".to_string())?,
    ))
}

fn read_zip(data: &[u8]) -> Fallible<BTreeMap<String, Vec<u8>>> {
    let eocd = (0..data.len().saturating_sub(21))
        .rev()
        .find(|&offset| data.get(offset..offset + 4) == Some(b"PK\x05\x06"))
        .ok_or_else(|| "missing ZIP end of central directory".to_string())?;
    let count = u16_at(data, eocd + 10)? as usize;
    let directory = u32_at(data, eocd + 16)? as usize;
    let mut files = BTreeMap::new();
    let mut position = directory;
    for _ in 0..count {
        if data.get(position..position + 4) != Some(b"PK\x01\x02") {
            return Err("invalid ZIP central directory entry".into());
        }
        let method = u16_at(data, position + 10)?;
        let crc = u32_at(data, position + 16)?;
        let packed = u32_at(data, position + 20)? as usize;
        let size = u32_at(data, position + 24)? as usize;
        let name_length = u16_at(data, position + 28)? as usize;
        let extra_length = u16_at(data, position + 30)? as usize;
        let comment_length = u16_at(data, position + 32)? as usize;
        let local = u32_at(data, position + 42)? as usize;
        let name = std::str::from_utf8(
            data.get(position + 46..position + 46 + name_length)
                .ok_or_else(|| "truncated ZIP name".to_string())?,
        )
        .map_err(|_| "non-UTF8 ZIP entry name".to_string())?
        .to_string();
        position += 46 + name_length + extra_length + comment_length;
        if data.get(local..local + 4) != Some(b"PK\x03\x04") {
            return Err(format!("invalid ZIP local header for {name}"));
        }
        let local_name = u16_at(data, local + 26)? as usize;
        let local_extra = u16_at(data, local + 28)? as usize;
        let body = data
            .get(
                local + 30 + local_name + local_extra
                    ..local + 30 + local_name + local_extra + packed,
            )
            .ok_or_else(|| format!("truncated ZIP body for {name}"))?;
        let out = match method {
            0 => body.to_vec(),
            8 => {
                let mut out = Vec::new();
                flate2::read::DeflateDecoder::new(body)
                    .take(size as u64 + 1)
                    .read_to_end(&mut out)
                    .map_err(|error| format!("{name}: {error}"))?;
                out
            }
            _ => return Err(format!("unsupported ZIP compression {method} for {name}")),
        };
        if out.len() != size || crc32(&out) != crc {
            return Err(format!("ZIP CRC/length mismatch: {name}"));
        }
        if files.insert(name.clone(), out).is_some() {
            return Err(format!("duplicate ZIP entry: {name}"));
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// NPY / PNG
// ---------------------------------------------------------------------------
struct Array {
    descr: String,
    shape: Vec<usize>,
    data: Vec<u8>,
}
impl Array {
    fn parse(bytes: &[u8]) -> Fallible<Self> {
        if bytes.get(..6) != Some(b"\x93NUMPY") {
            return Err("invalid NPY magic".into());
        }
        let major = *bytes.get(6).ok_or("truncated NPY header")?;
        let offset = if major == 1 { 10 } else { 12 };
        let size = match major {
            1 => u16::from_le_bytes(
                bytes
                    .get(8..10)
                    .ok_or("truncated NPY header")?
                    .try_into()
                    .map_err(|_| "truncated NPY header".to_string())?,
            ) as usize,
            _ => u32_at(bytes, 8)? as usize,
        };
        let header = std::str::from_utf8(
            bytes
                .get(offset..offset + size)
                .ok_or("truncated NPY header")?,
        )
        .map_err(|_| "non-UTF8 NPY header".to_string())?;
        if header.contains("'fortran_order': True") {
            return Err("Fortran order NPY unsupported".into());
        }
        let descr = between(header, "'descr': ", ",")
            .or_else(|| between(header, "'descr': ", "}"))
            .ok_or("NPY header lacks a dtype")?
            .trim()
            .trim_matches(['\'', '"'])
            .to_string();
        let shape = between(header, "'shape': ", ")").ok_or("NPY header lacks a shape")?;
        let shape = shape
            .trim_start_matches('(')
            .split(',')
            .filter(|part| !part.trim().is_empty())
            .map(|part| part.trim().parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "invalid NPY shape".to_string())?;
        Ok(Self {
            descr,
            shape,
            data: bytes[offset + size..].to_vec(),
        })
    }
    fn itemsize(&self) -> Fallible<usize> {
        let digits = self
            .descr
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .collect::<Vec<_>>();
        digits
            .iter()
            .rev()
            .collect::<String>()
            .parse::<usize>()
            .map_err(|_| format!("unsupported NPY dtype {}", self.descr))
    }
    fn is_float(&self) -> bool {
        self.descr
            .trim_start_matches(['<', '>', '|', '='])
            .starts_with('f')
    }
}
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = text.split_once(start)?.1;
    let length = rest.find(end)?;
    Some(&rest[..length])
}

struct Png {
    width: usize,
    height: usize,
    stride: usize,
    raw: Vec<u8>,
}
fn read_png(path: &Path) -> Fallible<Png> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return Err(format!("{}: invalid PNG signature", path.display()));
    }
    let mut position = 8;
    let (mut width, mut height) = (0usize, 0usize);
    let mut packed = Vec::new();
    while position < bytes.len() {
        let length = u32::from_be_bytes(
            bytes
                .get(position..position + 4)
                .ok_or("truncated PNG chunk")?
                .try_into()
                .map_err(|_| "truncated PNG chunk".to_string())?,
        ) as usize;
        let kind = bytes
            .get(position + 4..position + 8)
            .ok_or("truncated PNG chunk")?;
        let body = bytes
            .get(position + 8..position + 8 + length)
            .ok_or("truncated PNG chunk")?;
        let crc = u32::from_be_bytes(
            bytes
                .get(position + 8 + length..position + 12 + length)
                .ok_or("truncated PNG chunk")?
                .try_into()
                .map_err(|_| "truncated PNG chunk".to_string())?,
        );
        let mut checked = kind.to_vec();
        checked.extend_from_slice(body);
        if crc32(&checked) != crc {
            return Err(format!("{}: PNG chunk CRC mismatch", path.display()));
        }
        if kind == b"IHDR" {
            if body.len() != 13 {
                return Err("invalid PNG IHDR".into());
            }
            width = u32::from_be_bytes(body[0..4].try_into().unwrap()) as usize;
            height = u32::from_be_bytes(body[4..8].try_into().unwrap()) as usize;
            let (depth, color, compression, filter, interlace) =
                (body[8], body[9], body[10], body[11], body[12]);
            if (depth, color, compression, filter, interlace) != (8, 2, 0, 0, 0) {
                return Err(format!("{}: unsupported PNG format", path.display()));
            }
        }
        if kind == b"IDAT" {
            packed.extend_from_slice(body);
        }
        position += 12 + length;
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(packed.as_slice())
        .read_to_end(&mut raw)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let stride = width * 3 + 1;
    if raw.len() != stride * height {
        return Err(format!("{}: PNG data length mismatch", path.display()));
    }
    if (0..height).any(|row| raw[row * stride] != 0) {
        return Err(format!("{}: unsupported PNG row filters", path.display()));
    }
    Ok(Png {
        width,
        height,
        stride,
        raw,
    })
}

// ---------------------------------------------------------------------------
// Water components
// ---------------------------------------------------------------------------
struct Body {
    water_cells: usize,
    bounds: [usize; 4],
    open: bool,
    contacts: Vec<(u8, usize)>,
}
fn waterbodies(layers: &[u8]) -> Vec<Body> {
    let mut seen = vec![false; W * H];
    let mut bodies = Vec::new();
    for y in BORDER..H - BORDER {
        for x in BORDER..W - BORDER {
            let start = y * W + x;
            if layers[start] != 0 || seen[start] {
                continue;
            }
            let mut queue = VecDeque::from([start]);
            seen[start] = true;
            let mut cells = 0usize;
            let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
            let mut open = false;
            let mut votes: Vec<(u8, usize)> = Vec::new();
            while let Some(index) = queue.pop_front() {
                cells += 1;
                let (ix, iy) = (index % W, index / W);
                x0 = x0.min(ix);
                y0 = y0.min(iy);
                x1 = x1.max(ix);
                y1 = y1.max(iy);
                open |=
                    ix == BORDER || ix == W - BORDER - 1 || iy == BORDER || iy == H - BORDER - 1;
                for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                    let (xx, yy) = (ix as isize + dx, iy as isize + dy);
                    if xx < BORDER as isize
                        || yy < BORDER as isize
                        || xx >= (W - BORDER) as isize
                        || yy >= (H - BORDER) as isize
                    {
                        continue;
                    }
                    let neighbor = yy as usize * W + xx as usize;
                    if layers[neighbor] != 0 {
                        match votes
                            .iter_mut()
                            .find(|(layer, _)| *layer == layers[neighbor])
                        {
                            Some((_, count)) => *count += 1,
                            None => votes.push((layers[neighbor], 1)),
                        }
                    } else if !seen[neighbor] {
                        seen[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            bodies.push(Body {
                water_cells: cells,
                bounds: [x0, y0, x1 + 1, y1 + 1],
                open,
                contacts: votes,
            });
        }
    }
    bodies
}
fn contacts_json(contacts: &[(u8, usize)]) -> Value {
    let mut map = Map::new();
    for (layer, count) in contacts {
        map.insert(layer.to_string(), json!(count));
    }
    Value::Object(map)
}
fn body_json(body: &Body) -> Value {
    json!({
        "water_cells": body.water_cells,
        "bounds": body.bounds,
        "open": body.open,
        "contacts": contacts_json(&body.contacts),
    })
}

// ---------------------------------------------------------------------------
// Readback
// ---------------------------------------------------------------------------
fn arrays(path: &Path) -> Fallible<BTreeMap<String, Array>> {
    let data = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut out = BTreeMap::new();
    for (name, bytes) in read_zip(&data)? {
        if let Some(stem) = name.strip_suffix(".npy") {
            out.insert(stem.to_string(), Array::parse(&bytes)?);
        }
    }
    Ok(out)
}

fn indices(value: &Value, context: &str) -> Fallible<Vec<usize>> {
    value
        .as_array()
        .ok_or_else(|| format!("{context} is not an array"))?
        .iter()
        .map(|entry| {
            entry
                .as_u64()
                .map(|entry| entry as usize)
                .ok_or_else(|| format!("{context} holds a non-index entry"))
        })
        .collect()
}

fn verify(root: &Path) -> Fallible<Value> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("maps"))
        .map_err(|error| format!("{}: {error}", root.join("maps").display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let candidate = entry.path().join("data/terrain-preview").join(REPORT);
        if candidate.is_file() {
            paths.push(candidate);
        }
    }
    paths.sort();
    let mut results = Vec::new();
    let (mut images_checked, mut pixels_checked) = (0usize, 0usize);
    let mut all_passed = true;
    for path in &paths {
        let directory = path
            .parent()
            .ok_or_else(|| format!("{}: no report directory", path.display()))?;
        let name = directory
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{}: no map id", path.display()))?;
        let map_root = directory
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| format!("{}: no map root", path.display()))?;
        let report: Value = serde_json::from_slice(
            &fs::read(directory.join("report.json")).map_err(|error| format!("{name}: {error}"))?,
        )
        .map_err(|error| format!("{name}: invalid report: {error}"))?;
        let repair: Value =
            serde_json::from_slice(&fs::read(path).map_err(|error| format!("{name}: {error}"))?)
                .map_err(|error| format!("{name}: invalid repair record: {error}"))?;
        let baseline = repair["baseline"]
            .as_str()
            .ok_or_else(|| format!("{name}: repair record lacks a baseline"))?;
        let current = arrays(&directory.join("terrain.npz"))?;
        let old = arrays(&directory.join(baseline).join("terrain.npz"))?;
        let mut errors = Vec::new();
        fn check(errors: &mut Vec<String>, condition: bool, message: &str) {
            if !condition {
                errors.push(message.to_string());
            }
        }
        check(
            &mut errors,
            current.keys().collect::<Vec<_>>() == old.keys().collect::<Vec<_>>(),
            "array inventory",
        );
        check(
            &mut errors,
            report["game_cells"] == json!([800, 800]) && report["border_cells"] == json!(20),
            "frame",
        );
        for (key, array) in &current {
            if matches!(key.as_str(), "layers" | "game_heights" | "water_mask") {
                continue;
            }
            let same = old
                .get(key)
                .is_some_and(|previous| previous.data == array.data);
            check(&mut errors, same, &format!("protected array {key}"));
        }
        for key in ["layers", "game_heights", "water_mask"] {
            let array = current
                .get(key)
                .ok_or_else(|| format!("{name}: missing {key} array"))?;
            if array.shape != [W, H] {
                return Err(format!("{name}: {key} is not {W}x{H}"));
            }
        }
        let layers = &current["layers"].data;
        let previous_layers = &old["layers"].data;
        if layers.len() != W * H || previous_layers.len() != W * H {
            return Err(format!("{name}: layer arrays are not {W}x{H}"));
        }
        let changed = (0..W * H)
            .filter(|index| layers[*index] != previous_layers[*index])
            .collect::<Vec<_>>();
        check(
            &mut errors,
            changed == indices(&repair["changed_full_grid_indices"], "changed indices")?,
            "exact scope indices",
        );
        let mut allowed = BTreeSet::new();
        for site in repair["reviewed_sites"]
            .as_array()
            .ok_or_else(|| format!("{name}: reviewed sites are missing"))?
        {
            for merge in site["merges"]
                .as_array()
                .ok_or_else(|| format!("{name}: reviewed site merges are missing"))?
            {
                allowed.extend(indices(&merge["changed_full_grid_indices"], "site merge")?);
            }
        }
        for band in repair["shore_reconciliation"]
            .as_array()
            .ok_or_else(|| format!("{name}: shore reconciliation is missing"))?
        {
            allowed.extend(indices(&band["changed_full_grid_indices"], "shore band")?);
        }
        for fragment in repair["detached_fragment_merges"]
            .as_array()
            .ok_or_else(|| format!("{name}: detached fragment merges are missing"))?
        {
            allowed.extend(indices(
                &fragment["changed_full_grid_indices"],
                "detached fragment",
            )?);
        }
        check(
            &mut errors,
            changed.iter().all(|index| allowed.contains(index)),
            "outside declared scope",
        );
        check(
            &mut errors,
            changed.iter().all(|index| {
                (BORDER..W - BORDER).contains(&(index % W))
                    && (BORDER..H - BORDER).contains(&(index / W))
            }),
            "border changed",
        );
        let heights = &current["game_heights"];
        let itemsize = heights.itemsize()?;
        if itemsize != 4 || !heights.is_float() {
            return Err(format!("{name}: game_heights is not float32"));
        }
        if heights.data.len() != W * H * 4 {
            return Err(format!("{name}: game_heights is not {W}x{H}"));
        }
        let water_mask = &current["water_mask"].data;
        let layer_heights = report["game_layer_heights"]
            .as_array()
            .ok_or_else(|| format!("{name}: report lacks game layer heights"))?
            .iter()
            .map(|height| {
                height
                    .as_f64()
                    .ok_or_else(|| "invalid layer height".to_string())
            })
            .collect::<Fallible<Vec<_>>>()?;
        check(
            &mut errors,
            (0..W * H).all(|index| {
                let layer = layers[index];
                if layer > 5 {
                    return false;
                }
                let expected = if layer == 0 {
                    130.0
                } else {
                    layer_heights[(layer - 1) as usize]
                };
                true && (water_mask[index] != 0) == (layer == 0)
                    && f64::from(f32::from_le_bytes(
                        heights.data[index * 4..index * 4 + 4].try_into().unwrap(),
                    )) == expected
            }),
            "height/water consistency",
        );
        let terrain = directory.join("terrain.npz");
        check(
            &mut errors,
            repair["terrain_crc32"].as_u64()
                == Some(u64::from(crc32(
                    &fs::read(&terrain)
                        .map_err(|error| format!("{}: {error}", terrain.display()))?,
                ))),
            "repair checksum",
        );
        let mut sources = Vec::new();
        for file in report["final_images"]
            .as_array()
            .ok_or_else(|| format!("{name}: report lacks final images"))?
        {
            let file = file
                .as_str()
                .ok_or_else(|| format!("{name}: invalid image name"))?;
            let image = map_root.join(file);
            let bytes =
                fs::read(&image).map_err(|error| format!("{}: {error}", image.display()))?;
            let png = read_png(&image)?;
            check(
                &mut errors,
                (png.width, png.height) == (1104, 1002),
                &format!("image dimensions {file}"),
            );
            check(
                &mut errors,
                report["image_crc32"][file].as_u64() == Some(u64::from(crc32(&bytes))),
                &format!("image crc {file}"),
            );
            sources.push(json!({
                "file": file,
                "sha256": sha256::hex(&bytes),
                "size": [png.width, png.height],
            }));
        }
        let preview = read_png(&map_root.join("preview.png"))?;
        let mut mismatch = 0usize;
        for y in 0..800 {
            let start = (112 + y) * preview.stride + 1 + 28 * 3;
            for x in 0..800 {
                let index = (y + BORDER) * W + x + BORDER;
                let layer = layers[index];
                let Some(base) = COLORS.get(layer as usize) else {
                    mismatch += 1;
                    continue;
                };
                let expected = if layer == 0 {
                    COLORS[0]
                } else if [index - 1, index + 1, index - W, index + W]
                    .iter()
                    .any(|neighbor| layers[*neighbor] != 0 && layers[*neighbor] != layer)
                {
                    dark(base)
                } else {
                    *base
                };
                if preview.raw[start + x * 3..start + x * 3 + 3] != expected {
                    mismatch += 1;
                }
            }
        }
        check(&mut errors, mismatch == 0, "preview pixel mismatch");
        let bodies = waterbodies(layers);
        let mut bodies_json = bodies.iter().map(body_json).collect::<Vec<_>>();
        let closed = bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| !body.open)
            .map(|(index, body)| {
                check(
                    &mut errors,
                    body.contacts.len() == 1,
                    "closed shore multi-tier",
                );
                index
            })
            .collect::<Vec<_>>();
        // Every retained closed body must retain the owner of the old component
        // at overlapping cells.
        let mut old_labels = vec![0usize; W * H];
        let mut old_seen = vec![false; W * H];
        let mut owners: Vec<u8> = Vec::new();
        for start in (BORDER * W + BORDER)..(H - BORDER) * W {
            let (x, y) = (start % W, start / W);
            if !(BORDER..W - BORDER).contains(&x)
                || !(BORDER..H - BORDER).contains(&y)
                || previous_layers[start] != 0
                || old_seen[start]
            {
                continue;
            }
            let label = owners.len() + 1;
            old_seen[start] = true;
            let mut queue = VecDeque::from([start]);
            let mut votes: Vec<(u8, usize)> = Vec::new();
            while let Some(index) = queue.pop_front() {
                old_labels[index] = label;
                let (ix, iy) = (index % W, index / W);
                for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                    let (xx, yy) = (ix as isize + dx, iy as isize + dy);
                    if xx < BORDER as isize
                        || yy < BORDER as isize
                        || xx >= (W - BORDER) as isize
                        || yy >= (H - BORDER) as isize
                    {
                        continue;
                    }
                    let neighbor = yy as usize * W + xx as usize;
                    if previous_layers[neighbor] != 0 {
                        let layer = previous_layers[neighbor];
                        match votes.iter_mut().find(|(known, _)| *known == layer) {
                            Some((_, count)) => *count += 1,
                            None => votes.push((layer, 1)),
                        }
                    } else if !old_seen[neighbor] {
                        old_seen[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            let owner = votes
                .iter()
                .fold(None::<(u8, usize)>, |best, candidate| match best {
                    Some(best) if best.1 >= candidate.1 => Some(best),
                    _ => Some(*candidate),
                })
                .map(|(layer, _)| layer)
                .ok_or_else(|| format!("{name}: old water component has no land owner"))?;
            owners.push(owner);
        }
        for &index in &closed {
            let body = &bodies[index];
            let [x0, y0, x1, y1] = body.bounds;
            let mut overlap = BTreeSet::new();
            for y in y0..y1 {
                for x in x0..x1 {
                    let cell = y * W + x;
                    if layers[cell] == 0 && old_labels[cell] != 0 {
                        overlap.insert(old_labels[cell]);
                    }
                }
            }
            check(
                &mut errors,
                overlap.iter().all(|label| {
                    body.contacts
                        .iter()
                        .any(|(layer, _)| *layer == owners[*label - 1])
                }),
                "prior water owner changed",
            );
        }
        // Every enclosed shore must reach retained land of the same owner tier.
        let mut land_component = vec![0usize; W * H];
        let mut anchors: Vec<bool> = vec![true];
        for &index in &closed {
            let body = &bodies[index];
            let [x0, y0, x1, y1] = body.bounds;
            let mut shore_cells = BTreeSet::new();
            for y in y0..y1 {
                for x in x0..x1 {
                    let cell = y * W + x;
                    if layers[cell] != 0 {
                        continue;
                    }
                    for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                        let (xx, yy) = (x as isize + dx, y as isize + dy);
                        if xx < BORDER as isize
                            || yy < BORDER as isize
                            || xx >= (W - BORDER) as isize
                            || yy >= (H - BORDER) as isize
                        {
                            continue;
                        }
                        if layers[yy as usize * W + xx as usize] != 0 {
                            shore_cells.insert(yy as usize * W + xx as usize);
                        }
                    }
                }
            }
            for start in &shore_cells {
                if land_component[*start] != 0 {
                    continue;
                }
                let component = anchors.len();
                anchors.push(true);
                let layer = layers[*start];
                land_component[*start] = component;
                let mut queue = VecDeque::from([*start]);
                let mut anchor = false;
                while let Some(cell) = queue.pop_front() {
                    anchor |= layers[cell] == previous_layers[cell];
                    let (ix, iy) = (cell % W, cell / W);
                    for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                        let (xx, yy) = (ix as isize + dx, iy as isize + dy);
                        if xx < BORDER as isize
                            || yy < BORDER as isize
                            || xx >= (W - BORDER) as isize
                            || yy >= (H - BORDER) as isize
                        {
                            continue;
                        }
                        let neighbor = yy as usize * W + xx as usize;
                        if layers[neighbor] == layer && land_component[neighbor] == 0 {
                            land_component[neighbor] = component;
                            queue.push_back(neighbor);
                        }
                    }
                }
                anchors[component] = anchor;
            }
            let anchored = shore_cells
                .iter()
                .all(|cell| anchors[land_component[*cell]]);
            check(
                &mut errors,
                anchored,
                "shore disconnected from retained owner land",
            );
            bodies_json[index]["connected_to_unchanged_owner_land"] = json!(anchored);
        }
        let changed_water = changed
            .iter()
            .filter(|index| (layers[**index] == 0) != (previous_layers[**index] == 0))
            .count();
        let passed = errors.is_empty();
        all_passed &= passed;
        images_checked += sources.len();
        pixels_checked += PIXELS;
        println!("{name} {} cells {errors:?}", changed.len());
        results.push(json!({
            "map_id": name,
            "terrain_sha256": sha256::hex(&fs::read(&terrain).map_err(|error| format!("{}: {error}", terrain.display()))?),
            "changed_cells": changed.len(),
            "changed_water_cells": changed_water,
            "images": sources,
            "pixels_checked": PIXELS,
            "pixel_mismatches": mismatch,
            "closed_waterbodies": closed.iter().map(|index| bodies_json[*index].clone()).collect::<Vec<_>>(),
            "waterbodies": bodies_json,
            "errors": errors,
            "independent_checks_passed": passed,
        }));
    }
    if results.len() != 11 {
        return Err(format!(
            "expected 11 repaired regions, found {}",
            results.len()
        ));
    }
    Ok(json!({
        "date": DATE,
        "maps": results,
        "maps_checked": 11,
        "images_checked": images_checked,
        "pixels_checked": pixels_checked,
        "all_independent_checks_passed": all_passed,
    }))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Fallible<()> {
    let mut arguments = env::args().skip(1);
    let root = match arguments.next() {
        Some(root) => PathBuf::from(root),
        None => env::current_dir().map_err(|error| error.to_string())?,
    };
    let root = fs::canonicalize(&root).map_err(|error| format!("{}: {error}", root.display()))?;
    let output = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/preview-intrusion-readback.json"));
    let report = verify(&root)?;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(
        &output,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        ),
    )
    .map_err(|error| format!("{}: {error}", output.display()))?;
    if report["all_independent_checks_passed"] != json!(true) {
        return Err("independent readback found failures".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_published_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }
    #[test]
    fn water_components_keep_votes_and_open_edges() {
        let mut layers = vec![1u8; W * H];
        for y in 100..110 {
            for x in 100..110 {
                layers[y * W + x] = 0;
            }
        }
        layers[300 * W + 20] = 0;
        let bodies = waterbodies(&layers);
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0].water_cells, 100);
        assert!(!bodies[0].open);
        assert_eq!(bodies[0].contacts, vec![(1, 40)]);
        assert_eq!(bodies[0].bounds, [100, 100, 110, 110]);
        assert_eq!(bodies[1].water_cells, 1);
        assert!(bodies[1].open, "a border-touching cell is an open body");
    }
    #[test]
    fn npy_headers_are_parsed_without_numpy() {
        let mut bytes = b"\x93NUMPY\x01\x00".to_vec();
        let header = "{'descr': '|u1', 'fortran_order': False, 'shape': (2, 3,), }";
        bytes.extend_from_slice(&u16::try_from(header.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        let array = Array::parse(&bytes).unwrap();
        assert_eq!(array.descr, "|u1");
        assert_eq!(array.shape, [2, 3]);
        assert_eq!(array.data, [1, 2, 3, 4, 5, 6]);
        assert_eq!(array.itemsize().unwrap(), 1);
    }
}
