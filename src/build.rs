use crate::{
    Result, archive,
    binary::{Reader, Writer},
    err,
    map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    project::{self, Project},
    render, scripts,
    surface::Surface,
    terrain::{self, Terrain},
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
pub const STAGES: [&str; 11] = [
    "terrain",
    "passability",
    "map",
    "verify",
    "spatial",
    "preview",
    "coordinates",
    "accept",
    "readme",
    "dist",
    "wb",
];
#[derive(Debug, Default)]
pub struct Options {
    pub only: Vec<String>,
    pub skip: Vec<String>,
    pub bump: bool,
    pub release: bool,
    pub note: String,
    pub install: bool,
    pub no_zip: bool,
    pub wb_install: bool,
    pub wb_root: Option<PathBuf>,
}
pub fn run(root: &Path, id: &str, options: &Options) -> Result<Project> {
    for stage in options.only.iter().chain(&options.skip) {
        if !STAGES.contains(&stage.as_str()) {
            return Err(err(format!("unknown build stage {stage}")));
        }
    }
    let original = Project::load(root, id)?;
    if !crate::coordinate_pipeline::enabled(&original)
        && options.only.iter().any(|s| s == "spatial")
    {
        return Err(err("this map has not enabled COORDINATE_PIPELINE"));
    }
    if crate::coordinate_pipeline::enabled(&original) {
        crate::coordinate_pipeline::preflight(&original)?;
    }
    crate::texture_polish::enabled(&original)?;
    let version_path = project::version_path(root, id);
    let previous = fs::read(&version_path).ok();
    let mut cleanup = Vec::<(PathBuf, bool)>::new();
    let p = if options.bump || options.release {
        let note = if options.note.is_empty() {
            "修改"
        } else {
            &options.note
        };
        let state = project::bump(&original.state, options.release, 1, note)?;
        project::write_json(&version_path, &state)?;
        match Project::load(root, id) {
            Ok(p) => p,
            Err(e) => {
                restore_version(&version_path, previous.as_deref())?;
                return Err(e);
            }
        }
    } else {
        original
    };
    if options.bump || options.release {
        for path in [p.assets.clone(), p.dist.clone(), zip_path(&p)] {
            cleanup.push((path.clone(), path.exists()));
        }
    }
    println!("配方 {} → {}", p.id, p.name);
    println!("发布目录 {}", p.assets.display());
    let result = (|| {
        p.ensure_dirs()?;
        for stage in STAGES {
            if options.skip.iter().any(|s| s == stage)
                || (!options.only.is_empty() && !options.only.iter().any(|s| s == stage))
            {
                continue;
            }
            println!("\n---- {stage} ----");
            if crate::coordinate_pipeline::enabled(&p) {
                match stage {
                    "terrain" => crate::coordinate_pipeline::prepare(&p)?,
                    "passability" | "spatial" => {
                        crate::coordinate_pipeline::audit(&p)?;
                    }
                    "map" => crate::coordinate_pipeline::assemble(&p)?,
                    "verify" => {
                        crate::coordinate_pipeline::verify(&p)?;
                    }
                    "preview" => crate::coordinate_pipeline::preview(&p)?,
                    "coordinates" => {
                        crate::coordinates::generate(&p)?;
                    }
                    "accept" => crate::coordinate_pipeline::accept(&p)?,
                    "readme" => crate::coordinate_pipeline::readme(&p)?,
                    "dist" => dist(&p, !options.no_zip)?,
                    "wb" => export_wb(&p, options)?,
                    _ => unreachable!(),
                }
                continue;
            }
            match stage {
                "terrain" => {
                    let t = Terrain::generate(&p)?;
                    t.save(&p)?;
                    println!(
                        "{}×{}，{} 条斜坡，{} 个矿脉",
                        t.n,
                        t.n,
                        t.layout.ramps.len(),
                        t.layout.ore_cells.len()
                    );
                }
                "passability" => {
                    let t = Terrain::load(&p)?;
                    let s = Surface::generate(&t, &p)?;
                    connectivity(&t, &s, &p)?;
                }
                "map" => {
                    let t = Terrain::load(&p)?;
                    let s = Surface::generate(&t, &p)?;
                    assemble(&p, &t, &s)?;
                }
                "verify" => {
                    verify(&p)?;
                }
                "spatial" => {}
                "coordinates" => {
                    crate::coordinates::generate(&p)?;
                }
                "preview" => {
                    let t = Terrain::load(&p)?;
                    let s = Surface::generate(&t, &p)?;
                    render::previews(&t, &p, &s)?;
                }
                "accept" => {
                    accept(&p)?;
                }
                "readme" => {
                    readme(&p)?;
                }
                "dist" => {
                    dist(&p, !options.no_zip)?;
                }
                "wb" => {
                    export_wb(&p, options)?;
                }
                _ => unreachable!(),
            }
        }
        if options.install {
            if crate::coordinate_pipeline::enabled(&p) {
                crate::coordinate_pipeline::accept(&p)?;
            }
            install(&p)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        if options.bump || options.release {
            restore_version(&version_path, previous.as_deref())?;
            for (path, existed) in cleanup {
                if !existed && path.exists() {
                    if path.is_dir() {
                        fs::remove_dir_all(path)?;
                    } else {
                        fs::remove_file(path)?;
                    }
                }
            }
            eprintln!(
                "构建失败，版本号已回滚至 {}",
                project::format_version(&project::load_version(root, id)?)?
            );
        }
        return Err(error);
    }
    Ok(p)
}
fn restore_version(path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    match bytes {
        Some(b) => project::atomic_write(path, b),
        None => {
            if path.exists() {
                fs::remove_file(path)?;
            }
            Ok(())
        }
    }
}
pub fn assemble(p: &Project, t: &Terrain, s: &Surface) -> Result<()> {
    let template = p.string("TEMPLATE", "8p_template.map");
    project::safe_component(template)?;
    let mut m = MapFile::load(p.root.join("templates").join(template))?;
    let template_hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let template_bt = BlendTiles::parse(
        m.get("BlendTileData")?,
        template_hm.width,
        template_hm.height,
    )?;
    scripts::apply(&mut m, p)?;
    let mut objects = if p.flag("TERRAIN_ONLY") {
        Vec::new()
    } else {
        t.layout
            .objects
            .iter()
            .map(|o| o.object(p))
            .collect::<Vec<_>>()
    };
    crate::script_spawns::append(p, t, s, &mut objects)?;
    if !p.flag("TERRAIN_ONLY") {
        let nodes = scripts::parse_nodes(&m.get("PlayerScriptsList")?.payload, &m.table)?;
        let report = crate::production_spawns::repair(
            p,
            t.n,
            &t.z,
            &s.impassable,
            &s.build_block,
            &nodes,
            &mut objects,
        )?;
        project::write_json(&p.assets.join("production-spawns.json"), &report)?;
        let mut layout = t.layout.clone();
        for (source, actual) in layout.objects.iter_mut().zip(&objects) {
            let before = source.cell;
            source.cell = crate::script_spawns::to_cell(p, actual);
            if before != source.cell {
                for c in &mut layout.shipyard_cells {
                    if *c == before {
                        *c = source.cell;
                    }
                }
            }
        }
        project::write_json(&p.source().join("layout.json"), &layout)?;
    }
    // The object layout is declarative. IDs and all property keys are linked here.
    for o in &objects {
        for prop in &o.props {
            m.id(&prop.key)?;
        }
    }
    if p.flag("TERRAIN_ONLY") {
        let node = crate::scripts::parse_nodes(&m.get("PlayerScriptsList")?.payload, &m.table)?;
        let _ = node;
        objects.clear();
    }
    m.get_mut("HeightMapData")?.payload = t.heightmap(p).encode()?;
    m.get_mut("HeightMapData")?.version = 6;
    let blend = s.encode(t.n, &template_bt.blend_descriptions)?;
    m.get_mut("BlendTileData")?.payload = blend;
    m.get_mut("BlendTileData")?.version = 27;
    let payload = map::write_objects(&objects, &m)?;
    m.get_mut("ObjectsList")?.payload = payload;
    if !p.flag("TERRAIN_ONLY") {
        let report = crate::gift_clearance::apply(p, &mut m)?;
        project::write_json(&p.assets.join("gift-clearance.json"), &report)?;
        let mut spawn_report: serde_json::Value =
            serde_json::from_slice(&fs::read(p.assets.join("production-spawns.json"))?)?;
        crate::production_spawns::refresh_report(p, &m, &mut spawn_report)?;
        project::write_json(&p.assets.join("production-spawns.json"), &spawn_report)?;
        let actual = map::objects(m.get("ObjectsList")?, &m.table)?;
        let mut layout = t.layout.clone();
        for (source, object) in layout.objects.iter_mut().zip(&actual) {
            source.cell = crate::script_spawns::to_cell(p, object);
        }
        for object in actual
            .iter()
            .filter(|o| crate::gift_clearance::is_alternate(o))
        {
            if !layout.objects.iter().any(|o| {
                prop(&o.props, "waypointName").and_then(Value::text)
                    == prop(&object.props, "waypointName").and_then(Value::text)
            }) {
                layout.objects.push(crate::terrain::ObjectSpec {
                    cell: crate::script_spawns::to_cell(p, object),
                    version: object.version,
                    angle: object.angle,
                    road_type: object.road_type,
                    type_name: object.type_name.clone(),
                    props: object.props.clone(),
                });
            }
        }
        project::write_json(&p.source().join("layout.json"), &layout)?;
    }
    if !p.flag("TERRAIN_ONLY")
        && let Some(report) = crate::periodic_attack::apply(p, &mut m)?
    {
        project::write_json(&p.assets.join("periodic-attack.json"), &report)?;
    }
    if !p.flag("TERRAIN_ONLY") {
        let report = crate::production_facing::apply(p, &mut m)?;
        project::write_json(&p.assets.join("production-facing.json"), &report)?;
    }
    let mut props = map::read_props(&mut Reader::new(&m.get("WorldInfo")?.payload), &m.table)?;
    let tokens = s
        .textures
        .iter()
        .flat_map(|r| [r[1].as_str(), r[2].as_str()])
        .collect::<Vec<_>>()
        .join(";")
        + ";";
    for pr in &mut props {
        match pr.key.as_str() {
            "terrainTextureStrings" => pr.value = Value::Ascii(tokens.clone()),
            "mapName" | "mapDescription" => pr.value = Value::Ascii(String::new()),
            "weather" => pr.value = Value::Int(0),
            _ => {}
        }
    }
    let mut w = Writer::new();
    map::write_props(&mut w, &props, &m)?;
    m.get_mut("WorldInfo")?.payload = w.data;
    m.get_mut("StandingWaterAreas")?.payload = water(p)?;
    let resources = crate::resources::generate(&mut m, p)?;
    fs::write(p.assets.join("map.xml"), resources.xml)?;
    fs::write(p.assets.join("overrides.xml"), resources.overrides)?;
    // Share the accepted finished-map rules with ordinary builds. Apply after
    // objects, gameplay masks and resource IDs are fixed, so only materials vary.
    let textured = if crate::texture_polish::enabled(p)? {
        Some(crate::texture_polish::apply(p, &mut m, &p.name)?)
    } else {
        None
    };
    let path = p.assets.join(format!("{}.map", p.name));
    m.save(&path)?;
    if let Some(textured) = textured {
        textured.write_artifacts(p)?;
    }
    render::art(t, p, s)?;
    map_str(p)?;
    println!(
        "写入 {} ({} 字节，{} 个对象)",
        path.display(),
        fs::metadata(&path)?.len(),
        objects.len()
    );
    Ok(())
}
fn water(p: &Project) -> Result<Vec<u8>> {
    let cfg = &p.cfg["WATER"];
    let string = |k: &str, default: &str| cfg[k].as_str().unwrap_or(default).to_string();
    let mut w = Writer::new();
    w.u32(1);
    w.u32(1);
    w.s16("New Water Area")?;
    w.s16("")?;
    w.f32(cfg["uv_scroll"].as_f64().unwrap_or(0.06) as f32);
    w.u8(cfg["additive"].as_u64().unwrap_or(0) as u8);
    w.s16(&string("bump", "WaterRippleBump"))?;
    w.s16(&string("sky", "SkyEnv"))?;
    w.u32(4);
    let side =
        (p.usize("GRID", 840) - p.usize("BORDER", 20) * 2) as f32 * p.number("CELL", 10.) as f32;
    for [x, y] in [[0., 0.], [0., side], [side, side], [side, 0.]] {
        w.f32(x);
        w.f32(y);
    }
    w.u32(
        cfg["height"]
            .as_f64()
            .filter(|v| *v != 0.)
            .unwrap_or(p.number("WATER_Z", 200.)) as u32,
    );
    w.s16(&string("fx", "FXOceanRA3"))?;
    w.s16(&string("depth_colors", "LUTDepthTint.tga"))?;
    Ok(w.data)
}
fn map_str(p: &Project) -> Result<()> {
    let mut out = format!(
        "Intro:String\n{}\nEnd\n\nDebug:Spectate\n{}\nEnd\n\n",
        p.render(p.string("MAP_STR_INTRO", "\"\"")),
        p.render(p.string("MAP_STR_DEBUG", "\"\""))
    );
    if let Some(info) = p.cfg["MAP_STR_INFO"].as_object() {
        for (key, value) in info {
            out.push_str(&format!(
                "Info:{key}\n\"{}\"\nEnd\n\n",
                p.render(
                    value
                        .as_str()
                        .ok_or_else(|| err("MAP_STR_INFO values must be strings"))?
                )
            ));
        }
    }
    for i in 1..=8 {
        out.push_str(&format!("COUNTER:AI{i}\n\"AI{i}\"\nEnd\n\n"));
    }
    if !p.string("SERIES_NAME", "").is_empty() {
        out.push_str(&format!(
            "Series:Name\n\"{}\"\nEnd\n\nMap:DisplayName\n\"{}\"\nEnd\n\nMap:Description\n\"{}\"\nEnd\n\n",
            p.string("SERIES_NAME", ""), p.name, p.string("MAP_DESCRIPTION", &p.group)
        ));
    }
    crate::periodic_attack::append_strings(p, &mut out)?;
    fs::write(p.assets.join("map.str"), out)?;
    Ok(())
}
pub fn verify(p: &Project) -> Result<MapFile> {
    if crate::coordinate_pipeline::enabled(p) {
        return crate::coordinate_pipeline::verify(p);
    }
    let m = MapFile::load(p.assets.join(format!("{}.map", p.name)))?;
    m.validate_release()?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    if hm.width != p.usize("GRID", 0)
        || hm.height != hm.width
        || hm.border as usize != p.usize("BORDER", 0)
    {
        return Err(err("map dimensions do not match recipe"));
    }
    let limit = (hm.width - 2 * hm.border as usize) as f32 * p.number("CELL", 10.) as f32;
    let mut names = BTreeSet::new();
    for o in &objects {
        if !o.angle.is_finite()
            || o.position.iter().any(|v| !v.is_finite())
            || o.position[0] < 0.
            || o.position[1] < 0.
            || o.position[0] > limit
            || o.position[1] > limit
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
        crate::script_spawns::validate(p, &m, &hm, &bt, &objects)?;
        crate::periodic_attack::validate(p, &m)?;
        crate::production_facing::validate(p, &m)?;
    }
    scripts::validate(&m)?;
    crate::resources::validate_manifest(&m)?;
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
    Ok(m)
}
pub fn connectivity(t: &Terrain, s: &Surface, p: &Project) -> Result<()> {
    let n = t.n;
    let water = p.number("WATER_Z", 200.);
    let mut lab = vec![0usize; n * n];
    let mut sizes = vec![0];
    let mut q = VecDeque::new();
    for i in 0..n * n {
        if lab[i] != 0 || s.impassable[i] || t.z[i] < water {
            continue;
        }
        let label = sizes.len();
        let mut count = 0;
        lab[i] = label;
        q.push_back(i);
        while let Some(j) = q.pop_front() {
            count += 1;
            for k in neighbors(j, n) {
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
pub fn neighbors(i: usize, n: usize) -> impl Iterator<Item = usize> {
    let x = i % n;
    let y = i / n;
    [
        (x > 0).then(|| i - 1),
        (x + 1 < n).then_some(i + 1),
        (y > 0).then(|| i - n),
        (y + 1 < n).then_some(i + n),
    ]
    .into_iter()
    .flatten()
}
pub fn accept(p: &Project) -> Result<()> {
    if crate::coordinate_pipeline::enabled(p) {
        return crate::coordinate_pipeline::accept(p);
    }
    let m = verify(p)?;
    let t = Terrain::load(p)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    if hm.elevations != terrain::flip(&t.raw, t.n) {
        return Err(err("heightmap axis/content differs from terrain input"));
    }
    let bt = BlendTiles::parse(m.get("BlendTileData")?, t.n, t.n)?;
    let imp = terrain::flip(&bt.impassability, t.n);
    let n = t.n;
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
                    while yy >= 0 && (yy as usize) < n && (t.z[yy as usize * n + x] - z).abs() < 3.
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
            for yy in y.saturating_sub(r)..=(y + r).min(n - 1) {
                for xx in x.saturating_sub(r)..=(x + r).min(n - 1) {
                    let i = yy * n + xx;
                    if terrain::disk(i, n, ore, build_r) {
                        lo = lo.min(t.z[i]);
                        hi = hi.max(t.z[i]);
                        blocked |= imp[i] || bt.buildability[(n - 1 - yy) * n + xx];
                    }
                }
            }
            let d = [
                ore[0] - border as f64,
                ore[1] - border as f64,
                (n - 1 - border) as f64 - ore[0],
                (n - 1 - border) as f64 - ore[1],
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
        let span =
            (terrain::number(r, "w", 28.).min(terrain::number(r, "h", 28.)) * 0.5 - 1.).max(4.);
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
        let dirt = (0..=(2. * half / 8.) as usize).any(|j| {
            let v = -half + j as f64 * 8.;
            let x = (cx + ux * v).round_ties_even().clamp(0., (n - 1) as f64) as usize;
            let y = (cy + uy * v).round_ties_even().clamp(0., (n - 1) as f64) as usize;
            bt.textures
                .get(bt.tiles[(n - 1 - y) * n + x] as usize / 64)
                .is_some_and(|s| s == "Dirt_Yucatan06")
        });
        if along > 6.05 || gap > 12. || trans > 10. || monotone || !dirt {
            failures.push(format!(
                "斜坡 #{}：沿坡 {along:.2}，端缝 {gap:.1}，横向 {trans:.1}，单调 {}，土路 {dirt}",
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
fn sample(z: &[f64], n: usize, x: f64, y: f64) -> f64 {
    let x = x.clamp(0., (n - 1) as f64);
    let y = y.clamp(0., (n - 1) as f64);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(n - 1);
    let y1 = (y0 + 1).min(n - 1);
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    (z[y0 * n + x0] * (1. - fx) + z[y0 * n + x1] * fx) * (1. - fy)
        + (z[y1 * n + x0] * (1. - fx) + z[y1 * n + x1] * fx) * fy
}
pub fn game_files(p: &Project) -> [String; 3] {
    [
        format!("{}.map", p.name),
        format!("{}_art.tga", p.name),
        "map.str".into(),
    ]
}
pub fn zip_path(p: &Project) -> PathBuf {
    let mut name = p.dist.as_os_str().to_os_string();
    name.push(".zip");
    name.into()
}
pub fn release_files(p: &Project) -> Vec<String> {
    let mut names = game_files(p).to_vec();
    for name in ["map.xml", "overrides.xml", "NewWorldBuilder.notes.json"] {
        if p.assets.join(name).is_file() {
            names.push(name.into());
        }
    }
    names
}
pub fn dist(p: &Project, zip: bool) -> Result<()> {
    if crate::coordinate_pipeline::enabled(p) {
        crate::coordinate_pipeline::accept(p)?;
    }
    let mut files = Vec::new();
    for name in release_files(p) {
        files.push((
            name.clone(),
            fs::read(p.assets.join(&name))
                .map_err(|e| err(format!("missing game file {name}: {e}")))?,
        ));
    }
    crate::release::validate_files(&files)?;
    let parent = p.dist.parent().unwrap();
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".{}.{}.staging", p.name, std::process::id()));
    if staging.exists() {
        return Err(err("distribution staging path already exists"));
    }
    fs::create_dir(&staging)?;
    let result = (|| {
        for (name, data) in &files {
            fs::write(staging.join(name), data)?;
        }
        if p.dist.exists() {
            fs::remove_dir_all(&p.dist)?;
        }
        fs::rename(&staging, &p.dist)?;
        if zip {
            let entries = files
                .into_iter()
                .map(|(n, d)| (format!("{}/{n}", p.name), d))
                .collect::<Vec<_>>();
            project::atomic_write(&zip_path(p), &archive::zip(&entries)?)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}
pub fn install(p: &Project) -> Result<()> {
    let appdata = std::env::var_os("APPDATA")
        .ok_or_else(|| err("--install requires APPDATA (the game's Windows profile)"))?;
    let dst = PathBuf::from(appdata)
        .join("Red Alert 3/Maps")
        .join(&p.name);
    let files = release_files(p)
        .into_iter()
        .map(|n| fs::read(p.assets.join(&n)).map(|b| (n, b)))
        .collect::<std::io::Result<Vec<_>>>()?;
    crate::release::validate_files(&files)?;
    fs::create_dir_all(&dst)?;
    for (n, b) in files {
        fs::write(dst.join(n), b)?;
    }
    println!("已安装到 {}", dst.display());
    Ok(())
}
pub fn readme(p: &Project) -> Result<()> {
    let m = verify(p)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let prose = fs::read_to_string(p.map_dir().join("README.md")).unwrap_or_default();
    let mut counts = BTreeMap::new();
    for o in &objects {
        *counts.entry(o.type_name.as_str()).or_insert(0usize) += 1;
    }
    let mut text = format!(
        "# {}\n\n版本：{}\n\n![地图预览](preview.png)\n\n[全要素编号预览](preview--layout.png) · [编号与布局位置索引](layout-index.json)\n\n可玩区 {}×{} 格；高度图 {}×{}；对象 {} 个。\n\n配方：`maps/{}/config.json`；布局：`layout.json`。\n\n{}\n\n## 对象统计\n\n| 类型 | 数量 |\n|---|---|\n",
        p.name,
        p.version,
        hm.width - hm.border as usize * 2,
        hm.height - hm.border as usize * 2,
        hm.width,
        hm.height,
        objects.len(),
        p.id,
        prose.trim()
    );
    for (name, n) in counts {
        text.push_str(&format!("| {name} | {n} |\n"));
    }
    if crate::texture_polish::enabled(p)? {
        text.push_str("\n纹理采用通用 `classic-natural` 规则（已确认的 T2.2.9 方案）。[纹理核对](texture-report.json)");
        if p.assets.join("textures--comparison.png").is_file() {
            text.push_str(" · [材质分布对比](textures--comparison.png)");
        }
        text.push_str("。材质分布图使用示意颜色，游戏贴图细节仍应通过真实渲染或游戏检查。\n");
    }
    text.push_str("\n## 安装\n\n将发布包目录复制到 `%APPDATA%\\Red Alert 3\\Maps\\`，或运行 `cargo run --release -- build --map ");
    text.push_str(&p.id);
    text.push_str(" --install`。\n");
    fs::write(p.assets.join("README.md"), text)?;
    Ok(())
}
pub fn export_wb(p: &Project, options: &Options) -> Result<()> {
    let m = verify(p)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let dir = p.assets.join("wb");
    fs::create_dir_all(&dir)?;
    let mut heights = BufWriter::new(fs::File::create(dir.join("heightmap.xyz"))?);
    for y in 0..hm.height {
        for x in 0..hm.width {
            writeln!(
                heights,
                "{x} {y} {:.6}",
                hm.elevations[y * hm.width + x] as f64 * p.number("Z_SCALE", 1.)
            )?;
        }
    }
    heights.flush()?;
    let mut simple = String::new();
    let mut exact = String::from("# type,world_x,world_y,angle_degrees\n");
    for o in objects {
        if o.type_name.contains("Waypoint") {
            continue;
        }
        simple.push_str(&format!(
            "{},{:.6},{:.6}\n",
            o.type_name,
            o.position[0] as f64 / p.number("CELL", 10.),
            o.position[1] as f64 / p.number("CELL", 10.)
        ));
        exact.push_str(&format!(
            "{},{:.6},{:.6},{:.9}\n",
            o.type_name,
            o.position[0],
            o.position[1],
            o.angle.to_degrees()
        ));
    }
    fs::write(dir.join("object.txt"), simple)?;
    fs::write(dir.join("chai_objects.txt"), exact)?;
    let plugins = dir.join("plugins/CHAI_Import");
    fs::create_dir_all(&plugins)?;
    for name in ["Main.cs", "plugin_meta.json"] {
        fs::copy(
            p.root.join("templates/worldbuilder").join(name),
            plugins.join(name),
        )?;
    }
    fs::copy(
        p.root.join("templates/worldbuilder/CHAI_OptimizeMap.cs"),
        dir.join("CHAI_OptimizeMap.cs"),
    )?;
    fs::write(
        dir.join("README_导入说明.md"),
        "# WorldBuilder 导入\n\n`heightmap.xyz` 使用全图格坐标与世界高度。`object.txt` 用于内置导入器。\n\n安装 `plugins/CHAI_Import` 并把 `chai_objects.txt` 放入地编工作目录，可保留精确角度。\n",
    )?;
    if options.wb_install {
        let root = options
            .wb_root
            .clone()
            .or_else(|| std::env::var_os("RA3MAP_WB_ROOT").map(PathBuf::from))
            .ok_or_else(|| err("--wb requires --wb-root or RA3MAP_WB_ROOT"))?;
        for (src, dst) in [
            (
                dir.join("heightmap.xyz"),
                root.join("data/scripts/ImportXYZ/heightmap.xyz"),
            ),
            (
                dir.join("object.txt"),
                root.join("data/scripts/loadObjFromFiles/object.txt"),
            ),
            (dir.join("chai_objects.txt"), root.join("chai_objects.txt")),
            (
                plugins.join("Main.cs"),
                root.join("data/plugins/CHAI_Import/Main.cs"),
            ),
            (
                plugins.join("plugin_meta.json"),
                root.join("data/plugins/CHAI_Import/plugin_meta.json"),
            ),
        ] {
            fs::create_dir_all(dst.parent().unwrap())?;
            fs::copy(src, dst)?;
        }
    }
    Ok(())
}
