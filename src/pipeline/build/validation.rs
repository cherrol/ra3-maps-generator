//! Native map verification, gameplay acceptance and land connectivity.
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    gameplay::scripts,
    pipeline::project::Project,
    render,
    terrain::surface::Surface,
    terrain::{
        self, Terrain,
        grid::{neighbors_rect, sample},
    },
};
use std::collections::{BTreeSet, VecDeque};

pub fn verify(p: &Project) -> Result<MapFile> {
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        return crate::pipeline::coordinate_pipeline::verify(p);
    }
    verify_native(p).map(|(map, ..)| map)
}

fn verify_native(p: &Project) -> Result<(MapFile, HeightMap, BlendTiles)> {
    let m = MapFile::load(p.assets.join(format!("{}.map", p.name)))?;
    m.validate_release()?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    if hm.width != p.usize("GRID", 0)
        || hm.height != p.usize("GRID_HEIGHT", hm.width)
        || hm.border as usize != p.usize("BORDER", 0)
    {
        return Err(err("map dimensions do not match recipe"));
    }
    let limit = (hm.width - 2 * hm.border as usize) as f32 * p.number("CELL", 10.) as f32;
    let y_limit = (hm.height - 2 * hm.border as usize) as f32 * p.number("CELL", 10.) as f32;
    let mut names = BTreeSet::new();
    for o in &objects {
        if !o.angle.is_finite()
            || o.position.iter().any(|v| !v.is_finite())
            || o.position[0] < 0.
            || o.position[1] < 0.
            || o.position[0] > limit
            || o.position[1] > y_limit
            || o.position[2] != 0.
        {
            return Err(err(format!(
                "object outside playable bounds: {} {:?}",
                o.type_name, o.position
            )));
        }
        if let Some(name) = prop(&o.props, "waypointName").and_then(Value::text)
            && !names.insert(name)
        {
            return Err(err(format!("duplicate waypoint {name}")));
        }
    }
    if !p.flag("TERRAIN_ONLY") {
        for i in 1..=8 {
            let name = if i <= 6 {
                format!("Player_{i}_Start")
            } else {
                format!("Player_{i}_Start (imported)")
            };
            if !names.contains(name.as_str()) {
                return Err(err(format!("missing start {name}")));
            }
        }
        scripts::validate_waypoints(&m)?;
        crate::gameplay::script_spawns::validate(p, &m, &hm, &bt, &objects)?;
        crate::gameplay::periodic_attack::validate(p, &m)?;
        crate::gameplay::production_facing::validate(p, &m)?;
    }
    scripts::validate(&m)?;
    crate::resources::validate_manifest(&m)?;
    let layout = terrain::Layout::load(p)?;
    crate::terrain::ramp_clearance::validate(p, &m, &layout)?;
    let spawns = crate::terrain::spawn_placement::validate(p, &m, &layout)?;
    crate::pipeline::project::write_json(&p.assets.join("spawn-placement.json"), &spawns)?;
    let allocation = crate::terrain::resource_routes::validate(p, &m)?;
    crate::pipeline::project::write_json(
        &p.assets.join("resource-distribution.json"),
        &allocation,
    )?;
    if bt
        .tiles
        .iter()
        .any(|&v| v as usize / 64 >= bt.textures.len())
    {
        return Err(err("terrain references missing texture"));
    }
    println!(
        "回读通过：{} 个区块，{} 种纹理，{} 个对象",
        m.chunks.len(),
        bt.textures.len(),
        objects.len()
    );
    Ok((m, hm, bt))
}
pub fn connectivity(t: &Terrain, s: &Surface, p: &Project) -> Result<()> {
    let n = t.n;
    let h = t.height();
    let water = p.number("WATER_Z", 200.);
    let mut lab = vec![0usize; n * h];
    let mut sizes = vec![0];
    let mut q = VecDeque::new();
    for i in 0..n * h {
        if lab[i] != 0 || s.impassable[i] || t.z[i] < water {
            continue;
        }
        let label = sizes.len();
        let mut count = 0;
        lab[i] = label;
        q.push_back(i);
        while let Some(j) = q.pop_front() {
            count += 1;
            for k in neighbors_rect(j, n, h) {
                if lab[k] == 0 && !s.impassable[k] && t.z[k] >= water {
                    lab[k] = label;
                    q.push_back(k);
                }
            }
        }
        sizes.push(count);
    }
    let largest = sizes.iter().copied().max().unwrap_or(0);
    println!(
        "陆地可通行分区 {}，最大分区 {} 格",
        sizes.len() - 1,
        largest
    );
    Ok(())
}
pub fn accept(p: &Project) -> Result<()> {
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        return crate::pipeline::coordinate_pipeline::accept(p);
    }
    let (_, hm, bt) = verify_native(p)?;
    let t = Terrain::load(p)?;
    if hm.elevations != terrain::flip(&t.raw, t.n) {
        return Err(err("heightmap axis/content differs from terrain input"));
    }
    let imp = terrain::flip(&bt.impassability, t.n);
    let n = t.n;
    let h = t.height();
    let water = p.number("WATER_Z", 200.);
    let mut failures = Vec::new();
    if !p.flag("TERRAIN_ONLY") {
        for (bi, &base) in t.layout.base_cells.iter().enumerate() {
            let near = t
                .layout
                .ore_cells
                .iter()
                .enumerate()
                .filter(|(_, c)| (c[0] - base[0]).hypot(c[1] - base[1]) <= 62.)
                .collect::<Vec<_>>();
            if near.len() != 2 {
                failures.push(format!("P{} 近矿 {} 个，要求 2 个", bi + 1, near.len()));
            }
            for (j, ore) in near {
                let d = (ore[0] - base[0]).hypot(ore[1] - base[1]);
                if !p.terrain_flag("ore_beside") && !(43.5..=62.).contains(&d) {
                    failures.push(format!("P{} 近矿距离 {d:.1}", bi + 1));
                }
                let ang = t.layout.ore_angles[j];
                if !p.terrain_flag("ore_beside")
                    && ang.cos() * (base[0] - ore[0]) + ang.sin() * (base[1] - ore[1]) <= 0.
                {
                    failures.push(format!("P{} 近矿朝向未面向基地", bi + 1));
                }
            }
            let x = base[0].round_ties_even() as usize;
            let y = base[1].round_ties_even() as usize;
            let z = t.z[y * n + x];
            if z >= water {
                let mut span_x = 1;
                let mut span_y = 1;
                for d in [-1isize, 1] {
                    let mut xx = x as isize + d;
                    while xx >= 0 && (xx as usize) < n && (t.z[y * n + xx as usize] - z).abs() < 3.
                    {
                        span_x += 1;
                        xx += d;
                    }
                    let mut yy = y as isize + d;
                    while yy >= 0 && (yy as usize) < h && (t.z[yy as usize * n + x] - z).abs() < 3.
                    {
                        span_y += 1;
                        yy += d;
                    }
                }
                if span_x.max(span_y) < 80 {
                    failures.push(format!("P{} 平整跨度不足 80 格", bi + 1));
                }
            }
        }
        let build_r = p
            .number("ORE_CLEAR_RADIUS", 20.)
            .max(p.terrain_number("ore_mouth_r1", 20.));
        let border = p.usize("BORDER", 20);
        for &ore in &t.layout.ore_cells {
            let x = ore[0].round_ties_even() as usize;
            let y = ore[1].round_ties_even() as usize;
            let is_water = t.z[y * n + x] < water;
            let r = build_r.ceil() as usize;
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            let mut blocked = false;
            for yy in y.saturating_sub(r)..=(y + r).min(h - 1) {
                for xx in x.saturating_sub(r)..=(x + r).min(n - 1) {
                    let i = yy * n + xx;
                    if terrain::disk(i, n, ore, build_r) {
                        lo = lo.min(t.z[i]);
                        hi = hi.max(t.z[i]);
                        blocked |= imp[i] || bt.buildability[(h - 1 - yy) * n + xx];
                    }
                }
            }
            let d = [
                ore[0] - border as f64,
                ore[1] - border as f64,
                (n - 1 - border) as f64 - ore[0],
                (h - 1 - border) as f64 - ore[1],
            ]
            .into_iter()
            .fold(f64::INFINITY, f64::min);
            if (!is_water && hi - lo > 12.)
                || blocked
                || d < p.terrain_number("ore_border_clear", 14.)
            {
                failures.push(format!(
                    "矿 ({x},{y}) 建造盘不合格：高差 {:.1}，阻挡 {blocked}，边距 {d:.1}",
                    hi - lo
                ));
            }
        }
    }
    for (i, r) in t.layout.ramps.iter().enumerate() {
        let (ux, uy, _, _) = terrain::ramp_axes(r)?;
        let cx = terrain::number(r, "x", 0.);
        let cy = terrain::number(r, "y", 0.);
        let auto = r["auto"].as_bool().unwrap_or(false);
        let half = if auto {
            (terrain::number(r, "span", terrain::number(r, "w", 28.)) * 0.5).clamp(10., 20.)
        } else {
            (terrain::number(r, "w", 28.).min(terrain::number(r, "h", 28.)) * 0.25).max(6.)
        };
        let pad = if auto { 4 } else { 6 };
        let start = -half - pad as f64;
        let count = (2. * half + 2. * pad as f64).floor() as usize + 1;
        let profile = (0..count)
            .map(|j| {
                sample(
                    &t.z,
                    n,
                    cx + ux * (start + j as f64),
                    cy + uy * (start + j as f64),
                )
            })
            .collect::<Vec<_>>();
        let along = profile
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0., f64::max);
        let monotone = profile.windows(2).any(|w| w[1] - w[0] < -1.);
        let gap = (profile[0] - profile[pad])
            .abs()
            .max((profile[profile.len() - 1] - profile[profile.len() - 1 - pad]).abs());
        let natural = r["surface_style"] == "mountain";
        let span = if natural {
            terrain::ramp_width(r)? * 0.5
        } else {
            (terrain::number(r, "w", 28.).min(terrain::number(r, "h", 28.)) * 0.5 - 1.).max(4.)
        };
        let cross = (0..=(2. * span) as usize)
            .map(|j| {
                let v = -span + j as f64;
                sample(&t.z, n, cx - uy * v, cy + ux * v)
            })
            .collect::<Vec<_>>();
        let trans = cross
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0., f64::max);
        let mut material_samples = (0..=(2. * half / 8.) as usize).map(|j| {
            let v = -half + j as f64 * 8.;
            let x = (cx + ux * v).round_ties_even().clamp(0., (n - 1) as f64) as usize;
            let y = (cy + uy * v).round_ties_even().clamp(0., (h - 1) as f64) as usize;
            bt.textures
                .get(bt.tiles[(h - 1 - y) * n + x] as usize / 64)
                .is_some_and(|s| {
                    if natural {
                        s.starts_with("Grass_") || s.starts_with("Rock_") || s == "Dirt_Iceland05"
                    } else {
                        s == "Dirt_Yucatan06"
                    }
                })
        });
        let surface_ok = if natural {
            material_samples.all(|valid| valid)
        } else {
            material_samples.any(|valid| valid)
        };
        if along > 6.05 || gap > 12. || trans > 10. || monotone || !surface_ok {
            failures.push(format!(
                "斜坡 #{}：沿坡 {along:.2}，端缝 {gap:.1}，横向 {trans:.1}，单调 {}，材质 {surface_ok}",
                i + 1,
                !monotone
            ));
        }
    }
    let art = render::Image::load_tga(&p.assets.join(format!("{}_art.tga", p.name)))?;
    if art.width != 512 || art.height != 512 {
        failures.push("art.tga 尺寸必须为 512×512".into());
    }
    if !failures.is_empty() {
        return Err(err(format!("验收失败：\n{}", failures.join("\n"))));
    }
    println!("验收通过：近矿、建造盘、出生平台、斜坡、轴向、脚本引用");
    Ok(())
}
