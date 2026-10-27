use crate::{Result, err};
use chrono::{FixedOffset, Utc};
use serde_json::Value;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub id: String,
    pub cfg: Value,
    pub state: Value,
    pub version: String,
    pub group: String,
    pub name: String,
    pub assets: PathBuf,
    pub dist: PathBuf,
}
pub fn safe_component(s: &str) -> Result<()> {
    if s.is_empty()
        || s.contains(['/', '\\', '\0'])
        || s == "."
        || s == ".."
        || Path::new(s)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(err(format!("invalid path component: {s:?}")));
    }
    Ok(())
}
pub fn maps(root: &Path) -> Result<Vec<String>> {
    let mut maps = Vec::new();
    for e in fs::read_dir(root.join("maps"))? {
        let e = e?;
        if e.path().join("config.json").is_file() {
            maps.push(
                e.file_name()
                    .into_string()
                    .map_err(|_| err("map id is not UTF-8"))?,
            );
        }
    }
    maps.sort();
    Ok(maps)
}
pub fn select(root: &Path, requested: Option<&str>) -> Result<String> {
    let all = maps(root)?;
    if let Some(s) = requested {
        safe_component(s)?;
        if all.iter().any(|v| v == s) {
            return Ok(s.into());
        }
        return Err(err(format!(
            "unknown map {s}; available: {}",
            all.join(", ")
        )));
    }
    for s in [
        std::env::var("RA3MAP_ID").ok(),
        fs::read_to_string(root.join("maps/.default")).ok(),
    ]
    .into_iter()
    .flatten()
    {
        let s = s.trim();
        if all.iter().any(|v| v == s) {
            return Ok(s.into());
        }
    }
    if all.len() == 1 {
        return Ok(all[0].clone());
    }
    Err(err(format!("specify --map; available: {}", all.join(", "))))
}
impl Project {
    pub fn load(root: &Path, id: &str) -> Result<Self> {
        safe_component(id)?;
        let cfg: Value =
            serde_json::from_slice(&fs::read(root.join("maps").join(id).join("config.json"))?)?;
        if cfg["SCHEMA_VERSION"].as_u64() != Some(1) {
            return Err(err("unsupported recipe schema"));
        }
        let state = load_version(root, id)?;
        let version = format_version(&state)?;
        let group = cfg["MAP_NAME"]
            .as_str()
            .ok_or_else(|| err("recipe missing MAP_NAME"))?
            .to_string();
        safe_component(&group)?;
        let publish = cfg["PUBLISH_NAME"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&group);
        let name = format!("{publish}{version}");
        safe_component(&name)?;
        let assets = root.join("assets").join(&group).join(&name);
        let dist = root.join("dist").join(&group).join(&name);
        let p = Self {
            root: root.into(),
            id: id.into(),
            cfg,
            state,
            version,
            group,
            name,
            assets,
            dist,
        };
        let grid = p.usize("GRID", 0);
        let border = p.usize("BORDER", 0);
        if !(8..=4096).contains(&grid)
            || !grid.is_multiple_of(8)
            || border.checked_mul(2).is_none_or(|b| b >= grid)
        {
            return Err(err(
                "GRID must be a multiple of 8, <=4096, larger than twice BORDER",
            ));
        }
        if p.number("Z_SCALE", 0.) <= 0. || p.number("CELL", 0.) <= 0. {
            return Err(err("Z_SCALE and CELL must be positive"));
        }
        Ok(p)
    }
    pub fn source(&self) -> PathBuf {
        self.assets.join("source")
    }
    pub fn map_dir(&self) -> PathBuf {
        self.root.join("maps").join(&self.id)
    }
    pub fn path(&self, name: &str) -> Result<PathBuf> {
        let path = Path::new(name);
        if path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::Prefix(_) | Component::RootDir
                )
            })
        {
            return Err(err(
                "recipe paths must be relative and remain inside project",
            ));
        }
        Ok(self.map_dir().join(path))
    }
    pub fn string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.cfg[key].as_str().unwrap_or(default)
    }
    pub fn number(&self, key: &str, default: f64) -> f64 {
        self.cfg[key].as_f64().unwrap_or(default)
    }
    pub fn usize(&self, key: &str, default: usize) -> usize {
        self.cfg[key]
            .as_u64()
            .and_then(|v| v.try_into().ok())
            .unwrap_or(default)
    }
    pub fn flag(&self, key: &str) -> bool {
        self.cfg[key].as_bool().unwrap_or(false)
    }
    pub fn terrain_number(&self, key: &str, default: f64) -> f64 {
        self.cfg["TERRAIN"][key].as_f64().unwrap_or(default)
    }
    pub fn terrain_flag(&self, key: &str) -> bool {
        self.cfg["TERRAIN"][key].as_bool().unwrap_or(false)
    }
    pub fn render(&self, text: &str) -> String {
        text.replace("{version}", &self.version)
            .replace("{name}", &self.name)
    }
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(self.source())?;
        Ok(())
    }
}
pub fn version_path(root: &Path, id: &str) -> PathBuf {
    root.join("maps").join(id).join("version.json")
}
pub fn load_version(root: &Path, id: &str) -> Result<Value> {
    let p = version_path(root, id);
    if p.exists() {
        let st = serde_json::from_slice(&fs::read(p)?)?;
        format_version(&st)?;
        Ok(st)
    } else {
        Ok(serde_json::json!({"stage":"T","major":0,"minor":0,"patch":1,"history":[]}))
    }
}
pub fn format_version(st: &Value) -> Result<String> {
    let stage = st["stage"]
        .as_str()
        .filter(|s| *s == "T" || *s == "V")
        .ok_or_else(|| err("version stage must be T or V"))?;
    let values = ["major", "minor", "patch"].map(|k| {
        st[k]
            .as_u64()
            .ok_or_else(|| err(format!("invalid version field {k}")))
    });
    Ok(format!(
        "{}{}.{}.{}",
        stage,
        values[0].as_ref().map_err(|e| err(e.to_string()))?,
        values[1].as_ref().map_err(|e| err(e.to_string()))?,
        values[2].as_ref().map_err(|e| err(e.to_string()))?
    ))
}
pub fn parse_version(s: &str) -> Result<Value> {
    let s = s.trim().to_uppercase();
    let mut c = s.chars();
    let stage = c.next().ok_or_else(|| err("empty version"))?;
    if stage != 'T' && stage != 'V' {
        return Err(err("version must start with T or V"));
    }
    let n = c
        .as_str()
        .split('.')
        .map(|v| v.parse::<u64>().map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;
    if n.len() != 3 {
        return Err(err("expected T0.0.1 or V1.0.0"));
    }
    Ok(serde_json::json!({"stage":stage.to_string(),"major":n[0],"minor":n[1],"patch":n[2]}))
}
pub fn bump(st: &Value, release: bool, step: u64, note: &str) -> Result<Value> {
    format_version(st)?;
    let mut out = st.clone();
    if release && st["stage"] != "V" {
        out["stage"] = "V".into();
        out["major"] = 1.into();
        out["minor"] = 0.into();
        out["patch"] = 0.into();
    }
    let total = out["major"]
        .as_u64()
        .unwrap()
        .checked_mul(100)
        .and_then(|v| v.checked_add(out["minor"].as_u64()?.checked_mul(10)?))
        .and_then(|v| v.checked_add(out["patch"].as_u64()?))
        .and_then(|v| v.checked_add(step))
        .ok_or_else(|| err("version overflow"))?;
    out["major"] = (total / 100).into();
    out["minor"] = (total / 10 % 10).into();
    out["patch"] = (total % 10).into();
    record(&mut out, note)?;
    Ok(out)
}
pub fn record(st: &mut Value, note: &str) -> Result<()> {
    let version = format_version(st)?;
    let date = Utc::now()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .date_naive()
        .to_string();
    if st.get("history").is_none() {
        st["history"] = serde_json::json!([]);
    }
    st["history"]
        .as_array_mut()
        .ok_or_else(|| err("version history must be an array"))?
        .push(serde_json::json!({"version":version,"date":date,"note":note}));
    Ok(())
}
pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| err("path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    let result = (|| {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
