//! Coordinate-driven incremental authoring from a frozen, complete saved package.
use crate::{
    Result, err,
    formats::archive,
    formats::map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    pipeline::build,
    pipeline::project::{self, Project},
    render,
    terrain::spatial::{self, Issue, Policy, Scene},
    terrain::surface::Surface,
    terrain::{self, Layout, Terrain},
};
use serde::Deserialize;
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, fs, sync::Arc};
#[derive(Deserialize)]
struct Settings {
    source_package: String,
    layout: String,
    baseline: String,
    source_name: String,
    source_crc32: u32,
}
struct Source {
    settings: Settings,
    files: Vec<(String, Vec<u8>)>,
    map: Arc<MapFile>,
    layout: Layout,
}
struct Projection {
    source: Source,
    layout: Layout,
    map: Arc<MapFile>,
    terrain_xml: Option<String>,
}
pub fn enabled(p: &Project) -> bool {
    p.cfg["COORDINATE_PIPELINE"].is_object()
}
fn settings(p: &Project) -> Result<Settings> {
    Ok(serde_json::from_value(
        p.cfg["COORDINATE_PIPELINE"].clone(),
    )?)
}
fn source(p: &Project) -> Result<Source> {
    let settings = settings(p)?;
    let (files, layout) = crate::pipeline::frozen_source::FrozenSource::load(p)?.into_parts()?;
    let map = crate::pipeline::release::load_source_map(&files)?;
    Ok(Source {
        settings,
        files,
        map: Arc::new(map),
        layout,
    })
}
fn map_path(p: &Project) -> std::path::PathBuf {
    p.assets.join(format!("{}.map", p.name))
}
pub fn preflight(p: &Project) -> Result<()> {
    let s = source(p)?;
    let hm = HeightMap::parse(s.map.get("HeightMapData")?)?;
    if crate::coordinates::Frame::from_heightmap(p, &hm)?
        != crate::coordinates::Frame::from_project(p)
    {
        return Err(err(
            "recipe coordinate frame differs from the frozen native map",
        ));
    }
    if p.name != s.settings.source_name {
        return Err(err(
            "current release has not been adopted as the coordinate source; run adopt-coordinate-source",
        ));
    }
    if map_path(p).is_file() {
        let current = MapFile::load(map_path(p))?;
        if current.encode()? != s.map.encode()? {
            return Err(err(
                "current finished map differs from frozen coordinate source; run adopt-coordinate-source before building",
            ));
        }
    }
    for (name, bytes) in &s.files {
        let file = p.assets.join(name);
        if file.is_file() && fs::read(file)? != *bytes {
            return Err(err(format!(
                "current game file {name} differs from frozen source; run adopt-coordinate-source"
            )));
        }
    }
    layout_with_source(p, &s)?;
    Ok(())
}
fn owner(o: &terrain::ObjectSpec) -> &str {
    prop(&o.props, "originalOwner")
        .and_then(Value::text)
        .unwrap_or("")
}
fn identity(o: &terrain::ObjectSpec) -> &str {
    prop(&o.props, "uniqueID")
        .and_then(Value::text)
        .unwrap_or("")
}
fn sync_group(points: &mut [[f64; 2]], old: &Layout, new: &[terrain::ObjectSpec]) -> Result<()> {
    for point in points {
        let matches = old
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                !o.type_name.contains("Waypoint")
                    && (o.cell[0] - point[0]).hypot(o.cell[1] - point[1]) < 0.001
            })
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            *point = new[matches[0].0].cell;
        } else {
            return Err(err(format!(
                "coordinate metadata requires one physical source object at {point:?}"
            )));
        }
    }
    Ok(())
}
fn layout_with_source(p: &Project, s: &Source) -> Result<Layout> {
    let mut layout: Layout = serde_json::from_slice(&fs::read(p.path(&s.settings.layout)?)?)?;
    if layout.schema_version != 1 || layout.objects.len() != s.layout.objects.len() {
        return Err(err(
            "coordinate layout must preserve native object identities/count; adopt a complete edited source to add or remove objects",
        ));
    }
    if serde_json::to_value(&layout.ramps)? != serde_json::to_value(&s.layout.ramps)? {
        return Err(err(
            "ramp geometry changed without native terrain: use relocate-ramps and adopt its complete source",
        ));
    }
    let n = p.number("GRID", 0.);
    let h = p.number("GRID_HEIGHT", n);
    for (i, (a, b)) in layout.objects.iter().zip(&s.layout.objects).enumerate() {
        if a.type_name != b.type_name
            || identity(a) != identity(b)
            || owner(a) != owner(b)
            || a.version != b.version
            || a.road_type != b.road_type
            || serde_json::to_value(&a.props)? != serde_json::to_value(&b.props)?
        {
            return Err(err(format!(
                "object {} identity/properties changed; coordinate edits preserve native identities",
                i + 1
            )));
        }
        if !a.angle.is_finite() || a.cell.iter().any(|v| !v.is_finite()) {
            return Err(err("non-finite coordinate edit"));
        }
        let outside = a
            .cell
            .iter()
            .enumerate()
            .any(|(axis, v)| *v < 0. || *v >= [n, h][axis]);
        if outside && (a.cell != b.cell || a.type_name != "OilDerrick") {
            return Err(err(format!(
                "object {} creates or moves an out-of-grid object",
                i + 1
            )));
        }
    }
    // Objects are the coordinate authority. Derived lists follow their source identities.
    layout.base_cells = s.layout.base_cells.clone();
    layout.base_pad_cells = s.layout.base_pad_cells.clone();
    layout.ore_cells = s.layout.ore_cells.clone();
    layout.ore_angles = s.layout.ore_angles.clone();
    layout.derrick_cells = s.layout.derrick_cells.clone();
    layout.house_cells = s.layout.house_cells.clone();
    layout.post_cells = s.layout.post_cells.clone();
    layout.garage_cells = s.layout.garage_cells.clone();
    layout.airport_cells = s.layout.airport_cells.clone();
    layout.hospital_cells = s.layout.hospital_cells.clone();
    layout.academy_cells = s.layout.academy_cells.clone();
    layout.shipyard_cells = s.layout.shipyard_cells.clone();
    layout.bridge_cells = s.layout.bridge_cells.clone();
    for (seat, base) in layout.base_cells.iter_mut().enumerate() {
        let plain = format!("Player_{}_Start", seat + 1);
        let imported = format!("{plain} (imported)");
        let matching = layout
            .objects
            .iter()
            .filter(|o| {
                prop(&o.props, "waypointName")
                    .and_then(Value::text)
                    .is_some_and(|n| n == plain || n == imported)
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(err("coordinate layout requires one actual start per seat"));
        }
        *base = matching[0].cell;
    }
    for (i, pad) in layout.base_pad_cells.iter_mut().enumerate() {
        let seat = if s.layout.base_pad_cells.len() == s.layout.base_cells.len() {
            i
        } else {
            s.layout
                .base_cells
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (a[0] - pad[0])
                        .hypot(a[1] - pad[1])
                        .total_cmp(&(b[0] - pad[0]).hypot(b[1] - pad[1]))
                })
                .map(|(j, _)| j)
                .ok_or_else(|| err("spawn pad has no base"))?
        };
        pad[0] += layout.base_cells[seat][0] - s.layout.base_cells[seat][0];
        pad[1] += layout.base_cells[seat][1] - s.layout.base_cells[seat][1];
    }
    for group in [
        &mut layout.ore_cells,
        &mut layout.derrick_cells,
        &mut layout.house_cells,
        &mut layout.post_cells,
        &mut layout.garage_cells,
        &mut layout.airport_cells,
        &mut layout.hospital_cells,
        &mut layout.academy_cells,
        &mut layout.shipyard_cells,
        &mut layout.bridge_cells,
    ] {
        sync_group(group, &s.layout, &layout.objects)?;
    }
    for (i, point) in s.layout.ore_cells.iter().enumerate() {
        let j = s
            .layout
            .objects
            .iter()
            .position(|o| {
                o.type_name == "OreNode"
                    && (o.cell[0] - point[0]).hypot(o.cell[1] - point[1]) < 0.001
            })
            .ok_or_else(|| err("ore metadata has no native identity"))?;
        if layout.objects[j].angle != s.layout.objects[j].angle {
            layout.ore_angles[i] = -(layout.objects[j].angle as f64);
        }
    }
    if layout.base_cells.len() != 8 || layout.ore_cells.len() != layout.ore_angles.len() {
        return Err(err("invalid coordinate base/ore metadata"));
    }
    Ok(layout)
}
pub fn layout(p: &Project) -> Result<Layout> {
    let s = source(p)?;
    layout_with_source(p, &s)
}
fn projected(p: &Project) -> Result<Projection> {
    let s = source(p)?;
    let layout = layout_with_source(p, &s)?;
    let mut m = Arc::clone(&s.map);
    let mut objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    if objects.len() != layout.objects.len() {
        return Err(err("source native/layout object count differs"));
    }
    let mut changed = false;
    let mut moved_waypoint = false;
    for (i, (o, spec)) in objects.iter_mut().zip(&layout.objects).enumerate() {
        let original = &s.layout.objects[i];
        if spec.cell != original.cell {
            let object = spec.object(p);
            moved_waypoint |=
                o.type_name.contains("Waypoint") && o.position[..2] != object.position[..2];
            changed |= o.position[..2] != object.position[..2];
            o.position[0] = object.position[0];
            o.position[1] = object.position[1];
        }
        changed |= o.angle != spec.angle;
        o.angle = spec.angle;
    }
    if changed {
        let payload = map::write_objects(&objects, &m)?;
        let native = Arc::make_mut(&mut m);
        native.get_mut("ObjectsList")?.payload = payload;
        crate::gameplay::gift_clearance::sync_moved_areas(
            native,
            &map::objects(s.map.get("ObjectsList")?, &s.map.table)?,
            &objects,
        )?;
    }
    if p.cfg["NATIVE_CELL_EDITS"].is_string() {
        crate::terrain::native_cell_edits::apply(p, Arc::make_mut(&mut m))?;
    }
    if moved_waypoint {
        crate::gameplay::production_facing::apply(p, Arc::make_mut(&mut m))?;
    }
    let terrain_xml = if p.name != s.settings.source_name {
        let xml = s
            .files
            .iter()
            .find(|(n, _)| n == "map.xml")
            .ok_or_else(|| err("coordinate source has no map.xml"))?;
        let finalized = crate::resources::finalize_terrain_package(
            p,
            Arc::make_mut(&mut m),
            std::str::from_utf8(&xml.1)?,
            &p.version,
            crate::resources::TerrainFinalization::PreserveLookups,
        )?;
        Some(finalized.xml)
    } else {
        None
    };
    Ok(Projection {
        source: s,
        layout,
        map: m,
        terrain_xml,
    })
}
fn native_terrain(p: &Project, m: &MapFile, layout: Layout) -> Result<Terrain> {
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let n = hm.width;
    let raw = terrain::flip(&hm.elevations, n);
    let z = raw
        .iter()
        .map(|v| *v as f64 * p.number("Z_SCALE", 0.0390625))
        .collect::<Vec<_>>();
    Ok(Terrain {
        n,
        raw,
        land: z.iter().map(|v| *v > p.number("WATER_Z", 200.)).collect(),
        slope: terrain::gradient(&z, n),
        z,
        layout,
    })
}
fn native_surface(t: &Terrain, m: &MapFile) -> Result<Surface> {
    let bt = BlendTiles::parse(m.get("BlendTileData")?, t.n, t.height())?;
    Ok(Surface {
        chosen: terrain::flip(&bt.tiles, t.n)
            .iter()
            .map(|v| v / 64)
            .collect(),
        impassable: terrain::flip(&bt.impassability, t.n),
        build_block: terrain::flip(&bt.buildability, t.n),
        layer_edge: t.slope.iter().map(|v| *v > 20.).collect(),
        shore: vec![false; t.raw.len()],
        textures: bt
            .textures
            .iter()
            .map(|s| [s.clone(), s.clone(), String::new()])
            .collect(),
    })
}
fn policy(p: &Project, _s: &Settings) -> Result<Policy> {
    crate::terrain::spatial::policy(p)
}
pub(crate) fn spatial_policy(p: &Project) -> Result<Policy> {
    crate::terrain::spatial::policy(p)
}
fn source_clearance(p: &Project, m: &MapFile, layout: &Layout, s: &Source) -> Result<Option<Json>> {
    let mut checked =
        crate::terrain::ramp_clearance::validate_transition(p, &s.map, &s.layout, m, layout)?;
    if let Some(report) = checked.as_mut()
        && report["status"] == "compared_with_source"
    {
        report["source_crc32"] = json!(s.settings.source_crc32);
    }
    Ok(checked)
}
fn source_spawns(p: &Project, m: &MapFile, layout: &Layout, source: &Source) -> Result<Json> {
    let mut checked = crate::terrain::spawn_placement::validate_transition(
        p,
        &source.map,
        &source.layout,
        m,
        layout,
    )?;
    checked["source_crc32"] = json!(source.settings.source_crc32);
    Ok(checked)
}
fn analyse(
    p: &Project,
    m: &MapFile,
    layout: &Layout,
    source: Option<&Source>,
) -> Result<(Json, Vec<Issue>)> {
    let ramp_clearance = if let Some(source) = source {
        source_clearance(p, m, layout, source)?
    } else {
        crate::terrain::ramp_clearance::validate(p, m, layout)?
    };
    let spawn_placement = if let Some(source) = source {
        source_spawns(p, m, layout, source)?
    } else {
        // Initial adoption has prospective source settings, but no frozen
        // package exists yet. Validate the new layout before creating it.
        let mut initial = p.clone();
        initial
            .cfg
            .as_object_mut()
            .unwrap()
            .remove("COORDINATE_PIPELINE");
        crate::terrain::spawn_placement::validate(&initial, m, layout)?
    };
    let resource_distribution = if let Some(source) = source {
        crate::terrain::resource_routes::validate_transition(p, &source.map, m)?
    } else {
        // Prospective settings do not prove that a frozen source exists.
        // Every allocation must meet the current public policy before adoption.
        let mut initial = p.clone();
        initial
            .cfg
            .as_object_mut()
            .unwrap()
            .remove("COORDINATE_PIPELINE");
        crate::terrain::resource_routes::validate(&initial, m)?
    };
    let s = settings(p)?;
    let scene = Scene::load(p, m, policy(p, &s)?)?;
    let mut issues = scene.inspect(layout)?;
    let (ramp_profiles, ramp_issues) = scene.ramp_profiles(layout)?;
    issues.extend(ramp_issues);
    let (near_ores, quota_issues) = scene.near_ores(p, layout)?;
    issues.extend(quota_issues);
    let (routes, route_issues) = scene.routes(m)?;
    issues.extend(route_issues);
    let dir = p.assets.join("spatial");
    fs::create_dir_all(&dir)?;
    project::write_json(&dir.join("routes.json"), &routes)?;
    let mut csv = String::from(
        "\u{feff}seat,start_id,target_id,target_type,x,y,reachable,path_steps,world_distance,straight_distance_world,domain,route_status\n",
    );
    for r in &routes {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r["seat"],
            r["start_id"],
            r["target_id"],
            r["target_type"],
            r["target"][0],
            r["target"][1],
            r["reachable"],
            r["path_steps"]
                .as_u64()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r["world_distance"]
                .as_f64()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r["straight_distance_world"],
            r["domain"],
            r["route_status"]
        ));
    }
    fs::write(dir.join("routes.csv"), csv)?;
    let textures = spatial::materials(p, m)?;
    let mut counts = BTreeMap::<String, usize>::new();
    for i in &issues {
        *counts.entry(i.rule.clone()).or_default() += 1;
    }
    let report = json!({"schema_version":1,"map":p.name,"source_crc32":s.source_crc32,"native_crc32":crc32fast::hash(&m.encode()?),"frame":scene.frame,"policy":scene.policy,"geometry_model":"conservative circular reservation/obstacle radii, not extracted game meshes","route_model":"four-neighbor static land/water shortest paths; mixed surface pairs require transport and have no inferred distance; native impassability + object obstacles + clearance + per-edge height step; no inference about runtime AI decisions","entities":scene.entities,"counts":{"objects":scene.entities.len(),"ramps":layout.ramps.len(),"routes":routes.len(),"reachable_routes":routes.iter().filter(|r|r["reachable"]==true).count(),"unreachable_routes":routes.iter().filter(|r|r["reachable"]==false).count(),"requires_transport_routes":routes.iter().filter(|r|r["route_status"]=="requires_transport").count(),"issues_by_rule":counts},"textures":textures,"ramp_profiles":ramp_profiles,"near_ores":near_ores,"resource_distribution":resource_distribution,"issues":issues});
    let mut report = report;
    report["ramp_clearance"] = serde_json::to_value(ramp_clearance)?;
    report["spawn_placement"] = spawn_placement;
    Ok((report, issues))
}
pub fn audit(p: &Project) -> Result<Json> {
    audit_projected(p, &projected(p)?)
}
fn audit_projected(p: &Project, projection: &Projection) -> Result<Json> {
    let Projection {
        source: s,
        layout,
        map: m,
        ..
    } = projection;
    project::validate_current_rules(&p.cfg)?;
    let baseline_bytes = fs::read(p.path(&s.settings.baseline)?)?;
    let mut key = crc32fast::Hasher::new();
    key.update(b"coordinate-audit-v13-current-strict-policy");
    key.update(&serde_json::to_vec(
        &crate::terrain::resource_routes::binding(p)?,
    )?);
    key.update(&serde_json::to_vec(
        &crate::terrain::ramp_clearance::policy(p)?,
    )?);
    key.update(&serde_json::to_vec(&p.cfg.get("RAMP_CLEARANCE"))?);
    key.update(&serde_json::to_vec(
        &crate::terrain::spawn_placement::policy(p)?,
    )?);
    key.update(&crate::terrain::spawn_placement::RULES_VERSION.to_le_bytes());
    key.update(&serde_json::to_vec(
        &p.cfg.get("SPAWN_ORIENTATION_EXCEPTION"),
    )?);
    key.update(&serde_json::to_vec(&p.cfg.get("SPAWN_PLACEMENT"))?);
    key.update(&serde_json::to_vec(&p.cfg.get("WATER_Z"))?);
    key.update(&serde_json::to_vec(&p.flag("TERRAIN_ONLY"))?);
    key.update(&s.settings.source_crc32.to_le_bytes());
    if let Some(bytes) = crate::terrain::resource_routes::input(p)? {
        key.update(&bytes);
    }
    key.update(p.name.as_bytes());
    key.update(&m.encode()?);
    key.update(&serde_json::to_vec(layout)?);
    key.update(&serde_json::to_vec(&policy(p, &s.settings)?)?);
    key.update(&baseline_bytes);
    let input_crc32 = key.finalize();
    if let Ok(bytes) = fs::read(p.assets.join("spatial/report.json"))
        && let Ok(cached) = serde_json::from_slice::<Json>(&bytes)
        && cached["input_crc32"] == input_crc32
        && cached["passed"] == true
        && cached["spawn_placement"]["passed"] == true
        && let Some(artifacts) = cached["artifacts_crc32"].as_object()
        && artifacts.iter().all(|(name, crc)| {
            fs::read(p.assets.join("spatial").join(name))
                .is_ok_and(|b| json!(crc32fast::hash(&b)) == *crc)
        })
    {
        println!("坐标检查：输入及导出校验一致，复用已通过结果");
        if !p.assets.join("spatial.zip").is_file() {
            reports_archive(p)?;
        }
        let mut reused = cached;
        reused["cache_hit"] = json!(true);
        return Ok(reused);
    }
    let (mut report, issues) = analyse(p, m, layout, Some(s))?;
    let baseline: Vec<Issue> = serde_json::from_slice(&baseline_bytes)?;
    let regressions = issues.clone();
    report["regressions"] = serde_json::to_value(&regressions)?;
    report["passed"] = json!(regressions.is_empty());
    report["baseline_issues"] = json!(baseline.len());
    report["inherited_issues"] = json!(issues.len() - regressions.len());
    report["input_crc32"] = json!(input_crc32);
    report["cache_hit"] = json!(false);
    let mut md = format!(
        "# {} 坐标制作检查\n\n检查结果：{}。原生对象 {}，斜坡 {}，资源路线关系 {}。\n\n当前问题 {}，继承放行 {}；全部成品统一要求零问题。\n\n",
        p.name,
        if regressions.is_empty() {
            "通过严格检查"
        } else {
            "拒绝发布"
        },
        report["counts"]["objects"],
        report["counts"]["ramps"],
        report["counts"]["routes"],
        regressions.len(),
        issues.len() - regressions.len()
    );
    md.push_str("对象坐标是唯一编辑输入，矿位/医院等分类清单随对象身份派生。斜坡地形通过原生局部编辑器更新，随后显式采用新成品；普通构建保留来源高度、纹理、位图、脚本和资源身份。\n\n占地采用可配置的保守圆形模型；路线是静态四邻接陆路/水路估算，跨水陆关系标记为需要运输，包含装饰/建筑障碍与通行余量，不代表游戏寻路或 AI 实机行为。\n\n- `report.json`：对象占地、冲突坐标、继承与新增问题。\n- `routes.csv/json`：八个出生点到矿、油井和科技建筑的陆路/水路可达性与路径长度；跨水陆关系不推断距离。\n- `texture-coordinates.csv/json`：逐格基础材质、二/三路混合及悬崖纹理索引，描述字节保留原编号。\n- `../coordinates/index.html`：完整坐标交互地图。\n\n");
    for i in &regressions {
        md.push_str(&format!(
            "- **新增/恶化** {}，对象 {:?}，坐标 {:?}：{}\n",
            i.rule, i.ids, i.at, i.detail
        ));
    }
    md.push_str(
        "\n| 状态 | 检查项 | 对象编号 | 坐标 X,Y | 测量值 | 说明 |\n|---|---|---|---|---|---|\n",
    );
    let mut csv = String::from("\u{feff}status,rule,object_ids,x,y,metric,detail\n");
    for issue in &issues {
        let status = if regressions.iter().any(|r| r.key == issue.key) {
            "新增/恶化"
        } else {
            "来源既有"
        };
        md.push_str(&format!(
            "| {status} | {} | {:?} | {:.3},{:.3} | {:.3} | {} |\n",
            issue.rule, issue.ids, issue.at[0], issue.at[1], issue.metric, issue.detail
        ));
        csv.push_str(&format!(
            "\"{status}\",\"{}\",\"{:?}\",{},{},{},\"{}\"\n",
            issue.rule,
            issue.ids,
            issue.at[0],
            issue.at[1],
            issue.metric,
            issue.detail.replace('"', "\"\"")
        ));
    }
    fs::write(p.assets.join("spatial/issues.csv"), csv)?;
    fs::write(p.assets.join("spatial/README.md"), md)?;
    let artifacts = [
        "routes.json",
        "routes.csv",
        "texture-coordinates.json",
        "texture-coordinates.csv",
        "issues.csv",
        "README.md",
    ]
    .iter()
    .map(|name| {
        Ok((
            name.to_string(),
            crc32fast::hash(&fs::read(p.assets.join("spatial").join(name))?),
        ))
    })
    .collect::<Result<BTreeMap<_, _>>>()?;
    report["artifacts_crc32"] = serde_json::to_value(artifacts)?;
    project::write_json(&p.assets.join("spatial/report.json"), &report)?;
    reports_archive(p)?;
    if !regressions.is_empty() {
        return Err(err(format!(
            "coordinate checks rejected {} current issues; see spatial/report.json",
            regressions.len()
        )));
    }
    println!(
        "坐标检查：{} 个对象、{} 条坡、{} 条资源路线关系；当前问题 {}，严格验收通过",
        report["counts"]["objects"],
        report["counts"]["ramps"],
        report["counts"]["routes"],
        issues.len()
    );
    Ok(report)
}
fn reports_archive(p: &Project) -> Result<()> {
    let entries = [
        "README.md",
        "report.json",
        "issues.csv",
        "routes.json",
        "routes.csv",
        "texture-coordinates.json",
        "texture-coordinates.csv",
    ]
    .iter()
    .map(|name| {
        Ok((
            format!("spatial/{name}"),
            fs::read(p.assets.join("spatial").join(name))?,
        ))
    })
    .collect::<Result<Vec<_>>>()?;
    project::atomic_write(&p.assets.join("spatial.zip"), &archive::zip(&entries)?)
}
pub fn prepare(p: &Project) -> Result<()> {
    crate::coordinates::prepare(p)?;
    let Projection { layout, map: m, .. } = projected(p)?;
    native_terrain(p, &m, layout)?.save(p)
}
pub fn assemble(p: &Project) -> Result<()> {
    crate::coordinates::prepare(p)?;
    let projection = projected(p)?;
    audit_projected(p, &projection)?;
    let Projection {
        source: s,
        layout,
        map: m,
        terrain_xml,
    } = projection;
    let original = map::objects(s.map.get("ObjectsList")?, &s.map.table)?;
    let actual = map::objects(m.get("ObjectsList")?, &m.table)?;
    let changed = original != actual;
    if changed && p.name == s.settings.source_name {
        return Err(err(
            "coordinate edits change game content; rebuild with --bump",
        ));
    }
    let changed_art = if changed {
        let t = native_terrain(p, &m, layout.clone())?;
        Some(render::art_image(&t, p, &native_surface(&t, &m)?).tga_bytes()?)
    } else {
        None
    };
    let native_changed = m.encode()? != s.map.encode()?;
    let mut files = Vec::new();
    for (n, bytes) in &s.files {
        let out = if n == &format!("{}.map", s.settings.source_name) {
            format!("{}.map", p.name)
        } else if n == &format!("{}_art.tga", s.settings.source_name) {
            format!("{}_art.tga", p.name)
        } else {
            n.clone()
        };
        let value = if n.ends_with(".map") && native_changed {
            crate::formats::refpack::pack(&m.encode()?)?
        } else if n.ends_with("_art.tga") && changed {
            changed_art
                .as_ref()
                .expect("changed object minimap")
                .clone()
        } else if n == "map.xml" && (changed || p.name != s.settings.source_name) {
            let xml = crate::gameplay::production_spawns::update_xml(
                terrain_xml
                    .as_deref()
                    .ok_or_else(|| err("missing finalized coordinate XML"))?,
                &s.map,
                &m,
            )?;
            crate::pipeline::release::retarget_xml(&xml, &s.settings.source_name, &p.name)?
                .replace(
                    &format!("DisplayName=\"{}\"", s.settings.source_name),
                    &format!("DisplayName=\"{}\"", p.name),
                )
                .into_bytes()
        } else if n == "map.str" && p.name != s.settings.source_name {
            std::str::from_utf8(bytes)?
                .replace(&s.settings.source_name, &p.name)
                .into_bytes()
        } else {
            bytes.clone()
        };
        files.push((out, value));
    }
    crate::pipeline::release::validate_runtime_files(&files)?;
    crate::pipeline::scene::verify_files(p, &m, &files)?;
    // Check all inputs before writing any game file; identical rebuilds keep exact bytes.
    for (n, bytes) in files {
        project::atomic_write(&p.assets.join(n), &bytes)?;
    }
    native_terrain(p, &m, layout)?.save(p)?;
    Ok(())
}
pub fn verify(p: &Project) -> Result<MapFile> {
    let Projection {
        source,
        layout,
        map: expected,
        ..
    } = projected(p)?;
    let m = MapFile::load(map_path(p))?;
    crate::gameplay::ore_facing::verify(p, &m)?;
    if m.encode()? != expected.encode()? {
        return Err(err(
            "native map differs from coordinate inputs; adopt the current complete package before rebuilding",
        ));
    }
    let files = build::release_files(p)
        .iter()
        .map(|n| Ok((n.clone(), fs::read(p.assets.join(n))?)))
        .collect::<Result<Vec<_>>>()?;
    crate::pipeline::release::validate_runtime_files(&files)?;
    crate::gameplay::gift_clearance::validate(p, &m)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(m.get("BlendTileData")?, hm.width, hm.height)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    crate::gameplay::script_spawns::validate(p, &m, &hm, &bt, &objects)?;
    crate::gameplay::production_facing::validate(p, &m)?;
    crate::gameplay::periodic_attack::validate(p, &m)?;
    crate::terrain::resource_routes::validate(p, &m)?;
    source_clearance(p, &m, &layout, &source)?;
    source_spawns(p, &m, &layout, &source)?;
    let xml = crate::pipeline::scene::read_xml(p)?;
    crate::pipeline::scene::verify(p, &m, xml.as_deref())?;
    Ok(m)
}
pub fn accept(p: &Project) -> Result<()> {
    verify(p)?;
    audit(p)?;
    let art = render::Image::load_tga(&p.assets.join(format!("{}_art.tga", p.name)))?;
    if art.width != 512 || art.height != 512 {
        return Err(err("coordinate pipeline requires a 512x512 minimap"));
    }
    Ok(())
}
pub fn preview(p: &Project) -> Result<()> {
    let Projection { layout, map: m, .. } = projected(p)?;
    let t = native_terrain(p, &m, layout)?;
    render::previews(&t, p, &native_surface(&t, &m)?)?;
    Ok(())
}
pub fn readme(p: &Project) -> Result<()> {
    fs::write(
        p.assets.join("README.md"),
        format!(
            "# {}\n\n基于当前定稿成品的坐标制作流水，保留原生地形、纹理、位图与脚本；坐标占地、资源陆路和材质统计参与增量验收。\n\n[坐标地图](coordinates/index.html) · [制作检查](spatial/README.md) · [详细检查](spatial/report.json) · [资源路线](spatial/routes.csv)\n",
            p.name
        ),
    )?;
    Ok(())
}
/// Explicitly freeze a complete current release and its verified full layout as authoring inputs.
pub fn adopt(p: &Project) -> Result<()> {
    let initial = !enabled(p);
    let mut configured = p.clone();
    let previous = if initial { None } else { Some(source(p)?) };
    if initial {
        configured.cfg["COORDINATE_PIPELINE"] = json!({
            "source_package":"authoring/source.zip","layout":"authoring/layout.json","baseline":"authoring/baseline.json",
            "source_name":p.name,"source_crc32":0
        });
    }
    let p = &configured;
    let settings = settings(p)?;
    let mut files = build::release_files(p)
        .iter()
        .map(|n| Ok((n.clone(), fs::read(p.assets.join(n))?)))
        .collect::<Result<Vec<_>>>()?;
    let m = crate::pipeline::release::load_runtime_map(&files)?;
    let bytes = &files
        .iter()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap()
        .1;
    let saved: Layout = serde_json::from_slice(&fs::read(p.source().join("layout.json"))?)?;
    let (layout, id_map) = crate::coordinates::native_layout(p, &m, &saved)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let frame = crate::coordinates::Frame::from_heightmap(p, &hm)?;
    if frame != crate::coordinates::Frame::from_project(p) {
        return Err(err(
            "cannot adopt a native map with a different coordinate frame",
        ));
    }
    if objects.len() != layout.objects.len() {
        return Err(err("adopt requires a complete native source/layout.json"));
    }
    for (o, s) in objects.iter().zip(&layout.objects) {
        let c = frame.grid([o.position[0] as f64, o.position[1] as f64]);
        if o.type_name != s.type_name || (c[0] - s.cell[0]).hypot(c[1] - s.cell[1]) > 0.001 {
            return Err(err("adopt layout does not match native objects"));
        }
    }
    if initial {
        let mut initial_project = p.clone();
        initial_project
            .cfg
            .as_object_mut()
            .unwrap()
            .remove("COORDINATE_PIPELINE");
        crate::terrain::ramp_clearance::validate(&initial_project, &m, &layout)?;
        crate::terrain::spawn_placement::validate(&initial_project, &m, &layout)?;
        crate::terrain::resource_routes::validate(&initial_project, &m)?;
    } else {
        let previous = previous.as_ref().unwrap();
        let actual = crate::terrain::ramp_clearance::geometry_crc32(p, &m, &layout)?;
        let using_source_ramps =
            crate::terrain::ramp_clearance::geometry_crc32(p, &m, &previous.layout)?;
        if actual != using_source_ramps {
            let proof = build::placement_proof(p, bytes)?.ok_or_else(|| {
                err("changed ramp definitions require a validated native edit before adoption")
            })?;
            if proof["placement_geometry_crc32"] != actual {
                return Err(err(
                    "adopted ramp layout differs from validated native edit geometry",
                ));
            }
        }
    }
    let mut audit_project = p.clone();
    if initial {
        audit_project.cfg["RAMP_CLEARANCE"] =
            serde_json::to_value(crate::terrain::ramp_clearance::policy(p)?)?;
    }
    let (_, issues) = analyse(&audit_project, &m, &layout, previous.as_ref())?;
    if !issues.is_empty() {
        return Err(err(format!(
            "cannot adopt source with {} current spatial issues",
            issues.len()
        )));
    }
    let mut scene_project = p.clone();
    if initial {
        scene_project
            .cfg
            .as_object_mut()
            .unwrap()
            .remove("COORDINATE_PIPELINE");
    }
    if build::placement_proof(&scene_project, bytes)?.is_some() {
        scene_project = crate::pipeline::scene::incremental_context(&scene_project)?;
    }
    crate::pipeline::scene::verify_files(&scene_project, &m, &files)?;
    if !scene_project.flag("TERRAIN_ONLY") {
        crate::gameplay::gift_clearance::validate_with_layout(&scene_project, &m, &layout)?;
        crate::gameplay::production_spawns::validate_final_with_layout(
            &scene_project,
            &m,
            &layout,
        )?;
    }
    if let Err(error) =
        crate::pipeline::acceptance::require_delivery_with_layout(&scene_project, &files, &layout)
    {
        let candidate =
            crate::pipeline::acceptance::stage_candidate(&scene_project, &files, &layout)?;
        return Err(err(format!(
            "{error}; review candidate {} before adopting its source",
            candidate.display()
        )));
    }
    crate::gameplay::placement::validate_final_access(&scene_project, &m)?;
    let selected_layers = crate::terrain::layers::SelectedLayers::load(p)?;
    let source_crc32 = crc32fast::hash(bytes);
    files.push(("layout.json".into(), serde_json::to_vec_pretty(&layout)?));
    let package = archive::zip(&files)?;
    let config_path = p.map_dir().join("config.json");
    let mut cfg = p.cfg.clone();
    cfg["COORDINATE_PIPELINE"]["source_name"] = json!(p.name);
    cfg["COORDINATE_PIPELINE"]["source_crc32"] = json!(source_crc32);
    project::atomic_write(&p.path(&settings.source_package)?, &package)?;
    project::write_json(&p.path(&settings.layout)?, &layout)?;
    project::write_json(&p.path(&settings.baseline)?, &issues)?;
    project::write_json(&p.path("authoring/coordinate-id-map.json")?, &id_map)?;
    project::write_json(
        &p.path(&format!("histories/{}/coordinate-id-map.json", p.version))?,
        &id_map,
    )?;
    project::write_json(&config_path, &cfg)?;
    selected_layers
        .sync_current(p)
        .map_err(|e| err(format!("采用制作源已完成，蓝图同步失败：{e}")))?;
    println!("已采用 {} 的完整原生包、对象身份和坐标检查基准", p.name);
    Ok(())
}
