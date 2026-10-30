//! Render the real terrain reference independently of all game-design arrays.
use super::{Frame, array, crop_box, identity, mask_array, rendering};
use crate::{Result, err, formats::archive::unzip, render::Image};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

const REFERENCE: &str = "source-reference.npz";
static NEXT: AtomicUsize = AtomicUsize::new(0);

fn valid_bounds(bounds: [f64; 4]) -> Result<[f64; 4]> {
    if bounds.iter().any(|n| !n.is_finite()) || bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
        return Err(err("真实地形参考需要有效的 WGS84 经纬度范围"));
    }
    Ok(bounds)
}

/// Draw only measured elevation and the original water mask. Design layers,
/// morphology, water edits, and game heights are deliberately not read.
pub(super) fn image(
    name: &str,
    files: &BTreeMap<String, Vec<u8>>,
    f: &Frame,
    b: usize,
    target_bounds: Option<[f64; 4]>,
) -> Result<(Image, Value)> {
    let margin = b
        .checked_mul(2)
        .ok_or_else(|| err("真实地形边框尺寸溢出"))?;
    if f.w <= margin || f.h <= margin || f.w > 840 || f.h > 840 {
        return Err(err("真实地形预览网格或边框无效"));
    }
    let input = array(files, "elevation_m")?.ok_or_else(|| err("真实地形参考缺少 elevation_m"))?;
    if input.shape.len() != 2 || input.shape.iter().any(|&n| !(2..=4096).contains(&n)) {
        return Err(err("真实高程须为宽高 2–4096 的二维网格"));
    }
    let (iw, ih) = (input.shape[1], input.shape[0]);
    let original_dem = input.numbers()?;
    if original_dem
        .iter()
        .any(|z| !z.is_finite() || *z < -12000. || *z > 10000.)
    {
        return Err(err("真实高程含无效、缺失或异常数值"));
    }
    let supplied_water = mask_array(files, "water_mask", &input.shape)?;
    let water_source = if supplied_water.is_some() {
        "water_mask"
    } else {
        "elevation_m <= 0 (no supplied water mask)"
    };
    let original_water =
        supplied_water.unwrap_or_else(|| original_dem.iter().map(|&z| z <= 0.).collect());
    let original_province = mask_array(files, "land_mask", &input.shape)?
        .or(mask_array(files, "inside_province", &input.shape)?)
        .unwrap_or_else(|| original_water.iter().map(|&w| !w).collect());
    let crop = crop_box(&original_province, iw, ih, b)?;
    let (view_w, view_h) = (f.w - margin, f.h - margin);
    let actual = array(files, "bounds_wgs84")?
        .map(|bounds_array| -> Result<[f64; 4]> {
            let bounds: [f64; 4] = bounds_array
                .numbers()?
                .try_into()
                .map_err(|_| err("真实地形参考 bounds_wgs84 须含四个坐标"))?;
            let bounds = valid_bounds(bounds)?;
            valid_bounds([
                bounds[0] + crop[0] as f64 / iw as f64 * (bounds[2] - bounds[0]),
                bounds[3] - crop[3] as f64 / ih as f64 * (bounds[3] - bounds[1]),
                bounds[0] + crop[2] as f64 / iw as f64 * (bounds[2] - bounds[0]),
                bounds[3] - crop[1] as f64 / ih as f64 * (bounds[3] - bounds[1]),
            ])
        })
        .transpose()?;
    if let Some(target) = target_bounds {
        let target = valid_bounds(target)?;
        let actual =
            actual.ok_or_else(|| err("真实地形参考缺少当前取景核对所需的 bounds_wgs84"))?;
        for axis in 0..2 {
            // Keep the absolute tolerance at most 1e-4 degrees. For small
            // geographic frames, tighten it without rejecting f32 bounds.
            let span = (actual[axis + 2] - actual[axis]).min(target[axis + 2] - target[axis]);
            let tolerance = (span * 0.01).clamp(0.000015, 0.0001);
            for edge in [axis, axis + 2] {
                if (actual[edge] - target[edge]).abs() > tolerance {
                    return Err(err("真实地形参考的地理取景与当前预览不一致"));
                }
            }
        }
    }

    let mut dem = vec![0.; f.w * f.h];
    let mut water = vec![false; dem.len()];
    let mut province = water.clone();
    for y in 0..f.h {
        for x in 0..f.w {
            // This is the same pixel-center transform as the game preview;
            // it only changes display resolution, never source data values.
            let sx = (crop[0] as f64
                + (x as f64 - b as f64 + 0.5) / view_w as f64 * (crop[2] - crop[0]) as f64
                - 0.5)
                .clamp(0., (iw - 1) as f64);
            let sy = (crop[1] as f64
                + (y as f64 - b as f64 + 0.5) / view_h as f64 * (crop[3] - crop[1]) as f64
                - 0.5)
                .clamp(0., (ih - 1) as f64);
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (a, c) = (sx - x0 as f64, sy - y0 as f64);
            let (x1, y1) = ((x0 + 1).min(iw - 1), (y0 + 1).min(ih - 1));
            let i = y * f.w + x;
            dem[i] = original_dem[y0 * iw + x0] * (1. - a) * (1. - c)
                + original_dem[y0 * iw + x1] * a * (1. - c)
                + original_dem[y1 * iw + x0] * (1. - a) * c
                + original_dem[y1 * iw + x1] * a * c;
            let nearest = sy.round() as usize * iw + sx.round() as usize;
            water[i] = original_water[nearest];
            province[i] = original_province[nearest];
        }
    }
    let layers = vec![0; dem.len()];
    let image = rendering::render_image(
        &rendering::RenderInput {
            name,
            dem: &dem,
            layers: &layers,
            water: &water,
            province: &province,
            frame: f,
            border: b,
            heights: &[210., 280., 350., 420., 700.],
            thresholds: &[200., 500., 1500., 4000.],
        },
        0,
    );
    let metadata = json!({
        "schema_version":1,
        "kind":"real-terrain-reference",
        "reference":REFERENCE,
        "north_up":true,
        "source_grid":[iw,ih],
        "source_crop_pixels":crop,
        "game_cells":[view_w,view_h],
        "border_cells":b,
        "bounds_wgs84":actual,
        "elevation_array":"elevation_m",
        "water_array":water_source,
        "display_sampling":{"elevation":"bilinear","water":"nearest-neighbor"},
        "source_data_modified":false,
        "dem_smoothing_applied":false,
        "game_layer_processing_applied":false,
        "game_water_processing_applied":false,
        "source_water_cells":original_water.iter().filter(|&&v|v).count(),
        "display_water_cells":(b..f.h-b).map(|y| (b..f.w-b).filter(|&x| water[y*f.w+x]).count()).sum::<usize>()
    });
    Ok((image, metadata))
}

/// Replace only source.png and its current source/checksum reports. All source
/// input bytes, design arrays, other previews and playable artifacts stay intact.
pub fn generate(root: &Path, blueprint: &str) -> Result<PathBuf> {
    let (id, name) = identity(root, blueprint)?;
    let map_dir = root.join("maps").join(id);
    let internal = map_dir.join("data/terrain-preview");
    let reference = internal.join(REFERENCE);
    if !reference.is_file() {
        return Err(err(format!(
            "缺少独立真实地形参考 {}；请先恢复原始高程和原始水域，不使用游戏设计水域替代",
            reference.display()
        )));
    }
    let reference_bytes = fs::read(&reference)?;
    let files = unzip(&reference_bytes)?;
    let report_path = internal.join("report.json");
    let mut report: Value = serde_json::from_slice(&fs::read(&report_path)?)?;
    let game = report["game_cells"]
        .as_array()
        .filter(|v| v.len() == 2)
        .ok_or_else(|| err("预览报告缺少有效 game_cells"))?;
    let dimensions = game
        .iter()
        .map(|n| {
            n.as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| (2..=800).contains(n))
                .ok_or_else(|| err("预览报告 game_cells 无效"))
        })
        .collect::<Result<Vec<_>>>()?;
    let b = report["border_cells"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .filter(|&n| n == 20)
        .ok_or_else(|| err("公共真实地形预览须使用四周 20 格边框"))?;
    let f = Frame {
        w: dimensions[0] + 2 * b,
        h: dimensions[1] + 2 * b,
    };
    let target = if report["bounds_wgs84"].is_null() {
        None
    } else {
        Some(serde_json::from_value(report["bounds_wgs84"].clone())?)
    };
    if !report.is_object() || !report["image_crc32"].is_object() {
        return Err(err("预览报告缺少图片校验表"));
    }
    let (image, mut source_report) = image(&name, &files, &f, b, target)?;
    source_report["reference_crc32"] = json!(crc32fast::hash(&reference_bytes));

    let temp = map_dir.join(format!(
        ".source-preview-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temp)?;
    let paths = [
        map_dir.join("source.png"),
        report_path,
        internal.join("source-report.json"),
    ];
    let result = (|| -> Result<()> {
        // Fully render, serialize, and preserve old bytes before publishing.
        image.png(&temp.join("source.png"))?;
        let crc = crc32fast::hash(&fs::read(temp.join("source.png"))?);
        source_report["image_crc32"] = json!(crc);
        report["image_crc32"]["source.png"] = json!(crc);
        report["source_image"] = source_report.clone();
        fs::write(
            temp.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(
            temp.join("source-report.json"),
            serde_json::to_vec_pretty(&source_report)?,
        )?;
        let backups = paths
            .iter()
            .map(|p| {
                if p.exists() {
                    fs::read(p).map(Some)
                } else {
                    Ok(None)
                }
            })
            .collect::<std::io::Result<Vec<_>>>()?;
        let staged = ["source.png", "report.json", "source-report.json"];
        for (index, (file, path)) in staged.iter().zip(&paths).enumerate() {
            if let Err(error) = fs::rename(temp.join(file), path) {
                for (prior, previous) in paths[..index].iter().zip(&backups) {
                    if let Some(bytes) = previous {
                        let restore = temp.join("restore");
                        fs::write(&restore, bytes)?;
                        fs::rename(restore, prior)?;
                    } else {
                        fs::remove_file(prior)?;
                    }
                }
                return Err(error.into());
            }
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(temp);
    result?;
    Ok(map_dir.join("source.png"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{archive::zip, npy::encode};

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn reference() -> BTreeMap<String, Vec<u8>> {
        let mut dem = vec![1000f32; 48 * 48];
        dem[25 * 48 + 25] = 6000.;
        let mut water = vec![0; dem.len()];
        water[21 * 48 + 21] = 1;
        let province: Vec<u8> = (0..dem.len())
            .map(|i| ((20..28).contains(&(i % 48)) && (20..28).contains(&(i / 48))) as u8)
            .collect();
        [
            (
                "elevation_m.npy".into(),
                encode(
                    "<f4",
                    &[48, 48],
                    &dem.iter().flat_map(|z| z.to_le_bytes()).collect::<Vec<_>>(),
                ),
            ),
            ("water_mask.npy".into(), encode("|u1", &[48, 48], &water)),
            (
                "inside_province.npy".into(),
                encode("|u1", &[48, 48], &province),
            ),
            (
                "bounds_wgs84.npy".into(),
                encode(
                    "<f8",
                    &[4],
                    &[100f64, 20., 148., 68.]
                        .iter()
                        .flat_map(|z| z.to_le_bytes())
                        .collect::<Vec<_>>(),
                ),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "ra3-real-source-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let naming = root.join(crate::terrain_preview::NAMING_DOCUMENT);
        fs::create_dir_all(naming.parent().unwrap()).unwrap();
        fs::write(naming,
            "## 全系列定名\n| 地形原型 | 地图名 | 命名画面 |\n|---|---|---|\n| 云南 | 层峦织彩 | 山地 |\n").unwrap();
        let internal = root.join("maps/云南/data/terrain-preview");
        fs::create_dir_all(&internal).unwrap();
        let files: Vec<_> = reference().into_iter().collect();
        fs::write(internal.join(REFERENCE), zip(&files).unwrap()).unwrap();
        fs::write(
            internal.join("report.json"),
            serde_json::to_vec(&json!({
                "game_cells":[8,8],"border_cells":20,"bounds_wgs84":[120.,40.,128.,48.],
                "image_crc32":{"source.png":0,"preview.png":123},"preserved":"metadata"
            }))
            .unwrap(),
        )
        .unwrap();
        Fixture(root)
    }

    #[test]
    fn source_uses_original_water_and_unsmoothed_peaks_ignoring_design_arrays() {
        let mut files = reference();
        let frame = Frame { w: 48, h: 48 };
        let (before, metadata) =
            image("层峦织彩", &files, &frame, 20, Some([120., 40., 128., 48.])).unwrap();
        let pixel = |x: usize, y: usize| {
            &before.rgb[((112 + y) * before.width + 28 + x) * 3
                ..((112 + y) * before.width + 28 + x) * 3 + 3]
        };
        assert_eq!(pixel(1, 1), &[53, 116, 150]);
        assert_eq!(pixel(2, 2), &[128, 152, 77]);
        assert_eq!(pixel(5, 5), &[211, 213, 209]);
        files.insert(
            "layers.npy".into(),
            b"deliberately invalid design layers".to_vec(),
        );
        files.insert(
            "game_heights.npy".into(),
            b"deliberately invalid game heights".to_vec(),
        );
        files.insert(
            "design_water_mask.npy".into(),
            encode("|u1", &[48, 48], &vec![1; 48 * 48]),
        );
        let (after, _) =
            image("层峦织彩", &files, &frame, 20, Some([120., 40., 128., 48.])).unwrap();
        assert_eq!(before.rgb, after.rgb);
        assert_eq!(metadata["game_water_processing_applied"], false);
        assert_eq!(metadata["source_water_cells"], 1);
    }

    #[test]
    fn source_rejects_mismatched_frame_or_invalid_original_water() {
        let mut files = reference();
        let frame = Frame { w: 48, h: 48 };
        assert!(image("层峦织彩", &files, &frame, 20, Some([121., 40., 129., 48.])).is_err());
        files.insert(
            "water_mask.npy".into(),
            encode("|u1", &[48, 48], &vec![2; 48 * 48]),
        );
        assert!(image("层峦织彩", &files, &frame, 20, None).is_err());
    }

    #[test]
    fn elevation_only_reference_preserves_optional_input_contract() {
        let mut files = reference();
        files.retain(|name, _| name == "elevation_m.npy");
        let frame = Frame { w: 48, h: 48 };
        let (source, metadata) = image("层峦织彩", &files, &frame, 20, None).unwrap();
        assert_eq!(source.width, 312);
        assert_eq!(metadata["bounds_wgs84"], Value::Null);
        assert_eq!(metadata["source_water_cells"], 0);
        assert!(
            metadata["water_array"]
                .as_str()
                .unwrap()
                .contains("elevation_m")
        );
        assert!(image("层峦织彩", &files, &frame, 20, Some([120., 40., 128., 48.])).is_err());
    }

    #[test]
    fn source_command_preserves_design_and_source_bytes_and_fails_without_mutation() {
        let f = fixture();
        let map = f.0.join("maps/云南");
        let protected = [
            "preview.png",
            "preview--line.png",
            "preview--center--line.png",
            "config.json",
            "data/terrain-preview/terrain.npz",
            "data/terrain-preview/reference.npz",
            "data/terrain-preview/parameters.json",
        ];
        for file in protected {
            fs::write(map.join(file), format!("protected {file}")).unwrap();
        }
        let reference_before =
            fs::read(map.join("data/terrain-preview/source-reference.npz")).unwrap();
        let output = generate(&f.0, "云南").unwrap();
        let source = fs::read(&output).unwrap();
        for file in protected {
            assert_eq!(
                fs::read(map.join(file)).unwrap(),
                format!("protected {file}").as_bytes()
            );
        }
        assert_eq!(
            fs::read(map.join("data/terrain-preview/source-reference.npz")).unwrap(),
            reference_before
        );
        let path = map.join("data/terrain-preview/report.json");
        let report_bytes = fs::read(&path).unwrap();
        let report: Value = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(report["preserved"], "metadata");
        assert_eq!(report["image_crc32"]["preview.png"], 123);
        assert_eq!(
            report["image_crc32"]["source.png"],
            crc32fast::hash(&source)
        );
        fs::remove_file(map.join("data/terrain-preview/source-reference.npz")).unwrap();
        assert!(generate(&f.0, "云南").is_err());
        assert_eq!(fs::read(output).unwrap(), source);
        assert_eq!(fs::read(path).unwrap(), report_bytes);
    }
}
