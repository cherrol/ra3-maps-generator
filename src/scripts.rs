//! Script assets remain editable templates; all linking and validation is native.
use crate::{
    Result,
    binary::{Reader, Writer},
    err,
    map::{Chunk, MapFile, Table, read_props, write_props},
    project::Project,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs};
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub asset: String,
    pub version: u16,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,
}
#[derive(Deserialize)]
pub struct RuleChunk {
    version: u16,
    body: String,
}
#[derive(Deserialize)]
pub struct Rules {
    #[serde(rename = "SidesList")]
    sides: RuleChunk,
    #[serde(rename = "Teams")]
    teams: RuleChunk,
    pub names: Vec<(String, u32)>,
    pub scripts: Vec<Node>,
    #[serde(default)]
    pub spawn_waypoints: Vec<crate::script_spawns::SpawnWaypoint>,
    #[serde(default)]
    pub periodic_attack: Option<crate::periodic_attack::Config>,
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
pub fn load_rules(p: &Project) -> Result<Rules> {
    let key = p.string("RULESET", &p.id);
    crate::project::safe_component(key)?;
    Ok(serde_json::from_slice(&fs::read(
        p.root.join("templates/rules").join(format!("{key}.json")),
    )?)?)
}
pub fn apply(map: &mut MapFile, p: &Project) -> Result<()> {
    let rules = load_rules(p)?;
    // Names in the rule template are linked by meaning rather than numeric identity.
    let source = MapFile {
        order: rules.names.clone(),
        table: rules.names.iter().map(|(n, i)| (*i, n.clone())).collect(),
        chunks: Vec::new(),
    };
    for (n, _) in &rules.names {
        map.id(n)?;
    }
    let sides = remap_sides(&rules.sides, &source, map)?;
    let teams = remap_teams(&rules.teams, &source, map)?;
    map.get_mut("SidesList")?.payload = sides;
    map.get_mut("SidesList")?.version = rules.sides.version;
    map.get_mut("Teams")?.payload = teams;
    map.get_mut("Teams")?.version = rules.teams.version;
    let body = encode_nodes(&rules.scripts, map)?;
    map.get_mut("PlayerScriptsList")?.payload = body;
    validate(map)?;
    Ok(())
}
pub fn string_arguments(node: &Node) -> Result<Vec<(u32, String)>> {
    let bytes = unhex(&node.body)?;
    let mut r = Reader::new(&bytes);
    r.u32()?;
    if (node.asset == "Condition" && node.version >= 4)
        || (node.asset != "Condition" && node.version >= 2)
    {
        r.take(4)?;
    }
    let mut args = Vec::new();
    for _ in 0..r.u32()? {
        let ty = r.u32()?;
        if ty == 16 {
            r.take(12)?;
        } else {
            r.take(8)?;
            args.push((ty, r.s16()?));
        }
    }
    Ok(args)
}
fn remap_teams(c: &RuleChunk, source: &MapFile, dst: &MapFile) -> Result<Vec<u8>> {
    let bytes = unhex(&c.body)?;
    let mut r = Reader::new(&bytes);
    let n = r.u32()?;
    let mut w = Writer::new();
    w.u32(n);
    for _ in 0..n {
        let props = read_props(&mut r, &source.table)?;
        write_props(&mut w, &props, dst)?;
    }
    r.finish()?;
    Ok(w.data)
}
fn remap_sides(c: &RuleChunk, source: &MapFile, dst: &MapFile) -> Result<Vec<u8>> {
    let bytes = unhex(&c.body)?;
    let mut r = Reader::new(&bytes);
    let mut w = Writer::new();
    if c.version >= 6 {
        w.u8(r.u8()?);
    }
    let n = r.u32()?;
    w.u32(n);
    for _ in 0..n {
        let props = read_props(&mut r, &source.table)?;
        write_props(&mut w, &props, dst)?;
        let start = r.pos;
        let count = r.u32()?;
        for _ in 0..count {
            r.s16()?;
            r.s16()?;
            r.take(16)?;
            r.take(2)?;
            r.u32()?;
            r.s16()?;
            r.take(7)?;
        }
        w.bytes(&bytes[start..r.pos]);
    }
    r.finish()?;
    Ok(w.data)
}
pub fn validate(map: &MapFile) -> Result<()> {
    let nodes = parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let _ = script_names(&nodes)?;
    Ok(())
}
pub fn validate_waypoints(map: &MapFile) -> Result<()> {
    let objects = crate::map::objects(map.get("ObjectsList")?, &map.table)?;
    let waypoints = objects
        .iter()
        .filter_map(|o| {
            crate::map::prop(&o.props, "waypointName").and_then(crate::map::Value::text)
        })
        .collect::<BTreeSet<_>>();
    let nodes = parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    validate_wp_nodes(&nodes, &waypoints)
}
fn validate_wp_nodes(nodes: &[Node], waypoints: &BTreeSet<&str>) -> Result<()> {
    for node in nodes {
        if let Some(children) = &node.children {
            validate_wp_nodes(children, waypoints)?;
        } else if node.command.as_deref()
            == Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION")
        {
            let bytes = unhex(&node.body)?;
            let mut r = Reader::new(&bytes);
            r.take(8)?;
            let n = r.u32()?;
            for _ in 0..n {
                let ty = r.u32()?;
                if ty == 16 {
                    r.take(12)?;
                    continue;
                }
                r.take(8)?;
                let s = r.s16()?;
                if ty == 7 && s.starts_with("dual-") && !waypoints.contains(s.as_str()) {
                    return Err(err(format!("spawn script references missing waypoint {s}")));
                }
            }
        }
    }
    Ok(())
}
