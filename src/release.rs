//! Repackage a finished map without regenerating terrain, objects or scripts.
use crate::{
    Result, archive, build, err,
    map::MapFile,
    project::{self, Project},
    refpack, scripts,
};
use std::{collections::BTreeMap, fs, path::Path};

pub fn validate_files(files: &[(String, Vec<u8>)]) -> Result<()> {
    if files.iter().filter(|(n, _)| n.ends_with(".map")).count() != 1 {
        return Err(err("release requires exactly one .map"));
    }
    let data = files
        .iter()
        .find(|(n, _)| n.ends_with(".map"))
        .ok_or_else(|| err("release has no .map"))?;
    let map = MapFile::load_bytes(&data.1)?;
    let name = data.0.strip_suffix(".map").unwrap();
    for required in [format!("{name}_art.tga"), "map.str".into()] {
        if !files.iter().any(|(n, _)| *n == required) {
            return Err(err(format!(
                "release missing required game file {required}"
            )));
        }
    }
    map.validate_release()?;
    crate::resources::validate_manifest(&map)?;
    scripts::validate(&map)?;
    scripts::validate_waypoints(&map)?;
    let hm = crate::map::HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = crate::map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    crate::map::objects(map.get("ObjectsList")?, &map.table)?;
    if bt
        .tiles
        .iter()
        .any(|&tile| tile as usize / 64 >= bt.textures.len())
    {
        return Err(err("release terrain references a missing texture"));
    }
    let assets = map.get("AssetList").ok();
    let xml = files.iter().find(|(n, _)| n == "map.xml");
    if assets.is_some() && xml.is_none() {
        return Err(err(
            "WorldBuilder AssetList requires its matching map.xml; keep the complete saved package",
        ));
    }
    if let Some((_, xml)) = xml {
        let xml = std::str::from_utf8(xml)?;
        let declarations = without_xml_comments(xml)?;
        if (declarations.contains("source=\"overrides.xml\"")
            || declarations.contains("source='overrides.xml'"))
            && !files.iter().any(|(n, _)| n == "overrides.xml")
        {
            return Err(err("map.xml includes missing overrides.xml"));
        }
        let atlas_count = count_xml_tags(&declarations, "TerrainTextureAtlas");
        if atlas_count > 0 && assets.is_none() {
            return Err(err(
                "WorldBuilder package has a texture atlas but no AssetList; use its complete saved package",
            ));
        }
        if xml.contains("Scripts are stored in the merged .map PlayerScriptsList.") {
            return Err(err(
                "WorldBuilder script asset declarations were removed; use its complete saved package",
            ));
        }
        if atlas_count > 0 && count_xml_tags(&declarations, "GameScriptList") == 0 {
            return Err(err(
                "WorldBuilder package is missing script asset declarations",
            ));
        }
        if let Some(assets) = assets {
            if assets.payload[..4] == 0_u32.to_le_bytes() {
                return Err(err("WorldBuilder AssetList is empty"));
            }
            if atlas_count != 1 {
                return Err(err(
                    "WorldBuilder package requires exactly one terrain texture atlas",
                ));
            }
            if count_xml_tags(&declarations, "Tile") != bt.textures.len() {
                return Err(err(
                    "terrain atlas tile count differs from the map texture table",
                ));
            }
            for name in &bt.textures {
                let id = crate::resources::fast_hash(name);
                if !declarations.contains(&format!("TextureID=\"{id}\""))
                    && !declarations.contains(&format!("TextureID='{id}'"))
                {
                    return Err(err(format!(
                        "terrain atlas is missing texture {name} (ID {id})"
                    )));
                }
            }
            let lists =
                scripts::parse_nodes(&map.get("PlayerScriptsList")?.payload, &map.table)?.len();
            let declared = count_xml_tags(&declarations, "GameScriptList");
            if declared != lists {
                return Err(err(format!(
                    "WorldBuilder script asset declarations differ: map has {lists} lists, map.xml declares {declared}"
                )));
            }
        }
    }
    Ok(())
}

// Count declarations in the exported XML, excluding comments and similarly
// named elements (GameScriptListReference is not a GameScriptList).
fn count_xml_tags(xml: &str, name: &str) -> usize {
    xml.split('<')
        .skip(1)
        .filter(|part| {
            let tag = part
                .split(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
                .next()
                .unwrap_or("");
            tag.rsplit(':').next() == Some(name)
        })
        .count()
}

fn without_xml_comments(mut xml: &str) -> Result<String> {
    let mut out = String::with_capacity(xml.len());
    while let Some(start) = xml.find("<!--") {
        out.push_str(&xml[..start]);
        let rest = &xml[start + 4..];
        let end = rest
            .find("-->")
            .ok_or_else(|| err("unterminated map.xml comment"))?;
        xml = &rest[end + 3..];
    }
    out.push_str(xml);
    Ok(out)
}

/// Retarget the file path only. Asset IDs belong to the preserved resource
/// manifest and must remain stable, including TerrainTextureAtlas's ID.
pub fn retarget_xml(xml: &str, old_version: &str, version: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(pos) = rest.find("FileName=\"") {
        let end_prefix = pos + "FileName=\"".len();
        out.push_str(&rest[..end_prefix]);
        rest = &rest[end_prefix..];
        let end = rest
            .find('"')
            .ok_or_else(|| err("unterminated XML FileName"))?;
        let value = &rest[..end];
        let old = old_version.to_ascii_lowercase();
        let lower = value.to_ascii_lowercase();
        let mut cursor = 0;
        while let Some(offset) = lower[cursor..].find(&old) {
            let start = cursor + offset;
            out.push_str(&value[cursor..start]);
            out.push_str(&version.to_ascii_lowercase());
            cursor = start + old.len();
        }
        out.push_str(&value[cursor..]);
        rest = &rest[end..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn run(root: &Path, id: &str, source: &Path, options: &build::Options) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Repack)
}

pub fn polish_ramps(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Ramps)
}
pub fn relocate_ramps(
    root: &Path,
    id: &str,
    source: &Path,
    plan: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::RampMoves(plan))
}

pub fn repair_resources(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Resources)
}

pub fn polish_textures(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Textures)
}

pub fn repair_production(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Production)
}

pub fn update_scripts(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Scripts)
}

#[derive(Clone, Copy)]
enum Edit<'a> {
    Repack,
    Ramps,
    RampMoves(&'a Path),
    Resources,
    Textures,
    Production,
    Scripts,
}

fn run_inner(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
    edit: Edit<'_>,
) -> Result<Project> {
    let polish = matches!(edit, Edit::Ramps | Edit::RampMoves(_));
    let relocate = matches!(edit, Edit::RampMoves(_));
    let repair = matches!(edit, Edit::Resources);
    let textures = matches!(edit, Edit::Textures);
    let production = matches!(edit, Edit::Production);
    let update_scripts = matches!(edit, Edit::Scripts);
    if !options.bump
        || options.release
        || options.no_zip
        || !options.only.is_empty()
        || !options.skip.is_empty()
    {
        return Err(err(
            "repack requires --bump and a ZIP source; stage/release/no-zip options do not apply",
        ));
    }
    let original = Project::load(root, id)?;
    let source_zip = fs::read(source)?;
    let unpacked = archive::unzip(&source_zip)?;
    let mut flat = BTreeMap::new();
    let mut folder = None;
    for (path, data) in unpacked {
        if path.ends_with('/') {
            continue;
        }
        let (parent, name) = path
            .split_once('/')
            .ok_or_else(|| err("map ZIP needs a map directory"))?;
        project::safe_component(parent)?;
        project::safe_component(name)?;
        if folder.as_ref().is_some_and(|f| f != parent) {
            return Err(err("ZIP contains multiple map directories"));
        }
        folder = Some(parent.to_owned());
        flat.insert(name.to_owned(), data);
    }
    let names = flat
        .keys()
        .filter(|n| n.ends_with(".map"))
        .cloned()
        .collect::<Vec<_>>();
    if names.len() != 1 {
        return Err(err("ZIP must contain exactly one .map"));
    }
    let source_name = names[0].strip_suffix(".map").unwrap();
    let source_version = source_name
        .strip_prefix(&original.group)
        .ok_or_else(|| err("ZIP map name does not match the selected recipe"))?;
    project::parse_version(source_version)?;
    let mut map = MapFile::load_bytes(&flat[&names[0]])?;
    let state = project::bump(&original.state, false, 1, &options.note)?;
    let version = project::format_version(&state)?;
    let name = format!("{}{version}", original.group);
    let attack_report = if update_scripts {
        let before = map.clone();
        let report = crate::periodic_attack::apply(&original, &mut map)?
            .ok_or_else(|| err("selected ruleset has no periodic attack update to apply"))?;
        for chunk in &before.chunks {
            let name = &before.table[&chunk.name_idx];
            if !["Teams", "PlayerScriptsList"].contains(&name.as_str()) && map.get(name)? != chunk {
                return Err(err(format!("script update changed unrelated chunk {name}")));
            }
        }
        let xml = flat
            .get("map.xml")
            .ok_or_else(|| err("script update requires map.xml"))?;
        let xml = crate::resources::refresh_script_xml(std::str::from_utf8(xml)?, &map)?.replace(
            &format!("DisplayName=\"{source_name}\""),
            &format!("DisplayName=\"{name}\""),
        );
        flat.insert("map.xml".into(), xml.into_bytes());
        if let Some(strings) = flat.get_mut("map.str") {
            let mut updated = std::str::from_utf8(strings)?
                .replace(source_name, &name)
                .replace(
                    &format!("版本 {source_version}"),
                    &format!("版本 {version}"),
                );
            crate::periodic_attack::append_strings(&original, &mut updated)?;
            *strings = updated.into_bytes();
        }
        Some(report)
    } else {
        None
    };
    let production_report = if production {
        let before = map.clone();
        let report = crate::production_spawns::apply(&original, &mut map)?;
        let xml = flat
            .get("map.xml")
            .ok_or_else(|| err("production repair requires source map.xml"))?;
        let xml = crate::production_spawns::update_xml(std::str::from_utf8(xml)?, &before, &map)?;
        let xml = xml.replace(
            &format!("DisplayName=\"{source_name}\""),
            &format!("DisplayName=\"{name}\""),
        );
        flat.insert("map.xml".into(), xml.into_bytes());
        if let Some(strings) = flat.get_mut("map.str") {
            *strings = std::str::from_utf8(strings)?
                .replace(source_name, &name)
                .replace(
                    &format!("版本 {source_version}"),
                    &format!("版本 {version}"),
                )
                .into_bytes();
        }
        for chunk in &before.chunks {
            let chunk_name = &before.table[&chunk.name_idx];
            if !["ObjectsList", "PlayerScriptsList", "TriggerAreas"].contains(&chunk_name.as_str())
                && map.get(chunk_name)? != chunk
            {
                return Err(err(format!(
                    "production repair changed unrelated chunk {chunk_name}"
                )));
            }
        }
        Some(report)
    } else {
        None
    };
    if repair {
        let mut export = original.clone();
        export.name = name.clone();
        export.version = version.clone();
        let resources = crate::resources::generate(&mut map, &export)?;
        flat.insert("map.xml".into(), resources.xml);
        flat.entry("overrides.xml".into())
            .or_insert(resources.overrides);
    }
    let polished = if polish {
        if let Edit::RampMoves(path) = edit {
            let before = map.clone();
            let plan = serde_json::from_slice(&fs::read(path)?)?;
            let result = crate::ramps::relocate(&original, &mut map, source_name, &plan)?;
            let xml = flat
                .get("map.xml")
                .ok_or_else(|| err("ramp relocation requires source XML"))?;
            let xml =
                crate::production_spawns::update_xml(std::str::from_utf8(xml)?, &before, &map)?
                    .replace(
                        &format!("DisplayName=\"{source_name}\""),
                        &format!("DisplayName=\"{name}\""),
                    );
            flat.insert("map.xml".into(), xml.into_bytes());
            let strings = flat
                .get_mut("map.str")
                .ok_or_else(|| err("ramp relocation requires source map.str"))?;
            *strings = std::str::from_utf8(strings)?
                .replace(source_name, &name)
                .replace(
                    &format!("版本 {source_version}"),
                    &format!("版本 {version}"),
                )
                .into_bytes();
            Some(result)
        } else {
            Some(crate::ramps::apply(&original, &mut map, source_name)?)
        }
    } else {
        None
    };
    let textured = if textures {
        Some(crate::texture_polish::apply(
            &original,
            &mut map,
            source_name,
        )?)
    } else {
        None
    };
    let plain = map.encode()?;
    let checked = MapFile::decode(&plain)?;
    checked.validate_release()?;
    let assets = root.join("assets").join(&original.group).join(&name);
    let dist = root.join("dist").join(&original.group).join(&name);
    let zip_path = root
        .join("dist")
        .join(&original.group)
        .join(format!("{name}.zip"));
    if assets.exists() || dist.exists() || zip_path.exists() {
        return Err(err(
            "new release path already exists; refusing to overwrite it",
        ));
    }
    let mut files = vec![
        (format!("{name}.map"), refpack::pack(&plain)?),
        (
            format!("{name}_art.tga"),
            if relocate {
                polished.as_ref().unwrap().art_bytes(&original)?
            } else {
                flat.remove(&format!("{source_name}_art.tga"))
                    .ok_or_else(|| err("ZIP has no matching map art"))?
            },
        ),
        (
            "map.str".into(),
            flat.remove("map.str")
                .ok_or_else(|| err("ZIP has no map.str"))?,
        ),
    ];
    for file in ["map.xml", "overrides.xml", "NewWorldBuilder.notes.json"] {
        if let Some(data) = flat.remove(file) {
            let data = if file == "map.xml" {
                retarget_xml(std::str::from_utf8(&data)?, source_version, &version)?.into_bytes()
            } else {
                data
            };
            files.push((file.into(), data));
        }
    }
    validate_files(&files)?;
    let zipped = archive::zip(
        &files
            .iter()
            .map(|(n, b)| (format!("{name}/{n}"), b.clone()))
            .collect::<Vec<_>>(),
    )?;
    let round = archive::unzip(&zipped)?;
    for (n, b) in &files {
        if round.get(&format!("{name}/{n}")) != Some(b) {
            return Err(err("release ZIP round trip differs"));
        }
    }
    let report = serde_json::json!({
        "version":version,"source":source,"source_zip_crc32":crc32fast::hash(&source_zip),
        "source_version":source_version,"files":files.iter().map(|(n,_)|n).collect::<Vec<_>>(),
        "string_table":"strict descending IDs; existing IDs preserved",
        "operation":if relocate {"relocate-ramps"} else if update_scripts {"update-scripts"} else if production {"repair-production"} else if repair {"repair-resources"} else if polish {"polish-ramps"} else if textures {"polish-textures"} else {"repack"},
        "preserved_chunks":map.chunks.iter().filter(|c|(!polish || !["HeightMapData","BlendTileData"].contains(&map.table[&c.name_idx].as_str())) && (!relocate || map.table[&c.name_idx]!="ObjectsList") && (!textures || map.table[&c.name_idx]!="BlendTileData") && (!repair || map.table[&c.name_idx]!="AssetList") && (!production || !["ObjectsList","PlayerScriptsList","TriggerAreas"].contains(&map.table[&c.name_idx].as_str())) && (!update_scripts || !["Teams","PlayerScriptsList"].contains(&map.table[&c.name_idx].as_str()))).map(|c| serde_json::json!({"name":map.table[&c.name_idx],"version":c.version,"length":c.payload.len(),"crc32":crc32fast::hash(&c.payload)})).collect::<Vec<_>>(),
        "ramp_optimization":polished.as_ref().map(|p| &p.report),
        "texture_optimization":textured.as_ref().map(|p| &p.report),
        "production_spawns":production_report,
        "periodic_attack":attack_report,
        "scripts":scripts::script_names(&scripts::parse_nodes(&checked.get("PlayerScriptsList")?.payload,&checked.table)?)?.len(),
        "asset_list_preserved":map.get("AssetList").is_ok()&&!repair,"asset_list_rebuilt":repair,"ingame_test":"not run"
    });
    let result = (|| -> Result<()> {
        for dir in [&assets, &dist] {
            fs::create_dir_all(dir)?;
            for (n, b) in &files {
                project::atomic_write(&dir.join(n), b)?;
            }
        }
        project::write_json(&assets.join("release-report.json"), &report)?;
        project::atomic_write(&zip_path, &zipped)?;
        if polished.is_some() || repair || textured.is_some() || production || update_scripts {
            let mut preview_project = original.clone();
            preview_project.version = version.clone();
            preview_project.name = name.clone();
            preview_project.state = state.clone();
            preview_project.assets = assets.clone();
            preview_project.dist = dist.clone();
            if let Some(polished) = &polished {
                polished.write_artifacts(&preview_project)?;
            }
            if repair {
                crate::resources::write_repair_artifacts(&preview_project, source_name, &checked)?;
            }
            if let Some(textured) = &textured {
                textured.write_artifacts(&preview_project)?;
            }
            if let Some(report) = &production_report {
                crate::production_spawns::write_artifacts(
                    &preview_project,
                    source_name,
                    &checked,
                    report,
                )?;
            }
            if let Some(report) = &attack_report {
                crate::resources::write_repair_artifacts(&preview_project, source_name, &checked)?;
                project::write_json(&assets.join("periodic-attack.json"), report)?;
                let source_dir = root.join("assets").join(&original.group).join(source_name);
                for name in [
                    "gift-clearance.json",
                    "production-spawns.json",
                    "production-facing.json",
                ] {
                    if source_dir.join(name).is_file() {
                        fs::copy(source_dir.join(name), assets.join(name))?;
                    }
                }
                let readme = if report["enabled"] == false {
                    format!(
                        "# {}\n\n已取消分钟进攻：移除 {} 条 minute-attack-* 脚本及对应诊断显示。\n\n基于 {} 增量更新，保留其他脚本、编队、地形、对象、资源和赠送建筑避让及朝向。\n\n[脚本更新](periodic-attack.json) · [发布核对](release-report.json) · [全要素预览](preview--layout.png)\n",
                        name, report["scripts_removed"], source_name
                    )
                } else {
                    format!(
                        "# {}\n\n每 60 秒统计各位置作战部队：大于等于 20 时通过 Assault AI 信标向 P1↔P5、P2↔P7、P3↔P8、P4↔P6 的目标出生点发送进攻请求；少于 20 时不发新请求。不判断真人玩家。名单排除建筑、矿车、工程师及基地车。部队指挥权始终归 AI，不招募、不建立进攻编队、不切换控制权。诊断显示检查轮次、P7/P8 兵力和 AI 进攻指令累计；指令数不表示实际出动数。\n\n[脚本规则](periodic-attack.json) · [发布核对](release-report.json) · [全要素预览](preview--layout.png)\n\n基于 {} 增量更新，移除旧进攻编队并更新 AI 请求脚本；原地形、对象、资源身份及赠送避让保留。自动化验证通过，游戏内效果待复测。\n",
                        name, source_name
                    )
                };
                fs::write(assets.join("README.md"), readme)?;
            }
        }
        project::write_json(&project::version_path(root, id), &state)?;
        Ok(())
    })();
    if let Err(e) = result {
        for path in [&assets, &dist] {
            if path.exists() {
                fs::remove_dir_all(path)?;
            }
        }
        if zip_path.exists() {
            fs::remove_file(&zip_path)?;
        }
        return Err(e);
    }
    let p = Project::load(root, id)?;
    if options.install {
        build::install(&p)?;
    }
    println!(
        "已发布 {}（{} 个成品文件，{} 字节）。",
        zip_path.display(),
        files.len(),
        zipped.len()
    );
    Ok(p)
}
