//! Rebuild the complete runtime resource package, rather than dropping AssetList.
//! Hash and manifest ordering are checked against the verified WorldBuilder save.
mod atlas;
mod manifest;
pub mod official;
mod script_xml;
pub mod terrain_package;
pub mod xml_structure;
use crate::{
    Result, err,
    formats::binary::Reader,
    formats::map::{self, Chunk, MapFile, Value},
    gameplay::scripts,
    pipeline::project::Project,
};
use atlas::terrain_catalog;
pub use atlas::validate_terrain_package;
pub(crate) use atlas::{
    TerrainFinalization, finalize_terrain_package, sort_terrain_atlas, validate_terrain_atlas_order,
};
use manifest::{environment, objects, runtime_objects, text, world};
pub use manifest::{fast_hash, manifest, validate_manifest};
use script_xml::write_script_lists;
pub(crate) use script_xml::{player_names, refresh_script_xml};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn position(out: &mut String, p: [f32; 3]) {
    out.push_str(&format!(
        "<Position x=\"{}\" y=\"{}\" z=\"{}\"/>",
        p[0], p[1], p[2]
    ));
}
pub struct Package {
    pub xml: Vec<u8>,
    pub overrides: Vec<u8>,
}

/// Resource-only repairs keep the previous release's actual terrain/layout
/// snapshot, so the next incremental edit cannot fall back to older ramps.
pub fn write_repair_artifacts(p: &Project, source_name: &str, map: &MapFile) -> Result<()> {
    let source_dir = p.release_assets(source_name)?;
    if source_dir.join("source/terrain.npz").is_file()
        && source_dir.join("source/layout.json").is_file()
    {
        let mut source = p.clone();
        source.assets = source_dir.clone();
        let t = crate::terrain::Terrain::load(&source)?;
        let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
        if t.heightmap(p).elevations != hm.elevations {
            return Err(err(
                "source preview cache does not match repaired map terrain",
            ));
        }
        t.save(p)?;
        let bt = map::BlendTiles::parse(map.get("BlendTileData")?, t.n, t.n)?;
        let water = p.number("WATER_Z", 200.);
        let surface = crate::terrain::surface::Surface {
            chosen: crate::terrain::flip(&bt.tiles, t.n)
                .iter()
                .map(|v| v / 64)
                .collect(),
            impassable: crate::terrain::flip(&bt.impassability, t.n),
            build_block: crate::terrain::flip(&bt.buildability, t.n),
            layer_edge: t.slope.iter().map(|v| *v > 20.).collect(),
            shore: (0..t.n * t.n)
                .map(|i| {
                    t.z[i] >= water
                        && crate::terrain::grid::neighbors(i, t.n).any(|k| t.z[k] < water)
                })
                .collect(),
            textures: Vec::new(),
        };
        crate::render::previews(&t, p, &surface)?;
        let path = p.assets.join("layout-index.json");
        let mut index: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        index["layout"] = serde_json::json!("source/layout.json");
        index["source_map"] = serde_json::json!(format!("{}.map", p.name));
        crate::pipeline::project::write_json(&path, &index)?;
        for name in ["ramp-report.json", "ramps--comparison.png"] {
            if source_dir.join(name).is_file() {
                fs::copy(source_dir.join(name), p.assets.join(name))?;
            }
        }
    }
    fs::write(
        p.assets.join("README.md"),
        format!(
            "# {}\n\n基于 {} 修复完整资源封装。高度、通行与建造位图、斜坡纹理、对象及脚本保持原样，重新生成资源清单、纹理图集、脚本资产与地图 XML。\n\n[编号预览](preview--layout.png) · [编号索引](layout-index.json) · [发布核对](release-report.json)\n\n若附有斜坡报告和对比图，它们沿用上一版优化记录。本次记录覆盖原生封装和静态验证。\n",
            p.name, source_name
        ),
    )?;
    Ok(())
}

pub fn generate(map: &mut MapFile, p: &Project) -> Result<Package> {
    let hm = map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let props = world(map)?;
    let objects = objects(map)?;
    let env = environment(map)?;
    let mut sources: BTreeMap<String, Option<String>> = serde_json::from_str(include_str!(
        "../../templates/resources/stock-object-sources.json"
    ))?;
    if p.cfg
        .get("OBJECT_RESOURCE_SOURCES")
        .is_some_and(|value| !value.is_null() && !value.is_object())
    {
        return Err(err("OBJECT_RESOURCE_SOURCES must be an object"));
    }
    if let Some(extra) = p.cfg["OBJECT_RESOURCE_SOURCES"].as_object() {
        for (k, v) in extra {
            if !v.is_null() && !v.is_string() {
                return Err(err(format!(
                    "OBJECT_RESOURCE_SOURCES.{k} must be a source string or null"
                )));
            }
            sources.insert(k.clone(), v.as_str().map(str::to_owned));
        }
    }
    let sources: BTreeMap<_, _> = sources
        .into_iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v))
        .collect();
    let mut includes = BTreeSet::new();
    for name in runtime_objects(map)? {
        let source = sources
            .get(&name)
            .cloned()
            .or_else(|| {
                official::object(&name)
                    .and_then(|definition| definition.source.strip_prefix("Xml/"))
                    .map(|path| Some(format!("DATA:{path}")))
            })
            .ok_or_else(|| {
                err(format!(
                    "no registered resource source for {}; declare OBJECT_RESOURCE_SOURCES",
                    name
                ))
            })?;
        if let Some(path) = source {
            includes.insert(path);
        }
    }
    let resource_seed = match p.cfg.get("RESOURCE_ID_SEED") {
        Some(value) => value
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| err("RESOURCE_ID_SEED must be a uint32"))?,
        None => crc32fast::hash(p.id.as_bytes()),
    };
    let id = format!("ra3maps_{:08x}_{}", resource_seed, p.version);
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<AssetDeclaration xmlns=\"uri:ea.com:eala:asset\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><Tags/><Includes>",
    );
    for path in ["DATA:static.xml", "DATA:global.xml", "DATA:audio.xml"] {
        xml.push_str(&format!("<Include source=\"{path}\" type=\"reference\"/>"));
    }
    let mut textures = BTreeSet::new();
    for name in [&env.environment, &env.cloud, &env.macro_tex] {
        if !name.is_empty() {
            textures.insert(name);
        }
    }
    for name in textures {
        xml.push_str(&format!(
            "<Include source=\"ART:{}.xml\" type=\"instance\"/>",
            escape(name)
        ));
    }
    xml.push_str("<Include source=\"DATA:GlobalData/roads.xml\" type=\"instance\"/>");
    for path in includes {
        xml.push_str(&format!(
            "<Include source=\"{}\" type=\"instance\"/>",
            escape(&path)
        ));
    }
    xml.push_str("<Include source=\"DATA:sounds/musicscript_unknownzone.xml\" type=\"instance\"/><Include source=\"overrides.xml\" type=\"all\"/></Includes>");
    xml.push_str(&format!("<GameMap id=\"{id}\">"));
    let mut roads = BTreeSet::new();
    for o in &objects {
        if o.road_type != 0 {
            if roads.insert(o.type_name.clone()) {
                xml.push_str(&format!("<Road id=\"{}\"/>", escape(&o.type_name)));
            }
            continue;
        }
        if o.type_name.contains("Waypoint") {
            let name = text(&o.props, "waypointName", "");
            if !name.is_empty() {
                xml.push_str(&format!("<Waypoint id=\"{}\">", escape(name)));
                position(&mut xml, o.position);
                xml.push_str("</Waypoint>");
            }
            continue;
        }
        let tag = if o.type_name.contains("Tree")
            || o.type_name.contains("Bush")
            || o.type_name.contains("Rock")
            || o.type_name.contains("Collapse")
        {
            "Prop"
        } else {
            "Structure"
        };
        let owner = text(&o.props, "originalOwner", "PlyrNeutral/teamPlyrNeutral");
        let team = owner.split('/').next().unwrap_or("PlyrNeutral");
        let name = text(&o.props, "objectName", "");
        let name = if name.is_empty() { &o.type_name } else { name };
        let health = match map::prop(&o.props, "objectInitialHealth") {
            Some(Value::Int(v)) => *v,
            _ => 100,
        };
        xml.push_str(&format!("<{tag} Angle=\"{}\" EventList=\"\" Faction=\"{}\" Health=\"{health}\" Team=\"{}\" ThingTemplate=\"{}\" id=\"{}\">",o.angle,escape(owner),escape(team),escape(&o.type_name),escape(name)));
        position(&mut xml, o.position);
        xml.push_str(&format!("</{tag}>"));
    }
    let starts: Vec<_> = objects
        .iter()
        .filter(|o| {
            text(&o.props, "waypointName", "")
                .strip_prefix("Player_")
                .and_then(|s| s.strip_suffix("_Start"))
                .is_some_and(|n| n.parse::<usize>().is_ok())
        })
        .collect();
    let multiplayer =
        matches!(map::prop(&props,"isScenarioMultiplayer"),Some(Value::Bool(v)) if *v!=0);
    xml.push_str(&format!("<MapMetaData BorderSize=\"{}\" CRC=\"0\" Description=\"{}\" DisplayName=\"{}\" FileName=\"data\\maps\\{}\\{}.map\" Height=\"{}\" Width=\"{}\" IsMultiplayer=\"{multiplayer}\" IsOfficial=\"false\" NumPlayers=\"{}\">",hm.border,escape(p.string("MAP_DESCRIPTION", &p.group)),escape(&p.name),escape(&p.name),escape(&p.name),hm.height,hm.width,starts.len()));
    xml.push_str("<StartPosition Name=\"InitialCameraPosition\"><Position x=\"0\" y=\"0\" z=\"0\"/></StartPosition>");
    for o in &objects {
        let name = text(&o.props, "waypointName", "");
        if starts.iter().any(|start| std::ptr::eq(*start, o)) {
            xml.push_str(&format!("<StartPosition Name=\"{}\">", escape(name)));
            position(&mut xml, o.position);
            xml.push_str("</StartPosition>");
        }
    }
    xml.push_str(&format!(
        "</MapMetaData><EnvironmentData Cloud=\"{}\" Environment=\"{}\" Macro=\"{}\"/><WorldDict>",
        escape(&env.cloud),
        escape(&env.environment),
        escape(&env.macro_tex)
    ));
    for prop in &props {
        let (tag, value) = match &prop.value {
            Value::Bool(v) => ("BoolProperty", (*v != 0).to_string()),
            Value::Int(v) => ("IntProperty", v.to_string()),
            Value::Float(v) => ("RealProperty", v.to_string()),
            Value::Unknown(v) => ("AssetIdProperty", v.rsplit(':').next().unwrap_or(v).into()),
            Value::Ascii(v) | Value::Unicode(v) => ("StringProperty", v.clone()),
        };
        xml.push_str(&format!(
            "<{tag} Key=\"{}\" Value=\"{}\"/>",
            escape(&prop.key),
            escape(&value)
        ));
    }
    let water_areas = Reader::new(&map.get("StandingWaterAreas")?.payload).u32()?;
    let garrisons = objects
        .iter()
        .filter(|o| {
            o.type_name.starts_with("ST_")
                || ["EI_Structure_01", "EI_Structure_02"].contains(&o.type_name.as_str())
        })
        .count();
    let tech = objects
        .iter()
        .filter(|o| {
            o.type_name == "OilDerrick"
                || (o.type_name.contains("TechStructure") && !o.type_name.contains("Collapse"))
        })
        .count();
    xml.push_str(&format!("</WorldDict><MapAnalysis CellsExamined=\"{}\" GarrisonableStructures=\"{garrisons}\" TechStructures=\"{tech}\" WaterAreas=\"{water_areas}\" WaterCells=\"{}\">",hm.width*hm.height,if water_areas>0{(hm.width-2*hm.border as usize)*(hm.height-2*hm.border as usize)}else{0}));
    for _ in 0..water_areas {
        xml.push_str("<WaterAreaType>EXTERIOR</WaterAreaType>");
    }
    xml.push_str("</MapAnalysis></GameMap>");
    let catalog = terrain_catalog(p)?;
    let mut tiles = Vec::new();
    if bt.textures.len() > 64 {
        return Err(err(
            "2048 terrain atlas supports at most 64 stock texture families",
        ));
    }
    for name in &bt.textures {
        let row = catalog.get(name).ok_or_else(|| {
            err(format!(
                "terrain atlas lacks resource definition for {name}"
            ))
        })?;
        tiles.push((fast_hash(name), &row[1], &row[2]));
    }
    tiles.sort_by_key(|r| r.0);
    xml.push_str(&format!(
        "<TerrainTextureAtlas AllowLossyCompression=\"true\" AtlasSize=\"2048\" id=\"{id}\">"
    ));
    for (texture_id, diffuse, normal) in tiles {
        xml.push_str(&format!("<Tile BaseTexture=\"ART:Terrain/{}.tga\" NormalTexture=\"ART:Terrain/{}.tga\" TextureID=\"{texture_id}\"/>",escape(diffuse),escape(normal)));
    }
    xml.push_str("</TerrainTextureAtlas>");
    let players = player_names(map)?;
    let nodes = scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?;
    if players.len() != nodes.len() {
        return Err(err(
            "player/script list count differs during resource export",
        ));
    }
    write_script_lists(&mut xml, &players, &nodes)?;
    xml.push_str("</AssetDeclaration>\n");
    let xml = xml_structure::canonicalize_generated(&xml)?;
    let payload = manifest(map)?;
    let name_idx = map.id("AssetList")?;
    let chunk = Chunk {
        name_idx,
        version: 1,
        payload,
    };
    if let Ok(c) = map.get_mut("AssetList") {
        *c = chunk;
    } else {
        map.chunks.insert(0, chunk);
    }
    Ok(Package{xml:xml.into_bytes(),overrides:b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><AssetDeclaration xmlns=\"uri:ea.com:eala:asset\"><Tags/><Includes/></AssetDeclaration>\n".to_vec()})
}

#[cfg(test)]
mod tests {
    #[test]
    fn common_terrain_finalization_is_atomic_and_repeatable() {
        use std::path::Path;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = Project::load(root, "河南").unwrap();
        let entries = crate::formats::archive::unzip(
            &fs::read(p.path("authoring/source.zip").unwrap()).unwrap(),
        )
        .unwrap();
        let mut m =
            MapFile::load_bytes(entries.iter().find(|(n, _)| n.ends_with(".map")).unwrap().1)
                .unwrap();
        let xml =
            std::str::from_utf8(entries.iter().find(|(n, _)| *n == "map.xml").unwrap().1).unwrap();
        let original = m.clone();
        let bad = xml.replace("</WorldDict>", "");
        assert!(
            finalize_terrain_package(
                &p,
                &mut m,
                &bad,
                "T0.0.9",
                TerrainFinalization::CompactLookups
            )
            .is_err()
        );
        assert!(m.chunks == original.chunks);
        assert_eq!(m.table, original.table);
        let result = finalize_terrain_package(
            &p,
            &mut m,
            xml,
            "T0.0.9",
            TerrainFinalization::CompactLookups,
        )
        .unwrap();
        validate_terrain_package(&m, &result.xml, Some("T0.0.9")).unwrap();
        let extra_tile =
            "<Tile BaseTexture=\"unused\" NormalTexture=\"unused\" TextureID=\"4294967295\"/>";
        let with_unused = result.xml.replace(
            "</TerrainTextureAtlas>",
            &format!("{extra_tile}</TerrainTextureAtlas>"),
        );
        let pruned = finalize_terrain_package(
            &p,
            &mut m,
            &with_unused,
            "T0.0.9",
            TerrainFinalization::CompactLookups,
        )
        .unwrap();
        assert_eq!(pruned.xml, result.xml);
        let once = m.clone();
        let again = finalize_terrain_package(
            &p,
            &mut m,
            &result.xml,
            "T0.0.9",
            TerrainFinalization::CompactLookups,
        )
        .unwrap();
        assert!(!again.compaction.unwrap().changed);
        assert_eq!(again.xml, result.xml);
        assert!(m.chunks == once.chunks);
        assert_eq!(m.table, original.table);
        for c in &original.chunks {
            let name = &original.table[&c.name_idx];
            if !["BlendTileData", "WorldInfo"].contains(&name.as_str()) {
                assert!(m.get(name).unwrap() == c, "{name}");
            }
        }
    }

    #[test]
    fn atlas_sort_preserves_declarations_ids_comments_and_unsigned_order() {
        let xml = "before<TerrainTextureAtlas id='keep'><Tile BaseTexture='x' TextureID='4035140248'/>\n<!--keep--><Tile TextureID=\"1421329569\" BaseTexture='y'/><Tile TextureID='63961087'/></TerrainTextureAtlas>after";
        assert!(super::validate_terrain_atlas_order(xml).is_err());
        let sorted = super::sort_terrain_atlas(xml).unwrap();
        assert_eq!(
            sorted,
            "before<TerrainTextureAtlas id='keep'><Tile TextureID='63961087'/>\n<!--keep--><Tile TextureID=\"1421329569\" BaseTexture='y'/><Tile BaseTexture='x' TextureID='4035140248'/></TerrainTextureAtlas>after"
        );
        super::validate_terrain_atlas_order(&sorted).unwrap();
        assert_eq!(super::sort_terrain_atlas(&sorted).unwrap(), sorted);
        assert!(super::sort_terrain_atlas("<TerrainTextureAtlas id='a'><Tile TextureID='1'/><Tile TextureID='1'/></TerrainTextureAtlas>").is_err());
        assert!(
            super::sort_terrain_atlas(
                "<TerrainTextureAtlas id='a'><Tile TextureID='bad'/></TerrainTextureAtlas>"
            )
            .is_err()
        );
    }
    use super::*;
    #[test]
    fn hash_matches_installed_library_and_verified_terrain_ids() {
        for (name, hash) in [
            ("TSCloudMed", 0x8387f17c),
            ("EVDefault", 0xa5dc3710),
            ("MusicPalette_NotSet", 0xb5b5cf19),
            ("CC_Tree01", 0x66121f89),
            ("OilDerrick", 0xa3e28bbe),
            ("StPetersburgSidewalk03", 0xd6de96f2),
            ("Gravel_Yucatan01", 63961087),
            ("Grass_NewYork01", 3344070337),
            ("Dirt_Yucatan06", 272543819),
        ] {
            assert_eq!(fast_hash(name), hash);
            assert_eq!(fast_hash(&name.to_ascii_uppercase()), hash);
        }
    }
}
