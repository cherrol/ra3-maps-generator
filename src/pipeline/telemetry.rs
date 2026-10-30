//! Durable attempt/stage timings, including errors and unwinding/interruption.
//! These are process timings; compiler and human preparation time are separate.
use crate::{
    Result,
    pipeline::project::{self, Project},
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub fn attempt<T>(
    p: &Project,
    command: &str,
    work: impl FnOnce(&mut RunLog) -> Result<T>,
) -> Result<T> {
    let mut log = RunLog::start(p, command)?;
    let result = work(&mut log);
    let saved = log.finish(&result);
    match result {
        Ok(value) => {
            saved?;
            Ok(value)
        }
        Err(error) => {
            if let Err(log_error) = saved {
                eprintln!("计时归档失败：{log_error}");
            }
            Err(error)
        }
    }
}

pub struct RunLog {
    path: PathBuf,
    started: Instant,
    report: Value,
    finished: bool,
}

impl RunLog {
    pub fn start(p: &Project, command: &str) -> Result<Self> {
        let now = chrono::Utc::now();
        let id = format!(
            "{}-{}-{}",
            now.format("%Y%m%dT%H%M%S%.3fZ"),
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = p
            .root
            .join("work/runs")
            .join(&p.id)
            .join(format!("{id}.json"));
        let log = Self {
            path,
            started: Instant::now(),
            report: json!({"schema_version":1,"run_id":id,"map_id":p.id,
                "map":p.name,"command":command,"started_at":now.to_rfc3339(),
                "task_id":std::env::var("RA3MAP_TASK_ID").ok().filter(|v|!v.trim().is_empty()).unwrap_or_else(||id.clone()),
                "status":"running","stages":[],"events":[],
                "timing_scope":"this process attempt; excludes cargo compilation and human preparation; overlapping attempts must not be summed as wall time"}),
            finished: false,
        };
        log.persist()?;
        Ok(log)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn inputs(&mut self, manifest: Value) -> Result<()> {
        self.report["inputs"] = manifest;
        self.persist()
    }

    pub fn event(&mut self, kind: &str, details: Value) -> Result<()> {
        self.report["events"]
            .as_array_mut()
            .expect("events array")
            .push(json!({"kind":kind,"at":chrono::Utc::now().to_rfc3339(),"details":details}));
        self.persist()
    }

    /// Persist the start before running work, and always record its result.
    pub fn stage<T>(&mut self, name: &str, work: impl FnOnce() -> Result<T>) -> Result<T> {
        let index = self.report["stages"]
            .as_array()
            .expect("stages array")
            .len();
        self.report["stages"].as_array_mut().expect("stages array")
            .push(json!({"name":name,"status":"running","started_at":chrono::Utc::now().to_rfc3339()}));
        self.persist()?;
        eprintln!("[步骤] {name} 开始");
        let start = Instant::now();
        let result = work();
        let elapsed = start.elapsed().as_secs_f64();
        let stage = &mut self.report["stages"][index];
        stage["elapsed_seconds"] = json!(elapsed);
        stage["finished_at"] = json!(chrono::Utc::now().to_rfc3339());
        stage["status"] = json!(if result.is_ok() { "passed" } else { "failed" });
        if let Err(error) = &result {
            stage["error"] = json!(error.to_string());
        }
        eprintln!(
            "[计时] {name} {elapsed:.3}s {}",
            if result.is_ok() { "通过" } else { "失败" }
        );
        let saved = self.persist();
        match result {
            Ok(value) => {
                saved?;
                Ok(value)
            }
            Err(error) => {
                if let Err(log_error) = saved {
                    eprintln!("记录计时失败：{log_error}");
                }
                Err(error)
            }
        }
    }

    pub fn finish<T>(&mut self, result: &Result<T>) -> Result<()> {
        self.report["status"] = json!(if result.is_ok() { "passed" } else { "failed" });
        self.report["elapsed_seconds"] = json!(self.started.elapsed().as_secs_f64());
        self.report["finished_at"] = json!(chrono::Utc::now().to_rfc3339());
        if let Err(error) = result {
            self.report["error"] = json!(error.to_string());
        }
        self.persist()?;
        self.finished = true;
        eprintln!(
            "[计时] 本次尝试 {:.3}s；记录 {}",
            self.started.elapsed().as_secs_f64(),
            self.path.display()
        );
        Ok(())
    }

    fn persist(&self) -> Result<()> {
        project::write_json(&self.path, &self.report)
    }
}

impl Drop for RunLog {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        self.report["status"] = json!("interrupted");
        self.report["elapsed_seconds"] = json!(self.started.elapsed().as_secs_f64());
        self.report["finished_at"] = json!(chrono::Utc::now().to_rfc3339());
        if let Some(stages) = self.report["stages"].as_array_mut() {
            for stage in stages {
                if stage["status"] == "running" {
                    stage["status"] = json!("interrupted");
                }
            }
        }
        let _ = self.persist();
    }
}

/// Summarize all recorded attempts of a task, including failures and overlap.
/// Gaps outside measured processes remain explicitly unmeasured.
pub fn summary(root: &Path, map_id: &str) -> Result<Value> {
    let dir = root.join("work/runs").join(map_id);
    let mut records = std::collections::BTreeMap::<String, Vec<Value>>::new();
    if dir.is_dir() {
        for file in std::fs::read_dir(dir)? {
            let file = file?;
            if file.path().extension().is_none_or(|v| v != "json") {
                continue;
            }
            let record: Value = serde_json::from_slice(&std::fs::read(file.path())?)?;
            if record["map_id"] != map_id {
                return Err(crate::err("timing record map identity mismatch"));
            }
            let task = record["task_id"]
                .as_str()
                .unwrap_or("unassigned")
                .to_owned();
            records.entry(task).or_default().push(record);
        }
    }
    let groups = records.into_iter().map(|(task, runs)| {
        let mut intervals = Vec::new();
        let mut stages = std::collections::BTreeMap::<String, Value>::new();
        for run in &runs {
            if let (Some(start), Some(end)) = (run["started_at"].as_str(), run["finished_at"].as_str())
                && let (Ok(start), Ok(end)) = (chrono::DateTime::parse_from_rfc3339(start), chrono::DateTime::parse_from_rfc3339(end))
                && end >= start {
                intervals.push((start.timestamp_millis(), end.timestamp_millis()));
            }
            for stage in run["stages"].as_array().into_iter().flatten() {
                let row = stages.entry(stage["name"].as_str().unwrap_or("unknown").to_owned())
                    .or_insert_with(|| json!({"attempts":0,"failed":0,"measured_seconds":0.}));
                row["attempts"] = json!(row["attempts"].as_u64().unwrap_or(0) + 1);
                row["failed"] = json!(row["failed"].as_u64().unwrap_or(0) + u64::from(stage["status"] == "failed"));
                row["measured_seconds"] = json!(row["measured_seconds"].as_f64().unwrap_or(0.) + stage["elapsed_seconds"].as_f64().unwrap_or(0.));
            }
        }
        intervals.sort_unstable();
        let span = intervals.first().zip(intervals.iter().map(|v| v.1).max())
            .map(|(first, end)| (end - first.0) as f64 / 1000.);
        let timed = intervals.len();
        let mut union = 0i64;
        let mut current: Option<(i64,i64)> = None;
        for (start, end) in intervals {
            match current {
                Some((lo, hi)) if start <= hi => current = Some((lo, hi.max(end))),
                Some((lo, hi)) => { union += hi-lo; current = Some((start,end)); }
                None => current = Some((start,end)),
            }
        }
        if let Some((lo,hi)) = current { union += hi-lo; }
        let union = union as f64 / 1000.;
        json!({"task_id":task,"attempts":runs.len(),"failed_attempts":runs.iter().filter(|r|r["status"]=="failed").count(),
            "attempts_without_closed_interval":runs.len()-timed,"recorded_span_seconds":span,
            "process_interval_union_seconds":union,"unmeasured_gap_seconds":span.map(|v|(v-union).max(0.)),
            "summed_process_seconds":runs.iter().filter_map(|r|r["elapsed_seconds"].as_f64()).sum::<f64>(),
            "stages":stages,"run_ids":runs.iter().map(|r|r["run_id"].clone()).collect::<Vec<_>>()})
    }).collect::<Vec<_>>();
    Ok(json!({"schema_version":1,"map_id":map_id,"tasks":groups,
        "scope":"Recorded CLI attempts only. Stage sums include retries; overlapping process intervals are merged. Unmeasured gaps, compilation and authoring are not classified as a measured task stage."}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_summary_keeps_failures_and_merges_overlapping_attempts() {
        let root = std::env::temp_dir().join(format!(
            "ra3-task-time-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for (id, start, end, status) in [
            (
                "a",
                "2026-10-30T00:00:00Z",
                Some("2026-10-30T00:00:10Z"),
                "failed",
            ),
            (
                "b",
                "2026-10-30T00:00:05Z",
                Some("2026-10-30T00:00:15Z"),
                "passed",
            ),
            ("c", "2026-10-30T00:00:20Z", None, "running"),
        ] {
            project::write_json(&root.join(format!("work/runs/test/{id}.json")), &json!({"run_id":id,"task_id":"one-task","map_id":"test","started_at":start,"finished_at":end,"status":status,"elapsed_seconds":end.map(|_|10.),"stages":if end.is_some(){json!([{"name":"map","status":status,"elapsed_seconds":10.}])}else{json!([])}})).unwrap();
        }
        let report = summary(&root, "test").unwrap();
        let task = &report["tasks"][0];
        assert_eq!(task["failed_attempts"], 1);
        assert_eq!(task["process_interval_union_seconds"], 15.);
        assert_eq!(task["recorded_span_seconds"], 15.);
        assert_eq!(task["summed_process_seconds"], 20.);
        assert_eq!(task["attempts_without_closed_interval"], 1);
        assert_eq!(task["stages"]["map"]["failed"], 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failure_and_previous_stages_are_durable() {
        let root = std::env::temp_dir().join(format!(
            "ra3-timing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let p = Project {
            root: root.clone(),
            id: "test".into(),
            cfg: json!({}),
            state: json!({}),
            version: "T0.0.1".into(),
            group: "test".into(),
            name: "test".into(),
            assets: root.join("assets"),
            dist: root.join("dist"),
        };
        let mut log = RunLog::start(&p, "build").unwrap();
        log.stage("terrain", || Ok(())).unwrap();
        let failed: Result<()> = log.stage("map", || Err(crate::err("synthetic overlap")));
        log.finish(&failed).unwrap();
        let record: Value = serde_json::from_slice(&std::fs::read(log.path()).unwrap()).unwrap();
        assert_eq!(record["status"], "failed");
        assert_eq!(record["stages"][0]["status"], "passed");
        assert_eq!(record["stages"][1]["error"], "synthetic overlap");
        assert!(record["stages"][1]["elapsed_seconds"].is_number());
        drop(log);
        std::fs::remove_dir_all(root).unwrap();
    }
}
