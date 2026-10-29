//! Offline restoration of the versioned inputs used by the full regression suite.
use ra3maps::{Result, err, formats::archive, pipeline::project, pipeline::project::Project};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "tests/fixtures/historical-inputs.json";
const ARCHIVE: &str = "tests/fixtures/historical-inputs.zip";

#[derive(Deserialize, Serialize)]
struct Manifest {
    schema_version: u32,
    maps: Vec<String>,
    archive_bytes: u64,
    archive_crc32: u32,
    files: Vec<Entry>,
}

#[derive(Deserialize, Serialize)]
struct Entry {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    bytes: u64,
    crc32: u32,
    purpose: String,
}

impl Entry {
    fn target(&self) -> &str {
        self.target.as_deref().unwrap_or(&self.path)
    }
}

fn relative(name: &str) -> Result<&Path> {
    if !name.starts_with("assets/") && !name.starts_with("dist/") {
        return Err(err(format!(
            "fixture path must start with assets/ or dist/: {name}"
        )));
    }
    if name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(err(format!("invalid fixture path: {name}")));
    }
    Ok(Path::new(name))
}

fn fingerprint(bytes: &[u8], expected_size: u64, expected_crc: u32, name: &str) -> Result<()> {
    if bytes.len() as u64 != expected_size || crc32fast::hash(bytes) != expected_crc {
        return Err(err(format!("fixture length/CRC mismatch: {name}")));
    }
    Ok(())
}

fn history(root: &Path, manifest: &Manifest) -> Result<BTreeMap<String, Vec<u8>>> {
    let bytes = fs::read(root.join(ARCHIVE))?;
    fingerprint(
        &bytes,
        manifest.archive_bytes,
        manifest.archive_crc32,
        ARCHIVE,
    )?;
    let mut files = archive::unzip(&bytes)?;
    if files.len() != manifest.files.len() {
        return Err(err(
            "historical fixture archive and manifest file counts differ",
        ));
    }
    let mut selected = BTreeMap::new();
    for entry in &manifest.files {
        relative(&entry.path)?;
        relative(entry.target())?;
        let bytes = files.remove(&entry.path).ok_or_else(|| {
            err(format!(
                "historical fixture archive is missing {}",
                entry.path
            ))
        })?;
        fingerprint(&bytes, entry.bytes, entry.crc32, &entry.path)?;
        if selected.insert(entry.target().into(), bytes).is_some() {
            return Err(err(format!("duplicate fixture target: {}", entry.target())));
        }
    }
    Ok(selected)
}

fn inputs(root: &Path, manifest: &Manifest) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = history(root, manifest)?;
    for id in &manifest.maps {
        let p = Project::load(root, id)?;
        let cfg = &p.cfg["COORDINATE_PIPELINE"];
        if cfg["source_name"].as_str() != Some(&p.name) {
            return Err(err(format!(
                "{id}: version and frozen source differ; adopt the verified release first"
            )));
        }
        let source = cfg["source_package"]
            .as_str()
            .ok_or_else(|| err("missing source_package"))?;
        let package = archive::unzip(&fs::read(p.path(source)?)?)?;
        let map_name = format!("{}.map", p.name);
        let native = package
            .get(&map_name)
            .ok_or_else(|| err(format!("{id}: frozen source has no {map_name}")))?;
        let expected = cfg["source_crc32"]
            .as_u64()
            .ok_or_else(|| err("missing source_crc32"))?;
        if expected != crc32fast::hash(native) as u64 {
            return Err(err(format!(
                "{id}: frozen native map CRC differs from config"
            )));
        }
        if !package.contains_key("layout.json") {
            return Err(err(format!("{id}: frozen source has no layout.json")));
        }
        for (name, bytes) in package {
            project::safe_component(&name)?;
            let name = if name == "layout.json" {
                "source/layout.json"
            } else {
                &name
            };
            let target = p
                .assets
                .join(name)
                .strip_prefix(root)?
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            relative(&target)?;
            if files.insert(target.clone(), bytes).is_some() {
                return Err(err(format!("duplicate fixture target: {target}")));
            }
        }
    }
    Ok(files)
}

// Preflight every target before writing any file, including parent symlinks.
fn check_target(root: &Path, name: &str, expected: &[u8], require_present: bool) -> Result<bool> {
    let relative = relative(name)?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(err(format!(
                    "fixture target uses a symlink: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    match fs::read(root.join(relative)) {
        Ok(actual) if actual == expected => Ok(true),
        Ok(_) => Err(err(format!(
            "existing fixture differs; refusing to overwrite: {name}. Use a separate clean checkout."
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !require_present => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(err(format!(
            "missing fixture: {name}; run cargo run --locked --example test-data -- restore"
        ))),
        Err(e) => Err(e.into()),
    }
}

fn restore(root: &Path, files: &BTreeMap<String, Vec<u8>>, check_only: bool) -> Result<usize> {
    let mut missing = Vec::new();
    for (name, bytes) in files {
        if !check_target(root, name, bytes, check_only)? {
            missing.push((name, bytes));
        }
    }
    for (name, bytes) in &missing {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        if let Err(e) = file.write_all(bytes) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(e.into());
        }
    }
    Ok(missing.len())
}

fn capture(root: &Path, manifest: &mut Manifest) -> Result<()> {
    let mut files = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for entry in &mut manifest.files {
        relative(&entry.path)?;
        if !targets.insert(entry.target().to_owned()) {
            return Err(err(format!("duplicate fixture target: {}", entry.target())));
        }
        let bytes = fs::read(root.join(relative(entry.target())?))?;
        entry.bytes = bytes.len() as u64;
        entry.crc32 = crc32fast::hash(&bytes);
        if files.insert(entry.path.clone(), bytes).is_some() {
            return Err(err(format!("duplicate fixture path: {}", entry.path)));
        }
    }
    manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
    let zipped = archive::zip(&files.into_iter().collect::<Vec<_>>())?;
    manifest.archive_bytes = zipped.len() as u64;
    manifest.archive_crc32 = crc32fast::hash(&zipped);
    project::atomic_write(&root.join(ARCHIVE), &zipped)?;
    project::write_json(&root.join(MANIFEST), manifest)?;
    Ok(())
}

fn run() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "check".into());
    if args.next().is_some() || !["restore", "check", "capture"].contains(&command.as_str()) {
        return Err(err(
            "usage: cargo run --locked --example test-data -- <restore|check|capture>",
        ));
    }
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(root.join(MANIFEST))?)?;
    if manifest.schema_version != 1 || manifest.maps.is_empty() || manifest.files.is_empty() {
        return Err(err("unsupported or empty test-data manifest"));
    }
    if command == "capture" {
        capture(&root, &mut manifest)?;
        println!(
            "Captured {} historical files ({} compressed bytes)",
            manifest.files.len(),
            manifest.archive_bytes
        );
    } else {
        let files = inputs(&root, &manifest)?;
        let restored = restore(&root, &files, command == "check")?;
        println!(
            "Verified {} fixture files; restored {restored}; no network or game installation required",
            files.len()
        );
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("test-data: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ra3-test-data-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn conflict_is_detected_before_any_restoration_and_matching_files_are_idempotent() {
        let root = Temp::new();
        let files = BTreeMap::from([
            ("assets/a/new".into(), b"new".to_vec()),
            ("dist/b/existing".into(), b"expected".to_vec()),
        ]);
        fs::create_dir_all(root.0.join("dist/b")).unwrap();
        fs::write(root.0.join("dist/b/existing"), "local work").unwrap();
        assert!(restore(&root.0, &files, false).is_err());
        assert!(!root.0.join("assets").exists());
        assert_eq!(
            fs::read(root.0.join("dist/b/existing")).unwrap(),
            b"local work"
        );
        fs::write(root.0.join("dist/b/existing"), "expected").unwrap();
        assert!(restore(&root.0, &files, true).is_err());
        assert_eq!(restore(&root.0, &files, false).unwrap(), 1);
        assert_eq!(restore(&root.0, &files, false).unwrap(), 0);
        assert_eq!(restore(&root.0, &files, true).unwrap(), 0);
    }

    #[test]
    fn invalid_paths_and_damaged_archive_are_rejected_before_writing() {
        for path in [
            "assets/../config.json",
            "assets//a",
            "dist/./a",
            "assets/C:/a",
            "assets/a\\b",
            "/assets/a",
            "maps/a/config.json",
        ] {
            assert!(relative(path).is_err(), "{path}");
        }
        let root = Temp::new();
        fs::create_dir_all(root.0.join("tests/fixtures")).unwrap();
        let mut manifest = Manifest {
            schema_version: 1,
            maps: vec!["test".into()],
            archive_bytes: 0,
            archive_crc32: 0,
            files: vec![Entry {
                path: "assets/a/file".into(),
                target: None,
                bytes: 0,
                crc32: 0,
                purpose: "regression".into(),
            }],
        };
        fs::create_dir_all(root.0.join("assets/a")).unwrap();
        fs::write(root.0.join("assets/a/file"), "valid").unwrap();
        capture(&root.0, &mut manifest).unwrap();
        let captured = fs::read(root.0.join(ARCHIVE)).unwrap();
        capture(&root.0, &mut manifest).unwrap();
        assert_eq!(captured, fs::read(root.0.join(ARCHIVE)).unwrap());
        assert_eq!(
            history(&root.0, &manifest).unwrap()["assets/a/file"],
            b"valid"
        );
        manifest.files[0].crc32 ^= 1;
        assert!(history(&root.0, &manifest).is_err());
        fs::write(root.0.join(ARCHIVE), "damaged").unwrap();
        assert!(history(&root.0, &manifest).is_err());
    }

    #[test]
    fn historical_restore_uses_geographic_targets_without_changing_archive_members() {
        let root = Temp::new();
        let manifest: Manifest =
            serde_json::from_str(include_str!("../tests/fixtures/historical-inputs.json")).unwrap();
        let original = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIVE)).unwrap();
        fs::create_dir_all(root.0.join("tests/fixtures")).unwrap();
        fs::write(root.0.join(ARCHIVE), &original).unwrap();
        let files = history(&root.0, &manifest).unwrap();
        assert_eq!(
            restore(&root.0, &files, false).unwrap(),
            manifest.files.len()
        );
        for entry in &manifest.files {
            let target = entry
                .target
                .as_deref()
                .expect("explicit restore projection");
            assert_ne!(entry.path, target);
            let id = target.split('/').nth(1).unwrap();
            assert!(manifest.maps.iter().any(|map| map == id));
            assert_eq!(target.split('/').nth(2), entry.path.split('/').nth(2));
            if target.starts_with("assets/") {
                assert!(project::parse_version(target.split('/').nth(2).unwrap()).is_err());
            }
            assert_eq!(fs::read(root.0.join(target)).unwrap(), files[target]);
            assert!(!root.0.join(&entry.path).exists());
        }
        assert_eq!(fs::read(root.0.join(ARCHIVE)).unwrap(), original);
        assert_eq!(restore(&root.0, &files, true).unwrap(), 0);
        for old in ["[CHAI]千壑横原", "千壑横原"] {
            assert!(!root.0.join("assets").join(old).exists());
            assert!(!root.0.join("dist").join(old).exists());
        }
        let mut invalid = manifest;
        invalid.files[0].target = Some("assets/../outside".into());
        assert!(history(&root.0, &invalid).is_err());
        invalid.files[0].target = invalid.files[1].target.clone();
        assert!(history(&root.0, &invalid).is_err());
    }

    #[test]
    fn capture_reads_projected_targets_and_keeps_historical_archive_paths() {
        let root = Temp::new();
        fs::create_dir_all(root.0.join("tests/fixtures")).unwrap();
        fs::create_dir_all(root.0.join("assets/陕西/[CHAI]千壑横原T0.0.1")).unwrap();
        fs::write(
            root.0.join("assets/陕西/[CHAI]千壑横原T0.0.1/map.str"),
            "unchanged bytes",
        )
        .unwrap();
        let mut manifest = Manifest {
            schema_version: 1,
            maps: vec!["陕西".into()],
            archive_bytes: 0,
            archive_crc32: 0,
            files: vec![Entry {
                path: "assets/[CHAI]千壑横原/[CHAI]千壑横原T0.0.1/map.str".into(),
                target: Some("assets/陕西/[CHAI]千壑横原T0.0.1/map.str".into()),
                bytes: 0,
                crc32: 0,
                purpose: "path projection regression".into(),
            }],
        };
        capture(&root.0, &mut manifest).unwrap();
        let zipped = archive::unzip(&fs::read(root.0.join(ARCHIVE)).unwrap()).unwrap();
        assert_eq!(zipped[&manifest.files[0].path], b"unchanged bytes");
        assert!(!zipped.contains_key(manifest.files[0].target()));
        assert_eq!(
            history(&root.0, &manifest).unwrap()[manifest.files[0].target()],
            b"unchanged bytes"
        );
    }
}
