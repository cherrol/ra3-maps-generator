//! Small ZIP/NPY reader for existing blueprint inputs; deterministic ZIP writer.
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

pub fn unzip(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let eocd = (data.len().saturating_sub(65557)..data.len().saturating_sub(21))
        .rev()
        .find(|&i| data.get(i..i + 4) == Some(b"PK\x05\x06"))
        .ok_or_else(|| err("missing ZIP central directory"))?;
    let mut e = Reader::new(&data[eocd + 4..]);
    if e.u16()? != 0 || e.u16()? != 0 {
        return Err(err("multi-disk ZIP is unsupported"));
    }
    let count = e.u16()?;
    if e.u16()? != count {
        return Err(err("inconsistent ZIP entry count"));
    }
    let len = e.u32()? as usize;
    let off = e.u32()? as usize;
    let comment = e.u16()? as usize;
    if eocd + 22 + comment != data.len() {
        return Err(err("invalid ZIP footer"));
    }
    let mut r = Reader::new(
        data.get(off..off.checked_add(len).ok_or_else(|| err("ZIP overflow"))?)
            .ok_or_else(|| err("invalid ZIP directory offset"))?,
    );
    let mut files = BTreeMap::new();
    for _ in 0..count {
        if r.u32()? != 0x02014b50 {
            return Err(err("invalid ZIP directory entry"));
        }
        r.take(4)?;
        let flags = r.u16()?;
        let method = r.u16()?;
        r.take(4)?;
        let crc = r.u32()?;
        let packed = r.u32()? as usize;
        let size = r.u32()? as usize;
        let nl = r.u16()? as usize;
        let xl = r.u16()? as usize;
        let cl = r.u16()? as usize;
        r.take(8)?;
        let offset = r.u32()? as usize;
        let raw_name = r.take(nl)?;
        let extra = r.take(xl)?;
        let name = zip_name(raw_name, extra)?;
        r.take(cl)?;
        if flags & 1 != 0 || size > 256 * 1024 * 1024 {
            return Err(err("encrypted or oversized ZIP entry"));
        }
        let mut local = Reader::new(
            data.get(offset..)
                .ok_or_else(|| err("invalid ZIP local offset"))?,
        );
        if local.u32()? != 0x04034b50 {
            return Err(err("invalid ZIP local entry"));
        }
        local.u16()?; // version needed
        let local_flags = local.u16()?;
        let local_method = local.u16()?;
        if local_flags != flags || local_method != method {
            return Err(err("ZIP local/central flags or method differ"));
        }
        local.take(16)?; // timestamps, CRC and (possibly ZIP64 sentinel) lengths
        let ln = local.u16()? as usize;
        let lx = local.u16()? as usize;
        let local_name = local.take(ln)?;
        if local_name != raw_name {
            return Err(err("ZIP local/central file names differ"));
        }
        local.take(lx)?;
        let body = local.take(packed)?;
        let out = match method {
            0 => body.to_vec(),
            8 => {
                let mut out = Vec::new();
                DeflateDecoder::new(body)
                    .take(size as u64 + 1)
                    .read_to_end(&mut out)?;
                out
            }
            _ => return Err(err(format!("unsupported ZIP compression {method}"))),
        };
        if out.len() != size || crc32fast::hash(&out) != crc {
            return Err(err(format!("ZIP CRC/length mismatch: {name}")));
        }
        if files.insert(name, out).is_some() {
            return Err(err("duplicate ZIP entry"));
        }
    }
    r.finish()?;
    Ok(files)
}
// WorldBuilder ZIPs use GBK names with an Info-ZIP Unicode Path field.
fn zip_name(raw: &[u8], extra: &[u8]) -> Result<String> {
    if let Ok(name) = std::str::from_utf8(raw) {
        return Ok(name.to_owned());
    }
    let mut r = Reader::new(extra);
    while r.remaining() > 0 {
        let tag = r.u16()?;
        let len = r.u16()? as usize;
        let data = r.take(len)?;
        if tag == 0x7075 {
            let mut f = Reader::new(data);
            if f.u8()? != 1 || f.u32()? != crc32fast::hash(raw) {
                return Err(err("invalid ZIP Unicode Path CRC/version"));
            }
            return Ok(String::from_utf8(f.take(f.remaining())?.to_vec())?);
        }
    }
    Err(err("non-UTF8 ZIP name without Unicode Path field"))
}
pub fn zip(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut out = Writer::new();
    let mut central = Writer::new();
    for (name, data) in files {
        if name.contains('\\') || name.starts_with('/') || name.split('/').any(|p| p == "..") {
            return Err(err("invalid ZIP path"));
        }
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
        enc.write_all(data)?;
        let comp = enc.finish()?;
        let crc = crc32fast::hash(data);
        let pos = u32::try_from(out.data.len())?;
        let packed = u32::try_from(comp.len())?;
        let size = u32::try_from(data.len())?;
        let nl = u16::try_from(name.len())?;
        out.u32(0x04034b50);
        out.u16(20);
        out.u16(0x800);
        out.u16(8);
        out.u16(0);
        out.u16(0x21);
        out.u32(crc);
        out.u32(packed);
        out.u32(size);
        out.u16(nl);
        out.u16(0);
        out.bytes(name.as_bytes());
        out.bytes(&comp);
        central.u32(0x02014b50);
        central.u16(20);
        central.u16(20);
        central.u16(0x800);
        central.u16(8);
        central.u16(0);
        central.u16(0x21);
        central.u32(crc);
        central.u32(packed);
        central.u32(size);
        central.u16(nl);
        central.u16(0);
        central.u16(0);
        central.u16(0);
        central.u16(0);
        central.u32(0);
        central.u32(pos);
        central.bytes(name.as_bytes());
    }
    let start = u32::try_from(out.data.len())?;
    let len = u32::try_from(central.data.len())?;
    out.bytes(&central.data);
    out.u32(0x06054b50);
    out.u16(0);
    out.u16(0);
    out.u16(u16::try_from(files.len())?);
    out.u16(u16::try_from(files.len())?);
    out.u32(len);
    out.u32(start);
    out.u16(0);
    Ok(out.data)
}
