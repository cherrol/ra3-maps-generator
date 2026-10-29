//! Terrain generation, native edits, navigation and resource placement checks.
pub mod grid;
pub mod layers;
mod layout;
pub use grid::{disk, distance, edge, flip, gradient};
pub use layout::{Layout, ObjectSpec};
pub mod materials;
pub mod native_cell_edits;
pub mod ramp_clearance;
pub mod ramps;
pub mod resource_routes;
pub mod spatial;
pub mod spawn_placement;
pub mod surface;
pub mod texture_polish;

use crate::{
    Result, err,
    formats::archive::{unzip, zip},
    formats::map::HeightMap,
    formats::npy::{Npy, encode},
    pipeline::project::{Project, write_json},
};
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct Terrain {
    pub n: usize,
    pub raw: Vec<u16>,
    pub land: Vec<bool>,
    pub z: Vec<f64>,
    pub slope: Vec<f64>,
    pub layout: Layout,
}
pub fn point(v: &Value) -> Result<[f64; 2]> {
    Ok([
        v[0].as_f64().ok_or_else(|| err("expected x coordinate"))?,
        v[1].as_f64().ok_or_else(|| err("expected y coordinate"))?,
    ])
}
pub fn number(v: &Value, k: &str, default: f64) -> f64 {
    v[k].as_f64().unwrap_or(default)
}
pub fn ramp_axes(rs: &Value) -> Result<(f64, f64, f64, f64)> {
    let pair = rs
        .get("pair")
        .or_else(|| rs.get("layers"))
        .ok_or_else(|| err("ramp missing layer pair"))?
        .as_array()
        .ok_or_else(|| err("invalid ramp layers"))?;
    let values = pair
        .iter()
        .map(|v| v.as_f64().ok_or_else(|| err("invalid ramp layer")))
        .collect::<Result<Vec<_>>>()?;
    if values.len() != 2 || values.iter().any(|v| !v.is_finite()) || values[0] == values[1] {
        return Err(err(
            "a ramp must connect exactly two distinct terrain levels",
        ));
    }
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ux = number(rs, "ux", 1.);
    let uy = number(rs, "uy", 0.);
    let norm = ux.hypot(uy).max(1e-12);
    Ok((ux / norm, uy / norm, lo, hi))
}
/// Width of the full slope, excluding its feathered shoulders.
pub fn ramp_width(rs: &Value) -> Result<f64> {
    let (ux, uy, _, _) = ramp_axes(rs)?;
    let width = number(
        rs,
        "width",
        number(rs, "w", 25.) * uy.abs() + number(rs, "h", 25.) * ux.abs(),
    );
    if !width.is_finite() || width < 20. {
        return Err(err("ramp width must be at least 20 cells"));
    }
    Ok(width)
}
/// Check the original platform, never the interpolated height of a slope.
/// Non-finite values denote an existing slope rather than a flat platform.
pub(crate) fn ramp_layer_allowed(height: f64, lo: f64, hi: f64) -> bool {
    !height.is_finite() || (height - lo).abs().min((height - hi).abs()) <= 3.
}
pub fn ramp_cover(i: usize, n: usize, rs: &Value, pad: f64) -> bool {
    if rs["style"] == "mountain" {
        let (ux, uy, _, _) = ramp_axes(rs).expect("validated ramp");
        let dx = (i % n) as f64 - number(rs, "x", 0.);
        let dy = (i / n) as f64 - number(rs, "y", 0.);
        return (dx * ux + dy * uy).abs()
            <= (number(rs, "span", 25.) * 0.5).clamp(10., 20.) + 5. + pad
            && (-dx * uy + dy * ux).abs()
                <= ramp_width(rs).expect("validated width") * 0.5 + 5. + pad;
    }
    ((i % n) as f64 - number(rs, "x", 0.)).abs() <= number(rs, "w", 32.) * 0.5 + pad
        && ((i / n) as f64 - number(rs, "y", 0.)).abs() <= number(rs, "h", 32.) * 0.5 + pad
}
pub fn carve(z: &mut [f64], n: usize, ramps: &[Value], p: &Project) -> Result<()> {
    let h = z.len() / n;
    let ground = z.to_vec();
    for (index, rs) in ramps.iter().enumerate() {
        let (ux, uy, lo, hi) = ramp_axes(rs)?;
        let width = ramp_width(rs)?;
        if (hi - lo).abs() < 5. {
            continue;
        }
        if rs["style"] == "mountain" {
            crate::terrain::ramps::carve_mountain(z, n, rs, p.number("Z_SCALE", 0.0390625))
                .map_err(|e| err(format!("ramp #{}: {e}", index + 1)))?;
            continue;
        }
        let cx = number(rs, "x", 0.);
        let cy = number(rs, "y", 0.);
        let bw = number(rs, "w", 56.) * 0.5;
        let bh = number(rs, "h", 56.) * 0.5;
        let auto = rs["auto"].as_bool().unwrap_or(false);
        let half_u = if auto {
            (number(rs, "span", 32.) * 0.5).clamp(10., 20.)
        } else {
            (bw * 1.5).max(16.).min((bh * 1.5).max(16.)) * 0.5_f64.max(0.)
        };
        let half_u = if auto {
            half_u
        } else {
            half_u
                .max(6.)
                .max(0.75 * (hi - lo).abs() / p.terrain_number("ramp_max_step", 2.5).max(0.5))
        };
        let half_v = ((-bw * uy + bh * ux).abs())
            .max((-bw * uy - bh * ux).abs())
            .max(width * 0.5);
        let len = 2. * half_u;
        let feather = p.terrain_number("ramp_feather", 0.);
        let profile = (0..=(len.ceil() as usize))
            .map(|i| {
                let t = i as f64 / len;
                lo + (hi - lo) * (3. * t * t - 2. * t * t * t)
            })
            .collect::<Vec<_>>();
        let r = (half_u + half_v + bw + bh + feather + 2.).ceil() as isize;
        for y in (cy as isize - r).max(0)..=(cy as isize + r).min(h as isize - 1) {
            for x in (cx as isize - r).max(0)..=(cx as isize + r).min(n as isize - 1) {
                let rx = x as f64 - cx;
                let ry = y as f64 - cy;
                let u = rx * ux + ry * uy;
                let v = -rx * uy + ry * ux;
                let outside = (u.abs() - half_u).max(v.abs() - half_v).max(0.);
                let in_box = rx.abs() <= bw && ry.abs() <= bh;
                let weight = if in_box || outside <= 0. {
                    1.
                } else if feather > 0. {
                    (1. - outside / feather).clamp(0., 1.)
                } else {
                    0.
                };
                if weight == 0. {
                    continue;
                }
                let uu = (u + half_u).clamp(0., len);
                let k = (uu.floor() as usize).min(profile.len() - 2);
                let f = (uu - k as f64).clamp(0., 1.);
                let h = profile[k] * (1. - f) + profile[k + 1] * f;
                let i = y as usize * n + x as usize;
                if !ramp_layer_allowed(ground[i], lo, hi) {
                    return Err(err(format!(
                        "ramp #{} crosses a third terrain level at ({x},{y}); direct non-adjacent level connections are allowed",
                        index + 1
                    )));
                }
                z[i] = z[i] * (1. - weight) + h * weight;
            }
        }
    }
    Ok(())
}
impl Terrain {
    pub fn height(&self) -> usize {
        self.raw.len() / self.n
    }
    pub fn generate(p: &Project) -> Result<Self> {
        crate::coordinates::prepare(p)?;
        if !p.terrain_flag("use_v11_layers")
            || !p.cfg["TERRAIN"]["v11_plain"].as_bool().unwrap_or(true)
        {
            return Err(err("native terrain requires a declarative v11 blueprint"));
        }
        let n = p.usize("GRID", 0);
        let h = p.usize("GRID_HEIGHT", n);
        let layout = Layout::load(p)?;
        let archive = unzip(&fs::read(p.path(p.string("LAYERS", "layers.npz"))?)?)?;
        let base = Npy::parse(
            archive
                .get("base_z.npy")
                .ok_or_else(|| err("blueprint missing base_z"))?,
        )?;
        if base.shape != [h, n] {
            return Err(err("blueprint size differs from GRID / GRID_HEIGHT"));
        }
        let scale = Npy::parse(
            archive
                .get("z_scale.npy")
                .ok_or_else(|| err("blueprint missing z_scale"))?,
        )?
        .numbers()?[0];
        let mut z = base
            .numbers()?
            .into_iter()
            .map(|v| v * scale)
            .collect::<Vec<_>>();
        let water = p.number("WATER_Z", 200.);
        let sea = p.terrain_number("sea_flat", p.number("LAND_Z", 210.) - 20.);
        if !p.terrain_flag("v11_baked") {
            if p.terrain_number("v11_smooth", 0.) != 0. {
                return Err(err(
                    "native blueprint smoothing must be authored in the height input",
                ));
            }
            for v in &mut z {
                if *v < water {
                    *v = sea;
                }
            }
            let standard = p.string("HEIGHT_PRESET", "") == "standard";
            let levels = if standard {
                vec![210., 280., 350., 420.]
            } else {
                let mut lv = vec![p.number("LAND_Z", 210.) + 2.];
                if let Some(a) = p.cfg["TERRAIN"]["levels"].as_array() {
                    lv.extend(
                        a.iter()
                            .filter_map(Value::as_f64)
                            .filter(|v| *v > p.number("LAND_Z", 210.) + 2.),
                    );
                }
                lv.sort_by(f64::total_cmp);
                lv.dedup();
                lv
            };
            let dist = if standard {
                distance(&z.iter().map(|&v| v >= water).collect::<Vec<_>>(), n)
            } else {
                vec![0.; z.len()]
            };
            for (i, v) in z.iter_mut().enumerate() {
                *v = if *v < water {
                    if standard {
                        130. - ((dist[i] - 80.) / 220.).clamp(0., 1.) * 130.
                    } else {
                        sea
                    }
                } else if standard && *v >= 450. {
                    500. + ((*v - 450.) / 550.).clamp(0., 1.) * 500.
                } else {
                    *levels
                        .iter()
                        .min_by(|a, b| (**a - *v).abs().total_cmp(&(**b - *v).abs()))
                        .unwrap()
                };
            }
            carve(&mut z, n, &layout.ramps, p)?;
        }
        if let Some(coasts) = p.cfg["TERRAIN"]["landing_coasts"].as_array() {
            let dist = distance(&z.iter().map(|&v| v >= water).collect::<Vec<_>>(), n);
            for (i, v) in z.iter_mut().enumerate() {
                if *v < water
                    && (1. ..=12.).contains(&dist[i])
                    && coasts.iter().any(|c| {
                        point(c).is_ok_and(|pt| disk(i, n, pt, c[2].as_f64().unwrap_or(0.)))
                    })
                    && !layout.ramps.iter().any(|rs| ramp_cover(i, n, rs, 1.))
                {
                    let t = ((12. - dist[i]) / 11.).clamp(0., 1.);
                    *v = v.max(sea + ((water - 8.).min(sea + 70.) - sea) * t * t * (3. - 2. * t));
                }
            }
        }
        let land = z.iter().map(|&v| v >= water).collect::<Vec<_>>();
        let radius = p.terrain_number("base_min_flat_radius", 24.);
        let levels = if p.string("HEIGHT_PRESET", "") == "standard" {
            [210., 280., 350., 420.]
        } else {
            [212., 280., 350., 420.]
        };
        if radius > 0. {
            let pads = if layout.base_pad_cells.is_empty() {
                &layout.base_cells
            } else {
                &layout.base_pad_cells
            };
            for [bx, by] in pads {
                let ix = bx.round_ties_even() as usize;
                let iy = by.round_ties_even() as usize;
                let zv = z[iy * n + ix];
                let lv = if zv < water {
                    sea
                } else {
                    *levels
                        .iter()
                        .min_by(|a, b| (**a - zv).abs().total_cmp(&(**b - zv).abs()))
                        .unwrap()
                };
                if zv >= water && (zv - lv).abs() > 40. {
                    continue;
                }
                let r = radius.ceil() as usize + 2;
                for y in iy.saturating_sub(r)..=(iy + r).min(h - 1) {
                    for x in ix.saturating_sub(r)..=(ix + r).min(n - 1) {
                        let i = y * n + x;
                        let d = (x as f64 - ix as f64).hypot(y as f64 - iy as f64);
                        let mut w = ((radius - d) / 4.).clamp(0., 1.);
                        if if zv < water {
                            z[i] >= water
                        } else {
                            (z[i] - lv).abs() >= 1.5
                        } {
                            w = 0.;
                        }
                        z[i] = z[i] * (1. - w) + lv * w;
                    }
                }
            }
        }
        let zscale = p.number("Z_SCALE", 0.);
        let raw = z
            .iter()
            .map(|v| (v / zscale).round_ties_even().clamp(0., 65535.) as u16)
            .collect::<Vec<_>>();
        let z = raw.iter().map(|&v| v as f64 * zscale).collect::<Vec<_>>();
        let slope = gradient(&z, n);
        Ok(Self {
            n,
            raw,
            land,
            z,
            slope,
            layout,
        })
    }
    pub fn heightmap(&self, p: &Project) -> HeightMap {
        HeightMap {
            width: self.n,
            height: self.height(),
            border: p.usize("BORDER", 20) as u32,
            borders: vec![[
                0,
                0,
                (self.n - 2 * p.usize("BORDER", 20)) as u32,
                (self.height() - 2 * p.usize("BORDER", 20)) as u32,
            ]],
            elevations: flip(&self.raw, self.n),
        }
    }
    pub fn save(&self, p: &Project) -> Result<()> {
        p.ensure_dirs()?;
        crate::coordinates::prepare(p)?;
        let raw = self
            .raw
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let land = self.land.iter().map(|v| *v as u8).collect::<Vec<_>>();
        let arrays = vec![
            (
                "raw.npy".into(),
                encode("<u2", &[self.height(), self.n], &raw),
            ),
            (
                "land.npy".into(),
                encode("|b1", &[self.height(), self.n], &land),
            ),
        ];
        fs::write(p.source().join("terrain.npz"), zip(&arrays)?)?;
        write_json(&p.source().join("layout.json"), &self.layout)?;
        Ok(())
    }
    pub fn load(p: &Project) -> Result<Self> {
        let archive = unzip(&fs::read(p.source().join("terrain.npz"))?)?;
        let raw = Npy::parse(
            archive
                .get("raw.npy")
                .ok_or_else(|| err("terrain missing raw"))?,
        )?;
        let n = p.usize("GRID", 0);
        let h = p.usize("GRID_HEIGHT", n);
        if raw.shape != [h, n] {
            return Err(err("cached terrain size differs from recipe"));
        }
        let raw = raw
            .numbers()?
            .into_iter()
            .map(|v| v as u16)
            .collect::<Vec<_>>();
        let land_array = Npy::parse(
            archive
                .get("land.npy")
                .ok_or_else(|| err("terrain missing land"))?,
        )?;
        if land_array.shape != [h, n] {
            return Err(err("cached land mask size differs from recipe"));
        }
        let land = land_array
            .numbers()?
            .into_iter()
            .map(|v| v != 0.)
            .collect::<Vec<_>>();
        let layout: Layout = serde_json::from_slice(&fs::read(p.source().join("layout.json"))?)?;
        let z = raw
            .iter()
            .map(|&v| v as f64 * p.number("Z_SCALE", 0.))
            .collect::<Vec<_>>();
        let slope = gradient(&z, n);
        Ok(Self {
            n,
            raw,
            land,
            z,
            slope,
            layout,
        })
    }
}
pub fn compare_raw(path: &Path, terrain: &Terrain) -> Result<()> {
    let expected = Npy::parse(&fs::read(path)?)?.numbers()?;
    let mismatches = expected
        .iter()
        .zip(&terrain.raw)
        .filter(|(a, b)| **a != **b as f64)
        .count();
    if expected.len() != terrain.raw.len() || mismatches != 0 {
        return Err(err(format!(
            "terrain regression: {mismatches} cells differ"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Project);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ra3maps-layers-path-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let recipe = root.join("maps/Test");
            write_json(
                &recipe.join("config.json"),
                &json!({
                    "SCHEMA_VERSION": 1, "MAP_NAME": "Test", "GRID": 8,
                    "BORDER": 0, "CELL": 10., "Z_SCALE": 0.5,
                    "TERRAIN_ONLY": true,
                    "TERRAIN": {"use_v11_layers": true, "v11_baked": true,
                        "base_min_flat_radius": 0}
                }),
            )
            .unwrap();
            write_json(
                &recipe.join("layout.json"),
                &json!({"schema_version": 1, "base_cells": vec![[0, 0]; 8],
                    "ore_cells": [], "ore_angles": [], "objects": []}),
            )
            .unwrap();
            Self(Project::load(&root, "Test").unwrap())
        }
        fn blueprint(&self, path: &str, height: u16) -> PathBuf {
            let bytes = std::iter::repeat_n(height, 64)
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>();
            let arrays = vec![
                ("base_z.npy".into(), encode("<u2", &[8, 8], &bytes)),
                (
                    "z_scale.npy".into(),
                    encode("<f8", &[], &0.5f64.to_le_bytes()),
                ),
            ];
            let path = self.0.path(path).unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, zip(&arrays).unwrap()).unwrap();
            path
        }
    }

    #[test]
    fn layers_default_reads_the_functional_filename() {
        let fixture = Fixture::new();
        fixture.blueprint("layers.npz", 420);
        fixture.blueprint("v11_layers.npz", 700);
        assert_eq!(Terrain::generate(&fixture.0).unwrap().raw, vec![420; 64]);
    }

    #[test]
    fn layers_explicit_path_selects_the_same_blueprint_bytes() {
        let mut fixture = Fixture::new();
        let original = fixture.blueprint("layers.npz", 700);
        let versioned = fixture.blueprint("histories/v11/layers.npz", 420);
        fixture.0.cfg["LAYERS"] = json!("histories/v11/layers.npz");
        let from_version = Terrain::generate(&fixture.0).unwrap();
        fs::copy(versioned, original).unwrap();
        fixture.0.cfg.as_object_mut().unwrap().remove("LAYERS");
        assert_eq!(from_version.raw, Terrain::generate(&fixture.0).unwrap().raw);
        assert_eq!(from_version.raw, vec![420; 64]);
    }

    #[test]
    fn layers_paths_cannot_escape_the_recipe() {
        let mut fixture = Fixture::new();
        fixture.blueprint("layers.npz", 420);
        for path in ["../layers.npz", "/outside/layers.npz"] {
            fixture.0.cfg["LAYERS"] = json!(path);
            assert!(
                Terrain::generate(&fixture.0)
                    .unwrap_err()
                    .to_string()
                    .contains("recipe paths must be relative"),
                "{path}"
            );
        }
    }

    #[test]
    fn layers_missing_inputs_do_not_fall_back_to_other_filenames() {
        let mut fixture = Fixture::new();
        fixture.blueprint("v11_layers.npz", 420);
        assert!(Terrain::generate(&fixture.0).is_err());
        fixture.blueprint("layers.npz", 420);
        fixture.0.cfg["LAYERS"] = json!("missing/layers.npz");
        assert!(Terrain::generate(&fixture.0).is_err());
    }
}
