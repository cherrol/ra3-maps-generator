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
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

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

/// Repair obstructed gift slots with an explicit, source-bound local/base plan.
pub fn repair_gifts(
    root: &Path,
    id: &str,
    source: &Path,
    plan: &Path,
    options: &build::Options,
) -> Result<Project> {
    run_inner(root, id, source, options, Edit::Gifts(plan))
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
    Gifts(&'a Path),
    Scripts,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RampMaterialJoins {
    #[serde(default)]
    shoulder_cells: usize,
    #[serde(default)]
    ramp_ids: Vec<usize>,
    #[serde(default)]
    texture_pairs: Vec<[String; 2]>,
    #[serde(default)]
    seed_edges: Vec<[[usize; 2]; 2]>,
    #[serde(default)]
    groups: Vec<RampMaterialGroup>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RampMaterialGroup {
    ramp_ids: Vec<usize>,
    texture_pairs: Vec<[String; 2]>,
    #[serde(default)]
    seed_edges: Vec<[[usize; 2]; 2]>,
}

impl RampMaterialJoins {
    fn validated_groups(&self) -> Result<Vec<RampMaterialGroup>> {
        let groups = if self.groups.is_empty() {
            vec![RampMaterialGroup {
                ramp_ids: self.ramp_ids.clone(),
                texture_pairs: self.texture_pairs.clone(),
                seed_edges: self.seed_edges.clone(),
            }]
        } else {
            if !self.ramp_ids.is_empty()
                || !self.texture_pairs.is_empty()
                || !self.seed_edges.is_empty()
            {
                return Err(err(
                    "ramp material join groups and top-level ramp_ids/texture_pairs are mutually exclusive",
                ));
            }
            self.groups.clone()
        };
        let mut ids = BTreeSet::new();
        for group in &groups {
            if group.ramp_ids.is_empty()
                || group.ramp_ids.iter().any(|&id| id == 0 || !ids.insert(id))
                || group.texture_pairs.is_empty()
                || group.seed_edges.is_empty()
                || group.texture_pairs.iter().any(|pair| {
                    pair[0].trim().is_empty() || pair[1].trim().is_empty() || pair[0] == pair[1]
                })
            {
                return Err(err(
                    "ramp material joins require unique one-based ramp IDs across groups, explicit distinct texture pairs and reviewed seed_edges",
                ));
            }
        }
        Ok(groups)
    }
}

/// Point the localized `Map:DisplayName` value at this release. Incremental
/// edits copy the previous archive, so the value would otherwise keep the
/// version the strings were first generated with.
fn retarget_strings(strings: &str, name: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut rewrite = false;
    for line in strings.lines() {
        let trimmed = line.trim();
        if rewrite && trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            out.push(format!("\"{name}\""));
            rewrite = false;
            continue;
        }
        rewrite = trimmed == "Map:DisplayName";
        out.push(line.to_owned());
    }
    let mut text = out.join("\n");
    if strings.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn run_inner(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
    edit: Edit<'_>,
) -> Result<Project> {
    let project = Project::load(root, id)?;
    crate::pipeline::telemetry::attempt(&project, "incremental-release", |log| {
        let inputs = crate::pipeline::run_inputs::RunInputs::capture(&project)?;
        log.inputs(inputs.manifest())?;
        let source_bytes = fs::read(source)?;
        let source_hash = crate::formats::sha256::hex(&source_bytes);
        log.event(
            "source",
            serde_json::json!({"path":source,"sha256":source_hash}),
        )?;
        log.stage("edit-and-validate", || {
            run_edit(root, id, source, options, edit, &inputs, &source_hash)
        })
    })
}
fn run_edit(
    root: &Path,
    id: &str,
    source: &Path,
    options: &build::Options,
    edit: Edit<'_>,
    inputs: &crate::pipeline::run_inputs::RunInputs,
    source_hash: &str,
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
    let gifts = matches!(edit, Edit::Gifts(_));
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
    let mut map = load_source_map(
        &flat
            .iter()
            .map(|(name, data)| (name.clone(), data.clone()))
            .collect::<Vec<_>>(),
    )?;
    let source_map = map.clone();
    let (gift_plan, ramp_material_joins, gift_release_plan) = if let Edit::Gifts(path) = edit {
        let full: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        let mut gameplay = full.clone();
        let material = gameplay
            .as_object_mut()
            .ok_or_else(|| err("gift release plan must be an object"))?
            .remove("ramp_material_joins")
            .map(serde_json::from_value::<RampMaterialJoins>)
            .transpose()?;
        if let Some(material) = &material {
            material.validated_groups()?;
        }
        (
            Some(serde_json::from_value::<
                crate::gameplay::gift_clearance::RepairPlan,
            >(gameplay)?),
            material,
            Some(full),
        )
    } else {
        (None, None, None)
    };
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
    let mut production_report = if production || gifts {
        let before = map.clone();
        let report = if let Some(plan) = &gift_plan {
            let layout = crate::terrain::Layout::load_release(&original, source_name)?;
            crate::gameplay::gift_clearance::repair_obstructed_slots_with_plan(
                &original, &mut map, &layout, plan,
            )?
        } else {
            crate::gameplay::production_spawns::apply(&original, &mut map)?
        };
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
    let mut ramp_material_report = if let Some(joins) = &ramp_material_joins {
        let source_layout = crate::terrain::Layout::load_release(&original, source_name)?;
        let groups = joins.validated_groups()?;
        if groups
            .iter()
            .flat_map(|group| &group.ramp_ids)
            .any(|&id| id > source_layout.ramps.len())
        {
            return Err(err(
                "ramp material join plan refers to an unknown source ramp",
            ));
        }
        let actual_layout = reconcile_placement_layout(
            &original,
            &source_map,
            &map,
            &source_layout,
            &BTreeSet::new(),
        )?;
        let before = map.clone();
        let mut reports = Vec::new();
        let mut reviewed_components = BTreeSet::new();
        for group in &groups {
            let report = crate::terrain::ramps::repair_material_joins_seeded(
                &original,
                &mut map,
                &actual_layout,
                &group.ramp_ids,
                joins.shoulder_cells,
                &group.texture_pairs,
                &group.seed_edges,
            )?;
            for component in report["components"]
                .as_array()
                .ok_or_else(|| err("material join report has no component inventory"))?
            {
                let edges = component["edges"]
                    .as_array()
                    .filter(|edges| !edges.is_empty())
                    .ok_or_else(|| err("material join report has an empty boundary component"))?;
                if !reviewed_components.insert(serde_json::to_vec(edges)?) {
                    return Err(err(
                        "ramp material join groups select the same connected boundary; combine their ramp_ids into one group",
                    ));
                }
            }
            reports.push(report);
        }
        let report = serde_json::json!({
            "schema_version":1,
            "source_native_decoded_crc32":crc32fast::hash(&before.encode()?),
            "after_material_native_decoded_crc32":crc32fast::hash(&map.encode()?),
            "shoulder_cells":joins.shoulder_cells,
            "groups":reports,
        });
        for chunk in &before.chunks {
            let name = &before.table[&chunk.name_idx];
            if name != "BlendTileData" && map.get(name)? != chunk {
                return Err(err(format!(
                    "ramp material joins changed unrelated chunk {name}"
                )));
            }
        }
        let hm = crate::formats::map::HeightMap::parse(before.get("HeightMapData")?)?;
        let old = crate::formats::map::BlendTiles::parse(
            before.get("BlendTileData")?,
            hm.width,
            hm.height,
        )?;
        let checked =
            crate::formats::map::BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
        let bitmap_start = 4 + 8 * hm.width * hm.height;
        let bitmap_end = bitmap_start + 8 * hm.height * hm.width.div_ceil(8) + hm.width * hm.height;
        let old_masks = before
            .get("BlendTileData")?
            .payload
            .get(bitmap_start..bitmap_end)
            .ok_or_else(|| err("source ramp material gameplay masks are truncated"))?;
        let new_masks = map
            .get("BlendTileData")?
            .payload
            .get(bitmap_start..bitmap_end)
            .ok_or_else(|| err("candidate ramp material gameplay masks are truncated"))?;
        if map.table != before.table
            || old.tiles != checked.tiles
            || old.textures != checked.textures
            || old.impassability != checked.impassability
            || old.buildability != checked.buildability
            || old_masks != new_masks
        {
            return Err(err(
                "ramp material joins changed terrain materials, phases, atlas or gameplay masks",
            ));
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
    // Texture polishing repaints the existing archive; the localized display
    // name in map.str must still follow the new release identity, otherwise the
    // game's map list keeps showing the version it was first generated with.
    if textures && let Some(strings) = flat.get_mut("map.str") {
        let retargeted = retarget_strings(std::str::from_utf8(strings)?, &name);
        *strings = retargeted.into_bytes();
    }
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
    // Final production audits always cover every applicable native object and
    // script creation. Local repair/search settings cannot narrow publication.
    let scene_project = crate::pipeline::scene::incremental_context(&export)?;
    let scene_report = crate::pipeline::scene::analyse_files(&scene_project, &checked, &files)?;
    crate::pipeline::scene::require_passed(&scene_report)?;
    let declared_layout = match &polished {
        Some(polished) => polished.terrain.layout.clone(),
        None => crate::terrain::Layout::load_release(&original, source_name)?,
    };
    let candidate_layout = if gifts || production {
        let generated = production_report
            .as_ref()
            .and_then(|report| report["collision_avoidance"]["placements"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|placement| placement["waypoint"].as_str().map(str::to_owned))
            .collect();
        reconcile_placement_layout(&export, &source_map, &checked, &declared_layout, &generated)?
    } else {
        crate::coordinates::native_layout(&export, &checked, &declared_layout)?.0
    };
    if !export.flag("TERRAIN_ONLY") {
        crate::gameplay::gift_clearance::validate_with_layout(
            &export,
            &checked,
            &candidate_layout,
        )?;
        crate::gameplay::production_spawns::validate_final_with_layout(
            &export,
            &checked,
            &candidate_layout,
        )?;
    }
    let final_access = crate::gameplay::placement::validate_final_access(&export, &checked)?;
    let candidate_native_decoded_crc32 = crc32fast::hash(&checked.encode()?);
    let release_plan_crc32 = gift_release_plan
        .as_ref()
        .map(|plan| serde_json::to_vec(plan).map(|bytes| crc32fast::hash(&bytes)))
        .transpose()?;
    if let Some(material) = &mut ramp_material_report {
        material["candidate_native_decoded_crc32"] =
            serde_json::json!(candidate_native_decoded_crc32);
        material["release_plan_crc32"] = serde_json::json!(release_plan_crc32);
    }
    if gifts && let Some(gifts) = &mut production_report {
        if let Some(material) = &ramp_material_report {
            gifts["ramp_material_joins"] = material.clone();
        }
        gifts["final_production_access"] = final_access.clone();
        gifts["candidate_native_decoded_crc32"] = serde_json::json!(candidate_native_decoded_crc32);
        gifts["release_plan_crc32"] = serde_json::json!(release_plan_crc32);
    }
    // A reviewed retry must reproduce both its candidate and the source-bound
    // repair decisions/comparisons; do not silently replace archived evidence.
    let staged_gift = if let (Some(plan), Some(report)) = (&gift_release_plan, &production_report) {
        let pending =
            crate::pipeline::acceptance::stage_candidate(&export, &files, &candidate_layout)?;
        stage_gift_review(&pending, plan, report, ramp_material_report.as_ref())?;
        Some(pending)
    } else {
        None
    };
    if let Err(error) = crate::pipeline::acceptance::require_delivery_with_layout(
        &export,
        &files,
        &candidate_layout,
    ) {
        let candidate = match staged_gift {
            Some(candidate) => candidate,
            None => {
                crate::pipeline::acceptance::stage_candidate(&export, &files, &candidate_layout)?
            }
        };
        return Err(err(format!(
            "{error}; review candidate {} and retry the same edit before publication",
            candidate.display()
        )));
    }
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
        "operation":if terrain_package {"repair-terrain-package"} else if atlas {"repair-terrain-atlas"} else if mountains {"add-mountains"} else if relocate {"relocate-ramps"} else if update_scripts {"update-scripts"} else if gifts {"repair-gifts"} else if production {"repair-production"} else if repair {"repair-resources"} else if ramp_surfaces {"polish-ramp-surfaces"} else if polish {"polish-ramps"} else if textures {"polish-textures"} else {"repack"},
        "preserved_chunks":map.chunks.iter().filter(|c|source_map.get(&map.table[&c.name_idx]).is_ok_and(|old|old == *c)).map(|c| serde_json::json!({"name":map.table[&c.name_idx],"version":c.version,"length":c.payload.len(),"crc32":crc32fast::hash(&c.payload)})).collect::<Vec<_>>(),
        "terrain_package":terrain_report,
        "ramp_optimization":polished.as_ref().filter(|_| !mountains).map(|p| &p.report),
        "mountain_edit":polished.as_ref().filter(|_| mountains).map(|p| &p.report),
        "texture_optimization":textured.as_ref().map(|p| &p.report),
        "production_spawns":if production { production_report.as_ref() } else { None },
        "gift_repair":if gifts { production_report.as_ref() } else { None },
        "gift_repair_plan":gift_release_plan,
        "ramp_material_joins":ramp_material_report,
        "periodic_attack":attack_report,
        "scripts":scripts::script_names(&scripts::parse_nodes(&checked.get("PlayerScriptsList")?.payload,&checked.table)?)?.len(),
        "asset_list_preserved":map.get("AssetList").is_ok()&&!repair,"asset_list_rebuilt":repair,"ingame_test":"not run"
    });
    report.as_object_mut().unwrap().extend(placement);
    report["final_production_access"] = final_access;
    inputs.verify()?;
    if crate::formats::sha256::hex(&fs::read(source)?) != source_hash {
        return Err(err("release ZIP source changed during run"));
    }
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
        if polished.is_none() {
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
            if let Some(plan) = &gift_release_plan {
                project::write_json(&assets.join("gift-repair-plan.json"), plan)?;
                project::write_json(&assets.join("gift-repair.json"), report)?;
                project::write_json(&assets.join("gift-clearance.json"), report)?;
                if let Some(material) = &ramp_material_report {
                    project::write_json(&assets.join("ramp-material-joins.json"), material)?;
                }
                let preservation = if ramp_material_report.is_some() {
                    "保留高度、坡形、全部通行与建造位图、矿位、出生点及玩法条件；指定坡道仅补足局部原生材质混合，主材、子块相位及图集保持。"
                } else {
                    "保留地形、坡道、矿位、出生点、玩法条件及无关对象。"
                };
                fs::write(
                    assets.join("README.md"),
                    format!(
                        "# {name}\n\n基于 {source_name} 按源内容绑定的计划修复受阻赠送落点，先定主点、后定备用；局部修复与基地重选分别记录候选比较和理由。{preservation}移动落点同步保护区域、XML与完整同版布局。\n\n[修复计划](gift-repair-plan.json) · [修复与候选比较](gift-repair.json) · [发布核对](release-report.json)\n"
                    ),
                )?;
            }
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
        // The reviewed layout and later publication must identify the same
        // final native objects, including scenery displaced by placement.
        project::write_json(&export.source().join("layout.json"), &candidate_layout)?;
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

/// Reconcile an explicitly applied placement edit after verifying its immutable
/// source. The read-only native_layout checker intentionally stays strict.
fn reconcile_placement_layout(
    p: &Project,
    before: &MapFile,
    after: &MapFile,
    saved: &crate::terrain::Layout,
    declared_generated: &BTreeSet<String>,
) -> Result<crate::terrain::Layout> {
    use crate::{
        coordinates::Frame,
        formats::map::{self, HeightMap, Value, prop},
    };
    if saved.schema_version != 1 {
        return Err(err("unsupported placement source layout schema"));
    }
    let (mut layout, ids) = crate::coordinates::native_layout(p, before, saved)?;
    let old = map::objects(before.get("ObjectsList")?, &before.table)?;
    let new = map::objects(after.get("ObjectsList")?, &after.table)?;
    let source_frame = Frame::from_heightmap(p, &HeightMap::parse(before.get("HeightMapData")?)?)?;
    let frame = Frame::from_heightmap(p, &HeightMap::parse(after.get("HeightMapData")?)?)?;
    if source_frame != frame || new.len() < old.len() {
        return Err(err(
            "placement edit changed its coordinate frame or removed native objects",
        ));
    }
    // native_layout validates position, angle and stable names. Verify the rest
    // of each saved identity through that exact mapping before updating it.
    for entry in ids["entries"]
        .as_array()
        .ok_or_else(|| err("missing source identity mapping"))?
    {
        if entry["kind"] != "object" {
            continue;
        }
        let source = &saved.objects[entry["old_preview_id"].as_u64().unwrap() as usize - 1];
        let actual = &old[entry["preview_id"].as_u64().unwrap() as usize - 1];
        if source.version != actual.version
            || source.road_type != actual.road_type
            || source.props != actual.props
        {
            return Err(err(
                "saved placement source has stale native identity or properties",
            ));
        }
    }
    for (index, (a, b)) in old.iter().zip(&new).enumerate() {
        if a.type_name != b.type_name
            || a.version != b.version
            || a.road_type != b.road_type
            || a.props != b.props
            || a.position[2] != b.position[2]
            || !b.angle.is_finite()
            || b.position.iter().any(|v| !v.is_finite())
        {
            return Err(err(format!(
                "placement edit changed native identity/order/properties at object {}",
                index + 1
            )));
        }
    }
    let mut names = old
        .iter()
        .filter_map(|object| {
            prop(&object.props, "waypointName")
                .and_then(Value::text)
                .map(str::to_owned)
        })
        .collect::<BTreeSet<_>>();
    for object in &new[old.len()..] {
        let name = prop(&object.props, "waypointName")
            .and_then(Value::text)
            .ok_or_else(|| err("placement edit appended an undeclared non-waypoint object"))?;
        if !object.type_name.contains("Waypoint")
            || !declared_generated.contains(name)
            || !names.insert(name.to_owned())
        {
            return Err(err(format!(
                "placement edit appended an undeclared or duplicate waypoint: {name}"
            )));
        }
    }
    let old_cells = old
        .iter()
        .map(|object| source_frame.grid([object.position[0] as f64, object.position[1] as f64]))
        .collect::<Vec<_>>();
    let cells = new
        .iter()
        .map(|object| frame.grid([object.position[0] as f64, object.position[1] as f64]))
        .collect::<Vec<_>>();
    for group in [
        &mut layout.ore_cells,
        &mut layout.derrick_cells,
        &mut layout.house_cells,
        &mut layout.post_cells,
        &mut layout.garage_cells,
        &mut layout.airport_cells,
        &mut layout.hospital_cells,
        &mut layout.academy_cells,
        &mut layout.shipyard_cells,
        &mut layout.bridge_cells,
    ] {
        for point in group {
            let at = |index: usize| {
                (point[0] - old_cells[index][0]).hypot(point[1] - old_cells[index][1]) < 0.001
            };
            let physical = (0..old.len())
                .filter(|&i| at(i) && !old[i].type_name.contains("Waypoint"))
                .collect::<Vec<_>>();
            let matching = if physical.is_empty() {
                (0..old.len()).filter(|&i| at(i)).collect::<Vec<_>>()
            } else {
                physical
            };
            if matching.iter().any(|&i| old_cells[i] != cells[i]) {
                if matching.len() != 1 {
                    return Err(err(
                        "moved placement classification has ambiguous native identity",
                    ));
                }
                *point = cells[matching[0]];
            }
        }
    }
    for (index, point) in saved.ore_cells.iter().enumerate() {
        let matching = old
            .iter()
            .enumerate()
            .filter(|(i, object)| {
                object.type_name == "OreNode"
                    && (point[0] - old_cells[*i][0]).hypot(point[1] - old_cells[*i][1]) < 0.001
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if matching.len() == 1 && old[matching[0]].angle != new[matching[0]].angle {
            layout.ore_angles[index] = -(new[matching[0]].angle as f64);
        }
    }
    layout.objects = new
        .iter()
        .enumerate()
        .map(|(index, object)| crate::terrain::ObjectSpec {
            cell: cells[index],
            version: object.version,
            angle: object.angle,
            road_type: object.road_type,
            type_name: object.type_name.clone(),
            props: object.props.clone(),
        })
        .collect();
    Ok(crate::coordinates::native_layout(p, after, &layout)?.0)
}

fn stage_gift_review(
    pending: &Path,
    plan: &serde_json::Value,
    report: &serde_json::Value,
    material: Option<&serde_json::Value>,
) -> Result<()> {
    let directory = pending
        .parent()
        .ok_or_else(|| err("delivery candidate has no review directory"))?;
    let mut outputs = vec![
        ("gift-repair-plan.json", plan.clone()),
        ("gift-repair.json", report.clone()),
    ];
    if let Some(material) = material {
        outputs.push(("ramp-material-joins.json", material.clone()));
    }
    for (name, value) in outputs {
        let path = directory.join(name);
        let bytes = serde_json::to_vec_pretty(&value)?;
        if path.exists() {
            if fs::read(&path)? != bytes {
                return Err(err(format!(
                    "refusing to overwrite changed gift review evidence: {}",
                    path.display()
                )));
            }
        } else {
            project::atomic_write(&path, &bytes)?;
        }
    }
    Ok(())
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
    fn release_strings_take_the_new_display_name_and_leave_siblings_alone() {
        let source = "Series:Name\n\"九州大地\"\nEnd\n\nMap:DisplayName\n\"[CHAI]沃野奔流T0.1.1\"\nEnd\n\nMap:Description\n\"河南三层台地\"\nEnd\n";
        let out = retarget_strings(source, "[CHAI]沃野奔流T0.1.3");
        assert!(out.contains("Map:DisplayName\n\"[CHAI]沃野奔流T0.1.3\"\nEnd"));
        assert!(out.contains("Series:Name\n\"九州大地\"\nEnd"));
        assert!(out.contains("Map:Description\n\"河南三层台地\"\nEnd"));
        assert!(out.ends_with('\n'));
        assert_eq!(
            retarget_strings("no display name\n", "x"),
            "no display name\n"
        );
    }

    #[test]
    fn material_join_groups_keep_different_ramp_directions_explicit() {
        let parse = |value| serde_json::from_value::<RampMaterialJoins>(value).unwrap();
        let plan = parse(json!({"shoulder_cells":4,"groups":[
            {"ramp_ids":[1],"texture_pairs":[["Grass_A","Grass_B"]],"seed_edges":[[[1,1],[1,2]]]},
            {"ramp_ids":[2],"texture_pairs":[["Grass_B","Grass_A"]],"seed_edges":[[[4,1],[4,2]]]}
        ]}));
        assert_eq!(plan.validated_groups().unwrap().len(), 2);
        let duplicate = parse(json!({"groups":[
            {"ramp_ids":[1],"texture_pairs":[["A","B"]]},
            {"ramp_ids":[1],"texture_pairs":[["B","A"]]}
        ]}));
        assert!(duplicate.validated_groups().is_err());
        let mixed = parse(json!({"ramp_ids":[3],"texture_pairs":[["A","B"]],
            "groups":[{"ramp_ids":[1],"texture_pairs":[["A","B"]]}]}));
        assert!(mixed.validated_groups().is_err());
        for value in [
            json!({"ramp_ids":[0],"texture_pairs":[["A","B"]]}),
            json!({"ramp_ids":[1],"texture_pairs":[]}),
            json!({"ramp_ids":[1],"texture_pairs":[["A","A"]]}),
        ] {
            assert!(parse(value).validated_groups().is_err());
        }
        assert!(
            serde_json::from_value::<RampMaterialJoins>(json!({
                "ramp_ids":[1],"texture_pairs":[["A","B"]],"unbounded_repaint":true
            }))
            .is_err()
        );
    }

    #[test]
    fn placement_layout_reconciles_edits_without_relaxing_source_or_identity_checks() {
        use crate::formats::map::{self, Property, Value, prop};
        let p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "河南").unwrap();
        let before = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
        let saved = crate::terrain::Layout::load_release(&p, &p.name).unwrap();
        let objects = map::objects(before.get("ObjectsList").unwrap(), &before.table).unwrap();
        let waypoints = objects
            .iter()
            .enumerate()
            .filter(|(_, object)| {
                object.type_name.contains("Waypoint")
                    && prop(&object.props, "waypointName")
                        .and_then(Value::text)
                        .is_some_and(|name| !name.starts_with("Player_"))
            })
            .map(|(index, _)| index)
            .take(2)
            .collect::<Vec<_>>();
        let mut moved = objects.clone();
        moved[waypoints[0]].position[0] += p.number("CELL", 10.) as f32;
        moved[waypoints[0]].angle += 0.125;
        let mut after = before.clone();
        after.get_mut("ObjectsList").unwrap().payload = map::write_objects(&moved, &after).unwrap();
        assert!(crate::coordinates::native_layout(&p, &after, &saved).is_err());
        let updated =
            reconcile_placement_layout(&p, &before, &after, &saved, &BTreeSet::new()).unwrap();
        assert_eq!(updated.objects.len(), moved.len());
        assert_eq!(
            updated.objects[waypoints[0]].angle,
            moved[waypoints[0]].angle
        );
        assert_eq!(
            updated.objects[waypoints[0]].props,
            objects[waypoints[0]].props
        );
        assert_eq!(updated.base_cells, saved.base_cells);
        assert_eq!(updated.ramps, saved.ramps);
        crate::coordinates::native_layout(&p, &after, &updated).unwrap();

        let mut stale = saved.clone();
        stale.objects[waypoints[0]].props.push(Property {
            key: "undeclared_source_property".into(),
            value: Value::Bool(1),
        });
        assert!(reconcile_placement_layout(&p, &before, &after, &stale, &BTreeSet::new()).is_err());

        let mut reordered = objects.clone();
        reordered.swap(waypoints[0], waypoints[1]);
        after.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&reordered, &after).unwrap();
        assert!(reconcile_placement_layout(&p, &before, &after, &saved, &BTreeSet::new()).is_err());

        let mut appended = objects.clone();
        let mut extra = objects[waypoints[0]].clone();
        map::set(
            &mut extra.props,
            "waypointName",
            Value::Ascii("undeclared-placement-waypoint".into()),
        );
        map::set(
            &mut extra.props,
            "uniqueID",
            Value::Ascii("undeclared-placement-waypoint".into()),
        );
        appended.push(extra);
        after.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&appended, &after).unwrap();
        assert!(reconcile_placement_layout(&p, &before, &after, &saved, &BTreeSet::new()).is_err());
    }

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
