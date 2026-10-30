//! Content-bound independent delivery decisions. Geometry audits remain separate.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, HeightMap, MapFile},
    gameplay::{placement, scripts},
    pipeline::project::{self, Project},
    resources::official,
    terrain::Layout,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

/// Bump when the independent matrix or its required decision semantics change.
pub const RULES_REVISION: &str =
    "independent-delivery-v2-full-functional-reservations-terrain-transitions";
pub const ITEMS: [(&str, &str); 13] = [
    ("spawn_layout", "出生座位布局"),
    ("attack_routes", "对位进攻方向与沿线布置"),
    ("route_width_intrusions", "交战路线宽度与伸入地形"),
    ("gift_intrusion_exclusion", "赠送建筑避开伸入区域"),
    ("production_base_proximity", "赠送生产建筑靠近基地"),
    ("ore_distribution", "布矿"),
    ("neutral_building_siting", "中立建筑选址"),
    ("ramp_clearance", "坡口"),
    ("ramp_priority", "斜坡优先于矿"),
    ("resource_access", "交战区与资源通行便利"),
    ("shore_perimeter", "整圈岸线"),
    ("hotspot_distances", "热点双方实际路程"),
    ("building_angles", "建筑朝向（含矿脉）"),
];

/// A configured record belongs to this map; defaults belong to this release.
pub fn report_path(p: &Project) -> Result<PathBuf> {
    match p.cfg.get("DELIVERY_ACCEPTANCE") {
        Some(value) => {
            let relative = value
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| err("DELIVERY_ACCEPTANCE must name a map-relative JSON record"))?;
            p.path(relative)
        }
        None => Ok(p.assets.join("delivery-acceptance.json")),
    }
}

fn canonical_json_crc32(value: &Value) -> Result<u32> {
    Ok(crc32fast::hash(&serde_json::to_vec(value)?))
}

fn json_input(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn parameters(p: &Project) -> Result<Value> {
    let mut settings = p.cfg.clone();
    let settings = settings
        .as_object_mut()
        .ok_or_else(|| err("delivery acceptance requires an object recipe"))?;
    // Source adoption changes these pointers, not the accepted candidate.
    for key in [
        "DELIVERY_ACCEPTANCE",
        "COORDINATE_PIPELINE",
        "LAYOUT",
        "LAYERS",
        "RESOURCE_DISTRIBUTION",
    ] {
        settings.remove(key);
    }
    let common = match fs::read(p.root.join("rules/coordinate-pipeline.json")) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            serde_json::from_str(include_str!("../../rules/coordinate-pipeline.json"))?
        }
        Err(error) => return Err(error.into()),
    };
    let production = json_input(&p.root.join("rules/common-T0.5.1.json"))?;
    let policy_override = p.cfg["COORDINATE_PIPELINE"]
        .get("policy")
        .filter(|value| !value.is_null())
        .map(|value| {
            let relative = value
                .as_str()
                .ok_or_else(|| err("spatial policy path must be a string"))?;
            json_input(&p.path(relative)?)
        })
        .transpose()?;
    let plan = p
        .cfg
        .get("RESOURCE_DISTRIBUTION")
        .filter(|value| !value.is_null())
        .map(|value| {
            let relative = value
                .as_str()
                .ok_or_else(|| err("resource distribution path must be a string"))?;
            json_input(&p.path(relative)?)
        })
        .transpose()?;
    Ok(json!({
        "recipe":settings,
        "common_spatial":common,
        "spatial_override":policy_override,
        "production_placement":production.get("production_placement"),
        "resource_plan":plan
    }))
}

fn candidate<'a>(p: &Project, files: &'a [(String, Vec<u8>)]) -> Result<(&'a [u8], MapFile)> {
    let mut names = std::collections::BTreeSet::new();
    let mut native = None;
    for (name, bytes) in files {
        project::safe_component(name)?;
        if !names.insert(name) {
            return Err(err(format!("duplicate delivery candidate file {name}")));
        }
        if name.ends_with(".map") {
            if native.is_some() || name != &format!("{}.map", p.name) {
                return Err(err(
                    "delivery candidate must have exactly its same-version native map",
                ));
            }
            native = Some(bytes.as_slice());
        }
    }
    let native = native.ok_or_else(|| err("delivery candidate has no native map"))?;
    Ok((native, MapFile::load_bytes(native)?))
}

fn layout_matches(p: &Project, native: &MapFile, layout: &Layout) -> Result<Frame> {
    let frame = Frame::from_heightmap(p, &HeightMap::parse(native.get("HeightMapData")?)?)?;
    let objects = map::objects(native.get("ObjectsList")?, &native.table)?;
    if layout.schema_version != 1
        || layout.objects.len() != objects.len()
        || layout.objects.iter().zip(&objects).any(|(saved, actual)| {
            let point = frame.grid([actual.position[0] as f64, actual.position[1] as f64]);
            saved.type_name != actual.type_name
                || saved.version != actual.version
                || saved.road_type != actual.road_type
                || saved.angle != actual.angle
                || saved.props != actual.props
                || (saved.cell[0] - point[0]).hypot(saved.cell[1] - point[1]) > 0.001
        })
    {
        return Err(err(
            "independent delivery acceptance requires the complete candidate layout",
        ));
    }
    Ok(frame)
}

/// Compute identity without depending on outputs that publication has not written.
pub fn binding(p: &Project, files: &[(String, Vec<u8>)], layout: &Layout) -> Result<Value> {
    let (bytes, native) = candidate(p, files)?;
    let frame = layout_matches(p, &native, layout)?;
    if p.flag("TERRAIN_ONLY") {
        let facts = applicability(p, &native)?;
        if !layout.objects.is_empty()
            || !layout.base_cells.is_empty()
            || !layout.ore_cells.is_empty()
            || facts.gifts
            || facts.unresolved_creations
        {
            return Err(err(
                "TERRAIN_ONLY delivery must be a design artifact without playable objects or building creation scripts",
            ));
        }
    }
    let package = files
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                json!({"crc32":crc32fast::hash(bytes),"bytes":bytes.len()}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(json!({
        "native_crc32":crc32fast::hash(bytes),
        "native_decoded_crc32":crc32fast::hash(&native.encode()?),
        "package_files":package,
        "frame":frame,
        "layout_crc32":canonical_json_crc32(&serde_json::to_value(layout)?)?,
        "parameters_crc32":canonical_json_crc32(&parameters(p)?)?,
        "rules_revision":RULES_REVISION
    }))
}

/// A review starting point, never an automatically passing acceptance record.
pub fn pending_record(p: &Project, files: &[(String, Vec<u8>)], layout: &Layout) -> Result<Value> {
    let items = ITEMS
        .iter()
        .map(|(id, name)| {
            (
                (*id).to_owned(),
                json!({"name":name,"status":"pending","decision":"","evidence":[]}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(json!({
        "schema_version":1,
        "kind":"independent_delivery_acceptance",
        "rules_revision":RULES_REVISION,
        "map_id":p.id,"map":p.name,"version":p.version,
        "status":"pending","reviewer":"","reviewed_at":"",
        "binding":binding(p,files,layout)?,"items":items
    }))
}

/// Preserve an unpublished candidate for review without changing release state.
/// Existing files and human-edited records are never replaced with a template.
pub fn stage_candidate(
    p: &Project,
    files: &[(String, Vec<u8>)],
    layout: &Layout,
) -> Result<PathBuf> {
    let record = pending_record(p, files, layout)?;
    let identity = canonical_json_crc32(&record["binding"])?;
    let directory = p
        .map_dir()
        .join("data/delivery-candidates")
        .join(&p.name)
        .join(format!("{identity:08x}"));
    let root = p.root.canonicalize()?;
    let ancestor = directory
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| err("delivery candidate has no existing project ancestor"))?;
    if !ancestor.canonicalize()?.starts_with(&root) {
        return Err(err(
            "delivery candidate staging directory leaves the project",
        ));
    }
    fs::create_dir_all(directory.join("source"))?;
    if !directory.canonicalize()?.starts_with(&root)
        || !directory.join("source").canonicalize()?.starts_with(&root)
    {
        return Err(err(
            "delivery candidate staging directory leaves the project",
        ));
    }
    let layout_bytes = serde_json::to_vec_pretty(layout)?;
    let mut outputs = files
        .iter()
        .map(|(name, bytes)| (directory.join(name), bytes.as_slice()))
        .collect::<Vec<_>>();
    outputs.push((directory.join("source/layout.json"), &layout_bytes));
    for (path, bytes) in &outputs {
        if path.exists() {
            contained_file(&root, path)?;
            if fs::read(path)? != *bytes {
                return Err(err(format!(
                    "refusing to overwrite a different staged delivery candidate: {}",
                    path.display()
                )));
            }
        }
    }
    for (path, bytes) in outputs {
        if !path.exists() {
            project::atomic_write(&path, bytes)?;
        }
    }
    let pending = directory.join("delivery-acceptance.pending.json");
    if pending.exists() {
        contained_file(&root, &pending)?;
    } else {
        project::write_json(&pending, &record)?;
    }
    Ok(pending)
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| err(format!("independent delivery acceptance missing {field}")))
}

fn contained_file(root: &Path, path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(err(format!(
            "delivery acceptance file leaves the project: {}",
            path.display()
        )));
    }
    Ok(canonical)
}

fn evidence(root: &Path, parent: &Path, item: &Value) -> Result<()> {
    let files = item["evidence"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            err("independent delivery acceptance requires evidence files for every item")
        })?;
    for file in files {
        let relative = Path::new(text(file, "path")?);
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| matches!(part, Component::Prefix(_) | Component::RootDir))
        {
            return Err(err(
                "delivery evidence paths must be relative to the review record",
            ));
        }
        let path = contained_file(root, &parent.join(relative))?;
        let bytes = fs::read(path)?;
        if file["crc32"].as_u64() != Some(u64::from(crc32fast::hash(&bytes)))
            || file["bytes"].as_u64() != Some(bytes.len() as u64)
        {
            return Err(err(format!(
                "delivery evidence changed: {}",
                relative.display()
            )));
        }
    }
    Ok(())
}

#[derive(Default)]
struct Applicability {
    gifts: bool,
    production: bool,
    neutral: bool,
    shore: bool,
    unresolved_creations: bool,
}

fn applicability(p: &Project, native: &MapFile) -> Result<Applicability> {
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let mut result = Applicability {
        shore: hm.elevations.iter().any(|height| {
            f64::from(*height) * p.number("Z_SCALE", 0.0390625) <= p.number("WATER_Z", 200.)
        }),
        ..Default::default()
    };
    // Unknown/nonempty water declarations require an independent shore decision.
    for (name, version) in [("StandingWaterAreas", 2), ("RiverAreas", 3)] {
        if let Ok(chunk) = native.get(name) {
            result.shore |=
                chunk.version != version || chunk.payload.as_slice() != 0_u32.to_le_bytes();
        }
    }
    for object in map::objects(native.get("ObjectsList")?, &native.table)? {
        if object.type_name.contains("Waypoint") || object.type_name == "OreNode" {
            continue;
        }
        match official::object(&object.type_name) {
            Some(definition) if definition.pending.is_empty() => {
                if definition.kind_of.iter().any(|flag| flag == "STRUCTURE") {
                    let owner = map::prop(&object.props, "playerName").and_then(map::Value::text);
                    result.neutral |= owner.is_none_or(|owner| owner == "PlyrCivilian");
                }
            }
            _ => result.neutral = true,
        }
    }
    fn visit(nodes: &[scripts::Node], result: &mut Applicability) -> Result<()> {
        for node in nodes {
            if let Some(children) = &node.children {
                visit(children, result)?;
                continue;
            }
            let Some(command) = node.command.as_deref() else {
                result.unresolved_creations |=
                    matches!(node.asset.as_str(), "ScriptAction" | "ScriptActionFalse");
                continue;
            };
            if !command.starts_with("CREATE_") {
                continue;
            }
            let arguments = scripts::string_arguments(node)?;
            let kinds = arguments
                .iter()
                .filter(|(kind, _)| *kind == 15)
                .collect::<Vec<_>>();
            if kinds.is_empty() {
                result.unresolved_creations = true;
            }
            for (_, kind) in kinds {
                match official::object(kind) {
                    Some(definition) if definition.pending.is_empty() => {
                        if definition.kind_of.iter().any(|flag| flag == "STRUCTURE") {
                            result.gifts = true;
                            result.production |= placement::is_production(kind);
                            // Creation ownership is not inferred from its name.
                            result.neutral = true;
                        }
                    }
                    _ => result.unresolved_creations = true,
                }
            }
        }
        Ok(())
    }
    if let Ok(chunk) = native.get("PlayerScriptsList") {
        visit(
            &scripts::parse_nodes(&chunk.payload, &native.table)?,
            &mut result,
        )?;
    }
    Ok(result)
}

fn not_applicable_allowed(p: &Project, id: &str, layout: &Layout, facts: &Applicability) -> bool {
    if p.flag("TERRAIN_ONLY") {
        return match id {
            "route_width_intrusions" => false,
            "shore_perimeter" => !facts.shore,
            _ => true,
        };
    }
    match id {
        "gift_intrusion_exclusion" => !facts.gifts && !facts.unresolved_creations,
        "production_base_proximity" => !facts.production && !facts.unresolved_creations,
        "neutral_building_siting" => !facts.neutral && !facts.unresolved_creations,
        "shore_perimeter" => !facts.shore,
        "ramp_clearance" | "ramp_priority" => layout.ramps.is_empty(),
        _ => false,
    }
}

pub fn require_delivery_with_layout(
    p: &Project,
    files: &[(String, Vec<u8>)],
    layout: &Layout,
) -> Result<()> {
    let path = report_path(p)?;
    let root = p.root.canonicalize()?;
    let canonical = contained_file(&root, &path).map_err(|error| {
        err(format!(
            "independent delivery acceptance is required at {}: {error}; generate a pending record, complete its review, then retry publication",
            path.display()
        ))
    })?;
    let record: Value = serde_json::from_slice(&fs::read(&canonical)?)?;
    if record["schema_version"] != 1
        || record["kind"] != "independent_delivery_acceptance"
        || record["rules_revision"] != RULES_REVISION
        || record["map_id"] != p.id
        || record["map"] != p.name
        || record["version"] != p.version
    {
        return Err(err(
            "independent delivery acceptance has an obsolete schema, rule revision or release identity",
        ));
    }
    if record["binding"] != binding(p, files, layout)? {
        return Err(err(
            "independent delivery acceptance belongs to different candidate files, layout or effective parameters",
        ));
    }
    if record["status"] != "passed" {
        return Err(err("independent delivery acceptance is failed or pending"));
    }
    text(&record, "reviewer")?;
    chrono::DateTime::parse_from_rfc3339(text(&record, "reviewed_at")?)
        .map_err(|_| err("independent delivery acceptance reviewed_at must use RFC3339"))?;
    let items = record["items"]
        .as_object()
        .ok_or_else(|| err("independent delivery acceptance has no item matrix"))?;
    if items.len() != ITEMS.len()
        || items
            .keys()
            .any(|key| !ITEMS.iter().any(|(id, _)| id == key))
    {
        return Err(err(
            "independent delivery acceptance must contain the complete current item matrix",
        ));
    }
    let parent = canonical.parent().unwrap();
    let facts = if items
        .values()
        .any(|item| item["status"] == "not_applicable")
    {
        let (_, native) = candidate(p, files)?;
        applicability(p, &native)?
    } else {
        Applicability::default()
    };
    for (id, _) in ITEMS {
        let item = &items[id];
        text(item, "decision")
            .map_err(|_| err(format!("delivery item {id} has no independent decision")))?;
        match text(item, "status")? {
            "passed" => {}
            "not_applicable" if not_applicable_allowed(p, id, layout, &facts) => {
                text(item, "reason").map_err(|_| {
                    err(format!("delivery item {id} needs a not-applicable reason"))
                })?;
            }
            status => {
                return Err(err(format!(
                    "delivery item {id} cannot authorize publication: {status}"
                )));
            }
        }
        evidence(&root, parent, item)?;
    }
    Ok(())
}

/// Publication requires a real same-version snapshot, not an old blueprint.
pub fn require_delivery(p: &Project, files: &[(String, Vec<u8>)]) -> Result<()> {
    let layout = Layout::release_snapshot(p, &p.name)?
        .ok_or_else(|| err("delivery acceptance requires same-version source/layout.json"))?;
    require_delivery_with_layout(p, files, &layout)
}

/// Stage the current saved release without exposing internal snapshot loading.
pub fn stage_release(p: &Project, files: &[(String, Vec<u8>)]) -> Result<PathBuf> {
    let layout = Layout::release_snapshot(p, &p.name)?
        .ok_or_else(|| err("prepare-delivery requires same-version source/layout.json"))?;
    stage_candidate(p, files, &layout)
}
