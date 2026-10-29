//! Shared, offline terrain-design preview stage. Input is supplied per blueprint.
use crate::{
    Result, err,
    formats::archive::{unzip, zip},
    formats::npy::{Npy, encode as npy},
    pipeline::project,
    render::{self, Image},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
mod centers;
mod layering;
mod rendering;
mod water_design;
use layering::design_layers;
#[cfg(test)]
use layering::layer_counts;
use rendering::{RenderInput, render_image};

const COLORS: [[u8; 3]; 6] = [
    [53, 116, 150],
    [104, 151, 89],
    [160, 178, 99],
    [202, 180, 119],
    [162, 128, 103],
    [218, 221, 211],
];
struct Frame {
    w: usize,
    h: usize,
}

fn write_coordinate_system(
    map_dir: &Path,
    id: &str,
    name: &str,
    f: &Frame,
    border: usize,
) -> Result<()> {
    let frame = crate::coordinates::Frame {
        width: f.w,
        height: f.h,
        border,
        cell_size: 10.,
        z_scale: 0.0390625,
    };
    let mut descriptor = frame.descriptor()?;
    descriptor["scope"] = json!("terrain-preview-design");
    descriptor["map_id"] = json!(id);
    descriptor["map_name"] = json!(name);
    descriptor["source"] = json!({"kind":"terrain-preview-cache","path":format!("maps/{id}/data/terrain-preview/terrain.npz")});
    descriptor["parameter_source"] = json!({
        "width_height_border":"terrain preview cache and report",
        "cell_size_z_scale":"design defaults; use the playable recipe when authoring a finished map"
    });
    project::write_json(
        &map_dir.join("data/terrain-preview/coordinate-system.json"),
        &descriptor,
    )?;
    if !map_dir.join("config.json").is_file() {
        project::write_json(&map_dir.join("coordinate-system.json"), &descriptor)?;
    }
    Ok(())
}

/// Initialize only coordinate metadata from an existing preview cache/report.
/// Existing images, terrain arrays and playable-map coordinate frames stay intact.
pub fn initialize_coordinate_system(root: &Path, blueprint: &str) -> Result<PathBuf> {
    let (id, name) = identity(root, blueprint)?;
    let map_dir = root.join("maps").join(&id);
    let internal = map_dir.join("data/terrain-preview");
    let report: Value = serde_json::from_slice(&fs::read(internal.join("report.json"))?)?;
    let border = report["border_cells"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| err("预览报告缺少有效边框"))?;
    let files = unzip(&fs::read(internal.join("terrain.npz"))?)?;
    let elevation = array(&files, "elevation_m")?.ok_or_else(|| err("预览缓存缺少高程"))?;
    if elevation.shape.len() != 2 {
        return Err(err("预览缓存尺寸无效"));
    }
    let f = Frame {
        w: elevation.shape[1],
        h: elevation.shape[0],
    };
    let margin = border
        .checked_mul(2)
        .filter(|&margin| f.w > margin && f.h > margin)
        .ok_or_else(|| err("预览边框超过地形缓存尺寸"))?;
    if report["game_cells"] != json!([f.w - margin, f.h - margin]) {
        return Err(err("预览报告与地形缓存尺寸不一致"));
    }
    write_coordinate_system(&map_dir, &id, &name, &f, border)?;
    Ok(internal.join("coordinate-system.json"))
}

/// Resolve the generated display name from the maintained geographic mapping.
pub fn map_name(root: &Path, blueprint: &str) -> Result<String> {
    identity(root, blueprint).map(|(_, name)| name)
}

/// Resolve the real geographic name used by local directories and CLI selection.
pub fn map_id(root: &Path, blueprint: &str) -> Result<String> {
    identity(root, blueprint).map(|(id, _)| id)
}

fn identity(root: &Path, blueprint: &str) -> Result<(String, String)> {
    let text = fs::read_to_string(root.join("docs/九州大地命名.md"))?;
    let table = text
        .split("## 全系列定名")
        .nth(1)
        .ok_or_else(|| err("命名文档缺少“全系列定名”表"))?
        .split("\n## ")
        .next()
        .unwrap();
    let needle = blueprint.trim().trim_end_matches('省');
    for line in table.lines().filter(|l| l.starts_with('|')) {
        let cells: Vec<_> = line
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if cells.len() >= 3
            && !cells[0].starts_with("地形原型")
            && !cells[0].starts_with("真实地形")
            && !cells[0].starts_with('-')
            && (cells[0] == needle || cells[1] == needle)
        {
            project::safe_component(cells[0])?;
            project::safe_component(cells[1])?;
            return Ok((cells[0].into(), cells[1].into()));
        }
    }
    Err(err(format!(
        "docs/九州大地命名.md 中找不到蓝图 {blueprint} 的地图名称"
    )))
}
fn array(files: &std::collections::BTreeMap<String, Vec<u8>>, key: &str) -> Result<Option<Npy>> {
    files
        .get(&format!("{key}.npy"))
        .map(|b| Npy::parse(b))
        .transpose()
}
fn mask_array(
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    key: &str,
    shape: &[usize],
) -> Result<Option<Vec<bool>>> {
    array(files, key)?
        .map(|a| {
            if a.shape != shape {
                return Err(err(format!("{key} 的网格尺寸与高程不一致")));
            }
            let v = a.numbers()?;
            if v.iter().any(|v| !v.is_finite() || (*v != 0. && *v != 1.)) {
                return Err(err(format!("{key} 必须是 0/1 掩码")));
            }
            Ok(v.into_iter().map(|v| v != 0.).collect())
        })
        .transpose()
}
fn crop_box(mask: &[bool], w: usize, h: usize, border: usize) -> Result<[usize; 4]> {
    let indices: Vec<_> = mask
        .iter()
        .enumerate()
        .filter(|(_, v)| **v)
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        return Err(err("真实地形主体掩码为空"));
    }
    let x0 = indices.iter().map(|i| i % w).min().unwrap();
    let x1 = indices.iter().map(|i| i % w).max().unwrap() + 1;
    let y0 = indices.iter().map(|i| i / w).min().unwrap();
    let y1 = indices.iter().map(|i| i / w).max().unwrap() + 1;
    if w > 2 * border
        && h > 2 * border
        && x0 >= border
        && y0 >= border
        && x1 <= w - border
        && y1 <= h - border
    {
        return Ok([border, border, w - border, h - border]);
    }
    Ok([
        x0.saturating_sub(2),
        y0.saturating_sub(2),
        (x1 + 2).min(w),
        (y1 + 2).min(h),
    ])
}

fn processing(cfg: &Value) -> Result<Value> {
    let mut p = json!({"dem_box_blur_radius_cells":6,"cumulative_open_radius_cells":15,
        "cumulative_close_radius_cells":6,"peak_open_radius_cells":6,"majority_radius_cells":5,
        "minimum_component_area_cells":2500,"minimum_peak_area_cells":900,"minimum_neck_width_cells":20});
    for (key, value) in p.as_object_mut().unwrap() {
        if let Some(v) = cfg["processing"].get(key) {
            *value = v.clone();
        }
        if value.as_u64().is_none_or(|n| n > 100_000) {
            return Err(err(format!("无效的地形处理参数 {key}")));
        }
        if key.contains("radius") && value.as_u64().unwrap() > 100 {
            return Err(err(format!("{key} 半径过大")));
        }
    }
    Ok(p)
}
fn numbers(cfg: &Value, key: &str, default: Vec<f64>, count: usize) -> Result<Vec<f64>> {
    let v = if let Some(a) = cfg[key].as_array() {
        a.iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| err(format!("{key} 必须是数字数组")))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        default
    };
    if v.len() != count || v.iter().any(|v| !v.is_finite()) || v.windows(2).any(|p| p[0] >= p[1]) {
        return Err(err(format!("{key} 须包含 {count} 个递增有限数值")));
    }
    Ok(v)
}
/// Shared numeric implementation behind the one-sentence authoring workflow.
/// The supplied NPZ is north-up and contains elevation_m plus optional water/land masks.
pub fn generate(
    root: &Path,
    blueprint: &str,
    reference: &Path,
    requested_map: Option<&str>,
) -> Result<PathBuf> {
    let (id, name) = identity(root, blueprint)?;
    if let Some(requested) = requested_map {
        project::safe_component(requested)?;
        if map_id(root, requested)? != id {
            return Err(err(format!("蓝图 {blueprint} 应保存到 maps/{id}/")));
        }
    }
    let map_dir = root.join("maps").join(&id);
    let internal_config = map_dir.join("data/terrain-preview/parameters.json");
    let cfg = if internal_config.is_file() {
        serde_json::from_slice(&fs::read(internal_config)?)?
    } else if map_dir.join("preview.json").is_file() {
        serde_json::from_slice(&fs::read(map_dir.join("preview.json"))?)?
    } else {
        json!({})
    };
    let reference_bytes = fs::read(reference)?;
    let files = unzip(&reference_bytes)?;
    let input=array(&files,"elevation_m")?.ok_or_else(||err("提供的 NPZ 缺少 elevation_m；图片蓝图应先按公共制作规范解释为设计分层，不能冒充测量高程"))?;
    if input.shape.len() != 2 || input.shape.iter().any(|&n| !(2..=4096).contains(&n)) {
        return Err(err("高程必须是 2D 网格，宽高在 2–4096 之间"));
    }
    let (iw, ih) = (input.shape[1], input.shape[0]);
    let input_dem = input.numbers()?;
    if input_dem
        .iter()
        .any(|z| !z.is_finite() || *z < -12000. || *z > 10000.)
    {
        return Err(err("高程中包含无效、缺失或异常数值"));
    }
    let input_water = mask_array(&files, "water_mask", &input.shape)?
        .unwrap_or_else(|| input_dem.iter().map(|&z| z <= 0.).collect());
    let supplied_province = mask_array(&files, "land_mask", &input.shape)?.or(mask_array(
        &files,
        "inside_province",
        &input.shape,
    )?);
    let input_province =
        supplied_province.unwrap_or_else(|| input_water.iter().map(|&w| !w).collect());
    let b = cfg["border_cells"].as_u64().unwrap_or(20) as usize;
    if b != 20 {
        return Err(err("公共地形预览使用四周 20 格边框"));
    }
    let crop = crop_box(&input_province, iw, ih, b)?;
    // Keep the geographic crop intact and resample both axes to the square
    // playable grid required by the game. Image legends are outside this grid.
    let (view_w, view_h) = (800, 800);
    let f = Frame {
        w: view_w + 2 * b,
        h: view_h + 2 * b,
    };
    let sample = |x: usize, y: usize| -> (f64, f64) {
        (
            crop[0] as f64
                + (x as f64 - b as f64 + 0.5) / view_w as f64 * (crop[2] - crop[0]) as f64
                - 0.5,
            crop[1] as f64
                + (y as f64 - b as f64 + 0.5) / view_h as f64 * (crop[3] - crop[1]) as f64
                - 0.5,
        )
    };
    let mut dem = vec![0.; f.w * f.h];
    let mut water = vec![false; dem.len()];
    let mut province = water.clone();
    let mut land = water.clone();
    for y in 0..f.h {
        for x in 0..f.w {
            let (sx, sy) = sample(x, y);
            let (sx, sy) = (sx.clamp(0., (iw - 1) as f64), sy.clamp(0., (ih - 1) as f64));
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (a, c) = (sx - x0 as f64, sy - y0 as f64);
            let (x1, y1) = ((x0 + 1).min(iw - 1), (y0 + 1).min(ih - 1));
            let i = y * f.w + x;
            dem[i] = input_dem[y0 * iw + x0] * (1. - a) * (1. - c)
                + input_dem[y0 * iw + x1] * a * (1. - c)
                + input_dem[y1 * iw + x0] * (1. - a) * c
                + input_dem[y1 * iw + x1] * a * c;
            let nearest = sy.round() as usize * iw + sx.round() as usize;
            water[i] = input_water[nearest];
            province[i] = input_province[nearest];
            land[i] = x >= b && x < f.w - b && y >= b && y < f.h - b && !water[i];
        }
    }
    if let Some(reference) = cfg.get("water_design_land_reference")
        && let Some(design) = water_design::design_water(
            &map_dir.join("data/terrain-preview"),
            reference,
            &[f.h, f.w],
        )?
    {
        water = design;
        for (i, cell) in land.iter_mut().enumerate() {
            let (x, y) = (i % f.w, i / f.w);
            *cell = x >= b && x < f.w - b && y >= b && y < f.h - b && !water[i];
        }
    }
    if !land.iter().any(|&m| m) {
        return Err(err("蓝图游戏范围内没有陆地高程"));
    }
    let thresholds = numbers(
        &cfg,
        "elevation_thresholds_m",
        vec![200., 500., 1500., 4000.],
        4,
    )?;
    let heights = numbers(
        &cfg,
        "game_layer_heights",
        vec![210., 280., 350., 420., 700.],
        5,
    )?;
    if heights[0] <= 130. || heights[4] > 1000. {
        return Err(err("陆地高度应高于水域，第五层设计高度不得大于 1000"));
    }
    let parameters = processing(&cfg)?;
    let (mut layers, thresholds, heights, layer_policy) = if let Some(reference) =
        cfg.get("water_design_land_reference")
    {
        water_design::layers(
            &map_dir.join("data/terrain-preview"),
            reference,
            &dem,
            &province,
            &water,
            &land,
            &f,
            b,
        )?
    } else {
        let (layers, thresholds, policy) = design_layers(&dem, &land, &f, thresholds, &parameters)?;
        (layers, thresholds, heights, policy)
    };
    for (l, &w) in layers.iter_mut().zip(&water) {
        if w {
            *l = 0;
        }
    }
    let float_bytes = |v: &[f64]| {
        v.iter()
            .flat_map(|n| (*n as f32).to_le_bytes())
            .collect::<Vec<_>>()
    };
    let heights_grid: Vec<f64> = layers
        .iter()
        .map(|&l| {
            if l == 0 {
                130.
            } else {
                heights[l as usize - 1]
            }
        })
        .collect();
    let mut cache = vec![
        (
            "elevation_m.npy".into(),
            npy("<f4", &[f.h, f.w], &float_bytes(&dem)),
        ),
        ("layers.npy".into(), npy("|u1", &[f.h, f.w], &layers)),
        (
            "game_heights.npy".into(),
            npy("<f4", &[f.h, f.w], &float_bytes(&heights_grid)),
        ),
        (
            "water_mask.npy".into(),
            npy(
                "|u1",
                &[f.h, f.w],
                &water.iter().map(|&m| m as u8).collect::<Vec<_>>(),
            ),
        ),
        (
            "inside_province.npy".into(),
            npy(
                "|u1",
                &[f.h, f.w],
                &province.iter().map(|&m| m as u8).collect::<Vec<_>>(),
            ),
        ),
    ];
    let mut crop_bounds = Value::Null;
    if let Some(bounds) = array(&files, "bounds_wgs84")? {
        let v = bounds.numbers()?;
        if v.len() != 4 || v.iter().any(|v| !v.is_finite()) || v[0] >= v[2] || v[1] >= v[3] {
            return Err(err("bounds_wgs84 无效"));
        }
        let actual = vec![
            v[0] + crop[0] as f64 / iw as f64 * (v[2] - v[0]),
            v[3] - crop[3] as f64 / ih as f64 * (v[3] - v[1]),
            v[0] + crop[2] as f64 / iw as f64 * (v[2] - v[0]),
            v[3] - crop[1] as f64 / ih as f64 * (v[3] - v[1]),
        ];
        let step_x = (actual[2] - actual[0]) / view_w as f64;
        let step_y = (actual[3] - actual[1]) / view_h as f64;
        let full = vec![
            actual[0] - b as f64 * step_x,
            actual[1] - b as f64 * step_y,
            actual[2] + b as f64 * step_x,
            actual[3] + b as f64 * step_y,
        ];
        cache.push((
            "bounds_wgs84.npy".into(),
            npy("<f4", &[4], &float_bytes(&full)),
        ));
        cache.push((
            "playable_bounds_wgs84.npy".into(),
            npy("<f4", &[4], &float_bytes(&actual)),
        ));
        crop_bounds = json!(actual);
    }
    let render_input = RenderInput {
        name: &name,
        dem: &dem,
        layers: &layers,
        water: &water,
        province: &province,
        frame: &f,
        border: b,
        heights: &heights,
        thresholds: &thresholds,
    };
    let mut images = ["source.png", "preview.png", "preview--line.png"]
        .iter()
        .enumerate()
        .map(|(mode, &file)| (file, render_image(&render_input, mode as u8)))
        .collect::<Vec<_>>();
    let center_report = if map_dir.join("data/terrain-preview/centers.json").is_file() {
        let data = centers::read(&map_dir, &name)?;
        let bounds: [f64; 4] = serde_json::from_value(crop_bounds.clone())
            .map_err(|_| err("中心标点需要 bounds_wgs84 经纬度范围"))?;
        let mut image = render_image(&render_input, 2);
        let report = centers::draw(&mut image, &data, bounds, view_w, view_h)?;
        images.push((centers::FILE, image));
        Some(report)
    } else {
        None
    };
    fs::create_dir_all(&map_dir)?;
    let temp = map_dir.join(format!(".terrain-preview-{}", std::process::id()));
    fs::create_dir(&temp)?;
    let result = (|| -> Result<()> {
        for (name, image) in &images {
            image.png(&temp.join(name))?;
        }
        let internal = map_dir.join("data/terrain-preview");
        fs::create_dir_all(&internal)?;
        fs::write(internal.join("terrain.npz"), zip(&cache)?)?;
        let reference_crc = crc32fast::hash(&reference_bytes);
        fs::write(internal.join("reference.npz"), &reference_bytes)?;
        let metadata = json!({"schema_version":1,"blueprint":blueprint,"map_id":id,"map_name":name,"reference_crc32":reference_crc,
            "source_reference":reference.strip_prefix(root).unwrap_or(reference).to_string_lossy(),"north_up":true,"same_view_for_all_images":true,
            "game_cells":[view_w,view_h],"border_cells":b,"bounds_wgs84":crop_bounds,
            "size_policy":{"shape":"square","playable_cells":[800,800],
                "geographic_frame":"preserved; axes resampled independently",
                "maximum_playable_cells":[800,800],"long_side_cells":800,
                "dimension_multiple":8,"border_included_in_size":false,
                "source_playable_cells":[crop[2]-crop[0],crop[3]-crop[1]]},
            "elevation_thresholds_m":thresholds,"game_layer_heights":heights,"processing":parameters,
            "layer_policy":layer_policy,
            "terrain_shape_repair":cfg.get("terrain_shape_repair").cloned().unwrap_or(Value::Null),
            "water_policy":cfg.get("water_policy").cloned().unwrap_or(Value::Null),
            "scope":"entire rectangle including terrain outside the prototype outline; no province-based clipping",
            "final_images":images.iter().map(|(n,_)|*n).collect::<Vec<_>>(),
            "image_crc32":images.iter().map(|(n,_)|Ok((n.to_string(),crc32fast::hash(&fs::read(temp.join(n))?)))).collect::<Result<std::collections::BTreeMap<_,_>>>()?});
        fs::write(
            internal.join("report.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;
        write_coordinate_system(&map_dir, &id, &name, &f, b)?;
        if let Some(report) = &center_report {
            let mut report = report.clone();
            report["image_crc32"] = metadata["image_crc32"][centers::FILE].clone();
            fs::write(
                internal.join("centers-report.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
        }
        for (file, _) in &images {
            fs::rename(temp.join(file), map_dir.join(file))?;
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(temp);
    result?;
    Ok(map_dir)
}

/// Add the center overlay from the existing cache, without changing any of the
/// three original images, cached terrain, input reference, or playable recipe.
pub fn generate_centers(root: &Path, blueprint: &str) -> Result<PathBuf> {
    let (id, name) = identity(root, blueprint)?;
    let map_dir = root.join("maps").join(&id);
    let internal = map_dir.join("data/terrain-preview");
    let data = centers::read(&map_dir, &name)?;
    let report: Value = serde_json::from_slice(&fs::read(internal.join("report.json"))?)?;
    let bounds: [f64; 4] = serde_json::from_value(report["bounds_wgs84"].clone())?;
    if bounds.iter().any(|v| !v.is_finite()) || bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
        return Err(err("中心标点需要有效的 WGS84 游戏取景范围"));
    }
    let files = unzip(&fs::read(internal.join("terrain.npz"))?)?;
    let elevation = array(&files, "elevation_m")?.ok_or_else(|| err("预览缓存缺少高程"))?;
    if elevation.shape.len() != 2 || elevation.shape.iter().any(|&n| !(42..=840).contains(&n)) {
        return Err(err("预览缓存尺寸无效"));
    }
    let f = Frame {
        w: elevation.shape[1],
        h: elevation.shape[0],
    };
    let b = 20;
    let (w, h) = (f.w - 2 * b, f.h - 2 * b);
    if report["game_cells"] != json!([w, h]) || report["border_cells"] != b {
        return Err(err("预览报告与地形缓存尺寸不一致"));
    }
    let layer_array = array(&files, "layers")?.ok_or_else(|| err("预览缓存缺少分层"))?;
    if layer_array.shape != elevation.shape {
        return Err(err("预览分层尺寸不一致"));
    }
    let values = layer_array.numbers()?;
    if values
        .iter()
        .any(|v| !v.is_finite() || *v < 0. || *v > 5. || v.fract() != 0.)
    {
        return Err(err("预览分层无效"));
    }
    let layers: Vec<u8> = values.into_iter().map(|v| v as u8).collect();
    let water = mask_array(&files, "water_mask", &elevation.shape)?
        .ok_or_else(|| err("预览缓存缺少水域"))?;
    let province = mask_array(&files, "inside_province", &elevation.shape)?
        .ok_or_else(|| err("预览缓存缺少主体轮廓"))?;
    let heights = numbers(
        &report,
        "game_layer_heights",
        vec![210., 280., 350., 420., 700.],
        5,
    )?;
    let thresholds = numbers(
        &report,
        "elevation_thresholds_m",
        vec![200., 500., 1500., 4000.],
        4,
    )?;
    let mut image = render_image(
        &RenderInput {
            name: &name,
            dem: &elevation.numbers()?,
            layers: &layers,
            water: &water,
            province: &province,
            frame: &f,
            border: b,
            heights: &heights,
            thresholds: &thresholds,
        },
        2,
    );
    let mut overlay_report = centers::draw(&mut image, &data, bounds, w, h)?;
    let output = map_dir.join(centers::FILE);
    let temp = map_dir.join(format!(".centers-{}.png", std::process::id()));
    let result = (|| -> Result<()> {
        image.png(&temp)?;
        overlay_report["image_crc32"] = json!(crc32fast::hash(&fs::read(&temp)?));
        fs::rename(&temp, &output)?;
        fs::write(
            internal.join("centers-report.json"),
            serde_json::to_vec_pretty(&overlay_report)?,
        )?;
        write_coordinate_system(&map_dir, &id, &name, &f, b)?;
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result?;
    Ok(output)
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    struct NamingFixture(PathBuf);
    impl NamingFixture {
        fn new() -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "ra3-preview-names-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("docs")).unwrap();
            fs::write(
                root.join("docs/九州大地命名.md"),
                "## 全系列定名\n\n| 地形原型 | 地图名 | 命名画面 |\n|---|---|---|\n| 云南 | 层峦织彩 | 山地 |\n| 海南 | 翠屿浮光 | 海岛 |\n| ../outside | 非法目录 | 无 |\n",
            )
            .unwrap();
            Self(root)
        }
    }
    impl Drop for NamingFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn partial_naming_tables_resolve_geographic_ids_and_reject_unsafe_or_unknown_rows() {
        let f = NamingFixture::new();
        assert_eq!(map_id(&f.0, "云南省").unwrap(), "云南");
        assert_eq!(map_id(&f.0, "层峦织彩").unwrap(), "云南");
        assert_eq!(map_name(&f.0, "云南").unwrap(), "层峦织彩");
        for name in ["地图名", "../outside", "非法目录", "不存在"] {
            assert!(map_id(&f.0, name).is_err());
        }
        assert!(generate(&f.0, "云南", &f.0.join("absent.npz"), Some("海南")).is_err());
        assert!(!f.0.join("maps").exists());
    }

    #[test]
    fn coordinate_initialization_uses_geographic_directory_and_generated_display_name() {
        let f = NamingFixture::new();
        let internal = f.0.join("maps/云南/data/terrain-preview");
        fs::create_dir_all(&internal).unwrap();
        fs::write(
            internal.join("terrain.npz"),
            zip(&[(
                "elevation_m.npy".into(),
                npy("<f4", &[56, 48], &vec![0; 56 * 48 * 4]),
            )])
            .unwrap(),
        )
        .unwrap();
        project::write_json(
            &internal.join("report.json"),
            &json!({"game_cells":[34,42],"border_cells":7}),
        )
        .unwrap();
        let path = initialize_coordinate_system(&f.0, "层峦织彩").unwrap();
        assert_eq!(path, internal.join("coordinate-system.json"));
        let descriptor: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(descriptor["map_id"], "云南");
        assert_eq!(descriptor["map_name"], "层峦织彩");
        assert_eq!(
            descriptor["source"]["path"],
            "maps/云南/data/terrain-preview/terrain.npz"
        );
        assert!(!f.0.join("maps/层峦织彩").exists());
    }

    fn params() -> Value {
        processing(&json!({"processing":{
            "dem_box_blur_radius_cells":1,"cumulative_open_radius_cells":3,
            "cumulative_close_radius_cells":1,"peak_open_radius_cells":2,
            "majority_radius_cells":1,"minimum_component_area_cells":100,
            "minimum_peak_area_cells":100,"minimum_neck_width_cells":6
        }}))
        .unwrap()
    }

    #[test]
    fn low_relief_plain_gets_two_real_elevation_bands() {
        let f = Frame { w: 160, h: 120 };
        let dem: Vec<_> = (0..f.w * f.h)
            .map(|i| 30. + (i % f.w) as f64 * 0.5)
            .collect();
        let land = vec![true; dem.len()];
        let (layers, thresholds, policy) =
            design_layers(&dem, &land, &f, vec![200., 500., 1500., 4000.], &params()).unwrap();
        assert_eq!(policy["active_land_layers"], json!([1, 2]));
        assert!(thresholds[0] > 30. && thresholds[0] < 110.);
        assert_eq!(layers[60 * f.w + 10], 1);
        assert_eq!(layers[60 * f.w + 150], 2);
        assert_eq!(policy["adjustments"][0]["rule"], "local_two_layer_split");
    }

    #[test]
    fn plateau_floor_stays_playable_and_prominent_peak_survives() {
        let f = Frame { w: 160, h: 120 };
        let mut dem: Vec<_> = (0..f.w * f.h).map(|i| 4400. + (i % f.w) as f64).collect();
        for y in 40..80 {
            for x in 60..100 {
                dem[y * f.w + x] = 6000.;
            }
        }
        let land = vec![true; dem.len()];
        let (layers, thresholds, policy) =
            design_layers(&dem, &land, &f, vec![200., 500., 1500., 4000.], &params()).unwrap();
        assert!(thresholds[3] > 5000.);
        assert_eq!(layers[20 * f.w + 20], 4);
        assert_eq!(layers[60 * f.w + 80], 5);
        assert!(layer_counts(&layers, &land)[5] < dem.len() / 5);
        assert_eq!(policy["adjustments"][0]["rule"], "plateau_peak_threshold");
    }

    #[test]
    fn absent_high_bands_are_not_created() {
        let f = Frame { w: 160, h: 120 };
        let dem: Vec<_> = (0..f.w * f.h)
            .map(|i| 20. + (i % f.w) as f64 * 5.)
            .collect();
        let land = vec![true; dem.len()];
        let (_, thresholds, policy) =
            design_layers(&dem, &land, &f, vec![200., 500., 1500., 4000.], &params()).unwrap();
        assert_eq!(thresholds, vec![200., 500., 1500., 4000.]);
        assert_eq!(policy["active_land_layers"], json!([1, 2, 3]));
        assert_eq!(policy["adjustments"], json!([]));
    }

    #[test]
    fn perfectly_flat_data_does_not_invent_geographic_relief() {
        let f = Frame { w: 160, h: 120 };
        let dem = vec![50.; f.w * f.h];
        assert!(
            design_layers(
                &dem,
                &vec![true; dem.len()],
                &f,
                vec![200., 500., 1500., 4000.],
                &params()
            )
            .is_err()
        );
    }
}
