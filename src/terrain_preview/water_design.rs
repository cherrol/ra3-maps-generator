//! Keep approved land layers fixed while a preview's water mask is edited.
use super::{Frame, array, layering::layer_counts, mask_array, numbers};
use crate::{Result, err, formats::archive::unzip, terrain::grid::neighbors_rect};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    path::{Component, Path, PathBuf},
};

type LandLayers = (Vec<u8>, Vec<f64>, Vec<f64>, Value);

fn reference_path(base: &Path, cfg: &Value, key: &str) -> Result<PathBuf> {
    let value = cfg[key]
        .as_str()
        .ok_or_else(|| err(format!("水域整理参考缺少 {key}")))?;
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(err("水域整理参考路径须相对于本图 data/terrain-preview"));
    }
    Ok(base.join(path))
}

fn fill_land(old: &[u8], water: &[bool], f: &Frame, border: usize) -> Result<Vec<u8>> {
    if border.saturating_mul(2) >= f.w || border.saturating_mul(2) >= f.h {
        return Err(err("水域整理边框超过地形尺寸"));
    }
    let playable = |i: usize| {
        let (x, y) = (i % f.w, i / f.w);
        x >= border && x < f.w - border && y >= border && y < f.h - border
    };
    let mut result = vec![0; old.len()];
    let mut queue = VecDeque::new();
    for i in 0..old.len() {
        if !water[i] && old[i] > 0 {
            result[i] = old[i];
            if playable(i) {
                queue.push_back(i);
            }
        }
    }
    // Game-edge cells must inherit a land layer reachable inside the playable
    // frame, rather than one whose only connection is through the border.
    while let Some(i) = queue.pop_front() {
        for k in neighbors_rect(i, f.w, f.h) {
            if playable(k) && !water[k] && result[k] == 0 {
                result[k] = result[i];
                queue.push_back(k);
            }
        }
    }
    if (0..old.len()).any(|i| playable(i) && !water[i] && result[i] == 0) {
        return Err(err("移除的水域无法在可玩范围内接入参考地层"));
    }
    // Once playable layers are fixed, use them and the original border land
    // to fill only the remaining production-border cells.
    queue.extend((0..old.len()).filter(|&i| result[i] > 0));
    while let Some(i) = queue.pop_front() {
        for k in neighbors_rect(i, f.w, f.h) {
            if !water[k] && result[k] == 0 {
                result[k] = result[i];
                queue.push_back(k);
            }
        }
    }
    if result
        .iter()
        .zip(water)
        .any(|(&layer, &w)| !w && layer == 0)
    {
        return Err(err("移除的水域无法沿当前陆地接入参考地层"));
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn layers(
    base: &Path,
    cfg: &Value,
    dem: &[f64],
    province: &[bool],
    water: &[bool],
    land: &[bool],
    f: &Frame,
    border: usize,
) -> Result<LandLayers> {
    let files = unzip(&fs::read(reference_path(base, cfg, "terrain")?)?)?;
    let report: Value = serde_json::from_slice(&fs::read(reference_path(base, cfg, "report")?)?)?;
    let shape = [f.h, f.w];
    let old_dem = array(&files, "elevation_m")?.ok_or_else(|| err("参考地形缺少高程"))?;
    if old_dem.shape != shape
        || old_dem
            .numbers()?
            .iter()
            .zip(dem)
            .any(|(&a, &b)| a as f32 != b as f32)
    {
        return Err(err("水域整理参考高程或网格与当前取景不一致"));
    }
    let old_province = mask_array(&files, "inside_province", &shape)?
        .ok_or_else(|| err("参考地形缺少地理边界"))?;
    if old_province != province
        || report["border_cells"] != json!(border)
        || report["game_cells"] != json!([f.w - border * 2, f.h - border * 2])
    {
        return Err(err("水域整理参考边界或尺寸与当前取景不一致"));
    }
    let old = array(&files, "layers")?.ok_or_else(|| err("参考地形缺少分层"))?;
    if old.shape != shape {
        return Err(err("参考分层网格尺寸不一致"));
    }
    let old = old.numbers()?;
    if old
        .iter()
        .any(|&n| !(0. ..=5.).contains(&n) || n.fract() != 0.)
    {
        return Err(err("参考分层包含无效层号"));
    }
    let old: Vec<u8> = old.into_iter().map(|n| n as u8).collect();
    let old_water =
        mask_array(&files, "water_mask", &shape)?.ok_or_else(|| err("参考地形缺少水域"))?;
    if old.iter().zip(&old_water).any(|(&l, &w)| (l == 0) != w) {
        return Err(err("参考水域与分层不一致"));
    }
    let thresholds = numbers(&report, "elevation_thresholds_m", vec![], 4)?;
    let heights = numbers(&report, "game_layer_heights", vec![], 5)?;
    if heights[0] <= 130. || heights[4] > 1000. {
        return Err(err("参考地层高度无效"));
    }
    let old_heights = array(&files, "game_heights")?.ok_or_else(|| err("参考地形缺少设计高度"))?;
    if old_heights.shape != shape
        || old_heights.numbers()?.iter().zip(&old).any(|(&z, &l)| {
            z as f32
                != if l == 0 {
                    130.
                } else {
                    heights[l as usize - 1] as f32
                }
        })
    {
        return Err(err("参考地形高度与报告层高不一致"));
    }
    let result = fill_land(&old, water, f, border)?;
    let counts = layer_counts(&result, land);
    let active: Vec<_> = (1..=5).filter(|&l| counts[l] > 0).collect();
    let mut policy = report["layer_policy"].clone();
    if !policy.is_object() {
        return Err(err("参考报告缺少地层策略"));
    }
    policy["active_land_layers"] = json!(active);
    policy["land_layer_cell_counts"] = json!(counts);
    policy["water_design_land_reference"] = json!({
        "terrain":cfg["terrain"], "report":cfg["report"],
        "existing_land_layers_preserved":true,
        "filled_land_cells":old.iter().zip(water).filter(|(l,w)| **l == 0 && !**w).count(),
        "fill_method":"four-neighbor BFS within playable land first, then fill the production border",
        "playable_land_seeds_take_priority_over_border":true
    });
    Ok((result, thresholds, heights, policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{archive::zip, npy::encode};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn water_edit_keeps_existing_land_and_reference_altitude_policy() {
        let base = std::env::temp_dir().join(format!(
            "ra3-water-reference-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).unwrap();
        let f = Frame { w: 7, h: 5 };
        let old: Vec<u8> = (0..35)
            .map(|i| {
                if i % 7 == 3 {
                    0
                } else if i % 7 < 3 {
                    1
                } else {
                    2
                }
            })
            .collect();
        let dem = vec![99.125f64; 35];
        let province = vec![true; 35];
        let float_bytes = |v: &[f32]| v.iter().flat_map(|n| n.to_le_bytes()).collect::<Vec<_>>();
        let arrays = vec![
            (
                "elevation_m.npy".into(),
                encode("<f4", &[5, 7], &float_bytes(&[99.125; 35])),
            ),
            ("layers.npy".into(), encode("|u1", &[5, 7], &old)),
            (
                "inside_province.npy".into(),
                encode("|u1", &[5, 7], &[1; 35]),
            ),
            (
                "water_mask.npy".into(),
                encode(
                    "|u1",
                    &[5, 7],
                    &old.iter().map(|&l| (l == 0) as u8).collect::<Vec<_>>(),
                ),
            ),
            (
                "game_heights.npy".into(),
                encode(
                    "<f4",
                    &[5, 7],
                    &float_bytes(
                        &old.iter()
                            .map(|&l| match l {
                                0 => 130.,
                                1 => 230.,
                                _ => 300.,
                            })
                            .collect::<Vec<_>>(),
                    ),
                ),
            ),
        ];
        fs::write(base.join("terrain.npz"), zip(&arrays).unwrap()).unwrap();
        let thresholds = vec![19., 200., 400., 800.];
        fs::write(
            base.join("report.json"),
            serde_json::to_vec(&json!({
                "border_cells":1,"game_cells":[5,3],"elevation_thresholds_m":thresholds,
                "game_layer_heights":[230.,300.,370.,440.,700.],
                "layer_policy":{"adjustments":[{"rule":"local_two_layer_split","split_m":19.}]}
            }))
            .unwrap(),
        )
        .unwrap();
        let cfg = json!({"terrain":"terrain.npz","report":"report.json"});
        let mut water = vec![false; 35];
        water[2] = true;
        let land: Vec<_> = (0..35)
            .map(|i| i % 7 > 0 && i % 7 < 6 && i / 7 > 0 && i / 7 < 4 && !water[i])
            .collect();
        let (now, cut, heights, policy) =
            layers(&base, &cfg, &dem, &province, &water, &land, &f, 1).unwrap();
        for i in 0..35 {
            if old[i] > 0 && !water[i] {
                assert_eq!(now[i], old[i]);
            } else if water[i] {
                assert_eq!(now[i], 0);
            } else {
                assert!((1..=2).contains(&now[i]));
            }
        }
        assert_eq!(cut, thresholds);
        assert_eq!(heights[0], 230.);
        assert_eq!(policy["adjustments"][0]["split_m"], 19.);
        assert_eq!(
            policy["water_design_land_reference"]["filled_land_cells"],
            5
        );
        let mut wrong_dem = dem.clone();
        wrong_dem[34] += 1.;
        assert!(layers(&base, &cfg, &wrong_dem, &province, &water, &land, &f, 1).is_err());
        let mut wrong_province = province.clone();
        wrong_province[0] = false;
        assert!(layers(&base, &cfg, &dem, &wrong_province, &water, &land, &f, 1).is_err());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn removed_water_cannot_borrow_a_layer_across_current_water() {
        let f = Frame { w: 5, h: 3 };
        let old = vec![1, 1, 0, 0, 2, 1, 1, 0, 0, 2, 1, 1, 0, 0, 2];
        let water = vec![
            false, true, false, true, false, false, true, false, true, false, false, true, false,
            true, false,
        ];
        assert!(fill_land(&old, &water, &f, 0).is_err());
        let mut open_water = water.clone();
        open_water[1] = false;
        let filled = fill_land(&old, &open_water, &f, 0).unwrap();
        assert_eq!(filled[7], 1);
    }

    #[test]
    fn playable_fill_does_not_inherit_a_layer_only_connected_through_border() {
        let f = Frame { w: 7, h: 5 };
        let mut old: Vec<u8> = (0..35)
            .map(|i| {
                let (x, y) = (i % 7, i / 7);
                if x == 0 || x == 6 || y == 0 || y == 4 {
                    2
                } else if x == 1 {
                    0
                } else {
                    3
                }
            })
            .collect();
        old[0] = 0;
        let water = vec![false; 35];
        let filled = fill_land(&old, &water, &f, 1).unwrap();
        for y in 1..4 {
            assert_eq!(filled[y * 7 + 1], 3);
        }
        assert_eq!(filled[0], 2);
        for i in 0..old.len() {
            if old[i] > 0 {
                assert_eq!(filled[i], old[i]);
            }
        }
        // A production-border seed cannot rescue a playable component that
        // has no old land seed of its own.
        for y in 1..4 {
            for x in 1..6 {
                old[y * 7 + x] = 0;
            }
        }
        assert!(fill_land(&old, &water, &f, 1).is_err());
    }
}
