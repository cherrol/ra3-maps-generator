//! Complete current-package validation. Damaged imports must be rejected.
use ra3maps::{
    formats::{archive, map::MapFile, refpack},
    pipeline::{build, release},
};
mod support;

#[test]
fn current_complete_package_passes_and_missing_paired_files_fail() {
    let (_, files) = support::current_package();
    release::validate_runtime_files(&files).unwrap();
    for required in ["map.xml", "overrides.xml", "map.str"] {
        let missing: Vec<_> = files
            .iter()
            .filter(|(name, _)| name != required)
            .cloned()
            .collect();
        assert!(
            release::validate_runtime_files(&missing).is_err(),
            "{required}"
        );
    }
}

#[test]
fn empty_native_manifest_is_rejected_without_mutating_input() {
    let (_, mut files) = support::current_package();
    let entry = files
        .iter_mut()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap();
    let mut native = MapFile::load_bytes(&entry.1).unwrap();
    native.get_mut("AssetList").unwrap().payload = 0_u32.to_le_bytes().to_vec();
    entry.1 = refpack::pack(&native.encode().unwrap()).unwrap();
    let before = files.clone();
    assert!(release::validate_runtime_files(&files).is_err());
    assert_eq!(files, before);
}

#[test]
fn repair_entries_reject_invalid_source_before_editing_or_committing() {
    use std::{fs, path::Path};
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let scratch = Scratch(
        std::env::temp_dir().join(format!("ra3-repair-source-gate-{}", std::process::id())),
    );
    let p = support::copy_current_project(&scratch.0);
    let (_, mut files) = support::current_package();
    let native = files.iter_mut().find(|(n, _)| n.ends_with(".map")).unwrap();
    let mut map = MapFile::load_bytes(&native.1).unwrap();
    map.get_mut("AssetList").unwrap().payload = 0_u32.to_le_bytes().to_vec();
    native.1 = refpack::pack(&map.encode().unwrap()).unwrap();
    let expected = release::validate_files(&files).unwrap_err().to_string();
    let zipped = archive::zip(
        &files
            .into_iter()
            .map(|(n, b)| (format!("{}/{n}", p.name), b))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let source = scratch.0.join("invalid-source.zip");
    fs::write(&source, &zipped).unwrap();
    let protected = ["config.json", "version.json", "authoring/source.zip"].map(|name| {
        (
            p.map_dir().join(name),
            fs::read(p.map_dir().join(name)).unwrap(),
        )
    });
    let options = build::Options {
        bump: true,
        ..Default::default()
    };
    type Repair = fn(
        &Path,
        &str,
        &Path,
        &build::Options,
    ) -> ra3maps::Result<ra3maps::pipeline::project::Project>;
    for repair in [
        release::repair_resources as Repair,
        release::repair_terrain_atlas,
        release::repair_terrain_package,
    ] {
        let error = repair(&scratch.0, "河南", &source, &options).err().unwrap();
        assert_eq!(error.to_string(), expected);
        assert_eq!(fs::read(&source).unwrap(), zipped);
        for (path, before) in &protected {
            assert_eq!(&fs::read(path).unwrap(), before);
        }
        assert!(!scratch.0.join("assets").exists());
        assert!(!scratch.0.join("dist").exists());
    }
}

#[test]
fn removing_a_script_declaration_is_not_hidden_by_a_decoy_comment() {
    let (_, mut files) = support::current_package();
    let xml = &mut files
        .iter_mut()
        .find(|(name, _)| name == "map.xml")
        .unwrap()
        .1;
    let text = String::from_utf8(xml.clone()).unwrap();
    assert!(text.contains("<GameScriptList"));
    *xml = (text
        .replacen("<GameScriptList", "<GameScriptListReference", 1)
        .replacen("</GameScriptList>", "</GameScriptListReference>", 1)
        + "\n<!-- <GameScriptList id=\"decoy\"/> -->\n")
        .into_bytes();
    assert!(release::validate_runtime_files(&files).is_err());
}

#[test]
fn current_runtime_manifest_is_exact_and_stale_terrain_preloads_are_rejected() {
    let (p, mut files) = support::current_package();
    let native = MapFile::load_bytes(
        &files
            .iter()
            .find(|(name, _)| name.ends_with(".map"))
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        ra3maps::resources::manifest(&native).unwrap(),
        native.get("AssetList").unwrap().payload
    );
    let xml = &mut files
        .iter_mut()
        .find(|(name, _)| name == "map.xml")
        .unwrap()
        .1;
    let text = String::from_utf8(xml.clone()).unwrap();
    *xml = text.replace(&p.version, "T9.9.9").into_bytes();
    assert!(release::validate_runtime_files(&files).is_err());
}

#[test]
fn production_repair_cannot_publish_an_unresolved_nonfactory_native_object() {
    use ra3maps::{
        formats::{
            archive,
            map::{self, Value},
        },
        pipeline::{build, project},
    };
    let root = std::env::temp_dir().join(format!(
        "ra3-repair-full-audit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let cleanup = Cleanup(root);
    let mut p = support::copy_current_project(&cleanup.0);
    p.cfg["OFFICIAL_PLACEMENT"] = serde_json::json!({"mode":"strict","scope":"production_repair"});
    project::write_json(&p.map_dir().join("config.json"), &p.cfg).unwrap();
    let (_, mut files) = support::current_package();
    let entry = files
        .iter_mut()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap();
    let mut native = MapFile::load_bytes(&entry.1).unwrap();
    let mut objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    let ore = objects
        .iter_mut()
        .find(|o| o.type_name == "OreNode")
        .unwrap();
    map::set(&mut ore.props, "objectPrototypeScale", Value::Float(2.));
    native.id("objectPrototypeScale").unwrap();
    native.get_mut("ObjectsList").unwrap().payload = map::write_objects(&objects, &native).unwrap();
    entry.1 = refpack::pack(&native.encode().unwrap()).unwrap();
    // The package itself is complete; the unresolved physical instance must
    // fail the final full audit despite the request to repair only factories.
    release::validate_runtime_files(&files).unwrap();
    let input = cleanup.0.join("source.zip");
    std::fs::write(
        &input,
        archive::zip(
            &files
                .into_iter()
                .map(|(name, bytes)| (format!("{}/{name}", p.name), bytes))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    let version = project::version_path(&cleanup.0, "河南");
    let before = std::fs::read(&version).unwrap();
    let next = project::format_version(&project::bump(&p.state, false, 1, "").unwrap()).unwrap();
    let error = release::repair_production(
        &cleanup.0,
        "河南",
        &input,
        &build::Options {
            bump: true,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("official placement failed"), "{error}");
    assert_eq!(std::fs::read(version).unwrap(), before);
    let next_name = p.release_name(&next).unwrap();
    assert!(!p.release_assets(&next_name).unwrap().exists());
    assert!(
        !cleanup
            .0
            .join("dist/河南")
            .join(format!("{next_name}.zip"))
            .exists()
    );
}
