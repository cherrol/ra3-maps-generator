//! Stamp authored mountain relief into a finished map, preserving gameplay assets.
use super::*;
mod rework;

pub fn add_mountains(
    p: &Project,
    map: &mut MapFile,
    source_name: &str,
    plan: &Value,
) -> Result<Polished> {
    if plan["restore_source"].is_string() {
        return rework::apply(p, map, source_name, plan);
    }
    if plan["source"].as_str() != Some(source_name) {
        return Err(err("mountain plan does not match the source release"));
    }
    let source = map.clone();
    let mut hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err("mountain editing requires matching square v27 terrain"));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|h| f64::from(*h) * scale)
        .collect();
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let saved: Layout = serde_json::from_slice(&fs::read(
        p.release_assets(source_name)?.join("source/layout.json"),
    )?)?;
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
        if e.reserve == 0. {
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
    let clearance = terrain::distance(&locks, n);
    let dims = plan["relief_size"]
        .as_array()
        .ok_or_else(|| err("missing relief dimensions"))?;
    if dims.len() != 2 {
        return Err(err("invalid relief dimensions"));
    }
    let w = dims[0]
        .as_u64()
        .ok_or_else(|| err("invalid relief width"))? as usize;
    let h = dims[1]
        .as_u64()
        .ok_or_else(|| err("invalid relief height"))? as usize;
    let bytes = fs::read(
        p.path(
            plan["relief_file"]
                .as_str()
                .ok_or_else(|| err("missing relief file"))?,
        )?,
    )?;
    if w < 2 || h < 2 || w.checked_mul(h).and_then(|v| v.checked_mul(2)) != Some(bytes.len()) {
        return Err(err("relief byte length does not match dimensions"));
    }
    let relief: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| {
            (f64::from(u16::from_le_bytes([b[0], b[1]])) * number(plan, "relief_z_scale", scale)
                - number(plan, "relief_base", 210.))
            .max(0.)
        })
        .collect();
    let mut z = before.clone();
    let mut changed = vec![false; n * n];
    let mut rows = Vec::new();
    for mountain in plan["mountains"]
        .as_array()
        .ok_or_else(|| err("missing mountains"))?
    {
        let bounds = mountain["bounds"]
            .as_array()
            .ok_or_else(|| err("missing mountain bounds"))?;
        if bounds.len() != 4 {
            return Err(err("invalid mountain bounds"));
        }
        let b: Vec<_> = bounds
            .iter()
            .map(|v| {
                v.as_u64()
                    .map(|v| v as usize)
                    .ok_or_else(|| err("invalid mountain bound"))
            })
            .collect::<Result<_>>()?;
        let border = p.usize("BORDER", 20) + 2;
        if b[0] < border
            || b[1] < border
            || b[2] + border >= n
            || b[3] + border >= n
            || b[2] <= b[0] + 8
            || b[3] <= b[1] + 8
        {
            return Err(err("mountain bounds must lie inside the playable map"));
        }
        let amplitude = number(mountain, "height_scale", 1.);
        if !(0.1..=4.).contains(&amplitude) {
            return Err(err("invalid mountain height scale"));
        }
        let mut count = 0;
        let mut peak = 0_f64;
        let mut rise = 0_f64;
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
                let edge = ((x - b[0]).min(b[2] - x).min(y - b[1]).min(b[3] - y)) as f64;
                let value =
                    value * amplitude * smooth(edge / 4.) * smooth((clearance[i] - 2.) / 7.);
                if value < 2. || before[i] < p.number("WATER_Z", 200.) {
                    continue;
                }
                let raw = ((before[i] + value) / scale).round_ties_even();
                if raw > u16::MAX as f64 {
                    return Err(err("mountain height exceeds native range"));
                }
                z[i] = z[i].max(raw * scale);
                changed[i] = true;
                count += 1;
                peak = peak.max(z[i]);
                rise = rise.max(z[i] - before[i]);
            }
        }
        if count < 100 || rise < 100. {
            return Err(err("mountain was clipped by protected footprints"));
        }
        rows.push(json!({"id":mountain["id"],"bounds":b,"changed_cells":count,"peak_height":peak,"maximum_rise":rise}));
    }
    let mut imp = terrain::flip(&bt.impassability, n);
    let mut build = terrain::flip(&bt.buildability, n);
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    let rock = bt
        .textures
        .iter()
        .position(|t| t == "Rock_Yucatan01")
        .ok_or_else(|| err("source texture atlas has no mountain rock"))? as u16;
    for i in 0..n * n {
        if changed[i] {
            imp[i] = true;
            build[i] = true;
            tiles[i] = rock * 64 + tiles[i] % 64;
        }
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even() as u16)
        .collect();
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    let checked_hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let checked_bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    for i in 0..n * n {
        let game = (n - 1 - i / n) * n + i % n;
        if (!changed[i] || locks[i])
            && (z[i] != before[i]
                || checked_bt.impassability[game] != bt.impassability[game]
                || checked_bt.buildability[game] != bt.buildability[game])
        {
            return Err(err(
                "mountain edit changed ground outside its permitted footprint",
            ));
        }
        if changed[i] && (!checked_bt.impassability[game] || !checked_bt.buildability[game]) {
            return Err(err("mountain contains a traversable or buildable cell"));
        }
    }
    let offset = 4 + 8 * n * n;
    let mask_len = n * n.div_ceil(8);
    for mask in [1, 2, 3, 4, 6, 7] {
        let range = offset + mask * mask_len..offset + (mask + 1) * mask_len;
        if source.get("BlendTileData")?.payload[range.clone()]
            != map.get("BlendTileData")?.payload[range]
        {
            return Err(err("mountain edit changed an unrelated navigation bitmap"));
        }
    }
    for c in &source.chunks {
        let name = &source.table[&c.name_idx];
        if !["HeightMapData", "BlendTileData"].contains(&name.as_str()) && map.get(name)? != c {
            return Err(err(format!("mountain edit changed unrelated chunk {name}")));
        }
    }
    crate::gameplay::script_spawns::validate_with_layout(
        p,
        map,
        &checked_hm,
        &checked_bt,
        &objects,
        &layout,
    )?;
    if gifts.guarded {
        crate::gameplay::gift_clearance::validate_with_layout(p, map, &layout)?;
    }
    crate::gameplay::production_facing::validate(p, map)?;
    let new_scene = crate::terrain::spatial::Scene::load(p, map, policy)?;
    let inspect = |scene: &crate::terrain::spatial::Scene, map: &MapFile| -> Result<_> {
        let mut issues = scene.inspect(&layout)?;
        let (_, r) = scene.ramp_profiles(&layout)?;
        issues.extend(r);
        let (_, r) = scene.near_ores(p, &layout)?;
        issues.extend(r);
        let (routes, r) = scene.routes(map)?;
        issues.extend(r);
        Ok((issues, routes.len()))
    };
    let (old_issues, _) = inspect(&old_scene, &source)?;
    let (new_issues, routes) = inspect(&new_scene, map)?;
    let regressions = crate::terrain::spatial::regressions(&new_issues, &old_issues);
    if !regressions.is_empty() {
        return Err(err(format!(
            "mountain edit introduced spatial regressions: {}",
            serde_json::to_string(&regressions)?
        )));
    }
    let slope = terrain::gradient(&z, n);
    let report = json!({"operation":"add-mountains","source":source_name,"plan":plan,"mountains":rows,
        "changed_cells":changed.iter().filter(|v| **v).count(),"all_mountain_cells_ground_blocked":true,
        "unrelated_navigation_bitmaps_preserved":true,"objects_scripts_resources_preserved":true,
        "protected_footprints_preserved":true,"ramps_preserved":true,"routes":routes,
        "source_issues":old_issues.len(),"result_issues":new_issues.len(),"regressions":regressions,"ingame_test":"not run"});
    println!(
        "新增 {} 处高山，{} 格；{} 条资源路线，无新增空间问题",
        rows.len(),
        report["changed_cells"],
        routes
    );
    Ok(Polished {
        terrain: Terrain {
            n,
            raw,
            land: z.iter().map(|h| *h >= p.number("WATER_Z", 200.)).collect(),
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
