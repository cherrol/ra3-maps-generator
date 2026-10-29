//! 将赠送建筑朝向对方队伍中心，并吸附到四个允许角度。
use crate::{
    Result, err,
    formats::binary::Reader,
    formats::map::{self, MapFile, Value, prop},
    gameplay::scripts::{self, Node},
    pipeline::project::Project,
};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

const COMMAND: &str = "CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION";
const DIRECTION: &str = "opposing_team_center";
const ALLOWED_DEGREES: [i32; 4] = [45, 135, -45, -135];

fn enabled(p: &Project) -> Result<bool> {
    let common: Json = serde_json::from_slice(&std::fs::read(
        p.root.join("templates/rules/common-T0.5.1.json"),
    )?)?;
    if common["production_placement"]["facing"].as_str() != Some(DIRECTION) {
        return Err(err(
            "public production facing must use opposing_team_center",
        ));
    }
    if common["production_placement"]["allowed_angles_degrees"] != json!(ALLOWED_DEGREES) {
        return Err(err(
            "gift placement angles must be 45, 135, -45, -135 degrees",
        ));
    }
    Ok(true)
}

fn gifted_building(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    [
        "barracks",
        "warfactory",
        "airfield",
        "navalyard",
        "powerplant",
    ]
    .iter()
    .any(|suffix| kind.ends_with(suffix))
        || kind.contains("superweapon")
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
    let arguments = scripts::scalar_arguments(n)?;
    if arguments.len() != 4 {
        return Err(err("invalid production gift argument type"));
    }
    let mut angle = 0;
    for (argument, expected) in arguments.iter().zip([15, 3, 7, 17]) {
        if argument.kind != expected {
            return Err(err("invalid production gift argument type"));
        }
        if expected == 17 {
            angle = argument.real_offset;
        }
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
            let Some((_, kind)) = args
                .iter()
                .find(|(t, name)| *t == 15 && gifted_building(name))
            else {
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
            if !change && previous != heading {
                return Err(err(format!(
                    "gift heading must use the nearest legal angle toward the enemy team: {waypoint}/{kind}"
                )));
            }
            if change {
                body[offset..offset + 4].copy_from_slice(&heading.to_le_bytes());
                n.body = scripts::hex(&body);
            }
            entries.push(json!({"kind":kind,"seat":seat,"waypoint":waypoint,"position":point,"opposing_team_center":target,"from_radians":previous,"radians":heading,"degrees":heading.to_degrees().round() as i32,"changed":previous.to_bits()!=heading.to_bits()}));
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
    // Each quadrant's diagonal is the nearest legal heading. On an axis,
    // choose the nonnegative component's quadrant consistently, including -0.
    let degrees: f32 = match (dx >= 0., dy >= 0.) {
        (true, true) => 45.,
        (false, true) => 135.,
        (true, false) => -45.,
        (false, false) => -135.,
    };
    Ok(degrees.to_radians())
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
        json!({"direction":DIRECTION,"allowed_angles_degrees":ALLOWED_DEGREES,"selection":"nearest legal angle toward opposing_team_center","team_source":"BASES[seat][3]","coordinates":"actual map waypoints in world coordinates","units":"radians","actions":entries,"ingame_test":"not run"}),
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
    fn native_gift_actions_reject_illegal_or_wrong_headings_and_only_patch_the_angle() {
        use crate::formats::binary::Writer;
        let context = Context {
            points: BTreeMap::from([("gift-alt-test".into(), [10., 20.])]),
            targets: [[110., 40.]; 8],
        };
        for kind in [
            "AlliedBarracks",
            "SovietPowerPlant",
            "JapanSuperWeaponAdvanced",
        ] {
            for invalid in [
                0.,
                135_f32.to_radians(),
                45_f32.to_radians() + 0.000001,
                f32::NAN,
            ] {
                let mut w = Writer::new();
                w.u32(467);
                w.u32(3);
                w.u32(4);
                for (ty, value, angle) in [
                    (15, kind, 0.),
                    (3, "Player_1/teamPlayer_1", 0.),
                    (7, "gift-alt-test", 0.),
                    (17, "", invalid),
                ] {
                    w.u32(ty);
                    w.i32(0);
                    w.f32(angle);
                    w.s16(value).unwrap();
                }
                w.u32(1);
                let mut nodes = [Node {
                    asset: "ScriptAction".into(),
                    version: 3,
                    prefix: String::new(),
                    body: scripts::hex(&w.data),
                    command: Some(COMMAND.into()),
                    children: None,
                }];
                assert!(context.visit(&mut nodes, false, &mut Vec::new()).is_err());
                context.visit(&mut nodes, true, &mut Vec::new()).unwrap();
                context.visit(&mut nodes, false, &mut Vec::new()).unwrap();
                let offset = angle_offset(&nodes[0]).unwrap();
                w.data[offset..offset + 4].copy_from_slice(&45_f32.to_radians().to_le_bytes());
                let expected = scripts::hex(&w.data);
                assert_eq!(nodes[0].body, expected);
                context.visit(&mut nodes, true, &mut Vec::new()).unwrap();
                assert_eq!(nodes[0].body, expected);
            }
        }
    }
    #[test]
    fn gifts_snap_to_the_nearest_legal_diagonal_in_world_coordinates() {
        for (target, expected) in [
            ([100., 1.], 45_f32),
            ([1., 100.], 45.),
            ([-1., 100.], 135.),
            ([-100., 1.], 135.),
            ([100., -1.], -45.),
            ([1., -100.], -45.),
            ([-1., -100.], -135.),
            ([-100., -1.], -135.),
            ([100., 0.], 45.),
            ([0., 100.], 45.),
            ([-100., 0.], 135.),
            ([0., -100.], -45.),
            ([100., -0.], 45.),
        ] {
            let actual = heading([0., 0.], target).unwrap();
            assert_eq!(actual, expected.to_radians());
            for allowed in ALLOWED_DEGREES {
                let a = f64::from(allowed).to_radians();
                let dot = |h: f64| target[0] * h.cos() + target[1] * h.sin();
                assert!(dot(f64::from(actual)) + 0.0001 >= dot(a));
            }
        }
        assert!(heading([0., 0.], [0., 0.]).is_err());
        assert!(heading([0., 0.], [f64::NAN, 100.]).is_err());
        assert!(heading([0., 0.], [100., f64::INFINITY]).is_err());
    }
    #[test]
    fn production_power_and_superweapon_gifts_and_valid_owners_are_selected() {
        for t in [
            "AlliedBarracks",
            "sovietwarfactory",
            "JapanAirfield",
            "AlliedNavalYard",
            "SovietPowerPlant",
            "AlliedSuperWeapon",
            "JapanSuperWeaponAdvanced",
        ] {
            assert!(gifted_building(t));
        }
        for t in ["SovietConstructionYard", "SovietWarFactory_Collapse"] {
            assert!(!gifted_building(t));
        }
        assert_eq!(seat("Player_1/teamPlayer_1").unwrap(), 1);
        assert_eq!(seat("teamPlayer_6").unwrap(), 6);
        assert_eq!(seat("PlyrCivilian/teamPlyrCivilian").unwrap(), 7);
        assert_eq!(seat("teamPlyrCreeps").unwrap(), 8);
        assert!(seat("Player_9/team").is_err());
    }
}
