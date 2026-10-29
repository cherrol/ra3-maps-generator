//! Explicit, idempotent native cell patches for localized authoring edits.
use crate::{
    Result, err,
    formats::binary::Reader,
    formats::map::{HeightMap, MapFile},
    pipeline::project::Project,
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Cell {
    x: usize,
    y: usize,
    before_raw: u16,
    raw: u16,
    impassable: bool,
    build_blocked: bool,
    tile: Option<u16>,
}
pub fn apply(p: &Project, m: &mut MapFile) -> Result<()> {
    let Some(path) = p.cfg.get("NATIVE_CELL_EDITS").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let cells: Vec<Cell> = serde_json::from_slice(&std::fs::read(p.path(path)?)?)?;
    let mut hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let (w, h) = (hm.width, hm.height);
    let chunk = m.get("BlendTileData")?;
    let v = chunk.version;
    let mut reader = Reader::new(&chunk.payload);
    let n = reader.u32()? as usize;
    if n != w * h || cells.len() > n / 10 {
        return Err(err("native cell patch extent exceeds local edit limit"));
    }
    let plane = if (14..24).contains(&v) { 4 } else { 2 };
    reader.take(n * (2 + plane * 3))?;
    let bytes = h * w.div_ceil(8);
    let imp = reader.pos;
    if v < 24 {
        return Err(err("native cell patch requires buildability planes"));
    }
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
    let mut payload = chunk.payload.clone();
    let bit = |payload: &mut [u8], offset: usize, index: usize, value: bool| {
        let byte = offset + (index / w) * w.div_ceil(8) + (index % w) / 8;
        let mask = 1u8 << ((index % w) % 8);
        if value {
            payload[byte] |= mask;
        } else {
            payload[byte] &= !mask;
        }
    };
    for c in cells {
        if c.x < hm.border as usize
            || c.y < hm.border as usize
            || c.x >= w - hm.border as usize
            || c.y >= h - hm.border as usize
        {
            return Err(err("native cell patch outside playable bounds"));
        }
        let i = (h - 1 - c.y) * w + c.x;
        if hm.elevations[i] != c.before_raw && hm.elevations[i] != c.raw {
            return Err(err("native cell patch source height mismatch"));
        }
        hm.elevations[i] = c.raw;
        bit(&mut payload, imp, i, c.impassable);
        bit(&mut payload, build, i, c.build_blocked);
        if let Some(tile) = c.tile {
            payload[4 + i * 2..4 + i * 2 + 2].copy_from_slice(&tile.to_le_bytes());
            for layer in 0..3 {
                let offset = 4 + n * 2 + layer * n * plane + i * plane;
                payload[offset..offset + plane].fill(0);
            }
        }
    }
    m.get_mut("HeightMapData")?.payload = hm.encode()?;
    m.get_mut("BlendTileData")?.payload = payload;
    Ok(())
}
