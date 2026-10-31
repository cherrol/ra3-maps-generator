//! Immutable final-map inventory and shared, content-bound diagnostic artifacts.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, HeightMap, MapFile, Value, prop},
    gameplay::official_placement,
    pipeline::project::{self, Project},
    resources::{official, xml_structure},
    terrain::mobility,
};
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, fs};

const REVISION: &str = "final-native-scene-v6-strict-all-placement-targets";
const REPORT: &str = "scene-report.json";
const ARTIFACTS: [&str; 4] = [
    "source/final-scene.json",
    "official-placement.json",
    "xml-structure.json",
    "unit-routes.json",
];

fn xml_enforced(p: &Project) -> Result<bool> {
    if p.cfg["XML_STRUCTURE"]
        .get("mode")
        .is_some_and(|value| !value.is_string())
    {
        return Err(err("XML_STRUCTURE.mode must be a string"));
    }
    match p.cfg["XML_STRUCTURE"].get("mode").and_then(Json::as_str) {
        Some("strict") => Ok(true),
        Some("diagnostic") => Ok(false),
        Some(other) => Err(err(format!("unknown XML_STRUCTURE mode {other}"))),
        None if !p.cfg["XML_STRUCTURE"].is_null() && !p.cfg["XML_STRUCTURE"].is_object() => {
            Err(err("XML_STRUCTURE must be an object"))
        }
        None => Ok(true),
    }
}

pub fn binding(p: &Project) -> Result<Json> {
    Ok(json!({"algorithm":REVISION,"catalog":official::binding(),
        "placement":official_placement::binding(p)?,"mobility":mobility::binding(p)?,
        "xml":{"enforced":xml_enforced(p)?,"settings":p.cfg["XML_STRUCTURE"]}}))
}

/// Saved-package edits retain the same explicit settings and strict defaults.
pub(crate) fn incremental_context(p: &Project) -> Result<Project> {
    xml_enforced(p)?;
    official_placement::enforced(p)?;
    Ok(p.clone())
}

fn snapshot(p: &Project, map: &MapFile, native_crc32: u32) -> Result<Json> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let mut unresolved = Vec::new();
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?
        .iter()
        .enumerate()
        .map(|(i, object)| {
            let definition = official::object(&object.type_name);
            if definition.is_none()
                && object.road_type == 0
                && !object.type_name.contains("Waypoint")
            {
                unresolved.push(json!({"object_id":i+1,"kind":object.type_name}));
            }
            json!({"object_id":i+1,"kind":object.type_name,
                "identity":prop(&object.props,"uniqueID").and_then(Value::text),
                "waypoint":prop(&object.props,"waypointName").and_then(Value::text),
                "world_position":object.position,
                "cell":frame.grid([object.position[0] as f64,object.position[1] as f64]),
                "angle_radians":object.angle,"official_source":definition.map(|d|&d.source)})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"schema_version":1,"map_id":p.id,"map":p.name,"frame":frame,
        "native_crc32":native_crc32,"catalog_fingerprint":official::binding().fingerprint,
        "object_id_basis":"final native object order","objects":objects,
        "unresolved_objects":unresolved,
        "scope":"immutable inventory of the final native map; world, grid and local geometry share Frame"}),
    )
}

/// Compute all new checks against the same final MapFile. No game data is changed.
pub fn analyse(p: &Project, map: &MapFile, xml: Option<&str>) -> Result<Json> {
    let overrides = read_overrides(p)?;
    analyse_inner(p, map, xml, overrides.as_deref())
}

fn overrides_evidence(overrides: Option<&str>) -> Result<Json> {
    let Some(xml) = overrides else {
        return Ok(
            json!({"available":false,"objects":[],"locomotors":[],"includes":[],"asset_definitions":[]}),
        );
    };
    let document = xml_structure::asset_document(xml)?;
    let mut objects = Vec::new();
    let mut locomotors = Vec::new();
    let mut includes = Vec::new();
    let definitions = document.elements[document.root]
        .children
        .iter()
        .map(|index| &document.elements[*index])
        .filter(|element| !matches!(element.local_name(), "Tags" | "Includes"))
        .map(|element| json!({"type":element.local_name(),"id":element.attribute("id")}))
        .collect::<Vec<_>>();
    for element in &document.elements {
        match element.local_name() {
            "GameObject" => objects.push(element.attribute("id").unwrap_or("unknown")),
            "LocomotorTemplate" => locomotors.push(element.attribute("id").unwrap_or("unknown")),
            "Include" => includes.push(element.attribute("source").unwrap_or("unknown")),
            _ => {}
        }
    }
    Ok(
        json!({"available":true,"crc32":crc32fast::hash(xml.as_bytes()),
        "objects":objects,"locomotors":locomotors,"includes":includes,"asset_definitions":definitions,
        "scope":"package-specific definitions are bound to the report; SDK geometry is not claimed exact for unresolved overrides"}),
    )
}

fn embedded_assets_evidence(xml: Option<&str>) -> Json {
    let Some(xml) = xml else {
        return json!({"available":false,"asset_definitions":[],"unresolved_includes":[]});
    };
    let Ok(document) = xml_structure::asset_document(xml) else {
        // XML_STRUCTURE owns malformed-map diagnostics and its effective mode.
        return json!({"available":true,"parsed":false,"asset_definitions":[],"unresolved_includes":[]});
    };
    let definitions = document.elements[document.root]
        .children
        .iter()
        .map(|index| &document.elements[*index])
        .filter(|element| {
            !matches!(
                element.local_name(),
                "Tags" | "Includes" | "GameMap" | "TerrainTextureAtlas" | "GameScriptList"
            )
        })
        .map(|element| json!({"type":element.local_name(),"id":element.attribute("id")}))
        .collect::<Vec<_>>();
    let includes = document
        .elements
        .iter()
        .filter(|element| element.local_name() == "Include")
        .map(|element| element.attribute("source").unwrap_or("unknown"))
        .filter(|source| {
            let source = source.to_ascii_lowercase();
            !source.starts_with("data:") && !source.starts_with("art:") && source != "overrides.xml"
        })
        .collect::<Vec<_>>();
    json!({"available":true,"parsed":true,"asset_definitions":definitions,
        "unresolved_includes":includes,
        "scope":"inline assets and non-SDK includes in map.xml; DATA/ART resource includes retain the base SDK source assumption"})
}

fn analyse_inner(
    p: &Project,
    map: &MapFile,
    xml: Option<&str>,
    overrides: Option<&str>,
) -> Result<Json> {
    let mut placement = official_placement::audit(p, map)?;
    let mut routes = mobility::analyse(p, map)?;
    let mut override_report = overrides_evidence(overrides)?;
    let embedded = embedded_assets_evidence(xml);
    let overridden = ["objects", "locomotors", "includes", "asset_definitions"]
        .iter()
        .any(|key| {
            override_report[key]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
        || ["asset_definitions", "unresolved_includes"]
            .iter()
            .any(|key| {
                embedded[key]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
            });
    override_report["map_xml_assets"] = embedded;
    if overridden {
        let pending = json!({"rule":"package_overrides","status":"pending",
            "detail":"Package-specific asset overrides require resolution before SDK geometry can be enforced as exact.",
            "overrides":override_report});
        if placement["status"] != "not_applicable" {
            placement["passed"] = json!(false);
            placement["raw_passed"] = json!(false);
            placement["coverage_complete"] = json!(false);
            placement["findings"]
                .as_array_mut()
                .ok_or_else(|| err("invalid placement report"))?
                .push(pending.clone());
            placement["pending"]
                .as_array_mut()
                .ok_or_else(|| err("invalid placement report"))?
                .push(pending);
        }
        routes["package_overrides"] = json!({"status":"partial","evidence":override_report});
        if routes["status"] != "not_applicable" {
            routes["status"] = json!("partial");
        }
    }
    let mut structure = xml.map(xml_structure::analyse).unwrap_or_else(
        || json!({"passed":null,"status":"unavailable","findings":[],"scope":"map.xml is absent"}),
    );
    structure["enforced"] = json!(xml_enforced(p)?);
    let xml_passed = structure["enforced"] == true && structure["passed"] == true;
    let placement_passed = placement["status"] == "not_applicable"
        || (placement["enforced"] == true
            && placement["passed"] == true
            && placement["raw_passed"] == true);
    Ok(json!({"schema_version":1,"algorithm":REVISION,"map":p.name,
        "catalog_fingerprint":official::binding().fingerprint,
        "effective_bindings":binding(p)?,
        "passed":xml_passed && placement_passed,
        "raw_passed":structure["passed"] == true && (placement["raw_passed"] == true || placement["status"] == "not_applicable"),
        "xml_structure":structure,"official_placement":placement,"unit_routes":routes,"package_overrides":override_report,
        "scope":"automatic structural/placement checks plus diagnostic unit-state routes; independent delivery checks remain separate"}))
}

pub(crate) fn require_passed(report: &Json) -> Result<()> {
    official_placement::require_passed(&report["official_placement"])?;
    if report["xml_structure"]["enforced"] != true {
        return Err(err(
            "diagnostic XML structure cannot authorize production; use strict mode",
        ));
    }
    if report["xml_structure"]["passed"] != true {
        return Err(err("XML structure failed; see xml-structure.json"));
    }
    if report["passed"] != true || report["raw_passed"] != true {
        return Err(err(
            "production requires both passed and raw_passed for the complete final scene",
        ));
    }
    Ok(())
}

pub fn read_xml(p: &Project) -> Result<Option<String>> {
    match fs::read_to_string(p.assets.join("map.xml")) {
        Ok(xml) => Ok(Some(xml)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Inspect explicit diagnostic modes without production authorization or writes.
/// Strict audits also refresh the content-bound production evidence.
pub fn audit_existing(p: &Project) -> Result<Json> {
    let bytes = fs::read(p.assets.join(format!("{}.map", p.name)))?;
    let map = MapFile::load_bytes(&bytes)?;
    let xml = read_xml(p)?;
    if !xml_enforced(p)? || !official_placement::enforced(p)? {
        let mut report = analyse(p, &map, xml.as_deref())?;
        report["native_crc32"] = json!(crc32fast::hash(&map.encode()?));
        report["read_only"] = json!(true);
        return Ok(report);
    }
    verify(p, &map, xml.as_deref())
}

/// Diagnose all gift/resource reservations without changing any game file.
pub fn audit_gifts_existing(p: &Project) -> Result<Json> {
    let bytes = fs::read(p.assets.join(format!("{}.map", p.name)))?;
    let native = MapFile::load_bytes(&bytes)?;
    let saved = crate::terrain::Layout::release_snapshot(p, &p.name)?
        .ok_or_else(|| err("gift audit requires complete same-version source/layout.json"))?;
    let (layout, _) = crate::coordinates::native_layout(p, &native, &saved)?;
    let mut report = crate::gameplay::gift_clearance::audit_reservations(p, &native, &layout)?;
    report["map_id"] = json!(p.id);
    report["map"] = json!(p.name);
    report["native_crc32"] = json!(crc32fast::hash(&bytes));
    report["layout_crc32"] = json!(crc32fast::hash(&serde_json::to_vec(&layout)?));
    report["read_only_native"] = json!(true);
    project::write_json(&p.assets.join("gift-reservations.json"), &report)?;
    Ok(report)
}

fn read_overrides(p: &Project) -> Result<Option<String>> {
    match fs::read_to_string(p.assets.join("overrides.xml")) {
        Ok(xml) => Ok(Some(xml)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn package_xml(files: &[(String, Vec<u8>)]) -> Result<(Option<&str>, Option<&str>)> {
    fn read<'a>(files: &'a [(String, Vec<u8>)], name: &str) -> Result<Option<&'a str>> {
        Ok(files
            .iter()
            .find(|(file, _)| file == name)
            .map(|(_, bytes)| std::str::from_utf8(bytes))
            .transpose()?)
    }
    Ok((read(files, "map.xml")?, read(files, "overrides.xml")?))
}

pub(crate) fn analyse_files(
    p: &Project,
    map: &MapFile,
    files: &[(String, Vec<u8>)],
) -> Result<Json> {
    let (xml, overrides) = package_xml(files)?;
    analyse_inner(p, map, xml, overrides)
}

pub(crate) fn verify_files(
    p: &Project,
    map: &MapFile,
    files: &[(String, Vec<u8>)],
) -> Result<Json> {
    let (xml, overrides) = package_xml(files)?;
    verify_inner(p, map, xml, overrides, None)
}

/// Cache diagnostics only after binding actual native/XML bytes and effective policies.
/// Publication always validates its actual package independently of this cache.
pub fn verify(p: &Project, map: &MapFile, xml: Option<&str>) -> Result<Json> {
    let overrides = read_overrides(p)?;
    verify_inner(p, map, xml, overrides.as_deref(), None)
}

/// Commit a publication's already computed diagnostics after its transaction starts.
pub(crate) fn verify_prepared(
    p: &Project,
    map: &MapFile,
    files: &[(String, Vec<u8>)],
    report: Json,
) -> Result<Json> {
    let (xml, overrides) = package_xml(files)?;
    verify_inner(p, map, xml, overrides, Some(report))
}

fn verify_inner(
    p: &Project,
    map: &MapFile,
    xml: Option<&str>,
    overrides: Option<&str>,
    prepared: Option<Json>,
) -> Result<Json> {
    let native_crc32 = crc32fast::hash(&map.encode()?);
    let policies = binding(p)?;
    let mut key = crc32fast::Hasher::new();
    key.update(REVISION.as_bytes());
    key.update(&native_crc32.to_le_bytes());
    key.update(&serde_json::to_vec(&xml)?);
    key.update(&serde_json::to_vec(&overrides)?);
    key.update(&serde_json::to_vec(&p.cfg)?);
    key.update(&serde_json::to_vec(&policies)?);
    let input_crc32 = key.finalize();
    if let Ok(bytes) = fs::read(p.assets.join(REPORT))
        && let Ok(cached) = serde_json::from_slice::<Json>(&bytes)
        && cached["input_crc32"] == input_crc32
        && cached["algorithm"] == REVISION
        && cached["effective_bindings"] == policies
        && cached["passed"] == true
        && let Some(artifacts) = cached["artifacts_crc32"].as_object()
        && artifacts.len() == ARTIFACTS.len()
        && ARTIFACTS.iter().all(|name| artifacts.contains_key(*name))
        && artifacts.iter().all(|(name, crc)| {
            fs::read(p.assets.join(name)).is_ok_and(|bytes| {
                if json!(crc32fast::hash(&bytes)) != *crc {
                    return false;
                }
                let part = match name.as_str() {
                    "official-placement.json" => "official_placement",
                    "xml-structure.json" => "xml_structure",
                    "unit-routes.json" => "unit_routes",
                    _ => return true,
                };
                serde_json::from_slice::<Json>(&bytes).is_ok_and(|value| value == cached[part])
            })
        })
    {
        require_passed(&cached)?;
        return Ok(cached);
    }
    let mut report = match prepared.filter(|report| report["effective_bindings"] == policies) {
        Some(report) => report,
        None => analyse_inner(p, map, xml, overrides)?,
    };
    report["input_crc32"] = json!(input_crc32);
    report["native_crc32"] = json!(native_crc32);
    report["effective_bindings"] = policies;
    let artifacts = [
        ("source/final-scene.json", snapshot(p, map, native_crc32)?),
        (
            "official-placement.json",
            report["official_placement"].clone(),
        ),
        ("xml-structure.json", report["xml_structure"].clone()),
        ("unit-routes.json", report["unit_routes"].clone()),
    ];
    let mut checksums = BTreeMap::new();
    for (name, value) in artifacts {
        let path = p.assets.join(name);
        project::write_json(&path, &value)?;
        checksums.insert(name, crc32fast::hash(&fs::read(path)?));
    }
    report["artifacts_crc32"] = serde_json::to_value(checksums)?;
    project::write_json(&p.assets.join(REPORT), &report)?;
    require_passed(&report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Project, MapFile);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "ra3-final-scene-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let map =
            MapFile::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/template.map"))
                .unwrap();
        let hm = HeightMap::parse(map.get("HeightMapData").unwrap()).unwrap();
        Fixture(
            Project {
                assets: root.join("assets/test"),
                dist: root.join("dist/test"),
                root,
                id: "test".into(),
                name: "test".into(),
                group: "test".into(),
                version: "T0.0.1".into(),
                state: json!({}),
                cfg: json!({"GRID":hm.width,"GRID_HEIGHT":hm.height,"BORDER":hm.border,"CELL":7.5,"Z_SCALE":0.0390625,
                "TERRAIN_ONLY":true}),
            },
            map,
        )
    }
    fn xml() -> String {
        "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><GameMap id='test'><Waypoint id='start'><Position x='3' y='4' z='0'/></Waypoint><Structure id='house'><Position x='1' y='2' z='0'/></Structure></GameMap></AssetDeclaration>".into()
    }
    #[test]
    fn diagnostic_scene_cannot_authorize_verify_even_when_raw_checks_pass() {
        let mut f = fixture();
        let xml = xml();
        let native = f.1.encode().unwrap();
        f.0.cfg["XML_STRUCTURE"] = json!({"mode":"diagnostic"});
        let before = analyse(&f.0, &f.1, Some(&xml)).unwrap();
        assert_eq!(before["passed"], false);
        assert_eq!(before["raw_passed"], true);
        assert!(require_passed(&before).is_err());
        assert!(
            verify(&f.0, &f.1, Some(&xml))
                .unwrap_err()
                .to_string()
                .contains("diagnostic XML structure")
        );
        let diagnostic: Json =
            serde_json::from_slice(&fs::read(f.0.assets.join(REPORT)).unwrap()).unwrap();
        f.0.cfg["XML_STRUCTURE"]["mode"] = json!("strict");
        let strict = verify(&f.0, &f.1, Some(&xml)).unwrap();
        assert_ne!(strict["input_crc32"], diagnostic["input_crc32"]);
        assert_eq!(strict["passed"], true);
        assert_eq!(f.1.encode().unwrap(), native);
    }
    #[test]
    fn scene_cache_checks_exported_inventory_and_package_overrides() {
        let f = fixture();
        let xml = xml();
        let first = verify(&f.0, &f.1, Some(&xml)).unwrap();
        let inventory = f.0.source().join("final-scene.json");
        fs::write(&inventory, b"corrupt").unwrap();
        let repaired = verify(&f.0, &f.1, Some(&xml)).unwrap();
        assert_eq!(first["input_crc32"], repaired["input_crc32"]);
        let snapshot: Json = serde_json::from_slice(&fs::read(&inventory).unwrap()).unwrap();
        assert!(snapshot["objects"].is_array());
        project::atomic_write(&f.0.assets.join("overrides.xml"),b"<AssetDeclaration xmlns='uri:ea.com:eala:asset'><Tags/><Includes/></AssetDeclaration>").unwrap();
        let changed = verify(&f.0, &f.1, Some(&xml)).unwrap();
        assert_ne!(changed["input_crc32"], first["input_crc32"]);
        assert_eq!(changed["package_overrides"]["available"], true);
        let report_path = f.0.assets.join(REPORT);
        let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(123456);
        fs::OpenOptions::new()
            .write(true)
            .open(&report_path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(time))
            .unwrap();
        verify(&f.0, &f.1, Some(&xml)).unwrap();
        assert_eq!(fs::metadata(report_path).unwrap().modified().unwrap(), time);
    }

    #[test]
    fn partial_native_results_cannot_authorize_a_final_scene() {
        let f = fixture();
        let mut report = analyse(&f.0, &f.1, Some(&xml())).unwrap();
        assert_eq!(report["passed"], true);
        assert_eq!(report["raw_passed"], true);
        require_passed(&report).unwrap();
        report["raw_passed"] = json!(false);
        assert!(require_passed(&report).is_err());
        report["raw_passed"] = json!(true);
        report["official_placement"] = json!({"enforced":true,"passed":true,"raw_passed":false,"strict_scope":"all applicable native buildings and explicit building creation actions","findings":[],"diagnostic_findings":[{"kind":"OreNode","detail":"unresolved instance"}]});
        assert!(require_passed(&report).is_err());
    }

    #[test]
    fn saved_package_context_preserves_explicit_strict_modes() {
        let mut f = fixture();
        f.0.cfg["XML_STRUCTURE"] = json!({});
        f.0.cfg["SPAWN_PLACEMENT_REFERENCE"] = Json::Null;
        f.0.cfg["RAMP_CLEARANCE_REFERENCE"] = Json::Null;
        assert!(xml_enforced(&f.0).unwrap());
        f.0.cfg["COORDINATE_PIPELINE"] = json!({});
        f.0.cfg["TEXTURE_PROFILE"] = json!("legacy");
        f.0.cfg["RAMP_PROFILE"] = json!("legacy");
        f.0.cfg["SPAWN_PLACEMENT_REFERENCE"] = json!("old.json");
        f.0.cfg["RAMP_CLEARANCE_REFERENCE"] = json!("old.json");
        let inherited = incremental_context(&f.0).unwrap();
        assert!(xml_enforced(&inherited).unwrap());
        assert!(official_placement::enforced(&inherited).unwrap());
        assert_eq!(inherited.cfg, f.0.cfg);
        assert_eq!(f.0.cfg["XML_STRUCTURE"], json!({}));
        f.0.cfg["XML_STRUCTURE"]["mode"] = json!("strict");
        f.0.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":"strict"});
        let strict = incremental_context(&f.0).unwrap();
        assert!(xml_enforced(&strict).unwrap());
        assert!(official_placement::enforced(&strict).unwrap());
    }

    #[test]
    fn erased_cached_subreports_are_recomputed_and_nonobject_overrides_are_visible() {
        let f = fixture();
        let xml = xml();
        verify(&f.0, &f.1, Some(&xml)).unwrap();
        let path = f.0.assets.join(REPORT);
        let mut cached: Json = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        cached.as_object_mut().unwrap().remove("xml_structure");
        project::write_json(&path, &cached).unwrap();
        let repaired = verify(&f.0, &f.1, Some(&xml)).unwrap();
        assert_eq!(repaired["xml_structure"]["passed"], true);
        let overrides = "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><AIData id='AIData' DeepWaterDepth='100'/></AssetDeclaration>";
        let evidence = overrides_evidence(Some(overrides)).unwrap();
        assert_eq!(evidence["asset_definitions"][0]["type"], "AIData");
        assert_eq!(evidence["available"], true);
    }

    #[test]
    fn inline_assets_and_custom_includes_are_not_certified_as_base_sdk() {
        let inline = embedded_assets_evidence(Some(
            "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><Includes><Include source='DATA:static.xml'/><Include source='overrides.xml'/><Include source='custom.xml'/></Includes><GameMap id='test'/><GameObject id='AlliedWarFactory'/><LocomotorTemplate id='Tank'/></AssetDeclaration>",
        ));
        assert_eq!(inline["asset_definitions"].as_array().unwrap().len(), 2);
        assert_eq!(inline["unresolved_includes"], json!(["custom.xml"]));
        let mut f = fixture();
        f.0.cfg["TERRAIN_ONLY"] = json!(false);
        f.0.cfg["OFFICIAL_PLACEMENT"] = json!({"mode":"strict"});
        let mut objects = map::objects(f.1.get("ObjectsList").unwrap(), &f.1.table).unwrap();
        objects[0].type_name = "OreNode".into();
        objects[0].road_type = 0;
        f.1.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &f.1).unwrap();
        let xml = "<AssetDeclaration xmlns='uri:ea.com:eala:asset'><GameMap id='test'/><AIData id='AIData'/></AssetDeclaration>";
        let report = analyse_inner(&f.0, &f.1, Some(xml), None).unwrap();
        assert_eq!(report["official_placement"]["enforced"], true);
        assert_eq!(report["official_placement"]["passed"], false);
        assert!(require_passed(&report).is_err());
        assert!(
            report["official_placement"]["pending"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["rule"] == "package_overrides")
        );
    }
}
