//! 像素绘制、文字排版、图像缩放及 PNG/TGA 编解码。
use crate::{Result, err};
use flate2::{Compression, write::ZlibEncoder};
use std::{fs, io::Write, path::Path};

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
