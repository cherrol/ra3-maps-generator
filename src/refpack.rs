//! EA RefPack, including the community 0x80/0x81 large header variant.
use crate::{Result, binary::Reader, err};
use std::collections::HashMap;

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut r = Reader::new(data);
    let mut marker = r.u8()?;
    if marker == 0x80 || marker == 0x81 {
        marker |= 0x10;
    }
    if marker & 0x3e != 0x10 || r.u8()? != 0xfb {
        return Err(err("invalid RefPack header"));
    }
    let n = if marker & 0x80 != 0 { 4 } else { 3 };
    let size = |r: &mut Reader<'_>| -> Result<usize> {
        let mut v = 0usize;
        for &b in r.take(n)? {
            v = (v << 8) | b as usize;
        }
        Ok(v)
    };
    if marker & 1 != 0 {
        size(&mut r)?;
    }
    let expected = size(&mut r)?;
    let mut out = Vec::new();
    // Do not trust the declared length enough to allocate it before decoding.
    loop {
        let a = r.u8()?;
        let (lit, len, dist, stop) = if a < 0x80 {
            let b = r.u8()?;
            (
                (a & 3) as usize,
                ((a & 0x1c) >> 2) as usize + 3,
                (((a & 0x60) as usize) << 3) + b as usize + 1,
                false,
            )
        } else if a < 0xc0 {
            let b = r.u8()?;
            let c = r.u8()?;
            (
                (b >> 6) as usize,
                (a & 0x3f) as usize + 4,
                (((b & 0x3f) as usize) << 8) + c as usize + 1,
                false,
            )
        } else if a < 0xe0 {
            let b = r.u8()?;
            let c = r.u8()?;
            let d = r.u8()?;
            (
                (a & 3) as usize,
                (((a & 0x0c) as usize) << 6) + d as usize + 5,
                (((a & 0x10) as usize) << 12) + ((b as usize) << 8) + c as usize + 1,
                false,
            )
        } else if a < 0xfc {
            ((((a & 0x1f) as usize) + 1) * 4, 0, 0, false)
        } else {
            ((a & 3) as usize, 0, 0, true)
        };
        if out
            .len()
            .checked_add(lit)
            .and_then(|v| v.checked_add(len))
            .is_none_or(|v| v > expected)
        {
            return Err(err("RefPack output exceeds declared size"));
        }
        out.extend_from_slice(r.take(lit)?);
        if len > 0 {
            if dist == 0 || dist > out.len() {
                return Err(err("invalid RefPack back reference"));
            }
            for _ in 0..len {
                out.push(out[out.len() - dist]);
            }
        }
        if stop {
            break;
        }
    }
    if out.len() != expected {
        return Err(err(format!(
            "RefPack size mismatch: {} != {expected}",
            out.len()
        )));
    }
    r.finish()?;
    Ok(out)
}
fn header(n: usize) -> Result<Vec<u8>> {
    if n >= 1 << 24 {
        return Err(err("RefPack writer supports sizes below 16 MiB"));
    }
    Ok(vec![0x10, 0xfb, (n >> 16) as u8, (n >> 8) as u8, n as u8])
}
pub fn literals(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = header(data.len())?;
    let mut p = 0;
    while data.len() - p >= 4 {
        let n = ((data.len() - p) / 4 * 4).min(112);
        out.push(0xe0 | ((n / 4 - 1) as u8));
        out.extend_from_slice(&data[p..p + n]);
        p += n;
    }
    out.push(0xfc | ((data.len() - p) as u8));
    out.extend_from_slice(&data[p..]);
    Ok(out)
}
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    compress_with_chain(data, 24)
}
pub fn compress_with_chain(data: &[u8], max_chain: usize) -> Result<Vec<u8>> {
    let n = data.len();
    let mut out = header(n)?;
    let mut heads = HashMap::<[u8; 3], usize>::new();
    let mut prev = vec![usize::MAX; n];
    let insert = |p: usize, heads: &mut HashMap<[u8; 3], usize>, prev: &mut [usize]| {
        if p + 3 <= n {
            let key = [data[p], data[p + 1], data[p + 2]];
            prev[p] = heads.insert(key, p).unwrap_or(usize::MAX);
        }
    };
    let mut pos = 0;
    let mut start = 0;
    while pos < n {
        let (mut best, mut dist) = (0, 0);
        if pos + 3 <= n {
            let mut cand = heads
                .get(&[data[pos], data[pos + 1], data[pos + 2]])
                .copied()
                .unwrap_or(usize::MAX);
            for _ in 0..max_chain {
                if cand == usize::MAX || pos - cand > 131072 {
                    break;
                }
                let mut len = 0;
                let limit = (n - pos).min(1028);
                while len < limit && data[cand + len] == data[pos + len] {
                    len += 1;
                }
                if len > best {
                    best = len;
                    dist = pos - cand;
                    if len >= 512 {
                        break;
                    }
                }
                cand = prev[cand];
            }
        }
        let kind = if best >= 3 && dist <= 1024 && best <= 10 {
            2
        } else if (4..=67).contains(&best) && dist <= 16384 {
            3
        } else if best >= 5 {
            4
        } else {
            0
        };
        if kind != 0 {
            let lit = pos - start;
            let d = dist - 1;
            match kind {
                2 => {
                    out.push((lit | ((best - 3) << 2) | ((d >> 8) << 5)) as u8);
                    out.push(d as u8);
                }
                3 => {
                    out.push(0x80 | (best - 4) as u8);
                    out.push(((lit << 6) | ((d >> 8) & 0x3f)) as u8);
                    out.push(d as u8);
                }
                _ => {
                    out.push(
                        (0xc0 | lit | (((best - 5) >> 8) << 2) | (((d >> 16) & 1) << 4)) as u8,
                    );
                    out.push((d >> 8) as u8);
                    out.push(d as u8);
                    out.push((best - 5) as u8);
                }
            }
            out.extend_from_slice(&data[start..pos]);
            for p in pos..pos + best {
                insert(p, &mut heads, &mut prev);
            }
            pos += best;
            start = pos;
        } else {
            insert(pos, &mut heads, &mut prev);
            pos += 1;
            // Preserve the original encoder's eager 4-byte literal emission.
            if pos - start >= 4 {
                out.push(0xe0);
                out.extend_from_slice(&data[start..pos]);
                start = pos;
            }
        }
    }
    out.push(0xfc | ((pos - start) as u8));
    out.extend_from_slice(&data[start..pos]);
    Ok(out)
}
pub fn unpack(data: &[u8]) -> Result<Vec<u8>> {
    if data.starts_with(b"CkMp") {
        return Ok(data.to_vec());
    }
    if !data.starts_with(b"EAR\0") || data.len() < 8 {
        return Err(err("expected EAR or CkMp map"));
    }
    let expected = u32::from_le_bytes(data[4..8].try_into()?) as usize;
    let plain = decompress(&data[8..])?;
    if plain.len() != expected || !plain.starts_with(b"CkMp") {
        return Err(err("invalid EAR payload or length"));
    }
    Ok(plain)
}
pub fn pack(plain: &[u8]) -> Result<Vec<u8>> {
    if !plain.starts_with(b"CkMp") {
        return Err(err("expected CkMp plaintext"));
    }
    let mut out = b"EAR\0".to_vec();
    out.extend_from_slice(&u32::try_from(plain.len())?.to_le_bytes());
    out.extend(compress(plain)?);
    Ok(out)
}
