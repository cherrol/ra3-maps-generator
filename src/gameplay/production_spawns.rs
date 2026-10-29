//! 检查和修正赠送建筑的水陆、平坦度、完整占地及配套 XML。
use crate::{
    Result, err,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject, Value, prop},
    gameplay::script_spawns::{clear, keepouts, to_cell},
    gameplay::scripts::{self, Node},
    pipeline::project::Project,
    terrain,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

const POLICY_SOURCE: &str = "templates/rules/common-T0.5.1.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FootprintRule {
    surface: String,
    padding: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PlacementPolicy {
    full_footprint: bool,
    war_factory: FootprintRule,
    naval_yard: FootprintRule,
}

impl PlacementPolicy {
    /// Every ruleset, including the map-specific legacy scripts, shares this policy.
    fn load(p: &Project) -> Result<Self> {
        #[derive(Deserialize)]
        struct CommonRules {
            production_placement: PlacementPolicy,
        }
        let common: CommonRules =
            serde_json::from_slice(&std::fs::read(p.root.join(POLICY_SOURCE))?)?;
        let policy = common.production_placement;
        if !policy.full_footprint
            || policy.war_factory.surface != "land"
            || policy.naval_yard.surface != "water"
            || !(6..=16).contains(&policy.war_factory.padding)
            || !(8..=16).contains(&policy.naval_yard.padding)
        {
            return Err(err(
                "common production placement requires full land factory and water shipyard footprints",
            ));
        }
        Ok(policy)
    }

    fn padding(&self, wet: bool) -> usize {
        if wet {
            self.naval_yard.padding
        } else {
            self.war_factory.padding
        }
    }
}

fn water_only(name: &str) -> Option<bool> {
    let name = name.to_ascii_lowercase();
    if name.ends_with("warfactory") {
        Some(false)
    } else if name.ends_with("navalyard") || name == "shipyardtechstructure" {
        Some(true)
    } else {
        None
    }
}

/// Resolve actual create actions rather than assuming a faction or waypoint name.
fn bindings(nodes: &[Node], objects: &[MapObject]) -> Result<BTreeMap<usize, bool>> {
    let mut out = BTreeMap::new();
    for (i, o) in objects.iter().enumerate() {
        if let Some(wet) = water_only(&o.type_name) {
            out.insert(i, wet);
        }
    }
    fn visit(nodes: &[Node], objects: &[MapObject], out: &mut BTreeMap<usize, bool>) -> Result<()> {
        for node in nodes {
            if let Some(children) = &node.children {
                visit(children, objects, out)?;
            } else if node
                .command
                .as_deref()
                .is_some_and(|c| c.starts_with("CREATE_") && c.contains("AT_WAYPOINT"))
            {
                let args = scripts::string_arguments(node)?;
                let Some(wet) = args
                    .iter()
                    .find_map(|(ty, name)| (*ty == 15).then(|| water_only(name)).flatten())
                else {
                    continue;
                };
                let name = args
                    .iter()
                    .find(|(ty, _)| *ty == 7)
                    .ok_or_else(|| err("production create action has no waypoint"))?
                    .1
                    .as_str();
                let indices: Vec<_> = objects
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| {
                        prop(&o.props, "waypointName").and_then(Value::text) == Some(name)
                    })
                    .map(|(i, _)| i)
                    .collect();
                if indices.len() != 1 {
                    return Err(err(format!(
                        "production waypoint missing or duplicated: {name}"
                    )));
                }
                if out.insert(indices[0], wet).is_some_and(|old| old != wet) {
                    return Err(err(format!("factory and shipyard share waypoint {name}")));
                }
            }
        }
        Ok(())
    }
    visit(nodes, objects, &mut out)?;
    Ok(out)
}

struct Ground<'a> {
    p: &'a Project,
    n: usize,
    z: &'a [f64],
    impassable: &'a [bool],
    blocked: &'a [bool],
    policy: PlacementPolicy,
}
impl Ground<'_> {
    fn suitable(&self, c: [usize; 2], wet: bool) -> bool {
        let [x, y] = c;
        let padding = self.policy.padding(wet);
        let border = self.p.usize("BORDER", 20) + padding + 2;
        if x < border
            || y < border
            || x >= self.n.saturating_sub(border)
            || y >= (self.z.len() / self.n).saturating_sub(border)
        {
            return false;
        }
        let water = self.p.number("WATER_Z", 200.);
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for yy in y - padding..=y + padding {
            for xx in x - padding..=x + padding {
                let i = yy * self.n + xx;
                if (if wet {
                    self.z[i] >= water - 1.
                } else {
                    self.z[i] <= water
                }) || self.impassable[i]
                    || self.blocked[i]
                {
                    return false;
                }
                lo = lo.min(self.z[i]);
                hi = hi.max(self.z[i]);
            }
        }
        wet || hi - lo <= 1.
    }
}
fn cell(p: &Project, o: &MapObject) -> [usize; 2] {
    to_cell(p, o).map(|v| v.round_ties_even() as usize)
}

#[allow(clippy::too_many_arguments)]
pub fn repair(
    p: &Project,
    n: usize,
    z: &[f64],
    impassable: &[bool],
    blocked: &[bool],
    nodes: &[Node],
    objects: &mut [MapObject],
) -> Result<Json> {
    let frame = crate::coordinates::Frame::from_project(p);
    let ground = Ground {
        p,
        n,
        z,
        impassable,
        blocked,
        policy: PlacementPolicy::load(p)?,
    };
    let mut placements = Vec::new();
    for (index, wet) in bindings(nodes, objects)? {
        let from = cell(p, &objects[index]);
        let mut point = from;
        if !ground.suitable(from, wet) {
            let other: Vec<_> = objects
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != index)
                .map(|(_, o)| o.clone())
                .collect();
            let mut protected = keepouts(p, &other);
            // Saved maps also contain the supplemental power/superweapon points.
            for o in &other {
                if o.type_name.contains("Waypoint") {
                    protected.push((to_cell(p, o), 6.));
                }
            }
            let nearest_seat = objects
                .iter()
                .filter_map(|o| {
                    let name = prop(&o.props, "waypointName").and_then(Value::text)?;
                    let seat = name
                        .strip_prefix("Player_")?
                        .split('_')
                        .next()?
                        .parse::<usize>()
                        .ok()?;
                    let c = to_cell(p, o);
                    Some(((c[0] - from[0] as f64).hypot(c[1] - from[1] as f64), seat))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let default_radius = nearest_seat
                .and_then(|(_, seat)| {
                    p.cfg["SCRIPT_SPAWN_RADIUS_BY_SEAT"][seat.to_string()].as_u64()
                })
                .unwrap_or(80) as usize;
            let radius = p.usize("PRODUCTION_SPAWN_RADIUS", default_radius).min(n);
            let mut candidates = Vec::new();
            for y in
                from[1].saturating_sub(radius)..=from[1].saturating_add(radius).min(z.len() / n - 1)
            {
                for x in from[0].saturating_sub(radius)..=from[0].saturating_add(radius).min(n - 1)
                {
                    let d = (x as f64 - from[0] as f64).hypot(y as f64 - from[1] as f64);
                    if d <= radius as f64 {
                        candidates.push((d, x, y));
                    }
                }
            }
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
            let (_, x, y) = candidates
                .into_iter()
                .find(|&(_, x, y)| {
                    ground.suitable([x, y], wet)
                        && clear([x, y], ground.policy.padding(wet), &protected)
                })
                .ok_or_else(|| {
                    err(format!(
                        "no safe {} for production building near {from:?}",
                        if wet { "water" } else { "land" }
                    ))
                })?;
            point = [x, y];
            let position = frame.world([x as f64, y as f64]);
            objects[index].position[0] = position[0] as f32;
            objects[index].position[1] = position[1] as f32;
        }
        placements.push(json!({"object_index":index,"waypoint":prop(&objects[index].props,"waypointName").and_then(Value::text),"type":objects[index].type_name,"surface":if wet {"water"} else {"land"},"padding":ground.policy.padding(wet),"from":from,"cell":point,"moved":from!=point}));
    }
    Ok(
        json!({"placements":placements,"policy":ground.policy,"policy_source":format!("{POLICY_SOURCE}#/production_placement"),"ingame_test":"not run"}),
    )
}

pub fn validate(
    p: &Project,
    map: &MapFile,
    hm: &HeightMap,
    bt: &BlendTiles,
    objects: &[MapObject],
) -> Result<()> {
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let impassable = terrain::flip(&bt.impassability, hm.width);
    let blocked = terrain::flip(&bt.buildability, hm.width);
    let ground = Ground {
        p,
        n: hm.width,
        z: &z,
        impassable: &impassable,
        blocked: &blocked,
        policy: PlacementPolicy::load(p)?,
    };
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    for (i, wet) in bindings(&nodes, objects)? {
        if !ground.suitable(cell(p, &objects[i]), wet) {
            return Err(err(format!(
                "production footprint requires {}: {}",
                if wet { "water" } else { "flat land" },
                prop(&objects[i].props, "waypointName")
                    .and_then(Value::text)
                    .unwrap_or(&objects[i].type_name)
            )));
        }
    }
    Ok(())
}

pub fn apply(p: &Project, map: &mut MapFile) -> Result<Json> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let mut objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    let report = repair(
        p,
        hm.width,
        &z,
        &terrain::flip(&bt.impassability, hm.width),
        &terrain::flip(&bt.buildability, hm.width),
        &nodes,
        &mut objects,
    )?;
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, map)?;
    let collision = crate::gameplay::gift_clearance::apply(p, map)?;
    let facing = crate::gameplay::production_facing::apply(p, map)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    validate(p, map, &hm, &bt, &objects)?;
    let mut report = report;
    refresh_report(p, map, &mut report)?;
    report["collision_avoidance"] = collision;
    report["facing"] = facing;
    Ok(report)
}

pub fn refresh_report(p: &Project, map: &MapFile, report: &mut Json) -> Result<()> {
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    for entry in report["placements"]
        .as_array_mut()
        .ok_or_else(|| err("invalid spawn report"))?
    {
        let index = entry["object_index"]
            .as_u64()
            .ok_or_else(|| err("missing spawn index"))? as usize;
        let c = cell(p, &objects[index]);
        entry["cell"] = json!(c);
        entry["moved"] = json!(entry["from"] != json!(c));
    }
    Ok(())
}

/// Find one physical object's XML position, tolerating WorldBuilder decimal precision.
fn position_span(xml: &str, position: [f32; 3]) -> Result<(usize, usize)> {
    let mut matches = Vec::new();
    for (start, _) in xml.match_indices("<Position ") {
        let Some(end) = xml[start..].find("/>").map(|i| start + i + 2) else {
            continue;
        };
        let tag = &xml[start..end];
        let coordinates = ["x", "y", "z"].map(|axis| {
            let marker = format!("{axis}=\"");
            let part = tag.split_once(&marker)?.1;
            part.split('"').next()?.parse::<f64>().ok()
        });
        // Original WB XML stores decimal coordinates at slightly different
        // precision than its native float32 map. Refuse ambiguous matches.
        if coordinates
            .iter()
            .zip(position)
            .all(|(a, b)| a.is_some_and(|a| (a - b as f64).abs() <= 0.002))
        {
            matches.push((start, end));
        }
    }
    if matches.len() != 1 {
        return Err(err(format!(
            "source XML has missing or ambiguous object position: {position:?}"
        )));
    }
    Ok(matches[0])
}
fn sync_object_heading(xml: &mut String, object: &MapObject) -> Result<()> {
    let (position, _) = position_span(xml, object.position)?;
    let start = xml[..position]
        .rfind('<')
        .ok_or_else(|| err("missing object XML opening tag"))?;
    let tag = &xml[start..position];
    if !tag.contains(&format!("ThingTemplate=\"{}\"", object.type_name)) {
        return Err(err("object XML heading does not match native type"));
    }
    let value = tag
        .find("Angle=\"")
        .ok_or_else(|| err("missing object XML Angle"))?
        + "Angle=\"".len();
    let end = value
        + tag[value..]
            .find('"')
            .ok_or_else(|| err("unterminated XML Angle"))?;
    let previous: f32 = tag[value..end].parse()?;
    if !previous.is_finite() || (previous - object.angle).abs() > 0.000001 {
        xml.replace_range(start + value..start + end, &object.angle.to_string());
    }
    Ok(())
}
pub fn update_xml(xml: &str, before: &MapFile, after: &MapFile) -> Result<String> {
    let old = map::objects(before.get("ObjectsList")?, &before.table)?;
    let new = map::objects(after.get("ObjectsList")?, &after.table)?;
    if old.len() > new.len() {
        return Err(err("production repair removed objects"));
    }
    let mut xml = xml.to_owned();
    for (a, b) in old.iter().zip(&new) {
        // Frozen packages can already contain stale ore headings. Reconcile all
        // ore XML against the native result, including angle-only edits.
        let sync_heading = a.angle != b.angle || b.type_name == "OreNode";
        if a.position == b.position && !sync_heading {
            continue;
        }
        if let Some(name) = prop(&a.props, "waypointName").and_then(Value::text) {
            let marker = format!("<Waypoint id=\"{name}\"");
            let start = xml
                .find(&marker)
                .ok_or_else(|| err(format!("source XML missing waypoint {name}")))?;
            let end = start
                + xml[start..]
                    .find("</Waypoint>")
                    .ok_or_else(|| err("unterminated waypoint XML"))?;
            let position_start = start
                + xml[start..end]
                    .find("<Position ")
                    .ok_or_else(|| err("missing waypoint XML position"))?;
            let position_end = position_start
                + xml[position_start..end]
                    .find("/>")
                    .ok_or_else(|| err("unterminated waypoint XML position"))?
                + 2;
            xml.replace_range(
                position_start..position_end,
                &format!(
                    "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
                    b.position[0], b.position[1], b.position[2]
                ),
            );
            continue;
        }
        let pos = |o: &MapObject| {
            format!(
                "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
                o.position[0], o.position[1], o.position[2]
            )
        };
        if a.position != b.position {
            let (start, end) = position_span(&xml, a.position)?;
            xml.replace_range(start..end, &pos(b));
        }
        if sync_heading {
            sync_object_heading(&mut xml, b)?;
        }
    }
    let mut extra = String::new();
    for o in &new[old.len()..] {
        let name = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .ok_or_else(|| err("unexpected new non-waypoint object"))?;
        extra.push_str(&format!(
            "<Waypoint id=\"{name}\"><Position x=\"{}\" y=\"{}\" z=\"{}\"/></Waypoint>",
            o.position[0], o.position[1], o.position[2]
        ));
    }
    if !extra.is_empty() {
        let end = xml
            .find("</GameMap>")
            .ok_or_else(|| err("missing GameMap XML"))?;
        xml.insert_str(end, &extra);
    }
    crate::resources::refresh_script_xml(&xml, after)
}

pub fn write_artifacts(p: &Project, source_name: &str, map: &MapFile, report: &Json) -> Result<()> {
    use crate::{
        pipeline::project,
        render,
        terrain::surface::Surface,
        terrain::{Layout, ObjectSpec, Terrain},
    };
    use std::fs;
    let source_dir = p.release_assets(source_name)?;
    let mut layout = Layout::load_release(p, source_name)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let rules = scripts::load_rules(p)?;
    layout.objects = objects
        .iter()
        .filter(|o| {
            !prop(&o.props, "waypointName")
                .and_then(Value::text)
                .is_some_and(|name| rules.spawn_waypoints.iter().any(|s| s.name == name))
        })
        .map(|o| ObjectSpec {
            cell: to_cell(p, o),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    layout.shipyard_cells = objects
        .iter()
        .filter(|o| water_only(&o.type_name) == Some(true))
        .map(|o| to_cell(p, o))
        .collect();
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let n = hm.width;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, n, hm.height)?;
    let raw = terrain::flip(&hm.elevations, n);
    let z: Vec<_> = raw
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect();
    let t = Terrain {
        n,
        land: z.iter().map(|v| *v > p.number("WATER_Z", 200.)).collect(),
        slope: terrain::gradient(&z, n),
        raw,
        z,
        layout,
    };
    t.save(p)?;
    let s = Surface {
        chosen: terrain::flip(&bt.tiles, n).iter().map(|v| v / 64).collect(),
        impassable: terrain::flip(&bt.impassability, n),
        build_block: terrain::flip(&bt.buildability, n),
        layer_edge: vec![false; n * hm.height],
        shore: vec![false; n * hm.height],
        textures: vec![],
    };
    render::previews(&t, p, &s)?;
    let index_path = p.assets.join("layout-index.json");
    let mut index: Json = serde_json::from_slice(&fs::read(&index_path)?)?;
    index["layout"] = json!("source/layout.json");
    index["source_map"] = json!(format!("{}.map", p.name));
    project::write_json(&index_path, &index)?;
    project::write_json(&p.assets.join("production-spawns.json"), report)?;
    if let Some(collision) = report.get("collision_avoidance") {
        if collision["already_guarded"] == true && source_dir.join("gift-clearance.json").is_file()
        {
            fs::copy(
                source_dir.join("gift-clearance.json"),
                p.assets.join("gift-clearance.json"),
            )?;
        } else {
            project::write_json(&p.assets.join("gift-clearance.json"), collision)?;
        }
    }
    if let Some(facing) = report.get("facing") {
        project::write_json(&p.assets.join("production-facing.json"), facing)?;
    }
    if source_dir.join("periodic-attack.json").is_file() {
        fs::copy(
            source_dir.join("periodic-attack.json"),
            p.assets.join("periodic-attack.json"),
        )?;
    }
    for name in [
        "ramp-report.json",
        "ramps--comparison.png",
        "texture-report.json",
    ] {
        if source_dir.join(name).is_file() {
            fs::copy(source_dir.join(name), p.assets.join(name))?;
        }
    }
    fs::write(
        p.assets.join("README.md"),
        format!(
            "# {}\n\n基于 {} 更新公共赠送建筑规则：兵营、重工、机场、船厂、电厂和超武从各自实际主备落点朝向对方队伍出生区域中心，吸附到最近的 45、135、-45、-135 度。仅改变生成动作的朝向，不改变触发条件或归属。重工占地只允许平坦陆地，船厂占地只允许水域；运行时占用检查及主备避让继续保留。高度、纹理、通行及建造位图和资源身份保持原样。\n\n[朝向报告](production-facing.json) · [水陆与避让报告](production-spawns.json) · [全要素编号](preview--layout.png) · [发布核对](release-report.json)\n\n游戏内效果尚待实测。\n",
            p.name, source_name
        ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{formats::binary::Writer, terrain::Layout};
    #[test]
    fn wb_decimal_coordinates_match_native_float_precision_but_duplicates_fail() {
        let tag = "<Position x=\"4642.596482\" y=\"3106.560427\" z=\"0\" />";
        let position = [4642.596_f32, 3106.5603_f32, 0.];
        assert_eq!(position_span(tag, position).unwrap(), (0, tag.len()));
        assert!(position_span(&format!("{tag}{tag}"), position).is_err());
        assert!(position_span(tag, [4642.6, 3106.56, 0.]).is_err());
    }

    #[test]
    fn angle_only_edits_and_stale_ore_xml_follow_native_headings_idempotently() {
        let p = Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "陕西").unwrap();
        let before = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
        let mut objects = map::objects(before.get("ObjectsList").unwrap(), &before.table).unwrap();
        let ore = objects
            .iter_mut()
            .find(|o| o.type_name == "OreNode")
            .unwrap();
        let position = ore.position;
        ore.angle = std::f32::consts::FRAC_PI_4;
        let mut after = before.clone();
        after.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &after).unwrap();
        // Keep the actual package's script XML and physical objects, but
        // deliberately stale the first ore's angle.
        let xml = std::fs::read_to_string(p.assets.join("map.xml")).unwrap();
        let mut stale = xml.clone();
        let object = objects
            .iter()
            .find(|o| o.position == position && o.type_name == "OreNode")
            .unwrap();
        let mut wrong = object.clone();
        wrong.angle = -std::f32::consts::FRAC_PI_4;
        sync_object_heading(&mut stale, &wrong).unwrap();
        let edited = update_xml(&stale, &before, &after).unwrap();
        let repaired = update_xml(&stale, &after, &after).unwrap();
        assert_eq!(edited, repaired);
        assert_eq!(update_xml(&repaired, &after, &after).unwrap(), repaired);
        // Other inherited stale ores may also be reconciled, so inspect the
        // exact opening tag at this ore's unchanged native position.
        let (at, _) = position_span(&repaired, position).unwrap();
        let start = repaired[..at].rfind('<').unwrap();
        assert!(repaired[start..at].contains("Angle=\"0.7853982\""));
        let (old_start, old_end) = position_span(&xml, position).unwrap();
        let (_, end) = position_span(&repaired, position).unwrap();
        assert_eq!(&repaired[at..end], &xml[old_start..old_end]);
    }

    fn project() -> Project {
        let mut p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "广西").unwrap();
        p.cfg["GRID"] = json!(220);
        p
    }
    fn action(kind: &str, waypoint: &str) -> Node {
        let mut w = Writer::new();
        w.u32(467);
        w.u32(3);
        w.u32(2);
        for (ty, s) in [(15, kind), (7, waypoint)] {
            w.u32(ty);
            w.u32(0);
            w.f32(0.);
            w.s16(s).unwrap();
        }
        w.u32(1);
        Node {
            asset: "ScriptAction".into(),
            version: 3,
            prefix: String::new(),
            body: scripts::hex(&w.data),
            command: Some("CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION".into()),
            children: None,
        }
    }

    #[test]
    fn every_ruleset_reads_the_public_policy_and_rejects_wrong_surfaces() {
        let directory =
            std::env::temp_dir().join(format!("ra3-public-placement-{}", std::process::id()));
        let rules = directory.join("templates/rules");
        std::fs::create_dir_all(&rules).unwrap();
        let path = rules.join("common-T0.5.1.json");
        let mut common = json!({"production_placement": {
            "full_footprint":true,
            "war_factory":{"surface":"land","padding":7},
            "naval_yard":{"surface":"water","padding":9}
        }});
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        for id in ["陕西", "广西", "广东", "海南"] {
            let mut p =
                Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), id).unwrap();
            p.root = directory.clone();
            let policy = PlacementPolicy::load(&p).unwrap();
            assert_eq!(policy.padding(false), 7);
            assert_eq!(policy.padding(true), 9);
        }
        let mut p = project();
        p.root = directory.clone();
        common["production_placement"]["war_factory"]["surface"] = json!("water");
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        assert!(PlacementPolicy::load(&p).is_err());
        common["production_placement"]["war_factory"]["surface"] = json!("land");
        common["production_placement"]["full_footprint"] = json!(false);
        std::fs::write(&path, serde_json::to_vec(&common).unwrap()).unwrap();
        assert!(PlacementPolicy::load(&p).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn entire_footprint_must_match_surface_and_be_clear() {
        let p = project();
        let n = 220;
        let mut z = vec![210.; n * n];
        for y in 0..n {
            for x in 110..n {
                z[y * n + x] = 130.;
            }
        }
        let mut blocked = vec![false; n * n];
        let impassable = vec![false; n * n];
        let ground = Ground {
            p: &p,
            n,
            z: &z,
            impassable: &impassable,
            blocked: &blocked,
            policy: PlacementPolicy::load(&p).unwrap(),
        };
        assert!(ground.suitable([80, 100], false));
        assert!(!ground.suitable([80, 100], true));
        assert!(ground.suitable([140, 100], true));
        assert!(!ground.suitable([140, 100], false));
        assert!(!ground.suitable([105, 100], false));
        assert!(!ground.suitable([115, 100], true));
        assert!(!ground.suitable([10, 100], true));
        blocked[100 * n + 80] = true;
        let ground = Ground {
            p: &p,
            n,
            z: &z,
            impassable: &impassable,
            blocked: &blocked,
            policy: PlacementPolicy::load(&p).unwrap(),
        };
        assert!(!ground.suitable([80, 100], false));
    }

    #[test]
    fn repairs_scripted_factories_and_shipyards_without_changing_actions() {
        let p = project();
        let n = 220;
        let original_p =
            Project::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), "广西").unwrap();
        let mut layout = Layout::load(&original_p).unwrap();
        let mut prototype = layout.objects.remove(
            layout
                .objects
                .iter()
                .position(|o| o.type_name.contains("Waypoint"))
                .unwrap(),
        );
        let mut objects = Vec::new();
        for (name, point) in [("factory", [140., 80.]), ("shipyard", [80., 140.])] {
            prototype.cell = point;
            map::set(
                &mut prototype.props,
                "waypointName",
                Value::Ascii(name.into()),
            );
            objects.push(prototype.object(&p));
        }
        let mut z = vec![210.; n * n];
        for y in 0..n {
            for x in 110..n {
                z[y * n + x] = 130.;
            }
        }
        let flags = vec![false; n * n];
        for faction in ["allied", "soviet", "japan"] {
            let nodes = vec![
                action(&format!("{faction}warfactory"), "factory"),
                action(&format!("{faction}navalyard"), "shipyard"),
            ];
            let original = serde_json::to_value(&nodes).unwrap();
            let mut repaired = objects.clone();
            let report = repair(&p, n, &z, &flags, &flags, &nodes, &mut repaired).unwrap();
            assert!(
                report["placements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v["moved"] == true)
            );
            let ground = Ground {
                p: &p,
                n,
                z: &z,
                impassable: &flags,
                blocked: &flags,
                policy: PlacementPolicy::load(&p).unwrap(),
            };
            assert!(ground.suitable(cell(&p, &repaired[0]), false));
            assert!(ground.suitable(cell(&p, &repaired[1]), true));
            assert_eq!(serde_json::to_value(&nodes).unwrap(), original);
            let repeated = repair(&p, n, &z, &flags, &flags, &nodes, &mut repaired).unwrap();
            assert!(
                repeated["placements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|v| v["moved"] == false)
            );
        }
        assert!(bindings(&[action("sovietwarfactory", "missing")], &objects).is_err());
        assert!(
            bindings(
                &[
                    action("sovietwarfactory", "factory"),
                    action("sovietnavalyard", "factory")
                ],
                &objects
            )
            .is_err()
        );
        assert!(
            repair(
                &p,
                n,
                &vec![130.; n * n],
                &flags,
                &flags,
                &[action("sovietwarfactory", "factory")],
                &mut objects
            )
            .is_err()
        );
    }
}
