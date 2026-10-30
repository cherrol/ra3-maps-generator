//! Repaint a finished map using its registered atlas, without changing gameplay.
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile},
    pipeline::project::{self, Project},
    render::{self, Image},
    terrain::surface::{self, Surface},
    terrain::{self, Layout, ObjectSpec, Terrain, number},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

/// Approved five-layer palette, retaining the established natural placement rules.
pub const PROFILE: &str = "classic-natural";

pub fn enabled(p: &Project) -> Result<bool> {
    match p.string("TEXTURE_PROFILE", PROFILE) {
        PROFILE => Ok(true),
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
    source_chunks: BTreeMap<String, (u16, usize, u32)>,
    pub report: Value,
}

fn chunk_fingerprints(map: &MapFile) -> BTreeMap<String, (u16, usize, u32)> {
    map.chunks
        .iter()
        .map(|chunk| {
            (
                map.table[&chunk.name_idx].clone(),
                (
                    chunk.version,
                    chunk.payload.len(),
                    crc32fast::hash(&chunk.payload),
                ),
            )
        })
        .collect()
}

fn pick(names: &[String], candidates: &[&str], fallback: u16) -> u16 {
    candidates
        .iter()
        .find_map(|name| names.iter().position(|n| n == name))
        .map_or(fallback, |i| i as u16)
}

fn paint(t: &Terrain, p: &Project, names: &[String], before: &[u16]) -> Vec<u16> {
    let n = t.n;
    let h = t.height();
    let water = p.number("WATER_Z", 200.);
    let wet: Vec<_> = t.z.iter().map(|h| *h < water).collect();
    let to_water = terrain::distance(&wet, n);
    let to_land = terrain::distance(&wet.iter().map(|v| !v).collect::<Vec<_>>(), n);
    let broad = surface::noise_rect(n, h, 52., 2771);
    let detail = surface::noise_rect(n, h, 17., 9323);
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
    let primary = crate::terrain::materials::PRIMARY.map(|name| {
        names
            .iter()
            .position(|n| n == name)
            .expect("registered primary texture") as u16
    });
    // Keep the approved lower-layer variants; mountain layers use their selected
    // rock throughout the interior, without unrelated soil/grass noise patches.
    let palettes = [
        [
            primary[0],
            pick(names, &["Grass_CapeCod05", "Grass_NewYork01"], grass),
        ],
        [
            primary[1],
            pick(names, &["Grass_Geneva01", "Grass_MtRush04"], grass),
        ],
        [
            primary[2],
            pick(names, &["Dirt_Iceland05", "Dirt_Yucatan03"], grass),
        ],
        [primary[3]; 2],
        [primary[4]; 2],
    ];
    let dirt = pick(names, &["Dirt_Yucatan06", "Dirt_Yucatan03"], grass);
    let gravel = pick(names, &["Gravel_Yucatan01", "BB_Gravel01"], dirt);
    let rock = pick(names, &["Rock_Yucatan01", "Rock_Yucatan02"], gravel);
    let sand = pick(names, &["Sand_Hawaii02"], dirt);
    let city = pick(names, &["BB_Gravel01", "Gravel_Yucatan01"], gravel);
    let sea = pick(names, &["RA3_ShallowSeaFloor"], fallback);
    let reef = pick(names, &["Reef_Yucatan01", "RA3_ShallowSeaFloor"], sea);
    let shallows = pick(names, &["Reef_Yucatan02", "Reef_Havana02"], reef);
    let levels = crate::terrain::materials::levels(p);
    let peak = water + p.number("PEAK_IMPASSABLE", 235.);
    let cliff: Vec<_> = (0..n * h)
        .map(|i| {
            !wet[i]
                && crate::terrain::grid::neighbors_rect(i, n, h)
                    .any(|j| !wet[j] && (t.z[j] - t.z[i]).abs() >= 35.)
        })
        .collect();
    let to_cliff = terrain::distance(&cliff, n);
    let mut out = vec![0; n * h];
    for i in 0..n * h {
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
        let layer = crate::terrain::materials::layer(t.z[i], &levels, peak);
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
        if ramp["surface_style"] == "mountain" {
            continue;
        }
        let Ok((ux, uy, lo, hi)) = terrain::ramp_axes(ramp) else {
            continue;
        };
        let length = (number(ramp, "span", 25.) * 0.5).clamp(10., 20.);
        let width = number(ramp, "width", 25.) * 0.5;
        let cx = number(ramp, "x", 0.);
        let cy = number(ramp, "y", 0.);
        let radius = (length + width * 1.25 + 6.).ceil() as isize;
        for y in (cy as isize - radius).max(0)..=(cy as isize + radius).min(h as isize - 1) {
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
    if t.layout
        .ramps
        .iter()
        .any(|r| r["surface_style"] == "mountain")
    {
        let mut tiles: Vec<_> = out
            .iter()
            .zip(before)
            .map(|(family, original)| family * 64 + original % 64)
            .collect();
        let mut changed = vec![false; n * h];
        for ramp in &t.layout.ramps {
            if ramp["surface_style"] == "mountain" {
                crate::terrain::ramps::paint_mountain(
                    &mut tiles,
                    names,
                    &t.z,
                    n,
                    (ramp, ramp),
                    water,
                    &mut changed,
                );
            }
        }
        out = tiles.iter().map(|tile| tile / 64).collect();
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
    enabled(p)?;
    let source = map.clone();
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    let h = hm.height;
    if h != p.usize("GRID_HEIGHT", p.usize("GRID", 0))
        || n != p.usize("GRID", 0)
        || map.get("BlendTileData")?.version != 27
    {
        return Err(err("texture polishing requires matching v27 terrain"));
    }
    let old_textures = BlendTiles::parse(map.get("BlendTileData")?, n, h)?.textures;
    let added = crate::resources::terrain_package::ensure_materials(
        map,
        &crate::terrain::materials::STOCK.map(|row| row[0]),
    )?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, h)?;
    let before = terrain::flip(&bt.tiles, n);
    let mut layout = Layout::load_release(p, source_name)?;
    if crate::terrain::ramps::enabled(p)? {
        for ramp in &mut layout.ramps {
            ramp["surface_style"] = json!(crate::terrain::ramps::PROFILE);
        }
    }
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let rules = crate::gameplay::scripts::load_rules(p)?;
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
                h as f64
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
    layout.sync_imported_starts(p, source_name, map)?;
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
    crate::terrain::ramps::patch_blend(
        map,
        &bt,
        &after,
        &impassable,
        &build_block,
        &vec![true; n * h],
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
    let bitmap_start = 4 + 8 * n * h;
    let bitmap_end = bitmap_start + 8 * h * n.div_ceil(8) + n * h;
    if map.get("BlendTileData")?.payload[bitmap_start..bitmap_end]
        != source.get("BlendTileData")?.payload[bitmap_start..bitmap_end]
    {
        return Err(err("texture polishing changed gameplay bitmaps"));
    }
    let checked = BlendTiles::parse(map.get("BlendTileData")?, n, h)?;
    if checked.textures != bt.textures || !checked.textures.starts_with(&old_textures) {
        return Err(err("texture polishing changed atlas IDs"));
    }
    crate::gameplay::script_spawns::validate_with_layout(
        p,
        map,
        &hm,
        &checked,
        &objects,
        &terrain.layout,
    )?;
    let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    let report = json!({"source":source_name,"profile":PROFILE,"reference_version":"T2.2.9",
        "scope":"texture-polish-before-terrain-package-finalization",
        "preservation_scope":"Preservation flags and modified_chunks compare only the input and output of texture_polish::apply; final package changes are reported separately in finalization.",
        "palette_revision":crate::terrain::materials::REVISION,
        "layer_primary_textures":crate::terrain::materials::PRIMARY,
        "added_texture_families":added,
        "style":"three grass layers, two rock layers, narrow shores and per-cell material blends",
        "excluded_patterned_grass":["Grass_CapeCod01","Grass_CapeCod03","Grass_CapeCod07","Grass_Golf01","Grass_Golf03","Grass_Golf05"],
        "changed_texture_cells":changed,"total_cells":n*h,
        "before":histogram(&before,&bt.textures),"after":histogram(&after,&bt.textures),
        "modified_chunks":["BlendTileData"],"height_and_ramps_unchanged":true,
        "all_gameplay_bitmaps_unchanged":true,"objects_and_scripts_unchanged":true,
        "existing_atlas_and_resource_ids_unchanged":true,
        "blend_descriptions":checked.blend_descriptions.len()/18,
        "preview":"Representative colors of actual material assignments, not game screenshots or decoded texture pixels",
        "ingame_test":"not run"});
    println!(
        "纹理美化：{changed}/{} 格；地形、通行与建造位图、对象和脚本逐字节保持",
        n * h
    );
    Ok(Polished {
        terrain,
        before,
        after,
        names: bt.textures,
        impassable,
        build_block,
        source_chunks: chunk_fingerprints(&source),
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
        "Rock_Yucatan03" => [162, 128, 103],
        "Rock_Iceland02" => [218, 221, 211],
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
    /// Bind the polish-stage report to the actual map after package cleanup.
    /// Cleanup can remap texture families and update chunks outside BlendTileData.
    pub fn record_finalization(
        &mut self,
        map: &MapFile,
        compaction: Option<&crate::resources::terrain_package::Report>,
    ) -> Result<()> {
        let hm = HeightMap::parse(map.get("HeightMapData")?)?;
        let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
        let final_chunks = chunk_fingerprints(map);
        let changed_chunks = self
            .source_chunks
            .keys()
            .chain(final_chunks.keys())
            .filter(|name| self.source_chunks.get(*name) != final_chunks.get(*name))
            .collect::<std::collections::BTreeSet<_>>();
        let mapping = self
            .names
            .iter()
            .enumerate()
            .map(|(id, name)| {
                json!({"name":name,"polish_family_id":id,
                    "final_family_id":bt.textures.iter().position(|final_name| final_name == name)})
            })
            .collect::<Vec<_>>();
        self.report["finalization"] = json!({
            "scope":"native map after terrain-package finalization",
            "modified_chunks_since_polish_input":changed_chunks,
            "texture_table_unchanged":self.names == bt.textures,
            "texture_id_mapping":mapping,
            "texture_families":bt.textures,
            "after":histogram(&bt.tiles, &bt.textures),
            "compaction":compaction,
            "runtime_resource_ids":"Package finalization owns map and atlas version IDs; the polish-stage preservation flag does not cover them."
        });
        Ok(())
    }

    fn image(&self, tiles: &[u16]) -> Image {
        let n = self.terrain.n;
        let h = self.terrain.height();
        let mut im = Image::new(n, h, [0, 0, 0]);
        for i in 0..n * h {
            let mut rgb = color(&self.names[(tiles[i] / 64) as usize]).map(f64::from);
            // Show the encoded two-material edge using the same representative
            // colors on both sides; this is an illustrative half-cell fade.
            if let Some(j) = crate::terrain::grid::neighbors_rect(i, n, h)
                .find(|&j| tiles[i] / 64 != tiles[j] / 64)
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
            let gy = (self.terrain.z[(y + 1).min(h - 1) * n + x]
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
        self.write_artifacts_inner(p, true)
    }

    /// A full build renders its ordinary previews in the subsequent preview stage.
    /// Keep all other polish artifacts, including the actual source snapshot.
    pub fn write_artifacts_without_previews(&self, p: &Project) -> Result<()> {
        self.write_artifacts_inner(p, false)
    }

    fn write_artifacts_inner(&self, p: &Project, previews: bool) -> Result<()> {
        self.terrain.save(p)?;
        project::write_json(&p.assets.join("texture-report.json"), &self.report)?;
        if previews {
            let n = self.terrain.n;
            let h = self.terrain.height();
            let surface = Surface {
                chosen: self.after.iter().map(|v| v / 64).collect(),
                impassable: self.impassable.clone(),
                build_block: self.build_block.clone(),
                layer_edge: vec![false; n * h],
                shore: vec![false; n * h],
                textures: vec![],
            };
            render::previews(&self.terrain, p, &surface)?;
        }
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
        let source = p.release_assets(self.report["source"].as_str().unwrap())?;
        for name in ["ramp-report.json", "ramps--comparison.png"] {
            if source != p.assets && source.join(name).is_file() {
                fs::copy(source.join(name), p.assets.join(name))?;
            }
        }
        fs::write(
            p.assets.join("README.md"),
            format!(
                "# {}\n\n基于 {} 应用公共五层主纹理：前三层草地、第四层棕岩、第五层浅灰岩，保留成品地形与斜坡。前三层保留已确认的少量变体；先追加缺失的公共原生材质，再由封装收尾清理未使用项并同步编号。\n\n[纹理前后对比](textures--comparison.png) · [美化后材质分布](textures--after.png) · [纹理核对](texture-report.json) · [全要素编号](preview--layout.png)\n\n对比使用真实材质分配和示意颜色，并非游戏截图；尚未进行实机测试。高度、全部通行与建造位图、对象及脚本保持；地形材质编号、加载清单和地图／图集版本由公共封装同步。报告的保留结论限定纹理绘制阶段，收尾结果单列。若附有斜坡报告，它属于沿用的历史记录。\n",
                p.name,
                self.report["source"].as_str().unwrap()
            ),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::materials::{PRIMARY, STOCK};
    use std::path::Path;

    struct Fixture(Project);

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    fn fixture() -> (Fixture, Polished, MapFile) {
        use crate::formats::map::Chunk;
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ra3-texture-artifacts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let p = Project {
            assets: root.join("assets/texture-fixture/texture-fixtureT0.0.1"),
            dist: root.join("dist/texture-fixture/texture-fixtureT0.0.1"),
            root,
            id: "texture-fixture".into(),
            cfg: json!({"GRID":16,"BORDER":1,"CELL":10.,"Z_SCALE":1.}),
            state: json!({"stage":"T","major":0,"minor":0,"patch":1}),
            version: "T0.0.1".into(),
            group: "texture-fixture".into(),
            name: "texture-fixtureT0.0.1".into(),
        };
        project::write_json(&p.map_dir().join("version.json"), &p.state).unwrap();
        let terrain = Terrain {
            n: 16,
            raw: vec![210; 256],
            land: vec![true; 256],
            z: vec![210.; 256],
            slope: vec![0.; 256],
            layout: serde_json::from_value(json!({
                "schema_version":1,"base_cells":[],"ore_cells":[],
                "ore_angles":[],"objects":[]
            }))
            .unwrap(),
        };
        let names = vec!["Unused".into(), "Grass_NewYork01".into()];
        let surface = Surface {
            chosen: vec![1; 256],
            impassable: vec![false; 256],
            build_block: vec![false; 256],
            layer_edge: vec![false; 256],
            shore: vec![false; 256],
            textures: names
                .iter()
                .map(|name: &String| [name.clone(), String::new(), String::new()])
                .collect(),
        };
        let map = MapFile {
            order: vec![],
            table: [(1, "HeightMapData"), (2, "BlendTileData"), (3, "WorldInfo")]
                .map(|(id, name)| (id, name.into()))
                .into(),
            chunks: vec![
                Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: terrain.heightmap(&p).encode().unwrap(),
                },
                Chunk {
                    name_idx: 2,
                    version: 27,
                    payload: surface.encode(16, &[]).unwrap(),
                },
                Chunk {
                    name_idx: 3,
                    version: 1,
                    payload: vec![],
                },
            ],
        };
        let polished = Polished {
            terrain,
            before: vec![64; 256],
            after: vec![64; 256],
            names,
            impassable: surface.impassable,
            build_block: surface.build_block,
            source_chunks: chunk_fingerprints(&map),
            report: json!({"source":p.name,"modified_chunks":["BlendTileData"],
                "existing_atlas_and_resource_ids_unchanged":true}),
        };
        (Fixture(p), polished, map)
    }

    #[test]
    fn texture_profile_rejects_the_removed_legacy_switch() {
        let (mut fixture, _, mut map) = fixture();
        fixture.0.cfg["TEXTURE_PROFILE"] = json!("legacy");
        assert!(enabled(&fixture.0).is_err());
        let before = chunk_fingerprints(&map);
        let rejected = apply(&fixture.0, &mut map, "invalid-profile")
            .err()
            .unwrap();
        assert!(rejected.to_string().contains("TEXTURE_PROFILE"));
        assert_eq!(chunk_fingerprints(&map), before);
        fixture.0.cfg["TEXTURE_PROFILE"] = json!(PROFILE);
        assert!(enabled(&fixture.0).unwrap());
    }

    #[test]
    fn finalized_report_separates_polish_claims_from_compaction_and_loading_changes() {
        let (_fixture, mut polished, mut map) = fixture();
        let compaction = crate::resources::terrain_package::compact(&mut map).unwrap();
        map.get_mut("WorldInfo").unwrap().payload = vec![1, 2, 3];
        polished
            .record_finalization(&map, Some(&compaction))
            .unwrap();
        let report = &polished.report;
        assert_eq!(report["modified_chunks"], json!(["BlendTileData"]));
        assert_eq!(report["existing_atlas_and_resource_ids_unchanged"], true);
        let finalization = &report["finalization"];
        assert_eq!(
            finalization["modified_chunks_since_polish_input"],
            json!(["BlendTileData", "WorldInfo"])
        );
        assert_eq!(finalization["texture_table_unchanged"], false);
        assert_eq!(finalization["compaction"]["removed_textures"], 1);
        assert_eq!(
            finalization["texture_id_mapping"][0]["final_family_id"],
            Value::Null
        );
        assert_eq!(finalization["texture_id_mapping"][1]["final_family_id"], 0);
        assert_eq!(finalization["texture_families"], json!(["Grass_NewYork01"]));
        assert_eq!(finalization["after"]["Grass_NewYork01"], 256);
    }

    #[test]
    fn deferred_preview_preserves_existing_outputs_and_standalone_still_writes_index() {
        let (fixture, polished, _map) = fixture();
        let p = &fixture.0;
        fs::create_dir_all(&p.assets).unwrap();
        fs::write(p.assets.join("preview.png"), b"previous preview").unwrap();
        polished.write_artifacts_without_previews(p).unwrap();
        assert_eq!(
            fs::read(p.assets.join("preview.png")).unwrap(),
            b"previous preview"
        );
        assert!(!p.assets.join("layout-index.json").exists());
        for file in [
            "texture-report.json",
            "textures--after.png",
            "textures--comparison.png",
            "source/layout.json",
            "source/terrain.npz",
        ] {
            assert!(p.assets.join(file).is_file(), "missing {file}");
        }

        polished.write_artifacts(p).unwrap();
        assert_ne!(
            fs::read(p.assets.join("preview.png")).unwrap(),
            b"previous preview"
        );
        let index: Value =
            serde_json::from_slice(&fs::read(p.assets.join("layout-index.json")).unwrap()).unwrap();
        assert_eq!(index["source_map"], format!("{}.map", p.name));
        assert!(
            index["provenance"]
                .as_str()
                .unwrap()
                .contains("source/layout.json")
        );
        assert!(
            !index["provenance"]
                .as_str()
                .unwrap()
                .contains("only materials")
        );
    }

    #[test]
    fn five_plateaus_keep_grass_variants_and_two_distinct_rock_interiors() {
        let mut p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        p.cfg["LAYER_M"] = json!([
            [4000, 700.],
            [1500, 420.],
            [500, 350.],
            [200, 280.],
            [0, 210.]
        ]);
        let n = 320;
        let z: Vec<_> = (0..n * n)
            .map(|i| [210., 280., 350., 420., 700.][i / n / 64])
            .collect();
        let t = Terrain {
            n,
            raw: vec![0; n * n],
            slope: terrain::gradient(&z, n),
            land: vec![true; n * n],
            z,
            layout: serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[]})).unwrap(),
        };
        let names: Vec<_> = STOCK.iter().map(|r| r[0].to_owned()).collect();
        let before = vec![0; n * n];
        let variants = [
            "Grass_CapeCod05",
            "Grass_Geneva01",
            "Dirt_Iceland05",
            PRIMARY[3],
            PRIMARY[4],
        ];
        for configured_peak in [true, false] {
            if !configured_peak {
                p.cfg["LAYER_M"] = json!([[0, 210.], [200, 280.], [500, 350.], [1500, 420.]]);
            }
            let out = paint(&t, &p, &names, &before);
            for layer in 0..5 {
                let mut primary_count = 0;
                let mut count = 0;
                for y in layer * 64 + 12..(layer + 1) * 64 - 12 {
                    for x in 12..n - 12 {
                        let name = names[out[y * n + x] as usize].as_str();
                        assert!(
                            [PRIMARY[layer], variants[layer]].contains(&name),
                            "layer {layer}: {name}"
                        );
                        primary_count += usize::from(name == PRIMARY[layer]);
                        count += 1;
                    }
                }
                assert!(
                    primary_count > count / 2,
                    "primary must dominate layer {layer}"
                );
                if layer >= 3 {
                    assert_eq!(
                        primary_count, count,
                        "rock interiors must not get grass/soil patches"
                    );
                }
            }
        }
    }
}
