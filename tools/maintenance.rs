//! Fixed-list cache cleanup. The default is a read-only JSON plan.
use ra3maps::{Result, err};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

const SIGNATURE: &str = "Signature: 8a477f597d28d172789f06886806bc55";
// (cache, Cargo owner). None denotes Python bytecode with adjacent source files.
const CACHES: [(&str, Option<&str>); 7] = [
    ("target/debug", Some("")),
    ("target/yunnan-preview", Some("maps/云南/preview-tool")),
    ("work/local-ramp/target", Some("work/local-ramp")),
    (
        "work/high-mountains/tool/target",
        Some("work/high-mountains/tool"),
    ),
    (
        "work/terrain-material-preview/target",
        Some("work/terrain-material-preview/staging"),
    ),
    ("work/rename-release/__pycache__", None),
    ("work/china-terrain-preview/tools/__pycache__", None),
];

#[derive(Debug, Eq, PartialEq)]
struct Node {
    path: PathBuf,
    directory: bool,
    identity: Option<(u64, u64)>,
    links: Option<u64>,
    length: u64,
    physical: Option<u64>,
    modified: SystemTime,
}
struct Plan {
    root: PathBuf,
    candidates: Vec<Value>,
    nodes: Vec<Node>,
}

fn relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(err(format!(
            "invalid cache-relative path: {}",
            path.display()
        )));
    }
    Ok(())
}
fn linked(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        } // Junctions/reparse points.
    }
    false
}
fn metadata(root: &Path, path: &Path) -> Result<Option<fs::Metadata>> {
    relative(path)?;
    let mut current = root.to_path_buf();
    let components = path.components().collect::<Vec<_>>();
    for (i, part) in components.iter().enumerate() {
        current.push(part);
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if linked(&meta) {
            return Err(err(format!(
                "cache path uses a link: {}",
                current.display()
            )));
        }
        if i + 1 == components.len() {
            return Ok(Some(meta));
        }
        if !meta.is_dir() {
            return Err(err(format!(
                "cache parent is not a directory: {}",
                current.display()
            )));
        }
    }
    unreachable!()
}
fn node(root: &Path, path: &Path) -> Result<Node> {
    let meta = metadata(root, path)?.ok_or_else(|| {
        err(format!(
            "cache changed during inspection: {}",
            path.display()
        ))
    })?;
    if !meta.is_file() && !meta.is_dir() {
        return Err(err("unsupported entry in cache"));
    }
    #[cfg(unix)]
    let (identity, links, physical) = {
        use std::os::unix::fs::MetadataExt;
        (
            Some((meta.dev(), meta.ino())),
            Some(meta.nlink()),
            Some(meta.blocks() * 512),
        )
    };
    #[cfg(not(unix))]
    let (identity, links, physical) = (None, None, None);
    Ok(Node {
        path: path.into(),
        directory: meta.is_dir(),
        identity,
        links,
        physical,
        length: meta.len(),
        modified: meta.modified()?,
    })
}
fn require_source(root: &Path, path: &Path) -> Result<()> {
    if !metadata(root, path)?.is_some_and(|m| m.is_file() && m.len() != 0) {
        return Err(err(format!(
            "cache owner source is missing: {}",
            path.display()
        )));
    }
    Ok(())
}
fn verify_owner(root: &Path, cache: &str, owner: Option<&str>) -> Result<()> {
    if let Some(owner) = owner {
        let marker = if cache == "target/debug" {
            Path::new("target/CACHEDIR.TAG").to_path_buf()
        } else {
            Path::new(cache).join("CACHEDIR.TAG")
        };
        require_source(root, &marker)?;
        if !fs::read_to_string(root.join(&marker))?.starts_with(SIGNATURE) {
            return Err(err(format!(
                "Cargo cache signature differs: {}",
                marker.display()
            )));
        }
        let manifest = Path::new(owner).join("Cargo.toml");
        require_source(root, &manifest)?;
        require_source(root, &Path::new(owner).join("src/main.rs"))?;
        if !fs::read_to_string(root.join(manifest))?.contains("[package]") {
            return Err(err("cache owner is not a Cargo package"));
        }
    }
    Ok(())
}
fn walk(root: &Path, path: &Path, python: bool, nodes: &mut Vec<Node>) -> Result<()> {
    let current = node(root, path)?;
    if current.directory {
        let mut children = fs::read_dir(root.join(path))?
            .map(|entry| entry.map(|e| path.join(e.file_name())))
            .collect::<std::io::Result<Vec<_>>>()?;
        children.sort();
        nodes.push(current);
        for child in children {
            walk(root, &child, python, nodes)?;
        }
    } else {
        if python {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| err("invalid Python cache filename"))?;
            if !name.ends_with(".pyc") {
                return Err(err("non-bytecode data inside Python cache"));
            }
            let source = path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!("{}.py", name.split('.').next().unwrap()));
            require_source(root, &source)?;
        }
        nodes.push(current);
    }
    Ok(())
}
fn stats(nodes: &[Node]) -> Value {
    let mut seen = BTreeSet::new();
    let mut physical = 0;
    let mut unique = 0;
    for n in nodes {
        if let Some(id) = n.identity
            && seen.insert(id)
        {
            physical += n.physical.unwrap();
            if !n.directory {
                unique += n.length;
            }
        }
    }
    json!({"files":nodes.iter().filter(|n|!n.directory).count(),
        "directories":nodes.iter().filter(|n|n.directory).count(),
        "logical_bytes":nodes.iter().filter(|n|!n.directory).map(|n|n.length).sum::<u64>(),
        "unique_logical_bytes":if cfg!(unix) {Some(unique)}else{None},
        "physical_bytes":if cfg!(unix) {Some(physical)}else{None}})
}
fn plan(root: &Path) -> Result<Plan> {
    if linked(&fs::symlink_metadata(root)?) {
        return Err(err("cleanup root cannot be a link"));
    }
    let root = root.canonicalize()?;
    let mut candidates = Vec::new();
    let mut nodes = Vec::new();
    for (cache, owner) in CACHES {
        let kind = if owner.is_some() {
            "cargo"
        } else {
            "python-bytecode"
        };
        let Some(meta) = metadata(&root, Path::new(cache))? else {
            candidates.push(json!({"path":cache,"kind":kind,"status":"missing"}));
            continue;
        };
        if !meta.is_dir() {
            return Err(err(format!("cache is not a directory: {cache}")));
        }
        verify_owner(&root, cache, owner)?;
        let start = nodes.len();
        walk(&root, Path::new(cache), owner.is_none(), &mut nodes)?;
        candidates.push(
            json!({"path":cache,"kind":kind,"status":"eligible","size":stats(&nodes[start..])}),
        );
    }
    let mut links = BTreeMap::new();
    for n in nodes.iter().filter(|n| !n.directory) {
        if let Some(id) = n.identity {
            let count = links.entry(id).or_insert((n.links.unwrap(), 0));
            if count.0 != n.links.unwrap() {
                return Err(err("hard-link count changed during inspection"));
            }
            count.1 += 1;
        }
    }
    if links.values().any(|(actual, selected)| actual != selected) {
        return Err(err(
            "cache contains a hard link outside the selected cleanup trees",
        ));
    }
    Ok(Plan {
        root,
        candidates,
        nodes,
    })
}
fn execute(planned: Plan, apply: bool) -> Result<Value> {
    let mut report = json!({"schema_version":1,"mode":if apply {"apply"}else{"dry-run"},
        "root":planned.root,"apply_supported":cfg!(unix),"hardlinks_verified":cfg!(unix),
        "candidates":planned.candidates,"total":stats(&planned.nodes),
        "removed_files":0,"removed_directories":0});
    if !apply {
        return Ok(report);
    }
    if !cfg!(unix) {
        return Err(err(
            "apply is disabled on this platform: hard-link ownership cannot be verified with the supported stable APIs",
        ));
    }
    let executable = std::env::current_exe()?.canonicalize()?;
    if CACHES
        .iter()
        .any(|(cache, _)| executable.starts_with(planned.root.join(cache)))
    {
        return Err(err(
            "maintenance is running from a selected cache; use --release or a separate CARGO_TARGET_DIR",
        ));
    }
    if plan(&planned.root)?.nodes != planned.nodes {
        return Err(err("cache changed after planning; no files were removed"));
    }
    let mut removed_links = BTreeMap::new();
    let mut files = 0;
    for expected in planned.nodes.iter().filter(|n| !n.directory) {
        let actual = node(&planned.root, &expected.path)?;
        let removed = *removed_links.get(&expected.identity).unwrap_or(&0);
        if actual.directory
            || actual.identity != expected.identity
            || actual.length != expected.length
            || actual.modified != expected.modified
            || actual.links.unwrap() + removed != expected.links.unwrap()
        {
            return Err(err(format!(
                "cache changed before removal: {}",
                expected.path.display()
            )));
        }
        fs::remove_file(planned.root.join(&expected.path))?;
        *removed_links.entry(expected.identity).or_default() += 1;
        files += 1;
    }
    let mut directories = planned
        .nodes
        .iter()
        .filter(|n| n.directory)
        .collect::<Vec<_>>();
    directories.sort_by_key(|n| std::cmp::Reverse(n.path.components().count()));
    let mut removed_directories = 0;
    for expected in directories {
        let actual = node(&planned.root, &expected.path)?;
        if !actual.directory || actual.identity != expected.identity {
            return Err(err("cache directory changed before removal"));
        }
        // Unexpected new files survive: never use remove_dir_all for application.
        fs::remove_dir(planned.root.join(&expected.path))?;
        removed_directories += 1;
    }
    report["removed_files"] = json!(files);
    report["removed_directories"] = json!(removed_directories);
    Ok(report)
}
fn run() -> Result<()> {
    let mut apply = false;
    let mut dry_run = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "clean" => {}
            "--apply" => apply = true,
            "--dry-run" => dry_run = true,
            "--help" | "-h" => {
                println!(
                    "cargo run --release --locked --example maintenance -- [clean] [--dry-run | --apply]\nDefault: JSON dry-run. Only the fixed cache list is eligible. Run after builds finish. Unix supports apply; other platforms remain read-only."
                );
                return Ok(());
            }
            _ => return Err(err(format!("unknown maintenance argument: {arg}"))),
        }
    }
    if apply && dry_run {
        return Err(err("choose either --dry-run or --apply"));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&execute(
            plan(Path::new(env!("CARGO_MANIFEST_DIR")))?,
            apply
        )?)?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("maintenance: {e}");
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
            let root = std::env::temp_dir().join(format!(
                "ra3-maintenance-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            for d in ["src", "target/debug/empty", "target/release", "maps/test"] {
                fs::create_dir_all(root.join(d)).unwrap();
            }
            for (p, b) in [
                ("Cargo.toml", "[package]"),
                ("src/main.rs", "fn main() {}"),
                ("target/CACHEDIR.TAG", SIGNATURE),
                ("target/debug/artifact.o", "compiled"),
                ("maps/test/preview.png", "protected"),
                ("target/release/ra3maps", "retained"),
            ] {
                fs::write(root.join(p), b).unwrap();
            }
            Self(root)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn dry_run_preserves_data_and_apply_stays_within_the_fixed_cache() {
        let t = Temp::new();
        let result = execute(plan(&t.0).unwrap(), false).unwrap();
        assert_eq!(result["removed_files"], 0);
        assert!(t.0.join("target/debug/artifact.o").is_file());
        let result = execute(plan(&t.0).unwrap(), true);
        if cfg!(unix) {
            let r = result.unwrap();
            assert_eq!(r["removed_files"], 1);
            assert_eq!(r["removed_directories"], 2);
            assert!(!t.0.join("target/debug").exists());
        } else {
            assert!(result.is_err());
            assert!(t.0.join("target/debug/artifact.o").is_file());
        }
        assert_eq!(
            fs::read_to_string(t.0.join("maps/test/preview.png")).unwrap(),
            "protected"
        );
        assert_eq!(
            fs::read_to_string(t.0.join("target/release/ra3maps")).unwrap(),
            "retained"
        );
    }
    #[test]
    fn invalid_paths_markers_and_changed_plans_never_delete_files() {
        for p in ["", "../maps", "/tmp/cache", "target/../../maps"] {
            assert!(relative(Path::new(p)).is_err());
        }
        let t = Temp::new();
        let old = plan(&t.0).unwrap();
        fs::write(t.0.join("target/debug/new.o"), "new").unwrap();
        assert!(execute(old, true).is_err());
        assert!(t.0.join("target/debug/artifact.o").is_file());
        fs::write(t.0.join("target/CACHEDIR.TAG"), "not a cache").unwrap();
        assert!(plan(&t.0).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn external_hard_links_are_rejected_and_internal_aliases_count_once() {
        let t = Temp::new();
        let f = t.0.join("target/debug/artifact.o");
        let outside = t.0.join("maps/test/link.o");
        fs::hard_link(&f, &outside).unwrap();
        assert!(plan(&t.0).is_err());
        fs::remove_file(outside).unwrap();
        fs::hard_link(&f, t.0.join("target/debug/alias.o")).unwrap();
        let p = plan(&t.0).unwrap();
        let s = stats(&p.nodes);
        assert_eq!(
            s["logical_bytes"].as_u64().unwrap(),
            2 * s["unique_logical_bytes"].as_u64().unwrap()
        );
        assert_eq!(execute(p, true).unwrap()["removed_files"], 2);
    }
    #[cfg(unix)]
    #[test]
    fn parent_and_nested_symlinks_are_rejected_before_other_caches_change() {
        use std::os::unix::fs::symlink;
        let t = Temp::new();
        let link = t.0.join("target/debug/link");
        symlink(t.0.join("maps"), &link).unwrap();
        assert!(plan(&t.0).is_err());
        fs::remove_file(link).unwrap();
        fs::create_dir_all(t.0.join("work")).unwrap();
        symlink(t.0.join("maps"), t.0.join("work/local-ramp")).unwrap();
        assert!(plan(&t.0).is_err());
        assert!(t.0.join("target/debug/artifact.o").is_file());
    }
}
