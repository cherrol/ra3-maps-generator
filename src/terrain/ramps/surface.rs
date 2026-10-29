//! Ramp material selection and local blend-array updates.
use super::*;

pub(super) fn paint(
    tiles: &mut [u16],
    textures: &[String],
    z: &[f64],
    n: usize,
    ramps: (&Value, &Value),
    water: f64,
    changed: &mut [bool],
) {
    if ramps.1["surface_style"] == "mountain" {
        return paint_mountain(tiles, textures, z, n, ramps, water, changed);
    }
    let (old, new) = ramps;
    let idx = |s: &str| textures.iter().position(|t| t == s).map(|i| i as u16);
    let Some(dirt) = idx("Dirt_Yucatan06") else {
        return;
    };
    let gravel = idx("Gravel_Yucatan01").unwrap_or(dirt);
    let rock = idx("Rock_Yucatan01");
    let cx = number(new, "x", 0.);
    let cy = number(new, "y", 0.);
    for y in (cy as isize - 44).max(0)..=(cy as isize + 44).min((z.len() / n) as isize - 1) {
        for x in (cx as isize - 44).max(0)..=(cx as isize + 44).min(n as isize - 1) {
            let i = y as usize * n + x as usize;
            let (u, v) = frame(new, x as f64, y as f64);
            let mut family = tiles[i] / 64;
            if old_foot(old, x as f64, y as f64, 12.) && (family == dirt || family == gravel) {
                let names = if z[i] < water {
                    ["RA3_ShallowSeaFloor", "Gravel_Yucatan01"]
                } else if z[i] < 245. {
                    ["Grass_NewYork01", "Grass_MtRush04"]
                } else if z[i] < 315. {
                    ["Grass_MtRush04", "Grass_NewYork01"]
                } else if z[i] < 385. {
                    ["Dirt_Iceland05", "Grass_MtRush04"]
                } else {
                    ["Pavement_Amsterdam05", "Dirt_Iceland05"]
                };
                family = names.iter().find_map(|s| idx(s)).unwrap_or(family);
            }
            let width = mouth(new, u) * 0.75;
            let noise = ((x * 37 + y * 17).rem_euclid(7) as f64 - 3.) * 0.10;
            if u.abs() < half(new) + 2. && v.abs() < width + noise {
                family = dirt;
            } else if u.abs() < half(new) + 4. && v.abs() < width + 2. + noise {
                family = gravel;
            } else if contains(new, x as f64, y as f64, 1.)
                && let Some(rock) = rock
            {
                let slope_x = (sample(z, n, x as f64 + 1., y as f64)
                    - sample(z, n, x as f64 - 1., y as f64))
                .abs()
                    * 0.5;
                let slope_y = (sample(z, n, x as f64, y as f64 + 1.)
                    - sample(z, n, x as f64, y as f64 - 1.))
                .abs()
                    * 0.5;
                if slope_x.hypot(slope_y) > 24. {
                    family = rock;
                }
            }
            if family != tiles[i] / 64 {
                tiles[i] = family * 64 + tiles[i] % 64;
                changed[i] = true;
            }
        }
    }
}

pub(crate) fn patch_blend(
    map: &mut MapFile,
    bt: &BlendTiles,
    tiles: &[u16],
    imp: &[bool],
    build: &[bool],
    changed: &[bool],
    n: usize,
) -> Result<()> {
    let data = &mut map.get_mut("BlendTileData")?.payload;
    let area = tiles.len();
    let h = area / n;
    let mask_len = h * n.div_ceil(8);
    let tail_start = 4 + 8 * area + 8 * mask_len + area;
    let mut tail = crate::formats::binary::Reader::new(&data[tail_start..]);
    tail.take(12)?;
    let count = tail.u32()? as usize;
    for _ in 0..count {
        tail.take(16)?;
        tail.s16()?;
    }
    tail.take(8)?;
    let descriptions_start = tail_start + tail.pos;
    let mut descriptions = bt.blend_descriptions.clone();
    let mut cache = BTreeMap::new();
    // Reuse the existing exact records. A fresh cache on every incremental edit
    // used to append the same blends repeatedly until only an editor save shrank
    // the table. Full bytes keep corner/three-way flags and native magic distinct.
    for (i, row) in descriptions.as_chunks::<18>().0.iter().enumerate() {
        cache.entry(row.to_vec()).or_insert(u16::try_from(i + 1)?);
    }
    for y in 0..h {
        for x in 0..n {
            let i = y * n + x;
            let game = (h - 1 - y) * n + x;
            if changed[i] {
                data[4 + game * 2..6 + game * 2].copy_from_slice(&tiles[i].to_le_bytes());
            }
            // Use standard two-texture blend descriptions at the actual per-cell
            // material edge. Existing atlas definitions and IDs are never rebuilt.
            if !changed[i] && !crate::terrain::grid::neighbors_rect(i, n, h).any(|k| changed[k]) {
                continue;
            }
            // A neighbor can still blend toward a removed material even when
            // its own base tile never changed. Clear the complete local halo,
            // including three-way and cliff blends, before rebuilding edges.
            for array in 0..3 {
                let o = 4 + 2 * area + 2 * area * array + 2 * game;
                data[o..o + 2].copy_from_slice(&0_u16.to_le_bytes());
            }
            let neighbor = [
                (1_isize, 0_isize, 0_usize, 0_u8),
                (-1, 0, 0, 1),
                (0, -1, 1, 0),
                (0, 1, 1, 1),
            ]
            .into_iter()
            .find_map(|(dx, dy, dir, flags)| {
                let xx = x as isize + dx;
                let yy = y as isize + dy;
                if xx < 0 || yy < 0 || xx >= n as isize || yy >= h as isize {
                    return None;
                }
                let j = yy as usize * n + xx as usize;
                (tiles[j] / 64 != tiles[i] / 64).then_some((
                    tiles[j] / 64 * 64 + tiles[i] % 64,
                    dir,
                    flags,
                ))
            });
            if let Some((secondary, dir, flags)) = neighbor {
                let mut key = Vec::with_capacity(18);
                key.extend_from_slice(&(secondary as u32).to_le_bytes());
                let mut direction = [0; 4];
                direction[dir] = 1;
                key.extend_from_slice(&direction);
                key.extend_from_slice(&[flags, 0]);
                key.extend_from_slice(&u32::MAX.to_le_bytes());
                key.extend_from_slice(&0x7ada0000_u32.to_le_bytes());
                let blend = if let Some(id) = cache.get(&key) {
                    *id
                } else {
                    let id = u16::try_from(descriptions.len() / 18 + 1)
                        .map_err(|_| err("too many texture blends"))?;
                    descriptions.extend_from_slice(&key);
                    cache.insert(key, id);
                    id
                };
                let o = 4 + 2 * area + 2 * game;
                data[o..o + 2].copy_from_slice(&blend.to_le_bytes());
            }
        }
    }
    for (index, mask) in [(0, imp), (5, build)] {
        let offset = 4 + 8 * area + index * mask_len;
        for y in 0..h {
            for x in 0..n {
                let bit = (h - 1 - y) * n.div_ceil(8) + x / 8;
                let b = 1 << (x % 8);
                if mask[y * n + x] {
                    data[offset + bit] |= b;
                } else {
                    data[offset + bit] &= !b;
                }
            }
        }
    }
    data[tail_start + 4..tail_start + 8]
        .copy_from_slice(&u32::try_from(descriptions.len() / 18 + 1)?.to_le_bytes());
    data.splice(
        descriptions_start..descriptions_start + bt.blend_descriptions.len(),
        descriptions,
    );
    Ok(())
}
