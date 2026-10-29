//! 原生对象、斜坡与脚本候选点的身份和坐标清单。
use super::raster::Raster;
use crate::{
    Result, err,
    formats::map::{self, MapFile, Value, prop},
    gameplay::scripts,
    terrain::{Layout, number},
};
use serde::Serialize;
use serde_json::Value as Json;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Point {
    pub(super) id: String,
    pub(super) preview_id: Option<usize>,
    pub(super) category: String,
    pub(super) subtype: String,
    pub(super) name: String,
    pub(super) object_type: String,
    pub(super) owner: String,
    pub(super) owner_seat: Option<usize>,
    pub(super) neutral: bool,
    pub(super) grid: [f64; 2],
    pub(super) world: [f64; 2],
    pub(super) object_z: Option<f32>,
    pub(super) surface_z: Option<f64>,
    pub(super) layer: Option<usize>,
    pub(super) region: Option<usize>,
    pub(super) in_grid: bool,
    pub(super) in_playable: bool,
    pub(super) angle_radians: Option<f64>,
    pub(super) waypoint: Option<String>,
    pub(super) conditional: bool,
    pub(super) properties: Json,
}

pub(super) fn owner_seat(owner: &str) -> Option<usize> {
    match owner
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("team")
    {
        "PlyrCivilian" => Some(7),
        "PlyrCreeps" => Some(8),
        s => s
            .strip_prefix("Player_")
            .and_then(|s| s.parse().ok())
            .filter(|s| (1..=8).contains(s)),
    }
}
pub(super) fn neutral(owner: &str) -> bool {
    matches!(
        owner.split('/').next().unwrap_or(""),
        "PlyrNeutral" | "SkirmishCivilian" | "SkirmishNeutral" | "_neutral_" | "Neutral"
    )
}
pub(super) fn building(kind: &str) -> bool {
    let n = kind.to_ascii_lowercase();
    [
        "constructionyard",
        "barracks",
        "warfactory",
        "airfield",
        "navalyard",
        "powerplant",
        "basedefense",
        "basedefenseadvanced",
        "superweapon",
        "superweaponadvanced",
        "techstructure",
    ]
    .iter()
    .any(|s| n.ends_with(s))
        || n.starts_with("ei_structure_")
        || n.starts_with("st_")
}
pub(super) fn category(kind: &str, waypoint: &str) -> (&'static str, &'static str) {
    if kind.contains("Waypoint") {
        if waypoint.starts_with("Player_") && waypoint.contains("_Start") {
            ("出生点", "玩家出生点")
        } else {
            ("航点", "标记点")
        }
    } else if kind == "OreNode" {
        ("资源", "矿脉")
    } else if kind == "OilDerrick" {
        ("资源", "油井")
    } else if kind.contains("Collapse") {
        ("装饰", "废墟")
    } else if building(kind) {
        if kind.contains("TechStructure") {
            ("建筑", "中立科技建筑")
        } else if kind.starts_with("EI_Structure_") || kind.starts_with("ST_") {
            ("建筑", "民用建筑")
        } else {
            ("建筑", "基地或生产建筑")
        }
    } else if kind.contains("Tree") {
        ("装饰", "树木")
    } else if kind.contains("Bush") {
        ("装饰", "灌木")
    } else if kind.contains("Rock") {
        ("装饰", "石头")
    } else if kind.contains("Sidewalk") {
        ("道路", "人行道")
    } else {
        ("其他", "其他对象")
    }
}
pub(super) fn sample(r: &Raster, p: [f64; 2]) -> (Option<f64>, Option<usize>, Option<usize>) {
    if !r.frame.inside(p) {
        return (None, None, None);
    }
    let x = p[0].round().clamp(0., (r.frame.width - 1) as f64) as usize;
    let y = p[1].round().clamp(0., (r.frame.height - 1) as f64) as usize;
    let i = y * r.frame.width + x;
    let x0 = p[0].floor() as usize;
    let y0 = p[1].floor() as usize;
    let x1 = (x0 + 1).min(r.frame.width - 1);
    let y1 = (y0 + 1).min(r.frame.height - 1);
    let ax = p[0] - x0 as f64;
    let ay = p[1] - y0 as f64;
    let z = (r.z[y0 * r.frame.width + x0] * (1. - ax) + r.z[y0 * r.frame.width + x1] * ax)
        * (1. - ay)
        + (r.z[y1 * r.frame.width + x0] * (1. - ax) + r.z[y1 * r.frame.width + x1] * ax) * ay;
    (Some(z), Some(r.class[i]), Some(r.regions[i]))
}
pub(super) fn point_at(
    r: &Raster,
    id: String,
    preview_id: Option<usize>,
    description: (&str, &str, String, String, String),
    p: [f64; 2],
) -> Point {
    let (category, subtype, name, object_type, owner) = description;
    let (z, layer, region) = sample(r, p);
    Point {
        id,
        preview_id,
        category: category.into(),
        subtype: subtype.into(),
        name,
        object_type,
        owner_seat: owner_seat(&owner),
        neutral: neutral(&owner),
        owner,
        grid: p,
        world: r.frame.world(p),
        object_z: None,
        surface_z: z,
        layer,
        region,
        in_grid: r.frame.inside(p),
        in_playable: r.frame.playable(p),
        angle_radians: None,
        waypoint: None,
        conditional: false,
        properties: Json::Null,
    }
}
pub(super) fn angle(node: &scripts::Node) -> Result<Option<f64>> {
    Ok(scripts::real_argument(node, 17)?.map(|(value, _)| value as f64))
}
pub(super) fn points(map: &MapFile, r: &Raster, layout: &Layout) -> Result<Vec<Point>> {
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut result = Vec::new();
    let mut waypoints = BTreeMap::new();
    for (i, o) in objects.iter().enumerate() {
        let wp = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .unwrap_or("");
        let owner = prop(&o.props, "originalOwner")
            .and_then(Value::text)
            .unwrap_or("")
            .to_owned();
        let name = prop(&o.props, "uniqueID")
            .and_then(Value::text)
            .unwrap_or(&o.type_name)
            .to_owned();
        let (c, s) = category(&o.type_name, wp);
        let mut point = point_at(
            r,
            format!("O{}", i + 1),
            Some(i + 1),
            (c, s, name, o.type_name.clone(), owner),
            r.frame.grid([o.position[0] as f64, o.position[1] as f64]),
        );
        point.object_z = Some(o.position[2]);
        point.angle_radians = Some(o.angle as f64);
        point.properties = serde_json::to_value(&o.props)?;
        if !wp.is_empty() {
            point.waypoint = Some(wp.into());
            waypoints.insert(wp.to_owned(), point.grid);
        }
        result.push(point);
    }
    for (i, ramp) in layout.ramps.iter().enumerate() {
        let p = [number(ramp, "x", 0.), number(ramp, "y", 0.)];
        let id = layout.objects.len() + i + 1;
        let mut point = point_at(
            r,
            format!("A{id}"),
            Some(id),
            (
                "斜坡",
                "已定稿斜坡",
                format!("斜坡 {id}"),
                "Ramp".into(),
                String::new(),
            ),
            p,
        );
        point.properties = ramp.clone();
        result.push(point);
    }
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let mut sites = BTreeMap::<(String, String, String), Option<f64>>::new();
    fn visit(
        nodes: &[scripts::Node],
        sites: &mut BTreeMap<(String, String, String), Option<f64>>,
    ) -> Result<()> {
        for n in nodes {
            if let Some(c) = &n.children {
                visit(c, sites)?;
            } else if n
                .command
                .as_deref()
                .is_some_and(|s| s.starts_with("CREATE_") && s.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(n)?;
                let Some(kind) = args
                    .iter()
                    .find(|(t, s)| *t == 15 && building(s))
                    .map(|(_, s)| s.clone())
                else {
                    continue;
                };
                let Some(owner) = args.iter().find(|(t, _)| *t == 3).map(|(_, s)| s.clone()) else {
                    continue;
                };
                let Some(wp) = args.iter().find(|(t, _)| *t == 7).map(|(_, s)| s.clone()) else {
                    continue;
                };
                sites.insert((kind, owner, wp), angle(n)?);
            }
        }
        Ok(())
    }
    visit(&nodes, &mut sites)?;
    for ((kind, owner, wp), angle) in sites {
        let position = *waypoints
            .get(&wp)
            .ok_or_else(|| err(format!("script building has no coordinate waypoint {wp}")))?;
        let id = format!("G{}", result.iter().filter(|p| p.conditional).count() + 1);
        let role = if wp.starts_with("gift-alt-") {
            "备用赠送点"
        } else if wp.starts_with("dual-") {
            "主赠送点"
        } else {
            "条件生成点"
        };
        let mut point = point_at(
            r,
            id,
            None,
            ("脚本建筑候选", role, format!("{kind} @ {wp}"), kind, owner),
            position,
        );
        point.conditional = true;
        point.angle_radians = angle;
        point.waypoint = Some(wp);
        result.push(point);
    }
    Ok(result)
}
