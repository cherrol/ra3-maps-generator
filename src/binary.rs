use crate::{Result, err};

/// Bounds checked, little endian reader. Subchunks use independent readers.
pub struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| err("binary length overflow"))?;
        let value = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| err(format!("truncated input at {} (need {n} bytes)", self.pos)))?;
        self.pos = end;
        Ok(value)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    pub fn u24(&mut self) -> Result<u32> {
        let b = self.take(3)?;
        Ok(u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16)
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn vec3(&mut self) -> Result<[f32; 3]> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    pub fn s16(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        Ok(latin1(self.take(n)?))
    }
    pub fn unicode(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        let b = self.take(n * 2)?;
        Ok(String::from_utf16(
            &b.as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        )?)
    }
    pub fn varstr(&mut self) -> Result<String> {
        let mut n = 0u32;
        for shift in (0..35).step_by(7) {
            let b = self.u8()?;
            if shift == 28 && b > 15 {
                return Err(err("invalid string varint"));
            }
            n |= u32::from(b & 127) << shift;
            if b & 128 == 0 {
                let b = self.take(n as usize)?;
                if !b.is_ascii() {
                    return Err(err("non ASCII asset name"));
                }
                return Ok(String::from_utf8(b.to_vec())?);
            }
        }
        Err(err("unterminated string varint"))
    }
    pub fn finish(&self) -> Result<()> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(err(format!("{} unconsumed bytes", self.remaining())))
        }
    }
}
pub fn latin1(b: &[u8]) -> String {
    b.iter().map(|&v| char::from(v)).collect()
}
#[derive(Default)]
pub struct Writer {
    pub data: Vec<u8>,
}
impl Writer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bytes(&mut self, v: &[u8]) {
        self.data.extend_from_slice(v);
    }
    pub fn u8(&mut self, v: u8) {
        self.data.push(v);
    }
    pub fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u24(&mut self, v: u32) -> Result<()> {
        if v > 0xffffff {
            return Err(err("property key exceeds uint24"));
        }
        self.bytes(&v.to_le_bytes()[..3]);
        Ok(())
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn vec3(&mut self, v: [f32; 3]) {
        for f in v {
            self.f32(f);
        }
    }
    pub fn s16(&mut self, v: &str) -> Result<()> {
        let b = v
            .chars()
            .map(|c| u8::try_from(c as u32).map_err(|_| err("string is not Latin 1")))
            .collect::<Result<Vec<_>>>()?;
        self.u16(u16::try_from(b.len())?);
        self.bytes(&b);
        Ok(())
    }
    pub fn unicode(&mut self, v: &str) -> Result<()> {
        let b = v.encode_utf16().collect::<Vec<_>>();
        self.u16(u16::try_from(b.len())?);
        for c in b {
            self.u16(c);
        }
        Ok(())
    }
    pub fn varstr(&mut self, v: &str) -> Result<()> {
        if !v.is_ascii() {
            return Err(err("non ASCII asset name"));
        }
        let mut n = u32::try_from(v.len())?;
        while n >= 128 {
            self.u8((n as u8 & 127) | 128);
            n >>= 7;
        }
        self.u8(n as u8);
        self.bytes(v.as_bytes());
        Ok(())
    }
}
