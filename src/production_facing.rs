//! Gifted production buildings face the opposing team's actual starting region.
use crate::{
    Result,
    binary::Reader,
    err,
    map::{self, MapFile, Value, prop},
    project::Project,
    scripts::{self, Node},
};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

const COMMAND: &str = "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION";
const DIRECTION: &str = "opposing_team_center";

fn enabled(p: &Project) -> Result<bool> {
    let common: Json = serde_json::from_slice(&std::fs::read(
        p.root.join("templates/rules/common-T0.5.1.json"),
    )?)?;
    if common["production_placement"]["facing"].as_str() != Some(DIRECTION) {
        return Err(err(
            "public production facing must use opposing_team_center",
        ));
    }
    Ok(true)
}

fn production(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    ["barracks", "warfactory", "airfield", "navalyard"]
        .iter()
        .any(|suffix| kind.ends_with(suffix))
}

fn seat(team: &str) -> Result<usize> {
    let owner = team.split('/').next().unwrap().trim_start_matches("team");
    match owner {
        "PlyrCivilian" => Ok(7),
        "PlyrCreeps" => Ok(8),
        _ => owner
            .strip_prefix("Player_")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|s| (1..=8).contains(s))
            .ok_or_else(|| err(format!("unknown production gift owner {team}"))),
    }
}

/// The native ANGLE argument and map object headings both store radians.
fn angle_offset(n: &Node) -> Result<usize> {
    let bytes = scripts::unhex(&n.body)?;
    let mut r = Reader::new(&bytes);
    if n.command.as_deref() != Some(COMMAND) || r.u32()? != 467 {
        return Err(err(
            "production gift requires native oriented spawn command 467",
        ));
    }
    r.take(4)?;
    if r.u32()? != 4 {
        return Err(err("oriented production gift requires four arguments"));
    }
    let mut angle = 0;
    for expected in [15, 3, 7, 17] {
        if r.u32()? != expected {
            return Err(err("invalid production gift argument type"));
        }
        r.i32()?;
        if expected == 17 {
            angle = r.pos;
        }
        r.f32()?;
        r.s16()?;
    }
    Ok(angle)
}

struct Context {
    points: BTreeMap<String, [f64; 2]>,
    targets: [[f64; 2]; 8],
}
impl Context {
    fn load(p: &Project, map: &MapFile) -> Result<Self> {
        let mut points = BTreeMap::new();
        for o in map::objects(map.get("ObjectsList")?, &map.table)? {
            if let Some(name) = prop(&o.props, "waypointName").and_then(Value::text)
                && points
                    .insert(
                        name.to_owned(),
                        [o.position[0] as f64, o.position[1] as f64],
                    )
                    .is_some()
            {
                return Err(err(format!("duplicate facing waypoint {name}")));
            }
        }
        let bases = p.cfg["BASES"]
            .as_array()
            .ok_or_else(|| err("missing facing teams"))?;
        if bases.len() != 8 {
            return Err(err("production facing requires eight seats"));
        }
        let groups = bases
            .iter()
            .map(|b| b[3].as_str().ok_or_else(|| err("missing base team")))
            .collect::<Result<Vec<_>>>()?;
        let mut starts = Vec::new();
        for seat in 1..=8 {
            starts.push(
                *points
                    .get(&format!("Player_{seat}_Start"))
                    .or_else(|| points.get(&format!("Player_{seat}_Start (imported)")))
                    .ok_or_else(|| err(format!("missing facing start P{seat}")))?,
            );
        }
        let distinct: std::collections::BTreeSet<_> = groups.iter().collect();
        if distinct.len() != 2 {
            return Err(err("production facing requires exactly two teams"));
        }
        let mut targets = [[0.; 2]; 8];
        for seat in 0..8 {
            let enemies = starts
                .iter()
                .zip(&groups)
                .filter(|(_, g)| **g != groups[seat])
                .map(|(s, _)| *s)
                .collect::<Vec<_>>();
            for axis in 0..2 {
                targets[seat][axis] =
                    enemies.iter().map(|s| s[axis]).sum::<f64>() / enemies.len() as f64;
            }
        }
        Ok(Self { points, targets })
    }
    fn visit(&self, nodes: &mut [Node], change: bool, entries: &mut Vec<Json>) -> Result<()> {
        for n in nodes {
            if let Some(c) = &mut n.children {
                self.visit(c, change, entries)?;
                continue;
            }
            if !n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                continue;
            }
            let args = scripts::string_arguments(n)?;
            let Some((_, kind)) = args.iter().find(|(t, name)| *t == 15 && production(name)) else {
                continue;
            };
            let arg = |ty| {
                args.iter()
                    .find(|(t, _)| *t == ty)
                    .map(|(_, s)| s.as_str())
                    .ok_or_else(|| err("missing production facing argument"))
            };
            let seat = seat(arg(3)?)?;
            let waypoint = arg(7)?;
            let point = self
                .points
                .get(waypoint)
                .ok_or_else(|| err(format!("missing production gift point {waypoint}")))?;
            let target = self.targets[seat - 1];
            let heading = heading(*point, target)?;
            let offset = angle_offset(n)?;
            let mut body = scripts::unhex(&n.body)?;
            let previous = f32::from_le_bytes(body[offset..offset + 4].try_into()?);
            if !change && (!previous.is_finite() || (previous - heading).abs() > 0.00001) {
                return Err(err(format!(
                    "production gift faces away from enemy team: {waypoint}/{kind}"
                )));
            }
            if change {
                body[offset..offset + 4].copy_from_slice(&heading.to_le_bytes());
                n.body = scripts::hex(&body);
            }
            entries.push(json!({"kind":kind,"seat":seat,"waypoint":waypoint,"position":point,"opposing_team_center":target,"from_radians":previous,"radians":heading,"changed":previous.to_bits()!=heading.to_bits()}));
        }
        Ok(())
    }
}

fn heading(from: [f64; 2], target: [f64; 2]) -> Result<f32> {
    let dx = target[0] - from[0];
    let dy = target[1] - from[1];
    if !dx.is_finite() || !dy.is_finite() || dx.hypot(dy) < 1. {
        return Err(err("production gift has no distinct opposing direction"));
    }
    Ok(dy.atan2(dx) as f32)
}

pub fn apply(p: &Project, map: &mut MapFile) -> Result<Json> {
    enabled(p)?;
    let context = Context::load(p, map)?;
    let mut nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut entries = Vec::new();
    context.visit(&mut nodes, true, &mut entries)?;
    let payload = scripts::encode_nodes(&nodes, map)?;
    map.get_mut("PlayerScriptsList")?.payload = payload;
    validate(p, map)?;
    Ok(
        json!({"direction":DIRECTION,"team_source":"BASES[seat][3]","coordinates":"actual map waypoints in world coordinates","units":"radians","actions":entries,"ingame_test":"not run"}),
    )
}

pub fn validate(p: &Project, map: &MapFile) -> Result<()> {
    enabled(p)?;
    let mut nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    Context::load(p, map)?.visit(&mut nodes, false, &mut Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_world_headings_cover_all_four_axes_without_flipping_north() {
        for (target, expected) in [
            ([100., 0.], 0.),
            ([0., 100.], std::f32::consts::FRAC_PI_2),
            ([-100., 0.], std::f32::consts::PI),
            ([0., -100.], -std::f32::consts::FRAC_PI_2),
        ] {
            assert!((heading([0., 0.], target).unwrap() - expected).abs() < 0.00001);
        }
        assert!(heading([0., 0.], [0., 0.]).is_err());
    }
    #[test]
    fn only_production_types_and_valid_gift_owners_are_selected() {
        for t in [
            "AlliedBarracks",
            "sovietwarfactory",
            "JapanAirfield",
            "AlliedNavalYard",
        ] {
            assert!(production(t));
        }
        for t in [
            "SovietPowerPlant",
            "AlliedSuperWeapon",
            "SovietConstructionYard",
            "SovietWarFactory_Collapse",
        ] {
            assert!(!production(t));
        }
        assert_eq!(seat("Player_1/teamPlayer_1").unwrap(), 1);
        assert_eq!(seat("teamPlayer_6").unwrap(), 6);
        assert_eq!(seat("PlyrCivilian/teamPlyrCivilian").unwrap(), 7);
        assert_eq!(seat("teamPlyrCreeps").unwrap(), 8);
        assert!(seat("Player_9/team").is_err());
    }
}
