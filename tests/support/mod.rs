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
