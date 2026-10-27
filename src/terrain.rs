use crate::{
    Result,
    archive::{Npy, unzip, zip},
    err,
    map::{HeightMap, MapObject, Property},
    project::{Project, write_json},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectSpec {
    pub cell: [f64; 2],
    pub version: u16,
    pub angle: f32,
    pub road_type: u32,
    pub type_name: String,
    pub props: Vec<Property>,
}
impl ObjectSpec {
    pub fn object(&self, p: &Project) -> MapObject {
        let position = crate::coordinates::Frame::from_project(p).world(self.cell);
        MapObject {
            version: self.version,
            position: [position[0] as f32, position[1] as f32, 0.],
            angle: self.angle,
            road_type: self.road_type,
            type_name: self.type_name.clone(),
            props: self.props.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub schema_version: u32,
    pub base_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub base_pad_cells: Vec<[f64; 2]>,
    pub ore_cells: Vec<[f64; 2]>,
    pub ore_angles: Vec<f64>,
    #[serde(default)]
    pub ore_kind: Vec<i8>,
    #[serde(default)]
    pub ore_owner: Vec<i32>,
    #[serde(default)]
    pub derrick_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub house_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub post_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub garage_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub airport_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub hospital_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub academy_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub shipyard_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub bridge_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub ramps: Vec<Value>,
    pub objects: Vec<ObjectSpec>,
    #[serde(default)]
    pub city_cells: Vec<[f64; 2]>,
}
impl Layout {
    pub fn load(p: &Project) -> Result<Self> {
        if crate::coordinate_pipeline::enabled(p) {
            return crate::coordinate_pipeline::layout(p);
        }
        let layout: Self =
            serde_json::from_slice(&fs::read(p.path(p.string("LAYOUT", "layout.json"))?)?)?;
        if layout.schema_version != 1 {
            return Err(err("unsupported layout schema"));
        }
        let n = p.number("GRID", 0.);
        for c in layout
            .base_cells
            .iter()
            .chain(&layout.ore_cells)
            .chain(&layout.base_pad_cells)
            .chain(&layout.derrick_cells)
            .chain(&layout.house_cells)
            .chain(&layout.post_cells)
            .chain(&layout.garage_cells)
            .chain(&layout.airport_cells)
            .chain(&layout.hospital_cells)
            .chain(&layout.academy_cells)
            .chain(&layout.shipyard_cells)
            .chain(&layout.bridge_cells)
            .chain(layout.objects.iter().map(|o| &o.cell))
        {
            if c.iter().any(|v| !v.is_finite() || *v < 0. || *v >= n) {
                return Err(err("layout coordinate outside grid"));
            }
        }
        if layout.base_cells.len() != 8 {
            return Err(err("8-player template requires eight base cells"));
        }
        if layout.ore_cells.len() != layout.ore_angles.len() {
            return Err(err("ore positions and angles differ in length"));
        }
        for ramp in &layout.ramps {
            let x = ramp["x"].as_f64().ok_or_else(|| err("ramp missing x"))?;
            let y = ramp["y"].as_f64().ok_or_else(|| err("ramp missing y"))?;
            if x < 0.
                || y < 0.
                || x >= n
                || y >= n
                || number(ramp, "w", 0.) <= 0.
                || number(ramp, "h", 0.) <= 0.
            {
                return Err(err("invalid ramp center or size"));
            }
            ramp_axes(ramp)?;
            ramp_width(ramp)?;
        }
        if !p.flag("TERRAIN_ONLY") {
            for (i, base) in layout.base_cells.iter().enumerate() {
                let name = if i < 6 {
                    format!("Player_{}_Start", i + 1)
                } else {
                    format!("Player_{}_Start (imported)", i + 1)
                };
                let object = layout
                    .objects
                    .iter()
                    .find(|o| {
                        crate::map::prop(&o.props, "waypointName").and_then(crate::map::Value::text)
                            == Some(name.as_str())
                    })
                    .ok_or_else(|| err(format!("layout missing start object {name}")))?;
                if (object.cell[0] - base[0]).hypot(object.cell[1] - base[1]) > 0.001 {
                    return Err(err(format!(
                        "{name}: base_cells and object coordinates differ; update both"
                    )));
                }
            }
            let ores = layout
                .objects
                .iter()
                .filter(|o| o.type_name == "OreNode")
                .collect::<Vec<_>>();
            if ores.len() != layout.ore_cells.len() {
                return Err(err("ore_cells and OreNode object counts differ"));
            }
            for ore in &layout.ore_cells {
                if ores
                    .iter()
                    .filter(|o| (o.cell[0] - ore[0]).hypot(o.cell[1] - ore[1]) <= 0.001)
                    .count()
                    != 1
                {
                    return Err(err(
                        "ore_cells and OreNode object coordinates differ; update both",
                    ));
                }
            }
        }
        Ok(layout)
    }
}
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
// Preserve the established chamfer weight exactly for blueprint compatibility.
#[allow(clippy::approx_constant)]
pub fn distance(mask: &[bool], n: usize) -> Vec<f64> {
    let mut d = mask
        .iter()
        .map(|&v| if v { 0. } else { 1e9 })
        .collect::<Vec<f64>>();
    for y in 0..n {
        for x in 0..n {
            let i = y * n + x;
            if y > 0 {
                d[i] = d[i].min(d[i - n] + 1.);
                if x > 0 {
                    d[i] = d[i].min(d[i - n - 1] + 1.4142);
                }
                if x + 1 < n {
                    d[i] = d[i].min(d[i - n + 1] + 1.4142);
                }
            }
        }
        for x in 1..n {
            let i = y * n + x;
            d[i] = d[i].min(d[i - 1] + 1.);
        }
    }
    for y in (0..n).rev() {
        for x in 0..n {
            let i = y * n + x;
            if y + 1 < n {
                d[i] = d[i].min(d[i + n] + 1.);
                if x > 0 {
                    d[i] = d[i].min(d[i + n - 1] + 1.4142);
                }
                if x + 1 < n {
                    d[i] = d[i].min(d[i + n + 1] + 1.4142);
                }
            }
        }
        for x in (0..n - 1).rev() {
            let i = y * n + x;
            d[i] = d[i].min(d[i + 1] + 1.);
        }
    }
    d
}
pub fn gradient(z: &[f64], n: usize) -> Vec<f64> {
    (0..z.len())
        .map(|i| {
            let x = i % n;
            let y = i / n;
            let gx = if x == 0 {
                z[i + 1] - z[i]
            } else if x == n - 1 {
                z[i] - z[i - 1]
            } else {
                (z[i + 1] - z[i - 1]) * 0.5
            };
            let gy = if y == 0 {
                z[i + n] - z[i]
            } else if y == n - 1 {
                z[i] - z[i - n]
            } else {
                (z[i + n] - z[i - n]) * 0.5
            };
            (gx * gx + gy * gy).sqrt()
        })
        .collect()
}
pub fn edge(i: usize, n: usize, border: usize) -> f64 {
    let x = (i % n) as isize;
    let y = (i / n) as isize;
    let b = border as isize;
    [x - b, y - b, n as isize - 1 - b - x, n as isize - 1 - b - y]
        .into_iter()
        .min()
        .unwrap() as f64
}
pub fn disk(i: usize, n: usize, c: [f64; 2], r: f64) -> bool {
    ((i % n) as f64 - c[0]).powi(2) + ((i / n) as f64 - c[1]).powi(2) <= r * r
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
    ((i % n) as f64 - number(rs, "x", 0.)).abs() <= number(rs, "w", 32.) * 0.5 + pad
        && ((i / n) as f64 - number(rs, "y", 0.)).abs() <= number(rs, "h", 32.) * 0.5 + pad
}
pub fn carve(z: &mut [f64], n: usize, ramps: &[Value], p: &Project) -> Result<()> {
    let ground = z.to_vec();
    for (index, rs) in ramps.iter().enumerate() {
        let (ux, uy, lo, hi) = ramp_axes(rs)?;
        let width = ramp_width(rs)?;
        if (hi - lo).abs() < 5. {
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
        for y in (cy as isize - r).max(0)..=(cy as isize + r).min(n as isize - 1) {
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
    pub fn generate(p: &Project) -> Result<Self> {
        if !p.terrain_flag("use_v11_layers")
            || !p.cfg["TERRAIN"]["v11_plain"].as_bool().unwrap_or(true)
        {
            return Err(err("native terrain requires a declarative v11 blueprint"));
        }
        let n = p.usize("GRID", 0);
        let layout = Layout::load(p)?;
        let archive = unzip(&fs::read(p.path("v11_layers.npz")?)?)?;
        let base = Npy::parse(
            archive
                .get("base_z.npy")
                .ok_or_else(|| err("blueprint missing base_z"))?,
        )?;
        if base.shape != [n, n] {
            return Err(err("blueprint size differs from GRID"));
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
                for y in iy.saturating_sub(r)..=(iy + r).min(n - 1) {
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
            height: self.n,
            border: p.usize("BORDER", 20) as u32,
            borders: vec![[
                0,
                0,
                (self.n - 2 * p.usize("BORDER", 20)) as u32,
                (self.n - 2 * p.usize("BORDER", 20)) as u32,
            ]],
            elevations: flip(&self.raw, self.n),
        }
    }
    pub fn save(&self, p: &Project) -> Result<()> {
        p.ensure_dirs()?;
        let raw = self
            .raw
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        let land = self.land.iter().map(|v| *v as u8).collect::<Vec<_>>();
        let arrays = vec![
            ("raw.npy".into(), npy("<u2", &[self.n, self.n], &raw)),
            ("land.npy".into(), npy("|b1", &[self.n, self.n], &land)),
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
        if raw.shape != [n, n] {
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
        if land_array.shape != [n, n] {
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
pub fn flip<T: Clone>(data: &[T], n: usize) -> Vec<T> {
    data.chunks(n)
        .rev()
        .flat_map(|r| r.iter().cloned())
        .collect()
}
pub fn npy(dtype: &str, shape: &[usize], data: &[u8]) -> Vec<u8> {
    let dims = shape
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let tuple = if shape.is_empty() {
        String::new()
    } else {
        format!("{dims},")
    };
    let mut header =
        format!("{{'descr': '{dtype}', 'fortran_order': False, 'shape': ({tuple}), }}");
    let padding = (64 - (10 + header.len() + 1) % 64) % 64;
    header.extend(std::iter::repeat_n(' ', padding));
    header.push('\n');
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend_from_slice(&(header.len() as u16).to_le_bytes());
    out.extend(header.as_bytes());
    out.extend(data);
    out
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
