//! Replace a previous mountain edit with broad, rounded reference ridges.
use super::*;

fn bounds(m: &Value, n: usize, border: usize) -> Result<[usize; 4]> {
    let v = m["bounds"]
        .as_array()
        .filter(|v| v.len() == 4)
        .ok_or_else(|| err("invalid mountain bounds"))?;
    let mut b = [0; 4];
    for (i, v) in v.iter().enumerate() {
        b[i] = v
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| err("invalid mountain bound"))?;
    }
    if b[0] < border
        || b[1] < border
        || b[2] >= n.saturating_sub(border)
        || b[3] >= n.saturating_sub(border)
        || b[2] <= b[0] + 8
        || b[3] <= b[1] + 8
    {
        return Err(err("mountain bounds must lie inside the playable map"));
    }
    Ok(b)
}

fn append_materials(map: &mut MapFile, n: usize, names: &[Value]) -> Result<()> {
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let tail = 4 + 8 * n * n + 8 * n * n.div_ceil(8) + n * n;
    let mut r = crate::formats::binary::Reader::new(&map.get("BlendTileData")?.payload[tail..]);
    let cells = r.u32()?;
    r.take(8)?;
    let count = r.u32()? as usize;
    for i in 0..count {
        if r.u32()? != (i * 16) as u32 || r.u32()? != 16 || r.u32()? != 4 || r.u32()? != 0 {
            return Err(err(
                "mountain material extension requires stock texture families",
            ));
        }
        r.s16()?;
    }
    let position = tail + r.pos;
    let mut extra = crate::formats::binary::Writer::new();
    let mut textures = bt.textures;
    for name in names {
        let name = name
            .as_str()
            .ok_or_else(|| err("invalid mountain material"))?;
        if textures.iter().any(|n| n == name) {
            continue;
        }
        extra.u32(u32::try_from(textures.len() * 16)?);
        extra.u32(16);
        extra.u32(4);
        extra.u32(0);
        extra.s16(name)?;
        textures.push(name.to_owned());
    }
    if textures.len() > 64 {
        return Err(err("terrain atlas exceeds 64 families"));
    }
    let data = &mut map.get_mut("BlendTileData")?.payload;
    data[tail..tail + 4]
        .copy_from_slice(&(cells + (textures.len() - count) as u32 * 16).to_le_bytes());
    data[tail + 12..tail + 16].copy_from_slice(&(textures.len() as u32).to_le_bytes());
    data.splice(position..position, extra.data);
    Ok(())
}

fn soften(values: &[f64], w: usize, h: usize, sigma: f64) -> Vec<f64> {
    let radius = (sigma * 3.).ceil() as isize;
    let mut weights: Vec<_> = (-radius..=radius)
        .map(|d| (-(d * d) as f64 / (2. * sigma * sigma)).exp())
        .collect();
    let sum: f64 = weights.iter().sum();
    for w in &mut weights {
        *w /= sum;
    }
    let mut a = vec![0.; w * h];
    let mut b = a.clone();
    for y in 0..h {
        for x in 0..w {
            for (j, k) in weights.iter().enumerate() {
                let xx = (x as isize + j as isize - radius).clamp(0, w as isize - 1) as usize;
                a[y * w + x] += values[y * w + xx] * k;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            for (j, k) in weights.iter().enumerate() {
                let yy = (y as isize + j as isize - radius).clamp(0, h as isize - 1) as usize;
                b[y * w + x] += a[yy * w + x] * k;
            }
        }
    }
    b
}

pub(super) fn apply(
    p: &Project,
    map: &mut MapFile,
    source_name: &str,
    plan: &Value,
) -> Result<Polished> {
    if plan["source"].as_str() != Some(source_name) {
        return Err(err("mountain plan does not match the source release"));
    }
    let source = map.clone();
    let mut hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err("mountain rework requires square v27 terrain"));
    }
    let names = plan["rock_textures"]
        .as_array()
        .ok_or_else(|| err("missing rock texture names"))?;
    append_materials(map, n, names)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let old_bt = BlendTiles::parse(source.get("BlendTileData")?, n, n)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let layout_path = p.release_assets(source_name)?.join("source/layout.json");
    let saved: Layout = serde_json::from_slice(&fs::read(&layout_path).map_err(|e| {
        err(format!(
            "mountain source layout {}: {e}",
            layout_path.display()
        ))
    })?)?;
    let (layout, _) = crate::coordinates::native_layout(p, map, &saved)?;
    let policy = crate::pipeline::coordinate_pipeline::spatial_policy(p)?;
    let old_scene = crate::terrain::spatial::Scene::load(p, map, policy.clone())?;
    let mut locks = protected(p, &objects, n);
    let gifts = crate::gameplay::gift_clearance::reservations(p, map)?;
    for &([cx, cy], radius) in &gifts.footprints {
        let radius = radius + 2;
        for y in cy.saturating_sub(radius)..=(cy + radius).min(n - 1) {
            for x in cx.saturating_sub(radius)..=(cx + radius).min(n - 1) {
                locks[y * n + x] = true;
            }
        }
    }
    for e in &old_scene.entities {
        // Trees and rocks follow the new ground. Freezing their old elevation
        // creates grass-lined craters in the middle of the ridge.
        if e.reserve == 0. || e.class == "decoration" {
            continue;
        }
        for (i, lock) in locks.iter_mut().enumerate() {
            if ((i % n) as f64 - e.point[0]).hypot((i / n) as f64 - e.point[1]) <= e.reserve + 2. {
                *lock = true;
            }
        }
    }
    for (i, lock) in locks.iter_mut().enumerate() {
        if layout
            .ramps
            .iter()
            .any(|r| contains(r, (i % n) as f64, (i / n) as f64, 7.))
        {
            *lock = true;
        }
    }
    if let Some(bytes) = crate::terrain::resource_routes::input(p)? {
        let distribution: Value = serde_json::from_slice(&bytes)?;
        for route in distribution["routes"]
            .as_array()
            .ok_or_else(|| err("missing resource routes"))?
        {
            for key in ["main_path", "secondary_path"] {
                for point in route[key]
                    .as_array()
                    .ok_or_else(|| err("missing resource path"))?
                {
                    let x = point[0].as_u64().ok_or_else(|| err("invalid route x"))? as usize;
                    let y = point[1].as_u64().ok_or_else(|| err("invalid route y"))? as usize;
                    if x >= n || y >= n {
                        return Err(err("resource path outside map"));
                    }
                    for yy in y.saturating_sub(2)..=(y + 2).min(n - 1) {
                        for xx in x.saturating_sub(2)..=(x + 2).min(n - 1) {
                            locks[yy * n + xx] = true;
                        }
                    }
                }
            }
        }
    }
    let clearance = terrain::distance(&locks, n);
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|h| *h as f64 * scale)
        .collect();
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    let mut imp = terrain::flip(&bt.impassability, n);
    let mut build = terrain::flip(&bt.buildability, n);
    let name = plan["restore_source"]
        .as_str()
        .ok_or_else(|| err("missing terrain restore source"))?;
    let restore = MapFile::load(p.release_assets(name)?.join(format!("{name}.map")))?;
    let rh = HeightMap::parse(restore.get("HeightMapData")?)?;
    if rh.width != n || rh.height != n || rh.border != hm.border {
        return Err(err("restore frame differs"));
    }
    let rb = BlendTiles::parse(restore.get("BlendTileData")?, n, n)?;
    if !bt.textures.starts_with(&rb.textures) {
        return Err(err("restore material IDs differ"));
    }
    let rraw = terrain::flip(&rh.elevations, n);
    let rt = terrain::flip(&rb.tiles, n);
    let ri = terrain::flip(&rb.impassability, n);
    let rbuild = terrain::flip(&rb.buildability, n);
    let mountains = plan["mountains"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| err("missing mountains"))?;
    let mut ground = before.clone();
    let mut changed = vec![false; n * n];
    let mut footprint = changed.clone();
    let mut restored = 0;
    for m in mountains {
        let b = bounds(m, n, p.usize("BORDER", 20) + 2)?;
        for y in b[1]..=b[3] {
            for x in b[0]..=b[2] {
                let i = y * n + x;
                if !locks[i] && rraw[i] as f64 * scale != before[i] {
                    ground[i] = rraw[i] as f64 * scale;
                    tiles[i] = rt[i];
                    imp[i] = ri[i];
                    build[i] = rbuild[i];
                    changed[i] = true;
                    restored += 1;
                }
            }
        }
    }
    let mut z = ground.clone();
    let texture = |name: &str| {
        bt.textures
            .iter()
            .position(|t| t == name)
            .map(|i| i as u16)
            .ok_or_else(|| err(format!("missing material {name}")))
    };
    let gray = texture("Rock_Yucatan01")?;
    let brown = texture("Rock_Yucatan03")?;
    let gray_alt = texture("Rock_Yucatan02")?;
    let mut rows = Vec::new();
    for mountain in mountains {
        let b = bounds(mountain, n, p.usize("BORDER", 20) + 2)?;
        let size = mountain["relief_size"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or_else(|| err("invalid relief size"))?;
        let w = size[0]
            .as_u64()
            .ok_or_else(|| err("invalid relief width"))? as usize;
        let h = size[1]
            .as_u64()
            .ok_or_else(|| err("invalid relief height"))? as usize;
        let bytes = fs::read(
            p.path(
                mountain["relief_file"]
                    .as_str()
                    .ok_or_else(|| err("missing relief file"))?,
            )?,
        )?;
        if w < 2 || h < 2 || w.checked_mul(h).and_then(|v| v.checked_mul(2)) != Some(bytes.len()) {
            return Err(err("invalid relief length"));
        }
        let values: Vec<_> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| {
                (u16::from_le_bytes(*b) as f64 * number(plan, "relief_z_scale", scale)
                    - number(plan, "relief_base", 200.))
                .max(0.)
            })
            .collect();
        let sigma = number(mountain, "smoothing", 1.8);
        if !(0.1..=5.0).contains(&sigma) {
            return Err(err("invalid smoothing radius"));
        }
        let relief = soften(&values, w, h, sigma);
        let materials = fs::read(
            p.path(
                mountain["material_file"]
                    .as_str()
                    .ok_or_else(|| err("missing material file"))?,
            )?,
        )?;
        if materials.len() != w * h || materials.iter().any(|v| *v > 3) {
            return Err(err("invalid material stamps"));
        }
        let amplitude = number(mountain, "height_scale", 0.85);
        if !(0.1..=2.0).contains(&amplitude) {
            return Err(err("invalid mountain scale"));
        }
        let mut count = 0;
        let mut peak = 0_f64;
        let mut rise = 0_f64;
        let mut brown_cells = 0;
        for y in b[1]..=b[3] {
            for x in b[0]..=b[2] {
                let i = y * n + x;
                let mut u = (x - b[0]) as f64 / (b[2] - b[0]) as f64;
                let v = (y - b[1]) as f64 / (b[3] - b[1]) as f64;
                if mountain["mirror"] == true {
                    u = 1. - u;
                }
                let sx = u * (w - 1) as f64;
                let sy = v * (h - 1) as f64;
                let ix = sx.floor() as usize;
                let iy = sy.floor() as usize;
                let jx = (ix + 1).min(w - 1);
                let jy = (iy + 1).min(h - 1);
                let ax = sx - ix as f64;
                let ay = sy - iy as f64;
                let value = (relief[iy * w + ix] * (1. - ax) + relief[iy * w + jx] * ax)
                    * (1. - ay)
                    + (relief[jy * w + ix] * (1. - ax) + relief[jy * w + jx] * ax) * ay;
                let edge = (x - b[0]).min(b[2] - x).min(y - b[1]).min(b[3] - y) as f64;
                let value =
                    value * amplitude * smooth(edge / 6.) * smooth((clearance[i] - 2.) / 7.);
                if value < 2. || ground[i] <= p.number("WATER_Z", 200.) {
                    continue;
                }
                z[i] = ((ground[i] + value) / scale).round_ties_even() * scale;
                if z[i] / scale > u16::MAX as f64 {
                    return Err(err("mountain height exceeds native range"));
                }
                let family = match materials[iy * w + ix] {
                    2 => {
                        brown_cells += 1;
                        brown
                    }
                    3 => gray_alt,
                    0 if value < 18. => tiles[i] / 64,
                    _ => gray,
                };
                tiles[i] = family * 64 + tiles[i] % 64;
                imp[i] = true;
                build[i] = true;
                changed[i] = true;
                footprint[i] = true;
                peak = peak.max(z[i]);
                rise = rise.max(z[i] - ground[i]);
                count += 1;
            }
        }
        if count < 100 || rise < 100. {
            return Err(err("protected footprints clipped the reference mountain"));
        }
        rows.push(json!({"id":mountain["id"],"bounds":b,"mountain_cells":count,"peak_height":peak,"maximum_rise":rise,"brown_rock_cells":brown_cells,"reference_crop":mountain["reference_crop"]}));
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even() as u16)
        .collect();
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    let checked_hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let checked = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    for i in 0..n * n {
        let j = (n - 1 - i / n) * n + i % n;
        if (!changed[i] || locks[i])
            && (z[i] != before[i]
                || checked.impassability[j] != old_bt.impassability[j]
                || checked.buildability[j] != old_bt.buildability[j])
        {
            return Err(err("mountain rework changed protected ground"));
        }
        if footprint[i] && (!checked.impassability[j] || !checked.buildability[j]) {
            return Err(err("mountain contains an open cell"));
        }
    }
    let start = 4 + 8 * n * n;
    let len = n * n.div_ceil(8);
    for mask in [1, 2, 3, 4, 6, 7] {
        let r = start + mask * len..start + (mask + 1) * len;
        if source.get("BlendTileData")?.payload[r.clone()] != map.get("BlendTileData")?.payload[r] {
            return Err(err("mountain rework changed another navigation bitmap"));
        }
    }
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if !["HeightMapData", "BlendTileData"].contains(&name.as_str()) && map.get(name)? != chunk {
            return Err(err(format!("mountain changed unrelated chunk {name}")));
        }
    }
    crate::gameplay::script_spawns::validate(p, map, &checked_hm, &checked, &objects)?;
    if gifts.guarded {
        crate::gameplay::gift_clearance::validate(p, map)?;
    }
    crate::gameplay::production_facing::validate(p, map)?;
    crate::terrain::resource_routes::validate(p, map)?;
    let inspect = |s: &crate::terrain::spatial::Scene, m: &MapFile| -> Result<_> {
        let mut issues = s.inspect(&layout)?;
        let (_, r) = s.ramp_profiles(&layout)?;
        issues.extend(r);
        let (_, r) = s.near_ores(p, &layout)?;
        issues.extend(r);
        let (routes, r) = s.routes(m)?;
        issues.extend(r);
        Ok((issues, routes.len()))
    };
    let (old_issues, _) = inspect(&old_scene, &source)?;
    let new_scene = crate::terrain::spatial::Scene::load(p, map, policy)?;
    let (issues, routes) = inspect(&new_scene, map)?;
    let regressions = crate::terrain::spatial::regressions(&issues, &old_issues);
    if !regressions.is_empty() {
        return Err(err(format!(
            "mountain rework worsened spatial checks: {}",
            serde_json::to_string(&regressions)?
        )));
    }
    let report = json!({"operation":"add-mountains","profile":"44gg-rounded-ridge","source":source_name,"plan":plan,"mountains":rows,"restored_old_mountain_cells":restored,"mountain_cells":footprint.iter().filter(|v|**v).count(),"changed_height_cells":z.iter().zip(&before).filter(|(a,b)|a!=b).count(),"all_mountain_cells_ground_blocked":true,"unrelated_navigation_bitmaps_preserved":true,"protected_footprints_preserved":true,"resource_distribution_preserved":true,"source_issues":old_issues.len(),"result_issues":issues.len(),"regressions":regressions,"routes":routes,"added_textures":names,"ingame_test":"not run"});
    println!(
        "重做 {} 处山体；恢复旧山格 {restored}，资源路线 {routes}；空间检查无新增/恶化",
        rows.len()
    );
    let slope = terrain::gradient(&z, n);
    Ok(Polished {
        terrain: Terrain {
            n,
            raw,
            land: z.iter().map(|h| *h > p.number("WATER_Z", 200.)).collect(),
            z,
            slope,
            layout,
        },
        report,
        before,
        before_tiles,
        tiles,
        imp,
        build,
        textures: bt.textures,
        example: 0,
    })
}
