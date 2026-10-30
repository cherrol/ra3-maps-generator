//! Restore only the complete current Henan source into local test outputs.
use ra3maps::{
    Result, err,
    formats::archive,
    pipeline::{project::Project, release},
};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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

fn inputs(p: &Project, bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut source = archive::unzip(bytes)?;
    let map_name = format!("{}.map", p.name);
    let native = source
        .get(&map_name)
        .ok_or_else(|| err("current frozen source map is missing"))?;
    if p.cfg["COORDINATE_PIPELINE"]["source_name"].as_str() != Some(&p.name)
        || p.cfg["COORDINATE_PIPELINE"]["source_crc32"].as_u64()
            != Some(u64::from(crc32fast::hash(native)))
    {
        return Err(err(
            "current Henan source name/version/CRC differs from config",
        ));
    }
    let layout_bytes = source
        .remove("layout.json")
        .ok_or_else(|| err("current frozen source layout is missing"))?;
    // Match the build's derived source/layout.json serialization; the frozen
    // ZIP itself can omit its final newline without changing the layout.
    let layout: ra3maps::terrain::Layout = serde_json::from_slice(&layout_bytes)?;
    let mut layout = serde_json::to_vec_pretty(&layout)?;
    layout.push(b'\n');
    let game_files: Vec<_> = source.into_iter().collect();
    for (name, _) in &game_files {
        ra3maps::pipeline::project::safe_component(name)?;
    }
    release::validate_runtime_files(&game_files)?;
    let mut files = BTreeMap::new();
    for (name, data) in game_files
        .into_iter()
        .chain([(String::from("source/layout.json"), layout)])
    {
        let target = p
            .assets
            .join(name)
            .strip_prefix(&p.root)?
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        relative(&target)?;
        files.insert(target, data);
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

fn run() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "check".into());
    if args.next().is_some() || !["restore", "check"].contains(&command.as_str()) {
        return Err(err(
            "usage: cargo run --locked --example test-data -- <restore|check>",
        ));
    }
    let p = Project::load(&root, "河南")?;
    let path = p.cfg["COORDINATE_PIPELINE"]["source_package"]
        .as_str()
        .ok_or_else(|| err("current source_package is missing"))?;
    let files = inputs(&p, &fs::read(p.path(path)?)?)?;
    let restored = restore(&root, &files, command == "check")?;
    println!(
        "Verified {} current Henan source files; restored {restored}",
        files.len()
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("test-data: {error}");
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
                "ra3-current-test-data-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
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
    fn unsafe_paths_are_rejected() {
        for name in [
            "assets/../config.json",
            "assets//a",
            "dist/./a",
            "assets/C:/a",
            "assets/a\\b",
            "/assets/a",
            "maps/a/config.json",
        ] {
            assert!(relative(name).is_err(), "{name}");
        }
    }
    #[test]
    fn only_the_current_crc_bound_complete_source_is_restored() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "河南").unwrap();
        let bytes = fs::read(
            p.path(
                p.cfg["COORDINATE_PIPELINE"]["source_package"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let files = inputs(&p, &bytes).unwrap();
        assert_eq!(files.len(), 6);
        assert!(files.keys().all(|name| name.starts_with("assets/河南/")));
        p.cfg["COORDINATE_PIPELINE"]["source_crc32"] = serde_json::json!(0);
        assert!(inputs(&p, &bytes).is_err());
        assert!(inputs(&p, b"damaged archive").is_err());
    }
}
