//! Collect measured reports without manufacturing independent review decisions.
use crate::{
    Result,
    formats::{archive, sha256},
    pipeline::{
        acceptance,
        project::{self, Project},
    },
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

const REPORTS: &[&str] = &[
    "preflight-native.json",
    "layout-preflight.json",
    "scene-report.json",
    "official-placement.json",
    "spawn-placement.json",
    "ramp-clearance.json",
    "resource-distribution.json",
    "production-spawns.json",
    "production-facing.json",
    "ore-facing.json",
    "gift-clearance.json",
    "unit-routes.json",
    "xml-structure.json",
    "texture-report.json",
    "spatial/report.json",
];

pub fn collect(p: &Project) -> Result<Value> {
    let native = fs::read(p.assets.join(format!("{}.map", p.name)))?;
    let map = crate::formats::map::MapFile::load_bytes(&native)?;
    let crc = crc32fast::hash(&native);
    let decoded_crc = crc32fast::hash(&map.encode()?);
    let mut files = BTreeMap::new();
    let mut reports = Vec::new();
    let mut absent = Vec::new();
    for name in REPORTS {
        let path = p.assets.join(name);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                absent.push(*name);
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        let report: Value = serde_json::from_slice(&bytes)?;
        let fingerprints = [
            report.get("native_crc32"),
            report.get("native_decoded_crc32"),
            report.pointer("/binding/native_crc32"),
            report.pointer("/binding/native_decoded_crc32"),
        ];
        let values = fingerprints
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .collect::<Vec<_>>();
        let binding = if values.is_empty() {
            "unbound"
        } else if values
            .iter()
            .all(|v| *v == u64::from(crc) || *v == u64::from(decoded_crc))
        {
            "matches_candidate"
        } else {
            "different_candidate"
        };
        let member = format!("reports/{name}");
        reports.push(json!({"path":member,"source_path":path.strip_prefix(&p.root).unwrap_or(&path),"bytes":bytes.len(),"sha256":sha256::hex(&bytes),"crc32":crc32fast::hash(&bytes),"candidate_binding":binding,"reported_passed":report.get("passed")}));
        files.insert(member, bytes);
    }
    let review_path = acceptance::report_path(p)?;
    let independent = match fs::read(&review_path) {
        Ok(bytes) => {
            let record: Value = serde_json::from_slice(&bytes)?;
            files.insert("review/delivery-acceptance.json".into(), bytes);
            json!({"stored_status":record["status"],"revalidated":false,"path":"review/delivery-acceptance.json"})
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json!({"stored_status":"missing","revalidated":false})
        }
        Err(e) => return Err(e.into()),
    };
    let manifest = json!({"schema_version":1,"map_id":p.id,"map":p.name,"version":p.version,
        "native_crc32":crc,"native_decoded_crc32":decoded_crc,"native_sha256":sha256::hex(&native),
        "reports":reports,"missing_reports":absent,"independent_review":independent,
        "scope":"evidence inventory only; unbound/stale reports require review; no independent decision is generated or promoted to passed"});
    files.insert(
        "evidence-index.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    let entries = files.into_iter().collect::<Vec<_>>();
    let dir = p.map_dir().join("histories").join(&p.version);
    write_archive(&dir.join("evidence.zip"), &archive::zip(&entries)?)?;
    project::write_json(&dir.join("evidence-index.json"), &manifest)?;
    Ok(manifest)
}

fn write_archive(path: &Path, bytes: &[u8]) -> Result<()> {
    project::atomic_write(path, bytes)
}
