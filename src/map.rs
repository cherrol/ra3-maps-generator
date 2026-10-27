use crate::{
    Result,
    binary::{Reader, Writer},
    err, refpack,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};
pub type Table = BTreeMap<u32, String>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub name_idx: u32,
    pub version: u16,
    pub payload: Vec<u8>,
}
impl Chunk {
    pub fn read(r: &mut Reader<'_>) -> Result<Self> {
        let name_idx = r.u32()?;
        let version = r.u16()?;
        let n = r.u32()? as usize;
        Ok(Self {
            name_idx,
            version,
            payload: r.take(n)?.to_vec(),
        })
    }
    pub fn write(&self, w: &mut Writer) -> Result<()> {
        w.u32(self.name_idx);
        w.u16(self.version);
        w.u32(u32::try_from(self.payload.len())?);
        w.bytes(&self.payload);
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct MapFile {
    pub order: Vec<(String, u32)>,
    pub table: Table,
    pub chunks: Vec<Chunk>,
}
impl MapFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_bytes(&fs::read(path)?)
    }
    pub fn load_bytes(data: &[u8]) -> Result<Self> {
        Self::decode(&refpack::unpack(data)?)
    }
    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data);
        if r.take(4)? != b"CkMp" {
            return Err(err("invalid CkMp magic"));
        }
        let count = r.u32()? as usize;
        if count > r.remaining() / 5 {
            return Err(err("invalid asset count"));
        }
        let mut order = Vec::new();
        let mut table = Table::new();
        for _ in 0..count {
            let name = r.varstr()?;
            let id = r.u32()?;
            if id == 0 || table.insert(id, name.clone()).is_some() {
                return Err(err("invalid or duplicate asset id"));
            }
            order.push((name, id));
        }
        let mut chunks = Vec::new();
        while r.remaining() > 0 {
            let c = Chunk::read(&mut r)?;
            if !table.contains_key(&c.name_idx) {
                return Err(err(format!("unknown chunk id {}", c.name_idx)));
            }
            chunks.push(c);
        }
        Ok(Self {
            order,
            table,
            chunks,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer::new();
        w.bytes(b"CkMp");
        w.u32(u32::try_from(self.table.len())?);
        // WorldBuilder requires N, N-1, ..., 1. Linking adds names after the
        // imported table; insertion order must never become disk order.
        for (expected, (i, s)) in (1..=self.table.len() as u32)
            .rev()
            .zip(self.table.iter().rev())
        {
            if *i != expected {
                return Err(err("asset string IDs must be contiguous from 1"));
            }
            w.varstr(s)?;
            w.u32(*i);
        }
        for c in &self.chunks {
            c.write(&mut w)?;
        }
        Ok(w.data)
    }
    /// Check the actual serialized table, not merely whether our lenient
    /// importer can read it. Keep imports lenient so old releases can be repaired.
    pub fn validate_release(&self) -> Result<()> {
        if self.order.len() != self.table.len()
            || self.order.iter().enumerate().any(|(n, (s, id))| {
                *id as usize != self.order.len() - n || self.table.get(id) != Some(s)
            })
        {
            return Err(err(
                "release string table must be strictly N..1; rebuild or repack the map",
            ));
        }
        if let Ok(c) = self.get("AssetList") {
            let mut r = Reader::new(&c.payload);
            let n = r.u32()? as usize;
            if n.checked_mul(8) != Some(r.remaining()) {
                return Err(err("invalid AssetList resource count/length"));
            }
        }
        Ok(())
    }
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, refpack::pack(&self.encode()?)?)?;
        Ok(())
    }
    pub fn get(&self, name: &str) -> Result<&Chunk> {
        self.chunks
            .iter()
            .find(|c| self.table.get(&c.name_idx).is_some_and(|n| n == name))
            .ok_or_else(|| err(format!("missing chunk {name}")))
    }
    pub fn get_mut(&mut self, name: &str) -> Result<&mut Chunk> {
        self.chunks
            .iter_mut()
            .find(|c| self.table.get(&c.name_idx).is_some_and(|n| n == name))
            .ok_or_else(|| err(format!("missing chunk {name}")))
    }
    pub fn id(&mut self, name: &str) -> Result<u32> {
        if let Some((id, _)) = self.table.iter().find(|(_, s)| s.as_str() == name) {
            return Ok(*id);
        }
        let id = self
            .table
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| err("asset id overflow"))?;
        self.order.push((name.into(), id));
        self.table.insert(id, name.into());
        Ok(id)
    }
    pub fn key(&self, name: &str) -> Result<u32> {
        self.table
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(i, _)| *i)
            .ok_or_else(|| err(format!("unknown property {name}")))
    }
    pub fn remove(&mut self, name: &str) {
        self.chunks
            .retain(|c| self.table.get(&c.name_idx).is_none_or(|n| n != name));
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Value {
    Bool(u8),
    Int(i32),
    Float(f32),
    Ascii(String),
    Unicode(String),
    Unknown(String),
}
impl Value {
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Ascii(s) | Self::Unicode(s) | Self::Unknown(s) => Some(s),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Property {
    pub key: String,
    #[serde(flatten)]
    pub value: Value,
}
pub fn read_props(r: &mut Reader<'_>, table: &Table) -> Result<Vec<Property>> {
    let n = r.u16()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let ty = r.u8()?;
        let id = r.u24()?;
        let key = table
            .get(&id)
            .ok_or_else(|| err(format!("unknown property id {id}")))?
            .clone();
        let value = match ty {
            0 => Value::Bool(r.u8()?),
            1 => Value::Int(r.i32()?),
            2 => Value::Float(r.f32()?),
            3 => Value::Ascii(r.s16()?),
            4 => Value::Unicode(r.unicode()?),
            5 => Value::Unknown(r.s16()?),
            _ => return Err(err(format!("invalid property type {ty}"))),
        };
        out.push(Property { key, value });
    }
    Ok(out)
}
pub fn write_props(w: &mut Writer, props: &[Property], map: &MapFile) -> Result<()> {
    w.u16(u16::try_from(props.len())?);
    for p in props {
        let ty = match p.value {
            Value::Bool(_) => 0,
            Value::Int(_) => 1,
            Value::Float(_) => 2,
            Value::Ascii(_) => 3,
            Value::Unicode(_) => 4,
            Value::Unknown(_) => 5,
        };
        w.u8(ty);
        w.u24(map.key(&p.key)?)?;
        match &p.value {
            Value::Bool(v) => w.u8(*v),
            Value::Int(v) => w.i32(*v),
            Value::Float(v) => w.f32(*v),
            Value::Ascii(v) | Value::Unknown(v) => w.s16(v)?,
            Value::Unicode(v) => w.unicode(v)?,
        }
    }
    Ok(())
}
pub fn prop<'a>(props: &'a [Property], key: &str) -> Option<&'a Value> {
    props.iter().find(|p| p.key == key).map(|p| &p.value)
}
pub fn set(props: &mut Vec<Property>, key: &str, value: Value) {
    if let Some(p) = props.iter_mut().find(|p| p.key == key) {
        p.value = value;
    } else {
        props.push(Property {
            key: key.into(),
            value,
        });
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MapObject {
    pub version: u16,
    pub position: [f32; 3],
    pub angle: f32,
    pub road_type: u32,
    pub type_name: String,
    pub props: Vec<Property>,
}
pub fn objects(c: &Chunk, table: &Table) -> Result<Vec<MapObject>> {
    let mut r = Reader::new(&c.payload);
    let mut out = Vec::new();
    while r.remaining() > 0 {
        let c = Chunk::read(&mut r)?;
        if table.get(&c.name_idx).map(String::as_str) != Some("Object") {
            return Err(err("unexpected ObjectsList child"));
        }
        let mut b = Reader::new(&c.payload);
        let o = MapObject {
            version: c.version,
            position: b.vec3()?,
            angle: b.f32()?,
            road_type: b.u32()?,
            type_name: b.s16()?,
            props: read_props(&mut b, table)?,
        };
        b.finish()?;
        out.push(o);
    }
    Ok(out)
}
pub fn write_objects(objs: &[MapObject], map: &MapFile) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    for o in objs {
        let mut b = Writer::new();
        b.vec3(o.position);
        b.f32(o.angle);
        b.u32(o.road_type);
        b.s16(&o.type_name)?;
        write_props(&mut b, &o.props, map)?;
        Chunk {
            name_idx: map.key("Object")?,
            version: o.version,
            payload: b.data,
        }
        .write(&mut w)?;
    }
    Ok(w.data)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightMap {
    pub width: usize,
    pub height: usize,
    pub border: u32,
    pub borders: Vec<[u32; 4]>,
    pub elevations: Vec<u16>,
}
impl HeightMap {
    pub fn parse(c: &Chunk) -> Result<Self> {
        let mut r = Reader::new(&c.payload);
        let width = r.u32()? as usize;
        let height = r.u32()? as usize;
        let border = r.u32()?;
        if (border as usize)
            .checked_mul(2)
            .is_none_or(|b| b >= width.min(height))
        {
            return Err(err("heightmap border exceeds dimensions"));
        }
        let n = r.u32()? as usize;
        if n > r.remaining() / 8 {
            return Err(err("invalid heightmap borders"));
        }
        let mut borders = Vec::new();
        for _ in 0..n {
            let mut b = [0; 4];
            if c.version >= 6 {
                b[0] = r.u32()?;
                b[1] = r.u32()?;
            }
            b[2] = r.u32()?;
            b[3] = r.u32()?;
            borders.push(b);
        }
        let area = r.u32()? as usize;
        if width == 0 || height == 0 || width.checked_mul(height) != Some(area) {
            return Err(err("invalid heightmap dimensions"));
        }
        let size = if c.version >= 5 { 2 } else { 1 };
        if area > r.remaining() / size {
            return Err(err("truncated heightmap"));
        }
        let elevations = (0..area)
            .map(|_| {
                if size == 2 {
                    r.u16()
                } else {
                    r.u8().map(u16::from)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        r.finish()?;
        Ok(Self {
            width,
            height,
            border,
            borders,
            elevations,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer::new();
        w.u32(u32::try_from(self.width)?);
        w.u32(u32::try_from(self.height)?);
        w.u32(self.border);
        w.u32(u32::try_from(self.borders.len())?);
        for b in &self.borders {
            for v in b {
                w.u32(*v);
            }
        }
        w.u32(u32::try_from(self.elevations.len())?);
        for v in &self.elevations {
            w.u16(*v);
        }
        Ok(w.data)
    }
}
#[derive(Clone, Debug)]
pub struct BlendTiles {
    pub tiles: Vec<u16>,
    pub impassability: Vec<bool>,
    pub buildability: Vec<bool>,
    pub textures: Vec<String>,
    pub tail: Vec<u8>,
    pub blend_descriptions: Vec<u8>,
}
impl BlendTiles {
    pub fn parse(c: &Chunk, width: usize, height: usize) -> Result<Self> {
        let mut r = Reader::new(&c.payload);
        let n = r.u32()? as usize;
        if width.checked_mul(height) != Some(n) {
            return Err(err("blend tile dimensions mismatch"));
        }
        if n > r.remaining() / 8 {
            return Err(err("truncated blend tile arrays"));
        }
        let tiles = (0..n).map(|_| r.u16()).collect::<Result<Vec<_>>>()?;
        let size = if (14..24).contains(&c.version) { 4 } else { 2 };
        r.take(n * size * 3)?;
        let mut impassability = vec![false; n];
        let mut buildability = vec![false; n];
        if c.version > 6 {
            impassability = read_bits(&mut r, width, height)?;
        }
        if c.version >= 10 {
            read_bits(&mut r, width, height)?;
        }
        if c.version >= 11 {
            read_bits(&mut r, width, height)?;
        }
        if (14..25).contains(&c.version) {
            read_bits(&mut r, width, height)?;
        }
        if c.version >= 15 {
            read_bits(&mut r, width, height)?;
        }
        if (16..25).contains(&c.version) {
            r.take(n)?;
        }
        if c.version >= 17 {
            read_bits(&mut r, width, height)?;
        }
        if c.version >= 24 {
            buildability = read_bits(&mut r, width, height)?;
            read_bits(&mut r, width, height)?;
            read_bits(&mut r, width, height)?;
        }
        if c.version >= 25 {
            r.take(n)?;
        }
        let start = r.pos;
        r.u32()?;
        let blends = r.u32()?.saturating_sub(1) as usize;
        let cliffs = r.u32()?.saturating_sub(1) as usize;
        let count = r.u32()? as usize;
        if count > r.remaining() / 18 {
            return Err(err("invalid texture count"));
        }
        let mut textures = Vec::new();
        for _ in 0..count {
            r.take(16)?;
            textures.push(r.s16()?);
        }
        r.take(8)?;
        let blend_descriptions = r
            .take(
                blends
                    .checked_mul(18)
                    .ok_or_else(|| err("blend length overflow"))?,
            )?
            .to_vec();
        r.take(
            cliffs
                .checked_mul(38)
                .ok_or_else(|| err("cliff length overflow"))?,
        )?;
        r.finish()?;
        Ok(Self {
            tiles,
            impassability,
            buildability,
            textures,
            tail: c.payload[start..].to_vec(),
            blend_descriptions,
        })
    }
}
pub fn read_bits(r: &mut Reader<'_>, width: usize, height: usize) -> Result<Vec<bool>> {
    let mut v = Vec::new();
    for _ in 0..height {
        let row = r.take(width.div_ceil(8))?;
        v.extend((0..width).map(|x| row[x / 8] & (1 << (x % 8)) != 0));
    }
    Ok(v)
}
pub fn write_bits(w: &mut Writer, mask: &[bool], width: usize) {
    for row in mask.chunks(width) {
        for byte in row.chunks(8) {
            w.u8(byte
                .iter()
                .enumerate()
                .fold(0, |a, (i, v)| a | ((*v as u8) << i)));
        }
    }
}
