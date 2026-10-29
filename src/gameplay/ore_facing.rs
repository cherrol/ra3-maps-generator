//! Read back every ore heading; geometry checks do not prove legal model rotation.
use crate::{
    Result, err,
    formats::map::{self, MapFile, Value, prop},
    gameplay::scripts::{self, Node},
};
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

pub use super::production_facing::ALLOWED_DEGREES;
pub const TOLERANCE_RADIANS: f64 = 1e-6;

fn heading(angle: f32) -> Json {
    let radians = f64::from(angle);
    let finite = radians.is_finite();
    let normalized = finite.then(|| radians.sin().atan2(radians.cos()));
    let error = normalized.map(|a| {
        ALLOWED_DEGREES
            .iter()
            .map(|d| {
                let delta = a - f64::from(*d).to_radians();
                delta.sin().atan2(delta.cos()).abs()
            })
            .fold(f64::INFINITY, f64::min)
    });
    json!({
        "native_radians":finite.then_some(radians),
        "degrees":finite.then(|| radians.to_degrees()),
        "normalized_degrees":normalized.map(f64::to_degrees),
        "nearest_allowed_error_radians":error,
        "passed":error.is_some_and(|e| e <= TOLERANCE_RADIANS)
    })
}

fn actions<'a>(nodes: &'a [Node], path: &str, out: &mut Vec<(String, &'a Node)>) {
    for (i, node) in nodes.iter().enumerate() {
        let path = format!("{path}/{i}");
        if let Some(children) = &node.children {
            actions(children, &path, out);
        } else if matches!(node.asset.as_str(), "ScriptAction" | "ScriptActionFalse") {
            out.push((path, node));
        }
    }
}

/// Includes all native ores and explicit ore creation/orientation actions in all branches.
/// Named turns are resolved against native objects and named script-created ores.
/// Unspecified or dynamic ore headings fail closed instead of being inferred from waypoints.
pub fn report(native: &MapFile) -> Result<Json> {
    let mut objects = Vec::new();
    let mut names = BTreeSet::new();
    for (i, object) in map::objects(native.get("ObjectsList")?, &native.table)?
        .iter()
        .enumerate()
    {
        if !object.type_name.eq_ignore_ascii_case("OreNode") {
            continue;
        }
        for key in ["uniqueID", "objectName"] {
            if let Some(name) = prop(&object.props, key).and_then(Value::text)
                && !name.is_empty()
            {
                names.insert(name.to_owned());
            }
        }
        let mut row = heading(object.angle);
        row["object_id"] = json!(i + 1);
        row["type"] = json!(object.type_name);
        row["unique_id"] = json!(prop(&object.props, "uniqueID").and_then(Value::text));
        row["world"] = json!(object.position);
        objects.push(row);
    }
    let nodes = scripts::parse_nodes(&native.get("PlayerScriptsList")?.payload, &native.table)?;
    let mut leaves = Vec::new();
    actions(&nodes, "", &mut leaves);
    // Resolve names before checking turns, independently of script/branch ordering.
    for (_, node) in &leaves {
        let command = node.command.as_deref().unwrap_or("");
        if !(command.contains("CREATE") || command.contains("SPAWN")) {
            continue;
        }
        let args = scripts::scalar_arguments(node)?;
        if args
            .iter()
            .any(|a| a.kind == 15 && a.text.eq_ignore_ascii_case("OreNode"))
        {
            for arg in args.iter().filter(|a| a.kind == 14 && !a.text.is_empty()) {
                names.insert(arg.text.clone());
            }
        }
    }
    let mut script_actions = Vec::new();
    for (path, node) in leaves {
        let command = node.command.as_deref().unwrap_or("");
        let args = scripts::scalar_arguments(node)?;
        let typed_ore = args
            .iter()
            .any(|a| a.kind == 15 && a.text.eq_ignore_ascii_case("OreNode"));
        let named_ore = args.iter().any(|a| a.kind == 14 && names.contains(&a.text));
        let creates = command.contains("CREATE") || command.contains("SPAWN");
        let turns = ["ORIENT", "FACING", "ROTATE", "ANGLE", "FACE"]
            .iter()
            .any(|part| command.contains(part));
        if !(typed_ore && (creates || turns) || named_ore && turns) {
            continue;
        }
        let angles: Vec<_> = args.iter().filter(|a| a.kind == 17).collect();
        // ROTATE may supply a delta rather than an absolute heading; an allowed
        // delta is not evidence that the resulting ore orientation is legal.
        let pending = command.contains("ROTATE")
            || angles.len() != 1
            || angles.iter().any(|a| !a.text.is_empty());
        let headings: Vec<_> = angles.iter().map(|a| heading(a.real)).collect();
        let passed = !pending && headings.iter().all(|h| h["passed"] == true);
        script_actions.push(json!({
            "path":path,"command":command,"asset":node.asset,
            "ore_names":args.iter().filter(|a| a.kind == 14).map(|a| &a.text).collect::<Vec<_>>(),
            "headings":headings,"passed":passed,
            "status":if pending {"pending"} else if passed {"passed"} else {"failed"},
            "reason":if pending {Some("OreNode action requires an explicit constant absolute ANGLE; implicit, vector, dynamic or relative headings need independent verification")} else {None}
        }));
    }
    let failed_objects = objects.iter().filter(|r| r["passed"] != true).count();
    let failed_actions = script_actions
        .iter()
        .filter(|r| r["status"] == "failed")
        .count();
    let pending_actions = script_actions
        .iter()
        .filter(|r| r["status"] == "pending")
        .count();
    Ok(json!({
        "scope":"OreNode objects and explicit typed/named ore script creation and orientation; other buildings and unresolved indirect script targets require the complete building-facing acceptance",
        "native_crc32":crc32fast::hash(&native.encode()?),
        "allowed_degrees":ALLOWED_DEGREES,"tolerance_radians":TOLERANCE_RADIANS,
        "checked_objects":objects.len(),"checked_actions":script_actions.len(),
        "failed_objects":failed_objects,"failed_actions":failed_actions,"pending_actions":pending_actions,
        "passed":failed_objects == 0 && failed_actions == 0 && pending_actions == 0,
        "objects":objects,"script_actions":script_actions
    }))
}

pub fn validate(native: &MapFile) -> Result<()> {
    require_passed(&report(native)?)
}

fn require_passed(report: &Json) -> Result<()> {
    if report["passed"] != true {
        let ids: Vec<_> = report["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["passed"] != true)
            .map(|r| r["object_id"].as_u64().unwrap())
            .collect();
        return Err(err(format!(
            "OreNode facing failed: {} illegal objects (IDs {ids:?}), {} illegal script actions, {} pending script actions; required angles 45/135/-45/-135 degrees",
            report["failed_objects"], report["failed_actions"], report["pending_actions"]
        )));
    }
    Ok(())
}

/// Keep failed native evidence too; a later check cannot reuse a stale success.
pub fn verify(p: &crate::pipeline::project::Project, native: &MapFile) -> Result<()> {
    let result = report(native)?;
    crate::pipeline::project::write_json(&p.assets.join("ore-facing.json"), &result)?;
    require_passed(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_native_values_without_snapping_illegal_angles() {
        for degrees in [45_f32, 135., -45., -135., 225., 315., 405., -495.] {
            assert_eq!(heading(degrees.to_radians())["passed"], true, "{degrees}");
        }
        for degrees in [0_f32, 90., 180., 270., 11.25, -78.75, 45.001] {
            assert_eq!(heading(degrees.to_radians())["passed"], false, "{degrees}");
        }
        for radians in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(heading(radians)["passed"], false);
        }
        assert_eq!(heading(45_f32.to_radians() + 0.5e-6)["passed"], true);
        assert_eq!(heading(45_f32.to_radians() + 2e-6)["passed"], false);
    }
}
