//! Repackage a finished map without regenerating terrain, objects or scripts.
use crate::{
    Result, err,
    formats::archive,
    formats::map::MapFile,
    formats::refpack,
    gameplay::scripts,
    pipeline::build,
    pipeline::project::{self, Project},
};
use std::{collections::BTreeMap, fs, path::Path};

mod package;
pub(crate) use package::{load_runtime_map, load_source_map};
pub use package::{retarget_xml, validate_files, validate_runtime_files};

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

pub fn add_mountains(
    root: &Path,
    id: &str,
    source: &Path,
    plan: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Mountains(plan))
}

pub fn polish_ramp_surfaces(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::RampSurfaces)
}

pub fn repair_resources(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Resources)
}

/// Repair only the runtime atlas XML; preserve the original .map bytes.
pub fn repair_terrain_atlas(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::TerrainAtlas)
}

pub fn repair_terrain_package(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::TerrainPackage)
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
    RampSurfaces,
    RampMoves(&'a Path),
    Mountains(&'a Path),
    Resources,
    TerrainAtlas,
    TerrainPackage,
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
    let polish = matches!(
        edit,
        Edit::Ramps | Edit::RampSurfaces | Edit::RampMoves(_) | Edit::Mountains(_)
    );
    let mountains = matches!(edit, Edit::Mountains(_));
    let ramp_surfaces = matches!(edit, Edit::RampSurfaces);
    let relocate = matches!(edit, Edit::RampMoves(_));
    let repair = matches!(edit, Edit::Resources);
    let atlas = matches!(edit, Edit::TerrainAtlas);
    let terrain_package = matches!(edit, Edit::TerrainPackage);
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
    let source_version = original.release_version(source_name)?;
    let mut map = MapFile::load_bytes(&flat[&names[0]])?;
    let source_map = map.clone();
    let state = project::bump(&original.state, false, 1, &options.note)?;
    let version = project::format_version(&state)?;
    let name = original.release_name(&version)?;
    let mut export = original.clone();
    export.name = name.clone();
    export.version = version.clone();
    export.state = state.clone();
    export.assets = original.release_assets(&name)?;
    export.dist = root.join("dist").join(&original.id).join(&name);
    let selected_layers = crate::terrain::layers::SelectedLayers::load(&export)?;
    if atlas || terrain_package {
        map.validate_release()?;
        let xml = flat
            .get_mut("map.xml")
            .ok_or_else(|| err("atlas repair requires source map.xml"))?;
        *xml = crate::resources::sort_terrain_atlas(std::str::from_utf8(xml)?)?
            .replace(
                &format!("DisplayName=\"{source_name}\""),
                &format!("DisplayName=\"{name}\""),
            )
            .into_bytes();
        let strings = flat
            .get_mut("map.str")
            .ok_or_else(|| err("atlas repair requires source map.str"))?;
        *strings = std::str::from_utf8(strings)?
            .replace(source_name, &name)
            .replace(
                &format!("版本 {source_version}"),
                &format!("版本 {version}"),
            )
            .into_bytes();
    }
    let attack_report = if update_scripts {
        let before = map.clone();
        let report = crate::gameplay::periodic_attack::apply(&original, &mut map)?
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
            crate::gameplay::periodic_attack::append_strings(&original, &mut updated)?;
            *strings = updated.into_bytes();
        }
        Some(report)
    } else {
        None
    };
    let production_report = if production {
        let before = map.clone();
        let report = crate::gameplay::production_spawns::apply(&original, &mut map)?;
        let xml = flat
            .get("map.xml")
            .ok_or_else(|| err("production repair requires source map.xml"))?;
        let xml = crate::gameplay::production_spawns::update_xml(
            std::str::from_utf8(xml)?,
            &before,
            &map,
        )?;
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
        let resources = crate::resources::generate(&mut map, &export)?;
        flat.insert("map.xml".into(), resources.xml);
        flat.entry("overrides.xml".into())
            .or_insert(resources.overrides);
    }
    let polished = if polish {
        if let Edit::RampMoves(path) = edit {
            let before = map.clone();
            let plan = serde_json::from_slice(&fs::read(path)?)?;
            let result = crate::terrain::ramps::relocate(&original, &mut map, source_name, &plan)?;
            let xml = flat
                .get("map.xml")
                .ok_or_else(|| err("ramp relocation requires source XML"))?;
            let xml = crate::gameplay::production_spawns::update_xml(
                std::str::from_utf8(xml)?,
                &before,
                &map,
            )?
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
            let result = if let Edit::Mountains(path) = edit {
                let plan = serde_json::from_slice(&fs::read(path)?)?;
                crate::terrain::ramps::add_mountains(&original, &mut map, source_name, &plan)?
            } else if ramp_surfaces {
                crate::terrain::ramps::refresh_surfaces(&original, &mut map, source_name)?
            } else {
                crate::terrain::ramps::apply(&original, &mut map, source_name)?
            };
            let strings = flat
                .get_mut("map.str")
                .ok_or_else(|| err("ramp polishing requires source map.str"))?;
            *strings = std::str::from_utf8(strings)?
                .replace(source_name, &name)
                .replace(
                    &format!("版本 {source_version}"),
                    &format!("版本 {version}"),
                )
                .into_bytes();
            Some(result)
        }
    } else {
        None
    };
    let mut textured = if textures {
        Some(crate::terrain::texture_polish::apply(
            &original,
            &mut map,
            source_name,
        )?)
    } else {
        None
    };
    let finalization = if polish || textures || terrain_package {
        crate::resources::TerrainFinalization::CompactLookups
    } else {
        crate::resources::TerrainFinalization::PreserveLookups
    };
    let mut terrain_report = None;
    if let Some(xml) = flat.get_mut("map.xml") {
        let finalized = crate::resources::finalize_terrain_package(
            &original,
            &mut map,
            std::str::from_utf8(xml)?,
            &version,
            finalization,
        )?;
        terrain_report = finalized.compaction;
        if let Some(textured) = &mut textured {
            textured.record_finalization(&map, terrain_report.as_ref())?;
        }
        *xml = finalized
            .xml
            .replace(
                &format!("DisplayName=\"{source_name}\""),
                &format!("DisplayName=\"{name}\""),
            )
            .into_bytes();
    }
    if atlas && map.get("WorldInfo")? != source_map.get("WorldInfo")? {
        return Err(err(
            "atlas-only repair cannot fix the native terrain loading list; use repair-terrain-package",
        ));
    }
    let plain = map.encode()?;
    let checked = MapFile::decode(&plain)?;
    checked.validate_release()?;
    let placement = build::placement::release_fields(
        &original,
        source_name,
        &source_map,
        &checked,
        polished.as_ref().map(|p| &p.terrain.layout),
    )?;
    let assets = export.assets.clone();
    let dist = export.dist.clone();
    let zip_path = build::zip_path(&export);
    if assets.exists() || dist.exists() || zip_path.exists() {
        return Err(err(
            "new release path already exists; refusing to overwrite it",
        ));
    }
    let mut files = vec![
        (
            format!("{name}.map"),
            if atlas {
                flat[&names[0]].clone()
            } else {
                refpack::pack(&plain)?
            },
        ),
        (
            format!("{name}_art.tga"),
            if polish {
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
    validate_runtime_files(&files)?;
    let mut scene_project = crate::pipeline::scene::incremental_context(&export)?;
    if production {
        // A production repair must also enforce the actual packaged geometry
        // inputs, including local overrides, after every edit has completed.
        scene_project = crate::gameplay::official_placement::strict_project(&scene_project);
    }
    let scene_report = crate::pipeline::scene::analyse_files(&scene_project, &checked, &files)?;
    crate::pipeline::scene::require_passed(&scene_report)?;
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
    let mut report = serde_json::json!({
        "version":version,"source":source,"source_zip_crc32":crc32fast::hash(&source_zip),
        "map_crc32":crc32fast::hash(&files[0].1),
        "source_version":source_version,"files":files.iter().map(|(n,_)|n).collect::<Vec<_>>(),
        "string_table":"strict descending IDs; existing IDs preserved",
        "operation":if terrain_package {"repair-terrain-package"} else if atlas {"repair-terrain-atlas"} else if mountains {"add-mountains"} else if relocate {"relocate-ramps"} else if update_scripts {"update-scripts"} else if production {"repair-production"} else if repair {"repair-resources"} else if ramp_surfaces {"polish-ramp-surfaces"} else if polish {"polish-ramps"} else if textures {"polish-textures"} else {"repack"},
        "preserved_chunks":map.chunks.iter().filter(|c|source_map.get(&map.table[&c.name_idx]).is_ok_and(|old|old == *c)).map(|c| serde_json::json!({"name":map.table[&c.name_idx],"version":c.version,"length":c.payload.len(),"crc32":crc32fast::hash(&c.payload)})).collect::<Vec<_>>(),
        "terrain_package":terrain_report,
        "ramp_optimization":polished.as_ref().filter(|_| !mountains).map(|p| &p.report),
        "mountain_edit":polished.as_ref().filter(|_| mountains).map(|p| &p.report),
        "texture_optimization":textured.as_ref().map(|p| &p.report),
        "production_spawns":production_report,
        "periodic_attack":attack_report,
        "scripts":scripts::script_names(&scripts::parse_nodes(&checked.get("PlayerScriptsList")?.payload,&checked.table)?)?.len(),
        "asset_list_preserved":map.get("AssetList").is_ok()&&!repair,"asset_list_rebuilt":repair,"ingame_test":"not run"
    });
    report.as_object_mut().unwrap().extend(placement);
    let result = (|| -> Result<()> {
        for dir in [&assets, &dist] {
            fs::create_dir_all(dir)?;
            for (n, b) in &files {
                project::atomic_write(&dir.join(n), b)?;
            }
        }
        project::write_json(&assets.join("release-report.json"), &report)?;
        crate::pipeline::scene::verify_prepared(&scene_project, &checked, &files, scene_report)?;
        crate::coordinates::prepare(&export)?;
        project::atomic_write(&zip_path, &zipped)?;
        if let Some(polished) = &polished {
            polished.write_artifacts(&export)?;
        }
        if repair || terrain_package || atlas {
            crate::resources::write_repair_artifacts(&export, source_name, &checked)?;
        }
        if terrain_package {
            fs::write(
                assets.join("README.md"),
                format!(
                    "# {name}\n\n基于 {source_name} 清除全部未使用材质、整理混合记录，并同步原生和 XML 加载清单及地图/图集版本 ID。最终地形纹理最多 49 种；保留逐格材质与混合效果、高度、全部导航位图、对象及脚本。\n\n[发布核对](release-report.json)\n"
                ),
            )?;
        }
        if atlas {
            fs::write(
                assets.join("README.md"),
                format!(
                    "# {name}\n\n基于 {source_name} 整理运行时岩石材质表顺序。原生 .map 文件逐字节保留，高度、矿位、通行及脚本均不变。\n\n[发布核对](release-report.json)\n\n游戏内近景仍需复测；俯视渲染不能证明斜视效果正常。\n"
                ),
            )?;
        }
        if let Some(textured) = &textured {
            textured.write_artifacts(&export)?;
        }
        if let Some(report) = &production_report {
            crate::gameplay::production_spawns::write_artifacts(
                &export,
                source_name,
                &checked,
                report,
            )?;
        }
        if let Some(report) = &attack_report {
            crate::resources::write_repair_artifacts(&export, source_name, &checked)?;
            project::write_json(&assets.join("periodic-attack.json"), report)?;
            let source_dir = original.release_assets(source_name)?;
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
    finish_publication(
        &p,
        &selected_layers,
        options.install.then_some(build::install),
    )?;
    println!(
        "已发布 {}（{} 个成品文件，{} 字节）。",
        zip_path.display(),
        files.len(),
        zipped.len()
    );
    Ok(p)
}

fn finish_publication(
    p: &Project,
    selected_layers: &crate::terrain::layers::SelectedLayers,
    install: Option<fn(&Project) -> Result<()>>,
) -> Result<()> {
    selected_layers
        .sync_current(p)
        .map_err(|e| err(format!("发布已完成，蓝图同步失败：{e}")))?;
    if let Some(install) = install {
        install(p).map_err(|e| err(format!("发布已完成，安装失败：{e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn failed_installation_keeps_published_version_files_and_synced_blueprint() {
        struct Temp(PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let root = Temp(std::env::temp_dir().join(format!(
            "ra3maps-publish-install-failure-{}",
            std::process::id()
        )));
        project::write_json(
            &root.0.join("maps/Test/config.json"),
            &json!({"SCHEMA_VERSION":1,"MAP_NAME":"Test","GRID":8,
                "BORDER":0,"CELL":10.,"Z_SCALE":0.5,
                "LAYERS":"histories/v11/layers.npz"}),
        )
        .unwrap();
        let path = root.0.join("maps/Test/histories/v11/layers.npz");
        project::atomic_write(&path, b"selected original blueprint").unwrap();
        let version = project::version_path(&root.0, "Test");
        project::write_json(&version, &project::parse_version("T0.7.8").unwrap()).unwrap();
        let before = Project::load(&root.0, "Test").unwrap();
        let selected = crate::terrain::layers::SelectedLayers::load(&before).unwrap();
        project::atomic_write(&before.path("layers.npz").unwrap(), b"previous blueprint").unwrap();

        project::write_json(&version, &project::parse_version("T0.7.9").unwrap()).unwrap();
        let published = Project::load(&root.0, "Test").unwrap();
        for dir in [&published.assets, &published.dist] {
            project::atomic_write(&dir.join("committed.txt"), b"published file").unwrap();
        }
        let state = fs::read(&version).unwrap();
        let error = finish_publication(
            &published,
            &selected,
            Some(|p| {
                assert_eq!(
                    fs::read(p.path("layers.npz")?).unwrap(),
                    b"selected original blueprint"
                );
                Err(err("injected installation failure"))
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("发布已完成，安装失败"));
        assert_eq!(fs::read(version).unwrap(), state);
        assert_eq!(
            fs::read(published.path("layers.npz").unwrap()).unwrap(),
            b"selected original blueprint"
        );
        for dir in [&published.assets, &published.dist] {
            assert_eq!(
                fs::read(dir.join("committed.txt")).unwrap(),
                b"published file"
            );
        }
    }
}
