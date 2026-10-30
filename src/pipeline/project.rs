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
pub(crate) fn validate_current_rules(cfg: &Value) -> Result<()> {
    for key in ["SPAWN_PLACEMENT_REFERENCE", "RAMP_CLEARANCE_REFERENCE"] {
        if cfg.get(key).is_some() {
            return Err(err(format!(
                "{key} is no longer supported; use the current strict rules"
            )));
        }
    }
    for key in ["RAMP_PROFILE", "TEXTURE_PROFILE"] {
        if cfg[key].as_str() == Some("legacy") {
            return Err(err(format!("{key}=legacy is no longer supported")));
        }
    }
    Ok(())
}

impl Project {
    pub fn load(root: &Path, id: &str) -> Result<Self> {
        safe_component(id)?;
        let cfg: Value =
            serde_json::from_slice(&fs::read(root.join("maps").join(id).join("config.json"))?)?;
        validate_current_rules(&cfg)?;
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
        let assets = root.join("assets").join(id).join(&name);
        let dist = root.join("dist").join(id).join(&name);
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
        let height = p.usize("GRID_HEIGHT", grid);
        if !(8..=4096).contains(&grid)
            || !grid.is_multiple_of(8)
            || !(8..=4096).contains(&height)
            || !height.is_multiple_of(8)
            || border
                .checked_mul(2)
                .is_none_or(|b| b >= grid || b >= height)
        {
            return Err(err(
                "GRID and GRID_HEIGHT must be multiples of 8, <=4096, larger than twice BORDER",
            ));
        }
        if p.number("Z_SCALE", 0.) <= 0. || p.number("CELL", 0.) <= 0. {
            return Err(err("Z_SCALE and CELL must be positive"));
        }
        Ok(p)
    }
    pub fn publish_prefix(&self) -> &str {
        self.cfg["PUBLISH_NAME"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.group)
    }
    pub fn release_name(&self, version: &str) -> Result<String> {
        canonical_version(version)?;
        let name = format!("{}{version}", self.publish_prefix());
        safe_component(&name)?;
        Ok(name)
    }
    /// Accept the current publication prefix or the recipe's earlier title.
    pub fn release_version<'a>(&self, source_name: &'a str) -> Result<&'a str> {
        safe_component(source_name)?;
        for prefix in [self.publish_prefix(), self.group.as_str()] {
            if let Some(version) = source_name.strip_prefix(prefix)
                && canonical_version(version).is_ok()
            {
                return Ok(version);
            }
        }
        Err(err(
            "release name does not match the selected recipe or its version is not canonical",
        ))
    }
    pub fn release_assets(&self, source_name: &str) -> Result<PathBuf> {
        safe_component(&self.id)?;
        self.release_version(source_name)?;
        Ok(self.root.join("assets").join(&self.id).join(source_name))
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
fn canonical_version(version: &str) -> Result<()> {
    if format_version(&parse_version(version)?)? != version {
        return Err(err(
            "release version must use canonical T0.0.1 or V1.0.0 notation",
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(publish: Option<&str>) -> (Fixture, Project) {
        let root = Fixture(std::env::temp_dir().join(format!(
            "ra3maps-project-paths-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        let recipe = root.0.join("maps/陕西");
        fs::create_dir_all(&recipe).unwrap();
        write_json(
            &recipe.join("config.json"),
            &serde_json::json!({
                "SCHEMA_VERSION": 1, "MAP_NAME": "[CHAI]千壑横原",
                "PUBLISH_NAME": publish, "GRID": 8, "BORDER": 0,
                "Z_SCALE": 0.0390625, "CELL": 10
            }),
        )
        .unwrap();
        write_json(
            &recipe.join("version.json"),
            &parse_version("T0.7.8").unwrap(),
        )
        .unwrap();
        let p = Project::load(&root.0, "陕西").unwrap();
        (root, p)
    }

    #[test]
    fn storage_groups_by_terrain_id_and_keeps_the_complete_release_name() {
        let (_root, p) = fixture(None);
        assert_eq!(p.id, "陕西");
        assert_eq!(p.group, "[CHAI]千壑横原");
        assert_eq!(p.name, "[CHAI]千壑横原T0.7.8");
        assert_eq!(p.map_dir(), p.root.join("maps/陕西"));
        assert_eq!(p.assets, p.root.join("assets/陕西/[CHAI]千壑横原T0.7.8"));
        assert_eq!(p.dist, p.root.join("dist/陕西/[CHAI]千壑横原T0.7.8"));
        assert_eq!(
            crate::pipeline::build::zip_path(&p),
            p.root.join("dist/陕西/[CHAI]千壑横原T0.7.8.zip")
        );
        assert_eq!(
            p.release_assets("[CHAI]千壑横原T0.7.6").unwrap(),
            p.root.join("assets/陕西/[CHAI]千壑横原T0.7.6")
        );
        assert_eq!(
            crate::pipeline::build::game_files(&p)[0],
            "[CHAI]千壑横原T0.7.8.map"
        );
    }

    #[test]
    fn publish_override_accepts_the_same_recipes_earlier_prefix() {
        let (_root, p) = fixture(Some("[TEST]发布标题"));
        assert_eq!(p.group, "[CHAI]千壑横原");
        assert_eq!(p.publish_prefix(), "[TEST]发布标题");
        assert_eq!(p.name, "[TEST]发布标题T0.7.8");
        assert_eq!(p.release_name("V1.0.0").unwrap(), "[TEST]发布标题V1.0.0");
        for name in ["[CHAI]千壑横原T0.7.6", "[TEST]发布标题T0.7.6"] {
            assert_eq!(p.release_version(name).unwrap(), "T0.7.6");
            assert_eq!(
                p.release_assets(name).unwrap(),
                p.root.join("assets/陕西").join(name)
            );
        }
        assert_eq!(p.assets, p.root.join("assets/陕西/[TEST]发布标题T0.7.8"));
    }

    #[test]
    fn release_paths_reject_foreign_prefixes_and_noncanonical_or_unsafe_versions() {
        let (_root, p) = fixture(Some("[TEST]发布标题"));
        for name in [
            "[CHAI]碧嶂回澜T0.7.6",
            "T0.7.6",
            "[TEST]发布标题t0.7.6",
            "[TEST]发布标题T00.7.6",
            "[TEST]发布标题T0.7.6 ",
            "[TEST]发布标题T0.7.6/../T0.7.8",
            "[TEST]发布标题T0.7.6\\cache",
            "../[TEST]发布标题T0.7.6",
        ] {
            assert!(p.release_version(name).is_err(), "{name}");
            assert!(p.release_assets(name).is_err(), "{name}");
        }
        for version in ["t0.7.6", "T00.7.6", " T0.7.6", "T0.7.6/../../other"] {
            assert!(p.release_name(version).is_err(), "{version}");
        }
        let mut cfg = p.cfg.clone();
        cfg["PUBLISH_NAME"] = "../foreign".into();
        write_json(&p.map_dir().join("config.json"), &cfg).unwrap();
        assert!(Project::load(&p.root, "陕西").is_err());
        assert!(Project::load(&p.root, "../陕西").is_err());
    }
}
