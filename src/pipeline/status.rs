//! Project inventory derived from recipes, never from hand-maintained counts.
use crate::{Result, pipeline::project::Project};
use serde_json::{Value, json};
use std::fs;

pub fn inventory(root: &std::path::Path) -> Result<Value> {
    let mut rows = Vec::new();
    for entry in fs::read_dir(root.join("maps"))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let preview = entry.path().join("preview.png").is_file();
        let recipe = entry.path().join("config.json").is_file();
        if !recipe && !preview {
            continue;
        }
        let mut row = json!({"id":id,"preview_available":preview,"recipe_available":recipe});
        if recipe {
            match Project::load(root, &id) {
                Ok(p) => {
                    row["map"] = json!(p.name);
                    row["version"] = json!(p.version);
                    let frozen = &p.cfg["COORDINATE_PIPELINE"];
                    let availability = ["source_package", "layout", "baseline"]
                        .into_iter()
                        .map(|key| {
                            (
                                key,
                                frozen[key]
                                    .as_str()
                                    .is_some_and(|path| p.path(path).is_ok_and(|v| v.is_file())),
                            )
                        })
                        .collect::<std::collections::BTreeMap<_, _>>();
                    row["frozen_inputs_available"] = json!(availability.values().all(|v| *v));
                    row["frozen_input_files"] = json!(availability);
                    let review = crate::pipeline::acceptance::report_path(&p)?;
                    if let Ok(bytes) = fs::read(&review)
                        && let Ok(record) = serde_json::from_slice::<Value>(&bytes)
                    {
                        row["acceptance_record"] = json!({"status":record["status"],"identity_matches":record["map_id"]==p.id&&record["map"]==p.name&&record["version"]==p.version,"contents_revalidated":false});
                    }
                }
                Err(error) => row["recipe_error"] = json!(error.to_string()),
            }
        }
        rows.push(row);
    }
    rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let default = fs::read_to_string(root.join("maps/.default"))
        .ok()
        .map(|v| v.trim().to_owned());
    Ok(json!({"schema_version":1,"default_map":default,"maps":rows,
        "scope":"recipe and file availability only; stored acceptance status is not a fresh delivery verdict"}))
}

/// Reject missing active Markdown targets without treating historical links as
/// live production dependencies. Called by the explicit documentation checker.
pub fn check_links(root: &std::path::Path) -> Result<Value> {
    let mut paths = vec![
        root.join("README.md"),
        root.join("docs/README.md"),
        root.join("docs/SESSION_STATE.md"),
        root.join("docs/TODO.md"),
        root.join("reference/README.md"),
    ];
    fn collect(dir: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for item in fs::read_dir(dir)? {
            let item = item?;
            if item.file_type()?.is_dir() {
                collect(&item.path(), paths)?;
            } else if item.path().extension().is_some_and(|v| v == "md") {
                paths.push(item.path());
            }
        }
        Ok(())
    }
    collect(&root.join("docs/spec"), &mut paths)?;
    collect(&root.join(".agents/skills"), &mut paths)?;
    let mut missing = Vec::new();
    for path in &paths {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        for part in contents.split("](").skip(1) {
            let Some((target, _)) = part.split_once(')') else {
                continue;
            };
            let target = target
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .split('#')
                .next()
                .unwrap_or("");
            if target.is_empty()
                || target.contains("://")
                || target.contains("<id>")
                || target.starts_with("mailto:")
            {
                continue;
            }
            if !path.parent().unwrap().join(target).exists() {
                missing.push(
                    json!({"document":path.strip_prefix(root).unwrap_or(path),"target":target}),
                );
            }
        }
    }
    Ok(
        json!({"schema_version":1,"checked_documents":paths.len(),"missing":missing,"passed":missing.is_empty(),"scope":"literal local targets in active entrypoints/spec/skills; not a full Markdown parser"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::project;
    #[test]
    fn inventory_reads_current_version_and_keeps_preview_only_entries() {
        let root = std::env::temp_dir().join(format!("ra3-inventory-{}", std::process::id()));
        project::write_json(&root.join("maps/Example/config.json"),&json!({"SCHEMA_VERSION":1,"MAP_NAME":"Example","GRID":8,"BORDER":0,"CELL":10.,"Z_SCALE":1.})).unwrap();
        project::write_json(
            &root.join("maps/Example/version.json"),
            &project::parse_version("T0.4.2").unwrap(),
        )
        .unwrap();
        fs::create_dir_all(root.join("maps/Preview")).unwrap();
        fs::write(root.join("maps/Preview/preview.png"), b"fixture").unwrap();
        let report = inventory(&root).unwrap();
        assert_eq!(report["maps"][0]["version"], "T0.4.2");
        assert_eq!(report["maps"][1]["recipe_available"], false);
        fs::remove_dir_all(root).unwrap();
    }
}
