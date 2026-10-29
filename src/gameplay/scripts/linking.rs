//! 将玩法规则链接到地图字符串表，并校验脚本航点。
use super::{Node, RuleChunk, Rules, encode_nodes, parse_nodes, string_arguments, unhex, validate};
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{MapFile, read_props, write_props},
    pipeline::project::Project,
};
use std::{collections::BTreeSet, fs};

pub fn load_rules(p: &Project) -> Result<Rules> {
    let key = p.string("RULESET", &p.id);
    crate::pipeline::project::safe_component(key)?;
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
pub fn validate_waypoints(map: &MapFile) -> Result<()> {
    let objects = crate::formats::map::objects(map.get("ObjectsList")?, &map.table)?;
    let waypoints = objects
        .iter()
        .filter_map(|o| {
            crate::formats::map::prop(&o.props, "waypointName")
                .and_then(crate::formats::map::Value::text)
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
            for (ty, s) in string_arguments(node)? {
                if ty == 7 && s.starts_with("dual-") && !waypoints.contains(s.as_str()) {
                    return Err(err(format!("spawn script references missing waypoint {s}")));
                }
            }
        }
    }
    Ok(())
}
