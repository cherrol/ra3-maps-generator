//! Map previews, comparison boards and edit reports.
use super::*;

impl Polished {
    pub fn art_bytes(&self, p: &Project) -> Result<Vec<u8>> {
        let n = self.terrain.n;
        let surface = Surface {
            chosen: self.tiles.iter().map(|v| v / 64).collect(),
            impassable: self.imp.clone(),
            build_block: self.build.clone(),
            layer_edge: vec![false; n * n],
            shore: vec![false; n * n],
            textures: vec![],
        };
        render::art_image(&self.terrain, p, &surface).tga_bytes()
    }
    pub fn write_artifacts(&self, p: &Project) -> Result<()> {
        self.terrain.save(p)?;
        let mountains = self.report["operation"] == "add-mountains";
        project::write_json(
            &p.assets.join(if mountains {
                "mountain-report.json"
            } else {
                "ramp-report.json"
            }),
            &self.report,
        )?;
        let t = &self.terrain;
        let mut edge = vec![false; t.n * t.n];
        let mut shore = edge.clone();
        let water = p.number("WATER_Z", 200.);
        for i in 0..t.n * t.n {
            edge[i] = t.slope[i] > 20.;
            shore[i] =
                t.z[i] >= water && crate::terrain::grid::neighbors(i, t.n).any(|k| t.z[k] < water);
        }
        let surface = Surface {
            chosen: self.tiles.iter().map(|v| v / 64).collect(),
            impassable: self.imp.clone(),
            build_block: self.build.clone(),
            layer_edge: edge,
            shore,
            textures: vec![],
        };
        render::previews(t, p, &surface)?;
        {
            let source = p.release_assets(self.report["source"].as_str().unwrap())?;
            for name in [
                "production-spawns.json",
                "production-facing.json",
                "gift-clearance.json",
                "periodic-attack.json",
                "texture-report.json",
            ] {
                if source.join(name).is_file() {
                    fs::copy(source.join(name), p.assets.join(name))?;
                }
            }
        }
        let path = p.assets.join("layout-index.json");
        let mut index: Value = serde_json::from_slice(&fs::read(&path)?)?;
        index["layout"] = json!("source/layout.json");
        index["source_map"] = json!(format!("{}.map", p.name));
        index["provenance"] = if mountains {
            json!(
                "Four local mountain relief stamps; original objects, ramps, scripts and resource IDs preserved. See mountain-report.json."
            )
        } else if self.report["operation"] == "relocate-ramps" {
            json!(
                "Ramp centers authored at specified source preview positions. Added ramps and object moves are recorded in ramp-report.json; script contents were preserved."
            )
        } else {
            json!(
                "Objects decoded from the source release; polished ramp definitions are in source/layout.json. Original objects and scripts were preserved."
            )
        };
        project::write_json(&path, &index)?;
        if mountains {
            fs::write(
                p.assets.join("README.md"),
                format!(
                    "# {}\n\n基于 {}，在四处指定位置添加不规则岩石高山；山体全域禁止陆地通行和建造，保留其他通行位图，不增加禁飞区。\n\n山脚避开基地、矿盘、中立建筑、赠送建筑主备占地和原有坡道。对象、脚本及资源身份保持。\n\n[高山编辑记录](mountain-report.json) · [全要素预览](preview--layout.png)\n\n静态检查不能代替游戏内寻路实测。\n",
                    p.name,
                    self.report["source"].as_str().unwrap()
                ),
            )?;
            return Ok(());
        }
        self.comparison(p)?;
        if self.report["operation"] == "repair-shoulder" {
            fs::write(
                p.assets.join("README.md"),
                format!(
                    "# {}\n\n基于 {} 修整 {} 号坡口旁迁移遗留的方形高台，圆顺多层崖线，清理岩壁及碎石材质残边。坡心、方向、{} 格通行宽度保持，对象、矿位、脚本和资源身份逐字节保持。\n\n[前后对比](ramps--comparison.png) · [修整验收](ramp-report.json) · [坐标地图](coordinates/index.html)\n\n实际来源空间检查无新增或恶化问题。游戏内效果待复测。\n",
                    p.name,
                    self.report["source"].as_str().unwrap(),
                    self.report["ramps"][0]["preview_id"],
                    self.report["ramps"][0]["validation"]["width_cells"]
                ),
            )?;
            return Ok(());
        }
        if self.report["operation"] == "polish-ramp-surfaces" {
            fs::write(
                p.assets.join("README.md"),
                format!(
                    "# {}\n\n基于 {} 整理 {} 处坡口表面：去掉孤立的短土斑，坡面延续上下台地材质，清理旧混合边框。\n\n[逐坡记录](ramp-report.json) · [全要素预览](preview--layout.png)\n\n高度、坡形、全部通行/建造位图、对象、脚本和资源身份逐字节保持。\n",
                    p.name,
                    self.report["source"].as_str().unwrap(),
                    self.report["ramps"].as_array().unwrap().len()
                ),
            )?;
            return Ok(());
        }
        if self.report["operation"] == "relocate-ramps" {
            fs::write(
                p.assets.join("README.md"),
                format!(
                    "# {}\n\n基于 {} 按编号增设或移动斜坡，中心固定为来源标记坐标；移动操作恢复旧坡口，阻挡坡道的装饰及经授权的建筑、矿位挪到附近同层平地。\n\n[前后对比](ramps--comparison.png) · [全要素预览](preview--layout.png) · [编辑与验收](ramp-report.json)\n\n地形、纹理及通行位图只作局部更新，其他斜坡、资源身份、脚本与赠送朝向保留。小地图同步更新。\n",
                    p.name,
                    self.report["source"].as_str().unwrap()
                ),
            )?;
            return Ok(());
        }
        fs::write(
            p.assets.join("README.md"),
            format!(
                "# {}\n\n基于 {} 作局部斜坡优化。\n\n[斜坡前后对比](ramps--comparison.png) · [全要素编号](preview--layout.png) · [编号索引](layout-index.json) · [逐坡验收](ramp-report.json)\n\n调整地形 {} 条，其余 {} 条保留原地形并整理纹理。对象、脚本、资源资产与坡外高度保持原样；保留原有陆地/水域出生点与矿位连通关系。尚未进行游戏内实测。\n\n后续增量优化请保留本目录的 `source/layout.json`，以本版 ZIP 继续执行 `polish-ramps`。\n",
                p.name,
                self.report["source"].as_str().unwrap_or(""),
                self.report["optimized"],
                self.report["retained"]
            ),
        )?;
        Ok(())
    }
    fn comparison(&self, p: &Project) -> Result<()> {
        self.comparison_item(p, self.example, "ramps--comparison.png")?;
        if self.report["operation"] == "relocate-ramps" {
            for i in 0..self.report["ramps"].as_array().unwrap().len() {
                self.comparison_item(
                    p,
                    i,
                    &format!(
                        "ramps-{}--comparison.png",
                        self.report["ramps"][i]["preview_id"]
                    ),
                )?;
            }
        }
        Ok(())
    }
    fn comparison_item(&self, p: &Project, example: usize, filename: &str) -> Result<()> {
        let old = &self.report["ramps"][example]["before"];
        let new = &self.report["ramps"][example]["after"];
        let font = render::font();
        let mut board = Image::new(1040, 590, [22, 29, 38]);
        board.text(
            font.as_ref(),
            20,
            8,
            &format!(
                "{} · 斜坡编号 {}",
                p.name, self.report["ramps"][example]["preview_id"]
            ),
            24.,
            [255, 226, 120],
        );
        board.text(
            font.as_ref(),
            20,
            48,
            "左：优化前    右：优化后（高度与纹理类别示意，非游戏截图）",
            18.,
            [215, 220, 230],
        );
        let relocation = self.report["operation"] == "relocate-ramps";
        let cx = if relocation {
            (number(old, "x", 0.) + number(new, "x", 0.)) * 0.5
        } else {
            number(old, "x", 0.)
        };
        let cy = if relocation {
            (number(old, "y", 0.) + number(new, "y", 0.)) * 0.5
        } else {
            number(old, "y", 0.)
        };
        let extent = if relocation {
            88. + (number(old, "x", 0.) - number(new, "x", 0.))
                .abs()
                .max((number(old, "y", 0.) - number(new, "y", 0.)).abs())
        } else {
            78.
        };
        let n = self.terrain.n;
        for (panel, z, tiles) in [
            (0, &self.before, &self.before_tiles),
            (1, &self.terrain.z, &self.tiles),
        ] {
            for py in 0..500 {
                for px in 0..500 {
                    let x = cx + (px as f64 / 499. - 0.5) * extent;
                    let y = cy + (py as f64 / 499. - 0.5) * extent;
                    let ix = x.round().clamp(0., (n - 1) as f64) as usize;
                    let iy = y.round().clamp(0., (n - 1) as f64) as usize;
                    let h = sample(z, n, x, y);
                    let gx = (sample(z, n, x + 1., y) - sample(z, n, x - 1., y)) * 0.5;
                    let gy = (sample(z, n, x, y + 1.) - sample(z, n, x, y - 1.)) * 0.5;
                    let name = &self.textures[tiles[iy * n + ix] as usize / 64];
                    let mut color = if name.contains("Dirt_Yucatan06") {
                        [151., 119., 72.]
                    } else if name.contains("Gravel") {
                        [135., 133., 111.]
                    } else if name.contains("Rock") {
                        [115., 112., 101.]
                    } else if h < p.number("WATER_Z", 200.) {
                        [39., 100., 157.]
                    } else {
                        [110., 145., 65.]
                    };
                    let light = 0.58
                        + ((-gx * 0.18 * (-0.55) - gy * 0.18 * (-0.55) + 0.63)
                            / (1. + (gx * gx + gy * gy) * 0.0324).sqrt())
                        .clamp(-0.25, 1.)
                            * 0.53;
                    for c in &mut color {
                        *c = (*c * light).clamp(0., 255.);
                    }
                    board.pixel(
                        (20 + panel * 510 + px) as isize,
                        (85 + py) as isize,
                        color.map(|c| c as u8),
                    );
                }
            }
        }
        board.png(&p.assets.join(filename))
    }
}
