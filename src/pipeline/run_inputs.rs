//! A read-only snapshot of a build's selected inputs and executable/specification identity.
//! Generated assets, version.json and an unselected root blueprint mirror are excluded.
use crate::{Result, err, formats::sha256, pipeline::project::Project};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
struct Fingerprint {
    bytes: u64,
    sha256: String,
}
#[derive(Clone, Debug)]
pub struct RunInputs {
    root: PathBuf,
    map_id: String,
    cfg: Value,
    files: BTreeMap<String, Option<Fingerprint>>,
    git_head: Option<String>,
}
impl RunInputs {
    /// Capture actual disk inputs plus the caller's effective configuration. Does not write.
    pub fn capture(p: &Project) -> Result<Self> {
        let root = p.root.canonicalize()?;
        let mut snapshot = Self {
            root,
            map_id: p.id.clone(),
            cfg: p.cfg.clone(),
            files: BTreeMap::new(),
            git_head: None,
        };
        snapshot.files = snapshot.collect()?;
        snapshot.git_head = git_head(&snapshot.root);
        Ok(snapshot)
    }
    /// Verify before each stage. File creation/removal, same-size rewrites and rule/code changes are checked.
    pub fn verify(&self) -> Result<()> {
        let current = self.collect()?;
        let mut paths: BTreeSet<&str> = self.files.keys().map(String::as_str).collect();
        paths.extend(current.keys().map(String::as_str));
        for path in paths {
            if self.files.get(path) != current.get(path) {
                return Err(err(format!(
                    "run input changed during build: {path}; review the affected inputs/rules and start a new run"
                )));
            }
        }
        if git_head(&self.root) != self.git_head {
            return Err(err(
                "run code identity changed during build: Git HEAD; start a new run",
            ));
        }
        Ok(())
    }
    /// Also verify a caller that may have changed its in-memory effective configuration.
    pub fn verify_project(&self, p: &Project) -> Result<()> {
        if p.id != self.map_id || p.root.canonicalize()? != self.root || p.cfg != self.cfg {
            return Err(err(
                "run effective configuration changed during build; start a new run",
            ));
        }
        self.verify()
    }
    /// Only identities are exported; source/rule text and environment variables are never copied.
    pub fn manifest(&self) -> Value {
        json!({"schema_version":1,"kind":"build_run_inputs","map_id":self.map_id,
            "effective_configuration_sha256":sha256::hex(&serde_json::to_vec(&self.cfg).expect("JSON value is serializable")),
            "code_identity":{"package_version":env!("CARGO_PKG_VERSION"),"git_head":self.git_head},
            "files":self.files,
            "scope":"selected map inputs, public rules, active specifications/skills, source code and build manifests",
            "excluded_outputs":["version.json","assets/","dist/","work/","target/","unselected root layers.npz mirror"]})
    }
    fn collect(&self) -> Result<BTreeMap<String, Option<Fingerprint>>> {
        let mut files = BTreeMap::new();
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "build.rs",
            "rust-toolchain",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            "README.md",
        ] {
            self.file(&self.root.join(path), &mut files)?;
        }
        for dir in [
            "src",
            "rules",
            "templates",
            "docs/spec",
            ".agents/skills",
            ".agents/rules",
        ] {
            self.tree(&self.root.join(dir), &mut files)?;
        }
        let map = self.root.join("maps").join(&self.map_id);
        self.file(&map.join("config.json"), &mut files)?;
        for image in [
            "preview.png",
            "preview--line.png",
            "preview--center--line.png",
            "source.png",
        ] {
            self.file(&map.join(image), &mut files)?;
        }
        let frozen = self
            .cfg
            .get("COORDINATE_PIPELINE")
            .is_some_and(Value::is_object);
        let mut references: Vec<(String, bool)> = Vec::new();
        for (key, default) in [("LAYOUT", "layout.json"), ("LAYERS", "layers.npz")] {
            if let Some(path) = self.cfg.get(key).and_then(Value::as_str) {
                references.push((path.to_owned(), false));
            } else if !frozen {
                references.push((default.to_owned(), false));
            }
        }
        for key in [
            "RESOURCE_DISTRIBUTION",
            "NATIVE_CELL_EDITS",
            "NATIVE_CELL_EDIT_SCOPE",
            "DELIVERY_ACCEPTANCE",
        ] {
            if let Some(path) = self.cfg.get(key).and_then(Value::as_str) {
                references.push((path.to_owned(), key == "NATIVE_CELL_EDIT_SCOPE"));
            }
        }
        if let Some(coordinates) = self.cfg.get("COORDINATE_PIPELINE") {
            for key in ["source_package", "layout", "baseline", "policy"] {
                if let Some(path) = coordinates.get(key).and_then(Value::as_str) {
                    references.push((path.to_owned(), false));
                }
            }
        }
        if let Some(lock) = self.cfg.get("TERRAIN_LOCK") {
            for key in ["source", "design_mask"] {
                if let Some(path) = lock.get(key).and_then(Value::as_str) {
                    references.push((path.to_owned(), false));
                }
            }
        }
        let mut seen = BTreeSet::new();
        while let Some((reference, follow_scope_paths)) = references.pop() {
            let reference = reference.split('#').next().unwrap_or("");
            if !seen.insert(reference.to_owned()) {
                continue;
            }
            let relative = Path::new(reference);
            if relative.as_os_str().is_empty()
                || relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return Err(err(format!("invalid run input path: {reference}")));
            }
            let path = map.join(relative);
            self.file(&path, &mut files)?;
            // Explicit source scopes may reference pinned masks or ZIPs in nested path objects.
            if follow_scope_paths && path.is_file() {
                let bytes = fs::read(&path)
                    .map_err(|e| err(format!("cannot read run input {}: {e}", path.display())))?;
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    let mut scope_paths = Vec::new();
                    path_fields(&value, &mut scope_paths);
                    references.extend(scope_paths.into_iter().map(|path| (path, false)));
                }
            }
        }
        Ok(files)
    }
    fn file(&self, path: &Path, files: &mut BTreeMap<String, Option<Fingerprint>>) -> Result<()> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| err("run input leaves project root"))?;
        let name = relative.to_string_lossy().replace('\\', "/");
        match fs::metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                files.insert(name, None);
            }
            Err(e) => return Err(err(format!("cannot inspect run input {name}: {e}"))),
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(err(format!("run input is not a file: {name}")));
                }
                if !path.canonicalize()?.starts_with(&self.root) {
                    return Err(err(format!("run input escapes project root: {name}")));
                }
                let bytes = fs::read(path)
                    .map_err(|e| err(format!("cannot read run input {name}: {e}")))?;
                files.insert(
                    name,
                    Some(Fingerprint {
                        bytes: bytes.len() as u64,
                        sha256: sha256::hex(&bytes),
                    }),
                );
            }
        }
        Ok(())
    }
    fn tree(&self, path: &Path, files: &mut BTreeMap<String, Option<Fingerprint>>) -> Result<()> {
        match fs::read_dir(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(err(format!(
                    "cannot inspect run input directory {}: {e}",
                    path.display()
                )));
            }
            Ok(entries) => {
                if !path.canonicalize()?.starts_with(&self.root) {
                    return Err(err(format!(
                        "run input directory escapes project root: {}",
                        path.display()
                    )));
                }
                let mut entries = entries.collect::<std::io::Result<Vec<_>>>()?;
                entries.sort_by_key(|entry| entry.file_name());
                for entry in entries {
                    if entry.file_type()?.is_dir() {
                        self.tree(&entry.path(), files)?;
                    } else {
                        self.file(&entry.path(), files)?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn path_fields(value: &Value, paths: &mut Vec<String>) {
    match value {
        Value::Object(values) => {
            for (key, value) in values {
                if key == "path" {
                    if let Some(path) = value.as_str() {
                        paths.push(path.to_owned());
                    }
                } else {
                    path_fields(value, paths);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                path_fields(value, paths);
            }
        }
        _ => (),
    }
}
fn git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Project);
    impl Fixture {
        fn new(frozen: bool) -> Self {
            let root = std::env::temp_dir().join(format!(
                "ra3-run-inputs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let map = root.join("maps/synthetic");
            fs::create_dir_all(&map).unwrap();
            let mut cfg = json!({"LAYERS":"chosen.npz","RESOURCE_DISTRIBUTION":"resources.json"});
            if frozen {
                cfg["COORDINATE_PIPELINE"] = json!({"source_package":"authoring/source.zip","layout":"authoring/layout.json","baseline":"authoring/baseline.json"});
            }
            for (path, bytes) in [
                ("config.json", serde_json::to_vec(&cfg).unwrap()),
                ("chosen.npz", vec![1, 2, 3]),
                ("resources.json", b"{}".to_vec()),
            ] {
                fs::write(map.join(path), bytes).unwrap();
            }
            let p = Project {
                root: root.clone(),
                id: "synthetic".into(),
                cfg,
                state: json!({}),
                version: "T0.0.1".into(),
                group: "Test".into(),
                name: "TestT0.0.1".into(),
                assets: root.join("assets"),
                dist: root.join("dist"),
            };
            Self(p)
        }
        fn write(&self, path: &str, bytes: &[u8]) {
            let path = self.0.root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0.root).unwrap();
        }
    }
    #[test]
    fn detects_same_size_input_edits_and_reports_exact_path() {
        let f = Fixture::new(false);
        let snapshot = RunInputs::capture(&f.0).unwrap();
        snapshot.verify().unwrap();
        f.write("maps/synthetic/chosen.npz", &[3, 2, 1]);
        let error = snapshot.verify().unwrap_err().to_string();
        assert!(error.contains("maps/synthetic/chosen.npz"), "{error}");
    }
    #[test]
    fn detects_rule_spec_skill_and_code_changes_without_copying_their_contents() {
        for path in [
            "rules/shared.json",
            "docs/spec/scope.md",
            ".agents/skills/terrain/SKILL.md",
            "src/main.rs",
        ] {
            let f = Fixture::new(false);
            f.write(path, b"before");
            let snapshot = RunInputs::capture(&f.0).unwrap();
            let manifest = snapshot.manifest().to_string();
            assert!(!manifest.contains("before"));
            f.write(path, b"after!");
            assert!(snapshot.verify().unwrap_err().to_string().contains(path));
        }
    }
    #[test]
    fn catches_created_removed_and_nested_scope_inputs() {
        let mut f = Fixture::new(false);
        f.0.cfg["NATIVE_CELL_EDIT_SCOPE"] = json!("scope.json");
        f.write(
            "maps/synthetic/scope.json",
            br#"{"source":{"path":"authoring/source.zip"},"allowed_cells":{"path":"mask.npy"}}"#,
        );
        let snapshot = RunInputs::capture(&f.0).unwrap();
        f.write("maps/synthetic/mask.npy", b"mask");
        assert!(
            snapshot
                .verify()
                .unwrap_err()
                .to_string()
                .contains("mask.npy")
        );
        let snapshot = RunInputs::capture(&f.0).unwrap();
        fs::remove_file(f.0.root.join("maps/synthetic/mask.npy")).unwrap();
        assert!(
            snapshot
                .verify()
                .unwrap_err()
                .to_string()
                .contains("mask.npy")
        );
    }
    #[test]
    fn ignores_legitimate_generated_outputs_version_and_unselected_mirror() {
        let mut f = Fixture::new(true);
        let snapshot = RunInputs::capture(&f.0).unwrap();
        for path in [
            "maps/synthetic/version.json",
            "maps/synthetic/layers.npz",
            "assets/new/source/layout.json",
            "dist/new/map.xml",
            "work/task.log",
        ] {
            f.write(path, b"generated");
        }
        f.0.version = "T0.0.2".into();
        f.0.name = "TestT0.0.2".into();
        f.0.state = json!({"stage":"T","major":0,"minor":0,"patch":2});
        snapshot.verify_project(&f.0).unwrap();
    }
    #[test]
    fn changed_effective_configuration_and_new_rules_require_a_new_run() {
        let mut f = Fixture::new(false);
        let snapshot = RunInputs::capture(&f.0).unwrap();
        f.0.cfg["WATER_Z"] = json!(240);
        assert!(snapshot.verify_project(&f.0).is_err());
        f.write("rules/new-policy.json", b"{}");
        assert!(
            snapshot
                .verify()
                .unwrap_err()
                .to_string()
                .contains("rules/new-policy.json")
        );
    }
}
