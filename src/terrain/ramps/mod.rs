//! Local ramp edits on a finished map. Unrelated chunks and resource IDs stay intact.
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject},
    pipeline::project::{self, Project},
    render::{self, Image},
    terrain::surface::Surface,
    terrain::{self, Layout, ObjectSpec, Terrain, number},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
};

use super::grid::sample;

mod artifacts;
mod geometry;
use geometry::*;
mod material_joins;
mod surface;
pub use material_joins::{repair_material_joins, repair_material_joins_seeded};
use surface::paint;
pub(crate) use surface::patch_blend;

mod relocate;
pub use relocate::relocate;
mod peaks;
mod repair;
pub use peaks::add_mountains;
mod mountain;
pub(crate) use mountain::carve as carve_mountain;
pub(crate) use mountain::paint as paint_mountain;

pub const PROFILE: &str = "mountain";

pub fn enabled(p: &Project) -> Result<bool> {
    match p.string("RAMP_PROFILE", PROFILE) {
        PROFILE => Ok(true),
        name => Err(err(format!("unknown RAMP_PROFILE {name}"))),
    }
}

/// Fresh blueprints use the shared profile; native coordinate sources remain
/// immutable until explicitly edited and adopted as a complete new release.
pub(crate) fn configure_layout(p: &Project, layout: &mut Layout) -> Result<()> {
    if enabled(p)? {
        for r in &mut layout.ramps {
            r["surface_style"] = json!(PROFILE);
            if !p.terrain_flag("v11_baked") {
                r["style"] = json!(PROFILE);
            }
        }
    }
    Ok(())
}

pub struct Polished {
    pub terrain: Terrain,
    pub report: Value,
    before: Vec<f64>,
    before_tiles: Vec<u16>,
    tiles: Vec<u16>,
    imp: Vec<bool>,
    build: Vec<bool>,
    textures: Vec<String>,
    example: usize,
}
pub fn apply(p: &Project, map: &mut MapFile, source_name: &str) -> Result<Polished> {
    apply_inner(p, map, source_name, true)
}

pub fn refresh_surfaces(p: &Project, map: &mut MapFile, source_name: &str) -> Result<Polished> {
    apply_inner(p, map, source_name, false)
}

fn apply_inner(
    p: &Project,
    map: &mut MapFile,
    source_name: &str,
    geometry: bool,
) -> Result<Polished> {
    enabled(p)?;
    let source = map.clone();
    let mut hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err("ramp polishing requires matching square v27 terrain"));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let reserved_entities = if geometry && crate::pipeline::coordinate_pipeline::enabled(p) {
        crate::terrain::spatial::Scene::load(
            p,
            map,
            crate::pipeline::coordinate_pipeline::spatial_policy(p)?,
        )?
        .entities
    } else {
        Vec::new()
    };
    let scale = p.number("Z_SCALE", 0.0390625);
    let before: Vec<_> = terrain::flip(&hm.elevations, n)
        .iter()
        .map(|v| *v as f64 * scale)
        .collect();
    // A prior polish release carries its actual ramp definitions with it.
    let mut layout = Layout::load_release(p, source_name)?;
    layout.sync_imported_starts(p, source_name, &source)?;
    let clearance_policy = crate::terrain::ramp_clearance::policy(p)?
        .expect("the common ramp clearance policy is always enabled");
    let coordinate_frame = crate::coordinates::Frame::from_heightmap(p, &hm)?;
    let original_ramps = crate::terrain::ramp_clearance::ramps(&layout, &clearance_policy)?;
    let mut clearance_layout = layout.clone();
    // Recover the two supporting platforms from the actual finished map. The
    // old blueprint predates authored ramp additions, object moves and cuts.
    let mut levels = Vec::new();
    for r in &layout.ramps {
        let (_, _, lo, hi) = terrain::ramp_axes(r)?;
        terrain::ramp_width(r)?;
        levels.extend([lo, hi]);
    }
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    let native_gradient = terrain::gradient(&before, n);
    let base: Vec<_> = before
        .iter()
        .map(|h| {
            levels
                .iter()
                .copied()
                .min_by(|a, b| (a - h).abs().total_cmp(&(b - h).abs()))
                .unwrap_or(*h)
        })
        .collect();
    let platforms: Vec<_> = before
        .iter()
        .zip(&native_gradient)
        .enumerate()
        .map(|(i, (h, g))| {
            if *g <= 0.1 || (*g <= 3. && (base[i] - h).abs() <= 3.) {
                *h
            } else {
                f64::NAN
            }
        })
        .collect();
    let mut locks = protected(p, &objects, n);
    let starting_resources = crate::terrain::spawn_placement::protected_cells(p, &source, &layout)?;
    for (locked, reserved) in locks.iter_mut().zip(starting_resources) {
        *locked |= reserved;
    }
    let gift_reservations = crate::gameplay::gift_clearance::reservations(p, &source)?;
    for &([cx, cy], padding) in &gift_reservations.footprints {
        // Occupancy validation uses a square, rather than a circular marker.
        // Include the one-cell passability update halo around a terrain edit.
        let radius = padding + 2;
        for y in cy.saturating_sub(radius)..=(cy + radius).min(n - 1) {
            for x in cx.saturating_sub(radius)..=(cx + radius).min(n - 1) {
                locks[y * n + x] = true;
            }
        }
    }
    let water = p.number("WATER_Z", 200.);
    let mut z = before.clone();
    let old_imp = terrain::flip(&bt.impassability, n);
    let mut imp = old_imp.clone();
    let keys: Vec<_> = objects
        .iter()
        .filter(|o| {
            o.type_name == "OreNode"
                || map::prop(&o.props, "waypointName")
                    .and_then(map::Value::text)
                    .is_some_and(|s| s.starts_with("Player_"))
        })
        .map(|o| cell(p, o, n))
        .filter(|c| c[0] >= 0. && c[1] >= 0. && c[0] < n as f64 && c[1] < n as f64)
        .map(|c| {
            c[1].round().clamp(0., (n - 1) as f64) as usize * n
                + c[0].round().clamp(0., (n - 1) as f64) as usize
        })
        .collect();
    let original_land = labels(&z, &imp, n, water, false);
    let original_sea = labels(&z, &imp, n, water, true);
    let before_tiles = terrain::flip(&bt.tiles, n);
    let mut tiles = before_tiles.clone();
    let mut changed = vec![false; n * n];
    let mut rows = Vec::new();
    let mut example = 0;
    let mut largest_turn = 0_f64;
    let mut valid_shapes: Vec<_> = layout
        .ramps
        .iter()
        .map(|r| validate_shape(&before, n, r, p.usize("BORDER", 20)).is_some())
        .collect();
    for index in 0..layout.ramps.len() {
        let old = layout.ramps[index].clone();
        terrain::ramp_axes(&old)?;
        terrain::ramp_width(&old)?;
        let mut accepted = None;
        let mut rejected = [0_usize; 4];
        for candidate in if geometry {
            candidates(&base, n, &old)
        } else {
            Vec::new()
        } {
            // A source opening can already overlap a conservative reservation.
            // Exclude intersecting candidates; try a safe nearby candidate
            // before falling back to the original geometry and new materials.
            if reserved_entities.iter().any(|e| {
                if !e.in_playable || e.reserve <= 0. {
                    return false;
                }
                let (u, v) = frame(&candidate, e.point[0], e.point[1]);
                let along = half(&candidate) + number(&candidate, "landing", 0.);
                let across = terrain::ramp_width(&candidate).expect("validated width") * 0.5;
                (u.abs() - along).max(0.).hypot((v.abs() - across).max(0.)) < e.reserve
            }) {
                rejected[0] += 1;
                continue;
            }
            if objects.iter().any(|o| {
                if !o.type_name.contains("Tree")
                    && !o.type_name.contains("Bush")
                    && !o.type_name.contains("Rock")
                {
                    return false;
                }
                let c = cell(p, o, n);
                let (u, v) = frame(&candidate, c[0], c[1]);
                u.abs() < half(&candidate) + 3. && v.abs() < mouth(&candidate, u) + 2.5
            }) {
                rejected[0] += 1;
                continue;
            }
            let Some(patch) = sculpt(&z, (&base, &platforms), n, &old, &candidate, &locks, scale)
            else {
                rejected[0] += 1;
                continue;
            };
            if patch.is_empty() {
                continue;
            }
            let mut trial = z.clone();
            for &(i, h) in &patch {
                trial[i] = h;
            }
            let Some(metrics) = validate_shape(&trial, n, &candidate, p.usize("BORDER", 20)) else {
                rejected[1] += 1;
                continue;
            };
            // Preserve every valid opening, including ramps later in the list.
            // The final package separately requires every ramp to pass.
            if layout.ramps.iter().enumerate().any(|(i, r)| {
                i != index
                    && valid_shapes[i]
                    && validate_shape(&trial, n, r, p.usize("BORDER", 20)).is_none()
            }) {
                rejected[1] += 1;
                continue;
            }
            clearance_layout.ramps[index] = candidate.clone();
            let trial_ramps =
                crate::terrain::ramp_clearance::ramps(&clearance_layout, &clearance_policy)?;
            if !preserves_resource_approaches(
                &objects,
                &coordinate_frame,
                (&before, &trial),
                (&original_ramps[index], &trial_ramps[index]),
                &clearance_policy,
            ) {
                rejected[3] += 1;
                continue;
            }
            let trial_imp = masks(&trial, &imp, n, &candidate, &patch, p);
            if !preserve_connections(
                &original_land,
                &labels(&trial, &trial_imp, n, water, false),
                &keys,
            ) || !preserve_connections(
                &original_sea,
                &labels(&trial, &trial_imp, n, water, true),
                &keys,
            ) {
                rejected[2] += 1;
                continue;
            }
            accepted = Some((candidate, patch, trial, trial_imp, metrics));
            break;
        }
        let rejected_candidates = json!({"footprint_or_third_level":rejected[0],"lane_shape_or_boundary":rejected[1],"connectivity":rejected[2],"resource_approaches":rejected[3]});
        if let Some((candidate, patch, trial, trial_imp, metrics)) = accepted {
            let turn = delta(
                number(&candidate, "uy", 0.).atan2(number(&candidate, "ux", 1.)),
                number(&old, "uy", 0.).atan2(number(&old, "ux", 1.)),
            )
            .to_degrees();
            if turn.abs() > largest_turn {
                largest_turn = turn.abs();
                example = index;
            }
            paint(
                &mut tiles,
                &bt.textures,
                &trial,
                n,
                (&old, &candidate),
                water,
                &mut changed,
            );
            for &(i, _) in &patch {
                changed[i] = true;
            }
            rows.push(json!({"ramp_index":index+1,"status":"optimized","rotation_degrees":turn,"changed_height_cells":patch.len(),"before":old,"after":candidate,"validation":metrics,"rejected_candidates":rejected_candidates}));
            layout.ramps[index] = candidate;
            valid_shapes[index] = true;
            z = trial;
            imp = trial_imp;
        } else {
            // Even where geometry cannot move safely, remove the old 8x8
            // material apron using the unchanged, proven opening.
            let mut retained = old.clone();
            retained["surface_style"] = json!("mountain");
            paint(
                &mut tiles,
                &bt.textures,
                &z,
                n,
                (&old, &retained),
                water,
                &mut changed,
            );
            rows.push(json!({"ramp_index":index+1,"status":"texture-only","rejected_candidates":rejected_candidates,"reason":"Geometry retained to preserve protected footprints, full-width approaches and refinery reservations, lane profiles or existing land/sea resource connectivity; materials polished in place.","before":old,"after":retained}));
            layout.ramps[index] = retained;
        }
    }
    let raw: Vec<_> = z
        .iter()
        .map(|h| (h / scale).round_ties_even().clamp(0., 65535.) as u16)
        .collect();
    hm.elevations = terrain::flip(&raw, n);
    map.get_mut("HeightMapData")?.payload = hm.encode()?;
    let mut build = terrain::flip(&bt.buildability, n);
    let slope = terrain::gradient(&z, n);
    for i in 0..n * n {
        if geometry && changed[i] && !locks[i] {
            build[i] = build[i] || slope[i] > 1.5;
        }
    }
    patch_blend(map, &bt, &tiles, &imp, &build, &changed, n)?;
    // Check preservation on the actual map that will be packaged.
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if name != "BlendTileData"
            && (!geometry || name != "HeightMapData")
            && map.get(name)? != chunk
        {
            return Err(err(format!("ramp edit changed unrelated chunk {name}")));
        }
    }
    let checked_hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let checked_bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    if !geometry {
        let start = 4 + 8 * n * n;
        let end = start + 8 * n * n.div_ceil(8) + n * n;
        if source.get("BlendTileData")?.payload[start..end]
            != map.get("BlendTileData")?.payload[start..end]
        {
            return Err(err("ramp surface refresh changed gameplay bitmaps"));
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
    if gift_reservations.guarded {
        crate::gameplay::gift_clearance::validate_with_layout(p, map, &layout)?;
    }
    for i in 0..n * n {
        if locks[i] && raw[i] != (before[i] / scale).round_ties_even() as u16 {
            return Err(err("ramp edit changed a protected footprint"));
        }
    }
    let script_waypoints = crate::gameplay::scripts::load_rules(p)?.spawn_waypoints;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !map::prop(&o.props, "waypointName")
                .and_then(map::Value::text)
                .is_some_and(|name| script_waypoints.iter().any(|s| s.name == name))
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
    let optimized = rows.iter().filter(|r| r["status"] == "optimized").count();
    for row in &mut rows {
        row["preview_id"] =
            json!(layout.objects.len() + row["ramp_index"].as_u64().unwrap() as usize);
    }
    let report = json!({"source":source_name,"profile":"mountain","geometry_source":"actual source map heights and platforms","ramps":rows,"optimized":optimized,"retained":layout.ramps.len()-optimized,"map_objects":objects.len(),"indexed_layout_objects":layout.objects.len(),"objects_unchanged":true,"scripts_unchanged":true,"resource_assets_unchanged":true,"protected_footprints_unchanged":true,"ramp_approach_clearance_policy":clearance_policy,"existing_land_and_sea_resource_connections_preserved":true,"modified_chunks":["HeightMapData","BlendTileData"],"ingame_test":"not run"});
    let mut report = report;
    if !geometry {
        report["operation"] = json!("polish-ramp-surfaces");
        report["geometry_unchanged"] = json!(true);
        report["gameplay_bitmaps_unchanged"] = json!(true);
        report["modified_chunks"] = json!(["BlendTileData"]);
        for row in report["ramps"].as_array_mut().unwrap() {
            row["reason"] =
                json!("Surface-only refresh; native heights and all gameplay bitmaps preserved.");
            row.as_object_mut().unwrap().remove("rejected_candidates");
        }
    }
    println!(
        "斜坡地形优化 {optimized}/{}；其余仅整理纹理；对象、脚本和资源清单保持原样",
        layout.ramps.len()
    );
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
        example,
    })
}

#[cfg(test)]
mod tests;
