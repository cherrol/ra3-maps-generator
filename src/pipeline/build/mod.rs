//! Build stages, version transactions and native map assembly.
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    gameplay::scripts,
    pipeline::project::{self, Project},
    render,
    terrain::Terrain,
    terrain::surface::Surface,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
mod distribution;
pub(crate) mod placement;
mod validation;
pub use distribution::{dist, export_wb, game_files, install, readme, release_files, zip_path};
pub use placement::{placement_proof, validate_placement};
pub use validation::{accept, connectivity, verify};

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
    let selected_layers = crate::terrain::layers::SelectedLayers::load(&original)?;
    if !crate::pipeline::coordinate_pipeline::enabled(&original)
        && options.only.iter().any(|s| s == "spatial")
    {
        return Err(err("this map has not enabled COORDINATE_PIPELINE"));
    }
    if crate::pipeline::coordinate_pipeline::enabled(&original) {
        crate::pipeline::coordinate_pipeline::preflight(&original)?;
    }
    crate::terrain::texture_polish::enabled(&original)?;
    crate::terrain::ramps::enabled(&original)?;
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
        crate::coordinates::prepare(&p)?;
        for stage in STAGES {
            if options.skip.iter().any(|s| s == stage)
                || (!options.only.is_empty() && !options.only.iter().any(|s| s == stage))
            {
                continue;
            }
            println!("\n---- {stage} ----");
            run_stage(&p, stage, options)?;
        }
        if options.install && crate::pipeline::coordinate_pipeline::enabled(&p) {
            crate::pipeline::coordinate_pipeline::accept(&p)?;
        }
        selected_layers.sync_current(&p)?;
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
    if options.install {
        install(&p).map_err(|e| err(format!("构建已完成，安装失败：{e}")))?;
    }
    Ok(p)
}
fn run_stage(p: &Project, stage: &str, options: &Options) -> Result<()> {
    let frozen = crate::pipeline::coordinate_pipeline::enabled(p);
    match stage {
        "terrain" if frozen => crate::pipeline::coordinate_pipeline::prepare(p)?,
        "terrain" => {
            let t = Terrain::generate(p)?;
            t.save(p)?;
            println!(
                "{}×{}，{} 条斜坡，{} 个矿脉",
                t.n,
                t.height(),
                t.layout.ramps.len(),
                t.layout.ore_cells.len()
            );
        }
        "passability" | "spatial" if frozen => {
            crate::pipeline::coordinate_pipeline::audit(p)?;
        }
        "passability" => {
            let t = Terrain::load(p)?;
            let s = Surface::generate(&t, p)?;
            connectivity(&t, &s, p)?;
        }
        "map" if frozen => crate::pipeline::coordinate_pipeline::assemble(p)?,
        "map" => {
            let t = Terrain::load(p)?;
            let s = Surface::generate(&t, p)?;
            assemble(p, &t, &s)?;
        }
        "verify" => {
            verify(p)?;
        }
        "spatial" => {}
        "preview" if frozen => crate::pipeline::coordinate_pipeline::preview(p)?,
        "preview" => {
            let t = Terrain::load(p)?;
            let s = Surface::generate(&t, p)?;
            render::previews(&t, p, &s)?;
        }
        "coordinates" => {
            crate::coordinates::generate(p)?;
        }
        "accept" => accept(p)?,
        "readme" if frozen => crate::pipeline::coordinate_pipeline::readme(p)?,
        "readme" => readme(p)?,
        "dist" => dist(p, !options.no_zip)?,
        "wb" => export_wb(p, options)?,
        _ => unreachable!(),
    }
    Ok(())
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
    crate::coordinates::prepare(p)?;
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
    crate::gameplay::script_spawns::append(p, t, s, &mut objects)?;
    if !p.flag("TERRAIN_ONLY") {
        let nodes = scripts::parse_nodes(&m.get("PlayerScriptsList")?.payload, &m.table)?;
        let report = crate::gameplay::production_spawns::repair(
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
            source.cell = crate::gameplay::script_spawns::to_cell(p, actual);
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
    m.get_mut("HeightMapData")?.payload = t.heightmap(p).encode()?;
    m.get_mut("HeightMapData")?.version = 6;
    let blend = s.encode(t.n, &template_bt.blend_descriptions)?;
    m.get_mut("BlendTileData")?.payload = blend;
    m.get_mut("BlendTileData")?.version = 27;
    let payload = map::write_objects(&objects, &m)?;
    m.get_mut("ObjectsList")?.payload = payload;
    if !p.flag("TERRAIN_ONLY") {
        let report = crate::gameplay::gift_clearance::apply(p, &mut m)?;
        project::write_json(&p.assets.join("gift-clearance.json"), &report)?;
        let mut spawn_report: serde_json::Value =
            serde_json::from_slice(&fs::read(p.assets.join("production-spawns.json"))?)?;
        crate::gameplay::production_spawns::refresh_report(p, &m, &mut spawn_report)?;
        project::write_json(&p.assets.join("production-spawns.json"), &spawn_report)?;
        let actual = map::objects(m.get("ObjectsList")?, &m.table)?;
        let mut layout = t.layout.clone();
        for (source, object) in layout.objects.iter_mut().zip(&actual) {
            source.cell = crate::gameplay::script_spawns::to_cell(p, object);
        }
        for object in actual
            .iter()
            .filter(|o| crate::gameplay::gift_clearance::is_alternate(o))
        {
            if !layout.objects.iter().any(|o| {
                prop(&o.props, "waypointName").and_then(Value::text)
                    == prop(&object.props, "waypointName").and_then(Value::text)
            }) {
                layout.objects.push(crate::terrain::ObjectSpec {
                    cell: crate::gameplay::script_spawns::to_cell(p, object),
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
        && let Some(report) = crate::gameplay::periodic_attack::apply(p, &mut m)?
    {
        project::write_json(&p.assets.join("periodic-attack.json"), &report)?;
    }
    if !p.flag("TERRAIN_ONLY") {
        let report = crate::gameplay::production_facing::apply(p, &mut m)?;
        project::write_json(&p.assets.join("production-facing.json"), &report)?;
    }
    let mut props = map::read_props(&mut Reader::new(&m.get("WorldInfo")?.payload), &m.table)?;
    for pr in &mut props {
        match pr.key.as_str() {
            "mapName" | "mapDescription" => pr.value = Value::Ascii(String::new()),
            "weather" => pr.value = Value::Int(0),
            _ => {}
        }
    }
    let mut w = Writer::new();
    map::write_props(&mut w, &props, &m)?;
    m.get_mut("WorldInfo")?.payload = w.data;
    m.get_mut("StandingWaterAreas")?.payload = water(p)?;
    // Share the accepted finished-map rules with ordinary builds. Apply after
    // objects and gameplay masks are fixed, so only materials vary.
    let textured = if crate::terrain::texture_polish::enabled(p)? {
        Some(crate::terrain::texture_polish::apply(p, &mut m, &p.name)?)
    } else {
        None
    };
    let resources = crate::resources::generate(&mut m, p)?;
    fs::write(p.assets.join("overrides.xml"), resources.overrides)?;
    // Explicit legacy reproduction keeps its frozen lookup-table bytes. The
    // ordinary terrain editors always finalize their newly written blends.
    let finalization = if textured.is_some() || crate::terrain::ramps::enabled(p)? {
        crate::resources::TerrainFinalization::CompactLookups
    } else {
        crate::resources::TerrainFinalization::PreserveLookups
    };
    let finalized = crate::resources::finalize_terrain_package(
        p,
        &mut m,
        std::str::from_utf8(&resources.xml)?,
        &p.version,
        finalization,
    )?;
    fs::write(p.assets.join("map.xml"), finalized.xml)?;
    project::write_json(
        &p.assets.join("terrain-package.json"),
        &finalized.compaction,
    )?;
    let clearance = crate::terrain::ramp_clearance::validate(p, &m, &t.layout)?;
    project::write_json(&p.assets.join("ramp-clearance.json"), &clearance)?;
    let spawns = crate::terrain::spawn_placement::validate(p, &m, &t.layout)?;
    project::write_json(&p.assets.join("spawn-placement.json"), &spawns)?;
    let allocation = crate::terrain::resource_routes::validate(p, &m)?;
    project::write_json(&p.assets.join("resource-distribution.json"), &allocation)?;
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
    let height = (p.usize("GRID_HEIGHT", p.usize("GRID", 840)) - p.usize("BORDER", 20) * 2) as f32
        * p.number("CELL", 10.) as f32;
    for [x, y] in [[0., 0.], [0., height], [side, height], [side, 0.]] {
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
    crate::gameplay::periodic_attack::append_strings(p, &mut out)?;
    fs::write(p.assets.join("map.str"), out)?;
    Ok(())
}
