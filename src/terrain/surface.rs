use crate::{
    Result, err,
    formats::binary::Writer,
    formats::map::write_bits,
    pipeline::project::Project,
    terrain::{Terrain, disk, distance, flip, number, ramp_axes, ramp_cover},
};
use serde_json::Value;
#[derive(Debug)]
pub struct Surface {
    pub chosen: Vec<u16>,
    pub impassable: Vec<bool>,
    pub build_block: Vec<bool>,
    pub layer_edge: Vec<bool>,
    pub shore: Vec<bool>,
    pub textures: Vec<[String; 3]>,
}
pub fn dilate(mask: &[bool], n: usize, r: usize) -> Vec<bool> {
    let h = mask.len() / n;
    let mut integral = vec![0u32; (n + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0;
        for x in 0..n {
            row += mask[y * n + x] as u32;
            integral[(y + 1) * (n + 1) + x + 1] = integral[y * (n + 1) + x + 1] + row;
        }
    }
    let mut out = vec![false; mask.len()];
    for y in 0..h {
        for x in 0..n {
            let x0 = x.saturating_sub(r);
            let y0 = y.saturating_sub(r);
            let x1 = (x + r + 1).min(n);
            let y1 = (y + r + 1).min(h);
            let sum = integral[y1 * (n + 1) + x1] as i64 + integral[y0 * (n + 1) + x0] as i64
                - integral[y0 * (n + 1) + x1] as i64
                - integral[y1 * (n + 1) + x0] as i64;
            out[y * n + x] = sum > 0;
        }
    }
    out
}
/// MT19937 and 53-bit samples match the established texture noise seed.
struct Mt {
    state: [u32; 624],
    index: usize,
}
impl Mt {
    fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            state[i] = 1812433253u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }
    fn next(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }
    fn sample(&mut self) -> f64 {
        let a = (self.next() >> 5) as u64;
        let b = (self.next() >> 6) as u64;
        ((a << 26) + b) as f64 / 9007199254740992.
    }
}
pub(crate) fn noise_rect(n: usize, h: usize, cells: f64, seed: u32) -> Vec<f64> {
    let size = (n.max(h) as f64 / cells).ceil() as usize + 2;
    let mut rng = Mt::new(seed);
    let g = (0..size * size).map(|_| rng.sample()).collect::<Vec<_>>();
    let mut out = vec![0.; n * h];
    for y in 0..h {
        let gy = y as f64 / cells;
        let iy = gy.floor() as usize;
        let f = gy - iy as f64;
        let fy = f * f * (3. - 2. * f);
        for x in 0..n {
            let gx = x as f64 / cells;
            let ix = gx.floor() as usize;
            let f = gx - ix as f64;
            let fx = f * f * (3. - 2. * f);
            let top = g[iy * size + ix] * (1. - fx) + g[iy * size + ix + 1] * fx;
            let bot = g[(iy + 1) * size + ix] * (1. - fx) + g[(iy + 1) * size + ix + 1] * fx;
            out[y * n + x] = top * (1. - fy) + bot * fy;
        }
    }
    out
}
pub fn textures(p: &Project) -> Result<Vec<[String; 3]>> {
    let a = p.cfg["TEX_SET"]
        .as_array()
        .ok_or_else(|| err("missing TEX_SET"))?;
    let mut out = Vec::new();
    for row in a {
        if let Some(name) = row.as_str() {
            out.push(
                crate::resources::official::terrain_row(name).ok_or_else(|| {
                    err(format!(
                        "official terrain lacks diffuse/normal resources for {name}"
                    ))
                })?,
            );
            continue;
        }
        let mut names = [String::new(), String::new(), String::new()];
        for i in 0..3 {
            names[i] = row[i]
                .as_str()
                .ok_or_else(|| err("texture requires name, diffuse and normal"))?
                .into();
        }
        out.push(names);
    }
    if p.string("LAYER_TEX_PRESET", "") == "mountain" {
        for name in [
            "Grass_NewYork01",
            "Grass_MtRush04",
            "Dirt_Iceland05",
            "Pavement_Amsterdam05",
        ] {
            if !out.iter().any(|v| v[0] == name) {
                out.push(
                    crate::resources::official::terrain_row(name).ok_or_else(|| {
                        err(format!(
                            "official terrain lacks diffuse/normal resources for {name}"
                        ))
                    })?,
                );
            }
        }
    }
    if crate::terrain::texture_polish::enabled(p)? {
        for row in crate::terrain::materials::STOCK {
            if !out.iter().any(|v| v[0] == row[0]) {
                out.push(
                    crate::resources::official::terrain_row(row[0]).ok_or_else(|| {
                        err(format!(
                            "official terrain lacks diffuse/normal resources for {}",
                            row[0]
                        ))
                    })?,
                );
            }
        }
    }
    if out.is_empty() || out.len() > 1024 {
        return Err(err("invalid texture count"));
    }
    Ok(out)
}
impl Surface {
    pub fn generate(t: &Terrain, p: &Project) -> Result<Self> {
        let n = t.n;
        let h = t.height();
        let textures = textures(p)?;
        let index = |name: &str| textures.iter().position(|r| r[0] == name).map(|i| i as u16);
        let water = p.number("WATER_Z", 200.);
        let mut chosen = vec![0; n * h];
        let grass = p.cfg["GRASS_TEXTURES"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .filter_map(index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let grass = if grass.is_empty() {
            textures
                .iter()
                .enumerate()
                .filter(|(_, r)| r[0].starts_with("Grass"))
                .map(|(i, _)| i as u16)
                .collect()
        } else {
            grass
        };
        if grass.is_empty() {
            return Err(err("no grass textures"));
        }
        let mut layers = Vec::<(f64, Vec<u16>)>::new();
        if let Some(map) = p.cfg["LAYER_TEX"].as_object() {
            for (k, vals) in map {
                let z = k.parse::<f64>()?;
                let variants = vals
                    .as_array()
                    .ok_or_else(|| err("invalid LAYER_TEX"))?
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(index)
                    .collect::<Vec<_>>();
                layers.push((z, variants));
            }
        } else if p.string("LAYER_TEX_PRESET", "") == "mountain" {
            let legacy = [
                "Grass_NewYork01",
                "Grass_MtRush04",
                "Dirt_Iceland05",
                "Pavement_Amsterdam05",
            ];
            let primary: &[&str] = if crate::terrain::texture_polish::enabled(p)? {
                &crate::terrain::materials::PRIMARY
            } else {
                &legacy
            };
            for (i, name) in primary.iter().enumerate() {
                if let Some(z) = p.cfg["LAYER_M"][i][1].as_f64() {
                    layers.push((z, vec![index(name).unwrap()]));
                }
            }
        }
        layers.sort_by(|a, b| a.0.total_cmp(&b.0));
        let strict = p.flag("LAYER_TEX_STRICT") && !layers.is_empty();
        let cells = p.number("TEX_NOISE_CELLS", 9.);
        if cells <= 0. {
            return Err(err("TEX_NOISE_CELLS must be positive"));
        }
        let a = noise_rect(n, h, cells, 909);
        let b = noise_rect(n, h, (cells * 0.5).max(2.), 909 + 977);
        let noise = a
            .iter()
            .zip(b)
            .map(|(a, b)| (a + 0.5 * b) / 1.5)
            .collect::<Vec<_>>();
        let wet = t.z.iter().map(|v| *v < water).collect::<Vec<_>>();
        let dw = distance(&wet, n);
        let dl = distance(&wet.iter().map(|v| !*v).collect::<Vec<_>>(), n);
        let city_r = p.number("CITY_TEX_R", 30.);
        let city = (0..n * h)
            .map(|i| {
                t.layout
                    .city_cells
                    .iter()
                    .any(|&c| disk(i, n, c, city_r - 1e-9))
            })
            .collect::<Vec<_>>();
        let mut city_tex = p.cfg["CITY_TEX"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(Value::as_str)
                    .filter_map(index)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                ["SteelDeck01", "BB_Gravel01", "Dirt_Yucatan06"]
                    .into_iter()
                    .filter_map(index)
                    .collect()
            });
        if city_tex.is_empty() {
            city_tex = grass.clone();
        }
        for by in (0..h).step_by(8) {
            for bx in (0..n).step_by(8) {
                let i = by * n + bx;
                let mut zz = 0.;
                let mut ss = 0.;
                let mut ll = 0;
                let mut votes = vec![0usize; layers.len()];
                for y in by..by + 8 {
                    for x in bx..bx + 8 {
                        let j = y * n + x;
                        zz += t.z[j];
                        ss += t.slope[j];
                        ll += t.land[j] as usize;
                        if t.z[j] >= water && !layers.is_empty() {
                            let k = layers
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| {
                                    (a.0 - t.z[j]).abs().total_cmp(&(b.0 - t.z[j]).abs())
                                })
                                .unwrap()
                                .0;
                            votes[k] += 1;
                        }
                    }
                }
                zz /= 64.;
                ss /= 64.;
                let pick = if !p.flag("CITY_TEX_ROUND") && city[i] && ll > 32 && zz > water + 3. {
                    city_tex[(noise[i] * city_tex.len() as f64) as usize % city_tex.len()]
                } else if zz >= water && dw[i] > 0. && dw[i] <= p.number("BEACH_TEX_R", 6.) {
                    index("Sand_Hawaii02").unwrap_or(grass[0])
                } else if let Some(bands) = p.cfg["SEA_TEX_BANDS"].as_array().filter(|_| zz < water)
                {
                    let row = bands
                        .iter()
                        .find(|r| dl[i] <= r[0].as_f64().unwrap_or(0.))
                        .or_else(|| bands.last())
                        .ok_or_else(|| err("empty SEA_TEX_BANDS"))?;
                    index(row[1].as_str().unwrap_or(""))
                        .unwrap_or(index("RA3_ShallowSeaFloor").unwrap_or(grass[0]))
                } else {
                    let name = if strict && zz >= water {
                        None
                    } else if zz < water - 60. {
                        Some("RA3_ShallowSeaFloor")
                    } else if zz < water - 30. {
                        Some("Reef_Yucatan01")
                    } else if zz < water {
                        Some("Reef_Yucatan02")
                    } else if zz < water + 3. {
                        Some(if ll > 32 {
                            "Sand_Hawaii02"
                        } else {
                            "Reef_Havana02"
                        })
                    } else if ss > 34. {
                        Some(if zz % 2. < 1. {
                            "Rock_Yucatan02"
                        } else {
                            "Rock_Yucatan01"
                        })
                    } else if ss > 22. {
                        Some("Rock_Yucatan03")
                    } else {
                        None
                    };
                    if let Some(name) = name {
                        index(name).ok_or_else(|| err(format!("missing texture {name}")))?
                    } else {
                        let variants = if layers.is_empty() {
                            &grass
                        } else {
                            let k = if strict && votes.iter().sum::<usize>() > 0 {
                                let max = *votes.iter().max().unwrap();
                                votes.iter().position(|v| *v == max).unwrap()
                            } else {
                                layers
                                    .iter()
                                    .enumerate()
                                    .min_by(|(_, a), (_, b)| {
                                        (a.0 - zz).abs().total_cmp(&(b.0 - zz).abs())
                                    })
                                    .unwrap()
                                    .0
                            };
                            if layers[k].1.is_empty() {
                                &grass
                            } else {
                                &layers[k].1
                            }
                        };
                        variants[(noise[i] * variants.len() as f64) as usize % variants.len()]
                    }
                };
                for y in by..by + 8 {
                    for x in bx..bx + 8 {
                        chosen[y * n + x] = pick;
                    }
                }
            }
        }
        if p.flag("CITY_TEX_ROUND") {
            for i in 0..n * h {
                if city[i] && t.land[i] && t.z[i] > water + 3. {
                    chosen[i] =
                        city_tex[(noise[i] * city_tex.len() as f64) as usize % city_tex.len()];
                }
            }
        }
        let mut ramp_tex = vec![false; n * h];
        if let Some(dirt) = index("Dirt_Yucatan06") {
            for rs in &t.layout.ramps {
                if rs["surface_style"] == "mountain" {
                    for (i, marked) in ramp_tex.iter_mut().enumerate() {
                        *marked |= ramp_cover(i, n, rs, 0.);
                    }
                    continue;
                }
                let (ux, uy, lo, hi) = ramp_axes(rs)?;
                let auto = rs["auto"].as_bool().unwrap_or(false);
                let hw = (number(rs, "w", 56.) * 0.75).max(16.);
                let hh = (number(rs, "h", 56.) * 0.75).max(16.);
                let hu = if auto {
                    (number(rs, "span", 28.) * 0.5).clamp(10., 20.)
                } else {
                    (hw.min(hh) * 0.5).max(6.).max(0.75 * (hi - lo).abs() / 2.5) + 2.
                };
                let hv = if auto {
                    (number(rs, "width", 28.) * 0.35).clamp(4., 14.)
                } else {
                    (hw.min(hh) * 0.35).max(4.) + 2.
                };
                let cx = number(rs, "x", 0.);
                let cy = number(rs, "y", 0.);
                for by in (0..h).step_by(8) {
                    for bx in (0..n).step_by(8) {
                        let mut lane = 0;
                        let mut ed = 0;
                        for y in by..by + 8 {
                            for x in bx..bx + 8 {
                                let rx = x as f64 - cx;
                                let ry = y as f64 - cy;
                                let u = (rx * ux + ry * uy).abs();
                                let v = (-rx * uy + ry * ux).abs();
                                let inside =
                                    u <= hu && v <= hv && (auto || t.z[y * n + x] > water + 2.);
                                lane += inside as usize;
                                ed += (u <= hu + 8.
                                    && v <= hv + 8.
                                    && t.z[y * n + x] > water + 2.
                                    && !inside) as usize;
                            }
                        }
                        if lane >= 16 || ed >= 16 {
                            for y in by..by + 8 {
                                for x in bx..bx + 8 {
                                    let i = y * n + x;
                                    if lane >= 16 {
                                        chosen[i] = dirt;
                                    } else if let Some(gravel) = index("Gravel_Yucatan01") {
                                        chosen[i] = gravel;
                                    }
                                    ramp_tex[i] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut step = vec![false; n * h];
        for y in 0..h {
            for x in 0..n {
                let i = y * n + x;
                for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                    if nx < n && ny < h {
                        let j = ny * n + nx;
                        if !wet[i] && !wet[j] && (t.z[i] - t.z[j]).abs() >= 40. {
                            step[i] = true;
                            step[j] = true;
                        }
                    }
                }
            }
        }
        let step = dilate(&step, n, 1);
        let water_band = dilate(&wet, n, 4);
        let layer_edge = (0..n * h)
            .map(|i| step[i] && !wet[i] && !ramp_tex[i])
            .collect::<Vec<_>>();
        let shore = (0..n * h)
            .map(|i| water_band[i] && !wet[i] && !layer_edge[i] && !ramp_tex[i])
            .collect::<Vec<_>>();
        for i in 0..n * h {
            if layer_edge[i] {
                if let Some(rock) = index("Rock_Yucatan01").or_else(|| index("Rock_Yucatan02")) {
                    chosen[i] = rock;
                }
            } else if shore[i]
                && let Some(sand) = index("Sand_Hawaii02")
            {
                chosen[i] = sand;
            }
        }
        let coasts = p.cfg["TERRAIN"]["landing_coasts"].as_array();
        let border = p.usize("BORDER", 20);
        let mut impassable = vec![false; n * h];
        for i in 0..n * h {
            let y = i / n;
            let x = i % n;
            let coast = !wet[i]
                && [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ]
                .into_iter()
                .any(|(x, y)| x < n && y < h && wet[y * n + x]);
            let coast = coast
                && !t.layout.ramps.iter().any(|r| ramp_cover(i, n, r, 1.))
                && !coasts.is_some_and(|a| {
                    a.iter().any(|c| {
                        crate::terrain::point(c)
                            .is_ok_and(|pt| disk(i, n, pt, c[2].as_f64().unwrap_or(0.)))
                    })
                });
            impassable[i] = t.slope[i] > p.number("SLOPE_IMPASSABLE", 24.)
                || t.z[i] > water + p.number("PEAK_IMPASSABLE", 235.)
                || (!t.land[i] && t.z[i] > water)
                || super::grid::edge_rect(i, n, h, border) < 3.
                || coast;
        }
        for &base in &t.layout.base_cells {
            for (i, v) in impassable.iter_mut().enumerate() {
                if disk(i, n, base, p.number("PAD_CLEARANCE", 26.) - 1e-9) {
                    *v = false;
                }
            }
        }
        for &ore in &t.layout.ore_cells {
            for (i, v) in impassable.iter_mut().enumerate() {
                if super::grid::edge_rect(i, n, h, border) >= 1.
                    && disk(
                        i,
                        n,
                        ore,
                        p.number("ORE_CLEAR_RADIUS", 20.)
                            .max(p.terrain_number("ore_mouth_r1", 20.)),
                    )
                {
                    *v = false;
                }
            }
        }
        let mut build_block = vec![false; n * h];
        for o in &t.layout.objects {
            if o.type_name.contains("Tree")
                || o.type_name.contains("Bush")
                || o.type_name.contains("GardenRock")
            {
                let x = o.cell[0].round_ties_even() as usize;
                let y = o.cell[1].round_ties_even() as usize;
                for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                    for xx in x.saturating_sub(1)..=(x + 1).min(n - 1) {
                        build_block[yy * n + xx] = true;
                    }
                }
            }
        }
        let ramp_forbidden = dilate(&ramp_tex, n, 1);
        let mut protected = Vec::<([f64; 2], f64)>::new();
        protected.extend(
            t.layout
                .base_cells
                .iter()
                .map(|&c| (c, p.number("PAD_CLEARANCE", 26.) + 2.)),
        );
        for (cells, radius) in [
            (&t.layout.ore_cells, p.number("ORE_CLEAR_RADIUS", 18.) + 3.),
            (
                &t.layout.derrick_cells,
                p.number("RESOURCE_MIN_SPACING", 14.).max(14.) + 2.,
            ),
            (&t.layout.post_cells, 16.),
            (&t.layout.garage_cells, 20.),
            (&t.layout.airport_cells, 24.),
            (&t.layout.hospital_cells, 16.),
            (&t.layout.academy_cells, 16.),
            (&t.layout.shipyard_cells, 20.),
            (&t.layout.house_cells, 14.),
            (&t.layout.bridge_cells, 16.),
        ] {
            protected.extend(cells.iter().map(|&c| (c, radius)));
        }
        for o in &t.layout.objects {
            if o.type_name.contains("Waypoint")
                || o.type_name.contains("Tree")
                || o.type_name.contains("Bush")
                || o.type_name.contains("GardenRock")
            {
                continue;
            }
            let name = o.type_name.as_str();
            let radius = match name {
                "OreNode" => p.number("ORE_CLEAR_RADIUS", 18.) + 3.,
                "OilDerrick" => p.number("RESOURCE_MIN_SPACING", 14.).max(14.) + 2.,
                "AirportTechStructure" => 24.,
                "GarageTechStructure" | "AlliedNavalYard" => 20.,
                "HospitalTechStructure"
                | "VeterancyTechStructure"
                | "ObservationPostTechStructure" => 16.,
                _ if name.contains("ConstructionYard") || name.ends_with("Yard") => {
                    p.number("PAD_CLEARANCE", 26.)
                }
                _ if name.contains("Bridge") => 16.,
                _ => 14.,
            };
            protected.push((o.cell, radius));
        }
        for (i, blocked) in build_block.iter_mut().enumerate() {
            if *blocked
                && (wet[i]
                    || super::grid::edge_rect(i, n, h, border) < 4.
                    || ramp_forbidden[i]
                    || protected.iter().any(|&(c, r)| disk(i, n, c, r)))
            {
                *blocked = false;
            }
        }
        if t.layout
            .ramps
            .iter()
            .any(|r| r["surface_style"] == "mountain")
        {
            let names: Vec<_> = textures.iter().map(|row| row[0].clone()).collect();
            let mut tiles: Vec<_> = chosen.iter().map(|family| family * 64).collect();
            let mut changed = vec![false; n * h];
            for r in &t.layout.ramps {
                if r["surface_style"] == "mountain" {
                    crate::terrain::ramps::paint_mountain(
                        &mut tiles,
                        &names,
                        &t.z,
                        n,
                        (r, r),
                        water,
                        &mut changed,
                    );
                }
            }
            chosen = tiles.iter().map(|tile| tile / 64).collect();
        }
        Ok(Self {
            chosen,
            impassable,
            build_block,
            layer_edge,
            shore,
            textures,
        })
    }
    pub fn encode(&self, n: usize, blend_descriptions: &[u8]) -> Result<Vec<u8>> {
        let h = self.chosen.len() / n;
        if !blend_descriptions.len().is_multiple_of(18) {
            return Err(err("invalid blend descriptions"));
        }
        let mut w = Writer::new();
        w.u32(u32::try_from(n * h)?);
        let chosen = flip(&self.chosen, n);
        for (i, idx) in chosen.iter().enumerate() {
            let y = (h - 1 - i / n) % 8;
            let x = i % n % 8;
            let sub = ((y / 2 * 4 + x / 2) * 4 + y % 2 * 2 + x % 2) as u16;
            w.u16(idx * 64 + sub);
        }
        w.bytes(&vec![0; n * h * 6]);
        write_bits(&mut w, &flip(&self.impassable, n), n);
        let zero = vec![false; n * h];
        for _ in 0..3 {
            write_bits(&mut w, &zero, n);
        }
        write_bits(&mut w, &vec![true; n * h], n);
        write_bits(&mut w, &flip(&self.build_block, n), n);
        for _ in 0..2 {
            write_bits(&mut w, &zero, n);
        }
        w.bytes(&vec![0; n * h]);
        w.u32((self.textures.len() * 16) as u32);
        w.u32(u32::try_from(blend_descriptions.len() / 18 + 1)?);
        w.u32(1);
        w.u32(self.textures.len() as u32);
        for (i, row) in self.textures.iter().enumerate() {
            w.u32((i * 16) as u32);
            w.u32(16);
            w.u32(4);
            w.u32(0);
            w.s16(&row[0])?;
        }
        w.u32(0xcdcdcdcd);
        w.u32(0);
        w.bytes(blend_descriptions);
        Ok(w.data)
    }
}
