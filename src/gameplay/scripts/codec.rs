//! 编解码原生脚本树，保留容器前缀、条件、动作和启用标记。
use super::Node;
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{Chunk, MapFile, Table},
};

pub fn unhex(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err(err("odd hex length"));
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| {
            let v = |c: u8| match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(err("invalid hex digit")),
            };
            Ok(v(b[0])? * 16 + v(b[1])?)
        })
        .collect()
}
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{b:02x}").unwrap();
    }
    s
}
pub fn prefix_end(data: &[u8], asset: &str, version: u16) -> Result<usize> {
    let mut r = Reader::new(data);
    match asset {
        "Script" => {
            for _ in 0..4 {
                r.s16()?;
            }
            r.take(6)?;
            if version >= 2 {
                r.take(4)?;
                if version == 5 {
                    r.take(5)?;
                }
            }
            if version >= 3 {
                r.take(7)?;
                r.s16()?;
            }
            if version >= 4 {
                r.s16()?;
            }
            if version >= 6 {
                r.take(6)?;
            }
        }
        "ScriptGroup" => {
            r.s16()?;
            r.take(2)?;
        }
        "ScriptList" | "OrCondition" => {}
        _ => return Err(err("not a script container")),
    }
    Ok(r.pos)
}
pub fn parse_nodes(data: &[u8], table: &Table) -> Result<Vec<Node>> {
    parse_depth(data, table, 0)
}
fn parse_depth(data: &[u8], table: &Table, depth: usize) -> Result<Vec<Node>> {
    if depth > 64 {
        return Err(err("script nesting exceeds 64"));
    }
    let mut r = Reader::new(data);
    let mut out = Vec::new();
    while r.remaining() > 0 {
        let c = Chunk::read(&mut r)?;
        let asset = table
            .get(&c.name_idx)
            .ok_or_else(|| err("unknown script asset"))?
            .clone();
        let mut node = Node {
            asset: asset.clone(),
            version: c.version,
            prefix: String::new(),
            body: String::new(),
            command: None,
            children: None,
        };
        match asset.as_str() {
            "Script" | "ScriptGroup" | "OrCondition" | "ScriptList" => {
                let split = prefix_end(&c.payload, &asset, c.version)?;
                node.prefix = hex(&c.payload[..split]);
                node.children = Some(parse_depth(&c.payload[split..], table, depth + 1)?);
            }
            "ScriptAction" | "ScriptActionFalse" | "Condition" => {
                let mut body = c.payload;
                let has_key = (asset == "Condition" && c.version >= 4)
                    || (asset != "Condition" && c.version >= 2);
                if has_key {
                    let mut b = Reader::new(&body);
                    b.u32()?;
                    if b.u8()? != 3 {
                        return Err(err("script internal name must use ASCII property type"));
                    }
                    let key = b.u24()?;
                    node.command = Some(
                        table
                            .get(&key)
                            .ok_or_else(|| err("unknown script command"))?
                            .clone(),
                    );
                    body[5..8].fill(0);
                }
                node.body = hex(&body);
                validate_leaf(&node)?;
            }
            _ => return Err(err(format!("unknown script asset {asset}"))),
        }
        out.push(node);
    }
    Ok(out)
}
pub fn encode_nodes(nodes: &[Node], map: &mut MapFile) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    for node in nodes {
        let idx = map.id(&node.asset)?;
        let mut body = if let Some(children) = &node.children {
            let mut body = unhex(&node.prefix)?;
            prefix_end(&body, &node.asset, node.version)?;
            body.extend(encode_nodes(children, map)?);
            body
        } else {
            validate_leaf(node)?;
            unhex(&node.body)?
        };
        if let Some(command) = &node.command {
            let key = map.id(command)?;
            if key > 0xffffff {
                return Err(err("script command id overflow"));
            }
            body[5..8].copy_from_slice(&key.to_le_bytes()[..3]);
        }
        Chunk {
            name_idx: idx,
            version: node.version,
            payload: body,
        }
        .write(&mut w)?;
    }
    Ok(w.data)
}
fn validate_leaf(node: &Node) -> Result<()> {
    let body = unhex(&node.body)?;
    let mut r = Reader::new(&body);
    r.u32()?;
    let condition = node.asset == "Condition";
    if (condition && node.version >= 4) || (!condition && node.version >= 2) {
        r.take(4)?;
    }
    let count = r.u32()? as usize;
    if count > r.remaining() / 14 {
        return Err(err("invalid script argument count"));
    }
    for _ in 0..count {
        let ty = r.u32()?;
        if ty == 16 {
            r.take(12)?;
        } else {
            r.i32()?;
            let f = r.f32()?;
            if !f.is_finite() {
                return Err(err("non-finite script argument"));
            }
            r.s16()?;
        }
    }
    if ((condition && node.version >= 5) || (!condition && node.version >= 3)) && r.u32()? > 1 {
        return Err(err("invalid script enabled flag"));
    }
    if condition && node.version >= 5 && r.u32()? > 1 {
        return Err(err("invalid condition inverted flag"));
    }
    r.finish()
}
pub fn name(node: &Node) -> Result<Option<String>> {
    if node.asset == "Script" || node.asset == "ScriptGroup" {
        Ok(Some(Reader::new(&unhex(&node.prefix)?).s16()?))
    } else {
        Ok(None)
    }
}
pub fn script_names(nodes: &[Node]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for n in nodes {
        if n.asset == "Script"
            && let Some(name) = name(n)?
        {
            out.push(name);
        }
        if let Some(children) = &n.children {
            out.extend(script_names(children)?);
        }
    }
    Ok(out)
}
pub fn validate(map: &MapFile) -> Result<()> {
    let nodes = parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let _ = script_names(&nodes)?;
    Ok(())
}
