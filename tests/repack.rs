use ra3maps::{
    archive,
    binary::Writer,
    build,
    map::MapFile,
    project::{self, Project},
    refpack,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ra3-repack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn setup() -> (Temp, Vec<(String, Vec<u8>)>) {
    let temp = Temp::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let map_dir = temp.0.join("maps/千壑横原");
    fs::create_dir_all(&map_dir).unwrap();
    for name in ["config.json", "version.json"] {
        fs::copy(root.join("maps/千壑横原").join(name), map_dir.join(name)).unwrap();
    }
    // These tests use the historical WorldBuilder fixture, independently of
    // later public naming migrations in the real recipe.
    let config = map_dir.join("config.json");
    let mut cfg: serde_json::Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    cfg["MAP_NAME"] = serde_json::json!("[CHAI]叠岭连原");
    cfg.as_object_mut().unwrap().remove("PUBLISH_NAME");
    cfg.as_object_mut().unwrap().remove("COORDINATE_PIPELINE");
    project::write_json(&config, &cfg).unwrap();
    // Fixed version: the test must remain independent of later real releases.
    project::write_json(
        &map_dir.join("version.json"),
        &project::parse_version("T0.5.1").unwrap(),
    )
    .unwrap();
    let input =
        fs::read(root.join("maps/千壑横原/repack/inputs/[CHAI]叠岭连原T0.5.1--rebuild.zip"))
            .unwrap();
    let files = archive::unzip(&input)
        .unwrap()
        .into_iter()
        .filter(|(n, _)| !n.ends_with('/'))
        .collect();
    (temp, files)
}
fn source_map(files: &[(String, Vec<u8>)]) -> MapFile {
    MapFile::load_bytes(&files.iter().find(|(n, _)| n.ends_with(".map")).unwrap().1).unwrap()
}

#[test]
fn repack_repairs_legacy_order_preserves_wb_resources_and_installs_complete_package() {
    let (temp, mut files) = setup();
    let source = source_map(&files);
    // Construct the legacy append-order fault independently of MapFile::encode.
    let mut w = Writer::new();
    w.bytes(b"CkMp");
    w.u32(source.order.len() as u32);
    for (s, id) in source.order.iter().rev() {
        w.varstr(s).unwrap();
        w.u32(*id);
    }
    for c in &source.chunks {
        c.write(&mut w).unwrap();
    }
    assert!(
        MapFile::decode(&w.data)
            .unwrap()
            .validate_release()
            .is_err()
    );
    files
        .iter_mut()
        .find(|(n, _)| n.ends_with(".map"))
        .unwrap()
        .1 = refpack::pack(&w.data).unwrap();
    let input = temp.0.join("legacy.zip");
    fs::write(&input, archive::zip(&files).unwrap()).unwrap();
    let profile = temp.0.join("profile");
    let old = profile.join("Red Alert 3/Maps/keep-old-map");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("sentinel"), b"keep").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args(["repack", "--root"])
        .arg(&temp.0)
        .args([
            "--map",
            "千壑横原",
            "--bump",
            "--install",
            "--note",
            "修复打包格式",
        ])
        .arg(&input)
        .env("APPDATA", &profile)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let p = Project::load(&temp.0, "千壑横原").unwrap();
    assert_eq!(p.version, "T0.5.2");
    let files = archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
    assert_eq!(files.len(), 6);
    let result = MapFile::load_bytes(&files[&format!("{}/{}.map", p.name, p.name)]).unwrap();
    result.validate_release().unwrap();
    assert_eq!(result.table, source.table);
    assert_eq!(result.chunks, source.chunks);
    for (name, bytes) in &files {
        let local = name.split_once('/').unwrap().1;
        assert_eq!(*bytes, fs::read(p.dist.join(local)).unwrap());
        assert_eq!(
            *bytes,
            fs::read(profile.join("Red Alert 3/Maps").join(&p.name).join(local)).unwrap()
        );
    }
    let xml = std::str::from_utf8(&files[&format!("{}/map.xml", p.name)]).unwrap();
    assert!(xml.contains("<TerrainTextureAtlas"));
    assert!(xml.contains("id=\"CHAIT0.5.1\""));
    assert!(xml.contains("t0.5.2.map"));
    assert_eq!(xml.matches("<GameScriptList").count(), 19);
    assert_eq!(fs::read(old.join("sentinel")).unwrap(), b"keep");
}

#[test]
fn broken_wb_resource_package_is_rejected_without_bumping_or_publishing() {
    let (temp, mut files) = setup();
    let mut map = source_map(&files);
    map.remove("AssetList");
    files
        .iter_mut()
        .find(|(n, _)| n.ends_with(".map"))
        .unwrap()
        .1 = refpack::pack(&map.encode().unwrap()).unwrap();
    let input = temp.0.join("broken.zip");
    fs::write(&input, archive::zip(&files).unwrap()).unwrap();
    let before = fs::read(project::version_path(&temp.0, "千壑横原")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args(["repack", "--root"])
        .arg(&temp.0)
        .args(["--map", "千壑横原", "--bump"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("AssetList"));
    assert_eq!(
        before,
        fs::read(project::version_path(&temp.0, "千壑横原")).unwrap()
    );
    assert!(
        !temp
            .0
            .join("dist/[CHAI]叠岭连原/[CHAI]叠岭连原T0.5.2.zip")
            .exists()
    );
}

#[test]
fn wb_resource_manifest_and_xml_cannot_be_published_separately() {
    let (_temp, files) = setup();
    let files: Vec<_> = files
        .into_iter()
        .filter(|(name, _)| !name.ends_with('/'))
        .map(|(name, bytes)| (name.rsplit('/').next().unwrap().to_owned(), bytes))
        .collect();
    ra3maps::release::validate_files(&files).unwrap();
    let without_xml: Vec<_> = files
        .iter()
        .filter(|(name, _)| name != "map.xml")
        .cloned()
        .collect();
    assert!(
        ra3maps::release::validate_files(&without_xml)
            .unwrap_err()
            .to_string()
            .contains("matching map.xml")
    );
    let without_overrides: Vec<_> = files
        .iter()
        .filter(|(name, _)| name != "overrides.xml")
        .cloned()
        .collect();
    assert!(
        ra3maps::release::validate_files(&without_overrides)
            .unwrap_err()
            .to_string()
            .contains("missing overrides.xml")
    );
    let mut empty = files.clone();
    let map_bytes = &mut empty
        .iter_mut()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap()
        .1;
    let mut map = MapFile::load_bytes(map_bytes).unwrap();
    map.get_mut("AssetList").unwrap().payload = 0_u32.to_le_bytes().to_vec();
    *map_bytes = refpack::pack(&map.encode().unwrap()).unwrap();
    assert!(
        ra3maps::release::validate_files(&empty)
            .unwrap_err()
            .to_string()
            .contains("AssetList is empty")
    );
}

#[test]
fn partially_removed_script_asset_declarations_are_rejected_even_with_decoy_comments() {
    let (_temp, files) = setup();
    let mut files: Vec<_> = files
        .into_iter()
        .filter(|(name, _)| !name.ends_with('/'))
        .map(|(name, bytes)| (name.rsplit('/').next().unwrap().to_owned(), bytes))
        .collect();
    let xml = &mut files
        .iter_mut()
        .find(|(name, _)| name == "map.xml")
        .unwrap()
        .1;
    let text = String::from_utf8(xml.clone()).unwrap();
    assert_eq!(text.matches("<GameScriptList").count(), 19);
    *xml = (text
        .replacen("<GameScriptList", "<GameScriptListReference", 1)
        .replacen("</GameScriptList>", "</GameScriptListReference>", 1)
        + "\n<!-- <GameScriptList id=\"unused\"/> -->\n")
        .into_bytes();
    let error = ra3maps::release::validate_files(&files)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("map has 19 lists, map.xml declares 18"),
        "{error}"
    );
}

#[test]
fn rebuilt_runtime_manifest_matches_every_byte_of_the_verified_worldbuilder_save() {
    let (_temp, files) = setup();
    let map = source_map(&files);
    assert_eq!(
        ra3maps::resources::manifest(&map).unwrap(),
        map.get("AssetList").unwrap().payload
    );
}

#[test]
fn resource_repair_restores_pairing_without_changing_gameplay_or_polished_terrain() {
    let (temp, files) = setup();
    let source = source_map(&files);
    let mut broken = source.clone();
    broken.remove("AssetList");
    let mut files: Vec<_> = files
        .into_iter()
        .filter(|(name, _)| !name.ends_with("/map.xml"))
        .collect();
    files
        .iter_mut()
        .find(|(name, _)| name.ends_with(".map"))
        .unwrap()
        .1 = refpack::pack(&broken.encode().unwrap()).unwrap();
    let input = temp.0.join("missing-resources.zip");
    fs::write(&input, archive::zip(&files).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args(["repair-resources", "--root"])
        .arg(&temp.0)
        .args(["--map", "千壑横原", "--bump"])
        .arg(input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let p = Project::load(&temp.0, "千壑横原").unwrap();
    let repaired = MapFile::load(p.dist.join(format!("{}.map", p.name))).unwrap();
    assert_eq!(source.table, repaired.table);
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        assert_eq!(repaired.get(name).unwrap(), chunk, "{name}");
    }
    let xml = fs::read_to_string(p.dist.join("map.xml")).unwrap();
    assert_eq!(xml.matches("<GameScriptList ").count(), 19);
    let hm = ra3maps::map::HeightMap::parse(repaired.get("HeightMapData").unwrap()).unwrap();
    let bt = ra3maps::map::BlendTiles::parse(
        repaired.get("BlendTileData").unwrap(),
        hm.width,
        hm.height,
    )
    .unwrap();
    for name in &bt.textures {
        assert!(xml.contains(&format!(
            "TextureID=\"{}\"",
            ra3maps::resources::fast_hash(name)
        )));
    }
    assert!(xml.contains(&format!("{}\\{}.map", p.name, p.name)));
    assert!(xml.contains("<Waypoint id=\"Player_1_Start\""));
    let expected = archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
    assert_eq!(expected.len(), 6);
}

#[test]
fn polishing_finished_wb_map_only_changes_terrain_and_keeps_its_complete_package() {
    let (temp, files) = setup();
    let source = source_map(&files);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["layout.json", "v11_layers.npz"] {
        fs::copy(
            root.join("maps/千壑横原").join(name),
            temp.0.join("maps/千壑横原").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(temp.0.join("templates/rules")).unwrap();
    fs::copy(
        root.join("templates/rules/common-T0.5.1.json"),
        temp.0.join("templates/rules/common-T0.5.1.json"),
    )
    .unwrap();
    fs::copy(
        root.join("templates/rules/千壑横原.json"),
        temp.0.join("templates/rules/千壑横原.json"),
    )
    .unwrap();
    let input = temp.0.join("finished.zip");
    fs::write(&input, archive::zip(&files).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args(["polish-ramps", "--root"])
        .arg(&temp.0)
        .args(["--map", "千壑横原", "--bump"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let p = Project::load(&temp.0, "千壑横原").unwrap();
    let result = MapFile::load(p.dist.join(format!("{}.map", p.name))).unwrap();
    assert_eq!(result.table, source.table);
    let mut changed = Vec::new();
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if result.get(name).unwrap() != chunk {
            changed.push(name.as_str());
        }
    }
    assert_eq!(changed, ["HeightMapData", "BlendTileData"]);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(p.assets.join("ramp-report.json")).unwrap()).unwrap();
    assert!(report["optimized"].as_u64().unwrap() > 0);
    assert_eq!(report["protected_footprints_unchanged"], true);
    assert_eq!(
        report["existing_land_and_sea_resource_connections_preserved"],
        true
    );
    let terrain = ra3maps::terrain::Terrain::load(&p).unwrap();
    let hm = ra3maps::map::HeightMap::parse(result.get("HeightMapData").unwrap()).unwrap();
    assert_eq!(
        hm.elevations,
        ra3maps::terrain::flip(&terrain.raw, terrain.n)
    );
    let blend = result.get("BlendTileData").unwrap();
    let bt = ra3maps::map::BlendTiles::parse(blend, terrain.n, terrain.n).unwrap();
    let old_bt =
        ra3maps::map::BlendTiles::parse(source.get("BlendTileData").unwrap(), terrain.n, terrain.n)
            .unwrap();
    assert_eq!(bt.textures, old_bt.textures);
    assert!(bt.blend_descriptions.len() > old_bt.blend_descriptions.len());
    let area = terrain.n * terrain.n;
    let mut reader = ra3maps::binary::Reader::new(&blend.payload[4 + 2 * area..4 + 6 * area]);
    for _ in 0..2 * area {
        assert!(reader.u16().unwrap() as usize <= bt.blend_descriptions.len() / 18);
    }
    for description in bt.blend_descriptions.as_chunks::<18>().0 {
        assert!(
            (u32::from_le_bytes(description[..4].try_into().unwrap()) as usize)
                < bt.textures.len() * 64
        );
    }
    let zipped = archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
    assert_eq!(zipped.len(), 6);
    let xml = fs::read_to_string(p.dist.join("map.xml")).unwrap();
    assert!(xml.contains("id=\"CHAIT0.5.1\""));
    assert!(xml.contains("t0.5.2.map"));
    assert!(p.assets.join("preview--layout.png").is_file());
    assert!(p.assets.join("ramps--comparison.png").is_file());
}

#[test]
fn texture_polish_is_deterministic_and_preserves_all_gameplay_and_resource_data() {
    let (temp, files) = setup();
    let source = source_map(&files);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::copy(
        root.join("maps/千壑横原/layout.json"),
        temp.0.join("maps/千壑横原/layout.json"),
    )
    .unwrap();
    fs::create_dir_all(temp.0.join("templates/rules")).unwrap();
    fs::copy(
        root.join("templates/rules/common-T0.5.1.json"),
        temp.0.join("templates/rules/common-T0.5.1.json"),
    )
    .unwrap();
    fs::copy(
        root.join("templates/rules/千壑横原.json"),
        temp.0.join("templates/rules/千壑横原.json"),
    )
    .unwrap();
    let original = Project::load(&temp.0, "千壑横原").unwrap();
    let mut first = source.clone();
    let mut second = source.clone();
    let audit =
        ra3maps::texture_polish::apply(&original, &mut first, "[CHAI]叠岭连原T0.5.1").unwrap();
    ra3maps::texture_polish::apply(&original, &mut second, "[CHAI]叠岭连原T0.5.1").unwrap();
    assert_eq!(first.encode().unwrap(), second.encode().unwrap());
    assert!(audit.report["changed_texture_cells"].as_u64().unwrap() > 0);
    // These stock "grass" images contain large, directional dirt shapes.
    // A legal atlas ID alone does not make them suitable for area fills.
    for name in [
        "Grass_CapeCod01",
        "Grass_CapeCod03",
        "Grass_CapeCod07",
        "Grass_Golf01",
        "Grass_Golf03",
        "Grass_Golf05",
    ] {
        assert!(
            audit.report["after"].get(name).is_none(),
            "repeated transition texture {name}"
        );
    }
    let input = temp.0.join("finished.zip");
    fs::write(&input, archive::zip(&files).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args(["polish-textures", "--root"])
        .arg(&temp.0)
        .args(["--map", "千壑横原", "--bump"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let p = Project::load(&temp.0, "千壑横原").unwrap();
    let result = MapFile::load(p.dist.join(format!("{}.map", p.name))).unwrap();
    assert_eq!(source.table, result.table);
    for chunk in &source.chunks {
        let name = &source.table[&chunk.name_idx];
        if name != "BlendTileData" {
            assert_eq!(result.get(name).unwrap(), chunk, "{name}");
        }
    }
    let hm = ra3maps::map::HeightMap::parse(source.get("HeightMapData").unwrap()).unwrap();
    let n = hm.width;
    let old = source.get("BlendTileData").unwrap();
    let new = result.get("BlendTileData").unwrap();
    let bitmaps = 4 + 8 * n * n..4 + 8 * n * n + 8 * n * n.div_ceil(8) + n * n;
    assert_eq!(old.payload[bitmaps.clone()], new.payload[bitmaps]);
    let old_bt = ra3maps::map::BlendTiles::parse(old, n, n).unwrap();
    let new_bt = ra3maps::map::BlendTiles::parse(new, n, n).unwrap();
    assert_eq!(old_bt.textures, new_bt.textures);
    assert_ne!(old_bt.tiles, new_bt.tiles);
    let descriptions = new_bt.blend_descriptions.len() / 18;
    let mut nonzero = 0;
    for bytes in new.payload[4 + 2 * n * n..4 + 8 * n * n].as_chunks::<2>().0 {
        let id = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        assert!(id <= descriptions);
        nonzero += usize::from(id > 0);
    }
    assert!(nonzero > 0);
    for row in new_bt.blend_descriptions.as_chunks::<18>().0 {
        assert!(
            (u32::from_le_bytes(row[..4].try_into().unwrap()) as usize)
                < new_bt.textures.len() * 64
        );
    }
    assert!(p.assets.join("textures--comparison.png").is_file());
    assert!(p.assets.join("texture-report.json").is_file());
    assert!(p.assets.join("source/layout.json").is_file());
    let packaged = archive::unzip(&fs::read(build::zip_path(&p)).unwrap()).unwrap();
    assert_eq!(packaged.len(), 6);
    for name in ["overrides.xml", "map.str", "NewWorldBuilder.notes.json"] {
        let before = &files.iter().find(|(n, _)| n.ends_with(name)).unwrap().1;
        assert_eq!(&packaged[&format!("{}/{name}", p.name)], before);
    }
}
