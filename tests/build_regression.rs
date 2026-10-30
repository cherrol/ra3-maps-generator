//! Current Henan builds and publication transactions; no retired-map replays.
use ra3maps::{
    formats::archive,
    pipeline::{
        build::{self, Options},
        project::{self, Project},
    },
    terrain::Layout,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
mod support;
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ra3-current-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        support::copy_current_project(&path);
        Self(path)
    }
    fn project(&self) -> Project {
        Project::load(&self.0, "河南").unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn current_frozen_map_rebuild_preserves_bytes_and_passes_complete_validation() {
    let f = Fixture::new();
    let p = f.project();
    let source =
        archive::unzip(&fs::read(p.path("authoring/source.zip").unwrap()).unwrap()).unwrap();
    build::run(
        &f.0,
        "河南",
        &Options {
            only: vec!["map".into(), "spatial".into()],
            ..Options::default()
        },
    )
    .unwrap();
    let p = f.project();
    for name in build::release_files(&p) {
        assert_eq!(
            fs::read(p.assets.join(&name)).unwrap(),
            source[&name],
            "{name}"
        );
    }
    build::verify(&p).unwrap();
    let native =
        ra3maps::formats::map::MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    let allocation = ra3maps::terrain::resource_routes::validate(&p, &native)
        .unwrap()
        .unwrap();
    assert_eq!(allocation["home_counts"], json!([2, 2, 2, 2, 2, 2, 2, 2]));
    assert_eq!(allocation["home_maximum"], 2);
    assert_eq!(allocation["mine_count"], 48);
}

#[test]
fn failed_bump_restores_exact_version_and_keeps_preexisting_artifacts() {
    let f = Fixture::new();
    let version = project::version_path(&f.0, "河南");
    let original = fs::read(&version).unwrap();
    let next_state =
        project::bump(&project::load_version(&f.0, "河南").unwrap(), false, 1, "").unwrap();
    project::write_json(&version, &next_state).unwrap();
    let next = f.project();
    project::atomic_write(&version, &original).unwrap();
    fs::create_dir_all(&next.assets).unwrap();
    fs::write(next.assets.join("keep.txt"), b"existing artifact").unwrap();
    assert!(
        build::run(
            &f.0,
            "河南",
            &Options {
                bump: true,
                only: vec!["verify".into()],
                ..Options::default()
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(version).unwrap(), original);
    assert_eq!(
        fs::read(next.assets.join("keep.txt")).unwrap(),
        b"existing artifact"
    );
}

#[test]
fn invalid_stage_is_rejected_before_version_changes() {
    let f = Fixture::new();
    let path = project::version_path(&f.0, "河南");
    let before = fs::read(&path).unwrap();
    assert!(
        build::run(
            &f.0,
            "河南",
            &Options {
                bump: true,
                only: vec!["typo".into()],
                ..Options::default()
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn distribution_checks_complete_inputs_before_touching_existing_output() {
    let f = Fixture::new();
    let p = f.project();
    fs::create_dir_all(&p.dist).unwrap();
    fs::write(p.dist.join("keep.txt"), b"existing package").unwrap();
    assert!(build::dist(&p, true).is_err());
    assert_eq!(
        fs::read(p.dist.join("keep.txt")).unwrap(),
        b"existing package"
    );
    assert!(!build::zip_path(&p).exists());
}

#[test]
fn layout_metadata_cannot_override_the_actual_spawn_object() {
    let f = Fixture::new();
    let p = f.project();
    let path = p.path(p.string("LAYOUT", "layout.json")).unwrap();
    let mut layout: Layout = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let actual_base = layout.base_cells[0];
    layout.base_cells[0][0] += 20.;
    project::write_json(&path, &layout).unwrap();
    assert_eq!(Layout::load(&p).unwrap().base_cells[0], actual_base);
    let mut blueprint = p.clone();
    blueprint
        .cfg
        .as_object_mut()
        .unwrap()
        .remove("COORDINATE_PIPELINE");
    assert!(ra3maps::terrain::Terrain::generate(&blueprint).is_err());
}
