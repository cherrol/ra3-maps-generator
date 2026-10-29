use crate::{Result, pipeline::project::Project, terrain::Terrain, terrain::surface::Surface};
use std::fs;
mod image;
mod layout;
pub use image::Image;
pub(crate) fn font() -> Option<fontdue::Font> {
    let paths = std::env::var("RA3MAP_FONT").ok().into_iter().chain(
        [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/simhei.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
        .into_iter()
        .map(str::to_string),
    );
    for path in paths {
        if let Ok(data) = fs::read(path)
            && let Ok(font) = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
        {
            return Some(font);
        }
    }
    None
}
pub fn terrain_image(t: &Terrain, p: &Project, surface: Option<&Surface>) -> Image {
    let mut im = Image::new(t.n, t.height(), [0, 0, 0]);
    let water = p.number("WATER_Z", 200.);
    for (i, z) in t.z.iter().enumerate() {
        let mut c = if *z < water - 40. {
            [22, 62, 122]
        } else if *z < water {
            [46, 104, 172]
        } else if *z < water + 4. {
            [216, 204, 152]
        } else {
            let f = ((z - water - 4.) / 330.).clamp(0., 1.).powf(0.85);
            if f < 0.42 {
                let g = f / 0.42;
                [
                    (58. + 52. * g) as u8,
                    (122. + 44. * g) as u8,
                    (58. - 12. * g) as u8,
                ]
            } else if f < 0.66 {
                let g = (f - 0.42) / 0.24;
                [
                    (110. + 50. * g) as u8,
                    (166. - 16. * g) as u8,
                    (46. - 18. * g) as u8,
                ]
            } else if f < 0.85 {
                let g = (f - 0.66) / 0.19;
                [
                    (160. + 30. * g) as u8,
                    (150. - 20. * g) as u8,
                    (28. + 22. * g) as u8,
                ]
            } else {
                let g = (f - 0.85) / 0.15;
                [
                    (190. + 55. * g) as u8,
                    (130. + 80. * g) as u8,
                    (50. + 130. * g) as u8,
                ]
            }
        };
        if let Some(s) = surface {
            if s.layer_edge[i] {
                c = [128, 120, 108];
            }
            if s.shore[i] {
                c = [214, 198, 142];
            }
        }
        if !t.land[i] && (!p.terrain_flag("art_keep_sea") || *z >= water) {
            c = c.map(|v| (v as f64 * 0.72) as u8);
        }
        im.rgb[i * 3..i * 3 + 3].copy_from_slice(&c);
    }
    im
}
pub fn art_image(t: &Terrain, p: &Project, surface: &Surface) -> Image {
    let mut im = terrain_image(t, p, Some(surface)).resize(512, 512);
    for o in &t.layout.objects {
        if o.type_name.contains("Tree")
            || o.type_name.contains("Bush")
            || o.type_name.contains("GardenRock")
        {
            let x = (o.cell[0] * 512. / t.n as f64).round_ties_even() as isize;
            let y = (o.cell[1] * 512. / t.height() as f64).round_ties_even() as isize;
            let c = if o.type_name.contains("Rock") {
                [168, 164, 156]
            } else {
                [34, 110, 46]
            };
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    im.pixel(xx, yy, c);
                }
            }
        }
    }
    im
}
pub fn art(t: &Terrain, p: &Project, surface: &Surface) -> Result<()> {
    let im = art_image(t, p, surface);
    im.tga(&p.assets.join(format!("{}_art.tga", p.name)))?;
    im.png(&p.source().join("art_preview.png"))
}
pub fn previews(t: &Terrain, p: &Project, surface: &Surface) -> Result<()> {
    let font = font();
    let base = terrain_image(t, p, Some(surface));
    base.png(&p.source().join("terrain_preview.png"))?;
    let mut walk = base.resize(t.n, t.height());
    for (i, v) in surface.impassable.iter().enumerate() {
        if *v {
            for c in 0..3 {
                walk.rgb[i * 3 + c] = (walk.rgb[i * 3 + c] as f64 * 0.45) as u8;
            }
        }
    }
    walk.png(&p.source().join("passability_preview.png"))?;
    walk.png(&p.source().join("build_view.png"))?;
    let side = t.n * 2;
    let rows = t.height() * 2;
    let top = 120;
    let bottom = 85;
    let mut preview = Image::new(side, rows + top + bottom, [22, 29, 38]);
    let large = base.resize(side, rows);
    for y in 0..rows {
        let i = ((y + top) * side) * 3;
        preview.rgb[i..i + side * 3].copy_from_slice(&large.rgb[y * side * 3..(y + 1) * side * 3]);
    }
    preview.text(
        font.as_ref(),
        24,
        14,
        &p.render(p.string("PREVIEW_TITLE", "{name}")),
        36.,
        [255, 226, 120],
    );
    preview.text(
        font.as_ref(),
        24,
        65,
        p.string("PREVIEW_SUBTITLE", ""),
        24.,
        [210, 220, 235],
    );
    for rs in &t.layout.ramps {
        let x = (number(rs, "x", 0.) * 2.).round() as isize;
        let y = (number(rs, "y", 0.) * 2.).round() as isize + top as isize;
        preview.circle(x, y, 10, [255, 55, 55]);
    }
    for (i, c) in t.layout.base_cells.iter().enumerate() {
        let x = (c[0] * 2.).round() as isize;
        let y = (c[1] * 2.).round() as isize + top as isize;
        preview.circle(
            x,
            y,
            22,
            if i < 4 {
                [80, 185, 255]
            } else {
                [255, 100, 80]
            },
        );
        preview.text(
            font.as_ref(),
            x - 9,
            y - 18,
            &(i + 1).to_string(),
            28.,
            [255, 255, 255],
        );
    }
    for c in &t.layout.ore_cells {
        preview.circle(
            (c[0] * 2.).round() as isize,
            (c[1] * 2.).round() as isize + top as isize,
            5,
            [255, 215, 0],
        );
    }
    preview.text(
        font.as_ref(),
        24,
        (rows + top + 18) as isize,
        &p.render(p.string("PREVIEW_FOOTER", "北朝上 · 红圈为斜坡 · 金色为矿脉")),
        21.,
        [180, 195, 215],
    );
    preview.png(&p.assets.join("preview.png"))?;
    preview.png(&p.assets.join("preview--line.png"))?;
    layout::write(&base, t, p, font.as_ref())?;
    Ok(())
}
use crate::terrain::number;
