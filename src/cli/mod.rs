//! 命令调度和输入输出。
mod args;
mod help;
use args::{Args, find_root, parse};
use help::HELP;
use ra3maps::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile},
    formats::refpack,
    gameplay::scripts,
    pipeline::build,
    pipeline::project::{self, Project},
    terrain::Terrain,
};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::Path,
};

fn input(path: &str) -> Result<Vec<u8>> {
    if path == "-" {
        let mut b = Vec::new();
        io::stdin().read_to_end(&mut b)?;
        Ok(b)
    } else {
        Ok(fs::read(path)?)
    }
}
fn output(path: &str, data: &[u8]) -> Result<()> {
    if path == "-" {
        io::stdout().write_all(data)?;
    } else {
        fs::write(path, data)?;
    }
    Ok(())
}
fn paths(a: &Args, n: usize) -> Result<()> {
    if a.positional.len() != n {
        return Err(err(format!("expected {n} path arguments")));
    }
    Ok(())
}
pub(crate) fn run() -> Result<()> {
    let argv = env::args().skip(1).collect::<Vec<_>>();
    if argv.iter().any(|s| s == "--help" || s == "-h") {
        print!("{HELP}");
        return Ok(());
    }
    if argv.first().is_some_and(|s| s == "--version") {
        println!("ra3maps {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let (command, rest) = if argv.first().is_none_or(|s| s.starts_with('-')) {
        ("build", argv.as_slice())
    } else {
        (argv[0].as_str(), &argv[1..])
    };
    let mut args = parse(rest)?;
    match command {
        "check-xml" => {
            paths(&args, 1)?;
            let data = input(&args.positional[0])?;
            let report = ra3maps::resources::xml_structure::analyse(std::str::from_utf8(&data)?);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report["passed"] != true {
                return Err(err("XML structural subset check failed"));
            }
            return Ok(());
        }
        "pack" | "unpack" => {
            paths(&args, 2)?;
            let data = input(&args.positional[0])?;
            let data = if command == "pack" {
                refpack::pack(&data)?
            } else {
                refpack::unpack(&data)?
            };
            return output(&args.positional[1], &data);
        }
        "refpack" => {
            paths(&args, 3)?;
            let data = input(&args.positional[1])?;
            let data = match args.positional[0].as_str() {
                "compress" => refpack::compress(&data)?,
                "decompress" => refpack::decompress(&data)?,
                "literals" => refpack::literals(&data)?,
                s => return Err(err(format!("unknown RefPack operation {s}"))),
            };
            return output(&args.positional[2], &data);
        }
        "inspect" | "dump-chunks" | "dump-scripts" | "check-bounds" => {
            paths(&args, 1)?;
            let m = MapFile::load(&args.positional[0])?;
            if command == "dump-scripts" {
                let nodes = scripts::parse_nodes(&m.get("PlayerScriptsList")?.payload, &m.table)?;
                println!("{}", serde_json::to_string_pretty(&nodes)?);
                return Ok(());
            }
            let hm = HeightMap::parse(m.get("HeightMapData")?)?;
            let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
            let objs = map::objects(m.get("ObjectsList")?, &m.table)?;
            let side = (hm.width - 2 * hm.border as usize) as f32 * 10.;
            if command == "check-bounds" {
                for o in &objs {
                    if o.position[0] < 0.
                        || o.position[1] < 0.
                        || o.position[0] > side
                        || o.position[1] > side
                        || o.position.iter().any(|v| !v.is_finite())
                    {
                        return Err(err(format!(
                            "object outside playable bounds: {}",
                            o.type_name
                        )));
                    }
                }
                println!("{} 个对象边界通过", objs.len());
                return Ok(());
            }
            println!("{} 个字符串，{} 个区块", m.table.len(), m.chunks.len());
            for (i, c) in m.chunks.iter().enumerate() {
                println!(
                    "{i:3} v{:3} {:10} {}",
                    c.version,
                    c.payload.len(),
                    m.table[&c.name_idx]
                );
            }
            println!(
                "地形 {}×{}，边框 {}，纹理 {}，对象 {}",
                hm.width,
                hm.height,
                hm.border,
                bt.textures.len(),
                objs.len()
            );
            return Ok(());
        }
        _ => {}
    }
    let root = find_root(&args)?;
    if command == "terrain-preview-source" {
        paths(&args, 1)?;
        if args.opts.bump
            || args.opts.release
            || args.opts.install
            || args.map.is_some()
            || !args.opts.only.is_empty()
            || !args.opts.skip.is_empty()
        {
            return Err(err(
                "terrain-preview-source 只重绘真实地形图，不接受构建、版本或安装开关",
            ));
        }
        let image = ra3maps::terrain_preview::generate_source(&root, &args.positional[0])?;
        println!("真实地形图已保存至 {}", image.display());
        return Ok(());
    }
    if command == "terrain-preview-centers" {
        paths(&args, 1)?;
        if args.opts.bump
            || args.opts.release
            || args.opts.install
            || args.map.is_some()
            || !args.opts.only.is_empty()
            || !args.opts.skip.is_empty()
        {
            return Err(err(
                "terrain-preview-centers 只生成中心标点预览，不接受构建、版本或安装开关",
            ));
        }
        let image = ra3maps::terrain_preview::generate_centers(&root, &args.positional[0])?;
        println!("中心标点预览已保存至 {}", image.display());
        return Ok(());
    }
    if command == "terrain-preview" {
        paths(&args, 2)?;
        if args.opts.bump
            || args.opts.release
            || args.opts.install
            || !args.opts.only.is_empty()
            || !args.opts.skip.is_empty()
        {
            return Err(err(
                "terrain-preview 只生成地形预览，不接受构建、版本或安装开关",
            ));
        }
        let directory = ra3maps::terrain_preview::generate(
            &root,
            &args.positional[0],
            Path::new(&args.positional[1]),
            args.map.as_deref(),
        )?;
        println!(
            "预览已保存至 {}：source.png、preview.png、preview--line.png",
            directory.display()
        );
        if directory.join("preview--center--line.png").is_file() {
            println!("中心标点预览：preview--center--line.png");
        }
        return Ok(());
    }
    if command == "list" {
        for id in project::maps(&root)? {
            let p = Project::load(&root, &id)?;
            println!("{}  {}", p.id, p.name);
        }
        return Ok(());
    }
    let id = project::select(&root, args.map.as_deref())?;
    if command == "audit-scene" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        let report = ra3maps::pipeline::scene::audit_existing(&p)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if command == "audit-gifts" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        let report = ra3maps::pipeline::scene::audit_gifts_existing(&p)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if command == "adopt-coordinate-source" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        ra3maps::pipeline::coordinate_pipeline::adopt(&p)?;
        return Ok(());
    }
    if command == "coordinates" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        ra3maps::coordinates::generate(&p)?;
        return Ok(());
    }
    if command == "add-mountains" {
        paths(&args, 2)?;
        ra3maps::pipeline::release::add_mountains(
            &root,
            &id,
            Path::new(&args.positional[0]),
            Path::new(&args.positional[1]),
            &args.opts,
        )?;
        return Ok(());
    }
    if command == "relocate-ramps" {
        paths(&args, 2)?;
        ra3maps::pipeline::release::relocate_ramps(
            &root,
            &id,
            Path::new(&args.positional[0]),
            Path::new(&args.positional[1]),
            &args.opts,
        )?;
        return Ok(());
    }
    if command == "repair-gifts" {
        paths(&args, 2)?;
        ra3maps::pipeline::release::repair_gifts(
            &root,
            &id,
            Path::new(&args.positional[0]),
            Path::new(&args.positional[1]),
            &args.opts,
        )?;
        return Ok(());
    }
    if matches!(
        command,
        "repack"
            | "polish-ramps"
            | "polish-ramp-surfaces"
            | "repair-resources"
            | "repair-terrain-atlas"
            | "repair-terrain-package"
            | "polish-textures"
            | "repair-production"
            | "update-scripts"
    ) {
        paths(&args, 1)?;
        if command == "update-scripts" {
            ra3maps::pipeline::release::update_scripts(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-production" {
            ra3maps::pipeline::release::repair_production(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "polish-textures" {
            ra3maps::pipeline::release::polish_textures(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-terrain-package" {
            ra3maps::pipeline::release::repair_terrain_package(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-terrain-atlas" {
            ra3maps::pipeline::release::repair_terrain_atlas(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-resources" {
            ra3maps::pipeline::release::repair_resources(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "polish-ramp-surfaces" {
            ra3maps::pipeline::release::polish_ramp_surfaces(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "polish-ramps" {
            ra3maps::pipeline::release::polish_ramps(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else {
            ra3maps::pipeline::release::run(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        }
        return Ok(());
    }
    if command == "version" {
        let mut st = project::load_version(&root, &id)?;
        let changed = args.set.is_some() || args.opts.bump || args.opts.release;
        if let Some(version) = args.set {
            let history = st
                .get("history")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            st = project::parse_version(&version)?;
            st["history"] = history;
            project::record(&mut st, &args.opts.note)?;
        } else if args.opts.release && !args.opts.bump && st["stage"] != "V" {
            st = project::bump(&st, true, 0, &args.opts.note)?;
        } else if args.opts.bump || args.opts.release {
            st = project::bump(&st, args.opts.release, 1, &args.opts.note)?;
        }
        if changed {
            project::write_json(&project::version_path(&root, &id), &st)?;
        }
        println!("{} {}", id, project::format_version(&st)?);
        if (args.show || !changed)
            && let Some(hist) = st["history"].as_array()
        {
            for h in hist.iter().skip(hist.len().saturating_sub(12)) {
                println!(
                    "{}  {}  {}",
                    h["version"].as_str().unwrap_or(""),
                    h["date"].as_str().unwrap_or(""),
                    h["note"].as_str().unwrap_or("")
                );
            }
        }
        return Ok(());
    }
    if command == "prepare-delivery" {
        paths(&args, 0)?;
        if args.opts.bump || args.opts.release || args.opts.install || args.opts.wb_install {
            return Err(err(
                "prepare-delivery only prepares the current candidate for review; version and installation options do not apply",
            ));
        }
        let p = Project::load(&root, &id)?;
        let files = build::release_files(&p)
            .into_iter()
            .map(|name| Ok((name.clone(), fs::read(p.assets.join(name))?)))
            .collect::<Result<Vec<_>>>()?;
        let path = ra3maps::pipeline::acceptance::stage_release(&p, &files)?;
        println!("待核验候选：{}", path.display());
        return Ok(());
    }
    if command == "analyze-terrain" {
        let p = Project::load(&root, &id)?;
        let t = Terrain::load(&p)?;
        println!(
            "{}×{} z {:.2}..{:.2}, 水域 {:.2}%, 矿 {}, 斜坡 {}",
            t.n,
            t.n,
            t.z.iter().copied().fold(f64::INFINITY, f64::min),
            t.z.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            100. * t
                .z
                .iter()
                .filter(|v| **v < p.number("WATER_Z", 200.))
                .count() as f64
                / t.z.len() as f64,
            t.layout.ore_cells.len(),
            t.layout.ramps.len()
        );
        return Ok(());
    }
    if command == "compare-terrain" {
        paths(&args, 1)?;
        let p = Project::load(&root, &id)?;
        let t = Terrain::load(&p)?;
        ra3maps::terrain::compare_raw(Path::new(&args.positional[0]), &t)?;
        println!("高度场逐格一致");
        return Ok(());
    }
    if command != "build" {
        if !build::STAGES.contains(&command) {
            return Err(err(format!("unknown command {command}; use --help")));
        }
        args.opts.only = vec![command.into()];
    }
    if !args.positional.is_empty() {
        return Err(err("unexpected positional arguments"));
    }
    if args.opts.wb_install
        && !args.opts.only.is_empty()
        && !args.opts.only.iter().any(|v| v == "wb")
    {
        args.opts.only.push("wb".into());
    }
    build::run(&root, &id, &args.opts)?;
    Ok(())
}
