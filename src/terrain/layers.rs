//! Keep the selected original blueprint as the map's current root copy.
use crate::{
    Result,
    pipeline::project::{self, Project},
};
use std::{fs, io::ErrorKind, path::PathBuf};

pub(crate) struct SelectedLayers {
    source: PathBuf,
    bytes: Option<Vec<u8>>,
}

impl SelectedLayers {
    /// Read before committing a release so missing selected inputs fail early.
    pub(crate) fn load(p: &Project) -> Result<Self> {
        let source = p.path(p.string("LAYERS", "layers.npz"))?;
        let bytes = match fs::read(&source) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == ErrorKind::NotFound && !p.cfg["LAYERS"].is_string() => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self { source, bytes })
    }

    pub(crate) fn sync_current(&self, p: &Project) -> Result<bool> {
        if !is_current(p)? {
            return Ok(false);
        }
        let Some(bytes) = &self.bytes else {
            return Ok(false);
        };
        let target = p.path("layers.npz")?;
        if self.source == target {
            return Ok(false);
        }
        match fs::read(&target) {
            Ok(current) if current == *bytes => return Ok(false),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        project::atomic_write(&target, bytes)?;
        Ok(true)
    }
}

fn is_current(p: &Project) -> Result<bool> {
    Ok(project::format_version(&project::load_version(&p.root, &p.id)?)? == p.version)
}

/// Copy original NPZ bytes; native map heights never provide this blueprint.
/// Returns whether the root copy changed. A source-only map may omit LAYERS.
pub fn sync_selected(p: &Project) -> Result<bool> {
    if !is_current(p)? {
        return Ok(false);
    }
    SelectedLayers::load(p)?.sync_current(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{archive, npy};
    use serde_json::json;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::{Duration, SystemTime},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Project);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "ra3maps-selected-layers-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        project::write_json(
            &root.join("maps/Test/config.json"),
            &json!({"SCHEMA_VERSION":1,"MAP_NAME":"Test","GRID":8,
                "BORDER":0,"CELL":10.,"Z_SCALE":0.5}),
        )
        .unwrap();
        project::write_json(
            &project::version_path(&root, "Test"),
            &project::parse_version("T0.7.8").unwrap(),
        )
        .unwrap();
        Fixture(Project::load(&root, "Test").unwrap())
    }
    fn selected(f: &mut Fixture) -> Vec<u8> {
        let bytes = archive::zip(&[
            (
                "z_scale.npy".into(),
                npy::encode("<f8", &[], &0.125f64.to_le_bytes()),
            ),
            ("flip_v.npy".into(), npy::encode("|u1", &[], &[1])),
            (
                "original-metadata".into(),
                b"keep unknown input bytes".to_vec(),
            ),
        ])
        .unwrap();
        f.0.cfg["LAYERS"] = json!("histories/v11/layers.npz");
        let source = f.0.path("histories/v11/layers.npz").unwrap();
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(source, &bytes).unwrap();
        bytes
    }
    fn old_mtime(path: &std::path::Path) -> SystemTime {
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(123456);
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(time))
            .unwrap();
        fs::metadata(path).unwrap().modified().unwrap()
    }

    #[test]
    fn selected_npz_is_copied_exactly_and_repeated_sync_keeps_mtime() {
        let mut f = fixture();
        let bytes = selected(&mut f);
        let target = f.0.path("layers.npz").unwrap();
        assert!(sync_selected(&f.0).unwrap());
        assert_eq!(fs::read(&target).unwrap(), bytes);
        let modified = old_mtime(&target);
        assert!(!sync_selected(&f.0).unwrap());
        assert_eq!(fs::metadata(target).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn default_source_is_not_rewritten_and_missing_implicit_input_is_optional() {
        let mut f = fixture();
        assert!(!sync_selected(&f.0).unwrap());
        let bytes = selected(&mut f);
        f.0.cfg.as_object_mut().unwrap().remove("LAYERS");
        let target = f.0.path("layers.npz").unwrap();
        fs::write(&target, &bytes).unwrap();
        let modified = old_mtime(&target);
        assert!(!sync_selected(&f.0).unwrap());
        assert_eq!(fs::read(&target).unwrap(), bytes);
        assert_eq!(fs::metadata(target).unwrap().modified().unwrap(), modified);
    }

    #[test]
    fn missing_explicit_or_unsafe_selection_preserves_the_current_copy() {
        let mut f = fixture();
        let bytes = selected(&mut f);
        sync_selected(&f.0).unwrap();
        for path in ["missing/layers.npz", "../layers.npz", "/outside/layers.npz"] {
            f.0.cfg["LAYERS"] = json!(path);
            assert!(sync_selected(&f.0).is_err(), "{path}");
            assert_eq!(fs::read(f.0.path("layers.npz").unwrap()).unwrap(), bytes);
        }
        fs::remove_file(f.0.path("layers.npz").unwrap()).unwrap();
        f.0.cfg["LAYERS"] = json!("layers.npz");
        assert!(sync_selected(&f.0).is_err());
    }

    #[test]
    fn historical_export_cannot_overwrite_the_current_copy() {
        let mut f = fixture();
        selected(&mut f);
        let target = f.0.path("layers.npz").unwrap();
        fs::write(&target, b"current selected blueprint").unwrap();
        f.0.version = "T0.7.7".into();
        assert!(!sync_selected(&f.0).unwrap());
        assert_eq!(fs::read(&target).unwrap(), b"current selected blueprint");
        f.0.cfg["LAYERS"] = json!("missing/layers.npz");
        assert!(!sync_selected(&f.0).unwrap());
    }

    #[test]
    fn prepared_release_reuses_original_bytes_after_version_commit() {
        let mut f = fixture();
        let bytes = selected(&mut f);
        let source = f.0.path("histories/v11/layers.npz").unwrap();
        let chosen = SelectedLayers::load(&f.0).unwrap();
        f.0.version = "T0.7.9".into();
        assert!(!chosen.sync_current(&f.0).unwrap());
        fs::remove_file(source).unwrap();
        project::write_json(
            &project::version_path(&f.0.root, &f.0.id),
            &project::parse_version("T0.7.9").unwrap(),
        )
        .unwrap();
        assert!(chosen.sync_current(&f.0).unwrap());
        assert_eq!(fs::read(f.0.path("layers.npz").unwrap()).unwrap(), bytes);
    }

    #[test]
    fn failed_bumped_build_preserves_the_root_copy_and_exact_version_bytes() {
        let mut f = fixture();
        selected(&mut f);
        project::write_json(&f.0.map_dir().join("config.json"), &f.0.cfg).unwrap();
        let target = f.0.path("layers.npz").unwrap();
        fs::write(&target, b"previous selected blueprint").unwrap();
        let version = project::version_path(&f.0.root, &f.0.id);
        let before = fs::read(&version).unwrap();
        let result = crate::pipeline::build::run(
            &f.0.root,
            &f.0.id,
            &crate::pipeline::build::Options {
                bump: true,
                only: vec!["verify".into()],
                ..Default::default()
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(&version).unwrap(), before);
        assert_eq!(fs::read(target).unwrap(), b"previous selected blueprint");
    }

    #[test]
    fn missing_explicit_blueprint_stops_build_before_version_or_artifact_changes() {
        let mut f = fixture();
        f.0.cfg["LAYERS"] = json!("histories/missing/layers.npz");
        project::write_json(&f.0.map_dir().join("config.json"), &f.0.cfg).unwrap();
        let target = f.0.path("layers.npz").unwrap();
        fs::write(&target, b"previous selected blueprint").unwrap();
        let version = project::version_path(&f.0.root, &f.0.id);
        let before = fs::read(&version).unwrap();
        let result = crate::pipeline::build::run(
            &f.0.root,
            &f.0.id,
            &crate::pipeline::build::Options {
                bump: true,
                install: true,
                only: vec!["verify".into()],
                ..Default::default()
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(version).unwrap(), before);
        assert_eq!(fs::read(target).unwrap(), b"previous selected blueprint");
        assert!(!f.0.root.join("assets").exists());
        assert!(!f.0.root.join("dist").exists());
        assert!(!f.0.map_dir().join("coordinate-system.json").exists());
    }

    #[test]
    fn installation_failure_keeps_committed_build_version_artifacts_and_blueprint() {
        let mut f = fixture();
        let bytes = selected(&mut f);
        project::write_json(&f.0.map_dir().join("config.json"), &f.0.cfg).unwrap();
        fs::write(f.0.path("layers.npz").unwrap(), b"previous blueprint").unwrap();
        // Skipping all stages leaves no valid game package, so installation
        // fails before it can create or write anything in APPDATA.
        let error = crate::pipeline::build::run(
            &f.0.root,
            &f.0.id,
            &crate::pipeline::build::Options {
                bump: true,
                install: true,
                skip: crate::pipeline::build::STAGES
                    .iter()
                    .map(|stage| (*stage).into())
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("构建已完成，安装失败"));
        let committed = Project::load(&f.0.root, &f.0.id).unwrap();
        assert_eq!(committed.version, "T0.7.9");
        assert!(committed.source().is_dir());
        assert_eq!(
            fs::read(committed.path("layers.npz").unwrap()).unwrap(),
            bytes
        );
    }
}
