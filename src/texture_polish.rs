//! Repaint a finished map using its registered atlas, without changing gameplay.
use crate::{
    Result, err,
    map::{self, BlendTiles, HeightMap, MapFile},
    project::{self, Project},
    render::{self, Image},
    surface::{self, Surface},
    terrain::{self, Layout, ObjectSpec, Terrain, number},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

/// Accepted T2.2.9 palette and placement rules. Experimental previews are separate.
pub const PROFILE: &str = "classic-natural";

pub fn enabled(p: &Project) -> Result<bool> {
    match p.string("TEXTURE_PROFILE", PROFILE) {
        PROFILE => Ok(true),
        "legacy" => Ok(false),
        name => Err(err(format!("unknown TEXTURE_PROFILE {name}"))),
    }
}

pub struct Polished {
    terrain: Terrain,
    before: Vec<u16>,
    after: Vec<u16>,
    names: Vec<String>,
    impassable: Vec<bool>,
    build_block: Vec<bool>,
    pub report: Value,
}

fn pick(names: &[String], candidates: &[&str], fallback: u16) -> u16 {
    candidates
        .iter()
        .find_map(|name| names.iter().position(|n| n == name))
        .map_or(fallback, |i| i as u16)
}

fn paint(t: &Terrain, p: &Project, names: &[String], before: &[u16]) -> Vec<u16> {
    let n = t.n;
    let water = p.number("WATER_Z", 200.);
    let wet: Vec<_> = t.z.iter().map(|h| *h < water).collect();
    let to_water = terrain::distance(&wet, n);
    let to_land = terrain::distance(&wet.iter().map(|v| !v).collect::<Vec<_>>(), n);
    let broad = surface::noise(n, 52., 2771);
    let detail = surface::noise(n, 17., 9323);
    let field: Vec<_> = broad
        .iter()
        .zip(&detail)
        .map(|(a, b)| a * 0.75 + b * 0.25)
        .collect();
    let fallback = before[0] / 64;
    // These were inspected against the stock diffuse images. Several textures
    // named Grass are authored grass/dirt transitions, not seamless grass:
    // CapeCod01/03/07 and Golf01/03 stamp a visible dirt shape every eight cells.
    let grass = pick(
        names,
        &["Grass_NewYork01", "Grass_CapeCod05", "Grass_Geneva01"],
        fallback,
    );
    // Related grass families stay dominant; soil forms broad patches rather
    // than covering an entire high plateau with pavement or bare dirt.
    let palettes = [
        [
            grass,
            pick(names, &["Grass_CapeCod05", "Grass_NewYork01"], grass),
        ],
        [
            pick(names, &["Grass_MtRush04", "Grass_Geneva01"], grass),
            pick(names, &["Grass_Geneva01", "Grass_MtRush04"], grass),
        ],
        [
            pick(names, &["Grass_CapeCod05", "Grass_MtRush04"], grass),
            pick(names, &["Dirt_Iceland05", "Dirt_Yucatan03"], grass),
        ],
        [
            pick(names, &["Grass_CapeCod12", "Grass_CapeCod05"], grass),
            pick(names, &["Dirt_Iceland05", "Dirt_Yucatan03"], grass),
        ],
    ];
    let dirt = pick(names, &["Dirt_Yucatan06", "Dirt_Yucatan03"], grass);
    let gravel = pick(names, &["Gravel_Yucatan01", "BB_Gravel01"], dirt);
    let rock = pick(names, &["Rock_Yucatan01", "Rock_Yucatan02"], gravel);
    let sand = pick(names, &["Sand_Hawaii02"], dirt);
    let city = pick(names, &["BB_Gravel01", "Gravel_Yucatan01"], gravel);
    let sea = pick(names, &["RA3_ShallowSeaFloor"], fallback);
    let reef = pick(names, &["Reef_Yucatan01", "RA3_ShallowSeaFloor"], sea);
    let shallows = pick(names, &["Reef_Yucatan02", "Reef_Havana02"], reef);
    let mut levels: Vec<_> = p.cfg["LAYER_M"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r[1].as_f64())
        .collect();
    if levels.is_empty() {
        levels = p.cfg["TERRAIN"]["levels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_f64)
            .collect();
    }
    if levels.is_empty() {
        levels = vec![210., 280., 350., 420.];
    }
    levels.sort_by(f64::total_cmp);
    let cliff: Vec<_> = (0..n * n)
        .map(|i| {
            !wet[i]
                && crate::build::neighbors(i, n).any(|j| !wet[j] && (t.z[j] - t.z[i]).abs() >= 35.)
        })
        .collect();
    let to_cliff = terrain::distance(&cliff, n);
    let mut out = vec![0; n * n];
    for i in 0..n * n {
        if wet[i] {
            out[i] = if to_land[i] < 7. + field[i] * 3. {
                shallows
            } else if to_land[i] < 21. + field[i] * 9. {
                reef
            } else {
                sea
            };
            continue;
        }
        let layer = levels
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (**a - t.z[i]).abs().total_cmp(&(**b - t.z[i]).abs()))
            .unwrap()
            .0
            .min(3);
        let patch = if layer < 2 {
            field[i] > 0.58
        } else {
            field[i] > 0.72
        };
        let mut material = palettes[layer][patch as usize];
        let x = (i % n) as f64;
        let y = (i / n) as f64;
        // Compact, irregular gravel clearings replace large square city pads.
        if t.layout.city_cells.iter().any(|c| {
            (x - c[0]).hypot(y - c[1]) < p.number("CITY_TEX_R", 30.) * (0.43 + 0.15 * detail[i])
        }) {
            material = if field[i] > 0.44 { city } else { dirt };
        }
        if to_water[i] <= 3. + 2. * detail[i] {
            material = sand;
        } else if to_water[i] <= 5. + 2. * detail[i] && field[i] > 0.53 {
            material = dirt;
        }
        if to_cliff[i] < 0.6 + detail[i] * 0.8 {
            material = rock;
        } else if to_cliff[i] < 2. + detail[i] * 1.8 {
            material = if field[i] > 0.5 { gravel } else { dirt };
        }
        out[i] = material;
    }
    for ramp in &t.layout.ramps {
        let Ok((ux, uy, lo, hi)) = terrain::ramp_axes(ramp) else {
            continue;
        };
        let length = (number(ramp, "span", 25.) * 0.5).clamp(10., 20.);
        let width = number(ramp, "width", 25.) * 0.5;
        let cx = number(ramp, "x", 0.);
        let cy = number(ramp, "y", 0.);
        let radius = (length + width * 1.25 + 6.).ceil() as isize;
        for y in (cy as isize - radius).max(0)..=(cy as isize + radius).min(n as isize - 1) {
            for x in (cx as isize - radius).max(0)..=(cx as isize + radius).min(n as isize - 1) {
                let i = y as usize * n + x as usize;
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let u = (dx * ux + dy * uy).abs();
                let v = (-dx * uy + dy * ux).abs();
                // Do not visually advertise an adjacent cliff as a road.
                if u > length + 3. || t.slope[i] > 12. || t.z[i] < lo - 3. || t.z[i] > hi + 3. {
                    continue;
                }
                let mouth = width * (1. + 0.15 * (u / length).min(1.).powi(2));
                if v < mouth && u < length + 1. {
                    out[i] = dirt;
                } else if v < mouth + 2. + detail[i] && !wet[i] {
                    out[i] = gravel;
                }
            }
        }
    }
    out
}

fn histogram(tiles: &[u16], names: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for tile in tiles {
        *counts
            .entry(names[(tile / 64) as usize].clone())
            .or_default() += 1;
    }
    counts
}

pub fn apply(p: &Project, map: &mut MapFile, source_name: &str) -> Result<Polished> {
    let source = map.clone();
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    if n != hm.height || n != p.usize("GRID", 0) || map.get("BlendTileData")?.version != 27 {
        return Err(err(
            "texture polishing requires matching square v27 terrain",
        ));
    }
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    let before = terrain::flip(&bt.tiles, n);
    let source_layout = p
        .root
        .join("assets")
        .join(&p.group)
        .join(source_name)
        .join("source/layout.json");
    let mut layout = if source_layout.is_file() {
        serde_json::from_slice::<Layout>(&fs::read(source_layout)?)?
    } else {
        Layout::load(p)?
    };
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let rules = crate::scripts::load_rules(p)?;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !map::prop(&o.props, "waypointName")
                .and_then(map::Value::text)
                .is_some_and(|name| rules.spawn_waypoints.iter().any(|s| s.name == name))
        })
        .map(|o| ObjectSpec {
            cell: [
                o.position[0] as f64 / p.number("CELL", 10.) + p.number("BORDER", 20.),
                n as f64
                    - 1.
                    - p.number("BORDER", 20.)
                    - o.position[1] as f64 / p.number("CELL", 10.),
            ],
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    let raw = terrain::flip(&hm.elevations, n);
    let z: Vec<_> = raw
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let terrain = Terrain {
        n,
        land: z.iter().map(|h| *h >= p.number("WATER_Z", 200.)).collect(),
        slope: terrain::gradient(&z, n),
        raw,
        z,
        layout,
    };
    let families = paint(&terrain, p, &bt.textures, &before);
    let after: Vec<_> = families
        .iter()
        .zip(&before)
        .map(|(family, tile)| family * 64 + tile % 64)
        .collect();
    let impassable = terrain::flip(&bt.impassability, n);
    let build_block = terrain::flip(&bt.buildability, n);
    crate::ramps::patch_blend(
        map,
        &bt,
        &after,
        &impassable,
        &build_block,
        &vec![true; n * n],
        n,
    )?;
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if name != "BlendTileData" && map.get(name)? != chunk {
            return Err(err(format!(
                "texture polishing changed unrelated chunk {name}"
            )));
        }
    }
    // Verify every gameplay bitmap byte, including unused/unknown masks.
    let bitmap_start = 4 + 8 * n * n;
    let bitmap_end = bitmap_start + 8 * n * n.div_ceil(8) + n * n;
    if map.get("BlendTileData")?.payload[bitmap_start..bitmap_end]
        != source.get("BlendTileData")?.payload[bitmap_start..bitmap_end]
    {
        return Err(err("texture polishing changed gameplay bitmaps"));
    }
    let checked = BlendTiles::parse(map.get("BlendTileData")?, n, n)?;
    if checked.textures != bt.textures {
        return Err(err("texture polishing changed atlas IDs"));
    }
    crate::script_spawns::validate(p, map, &hm, &checked, &objects)?;
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    let report = json!({"source":source_name,"profile":PROFILE,"reference_version":"T2.2.9","style":"seamless stock grass, sparse soil, narrow shores and per-cell material blends",
        "excluded_patterned_grass":["Grass_CapeCod01","Grass_CapeCod03","Grass_CapeCod07","Grass_Golf01","Grass_Golf03","Grass_Golf05"],
        "changed_texture_cells":changed,"total_cells":n*n,
        "before":histogram(&before,&bt.textures),"after":histogram(&after,&bt.textures),
        "modified_chunks":["BlendTileData"],"height_and_ramps_unchanged":true,
        "all_gameplay_bitmaps_unchanged":true,"objects_and_scripts_unchanged":true,
        "atlas_and_resource_ids_unchanged":true,
        "blend_descriptions":checked.blend_descriptions.len()/18,
        "preview":"Representative colors of actual material assignments, not game screenshots or decoded texture pixels",
        "ingame_test":"not run"});
    println!(
        "纹理美化：{changed}/{} 格；地形、通行与建造位图、对象和脚本逐字节保持",
        n * n
    );
    Ok(Polished {
        terrain,
        before,
        after,
        names: bt.textures,
        impassable,
        build_block,
        report,
    })
}

// Colors illustrate material placement; the game uses the original atlas.
fn color(name: &str) -> [u8; 3] {
    match name {
        "Grass_NewYork01" => [111, 143, 66],
        "Grass_MtRush04" => [81, 119, 65],
        "Grass_CapeCod03" => [121, 146, 73],
        "Grass_Golf05" => [106, 127, 61],
        "Grass_CapeCod05" => [96, 112, 65],
        "Grass_Geneva03" | "Grass_Geneva01" => [67, 103, 62],
        "Grass_CapeCod09" => [128, 136, 78],
        "Grass_CapeCod12" => [145, 142, 99],
        "Dirt_Iceland05" => [139, 126, 101],
        "Dirt_Yucatan06" => [156, 124, 83],
        "Sand_Hawaii02" => [205, 188, 139],
        "RA3_ShallowSeaFloor" => [35, 73, 105],
        "Reef_Yucatan01" => [47, 100, 123],
        "Reef_Yucatan02" | "Reef_Havana02" => [60, 123, 137],
        _ if name.starts_with("Grass") => [109, 136, 70],
        _ if name.starts_with("Rock") => [120, 119, 104],
        _ if name.starts_with("Pavement") || name.starts_with("Steel") => [140, 143, 139],
        _ if name.contains("Gravel") => [148, 140, 113],
        _ => [144, 127, 93],
    }
}

impl Polished {
    fn image(&self, tiles: &[u16]) -> Image {
        let n = self.terrain.n;
        let mut im = Image::new(n, n, [0, 0, 0]);
        for i in 0..n * n {
            let mut rgb = color(&self.names[(tiles[i] / 64) as usize]).map(f64::from);
            // Show the encoded two-material edge using the same representative
            // colors on both sides; this is an illustrative half-cell fade.
            if let Some(j) = crate::build::neighbors(i, n).find(|&j| tiles[i] / 64 != tiles[j] / 64)
            {
                let adjacent = color(&self.names[(tiles[j] / 64) as usize]);
                for c in 0..3 {
                    rgb[c] = rgb[c] * 0.8 + adjacent[c] as f64 * 0.2;
                }
            }
            let x = i % n;
            let y = i / n;
            let gx = (self.terrain.z[y * n + (x + 1).min(n - 1)]
                - self.terrain.z[y * n + x.saturating_sub(1)])
                * 0.5;
            let gy = (self.terrain.z[(y + 1).min(n - 1) * n + x]
                - self.terrain.z[y.saturating_sub(1) * n + x])
                * 0.5;
            let light = (0.82
                + 0.27 * (-gx * 0.12 * -0.55 - gy * 0.12 * -0.55 + 0.63)
                    / (1. + (gx * gx + gy * gy) * 0.0144).sqrt())
            .clamp(0.62, 1.1);
            for (c, value) in rgb.into_iter().enumerate() {
                im.rgb[i * 3 + c] = (value * light).clamp(0., 255.) as u8;
            }
        }
        im
    }
    pub fn write_artifacts(&self, p: &Project) -> Result<()> {
        self.terrain.save(p)?;
        project::write_json(&p.assets.join("texture-report.json"), &self.report)?;
        let n = self.terrain.n;
        let surface = Surface {
            chosen: self.after.iter().map(|v| v / 64).collect(),
            impassable: self.impassable.clone(),
            build_block: self.build_block.clone(),
            layer_edge: vec![false; n * n],
            shore: vec![false; n * n],
            textures: vec![],
        };
        render::previews(&self.terrain, p, &surface)?;
        let index_path = p.assets.join("layout-index.json");
        let mut index: Value = serde_json::from_slice(&fs::read(&index_path)?)?;
        index["layout"] = json!("source/layout.json");
        index["source_map"] = json!(format!("{}.map", p.name));
        index["provenance"] = json!(
            "Actual objects and ramp definitions carried from the source release; only materials were repainted."
        );
        project::write_json(&index_path, &index)?;
        let before = self.image(&self.before);
        let after = self.image(&self.after);
        before.png(&p.assets.join("textures--before.png"))?;
        after.png(&p.assets.join("textures--after.png"))?;
        let font = render::font();
        let mut board = Image::new(1520, 870, [22, 29, 38]);
        board.text(
            font.as_ref(),
            20,
            10,
            &format!("{} · 纹理美化前后", p.name),
            24.,
            [255, 226, 120],
        );
        board.text(
            font.as_ref(),
            20,
            47,
            "左：原版     右：自然草地、土壤、岩壁与海岸过渡",
            18.,
            [210, 221, 233],
        );
        for (column, im) in [before, after].iter().enumerate() {
            let resized = im.resize(730, 730);
            for y in 0..730 {
                for x in 0..730 {
                    let i = (y * 730 + x) * 3;
                    board.pixel(
                        (20 + column * 750 + x) as isize,
                        (86 + y) as isize,
                        [resized.rgb[i], resized.rgb[i + 1], resized.rgb[i + 2]],
                    );
                }
            }
        }
        board.text(
            font.as_ref(),
            20,
            833,
            "按真实材质分布绘制的示意色；非游戏截图。地形、斜坡与玩法数据保持原样。",
            17.,
            [170, 186, 202],
        );
        board.png(&p.assets.join("textures--comparison.png"))?;
        // Carry forward the last ramp audit, labelled as historical evidence.
        let source = p
            .root
            .join("assets")
            .join(&p.group)
            .join(self.report["source"].as_str().unwrap());
        for name in ["ramp-report.json", "ramps--comparison.png"] {
            if source != p.assets && source.join(name).is_file() {
                fs::copy(source.join(name), p.assets.join(name))?;
            }
        }
        fs::write(
            p.assets.join("README.md"),
            format!(
                "# {}\n\n基于 {} 优化纹理，保留成品地形与斜坡。草地和土壤成片变化，收窄岸线与道路边缘，并写入逐格材质混合。仅使用原图已登记的纹理。\n\n[纹理前后对比](textures--comparison.png) · [美化后材质分布](textures--after.png) · [纹理核对](texture-report.json) · [全要素编号](preview--layout.png)\n\n对比使用真实材质编号和示意颜色，并非游戏截图；尚未进行实机测试。高度、全部通行与建造位图、对象、脚本及资源清单逐字节保持。若附有斜坡报告，它属于沿用的历史记录。\n",
                p.name,
                self.report["source"].as_str().unwrap()
            ),
        )?;
        Ok(())
    }
}
