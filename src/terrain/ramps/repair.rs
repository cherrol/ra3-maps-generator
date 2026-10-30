//! Apply an authored local shoulder patch while preserving the playable ramp.
use super::*;

fn bounds(plan: &Value, key: &str, n: usize) -> Result<[usize; 4]> {
    let values = plan[key]
        .as_array()
        .ok_or_else(|| err("missing repair bounds"))?;
    if values.len() != 4 {
        return Err(err("repair bounds need four coordinates"));
    }
    let mut b = [0; 4];
    for (out, v) in b.iter_mut().zip(values) {
        *out = v.as_u64().ok_or_else(|| err("invalid repair bound"))? as usize;
    }
    if b[0] < 2 || b[1] < 2 || b[2] >= n - 2 || b[3] >= n - 2 || b[0] >= b[2] || b[1] >= b[3] {
        return Err(err("repair bounds outside terrain"));
    }
    Ok(b)
}

pub(super) fn apply(
    p: &Project,
    map: &mut MapFile,
    source_name: &str,
    plan: &Value,
) -> Result<Polished> {
    if plan["source"].as_str() != Some(source_name) {
        return Err(err("shoulder repair source differs"));
    }
    let source = map.clone();
    let mut hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err("shoulder repair requires matching square v27 terrain"));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let source_dir = p.release_assets(source_name)?;
    let layout: Layout = serde_json::from_slice(&fs::read(source_dir.join("source/layout.json"))?)?;
    let id = plan["ramp_id"]
        .as_u64()
        .ok_or_else(|| err("missing repair ramp ID"))? as usize;
    let index = id
        .checked_sub(layout.objects.len() + 1)
        .ok_or_else(|| err("invalid repair ramp ID"))?;
    let r = layout
        .ramps
        .get(index)
        .ok_or_else(|| err("repair ramp ID outside layout"))?;
    let b = bounds(plan, "bounds", n)?;
    let surface = bounds(plan, "surface_bounds", n)?;
    if b[0] < surface[0] || b[1] < surface[1] || b[2] > surface[2] || b[3] > surface[3] {
        return Err(err("repair surface must contain height patch"));
    }
    for [x, y] in [[surface[0], surface[1]], [surface[2], surface[3]]] {
        if (x as f64 - number(r, "x", 0.)).hypot(y as f64 - number(r, "y", 0.)) > 90. {
            return Err(err("shoulder patch extends beyond local ramp area"));
        }
    }
    let patch = fs::read(
        p.path(
            plan["height_file"]
                .as_str()
                .ok_or_else(|| err("missing shoulder height file"))?,
        )?,
    )?;
    if patch.len() != (b[2] - b[0]) * (b[3] - b[1]) * 2 {
        return Err(err("shoulder height file dimensions differ"));
    }
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|v| *v as f64 * scale)
        .collect();
    let mut z = before.clone();
    let mut locks = protected(p, &objects, n);
    let reservations = crate::gameplay::gift_clearance::reservations(p, map)?;
    for &([cx, cy], pad) in &reservations.footprints {
        let radius = pad + 2;
        for y in cy.saturating_sub(radius)..=(cy + radius).min(n - 1) {
            for x in cx.saturating_sub(radius)..=(cx + radius).min(n - 1) {
                locks[y * n + x] = true;
            }
        }
    }
    let mut changed = vec![false; n * n];
    let mut touched = Vec::new();
    for y in b[1]..b[3] {
        for x in b[0]..b[2] {
            let offset = ((y - b[1]) * (b[2] - b[0]) + x - b[0]) * 2;
            let raw = u16::from_le_bytes([patch[offset], patch[offset + 1]]);
            if raw == u16::MAX {
                continue;
            }
            let i = y * n + x;
            let h = raw as f64 * scale;
            if h == z[i] {
                continue;
            }
            if locks[i] || h < p.number("WATER_Z", 200.) || (h - z[i]).abs() > 150. {
                return Err(err(
                    "shoulder repair touches a protected footprint or changes terrain layers",
                ));
            }
            z[i] = h;
            changed[i] = true;
            touched.push((i, h));
        }
    }
    if touched.is_empty() {
        return Err(err("shoulder patch has no height changes"));
    }
    let metrics = validate_shape(&z, n, r, p.usize("BORDER", 20))
        .ok_or_else(|| err("shoulder repair damages full-width ramp"))?;
    for other in &layout.ramps {
        if validate_shape(&before, n, other, p.usize("BORDER", 20)).is_some()
            && validate_shape(&z, n, other, p.usize("BORDER", 20)).is_none()
        {
            return Err(err("shoulder repair damages another ramp"));
        }
    }
    let old_imp = terrain::flip(&bt.impassability, n);
    let mut imp = masks(&z, &old_imp, n, r, &touched, p);
    let mut build = terrain::flip(&bt.buildability, n);
    let slope = terrain::gradient(&z, n);
    for i in 0..n * n {
        if locks[i] {
            imp[i] = old_imp[i];
        } else if changed[i] {
            build[i] = slope[i] > 1.5;
        }
    }
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    let pick = |name: &str| {
        bt.textures
            .iter()
            .position(|v| v == name)
            .map(|i| i as u16)
            .ok_or_else(|| err(format!("missing shoulder texture {name}")))
    };
    let rock = pick(
        plan["rock_texture"]
            .as_str()
            .ok_or_else(|| err("missing shoulder rock texture"))?,
    )?;
    let repaint: Vec<&str> = plan["repaint_materials"]
        .as_array()
        .ok_or_else(|| err("missing shoulder repaint materials"))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| err("invalid shoulder repaint material"))
        })
        .collect::<Result<_>>()?;
    let materials: Vec<(f64, u16)> = plan["materials"]
        .as_array()
        .ok_or_else(|| err("missing shoulder materials"))?
        .iter()
        .map(|row| {
            let height = row[0]
                .as_f64()
                .ok_or_else(|| err("invalid shoulder material height"))?;
            if !height.is_finite() {
                return Err(err("invalid shoulder material height"));
            }
            let name = row[1]
                .as_str()
                .ok_or_else(|| err("invalid shoulder material name"))?;
            Ok((height, pick(name)?))
        })
        .collect::<Result<_>>()?;
    if materials.is_empty() || materials.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(err(
            "shoulder materials must have increasing terrace heights",
        ));
    }
    for y in surface[1]..surface[3] {
        for x in surface[0]..surface[2] {
            let i = y * n + x;
            let gx = (sample(&z, n, x as f64 + 1., y as f64)
                - sample(&z, n, x as f64 - 1., y as f64))
                * 0.5;
            let gy = (sample(&z, n, x as f64, y as f64 + 1.)
                - sample(&z, n, x as f64, y as f64 - 1.))
                * 0.5;
            let material = if gx.hypot(gy) > 12. {
                rock
            } else {
                materials
                    .iter()
                    .min_by(|a, b| (a.0 - z[i]).abs().total_cmp(&(b.0 - z[i]).abs()))
                    .unwrap()
                    .1
            };
            // Only affected terrain and inherited cliff/apron materials are repainted.
            let name = &bt.textures[tiles[i] as usize / 64];
            if changed[i] || repaint.contains(&name.as_str()) {
                tiles[i] = material * 64 + tiles[i] % 64;
                changed[i] = true;
            }
        }
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even() as u16)
        .collect();
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if !["HeightMapData", "BlendTileData"].contains(&name.as_str()) && map.get(name)? != chunk {
            return Err(err("shoulder repair changed unrelated chunk"));
        }
    }
    let checked_bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    crate::gameplay::script_spawns::validate_with_layout(
        p,
        map,
        &hm,
        &checked_bt,
        &objects,
        &layout,
    )?;
    crate::gameplay::production_spawns::validate_with_layout(
        p,
        map,
        &hm,
        &checked_bt,
        &objects,
        &layout,
    )?;
    crate::gameplay::gift_clearance::validate_with_layout(p, map, &layout)?;
    crate::gameplay::production_facing::validate(p, map)?;
    // Compare with the actual source scene, without replacing its baseline.
    let policy = crate::pipeline::coordinate_pipeline::spatial_policy(p)?;
    let source_scene = crate::terrain::spatial::Scene::load(p, &source, policy.clone())?;
    let scene = crate::terrain::spatial::Scene::load(p, map, policy)?;
    let collect = |scene: &crate::terrain::spatial::Scene,
                   m: &MapFile|
     -> Result<Vec<crate::terrain::spatial::Issue>> {
        let mut issues = scene.inspect(&layout)?;
        issues.extend(scene.ramp_profiles(&layout)?.1);
        issues.extend(scene.near_ores(p, &layout)?.1);
        issues.extend(scene.routes(m)?.1);
        Ok(issues)
    };
    let regressions = crate::terrain::spatial::regressions(
        &collect(&scene, map)?,
        &collect(&source_scene, &source)?,
    );
    if !regressions.is_empty() {
        return Err(err(format!(
            "shoulder repair spatial regressions: {}",
            serde_json::to_string(&regressions)?
        )));
    }
    let report = json!({"operation":"repair-shoulder","source":source_name,"optimized":1,"retained":layout.ramps.len()-1,"height_bounds":b,"surface_bounds":surface,"changed_height_cells":touched.len(),"objects_unchanged":true,"scripts_unchanged":true,"resource_assets_unchanged":true,"spatial_regressions":regressions,"ramps":[{"preview_id":id,"ramp_index":index+1,"before":r,"after":r,"status":"shoulder-repaired","validation":metrics}],"ingame_test":"not run"});
    Ok(Polished {
        terrain: Terrain {
            n,
            raw,
            land: z.iter().map(|h| *h >= p.number("WATER_Z", 200.)).collect(),
            slope,
            z,
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
