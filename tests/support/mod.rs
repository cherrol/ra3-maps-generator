use serde_json::Value;
use std::{fs, path::Path};

pub fn copy_declared_inputs(source: &Path, destination: &Path, cfg: &Value) {
    for (key, default) in [("LAYERS", "layers.npz"), ("LAYOUT", "layout.json")] {
        let relative = cfg[key].as_str().unwrap_or(default);
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(relative), target).unwrap();
    }
}

#[allow(dead_code)]
pub fn current_package() -> (ra3maps::pipeline::project::Project, Vec<(String, Vec<u8>)>) {
    use ra3maps::{formats::archive, pipeline::project::Project};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = Project::load(root, "河南").unwrap();
    let source = p.cfg["COORDINATE_PIPELINE"]["source_package"]
        .as_str()
        .unwrap();
    let mut files = archive::unzip(&fs::read(p.path(source).unwrap()).unwrap()).unwrap();
    let native = files.get(&format!("{}.map", p.name)).unwrap();
    assert_eq!(
        u64::from(crc32fast::hash(native)),
        p.cfg["COORDINATE_PIPELINE"]["source_crc32"]
            .as_u64()
            .unwrap()
    );
    files.remove("layout.json");
    (p, files.into_iter().collect())
}

#[allow(dead_code)]
pub fn current_map() -> (
    ra3maps::pipeline::project::Project,
    ra3maps::formats::map::MapFile,
) {
    let (p, files) = current_package();
    let native = ra3maps::formats::map::MapFile::load_bytes(
        &files
            .iter()
            .find(|(name, _)| name.ends_with(".map"))
            .unwrap()
            .1,
    )
    .unwrap();
    (p, native)
}

#[allow(dead_code)]
pub fn copy_current_project(destination: &Path) -> ra3maps::pipeline::project::Project {
    use ra3maps::pipeline::project::Project;
    fn tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                tree(&entry.path(), &destination.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
            }
        }
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let original = Project::load(source, "河南").unwrap();
    let recipe = destination.join("maps/河南");
    fs::create_dir_all(&recipe).unwrap();
    tree(&source.join("templates"), &destination.join("templates"));
    for name in ["config.json", "version.json", "layers.npz"] {
        fs::copy(original.map_dir().join(name), recipe.join(name)).unwrap();
    }
    copy_declared_inputs(&original.map_dir(), &recipe, &original.cfg);
    for name in [
        original.cfg["COORDINATE_PIPELINE"]["source_package"].as_str(),
        original.cfg["COORDINATE_PIPELINE"]["baseline"].as_str(),
        original.cfg["COORDINATE_PIPELINE"]["layout"].as_str(),
        original.cfg["COORDINATE_PIPELINE"]["policy"].as_str(),
        original.cfg["RESOURCE_DISTRIBUTION"].as_str(),
    ]
    .into_iter()
    .flatten()
    {
        let target = recipe.join(name);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(original.path(name).unwrap(), target).unwrap();
    }
    fs::write(
        destination.join("Cargo.toml"),
        "# isolated current-map regression\n",
    )
    .unwrap();
    Project::load(destination, "河南").unwrap()
}
