use crate::{Result, err, project::Project, surface::Surface, terrain::Terrain};
use flate2::{Compression, write::ZlibEncoder};
use std::{fs, io::Write, path::Path};
mod layout;
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}
impl Image {
    pub fn new(width: usize, height: usize, color: [u8; 3]) -> Self {
        Self {
            width,
            height,
            rgb: color.repeat(width * height),
        }
    }
    pub fn pixel(&mut self, x: isize, y: isize, color: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let i = (y as usize * self.width + x as usize) * 3;
            self.rgb[i..i + 3].copy_from_slice(&color);
        }
    }
    pub fn circle(&mut self, x: isize, y: isize, r: isize, color: [u8; 3]) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r && dx * dx + dy * dy >= (r - 2) * (r - 2) {
                    self.pixel(x + dx, y + dy, color);
                }
            }
        }
    }
    pub fn resize(&self, width: usize, height: usize) -> Self {
        let mut out = Self::new(width, height, [0, 0, 0]);
        for y in 0..height {
            for x in 0..width {
                let fy = ((y as f64 + 0.5) * self.height as f64 / height as f64 - 0.5)
                    .clamp(0., (self.height - 1) as f64);
                let fx = ((x as f64 + 0.5) * self.width as f64 / width as f64 - 0.5)
                    .clamp(0., (self.width - 1) as f64);
                let x0 = fx as usize;
                let y0 = fy as usize;
                let ax = fx - x0 as f64;
                let ay = fy - y0 as f64;
                let x1 = (x0 + 1).min(self.width - 1);
                let y1 = (y0 + 1).min(self.height - 1);
                for c in 0..3 {
                    out.rgb[(y * width + x) * 3 + c] =
                        ((self.rgb[(y0 * self.width + x0) * 3 + c] as f64 * (1. - ax)
                            + self.rgb[(y0 * self.width + x1) * 3 + c] as f64 * ax)
                            * (1. - ay)
                            + (self.rgb[(y1 * self.width + x0) * 3 + c] as f64 * (1. - ax)
                                + self.rgb[(y1 * self.width + x1) * 3 + c] as f64 * ax)
                                * ay)
                            .round() as u8;
                }
            }
        }
        out
    }
    pub fn png(&self, path: &Path) -> Result<()> {
        let mut raw = Vec::with_capacity(self.rgb.len() + self.height);
        for row in self.rgb.chunks(self.width * 3) {
            raw.push(0);
            raw.extend(row);
        }
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&raw)?;
        let comp = enc.finish()?;
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&(self.width as u32).to_be_bytes());
        ihdr.extend_from_slice(&(self.height as u32).to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        png_chunk(&mut out, b"IHDR", &ihdr);
        png_chunk(&mut out, b"IDAT", &comp);
        png_chunk(&mut out, b"IEND", &[]);
        fs::write(path, out)?;
        Ok(())
    }
    pub fn tga(&self, path: &Path) -> Result<()> {
        fs::write(path, self.tga_bytes()?)?;
        Ok(())
    }
    pub fn tga_bytes(&self) -> Result<Vec<u8>> {
        let mut out = vec![0; 18];
        out[2] = 2;
        out[12..14].copy_from_slice(&u16::try_from(self.width)?.to_le_bytes());
        out[14..16].copy_from_slice(&u16::try_from(self.height)?.to_le_bytes());
        out[16] = 24;
        for row in self.rgb.chunks(self.width * 3).rev() {
            for c in row.chunks(3) {
                out.extend_from_slice(&[c[2], c[1], c[0]]);
            }
        }
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(b"TRUEVISION-XFILE.\0");
        Ok(out)
    }
    pub fn load_tga(path: &Path) -> Result<Self> {
        let b = fs::read(path)?;
        if b.len() < 18 || b[2] != 2 || b[16] != 24 {
            return Err(err("expected 24-bit uncompressed TGA"));
        }
        let width = u16::from_le_bytes([b[12], b[13]]) as usize;
        let height = u16::from_le_bytes([b[14], b[15]]) as usize;
        if width == 0 || height == 0 {
            return Err(err("empty TGA"));
        }
        let offset = 18 + b[0] as usize;
        let pixels = b
            .get(offset..offset + width * height * 3)
            .ok_or_else(|| err("truncated TGA"))?;
        let mut out = Self::new(width, height, [0, 0, 0]);
        for y in 0..height {
            let sy = if b[17] & 0x20 != 0 { y } else { height - 1 - y };
            for x in 0..width {
                let i = (sy * width + x) * 3;
                let j = (y * width + x) * 3;
                out.rgb[j..j + 3].copy_from_slice(&[pixels[i + 2], pixels[i + 1], pixels[i]]);
            }
        }
        Ok(out)
    }
    pub fn text(
        &mut self,
        font: Option<&fontdue::Font>,
        x: isize,
        y: isize,
        text: &str,
        size: f32,
        color: [u8; 3],
    ) {
        let Some(font) = font else {
            for (i, ch) in text.chars().enumerate() {
                if ch.is_ascii_digit() {
                    self.digit(x + i as isize * 10, y, ch as usize - '0' as usize, color);
                }
            }
            return;
        };
        let mut cursor = x;
        for ch in text.chars() {
            let (m, alpha) = font.rasterize(ch, size);
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let px = cursor + m.xmin as isize + xx as isize;
                    let py =
                        y + (size as isize - m.height as isize - m.ymin as isize) + yy as isize;
                    if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
                        continue;
                    }
                    let i = (py as usize * self.width + px as usize) * 3;
                    let a = alpha[yy * m.width + xx] as u32;
                    for (c, col) in color.iter().enumerate() {
                        self.rgb[i + c] =
                            ((self.rgb[i + c] as u32 * (255 - a) + *col as u32 * a) / 255) as u8;
                    }
                }
            }
            cursor += m.advance_width.round() as isize;
        }
    }
    fn digit(&mut self, x: isize, y: isize, n: usize, color: [u8; 3]) {
        const DIGITS: [[u8; 5]; 10] = [
            [7, 5, 5, 5, 7],
            [2, 6, 2, 2, 7],
            [7, 1, 7, 4, 7],
            [7, 1, 7, 1, 7],
            [5, 5, 7, 1, 1],
            [7, 4, 7, 1, 7],
            [7, 4, 7, 5, 7],
            [7, 1, 1, 1, 1],
            [7, 5, 7, 5, 7],
            [7, 5, 7, 1, 7],
        ];
        for (yy, row) in DIGITS[n % 10].iter().enumerate() {
            for xx in 0..3 {
                if row & (1 << (2 - xx)) != 0 {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            self.pixel(x + xx * 2 + dx, y + yy as isize * 2 + dy, color);
                        }
                    }
                }
            }
        }
    }
}
fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32fast::hash(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}
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
    let mut im = Image::new(t.n, t.n, [0, 0, 0]);
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
            let y = (o.cell[1] * 512. / t.n as f64).round_ties_even() as isize;
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
    let mut walk = base.resize(t.n, t.n);
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
    let top = 120;
    let bottom = 85;
    let mut preview = Image::new(side, side + top + bottom, [22, 29, 38]);
    let large = base.resize(side, side);
    for y in 0..side {
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
        (side + top + 18) as isize,
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
