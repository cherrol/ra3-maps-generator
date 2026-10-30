//! One reader for CRC-bound, flat native authoring packages.
use crate::{
    Result, err,
    formats::archive,
    pipeline::project::{self, Project},
    terrain::Layout,
};
use std::{collections::BTreeMap, fs};
type GameFiles = Vec<(String, Vec<u8>)>;

pub(crate) struct FrozenSource {
    name: String,
    entries: BTreeMap<String, Vec<u8>>,
}

impl FrozenSource {
    pub fn load(p: &Project) -> Result<Self> {
        let cfg = &p.cfg["COORDINATE_PIPELINE"];
        let path = cfg["source_package"]
            .as_str()
            .ok_or_else(|| err("coordinate source package is missing"))?;
        let name = cfg["source_name"]
            .as_str()
            .ok_or_else(|| err("coordinate source name is missing"))?;
        p.release_version(name)?;
        let crc32 = cfg["source_crc32"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| err("coordinate source CRC must be a uint32"))?;
        let entries = archive::unzip(&fs::read(p.path(path)?)?)?;
        for name in entries.keys() {
            project::safe_component(name)?;
        }
        let source = Self {
            name: name.into(),
            entries,
        };
        if crc32fast::hash(source.map_bytes()?) != crc32 {
            return Err(err(
                "coordinate source checksum differs (CRC); explicitly adopt the complete new source",
            ));
        }
        Ok(source)
    }

    fn map_bytes(&self) -> Result<&[u8]> {
        self.entries
            .get(&format!("{}.map", self.name))
            .map(Vec::as_slice)
            .ok_or_else(|| err("coordinate source map missing"))
    }

    pub fn layout(&self) -> Result<Layout> {
        Ok(serde_json::from_slice(
            self.entries
                .get("layout.json")
                .ok_or_else(|| err("coordinate source layout missing"))?,
        )?)
    }

    pub fn into_parts(self) -> Result<(GameFiles, Layout)> {
        let layout = self.layout()?;
        let files = self
            .entries
            .into_iter()
            .filter(|(name, _)| !["layout.json", "coordinate-id-map.json"].contains(&name.as_str()))
            .collect();
        Ok((files, layout))
    }
}
