//! Canonicalize RA3's terrain lookup tables without changing painted cells.
//! WorldBuilder does this on save; incremental edits must do it before export.
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{HeightMap, MapFile},
};
use serde::Serialize;
use std::collections::BTreeMap;

/// WorldHeightMapEdit rejects a saved map with 50 or more texture families.
pub const MAX_TEXTURES: usize = 49;

#[derive(Debug, Serialize)]
pub struct Report {
    pub textures_before: usize,
    pub textures_after: usize,
    pub unused_textures: usize,
    pub removed_textures: usize,
    pub blends_before: usize,
    pub blends_after: usize,
    pub cliffs_before: usize,
    pub cliffs_after: usize,
    pub changed: bool,
}

#[derive(Clone)]
struct Texture {
    start: u32,
    count: u32,
    size: u32,
    magic: u32,
    name: String,
}
struct Data {
    planes: [Vec<u16>; 4],
    // Navigation, buildability, air flags and shrubbery are opaque and exact.
    bitmaps: Vec<u8>,
    textures: Vec<Texture>,
    magic: Vec<u8>,
    blends: Vec<Vec<u8>>,
    cliffs: Vec<Vec<u8>>,
}
impl Data {
    fn parse(map: &MapFile) -> Result<Self> {
        let hm = HeightMap::parse(map.get("HeightMapData")?)?;
        let c = map.get("BlendTileData")?;
        if c.version != 27 {
            return Err(err(
                "terrain package normalization requires RA3 BlendTileData v27",
            ));
        }
        let area = hm
            .width
            .checked_mul(hm.height)
            .ok_or_else(|| err("terrain area overflow"))?;
        let mut r = Reader::new(&c.payload);
        if r.u32()? as usize != area {
            return Err(err("terrain package dimensions differ"));
        }
        let mut planes: [Vec<u16>; 4] = std::array::from_fn(|_| Vec::with_capacity(area));
        for plane in &mut planes {
            for _ in 0..area {
                plane.push(r.u16()?);
            }
        }
        let bitmaps = r
            .take(8 * hm.height * hm.width.div_ceil(8) + area)?
            .to_vec();
        let cells = r.u32()?;
        let blend_count = r.u32()?.saturating_sub(1) as usize;
        let cliff_count = r.u32()?.saturating_sub(1) as usize;
        let count = r.u32()? as usize;
        if count == 0 || count > r.remaining() / 18 {
            return Err(err("invalid terrain texture count"));
        }
        let mut textures = Vec::new();
        let mut expected = 0_u32;
        for _ in 0..count {
            let t = Texture {
                start: r.u32()?,
                count: r.u32()?,
                size: r.u32()?,
                magic: r.u32()?,
                name: r.s16()?,
            };
            if t.start != expected || t.size.checked_mul(t.size) != Some(t.count) || t.count == 0 {
                return Err(err("noncontiguous terrain texture cells"));
            }
            expected = expected
                .checked_add(t.count)
                .ok_or_else(|| err("terrain cell overflow"))?;
            textures.push(t);
        }
        if cells != expected || cells > 16384 {
            return Err(err(
                "terrain texture cell count differs or exceeds u16 tile space",
            ));
        }
        let magic = r.take(8)?.to_vec();
        let blends = (0..blend_count)
            .map(|_| r.take(18).map(Vec::from))
            .collect::<Result<_>>()?;
        let cliffs = (0..cliff_count)
            .map(|_| r.take(38).map(Vec::from))
            .collect::<Result<_>>()?;
        r.finish()?;
        Ok(Self {
            planes,
            bitmaps,
            textures,
            magic,
            blends,
            cliffs,
        })
    }
    fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer::new();
        w.u32(u32::try_from(self.planes[0].len())?);
        for plane in &self.planes {
            for &tile in plane {
                w.u16(tile);
            }
        }
        w.bytes(&self.bitmaps);
        w.u32(self.textures.iter().map(|t| t.count).sum());
        w.u32(u32::try_from(self.blends.len() + 1)?);
        w.u32(u32::try_from(self.cliffs.len() + 1)?);
        w.u32(u32::try_from(self.textures.len())?);
        for t in &self.textures {
            w.u32(t.start);
            w.u32(t.count);
            w.u32(t.size);
            w.u32(t.magic);
            w.s16(&t.name)?;
        }
        w.bytes(&self.magic);
        for b in self.blends.iter().chain(&self.cliffs) {
            w.bytes(b);
        }
        Ok(w.data)
    }
}
/// Append stock families while keeping all existing IDs, phases and masks exact.
pub(crate) fn ensure_materials(map: &mut MapFile, names: &[&str]) -> Result<Vec<String>> {
    let mut data = Data::parse(map)?;
    let mut added = Vec::new();
    for name in names {
        if data.textures.iter().any(|t| t.name == *name) {
            continue;
        }
        if data.textures.len() >= 64 {
            return Err(err("terrain atlas exceeds 64 stock texture families"));
        }
        if data.textures.iter().any(|t| t.count != 16 || t.size != 4) {
            return Err(err(
                "material extension requires stock 4x4 texture families",
            ));
        }
        data.textures.push(Texture {
            start: (data.textures.len() * 16) as u32,
            count: 16,
            size: 4,
            magic: 0,
            name: (*name).to_owned(),
        });
        added.push((*name).to_owned());
    }
    if !added.is_empty() {
        map.get_mut("BlendTileData")?.payload = data.encode()?;
    }
    Ok(added)
}

fn referenced(indices: impl Iterator<Item = u16>, count: usize, kind: &str) -> Result<Vec<bool>> {
    let mut used = vec![false; count];
    for i in indices.filter(|&i| i != 0) {
        *used
            .get_mut(i as usize - 1)
            .ok_or_else(|| err(format!("missing terrain {kind} reference {i}")))? = true;
    }
    Ok(used)
}
fn tile(d: &[u8]) -> u32 {
    u32::from_le_bytes(d[..4].try_into().unwrap())
}
fn compact_descriptions(
    desc: &[Vec<u8>],
    used: &[bool],
    tiles: &[Option<u32>],
) -> Result<(Vec<Vec<u8>>, Vec<u16>)> {
    let mut result = Vec::new();
    let mut lookup = BTreeMap::new();
    let mut remap = vec![0; desc.len() + 1];
    for (i, d) in desc.iter().enumerate().filter(|(i, _)| used[*i]) {
        let mut d = d.clone();
        let mapped = tiles
            .get(tile(&d) as usize)
            .copied()
            .flatten()
            .ok_or_else(|| err("description texture was removed"))?;
        d[..4].copy_from_slice(&mapped.to_le_bytes());
        let id = if let Some(&id) = lookup.get(&d) {
            id
        } else {
            let id = u16::try_from(result.len() + 1)
                .map_err(|_| err("too many distinct terrain blends"))?;
            lookup.insert(d.clone(), id);
            result.push(d);
            id
        };
        remap[i + 1] = id;
    }
    Ok((result, remap))
}

struct Usage {
    blends: Vec<bool>,
    cliffs: Vec<bool>,
    tiles: Vec<bool>,
}

fn usage(d: &Data) -> Result<Usage> {
    let used_blends = referenced(
        d.planes[1].iter().chain(&d.planes[2]).copied(),
        d.blends.len(),
        "blend",
    )?;
    let used_cliffs = referenced(d.planes[3].iter().copied(), d.cliffs.len(), "cliff")?;
    let cells: usize = d.textures.iter().map(|t| t.count as usize).sum();
    let mut used_tiles = vec![false; cells * 4];
    for t in d.planes[0]
        .iter()
        .map(|&v| v as u32)
        .chain(
            d.blends
                .iter()
                .zip(&used_blends)
                .filter(|(_, used)| **used)
                .map(|(b, _)| tile(b)),
        )
        .chain(
            d.cliffs
                .iter()
                .zip(&used_cliffs)
                .filter(|(_, used)| **used)
                .map(|(b, _)| tile(b)),
        )
    {
        *used_tiles
            .get_mut(t as usize)
            .ok_or_else(|| err(format!("terrain references missing texture tile {t}")))? = true;
    }
    Ok(Usage {
        blends: used_blends,
        cliffs: used_cliffs,
        tiles: used_tiles,
    })
}

/// Compact both blend planes and cliff UV mappings, including references to
/// secondary-only textures. Remove every unused family and remap the retained
/// references together; the resulting table must fit the engine limit.
/// All blend flags, phases and bitmap bytes survive.
pub fn compact(map: &mut MapFile) -> Result<Report> {
    let mut d = Data::parse(map)?;
    let before = (d.textures.len(), d.blends.len(), d.cliffs.len());
    let used = usage(&d)?;
    let used_blends = used.blends;
    let used_cliffs = used.cliffs;
    let used_tiles = used.tiles;
    let family_used: Vec<_> = d
        .textures
        .iter()
        .map(|t| {
            used_tiles[t.start as usize * 4..(t.start + t.count) as usize * 4]
                .iter()
                .any(|v| *v)
        })
        .collect();
    if family_used.iter().filter(|used| **used).count() > MAX_TEXTURES {
        return Err(err(format!(
            "terrain uses more than {MAX_TEXTURES} texture families; replace a painted texture before publishing"
        )));
    }
    let cells = used_tiles.len() / 4;
    let mut remap = vec![None; cells * 4];
    let mut textures = Vec::new();
    let mut unused_textures = 0;
    let mut start = 0;
    for (index, t) in d.textures.iter().enumerate() {
        let range = t.start as usize * 4..(t.start + t.count) as usize * 4;
        if !used_tiles[range.clone()].iter().any(|v| *v) {
            unused_textures += 1;
        }
        if !family_used[index] {
            continue;
        }
        for (offset, i) in range.enumerate() {
            remap[i] = Some(start * 4 + offset as u32);
        }
        let mut t = t.clone();
        t.start = start;
        start += t.count;
        textures.push(t);
    }
    let (blends, blend_remap) = compact_descriptions(&d.blends, &used_blends, &remap)?;
    let (cliffs, cliff_remap) = compact_descriptions(&d.cliffs, &used_cliffs, &remap)?;
    for v in &mut d.planes[0] {
        *v = u16::try_from(remap[*v as usize].ok_or_else(|| err("base texture was removed"))?)?;
    }
    for plane in &mut d.planes[1..3] {
        for v in plane {
            *v = blend_remap[*v as usize];
        }
    }
    for v in &mut d.planes[3] {
        *v = cliff_remap[*v as usize];
    }
    d.textures = textures;
    d.blends = blends;
    d.cliffs = cliffs;
    let payload = d.encode()?;
    let changed = payload != map.get("BlendTileData")?.payload;
    let report = Report {
        textures_before: before.0,
        textures_after: d.textures.len(),
        unused_textures,
        removed_textures: before.0 - d.textures.len(),
        blends_before: before.1,
        blends_after: d.blends.len(),
        cliffs_before: before.2,
        cliffs_after: d.cliffs.len(),
        changed,
    };
    map.get_mut("BlendTileData")?.payload = payload;
    Ok(report)
}

pub fn validate_references(map: &MapFile) -> Result<()> {
    // Validate both blend planes and cliff references directly. Publication
    // checks need no full-map clone, table compaction or re-encoding.
    usage(&Data::parse(map)?).map(|_| ())
}

pub fn validate_texture_limit(map: &MapFile) -> Result<()> {
    let count = Data::parse(map)?.textures.len();
    if count > MAX_TEXTURES {
        return Err(err(format!(
            "terrain texture count {count} exceeds the WorldBuilder limit of {MAX_TEXTURES}; repair-terrain-package before publishing"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::map::{Chunk, HeightMap};

    fn fixture() -> MapFile {
        let mut blend = vec![0; 18];
        blend[..4].copy_from_slice(&65_u32.to_le_bytes());
        blend[4..].copy_from_slice(&[1, 0, 1, 0, 3, 1, 255, 255, 255, 255, 0, 0, 218, 122]);
        let mut unused = blend.clone();
        unused[..4].copy_from_slice(&129_u32.to_le_bytes());
        let mut cliff = vec![0; 38];
        cliff[..4].copy_from_slice(&130_u32.to_le_bytes());
        for (i, v) in cliff[4..].iter_mut().enumerate() {
            *v = i as u8;
        }
        let data = Data {
            planes: [
                vec![0, 1, 2, 3, 4, 5],
                vec![1, 2, 0, 1, 0, 0],
                vec![0, 2, 0, 0, 0, 0],
                vec![0, 0, 1, 2, 0, 0],
            ],
            bitmaps: (0..(8 * 2 + 6)).map(|i| i as u8).collect(),
            textures: (0..4)
                .map(|i| Texture {
                    start: i * 16,
                    count: 16,
                    size: 4,
                    magic: 0,
                    name: format!("Texture{i}"),
                })
                .collect(),
            magic: vec![205, 205, 205, 205, 0, 0, 0, 0],
            blends: vec![blend.clone(), blend, unused],
            cliffs: vec![cliff.clone(), cliff],
        };
        let hm = HeightMap {
            width: 3,
            height: 2,
            border: 0,
            borders: vec![],
            elevations: vec![100; 6],
        };
        MapFile {
            order: vec![("BlendTileData".into(), 2), ("HeightMapData".into(), 1)],
            table: [(1, "HeightMapData".into()), (2, "BlendTileData".into())].into(),
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 27,
                    payload: data.encode().unwrap(),
                },
            ],
        }
    }

    #[test]
    fn material_extension_preserves_all_planes_masks_and_existing_family_ids() {
        let mut map = fixture();
        let original = Data::parse(&map).unwrap();
        let names = crate::terrain::materials::STOCK.map(|r| r[0]);
        assert_eq!(
            ensure_materials(&mut map, &names).unwrap().len(),
            names.len()
        );
        let after = Data::parse(&map).unwrap();
        assert_eq!(after.planes, original.planes);
        assert_eq!(after.bitmaps, original.bitmaps);
        assert_eq!(after.blends, original.blends);
        assert_eq!(after.cliffs, original.cliffs);
        for (before, after) in original.textures.iter().zip(&after.textures) {
            assert_eq!((before.start, &before.name), (after.start, &after.name));
        }
        let once = map.encode().unwrap();
        assert!(ensure_materials(&mut map, &names).unwrap().is_empty());
        assert_eq!(map.encode().unwrap(), once);
    }

    #[test]
    fn full_atlas_extension_rejects_without_partially_changing_the_map() {
        let mut map = fixture();
        let names: Vec<_> = (0..59).map(|i| format!("Extra{i}")).collect();
        ensure_materials(
            &mut map,
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .unwrap();
        let before = map.encode().unwrap();
        assert!(ensure_materials(&mut map, &["LastSlot", "Overflow"]).is_err());
        assert_eq!(map.encode().unwrap(), before);
    }

    #[test]
    fn compaction_preserves_secondary_three_way_cliff_uvs_and_row_padded_bitmaps() {
        let mut m = fixture();
        let original = m.clone();
        let a = Data::parse(&m).unwrap();
        let r = compact(&mut m).unwrap();
        assert_eq!(
            (
                r.blends_before,
                r.blends_after,
                r.cliffs_before,
                r.cliffs_after
            ),
            (3, 1, 2, 1)
        );
        assert_eq!(r.unused_textures, 1);
        assert_eq!(
            (r.textures_before, r.textures_after, r.removed_textures),
            (4, 3, 1)
        );
        let b = Data::parse(&m).unwrap();
        assert_eq!(a.bitmaps, b.bitmaps);
        assert_eq!(a.planes[0], b.planes[0]);
        assert_eq!(
            a.textures
                .iter()
                .take(3)
                .map(|t| &t.name)
                .collect::<Vec<_>>(),
            b.textures.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        for plane in [1, 2] {
            for i in 0..6 {
                let old = a.planes[plane][i];
                let new = b.planes[plane][i];
                assert_eq!(old == 0, new == 0);
                if old > 0 {
                    assert_eq!(a.blends[old as usize - 1], b.blends[new as usize - 1]);
                }
            }
        }
        for i in 0..6 {
            let old = a.planes[3][i];
            let new = b.planes[3][i];
            if old > 0 {
                assert_eq!(a.cliffs[old as usize - 1], b.cliffs[new as usize - 1]);
            }
        }
        assert_eq!(
            m.get("HeightMapData").unwrap(),
            original.get("HeightMapData").unwrap()
        );
        let saved = m.get("BlendTileData").unwrap().clone();
        assert!(!compact(&mut m).unwrap().changed);
        assert_eq!(&saved, m.get("BlendTileData").unwrap());
    }

    #[test]
    fn over_limit_removes_only_unused_family_and_preserves_every_reference() {
        let mut m = fixture();
        let mut data = Data::parse(&m).unwrap();
        data.textures = (0..50)
            .map(|i| Texture {
                start: i * 16,
                count: 16,
                size: 4,
                magic: 0,
                name: format!("Texture{i}"),
            })
            .collect();
        data.planes = std::array::from_fn(|_| vec![0; 50]);
        for i in 0..50 {
            data.planes[0][i] = if i == 1 { 0 } else { i as u16 * 64 + 3 };
        }
        data.planes[1][0] = 1;
        data.planes[2][1] = 1;
        data.planes[3][2] = 1;
        data.blends[0][..4].copy_from_slice(&(49_u32 * 64 + 1).to_le_bytes());
        data.cliffs[0][..4].copy_from_slice(&(48_u32 * 64 + 2).to_le_bytes());
        data.bitmaps = (0..8 * 50_usize.div_ceil(8) + 50)
            .map(|i| i as u8)
            .collect();
        let hm = HeightMap {
            width: 50,
            height: 1,
            border: 0,
            borders: vec![],
            elevations: vec![100; 50],
        };
        m.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        m.get_mut("BlendTileData").unwrap().payload = data.encode().unwrap();
        assert!(validate_texture_limit(&m).is_err());
        let report = compact(&mut m).unwrap();
        assert_eq!((report.textures_after, report.removed_textures), (49, 1));
        validate_texture_limit(&m).unwrap();
        let after = Data::parse(&m).unwrap();
        assert_eq!(data.bitmaps, after.bitmaps);
        assert!(!after.textures.iter().any(|t| t.name == "Texture1"));
        for (&old, &new) in data.planes[0].iter().zip(&after.planes[0]) {
            assert_eq!(
                (&data.textures[old as usize / 64].name, old % 64),
                (&after.textures[new as usize / 64].name, new % 64)
            );
        }
        for plane in [1, 2, 3] {
            let (old_desc, new_desc) = if plane == 3 {
                (&data.cliffs, &after.cliffs)
            } else {
                (&data.blends, &after.blends)
            };
            for (&old, &new) in data.planes[plane].iter().zip(&after.planes[plane]) {
                assert_eq!(old == 0, new == 0);
                if old == 0 {
                    continue;
                }
                let a = &old_desc[old as usize - 1];
                let b = &new_desc[new as usize - 1];
                assert_eq!(
                    (&data.textures[tile(a) as usize / 64].name, tile(a) % 64),
                    (&after.textures[tile(b) as usize / 64].name, tile(b) % 64)
                );
                assert_eq!(&a[4..], &b[4..]);
            }
        }
        let saved = m.encode().unwrap();
        assert!(!compact(&mut m).unwrap().changed);
        assert_eq!(m.encode().unwrap(), saved);

        data.planes[0][1] = 64;
        m.get_mut("BlendTileData").unwrap().payload = data.encode().unwrap();
        let saved = m.encode().unwrap();
        assert!(compact(&mut m).unwrap_err().to_string().contains("49"));
        assert_eq!(m.encode().unwrap(), saved);
    }

    #[test]
    fn dangling_secondary_or_cliff_reference_is_rejected_without_mutation() {
        for plane in [1, 2, 3] {
            let mut m = fixture();
            let mut data = Data::parse(&m).unwrap();
            data.planes[plane][0] = 100;
            m.get_mut("BlendTileData").unwrap().payload = data.encode().unwrap();
            let original = m.clone();
            assert!(compact(&mut m).is_err());
            assert_eq!(m.chunks, original.chunks);
        }
        let mut m = fixture();
        let mut data = Data::parse(&m).unwrap();
        data.blends[0][..4].copy_from_slice(&9999_u32.to_le_bytes());
        m.get_mut("BlendTileData").unwrap().payload = data.encode().unwrap();
        assert!(
            compact(&mut m)
                .unwrap_err()
                .to_string()
                .contains("missing texture tile")
        );
    }
}
