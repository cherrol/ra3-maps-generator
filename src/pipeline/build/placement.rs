//! Placement transitions and the proofs binding them to published native bytes.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, HeightMap, MapFile},
    pipeline::project::Project,
    terrain::{Layout, ObjectSpec, ramp_clearance, spawn_placement},
};
use serde_json::{Value, json};
use std::fs;

fn resources(m: &MapFile) -> Result<Vec<map::MapObject>> {
    Ok(map::objects(m.get("ObjectsList")?, &m.table)?
        .into_iter()
        .filter(|o| ramp_clearance::radius(&o.type_name).is_some())
        .collect())
}

/// Spawn checks can read an old, layout-free package without a rules template.
/// A same-version saved layout remains authoritative and must parse
/// successfully. An unrelated recipe blueprint cannot override native starts.
fn spawn_layout(p: &Project, release: &str, native: &MapFile) -> Result<Layout> {
    if let Some(layout) = Layout::release_snapshot(p, release)? {
        return Ok(layout);
    }
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(native.get("ObjectsList")?, &native.table)?;
    let bases = Layout::native_base_cells(p, native)?;
    let mut layout: Layout = serde_json::from_value(json!({
        "schema_version":1,"base_cells":bases,"ore_cells":[],"ore_angles":[],"objects":[]
    }))?;
    for object in objects {
        let cell = frame.grid([object.position[0] as f64, object.position[1] as f64]);
        if object.type_name == "OreNode" {
            layout.ore_cells.push(cell);
            layout.ore_angles.push(-(object.angle as f64));
        }
        layout.objects.push(ObjectSpec {
            cell,
            version: object.version,
            angle: object.angle,
            road_type: object.road_type,
            type_name: object.type_name,
            props: object.props,
        });
    }
    Ok(layout)
}

fn spawn_binding(p: &Project) -> Result<Value> {
    let reference = p
        .cfg
        .get("SPAWN_PLACEMENT_REFERENCE")
        .map(|value| -> Result<Value> {
            let path = value
                .as_str()
                .ok_or_else(|| err("SPAWN_PLACEMENT_REFERENCE must name a fixture file"))?;
            Ok(json!({"path":path,"crc32":crc32fast::hash(&fs::read(p.path(path)?)?)}))
        })
        .transpose()?;
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        // A cached report cannot replace checking the actual CRC-bound source.
        crate::pipeline::frozen_source::FrozenSource::load(p)?;
    }
    Ok(json!({
        "schema_version":1,
        "policy":spawn_placement::policy(p)?,
        "water_height":p.number("WATER_Z",200.),
        "terrain_only":p.flag("TERRAIN_ONLY"),
        "strict":p.cfg.get("SPAWN_PLACEMENT").is_some(),
        "source":p.cfg.get("COORDINATE_PIPELINE"),
        "historical_reference":reference
    }))
}

fn spawn_layout_stamp(layout: &Layout) -> Result<u32> {
    // Native bytes already bind all objects and terrain. Bind the additional
    // seat declarations while tolerating the JSON reader's one-ULP rounding.
    let bases = layout
        .base_cells
        .iter()
        .map(|cell| cell.map(|coordinate| (coordinate * 1_000_000.).round()))
        .collect::<Vec<_>>();
    Ok(crc32fast::hash(&serde_json::to_vec(&bases)?))
}

/// Validate the edit and construct the placement fields of its publication proof.
pub(crate) fn release_fields(
    p: &Project,
    source_name: &str,
    before: &MapFile,
    after: &MapFile,
    edited_layout: Option<&Layout>,
) -> Result<serde_json::Map<String, Value>> {
    let before_spawns = spawn_layout(p, source_name, before)?;
    let after_spawns = edited_layout.unwrap_or(&before_spawns);
    let spawn_report =
        spawn_placement::validate_transition(p, before, &before_spawns, after, after_spawns)?;
    let allocation = crate::terrain::resource_routes::validate_transition(p, before, after)?;
    let clearance = if let Some(layout) = edited_layout {
        ramp_clearance::validate_transition(
            p,
            before,
            &Layout::load_release(p, source_name)?,
            after,
            layout,
        )?
    } else {
        if before.get("HeightMapData")? != after.get("HeightMapData")?
            || resources(before)? != resources(after)?
        {
            return Err(err(
                "resource or terrain edits require complete ramp-layout validation",
            ));
        }
        let strict = if p.cfg.get("RAMP_CLEARANCE").is_some() {
            ramp_clearance::validate(p, after, &Layout::load_release(p, source_name)?)?
        } else {
            None
        };
        Some(json!({
            "passed":true,"status":"unchanged_resource_geometry",
            "policy":ramp_clearance::policy(p)?,"strict_check":strict
        }))
    };
    let frame = Frame::from_heightmap(p, &HeightMap::parse(after.get("HeightMapData")?)?)?;
    let geometry = edited_layout
        .map(|layout| ramp_clearance::geometry_crc32(p, after, layout))
        .transpose()?;
    Ok(serde_json::Map::from_iter([
        ("ramp_clearance".into(), serde_json::to_value(clearance)?),
        ("spawn_placement".into(), spawn_report),
        ("spawn_placement_binding".into(), spawn_binding(p)?),
        (
            "resource_distribution".into(),
            serde_json::to_value(allocation)?,
        ),
        (
            "resource_distribution_binding".into(),
            crate::terrain::resource_routes::binding(p)?,
        ),
        (
            "spawn_layout_crc32".into(),
            json!(spawn_layout_stamp(after_spawns)?),
        ),
        ("coordinate_frame".into(), serde_json::to_value(frame)?),
        (
            "placement_geometry_crc32".into(),
            serde_json::to_value(geometry)?,
        ),
    ]))
}

fn read_proof(p: &Project) -> Result<Option<Value>> {
    let data = match fs::read(p.assets.join("release-report.json")) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(serde_json::from_slice(&data)?))
}

fn matching_proof(
    p: &Project,
    bytes: &[u8],
    native: &MapFile,
    report: Value,
) -> Result<Option<Value>> {
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    if report["version"] == p.version
        && report["map_crc32"] == crc32fast::hash(bytes)
        && report["coordinate_frame"] == serde_json::to_value(frame)?
        && report["ramp_clearance"]["passed"] == true
        && report["ramp_clearance"]["policy"] == serde_json::to_value(ramp_clearance::policy(p)?)?
        && report["spawn_placement"]["passed"] == true
        && report["spawn_placement_binding"] == spawn_binding(p)?
        && report["resource_distribution_binding"] == crate::terrain::resource_routes::binding(p)?
    {
        return Ok(Some(report));
    }
    Ok(None)
}

pub fn placement_proof(p: &Project, bytes: &[u8]) -> Result<Option<Value>> {
    let Some(report) = read_proof(p)? else {
        return Ok(None);
    };
    matching_proof(p, bytes, &MapFile::load_bytes(bytes)?, report)
}

/// Cached ramp definitions are trusted only when bound to a validated edit.
/// Ordinary builds use the declared layout or verified frozen-source inputs.
pub fn validate_placement(p: &Project, files: &[(String, Vec<u8>)]) -> Result<()> {
    let (_, bytes) = files
        .iter()
        .find(|(name, _)| name.ends_with(".map"))
        .ok_or_else(|| err("placement validation requires a native map"))?;
    let native = MapFile::load_bytes(bytes)?;
    validate_native(p, bytes, &native)
}

pub(crate) fn validate_native(p: &Project, bytes: &[u8], native: &MapFile) -> Result<()> {
    let proof = match read_proof(p)? {
        Some(report) => matching_proof(p, bytes, native, report)?,
        None => None,
    };
    if let Some(proof) = proof {
        let spawn_layout = spawn_layout(p, &p.name, native)?;
        if proof["spawn_layout_crc32"] != spawn_layout_stamp(&spawn_layout)? {
            return Err(err(
                "cached spawn layout differs from the validated release seat declarations",
            ));
        }
        if proof["ramp_clearance"]["status"] == "unchanged_resource_geometry" {
            return Ok(());
        }
        let layout: Layout = serde_json::from_slice(&fs::read(p.source().join("layout.json"))?)?;
        if proof["placement_geometry_crc32"] != ramp_clearance::geometry_crc32(p, native, &layout)?
        {
            return Err(err(
                "cached ramp layout differs from the validated release geometry",
            ));
        }
        return Ok(());
    }
    if crate::pipeline::coordinate_pipeline::enabled(p) {
        let expected = crate::pipeline::coordinate_pipeline::verify(p)?;
        if expected.encode()? != native.encode()? {
            return Err(err("published map differs from verified coordinate inputs"));
        }
        return Ok(());
    }
    ramp_clearance::validate(p, native, &Layout::load(p)?)?;
    spawn_placement::validate(p, native, &Layout::load(p)?)?;
    crate::terrain::resource_routes::validate(p, native)?;
    Ok(())
}
