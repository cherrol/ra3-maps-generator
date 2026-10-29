//! NumPy 数组的形状、数值解码及统一写入。
use crate::{Result, err, formats::binary::Reader};

#[derive(Debug)]
pub struct Npy {
    pub shape: Vec<usize>,
    pub dtype: String,
    pub bytes: Vec<u8>,
}
impl Npy {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data);
        if r.take(6)? != b"\x93NUMPY" {
            return Err(err("invalid NPY magic"));
        }
        let major = r.u8()?;
        r.u8()?;
        let len = match major {
            1 => r.u16()? as usize,
            2 | 3 => r.u32()? as usize,
            _ => return Err(err("unsupported NPY version")),
        };
        let header = std::str::from_utf8(r.take(len)?)?;
        if header.contains("'fortran_order': True") {
            return Err(err("Fortran order NPY unsupported"));
        }
        let dtype = header
            .split("'descr':")
            .nth(1)
            .ok_or_else(|| err("NPY missing dtype"))?
            .trim()
            .split('\'')
            .nth(1)
            .ok_or_else(|| err("NPY invalid dtype"))?
            .to_string();
        let shape = header
            .split("'shape':")
            .nth(1)
            .and_then(|s| s.split('(').nth(1))
            .and_then(|s| s.split(')').next())
            .ok_or_else(|| err("NPY missing shape"))?
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<usize>().map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            shape,
            dtype,
            bytes: r.take(r.remaining())?.to_vec(),
        })
    }
    pub fn numbers(&self) -> Result<Vec<f64>> {
        let (size, kind) = match self.dtype.as_str() {
            "<f8" | "=f8" => (8, 'd'),
            "<f4" | "=f4" => (4, 'f'),
            "<u2" | "=u2" => (2, 'u'),
            "<i2" | "=i2" => (2, 's'),
            "<i4" | "=i4" => (4, 'i'),
            "<i8" | "=i8" => (8, 'l'),
            "|u1" | "|b1" => (1, 'b'),
            _ => return Err(err(format!("unsupported NPY dtype {}", self.dtype))),
        };
        let count = self.shape.iter().try_fold(1usize, |a, &n| {
            a.checked_mul(n).ok_or_else(|| err("NPY shape overflow"))
        })?;
        if count.checked_mul(size) != Some(self.bytes.len()) {
            return Err(err("NPY size/shape mismatch"));
        }
        Ok(self
            .bytes
            .chunks_exact(size)
            .map(|b| match kind {
                'd' => f64::from_le_bytes(b.try_into().unwrap()),
                'f' => f32::from_le_bytes(b.try_into().unwrap()) as f64,
                'u' => u16::from_le_bytes(b.try_into().unwrap()) as f64,
                's' => i16::from_le_bytes(b.try_into().unwrap()) as f64,
                'i' => i32::from_le_bytes(b.try_into().unwrap()) as f64,
                'l' => i64::from_le_bytes(b.try_into().unwrap()) as f64,
                _ => b[0] as f64,
            })
            .collect())
    }
}

pub fn encode(dtype: &str, shape: &[usize], data: &[u8]) -> Vec<u8> {
    let dims = shape
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let tuple = if shape.is_empty() {
        String::new()
    } else {
        format!("{dims},")
    };
    let mut header =
        format!("{{'descr': '{dtype}', 'fortran_order': False, 'shape': ({tuple}), }}");
    let padding = (64 - (10 + header.len() + 1) % 64) % 64;
    header.extend(std::iter::repeat_n(' ', padding));
    header.push('\n');
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend_from_slice(&(header.len() as u16).to_le_bytes());
    out.extend(header.as_bytes());
    out.extend(data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_vector_and_rectangular_arrays_keep_shapes_values_and_aligned_headers() {
        for (shape, values) in [
            (vec![], vec![2.5_f32]),
            (vec![3], vec![0.5, -1., 8.]),
            (vec![2, 3], vec![0., 1., 2., 3., 4., 5.]),
            (vec![0, 3], vec![]),
        ] {
            let bytes = values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>();
            let encoded = encode("<f4", &shape, &bytes);
            let header_length = u16::from_le_bytes([encoded[8], encoded[9]]) as usize;
            assert_eq!((10 + header_length) % 64, 0);
            let parsed = Npy::parse(&encoded).unwrap();
            assert_eq!(parsed.shape, shape);
            assert_eq!(parsed.bytes, bytes);
            assert_eq!(
                parsed.numbers().unwrap(),
                values.iter().map(|&v| v as f64).collect::<Vec<_>>()
            );
            assert_eq!(encode(&parsed.dtype, &parsed.shape, &parsed.bytes), encoded);
        }
        let scalar = encode("<f4", &[], &1_f32.to_le_bytes());
        assert!(
            std::str::from_utf8(&scalar[10..64])
                .unwrap()
                .contains("'shape': ()")
        );
        assert!(
            Npy::parse(&encode("<f4", &[2, 3], &[0; 20]))
                .unwrap()
                .numbers()
                .is_err()
        );
    }
}
