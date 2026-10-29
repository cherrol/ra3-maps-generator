//! Validated game distribution, installation and authoring exports.
use super::{Options, placement, verify};
use crate::{
    Result, err,
    formats::archive,
    formats::map::{self, HeightMap},
    pipeline::project::{self, Project},
};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
};

pub fn game_files(p: &Project) -> [String; 3] {
    [
        format!("{}.map", p.name),
        format!("{}_art.tga", p.name),
        "map.str".into(),
    ]
}
pub fn zip_path(p: &Project) -> PathBuf {
    p.dist.parent().unwrap().join(format!("{}.zip", p.name))
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

fn publication_files(p: &Project) -> Result<Vec<(String, Vec<u8>)>> {
    let files = release_files(p)
        .into_iter()
        .map(|name| {
            fs::read(p.assets.join(&name))
                .map(|bytes| (name.clone(), bytes))
                .map_err(|e| err(format!("missing game file {name}: {e}")))
        })
        .collect::<Result<Vec<_>>>()?;
    let native = crate::pipeline::release::load_runtime_map(&files)?;
    let (_, bytes) = files
        .iter()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap();
    placement::validate_native(p, bytes, &native)?;
    Ok(files)
}
pub fn dist(p: &Project, zip: bool) -> Result<()> {
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        crate::pipeline::coordinate_pipeline::accept(p)?;
    }
    let files = publication_files(p)?;
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
    let files = publication_files(p)?;
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
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        text.push_str("\n本次构建沿用冻结制作源中的地表材质。应用公共五层主纹理请从完整成品显式运行 `polish-textures`。\n");
    } else if crate::terrain::texture_polish::enabled(p)? {
        text.push_str("\n纹理采用通用 `classic-natural` 五层主纹理方案：前三层草地、第四层棕岩、第五层浅灰岩。[纹理核对](texture-report.json)");
        if p.assets.join("textures--comparison.png").is_file() {
            text.push_str(" · [材质分布对比](textures--comparison.png)");
        }
        text.push_str("。材质分布图使用示意颜色，不表示实际渲染细节。\n");
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
