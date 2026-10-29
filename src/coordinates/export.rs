//! 坐标清单、总览图和使用说明输出。
use super::{
    points::Point,
    raster::{Boundary, Layer, Raster},
};
use crate::{Result, pipeline::project::Project, render::Image};
use serde_json::Value as Json;
use std::{fs, path::Path};

pub(super) fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub(super) fn csv_row(out: &mut String, fields: &[String]) {
    out.push_str(
        &fields
            .iter()
            .map(|s| quoted(s))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');
}
pub(super) fn write_points(path: &Path, points: &[Point], conditional: bool) -> Result<()> {
    let mut out = String::from(
        "\u{feff}id,preview_id,category,subtype,name,object_type,owner,owner_seat,neutral,x,y,world_x,world_y,surface_z,object_z_raw,layer,region,in_grid,in_playable,angle_radians,waypoint\n",
    );
    for p in points.iter().filter(|p| p.conditional == conditional) {
        csv_row(
            &mut out,
            &[
                p.id.clone(),
                p.preview_id.map(|v| v.to_string()).unwrap_or_default(),
                p.category.clone(),
                p.subtype.clone(),
                p.name.clone(),
                p.object_type.clone(),
                p.owner.clone(),
                p.owner_seat.map(|v| v.to_string()).unwrap_or_default(),
                p.neutral.to_string(),
                p.grid[0].to_string(),
                p.grid[1].to_string(),
                p.world[0].to_string(),
                p.world[1].to_string(),
                p.surface_z.map(|v| v.to_string()).unwrap_or_default(),
                p.object_z.map(|v| v.to_string()).unwrap_or_default(),
                p.layer.map(|v| v.to_string()).unwrap_or_default(),
                p.region.map(|v| v.to_string()).unwrap_or_default(),
                p.in_grid.to_string(),
                p.in_playable.to_string(),
                p.angle_radians.map(|v| v.to_string()).unwrap_or_default(),
                p.waypoint.clone().unwrap_or_default(),
            ],
        );
    }
    fs::write(path, out)?;
    Ok(())
}
pub(super) fn overview(
    path: &Path,
    r: &Raster,
    layers: &[Layer],
    boundaries: &[Boundary],
    points: &[Point],
) -> Result<()> {
    let scale = 3usize;
    let left = 75;
    let top = 90;
    let bottom = points
        .iter()
        .filter(|p| !p.conditional)
        .map(|p| p.grid[1])
        .fold(r.frame.height as f64, f64::max)
        .ceil() as usize
        + 12;
    let mut image = Image::new(
        left + r.frame.width * scale + 360,
        top + bottom * scale + 50,
        [23, 29, 38],
    );
    let font = crate::render::font();
    image.text(
        font.as_ref(),
        20,
        16,
        &format!(
            "地图坐标与地形层 · X向东 / Y向南 · 1格={}游戏单位",
            r.frame.cell_size
        ),
        26.,
        [238, 239, 230],
    );
    for i in 0..r.raw.len() {
        let color = layers[r.class[i]].color;
        for yy in 0..scale {
            for xx in 0..scale {
                image.pixel(
                    (left + (i % r.frame.width) * scale + xx) as isize,
                    (top + (i / r.frame.width) * scale + yy) as isize,
                    color,
                );
            }
        }
    }
    let line = |im: &mut Image, a: [f64; 2], b: [f64; 2], color: [u8; 3]| {
        let ax = left as f64 + a[0] * scale as f64;
        let ay = top as f64 + a[1] * scale as f64;
        let bx = left as f64 + b[0] * scale as f64;
        let by = top as f64 + b[1] * scale as f64;
        let n = (bx - ax).abs().max((by - ay).abs()).ceil().max(1.) as usize;
        for j in 0..=n {
            im.pixel(
                (ax + (bx - ax) * j as f64 / n as f64) as isize,
                (ay + (by - ay) * j as f64 / n as f64) as isize,
                color,
            );
        }
    };
    for b in boundaries {
        let c = if b.kind == "shoreline" {
            [60, 190, 240]
        } else if b.kind == "cliff" {
            [45, 47, 50]
        } else {
            [183, 83, 164]
        };
        for pair in b.points.windows(2) {
            line(&mut image, pair[0], pair[1], c);
        }
    }
    for x in (0..r.frame.width).step_by(50) {
        line(
            &mut image,
            [x as f64, 0.],
            [x as f64, r.frame.height as f64],
            [130, 139, 139],
        );
        image.text(
            font.as_ref(),
            (left + x * scale) as isize,
            top as isize - 28,
            &x.to_string(),
            16.,
            [220, 225, 225],
        );
    }
    for y in (0..r.frame.height).step_by(50) {
        line(
            &mut image,
            [0., y as f64],
            [r.frame.width as f64, y as f64],
            [130, 139, 139],
        );
        image.text(
            font.as_ref(),
            8,
            (top + y * scale) as isize,
            &y.to_string(),
            16.,
            [220, 225, 225],
        );
    }
    for p in points.iter().filter(|p| {
        !p.conditional && matches!(p.category.as_str(), "资源" | "建筑" | "出生点" | "斜坡")
    }) {
        let x = (left as f64 + p.grid[0] * scale as f64) as isize;
        let y = (top as f64 + p.grid[1] * scale as f64) as isize;
        let color = match p.category.as_str() {
            "资源" => [255, 215, 30],
            "建筑" => [190, 60, 210],
            "出生点" => [40, 145, 255],
            _ => [255, 65, 45],
        };
        image.circle(x, y, 5, color);
        if p.in_grid {
            image.text(
                font.as_ref(),
                x + 5,
                y - 5,
                &p.preview_id.unwrap().to_string(),
                13.,
                color,
            );
        }
    }
    let x = (left + r.frame.width * scale + 20) as isize;
    image.text(
        font.as_ref(),
        x,
        100,
        "地形标高 / 全图格点数",
        19.,
        [240, 240, 230],
    );
    for (i, l) in layers.iter().enumerate() {
        image.circle(x + 7, 150 + i as isize * 50, 7, l.color);
        image.text(
            font.as_ref(),
            x + 23,
            139 + i as isize * 50,
            &format!("{}: {}", l.label, l.cells),
            17.,
            [225, 229, 232],
        );
    }
    image.text(
        font.as_ref(),
        x,
        500,
        "深色：陡崖交界",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        535,
        "紫色：缓坡层级过渡",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        570,
        "蓝色：水陆交界",
        18.,
        [220, 225, 230],
    );
    image.text(
        font.as_ref(),
        x,
        625,
        "完整点位请打开 index.html",
        17.,
        [220, 225, 230],
    );
    image.png(path)
}
pub(super) fn write_readme(dir: &Path, p: &Project, data: &Json) -> Result<()> {
    let s = &data["statistics"];
    let mut out = format!(
        "# {} 坐标资料\n\n打开 `index.html` 查看离线交互地图，可按现有编号、名称、R区域号、B交界号或 `X,Y` 定位。\n\n坐标原点在完整高度网格的西北角；X 向东，Y 向南；1格={}游戏单位。完整采样网格 {}×{}，可玩采样索引为 X={}–{}、Y={}–{}。世界转换：`world_x=(x-border)*cell_size`，`world_y=(height-1-border-y)*cell_size`。对象 Z 原始值与脚下地形高度分列；图外对象的地形高度不猜测。\n\n## 统计\n\n原生对象 {} 个、斜坡 {} 条、脚本建筑候选 {} 条、图外对象 {} 个；建筑 {} 座（其中中立 {}），资源点 {}。水域格点 {}，分层连通区 {} 个，交界折线 {} 条。\n\n",
        p.name,
        data["frame"]["cell_size"],
        data["frame"]["width"],
        data["frame"]["height"],
        data["coordinate_system"]["playable_sample_bounds"][0][0],
        data["coordinate_system"]["playable_sample_bounds"][1][0],
        data["coordinate_system"]["playable_sample_bounds"][0][1],
        data["coordinate_system"]["playable_sample_bounds"][1][1],
        s["map_objects"],
        s["ramps"],
        s["script_building_candidates"],
        s["off_grid_objects"],
        s["buildings"],
        s["neutral_buildings"],
        s["resource_points"],
        s["water_cells"],
        s["regions"],
        s["boundary_paths"]
    );
    out.push_str("## 文件\n\n- `coordinate-system.json`：本图独立的格坐标/世界坐标声明，包含宽高、边框、单位、轴向和正逆转换；与本版 `source/coordinate-system.json` 一致。\n- `coordinate-id-map.json`：原生对象顺序编号与旧预览编号的对照，含已记录版本的转换历史。\n- `coordinates.json`：完整坐标系统、层统计、区域、交界折线、全部点和逐行地形数据。\n- `objects.csv`：原生对象与斜坡，保留预览编号、归属、格坐标和世界坐标。\n- `script-buildings.csv`：按实际脚本提取的条件生成/主备用候选点，不能当作已生成建筑。\n- `layers.csv`、`regions.csv`：层级及连通区域的格点数量、范围、中心与高度。\n- `boundary-points.csv`：每条层交接或岸线的所有折线顶点，半格精度，无抽稀。\n- `terrain-runs.csv`：每行的完整连续坐标范围，可无损还原所有高度格点、层、区域和通行/建造标记。\n- `overview.png`：带坐标网格的总览。\n\n层着色按最近的配置标高归类，缓坡可能经过多个标高带；`flat_cells` 只计算接近标高且平整的台地，坡面单独标记。实际斜坡连接层以斜坡点的 `pair` 为准。交界是相邻层级带共享格边：陡崖和缓坡过渡分别统计；水陆交界只从实际水域提取。区域为四邻接连通区，`R` 号和 `B` 号在本版报告内有效。\n\n对象编号对应当前地图预览，不等于游戏中的单位编队编号。中立标识区分 `PlyrNeutral`、`SkirmishCivilian` 与固定 AI 的 `PlyrCivilian`。总数含图外对象；统计分别给出完整网格和可玩区范围。\n\n本资料直接读取当前原生地图，不修改游戏文件，不递增地图版本。\n");
    fs::write(dir.join("README.md"), out)?;
    Ok(())
}
