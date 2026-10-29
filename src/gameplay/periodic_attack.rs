//! 按地图配置定时统计兵力，并向 AI 发送对位进攻请求。
use crate::{
    Result, err,
    formats::binary::{Reader, Writer},
    formats::map::{self, MapFile, Property, Value, prop},
    gameplay::scripts::{self, Node},
    pipeline::project::Project,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};
const PREFIX: &str = "minute-attack-";
const TEAM: &str = "team_minute_attack_";
const TIMER: &str = "minute-attack-clock";
const ROSTER: &str = "minute-attack-military";
const ROUND: &str = "minute-attack-round";
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub interval_seconds: u32,
    pub minimum_units: i32,
    pub pairs: Vec<[usize; 2]>,
    pub unit_types: Vec<String>,
    #[serde(default)]
    pub diagnostics: bool,
}
impl Config {
    fn validate(&self) -> Result<()> {
        if self.interval_seconds != 60 || self.minimum_units != 20 {
            return Err(err(
                "paired attacks require a 60-second interval and >=20 troops",
            ));
        }
        let mut seats = BTreeSet::new();
        for pair in &self.pairs {
            if pair[0] == pair[1] {
                return Err(err("attack pair cannot target itself"));
            }
            for s in pair {
                if !(1..=8).contains(s) || !seats.insert(*s) {
                    return Err(err("attack pairs must cover eight distinct seats"));
                }
            }
        }
        if seats.len() != 8 {
            return Err(err("attack pairs must cover all eight seats"));
        }
        let stock: Vec<String> = serde_json::from_str(include_str!(
            "../../templates/resources/stock-global-objects.json"
        ))?;
        let mut types = BTreeSet::new();
        for t in &self.unit_types {
            if !stock.contains(t)
                || !types.insert(t)
                || [
                    "miner",
                    "engineer",
                    "mcv",
                    "surveyor",
                    "outpost",
                    "constructionyard",
                    "refinery",
                ]
                .iter()
                .any(|s| t.to_ascii_lowercase().contains(s))
            {
                return Err(err(format!("invalid or noncombat attack unit type {t}")));
            }
        }
        if types.is_empty() {
            return Err(err("attack roster is empty"));
        }
        Ok(())
    }
    pub fn targets(&self) -> Result<[usize; 8]> {
        self.validate()?;
        let mut out = [0; 8];
        for [a, b] in &self.pairs {
            out[a - 1] = *b;
            out[b - 1] = *a;
        }
        Ok(out)
    }
}
fn player(seat: usize) -> String {
    match seat {
        7 => "PlyrCivilian".into(),
        8 => "PlyrCreeps".into(),
        _ => format!("Player_{seat}"),
    }
}
#[derive(Clone)]
struct Arg(u32, i32, f32, String);
fn text(t: u32, s: impl Into<String>) -> Arg {
    Arg(t, 0, 0., s.into())
}
fn int(t: u32, i: i32) -> Arg {
    Arg(t, i, 0., String::new())
}
fn real(v: f32) -> Arg {
    Arg(1, 0, v, String::new())
}
fn leaf(command: &str, opcode: u32, args: &[Arg], condition: bool) -> Result<Node> {
    #[derive(Deserialize)]
    struct Signature {
        kind: String,
        opcode: u32,
        argument_types: Vec<u32>,
    }
    #[derive(Deserialize)]
    struct Registry {
        commands: BTreeMap<String, Signature>,
    }
    let registry: Registry = serde_json::from_str(include_str!(
        "../../templates/rules/periodic-command-signatures.json"
    ))?;
    let signature = registry
        .commands
        .get(command)
        .ok_or_else(|| err(format!("unverified periodic command {command}")))?;
    if signature.opcode != opcode
        || signature.kind != if condition { "condition" } else { "action" }
        || signature.argument_types != args.iter().map(|a| a.0).collect::<Vec<_>>()
    {
        return Err(err(format!(
            "periodic command signature mismatch: {command}"
        )));
    }
    let arguments = args
        .iter()
        .map(|Arg(t, i, f, s)| (*t, *i, *f, s.as_str()))
        .collect::<Vec<_>>();
    scripts::leaf(command, opcode, &arguments, condition, false)
}
fn action(command: &str, id: u32, args: &[Arg]) -> Result<Node> {
    leaf(command, id, args, false)
}
fn condition(command: &str, id: u32, args: &[Arg]) -> Result<Node> {
    leaf(command, id, args, true)
}
fn script(suffix: &str, once: bool, conditions: Vec<Node>, actions: Vec<Node>) -> Result<Node> {
    executable(suffix, once, false, conditions, actions, Vec::new())
}
fn executable(
    suffix: &str,
    once: bool,
    subroutine: bool,
    conditions: Vec<Node>,
    actions: Vec<Node>,
    otherwise: Vec<Node>,
) -> Result<Node> {
    let mut w = Writer::new();
    w.s16(&format!("{PREFIX}{suffix}"))?;
    for _ in 0..3 {
        w.s16("")?;
    }
    w.bytes(&[1, u8::from(once), 1, 1, 1, u8::from(subroutine)]);
    w.u32(if subroutine { 0 } else { 1 });
    w.bytes(&[0, 0]);
    w.u32(0);
    w.u8(1);
    w.s16("")?;
    w.s16("X")?;
    let mut children = vec![Node {
        asset: "OrCondition".into(),
        version: 1,
        prefix: String::new(),
        body: String::new(),
        command: None,
        children: Some(conditions),
    }];
    children.extend(actions);
    children.extend(otherwise.into_iter().map(|mut n| {
        n.asset = "ScriptActionFalse".into();
        n
    }));
    Ok(Node {
        asset: "Script".into(),
        version: 4,
        prefix: scripts::hex(&w.data),
        body: String::new(),
        command: None,
        children: Some(children),
    })
}
fn expired() -> Result<Node> {
    condition("TIMER_EXPIRED", 4, &[text(4, TIMER)])
}
fn restart(c: &Config) -> Result<Node> {
    action(
        "SET_MILLISECOND_TIMER",
        21,
        &[text(4, TIMER), real(c.interval_seconds as f32)],
    )
}
fn waypoints(map: &MapFile) -> Result<[String; 8]> {
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut names: [String; 8] = std::array::from_fn(|_| String::new());
    for seat in 1..=8 {
        names[seat - 1] = objects
            .iter()
            .filter_map(|o| prop(&o.props, "waypointName").and_then(Value::text))
            .find(|n| {
                *n == format!("Player_{seat}_Start")
                    || *n == format!("Player_{seat}_Start (imported)")
            })
            .ok_or_else(|| err(format!("missing attack target P{seat}")))?
            .into();
    }
    Ok(names)
}
fn truth() -> Result<Node> {
    condition("CONDITION_TRUE", 3, &[])
}
fn requests(seat: usize) -> String {
    format!("{PREFIX}requests-p{seat}")
}
fn call(suffix: &str) -> Result<Node> {
    action(
        "CALL_SUBROUTINE",
        10,
        &[text(13, format!("{PREFIX}{suffix}"))],
    )
}
// These requests go to the player's AI. No team or unit is ever taken over.
fn generated(c: &Config, waypoints: &[String; 8]) -> Result<Vec<Node>> {
    let targets = c.targets()?;
    let mut boot = vec![action("SET_COUNTER", 2, &[text(4, ROUND), int(0, 0)])?];
    for unit in &c.unit_types {
        boot.push(action(
            "OBJECTLIST_ADDOBJECTTYPE",
            216,
            &[text(15, unit.to_ascii_lowercase()), text(48, ROSTER)],
        )?);
    }
    for seat in 1..=8 {
        for counter in [format!("{PREFIX}count-p{seat}"), requests(seat)] {
            boot.push(action("SET_COUNTER", 2, &[text(4, counter), int(0, 0)])?);
        }
    }
    if c.diagnostics {
        for (counter, _) in diagnostic_labels() {
            boot.push(action(
                "DISPLAY_COUNTER",
                188,
                &[text(4, &counter), text(25, format!("COUNTER:{counter}"))],
            )?);
        }
    }
    boot.push(restart(c)?);
    let mut out = vec![script("boot", true, vec![truth()?], boot)?];
    for seat in 1..=8 {
        let count = format!("{PREFIX}count-p{seat}");
        out.push(executable(
            &format!("p{seat}-count"),
            false,
            true,
            vec![truth()?],
            vec![action(
                "SET_PLAYER_OWNERSHIP_OF_TYPE_COUNTER",
                317,
                &[text(61, ROSTER), text(11, player(seat)), text(4, &count)],
            )?],
            Vec::new(),
        )?);
        out.push(executable(
            &format!("p{seat}-request"),
            false,
            true,
            vec![condition(
                "COUNTER",
                1,
                &[text(4, count), int(6, 3), int(0, c.minimum_units)],
            )?],
            vec![
                action(
                    "PLACE_AI_BEACON_LOCATION",
                    571,
                    &[
                        text(11, player(seat)),
                        text(103, "Assault"),
                        text(7, &waypoints[targets[seat - 1] - 1]),
                    ],
                )?,
                action(
                    "COUNTER_MATH_VALUE",
                    292,
                    &[text(4, requests(seat)), int(57, 0), int(0, 1)],
                )?,
            ],
            // Below threshold: no request, no change to the AI's current orders.
            Vec::new(),
        )?);
    }
    let mut clock = vec![action(
        "COUNTER_MATH_VALUE",
        292,
        &[text(4, ROUND), int(57, 0), int(0, 1)],
    )?];
    for seat in 1..=8 {
        clock.push(call(&format!("p{seat}-count"))?);
        clock.push(call(&format!("p{seat}-request"))?);
    }
    clock.push(restart(c)?);
    out.push(script("clock", false, vec![expired()?], clock)?);
    Ok(out)
}
fn diagnostic_labels() -> Vec<(String, String)> {
    let mut out = vec![(ROUND.into(), "进攻检查轮次".into())];
    for seat in [7, 8] {
        out.push((
            format!("{PREFIX}count-p{seat}"),
            format!("P{seat} 作战兵力"),
        ));
        out.push((requests(seat), format!("P{seat} AI进攻指令累计")));
    }
    out
}
pub fn append_strings(p: &Project, strings: &mut String) -> Result<()> {
    let config = scripts::load_rules(p)?.periodic_attack;
    if config.is_none() && !strings.contains(&format!("COUNTER:{PREFIX}")) {
        return Ok(());
    }
    // Refresh attack counters and remove obsolete team-order diagnostics.
    let mut clean = String::new();
    let mut skip = false;
    for line in strings.lines() {
        if line.starts_with(&format!("COUNTER:{PREFIX}")) {
            skip = true;
        }
        if skip {
            if line == "End" {
                skip = false;
            }
        } else {
            clean.push_str(line);
            clean.push('\n');
        }
    }
    clean.truncate(clean.trim_end().len());
    clean.push('\n');
    if config.is_some_and(|c| c.diagnostics) {
        for (counter, label) in diagnostic_labels() {
            clean.push_str(&format!("\nCOUNTER:{counter}\n\"{label}\"\nEnd\n"));
        }
    }
    *strings = clean;
    Ok(())
}
fn teams(map: &MapFile) -> Result<Vec<Vec<Property>>> {
    let mut r = Reader::new(&map.get("Teams")?.payload);
    let mut out = Vec::new();
    for _ in 0..r.u32()? {
        out.push(map::read_props(&mut r, &map.table)?);
    }
    r.finish()?;
    Ok(out)
}
fn write_teams(map: &mut MapFile, teams: &[Vec<Property>]) -> Result<()> {
    for team in teams {
        for property in team {
            map.id(&property.key)?;
        }
    }
    let mut w = Writer::new();
    w.u32(teams.len() as u32);
    for team in teams {
        map::write_props(&mut w, team, map)?;
    }
    map.get_mut("Teams")?.payload = w.data;
    Ok(())
}
pub fn authored_scripts(nodes: &[Node]) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    for n in nodes {
        if scripts::name(n)?.is_some_and(|s| s.starts_with(PREFIX)) {
            continue;
        }
        let mut n = n.clone();
        if let Some(children) = &n.children {
            n.children = Some(authored_scripts(children)?);
        }
        out.push(n);
    }
    Ok(out)
}
pub fn authored_teams(map: &mut MapFile) -> Result<()> {
    let original = teams(map)?
        .into_iter()
        .filter(|p| {
            !prop(p, "teamName")
                .and_then(Value::text)
                .unwrap_or("")
                .starts_with(TEAM)
        })
        .collect::<Vec<_>>();
    write_teams(map, &original)
}
pub fn apply(p: &Project, map: &mut MapFile) -> Result<Option<Json>> {
    let Some(c) = scripts::load_rules(p)?.periodic_attack else {
        let parsed = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
        let nodes = authored_scripts(&parsed)?;
        let removed = scripts::script_names(&parsed)?.len() - scripts::script_names(&nodes)?.len();
        let old_teams = map.get("Teams")?.payload.clone();
        authored_teams(map)?;
        let teams_changed = map.get("Teams")?.payload != old_teams;
        if removed == 0 && !teams_changed {
            return Ok(None);
        }
        map.get_mut("PlayerScriptsList")?.payload = scripts::encode_nodes(&nodes, map)?;
        validate(p, map)?;
        return Ok(Some(json!({
            "enabled": false,
            "scripts_removed": removed,
            "scripts_added": 0,
            "legacy_teams_removed": teams_changed,
            "diagnostics": false,
            "ingame_test": "not run"
        })));
    };
    c.validate()?;
    let wp = waypoints(map)?;
    let scripts = generated(&c, &wp)?;
    let parsed =
        crate::gameplay::scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut nodes = authored_scripts(&parsed)?;
    let root = nodes
        .first_mut()
        .and_then(|n| n.children.as_mut())
        .ok_or_else(|| err("map has no global script list"))?;
    root.extend(scripts);
    // Remove only this module's legacy attack teams, preserving authored teams.
    authored_teams(map)?;
    let payload = crate::gameplay::scripts::encode_nodes(&nodes, map)?;
    map.get_mut("PlayerScriptsList")?.payload = payload;
    validate(p, map)?;
    Ok(Some(
        json!({"interval_seconds":c.interval_seconds,"minimum_units":c.minimum_units,"comparison":">=","human_player_check":false,"pairs":c.pairs,"targets":c.targets()?,"counted_types":c.unit_types,"excluded":"buildings, harvesters, engineers, MCVs and outposts","under_threshold":"no request; AI retains control and current orders","scripts_added":18,"native_true_opcode":3,"object_lists_initialized":true,"synchronous_census_and_decision":true,"diagnostics":c.diagnostics,"attack_teams":0,"unit_recruitment":false,"control_transfers":false,"ai_retains_control":true,"request_command":"PLACE_AI_BEACON_LOCATION","beacon_type":"Assault","target":"paired seat start waypoint","ingame_test":"not run"}),
    ))
}
pub fn validate(p: &Project, map: &MapFile) -> Result<()> {
    let expected = if let Some(c) = scripts::load_rules(p)?.periodic_attack {
        generated(&c, &waypoints(map)?)?
    } else {
        Vec::new()
    };
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut actual = Vec::new();
    fn collect(nodes: &[Node], out: &mut Vec<Node>) -> Result<()> {
        for n in nodes {
            if scripts::name(n)?.is_some_and(|s| s.starts_with(PREFIX)) {
                out.push(n.clone());
            } else if let Some(c) = &n.children {
                collect(c, out)?;
            }
        }
        Ok(())
    }
    collect(&nodes, &mut actual)?;
    if serde_json::to_value(&actual)? != serde_json::to_value(&expected)? {
        return Err(err("paired attack scripts differ from the map recipe"));
    }
    let actual = teams(map)?
        .into_iter()
        .filter(|p| {
            prop(p, "teamName")
                .and_then(Value::text)
                .unwrap_or("")
                .starts_with(TEAM)
        })
        .collect::<Vec<_>>();
    if !actual.is_empty() {
        return Err(err("legacy minute-attack teams must be removed"));
    }
    scripts::validate_waypoints(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        Config {
            interval_seconds: 60,
            minimum_units: 20,
            pairs: vec![[1, 5], [2, 7], [3, 8], [4, 6]],
            unit_types: vec!["SovietAntiVehicleVehicleTech1".into()],
            diagnostics: false,
        }
    }
    fn nodes() -> Vec<Node> {
        generated(&config(), &std::array::from_fn(|i| format!("p{}", i + 1))).unwrap()
    }
    fn args(n: &Node) -> Vec<Arg> {
        let bytes = scripts::unhex(&n.body).unwrap();
        let mut r = Reader::new(&bytes);
        r.take(8).unwrap();
        let count = r.u32().unwrap();
        (0..count)
            .map(|_| {
                Arg(
                    r.u32().unwrap(),
                    r.i32().unwrap(),
                    r.f32().unwrap(),
                    r.s16().unwrap(),
                )
            })
            .collect()
    }
    fn leaves<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>) {
        for n in nodes {
            if n.command.is_some() {
                out.push(n);
            }
            if let Some(c) = &n.children {
                leaves(c, out);
            }
        }
    }
    #[test]
    fn pairings_are_exact_reciprocal_and_cover_every_seat() {
        assert_eq!(config().targets().unwrap(), [5, 7, 8, 6, 1, 4, 2, 3]);
        let mut c = config();
        c.pairs[1] = [2, 5];
        assert!(c.targets().is_err());
    }
    #[test]
    fn boot_truth_matches_original_binary_and_invalid_native_signatures_fail() {
        let original: Json =
            serde_json::from_str(include_str!("../../templates/rules/common-T0.5.1.json")).unwrap();
        let nodes: Vec<Node> = serde_json::from_value(original["scripts"].clone()).unwrap();
        let mut all = vec![];
        leaves(&nodes, &mut all);
        let original = all
            .iter()
            .find(|n| n.command.as_deref() == Some("CONDITION_TRUE"))
            .unwrap();
        assert_eq!(truth().unwrap().body, original.body);
        assert!(condition("CONDITION_TRUE", 0, &[]).is_err());
        assert!(
            action(
                "PLACE_AI_BEACON_LOCATION",
                571,
                &[text(3, "team"), text(103, "Assault"), text(7, "p5")]
            )
            .is_err()
        );
    }
    #[test]
    fn periodic_requests_are_player_assault_beacons_to_exact_paired_waypoints() {
        let nodes = nodes();
        assert_eq!(nodes.len(), 18);
        for seat in 1..=8 {
            let request = nodes[seat * 2].children.as_ref().unwrap();
            assert_eq!(
                request[1].command.as_deref(),
                Some("PLACE_AI_BEACON_LOCATION")
            );
            let a = args(&request[1]);
            assert_eq!(a.iter().map(|a| a.0).collect::<Vec<_>>(), vec![11, 103, 7]);
            assert_eq!(a[0].3, player(seat));
            assert_eq!(a[1].3, "Assault");
            assert_eq!(
                a[2].3,
                format!("p{}", config().targets().unwrap()[seat - 1])
            );
            assert_eq!(args(&request[2])[0].3, requests(seat));
        }
    }
    #[test]
    fn nineteen_does_nothing_and_twenty_requests_without_any_unit_or_team_control() {
        let nodes = nodes();
        let mut all = vec![];
        leaves(&nodes, &mut all);
        let allowed = [
            "CONDITION_TRUE",
            "TIMER_EXPIRED",
            "COUNTER",
            "SET_COUNTER",
            "SET_MILLISECOND_TIMER",
            "OBJECTLIST_ADDOBJECTTYPE",
            "SET_PLAYER_OWNERSHIP_OF_TYPE_COUNTER",
            "COUNTER_MATH_VALUE",
            "CALL_SUBROUTINE",
            "PLACE_AI_BEACON_LOCATION",
        ];
        for leaf in all {
            assert!(allowed.contains(&leaf.command.as_deref().unwrap()));
            assert!(
                !args(leaf).iter().any(|a| [3, 14].contains(&a.0)),
                "no Team or Unit arguments are allowed"
            );
        }
        for seat in 1..=8 {
            let request = nodes[seat * 2].children.as_ref().unwrap();
            let cond = args(&request[0].children.as_ref().unwrap()[0]);
            assert_eq!((cond[1].1, cond[2].1), (3, 20));
            for count in [0, 19, 20, 21, 50] {
                assert_eq!(count >= cond[2].1, count >= 20);
            }
            assert!(!request.iter().any(|n| n.asset == "ScriptActionFalse"));
        }
    }
    #[test]
    fn clock_counts_then_requests_before_restarting_and_subroutines_are_not_polled() {
        let nodes = nodes();
        for node in &nodes[1..17] {
            let bytes = scripts::unhex(&node.prefix).unwrap();
            let mut r = Reader::new(&bytes);
            for _ in 0..4 {
                r.s16().unwrap();
            }
            assert_eq!(r.take(6).unwrap(), &[1, 0, 1, 1, 1, 1]);
            assert_eq!(r.u32().unwrap(), 0);
        }
        let clock = nodes[17].children.as_ref().unwrap();
        assert_eq!(
            clock[0].children.as_ref().unwrap()[0].command.as_deref(),
            Some("TIMER_EXPIRED")
        );
        let calls: Vec<_> = clock
            .iter()
            .filter(|n| n.command.as_deref() == Some("CALL_SUBROUTINE"))
            .map(|n| args(n)[0].3.clone())
            .collect();
        let expected: Vec<_> = (1..=8)
            .flat_map(|s| {
                [
                    format!("minute-attack-p{s}-count"),
                    format!("minute-attack-p{s}-request"),
                ]
            })
            .collect();
        assert_eq!(calls, expected);
        assert_eq!(
            clock.last().unwrap().command.as_deref(),
            Some("SET_MILLISECOND_TIMER")
        );
        assert_eq!(args(clock.last().unwrap())[1].2, 60.);
    }
    #[test]
    fn census_list_is_initialized_and_diagnostics_count_requests_instead_of_teams() {
        let mut c = config();
        c.diagnostics = true;
        let nodes = generated(&c, &std::array::from_fn(|i| format!("p{}", i + 1))).unwrap();
        let boot = nodes[0].children.as_ref().unwrap();
        let roster = boot
            .iter()
            .find(|n| n.command.as_deref() == Some("OBJECTLIST_ADDOBJECTTYPE"))
            .unwrap();
        assert_eq!(args(roster)[1].3, ROSTER);
        assert_eq!(args(&nodes[1].children.as_ref().unwrap()[1])[0].3, ROSTER);
        let labels: Vec<_> = boot
            .iter()
            .filter(|n| n.command.as_deref() == Some("DISPLAY_COUNTER"))
            .map(|n| args(n)[0].3.clone())
            .collect();
        assert_eq!(labels.len(), 5);
        assert!(labels.contains(&requests(7)));
        assert!(!labels.iter().any(|s| s.contains("orders")));
    }
}
