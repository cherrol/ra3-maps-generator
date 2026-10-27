use ra3maps::{
    Result,
    build::{self, Options},
    err,
    map::{self, BlendTiles, HeightMap, MapFile},
    project::{self, Project},
    refpack, scripts,
    terrain::Terrain,
};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const HELP: &str = "RA3 地图工坊 · 原生 Rust\n\n用法：ra3maps <命令> [参数]\n\n  build                 完整构建：地形 → 通行性 → 地图 → 校验 → 预览 → 验收 → 说明 → 发布 → WB\n  repack <源ZIP>        修复成品封装并递增版本（配合 --map、--bump）\n  relocate-ramps <ZIP> <方案JSON> 增设或移动指定斜坡并更新局部地形（--map、--bump）\n  polish-ramps <源ZIP>  局部优化斜坡并发布新版本（配合 --map、--bump）\n  polish-textures <源ZIP> 自然地表纹理美化并发布新版本（--map、--bump）\n  update-scripts <源ZIP> 更新本图定时进攻脚本（--map、--bump）\n  repair-production <源ZIP> 修复赠送重工/船厂的水陆落点（--map、--bump）\n  repair-resources <源ZIP> 重建成品资源清单、图集及XML（--map、--bump）\n  list                  列出 maps/ 中的配方\n  version               查看或修改地图版本\n  terrain / passability / map / verify / preview / accept / readme / dist / wb\n                        单独执行构建阶段\n  inspect <文件>         检查地图区块、地形、纹理和对象\n  dump-scripts <文件>    导出脚本树 JSON\n  check-bounds <文件>    校验对象边界\n  unpack <输入> <输出>  EAR → CkMp（路径 - 表示标准输入/输出）\n  pack <输入> <输出>    CkMp → EAR\n  refpack <compress|decompress|literals> <输入> <输出>\n  spatial               坐标占地、资源通路与纹理检查（已启用的地图）\n  adopt-coordinate-source 将当前完整成品设为坐标制作基准（--map）\n  coordinates           导出离线坐标地图、层界与对象清单（--map）\n  analyze-terrain        统计当前地图地形\n\n构建参数：\n  --map <id>            配方（默认 RA3MAP_ID / maps/.default）\n  --root <目录>         工程根目录（可从任意工作目录调用）\n  --only <阶段>         只运行指定阶段，可重复\n  --skip <阶段>         跳过指定阶段，可重复\n  --bump                版本 +0.0.1，失败自动回滚并清理新产物\n  --release             配合构建转正式版并递增\n  --note <说明>         写入版本历史\n  --no-zip              发布时不生成 ZIP\n  --install             复制完整成品文件到 APPDATA/Red Alert 3/Maps\n  --wb --wb-root <目录> 导出后安装 WorldBuilder 导入文件\n\n版本参数：--show / --bump / --release / --set T0.0.1 / --note <说明>\n环境：RA3MAP_ID、RA3MAP_ROOT、RA3MAP_FONT、APPDATA、RA3MAP_WB_ROOT\n";
#[derive(Default)]
struct Args {
    root: Option<PathBuf>,
    map: Option<String>,
    opts: Options,
    set: Option<String>,
    show: bool,
    positional: Vec<String>,
}
fn parse(input: &[String]) -> Result<Args> {
    let mut a = Args::default();
    let mut i = 0;
    while i < input.len() {
        let arg = &input[i];
        let mut val = || -> Result<String> {
            i += 1;
            input
                .get(i)
                .cloned()
                .ok_or_else(|| err(format!("{arg} requires a value")))
        };
        match arg.as_str() {
            "--root" => a.root = Some(val()?.into()),
            "--map" => a.map = Some(val()?),
            "--only" => a.opts.only.push(val()?),
            "--skip" => a.opts.skip.push(val()?),
            "--bump" => a.opts.bump = true,
            "--release" => a.opts.release = true,
            "--install" => a.opts.install = true,
            "--no-zip" => a.opts.no_zip = true,
            "--wb" | "--install-wb" => a.opts.wb_install = true,
            "--wb-root" => a.opts.wb_root = Some(val()?.into()),
            "--note" => a.opts.note = val()?,
            "--set" => a.set = Some(val()?),
            "--show" => a.show = true,
            "--" => {
                a.positional.extend_from_slice(&input[i + 1..]);
                break;
            }
            "-" => a.positional.push(arg.clone()),
            s if s.starts_with('-') => return Err(err(format!("unknown option {s}"))),
            _ => a.positional.push(arg.clone()),
        }
        i += 1;
    }
    Ok(a)
}
fn find_root(a: &Args) -> Result<PathBuf> {
    if let Some(root) = a
        .root
        .clone()
        .or_else(|| env::var_os("RA3MAP_ROOT").map(PathBuf::from))
    {
        return Ok(fs::canonicalize(root)?);
    }
    let cwd = env::current_dir()?;
    for root in cwd.ancestors() {
        if root.join("Cargo.toml").is_file() && root.join("maps").is_dir() {
            return Ok(root.into());
        }
    }
    if let Ok(exe) = env::current_exe() {
        for root in exe.ancestors() {
            if root.join("maps").is_dir() && root.join("templates").is_dir() {
                return Ok(root.into());
            }
        }
    }
    Err(err("cannot find project root; use --root or RA3MAP_ROOT"))
}
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
fn main() {
    if let Err(e) = run() {
        eprintln!("错误：{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
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
    if command == "list" {
        for id in project::maps(&root)? {
            let p = Project::load(&root, &id)?;
            println!("{}  {}", p.id, p.name);
        }
        return Ok(());
    }
    let id = project::select(&root, args.map.as_deref())?;
    if command == "adopt-coordinate-source" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        ra3maps::coordinate_pipeline::adopt(&p)?;
        return Ok(());
    }
    if command == "coordinates" {
        paths(&args, 0)?;
        let p = Project::load(&root, &id)?;
        ra3maps::coordinates::generate(&p)?;
        return Ok(());
    }
    if command == "relocate-ramps" {
        paths(&args, 2)?;
        ra3maps::release::relocate_ramps(
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
            | "repair-resources"
            | "polish-textures"
            | "repair-production"
            | "update-scripts"
    ) {
        paths(&args, 1)?;
        if command == "update-scripts" {
            ra3maps::release::update_scripts(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-production" {
            ra3maps::release::repair_production(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "polish-textures" {
            ra3maps::release::polish_textures(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "repair-resources" {
            ra3maps::release::repair_resources(
                &root,
                &id,
                Path::new(&args.positional[0]),
                &args.opts,
            )?;
        } else if command == "polish-ramps" {
            ra3maps::release::polish_ramps(&root, &id, Path::new(&args.positional[0]), &args.opts)?;
        } else {
            ra3maps::release::run(&root, &id, Path::new(&args.positional[0]), &args.opts)?;
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
