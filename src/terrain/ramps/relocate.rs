use super::*;

fn decoration(o: &MapObject) -> bool {
    ["Tree", "Bush", "GardenRock"]
        .iter()
        .any(|s| o.type_name.contains(s))
}
fn unique(o: &MapObject) -> &str {
    map::prop(&o.props, "uniqueID")
        .and_then(map::Value::text)
        .unwrap_or("")
}
fn lane(r: &Value, x: f64, y: f64, pad: f64) -> bool {
    let (u, v) = frame(r, x, y);
    u.abs() <= half(r) + 5. + number(r, "landing", 0.) + pad
        && v.abs()
            <= mouth(r, u)
                + if number(r, "landing", 0.) > 0. {
                    7. + pad
                } else {
                    5. + pad
                }
}
fn carve_new(
    z: &[f64],
    platforms: &[f64],
    n: usize,
    r: &Value,
    locks: &[bool],
    scale: f64,
) -> Option<Vec<(usize, f64)>> {
    let (_, _, lo, hi) = terrain::ramp_axes(r).ok()?;
    let mut patch = Vec::new();
    for i in 0..n * n {
        let (u, v) = frame(r, (i % n) as f64, (i / n) as f64);
        let weight = (1. - smooth((u.abs() - half(r) - number(r, "landing", 0.)) / 5.))
            * (1. - smooth((v.abs() - mouth(r, u) - 2.) / 5.));
        if weight <= 0. {
            continue;
        }
        if !terrain::ramp_layer_allowed(platforms[i], lo, hi) {
            return None;
        }
        let h = lo + (hi - lo) * smooth((u + half(r)) / (2. * half(r)));
        let h = ((z[i] * (1. - weight) + h * weight) / scale).round_ties_even() * scale;
        if (h - z[i]).abs() < scale * 0.5 {
            continue;
        }
        if locks[i] || z[i] < lo - 3. || z[i] > hi + 3. {
            return None;
        }
        patch.push((i, h));
    }
    Some(patch)
}

fn move_object(
    p: &Project,
    objects: &mut [MapObject],
    index: usize,
    ground: (&[f64], &[bool], &[bool]),
    n: usize,
    ramps: &[Value],
    original_height: f64,
) -> Result<Value> {
    let (z, imp, build) = ground;
    let old = cell(p, &objects[index], n);
    let ore = objects[index].type_name == "OreNode";
    let radius: usize = if ore {
        p.number("ORE_CLEAR_RADIUS", 20.)
            .max(p.terrain_number("ore_mouth_r1", 20.))
            .ceil() as usize
    } else if decoration(&objects[index]) {
        2
    } else {
        8
    };
    let mut points = Vec::new();
    let bx = old[0].round() as usize;
    let by = old[1].round() as usize;
    for y in by.saturating_sub(60)..=(by + 60).min(n - 1) {
        for x in bx.saturating_sub(60)..=(bx + 60).min(n - 1) {
            points.push(((x as f64 - old[0]).hypot(y as f64 - old[1]), x, y));
        }
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
    let target = points
        .into_iter()
        .find_map(|(_, x, y)| {
            let border = p.usize("BORDER", 20) + radius + 3;
            if x < border || y < border || x + border >= n || y + border >= n {
                return None;
            }
            if ore
                && [
                    x as f64 - p.number("BORDER", 20.),
                    y as f64 - p.number("BORDER", 20.),
                    (n - 1) as f64 - p.number("BORDER", 20.) - x as f64,
                    (n - 1) as f64 - p.number("BORDER", 20.) - y as f64,
                ]
                .into_iter()
                .any(|d| d < p.terrain_number("ore_border_clear", 14.))
            {
                return None;
            }
            if ramps
                .iter()
                .any(|r| lane(r, x as f64, y as f64, radius as f64 + 4.))
            {
                return None;
            }
            if (y - radius..=y + radius).any(|yy| {
                (x - radius..=x + radius).any(|xx| {
                    let h = z[yy * n + xx];
                    h <= p.number("WATER_Z", 200.)
                        || (h - original_height).abs() > 1.
                        || (ore && (imp[yy * n + xx] || build[yy * n + xx]))
                })
            }) {
                return None;
            }
            if objects.iter().enumerate().any(|(i, o)| {
                if i == index || o.type_name.contains("Sidewalk") {
                    return false;
                }
                let c = cell(p, o, n);
                let other = if decoration(o) {
                    2.
                } else if o.type_name == "OreNode" {
                    20.
                } else if o.type_name.contains("Waypoint") {
                    3.
                } else {
                    8.
                };
                (c[0] - x as f64).hypot(c[1] - y as f64) < radius as f64 + other + 2.
            }) {
                return None;
            }
            Some([x as f64, y as f64])
        })
        .ok_or_else(|| {
            err(format!(
                "no safe relocation for {}",
                unique(&objects[index])
            ))
        })?;
    let frame = crate::coordinates::Frame {
        width: n,
        height: n,
        ..crate::coordinates::Frame::from_project(p)
    };
    let position = frame.world(target);
    objects[index].position[0] = position[0] as f32;
    objects[index].position[1] = position[1] as f32;
    Ok(
        json!({"object_index":index,"unique_id":unique(&objects[index]),"type":objects[index].type_name,"from":old,"to":target}),
    )
}

fn ore_foundation(
    p: &Project,
    point: [f64; 2],
    z: &[f64],
    imp: &[bool],
    build: &[bool],
    n: usize,
) -> Result<Value> {
    let radius = p
        .number("ORE_CLEAR_RADIUS", 20.)
        .max(p.terrain_number("ore_mouth_r1", 20.));
    let border = p.number("BORDER", 20.);
    let clearance = [
        point[0] - border,
        point[1] - border,
        n as f64 - 1. - border - point[0],
        n as f64 - 1. - border - point[1],
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min);
    if clearance < p.terrain_number("ore_border_clear", 14.) {
        return Err(err("moved ore is too close to map boundary"));
    }
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    let mut blocked = false;
    let mut samples = 0;
    let cx = point[0].round() as usize;
    let cy = point[1].round() as usize;
    let r = radius.ceil() as usize;
    for y in cy.saturating_sub(r)..=(cy + r).min(n - 1) {
        for x in cx.saturating_sub(r)..=(cx + r).min(n - 1) {
            let i = y * n + x;
            if terrain::disk(i, n, point, radius) {
                low = low.min(z[i]);
                high = high.max(z[i]);
                blocked |= imp[i] || build[i];
                samples += 1;
            }
        }
    }
    if low <= p.number("WATER_Z", 200.) || high - low > 12. || blocked {
        return Err(err(format!(
            "moved ore build disk invalid at {point:?}: range={}, blocked={blocked}",
            high - low
        )));
    }
    Ok(
        json!({"radius":radius,"samples":samples,"height":low,"height_range":high-low,"blocked":false,"border_clearance":clearance}),
    )
}

/// Close an old opening using the original baked blueprint's two endpoint
/// levels. Existing flat third levels are never replaced by interpolated ramp
/// heights or quantized to one of the other levels.
fn close_old(
    z: &mut [f64],
    ground: (&[f64], &[f64]),
    n: usize,
    old: &Value,
    locks: &[bool],
    scale: f64,
    changed: &mut [bool],
) -> Result<()> {
    let (base, platforms) = ground;
    let (_, _, lo, hi) = terrain::ramp_axes(old)?;
    for i in 0..n * n {
        if !old_foot(old, (i % n) as f64, (i / n) as f64, 5.)
            && !lane(old, (i % n) as f64, (i / n) as f64, 0.)
        {
            continue;
        }
        if platforms[i].is_finite() && !terrain::ramp_layer_allowed(platforms[i], lo, hi) {
            continue;
        }
        if z[i] < lo - 3. || z[i] > hi + 3. {
            continue;
        }
        let h = if (base[i] - lo).abs() <= (base[i] - hi).abs() {
            lo
        } else {
            hi
        };
        let h = (h / scale).round_ties_even() * scale;
        if (h - z[i]).abs() < scale * 0.5 {
            continue;
        }
        // The old shoulder may graze a resource's protected approach. Preserve
        // that footprint while closing the unprotected ramp opening itself.
        if locks[i] {
            continue;
        }
        z[i] = h;
        changed[i] = true;
    }
    Ok(())
}

pub fn relocate(
    p: &Project,
    map: &mut MapFile,
    source_name: &str,
    plan: &Value,
) -> Result<Polished> {
    if plan["operation"] == "repair-shoulder" {
        return super::repair::apply(p, map, source_name, plan);
    }
    if plan["source"].as_str() != Some(source_name) || plan["center_policy"] != "exact_target" {
        return Err(err(
            "relocation plan must match the source release and exact target policy",
        ));
    }
    let source = map.clone();
    let directory = p.release_assets(source_name)?;
    let mut layout: Layout =
        serde_json::from_slice(&fs::read(directory.join("source/layout.json"))?)?;
    let index: Value = serde_json::from_slice(&fs::read(directory.join("layout-index.json"))?)?;
    let entries = index["entries"]
        .as_array()
        .ok_or_else(|| err("missing source preview index"))?;
    let entry = |id: u64| {
        entries
            .iter()
            .find(|e| e["id"] == id)
            .ok_or_else(|| err(format!("missing preview id {id}")))
    };
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) {
        return Err(err("ramp relocation grid differs"));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|v| *v as f64 * scale)
        .collect();
    let path = p.root.join(
        plan["blueprint"]
            .as_str()
            .ok_or_else(|| err("missing relocation blueprint"))?,
    );
    let (base, platforms) = blueprint(p, n, &path)?;
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let original_objects = objects.clone();
    let source_imp = terrain::flip(&bt.impassability, n);
    let source_build = terrain::flip(&bt.buildability, n);
    let mut moves = Vec::new();
    let mut claimed = std::collections::BTreeSet::new();
    for request in plan["moves"]
        .as_array()
        .ok_or_else(|| err("missing ramp moves"))?
    {
        let id = request["ramp_id"]
            .as_u64()
            .ok_or_else(|| err("missing ramp id"))?;
        let old_entry = entry(id)?;
        if old_entry["category"] != "斜坡" || !claimed.insert(id) {
            return Err(err("relocation id is not a distinct ramp"));
        }
        let ri = old_entry["source_pointer"]
            .as_str()
            .and_then(|s| s.strip_prefix("/ramps/"))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| err("invalid ramp pointer"))?;
        let old = layout
            .ramps
            .get(ri)
            .ok_or_else(|| err("ramp pointer outside layout"))?
            .clone();
        let target_id = request["target_id"]
            .as_u64()
            .ok_or_else(|| err("missing ramp target"))?;
        let target = &entry(target_id)?["cell"];
        let mut new = old.clone();
        new["x"] = target[0].clone();
        new["y"] = target[1].clone();
        new["style"] = json!("natural");
        new["landing"] = json!(12.);
        terrain::ramp_axes(&new)?;
        terrain::ramp_width(&new)?;
        moves.push((ri, id, target_id, old, new));
    }
    let mut added = std::collections::BTreeSet::new();
    for request in plan["additions"].as_array().into_iter().flatten() {
        let target_id = request["target_id"]
            .as_u64()
            .ok_or_else(|| err("missing new ramp target"))?;
        let target = &entry(target_id)?["cell"];
        let mut seed = request["ramp"].clone();
        seed["x"] = target[0].clone();
        seed["y"] = target[1].clone();
        seed["style"] = json!("natural");
        seed["auto"] = json!(true);
        terrain::ramp_axes(&seed)?;
        terrain::ramp_width(&seed)?;
        let ri = layout.ramps.len();
        let id = layout.objects.len() as u64 + ri as u64 + 1;
        if !claimed.insert(id) {
            return Err(err("new ramp id conflicts with relocation"));
        }
        layout.ramps.push(seed.clone());
        added.insert(ri);
        moves.push((ri, id, target_id, seed.clone(), seed));
    }
    if moves.is_empty() {
        return Err(err("ramp edit plan has no moves or additions"));
    }
    let mut forbidden = layout.ramps.clone();
    forbidden.extend(moves.iter().map(|m| m.4.clone()));
    let mut moved_objects = Vec::new();
    for id in plan["move_buildings"].as_array().into_iter().flatten() {
        let e = entry(id.as_u64().ok_or_else(|| err("invalid building id"))?)?;
        let name = e["unique_id"]
            .as_str()
            .ok_or_else(|| err("building has no unique id"))?;
        let i = objects
            .iter()
            .position(|o| unique(o) == name)
            .ok_or_else(|| err("building missing from source map"))?;
        if decoration(&objects[i]) {
            return Err(err("authorized building id is decoration"));
        }
        let c = cell(p, &objects[i], n);
        let h = sample(&before, n, c[0], c[1]);
        let mut row = move_object(
            p,
            &mut objects,
            i,
            (&before, &source_imp, &source_build),
            n,
            &forbidden,
            h,
        )?;
        row["preview_id"] = id.clone();
        moved_objects.push(row);
    }
    let locks = protected(p, &objects, n);
    let mut z = before.clone();
    let mut changed = vec![false; n * n];
    for (ri, _, _, old, _) in &moves {
        if added.contains(ri) {
            continue;
        }
        close_old(
            &mut z,
            (&base, &platforms),
            n,
            old,
            &locks,
            scale,
            &mut changed,
        )?;
    }
    let mut rows = Vec::new();
    for (ri, id, target_id, old, seed) in &moves {
        let mut accepted = None;
        let mut rejected = [0_usize; 2];
        for candidate in candidates(&base, n, seed)
            .into_iter()
            .filter(|c| c["x"] == seed["x"] && c["y"] == seed["y"])
        {
            let Some(patch) = carve_new(&z, &platforms, n, &candidate, &locks, scale) else {
                rejected[0] += 1;
                continue;
            };
            let mut trial = z.clone();
            for &(i, h) in &patch {
                trial[i] = h;
            }
            let Some(metrics) = validate_shape(&trial, n, &candidate, p.usize("BORDER", 20)) else {
                rejected[1] += 1;
                continue;
            };
            accepted = Some((candidate, patch, trial, metrics));
            break;
        }
        let (candidate,patch,trial,metrics)=accepted.ok_or_else(||err(format!("ramp {id} cannot be carved at target {target_id}: footprint/third-level={} profile={}",rejected[0],rejected[1])))?;
        for &(i, _) in &patch {
            changed[i] = true;
        }
        z = trial;
        rows.push(json!({"ramp_index":ri+1,"preview_id":id,"target_preview_id":target_id,"status":if added.contains(ri){"added"}else{"relocated"},"before":old,"after":candidate,"validation":metrics,"changed_height_cells":patch.len()}));
        layout.ramps[*ri] = candidate;
    }
    let actual: Vec<_> = moves.iter().map(|m| layout.ramps[m.0].clone()).collect();
    for i in 0..objects.len() {
        let c = cell(p, &objects[i], n);
        if decoration(&objects[i]) && actual.iter().any(|r| lane(r, c[0], c[1], 3.)) {
            if plan["relocate_obstructing_decoration"] != true {
                return Err(err("decoration blocks the relocated ramp"));
            }
            let h = sample(&before, n, c[0], c[1]);
            moved_objects.push(move_object(
                p,
                &mut objects,
                i,
                (&z, &source_imp, &source_build),
                n,
                &layout.ramps,
                h,
            )?);
        }
    }
    let touched: Vec<_> = changed
        .iter()
        .enumerate()
        .filter(|(_, b)| **b)
        .map(|(i, _)| (i, z[i]))
        .collect();
    let mut imp = terrain::flip(&bt.impassability, n);
    for r in &actual {
        imp = masks(&z, &imp, n, r, &touched, p);
    }
    let original_imp = terrain::flip(&bt.impassability, n);
    for i in 0..n * n {
        if locks[i] {
            imp[i] = original_imp[i];
        }
    }
    let water = p.number("WATER_Z", 200.);
    let mut keys: Vec<_> = original_objects
        .iter()
        .filter(|o| {
            o.type_name == "OreNode"
                || map::prop(&o.props, "waypointName")
                    .and_then(map::Value::text)
                    .is_some_and(|s| s.starts_with("Player_"))
        })
        .map(|o| cell(p, o, n))
        .filter(|c| c[0] >= 0. && c[1] >= 0. && c[0] < n as f64 && c[1] < n as f64)
        .map(|c| c[1].round() as usize * n + c[0].round() as usize)
        .collect();
    for (old, new) in original_objects.iter().zip(&objects) {
        if old.type_name == "OreNode" && old.position != new.position {
            let c = cell(p, new, n);
            keys.push(c[1].round() as usize * n + c[0].round() as usize);
        }
    }
    for sea in [false, true] {
        if !preserve_connections(
            &labels(&before, &terrain::flip(&bt.impassability, n), n, water, sea),
            &labels(&z, &imp, n, water, sea),
            &keys,
        ) {
            return Err(err("relocation disconnects existing base/resource paths"));
        }
    }
    for row in &rows {
        if validate_shape(&z, n, &row["after"], p.usize("BORDER", 20)).is_none() {
            return Err(err("relocated ramps interfere"));
        }
    }
    for (ri, r) in layout.ramps.iter().enumerate() {
        if !moves.iter().any(|m| m.0 == ri)
            && r["style"] == "natural"
            && validate_shape(&before, n, r, p.usize("BORDER", 20)).is_some()
            && validate_shape(&z, n, r, p.usize("BORDER", 20)).is_none()
        {
            return Err(err("relocation damages another ramp"));
        }
    }
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    for (ri, _, _, old, _) in &moves {
        if added.contains(ri) {
            continue;
        }
        // Clear the original material apron without repainting a new old lane.
        clear_old_paint(&mut tiles, &bt.textures, &z, n, old, water, &mut changed);
    }
    for r in &actual {
        paint(&mut tiles, &bt.textures, &z, n, (r, r), water, &mut changed);
    }
    let slope = terrain::gradient(&z, n);
    let mut build = terrain::flip(&bt.buildability, n);
    for i in 0..n * n {
        if changed[i] && !locks[i] {
            build[i] = slope[i] > 1.5;
        }
    }
    for row in &moved_objects {
        for k in ["from", "to"] {
            let c = &row[k];
            let cx = c[0].as_f64().unwrap().round() as usize;
            let cy = c[1].as_f64().unwrap().round() as usize;
            for y in cy.saturating_sub(2)..=(cy + 2).min(n - 1) {
                for x in cx.saturating_sub(2)..=(cx + 2).min(n - 1) {
                    let i = y * n + x;
                    build[i] = slope[i] > 1.5 || (k == "to" && row["type"] != "OreNode");
                    changed[i] = true;
                }
            }
        }
    }
    let mut moved_ore_sites = Vec::new();
    for row in &moved_objects {
        if row["type"] == "OreNode" {
            let c = [
                row["to"][0].as_f64().unwrap(),
                row["to"][1].as_f64().unwrap(),
            ];
            moved_ore_sites.push(json!({"object_index":row["object_index"],"cell":c,"validation":ore_foundation(p,c,&z,&imp,&build,n)?}));
        }
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even() as u16)
        .collect();
    for i in 0..n * n {
        if locks[i] && raw[i] != (before[i] / scale).round_ties_even() as u16 {
            return Err(err("relocation changed a protected foundation"));
        }
    }
    let mut hm = hm;
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if !["HeightMapData", "BlendTileData", "ObjectsList"].contains(&name.as_str())
            && map.get(name)? != chunk
        {
            return Err(err(format!("relocation changed unrelated chunk {name}")));
        }
    }
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt_checked = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    crate::gameplay::production_spawns::validate(p, map, &hm, &bt_checked, &objects)?;
    crate::gameplay::gift_clearance::validate(p, map)?;
    crate::gameplay::production_facing::validate(p, map)?;
    crate::gameplay::periodic_attack::validate(p, map)?;
    for (old, new) in original_objects.iter().zip(&objects) {
        if old.position != new.position {
            let a = cell(p, old, n);
            let b = cell(p, new, n);
            for c in &mut layout.hospital_cells {
                if (c[0] - a[0]).hypot(c[1] - a[1]) < 0.01 {
                    *c = b;
                }
            }
            if old.type_name == "OreNode" {
                for c in &mut layout.ore_cells {
                    if (c[0] - a[0]).hypot(c[1] - a[1]) < 0.01 {
                        *c = b;
                    }
                }
            }
        }
    }
    let excluded = crate::gameplay::scripts::load_rules(p)?.spawn_waypoints;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !map::prop(&o.props, "waypointName")
                .and_then(map::Value::text)
                .is_some_and(|s| excluded.iter().any(|w| w.name == s))
        })
        .map(|o| ObjectSpec {
            cell: cell(p, o, n),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    let report = json!({"operation":"relocate-ramps","source":source_name,"ramps":rows,"optimized":moves.len(),"added":added.len(),"retained":layout.ramps.len()-moves.len(),"object_moves":moved_objects,"moved_ore_sites":moved_ore_sites,"map_objects":objects.len(),"indexed_layout_objects":layout.objects.len(),"objects_unchanged":false,"scripts_unchanged":true,"resource_assets_unchanged":true,"existing_land_and_sea_resource_connections_preserved":true,"modified_chunks":["HeightMapData","BlendTileData","ObjectsList"],"ingame_test":"not run"});
    Ok(Polished {
        terrain: Terrain {
            n,
            raw,
            land: z.iter().map(|h| *h >= water).collect(),
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

fn clear_old_paint(
    tiles: &mut [u16],
    textures: &[String],
    z: &[f64],
    n: usize,
    old: &Value,
    water: f64,
    changed: &mut [bool],
) {
    for i in 0..n * n {
        if !old_foot(old, (i % n) as f64, (i / n) as f64, 8.) {
            continue;
        }
        let name = &textures[tiles[i] as usize / 64];
        if !["Dirt_Yucatan06", "Gravel_Yucatan01"].contains(&name.as_str()) {
            continue;
        }
        let material = if z[i] < water {
            "RA3_ShallowSeaFloor"
        } else if z[i] < 245. {
            "Grass_NewYork01"
        } else if z[i] < 315. {
            "Grass_MtRush04"
        } else if z[i] < 385. {
            "Dirt_Iceland05"
        } else {
            "Pavement_Amsterdam05"
        };
        if let Some(t) = textures.iter().position(|s| s == material) {
            tiles[i] = t as u16 * 64 + tiles[i] % 64;
            changed[i] = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moved_ore_requires_full_clear_build_disk_and_boundary_clearance() {
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "陕西").unwrap();
        let n = 128;
        let mut z = vec![212.; n * n];
        let imp = vec![false; n * n];
        let mut build = imp.clone();
        let valid = ore_foundation(&p, [64., 64.], &z, &imp, &build, n).unwrap();
        assert_eq!(valid["height_range"], 0.);
        assert_eq!(valid["blocked"], false);
        build[64 * n + 74] = true;
        assert!(ore_foundation(&p, [64., 64.], &z, &imp, &build, n).is_err());
        build[64 * n + 74] = false;
        z[64 * n + 74] = 280.;
        assert!(ore_foundation(&p, [64., 64.], &z, &imp, &build, n).is_err());
        assert!(ore_foundation(&p, [25., 64.], &z, &imp, &build, n).is_err());
    }
    #[test]
    fn relocation_closes_old_opening_and_carves_exact_new_center() {
        let n = 128;
        let scale = 0.0390625;
        let base: Vec<_> = (0..n * n)
            .map(|i| if i % n < 64 { 210. } else { 280. })
            .collect();
        let old = json!({"x":64.,"y":40.,"ux":1.,"uy":0.,"w":30.,"h":20.,"span":30.,"width":20.,"pair":[210.,280.]});
        let mut new = old.clone();
        new["y"] = json!(88.);
        let locks = vec![false; n * n];
        let mut z = base.clone();
        for (i, h) in sculpt(&z, (&base, &base), n, &old, &old, &locks, scale).unwrap() {
            z[i] = h;
        }
        assert!(validate_shape(&z, n, &old, 0).is_some());
        close_old(
            &mut z,
            (&base, &base),
            n,
            &old,
            &locks,
            scale,
            &mut vec![false; n * n],
        )
        .unwrap();
        assert!(z.iter().zip(&base).all(|(a, b)| (a - b).abs() < 0.001));
        for (i, h) in carve_new(&z, &base, n, &new, &locks, scale).unwrap() {
            z[i] = h;
        }
        assert!(validate_shape(&z, n, &new, 0).is_some());
        assert_eq!(z[88 * n + 64], 245.);
        assert_eq!(z[40 * n + 64], 280.);
        assert!(validate_shape(&z, n, &old, 0).is_none());
    }
    #[test]
    fn old_closure_preserves_resource_locks_and_real_third_platforms() {
        let n = 128;
        let old = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"w":30.,"h":20.,"span":30.,"width":20.,"pair":[210.,350.]});
        let base = vec![210.; n * n];
        let mut z = vec![250.; n * n];
        let mut platforms = vec![f64::NAN; n * n];
        let mut locks = vec![false; n * n];
        locks[64 * n + 64] = true;
        platforms[65 * n + 64] = 280.;
        z[65 * n + 64] = 280.;
        close_old(
            &mut z,
            (&base, &platforms),
            n,
            &old,
            &locks,
            0.0390625,
            &mut vec![false; n * n],
        )
        .unwrap();
        assert_eq!(z[64 * n + 64], 250.);
        assert_eq!(z[65 * n + 64], 280.);
        assert_eq!(z[64 * n + 63], 210.);
    }
    #[test]
    fn diagonal_full_width_has_quantized_landings_and_keeps_center_fixed() {
        let n = 128;
        let r = json!({"x":64.,"y":64.,"ux":-0.8,"uy":-0.6,"span":35.,"width":25.,"pair":[212.,350.],"landing":12.});
        let mut z: Vec<_> = (0..n * n)
            .map(|i| if i / n < 64 { 350. } else { 212. })
            .collect();
        let patch = carve_new(
            &z,
            &vec![f64::NAN; n * n],
            n,
            &r,
            &vec![false; n * n],
            0.0390625,
        )
        .unwrap();
        for (i, h) in patch {
            z[i] = h;
        }
        let shape = validate_shape(&z, n, &r, 0).unwrap();
        assert_eq!(shape["parallel_lanes"], 26);
        assert_eq!(shape["width_cells"], 25.);
        assert!((z[64 * n + 64] - 281.).abs() < 0.04);
        for sign in [-1., 1.] {
            let x = 64. - 0.8 * sign * 27.5;
            let y = 64. - 0.6 * sign * 27.5;
            let h = sample(&z, n, x, y);
            assert!((h - if sign < 0. { 212. } else { 350. }).abs() < 0.04);
        }
    }
}
