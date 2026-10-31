//! Lossless player-script extraction, with native identity and reference inventory.
use ra3maps::{
    Result, err,
    formats::{
        binary::{Reader, Writer},
        map::{Chunk, MapFile},
        sha256,
    },
    gameplay::scripts::{self, Node},
    resources,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

fn kind_name(kind: u32) -> &'static str {
    match kind {
        2 => "script_or_group",
        3 => "team",
        4 => "counter_or_timer",
        5 => "flag",
        7 => "waypoint",
        9 => "trigger_area",
        11 => "player",
        13 => "subroutine",
        14 => "named_object_or_reference",
        15 => "object_type",
        33 => "upgrade",
        34 => "command_button",
        48 => "object_list",
        54 => "object_reference",
        61 => "object_type_or_list",
        _ => "other",
    }
}

fn references(nodes: &[Node], out: &mut BTreeMap<u32, BTreeSet<String>>) -> Result<()> {
    for node in nodes {
        if let Some(children) = &node.children {
            references(children, out)?;
        } else {
            for (kind, text) in scripts::string_arguments(node)? {
                if !text.is_empty() {
                    out.entry(kind).or_default().insert(text);
                }
            }
        }
    }
    Ok(())
}

struct Source {
    map: MapFile,
    players: Vec<String>,
    lists: Vec<Chunk>,
    identity: Value,
}

fn source(map: MapFile, identity: Value) -> Result<Source> {
    let players = resources::player_names(&map)?;
    let parent = map.get("PlayerScriptsList")?;
    let mut reader = Reader::new(&parent.payload);
    let mut lists = Vec::new();
    while reader.remaining() > 0 {
        let list = Chunk::read(&mut reader)?;
        if map.table.get(&list.name_idx).map(String::as_str) != Some("ScriptList") {
            return Err(err("expected a native ScriptList for each player"));
        }
        lists.push(list);
    }
    if lists.len() != players.len() {
        return Err(err("player and script-list counts differ"));
    }
    Ok(Source {
        map,
        players,
        lists,
        identity,
    })
}

fn map_source(bytes: &[u8]) -> Result<Source> {
    source(
        MapFile::load_bytes(bytes)?,
        json!({"map_sha256":sha256::hex(bytes)}),
    )
}

fn rules_source(bytes: &[u8]) -> Result<Source> {
    let value: Value = serde_json::from_slice(bytes)?;
    let order: Vec<(String, u32)> = serde_json::from_value(value["names"].clone())?;
    let table: BTreeMap<_, _> = order.iter().map(|(name, id)| (*id, name.clone())).collect();
    if table.len() != order.len() {
        return Err(err("duplicate ruleset string-table ID"));
    }
    let mut map = MapFile {
        order,
        table,
        chunks: Vec::new(),
    };
    let version = value["SidesList"]["version"]
        .as_u64()
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| err("ruleset must declare a valid SidesList version"))?;
    let body = value["SidesList"]["body"]
        .as_str()
        .ok_or_else(|| err("ruleset must contain native SidesList data"))?;
    map.chunks.push(Chunk {
        name_idx: map.key("SidesList")?,
        version,
        payload: scripts::unhex(body)?,
    });
    let nodes: Vec<Node> = serde_json::from_value(value["scripts"].clone())?;
    let payload = scripts::encode_nodes(&nodes, &mut map)?;
    if serde_json::to_value(scripts::parse_nodes(&payload, &map.table)?)?
        != serde_json::to_value(&nodes)?
    {
        return Err(err("ruleset scripts failed their native round trip"));
    }
    map.chunks.push(Chunk {
        name_idx: map.key("PlayerScriptsList")?,
        version: 1,
        payload,
    });
    source(map, json!({"ruleset_sha256":sha256::hex(bytes)}))
}

fn extract(source: &Source, index: usize) -> Result<Value> {
    let map = &source.map;
    let player = &source.players[index];
    let mut writer = Writer::new();
    source.lists[index].write(&mut writer)?;
    let native_list = writer.data;
    let nodes = scripts::parse_nodes(&native_list, &map.table)?;
    let count = scripts::script_names(&nodes)?.len();

    // Verify actual bytes, including conditions, ordering and activation flags.
    let mut original_table = map.clone();
    if scripts::encode_nodes(&nodes, &mut original_table)? != native_list {
        return Err(err("selected scripts failed their exact native round trip"));
    }
    // Command keys must also relink without depending on the source string IDs.
    let mut fresh = MapFile {
        order: Vec::new(),
        table: BTreeMap::new(),
        chunks: Vec::new(),
    };
    let linked = scripts::encode_nodes(&nodes, &mut fresh)?;
    if serde_json::to_value(scripts::parse_nodes(&linked, &fresh.table)?)?
        != serde_json::to_value(&nodes)?
    {
        return Err(err(
            "selected scripts failed independent string-table linking",
        ));
    }
    let mut refs = BTreeMap::new();
    references(&nodes, &mut refs)?;
    let inventory: Vec<_> = refs
        .into_iter()
        .map(|(kind, values)| {
            json!({"argument_kind":kind,"meaning":kind_name(kind),"values":values})
        })
        .collect();
    let mut identity = source.identity.clone();
    let fields = identity
        .as_object_mut()
        .expect("source identity is an object");
    fields.insert("player".into(), json!(player));
    fields.insert("script_list_index".into(), json!(index));
    fields.insert(
        "script_list_sha256".into(),
        json!(sha256::hex(&native_list)),
    );
    fields.insert("script_count".into(), json!(count));
    Ok(json!({
        "schema_version":1,
        "kind":"native_player_script_extract",
        "source":identity,
        "native_text_encoding":"Latin-1 byte mapping, matching scripts::Node; decode original GBK bytes only for display",
        "references":{
            "scope":"All nonempty string arguments, including reads, writes and local definitions; not a resolved dependency graph",
            "requires_source_context":true,
            "by_argument_kind":inventory
        },
        "scripts":nodes
    }))
}

fn filename(player: &str, index: usize) -> String {
    let mut result = String::new();
    for (i, c) in player.chars().enumerate() {
        if c.is_ascii_alphanumeric() {
            if i > 0 && c.is_ascii_uppercase() && !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else if !result.is_empty() && !result.ends_with('-') {
            result.push('-');
        }
    }
    let result = result.trim_matches('-');
    if result.is_empty() {
        format!("list-{index:02}.json")
    } else {
        format!("{result}.json")
    }
}

fn all(source: &Source, directory: &Path) -> Result<Vec<(PathBuf, Value)>> {
    let mut output = Vec::new();
    let mut entries = Vec::new();
    let mut files = BTreeSet::from(["index.json".to_string()]);
    let mut assembled = Vec::new();
    let mut map = source.map.clone();
    let mut count = 0;
    for (index, player) in source.players.iter().enumerate() {
        let file = filename(player, index);
        if !files.insert(file.clone()) {
            return Err(err("player identities produce duplicate output filenames"));
        }
        let value = extract(source, index)?;
        let nodes: Vec<Node> = serde_json::from_value(value["scripts"].clone())?;
        assembled.extend(scripts::encode_nodes(&nodes, &mut map)?);
        count += scripts::script_names(&nodes)?.len();
        entries.push(json!({
            "file":file,
            "player":player,
            "script_list_index":index,
            "script_count":value["source"]["script_count"],
            "script_list_sha256":value["source"]["script_list_sha256"]
        }));
        output.push((directory.join(file), value));
    }
    let parent = source.map.get("PlayerScriptsList")?;
    if assembled != parent.payload {
        return Err(err(
            "all exported lists must reconstruct the complete source script block",
        ));
    }
    output.push((
        directory.join("index.json"),
        json!({
            "schema_version":1,
            "kind":"native_script_extract_index",
            "source":source.identity,
            "player_scripts_version":parent.version,
            "player_scripts_sha256":sha256::hex(&parent.payload),
            "source_asset_names":source.map.order,
            "script_list_count":source.lists.len(),
            "script_count":count,
            "lists":entries
        }),
    ));
    Ok(output)
}

fn encoded(value: &Value) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn run() -> Result<()> {
    let mut args: Vec<_> = env::args().skip(1).collect();
    let check = args.last().is_some_and(|arg| arg == "--check");
    if check {
        args.pop();
    }
    let rules = args.first().is_some_and(|arg| arg == "--rules");
    if args.len() != 3
        || args.iter().enumerate().any(|(index, arg)| {
            arg.starts_with('-') && !(rules && index == 0 || !rules && index == 1 && arg == "--all")
        })
    {
        return Err(err(
            "usage: extract-player-scripts <source.map> <player|--all> <output> [--check]; or --rules <rules.json> <output-directory> [--check]",
        ));
    }
    let input = Path::new(&args[usize::from(rules)]);
    let bytes = fs::read(input)?;
    let source = if rules {
        rules_source(&bytes)?
    } else {
        map_source(&bytes)?
    };
    let batch = rules || args[1] == "--all";
    let output = if batch {
        all(&source, Path::new(&args[2]))?
    } else {
        let indices: Vec<_> = source
            .players
            .iter()
            .enumerate()
            .filter(|(_, player)| player.as_str() == args[1])
            .map(|(index, _)| index)
            .collect();
        let [index] = indices.as_slice() else {
            return Err(err("source must contain exactly one matching player"));
        };
        vec![(PathBuf::from(&args[2]), extract(&source, *index)?)]
    };
    // Prepare and check every artifact before writing any, including empty lists.
    let mut prepared = Vec::new();
    for (path, value) in &output {
        if path.exists() && fs::canonicalize(input)? == fs::canonicalize(path)? {
            return Err(err("output must not replace the source input"));
        }
        let bytes = encoded(value)?;
        if check && fs::read(path)? != bytes {
            return Err(err(format!(
                "saved scripts differ from the source extraction: {}",
                path.display()
            )));
        }
        prepared.push((path, bytes));
    }
    if !check {
        for (path, bytes) in prepared {
            if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
                fs::create_dir_all(parent)?;
            }
            if fs::read(path).ok().as_deref() != Some(bytes.as_slice()) {
                fs::write(path, bytes)?;
            }
        }
    }
    let count: usize = output
        .iter()
        .filter(|(_, value)| value["kind"] == "native_player_script_extract")
        .map(|(_, value)| value["source"]["script_count"].as_u64().unwrap() as usize)
        .sum();
    println!(
        "{} {}: {} scripts, {} artifacts at {} (native round trip, independent linking{} verified)",
        if check { "Checked" } else { "Extracted" },
        input.display(),
        count,
        output.len(),
        args[2],
        if batch {
            " and full-block reconstruction"
        } else {
            ""
        }
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("extract-player-scripts: {error}");
        std::process::exit(1);
    }
}
