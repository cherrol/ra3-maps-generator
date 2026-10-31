//! 玩家身份与脚本资源声明的 XML 输出。
use super::{escape, text};
use crate::{
    Result, err,
    formats::binary::Reader,
    formats::map::{self, MapFile},
    gameplay::scripts::{self, Node},
};

/// Player identities in native SidesList order, also used by script lists.
pub fn player_names(map: &MapFile) -> Result<Vec<String>> {
    let c = map.get("SidesList")?;
    let mut r = Reader::new(&c.payload);
    if c.version >= 6 {
        r.u8()?;
    }
    let count = r.u32()?;
    let mut names = Vec::new();
    for _ in 0..count {
        let props = map::read_props(&mut r, &map.table)?;
        let name = text(&props, "playerName", "");
        names.push(if name.is_empty() {
            "_neutral_".into()
        } else {
            name.into()
        });
        for _ in 0..r.u32()? {
            r.s16()?;
            r.s16()?;
            r.take(16)?;
            r.take(2)?;
            r.u32()?;
            r.s16()?;
            r.take(7)?;
        }
    }
    r.finish()?;
    Ok(names)
}
fn script_xml(out: &mut String, nodes: &[Node]) -> Result<()> {
    for node in nodes {
        match node.asset.as_str() {
            "Script" | "ScriptGroup" => {
                let name = escape(&scripts::name(node)?.unwrap_or_default());
                out.push_str(&format!("<{} id=\"{name}\">", node.asset));
                script_xml(out, node.children.as_deref().unwrap_or_default())?;
                out.push_str(&format!("</{}>", node.asset));
            }
            "ScriptAction" | "ScriptActionFalse" => {
                let command = escape(
                    node.command
                        .as_deref()
                        .ok_or_else(|| err("action has no resource command"))?,
                );
                out.push_str(&format!("<Action Type=\"{command}\"><ParameterList>"));
                for (ty, value) in scripts::string_arguments(node)? {
                    if ty == 15 {
                        out.push_str(&format!("<ObjectType Object=\"{}\"/>", escape(&value)));
                    } else if ty == 61 {
                        out.push_str("<ObjectTypeList/>");
                    }
                }
                out.push_str("</ParameterList></Action>");
            }
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn refresh_script_xml(xml: &str, map: &MapFile) -> Result<String> {
    let mut out = xml.to_owned();
    while let Some(start) = out.find("<GameScriptList ") {
        let tag_end = start
            + out[start..]
                .find('>')
                .ok_or_else(|| err("unterminated script XML"))?
            + 1;
        let end = if out[start..tag_end].trim_end().ends_with("/>") {
            tag_end
        } else {
            tag_end
                + out[tag_end..]
                    .find("</GameScriptList>")
                    .ok_or_else(|| err("unterminated script list XML"))?
                + "</GameScriptList>".len()
        };
        out.replace_range(start..end, "");
    }
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut declarations = String::new();
    let players = player_names(map)?;
    write_script_lists(&mut declarations, &players, &nodes)?;
    let end = out
        .rfind("</AssetDeclaration>")
        .ok_or_else(|| err("missing XML asset declaration"))?;
    out.insert_str(end, &declarations);
    Ok(out)
}

pub(super) fn write_script_lists(
    out: &mut String,
    players: &[String],
    nodes: &[Node],
) -> Result<()> {
    for (player, node) in players.iter().zip(nodes) {
        out.push_str(&format!(
            "<GameScriptList id=\"{}\"><ScriptSet>",
            escape(player)
        ));
        script_xml(out, node.children.as_deref().unwrap_or_default())?;
        out.push_str("</ScriptSet></GameScriptList>");
    }
    Ok(())
}
