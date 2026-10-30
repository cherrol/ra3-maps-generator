//! Preserve source DEM values and rasterize geographic water independently of game design.
//! Run with `--root <repository>`, optionally `--map <region>`; existing map-local
//! `raw/source-lakes.geojson` files are sufficient for subsequent offline runs.
use ra3maps::{
    Result, err,
    formats::{
        archive::{unzip, zip},
        npy::{Npy, encode},
    },
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const LEGACY_WATER_REGIONS: [&str; 5] = ["云南", "广东", "广西", "海南", "陕西"];
const GLOBAL_LAKE_FILES: [&str; 2] = [
    "work/china-terrain-preview/sources/lakes.geojson",
    "work/china-terrain-preview/sources/extra-reservoirs.geojson",
];

struct SourceFeature {
    feature: Value,
    bounds: [f64; 4],
    origin: Value,
}

struct LakeSources {
    features: Vec<SourceFeature>,
    inputs: Vec<Value>,
}

struct Prepared {
    base: PathBuf,
    lakes: Vec<u8>,
    reference: Vec<u8>,
    report: Vec<u8>,
    summary: String,
}

fn array<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| err(format!("missing source array {name}")))
}

fn mask(files: &BTreeMap<String, Vec<u8>>, name: &str, shape: &[usize]) -> Result<Vec<u8>> {
    let value = Npy::parse(array(files, name)?)?;
    if value.shape != shape {
        return Err(err(format!("source mask shape differs from DEM: {name}")));
    }
    let values = value.numbers()?;
    if values.iter().any(|&n| n != 0. && n != 1.) {
        return Err(err(format!(
            "source mask must contain only 0 and 1: {name}"
        )));
    }
    Ok(values.into_iter().map(|n| n as u8).collect())
}

fn polygons(geometry: &Value) -> Result<Vec<&Value>> {
    match geometry["type"].as_str() {
        Some("Polygon") => Ok(vec![&geometry["coordinates"]]),
        Some("MultiPolygon") => Ok(geometry["coordinates"]
            .as_array()
            .ok_or_else(|| err("MultiPolygon coordinates must be an array"))?
            .iter()
            .collect()),
        _ => Err(err("source lake geometry must be Polygon or MultiPolygon")),
    }
}

fn coordinates(point: &Value) -> Result<[f64; 2]> {
    let x = point
        .get(0)
        .and_then(Value::as_f64)
        .ok_or_else(|| err("invalid lake longitude"))?;
    let y = point
        .get(1)
        .and_then(Value::as_f64)
        .ok_or_else(|| err("invalid lake latitude"))?;
    if !x.is_finite()
        || !y.is_finite()
        || !(-180. ..=180.).contains(&x)
        || !(-90. ..=90.).contains(&y)
    {
        return Err(err("invalid source lake coordinate"));
    }
    Ok([x, y])
}

fn geometry_bounds(geometry: &Value) -> Result<[f64; 4]> {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for polygon in polygons(geometry)? {
        for ring in polygon
            .as_array()
            .ok_or_else(|| err("invalid source polygon rings"))?
        {
            for point in ring
                .as_array()
                .ok_or_else(|| err("invalid source polygon ring"))?
            {
                let [x, y] = coordinates(point)?;
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x),
                    bounds[3].max(y),
                ];
            }
        }
    }
    if !bounds.iter().all(|n| n.is_finite()) {
        return Err(err("empty source lake polygon"));
    }
    Ok(bounds)
}

fn intersects(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

/// Scanline sampling matches the source ingest tool. Even-odd filling is done
/// within each polygon, preserving interior rings; polygons are then unioned.
fn raster(geometry: &Value, width: usize, height: usize, bounds: [f64; 4]) -> Result<Vec<u8>> {
    let mut mask = vec![0; width * height];
    for polygon in polygons(geometry)? {
        let mut intersections = vec![Vec::<f64>::new(); height];
        for ring in polygon
            .as_array()
            .ok_or_else(|| err("invalid source polygon rings"))?
        {
            let points = ring
                .as_array()
                .ok_or_else(|| err("invalid source polygon ring"))?;
            if points.len() < 3 {
                return Err(err("source lake ring has fewer than three vertices"));
            }
            let points: Vec<_> = points
                .iter()
                .map(|p| {
                    coordinates(p).map(|[x, y]| {
                        (
                            (x - bounds[0]) / (bounds[2] - bounds[0]) * width as f64 - 0.5,
                            (bounds[3] - y) / (bounds[3] - bounds[1]) * height as f64 - 0.5,
                        )
                    })
                })
                .collect::<Result<_>>()?;
            for i in 0..points.len() {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                if a.1 == b.1 {
                    continue;
                }
                let y0 = a.1.min(b.1).ceil().clamp(0., height as f64) as usize;
                let y1 = a.1.max(b.1).ceil().clamp(0., height as f64) as usize;
                for (y, row) in intersections.iter_mut().enumerate().take(y1).skip(y0) {
                    let yy = y as f64;
                    if (a.1 > yy) != (b.1 > yy) {
                        row.push(a.0 + (b.0 - a.0) * (yy - a.1) / (b.1 - a.1));
                    }
                }
            }
        }
        for (y, row) in intersections.iter_mut().enumerate() {
            row.sort_by(f64::total_cmp);
            if row.len() % 2 != 0 {
                return Err(err("unpaired source polygon scanline intersections"));
            }
            for pair in row.as_chunks::<2>().0 {
                let x0 = pair[0].ceil().clamp(0., width as f64) as usize;
                let x1 = pair[1].ceil().clamp(0., width as f64) as usize;
                mask[y * width + x0..y * width + x1].fill(1);
            }
        }
    }
    Ok(mask)
}

impl LakeSources {
    fn read(root: &Path) -> Result<Self> {
        let mut out = Self {
            features: Vec::new(),
            inputs: Vec::new(),
        };
        for path in GLOBAL_LAKE_FILES {
            let bytes = fs::read(root.join(path)).map_err(|e| err(format!(
                "cannot read full source lakes {path}: {e}; existing map-local raw/source-lakes.geojson can be used offline"
            )))?;
            let collection: Value = serde_json::from_slice(&bytes)?;
            let features = collection["features"]
                .as_array()
                .ok_or_else(|| err("source lakes must be a FeatureCollection"))?;
            out.inputs.push(
                json!({"path":path,"crc32":crc32fast::hash(&bytes),"feature_count":features.len()}),
            );
            for (index, feature) in features.iter().enumerate() {
                out.features.push(SourceFeature {
                    feature: feature.clone(),
                    bounds: geometry_bounds(&feature["geometry"])?,
                    origin: json!({"path":path,"feature_index":index}),
                });
            }
        }
        Ok(out)
    }

    fn local_collection(&self, bounds: [f64; 4]) -> Value {
        let selected: Vec<_> = self
            .features
            .iter()
            .filter(|f| intersects(f.bounds, bounds))
            .collect();
        json!({
            "type":"FeatureCollection",
            "features": selected.iter().map(|f| f.feature.clone()).collect::<Vec<_>>(),
            "source_inputs": self.inputs,
            "source_feature_origins": selected.iter().map(|f| f.origin.clone()).collect::<Vec<_>>(),
            "selection": "complete original features whose geographic bounding boxes intersect the original input frame; no area threshold, clipping or geometry edits",
            "bounds_wgs84": bounds,
        })
    }
}

fn combine_water(
    region: &str,
    ocean: &[u8],
    lakes: &[u8],
    legacy: Option<&[u8]>,
) -> Result<Vec<u8>> {
    if ocean.len() != lakes.len() || legacy.is_some_and(|m| m.len() != ocean.len()) {
        return Err(err("source water masks have inconsistent sizes"));
    }
    if LEGACY_WATER_REGIONS.contains(&region) && legacy.is_none() {
        return Err(err(
            "legacy region is missing its original source_water_mask",
        ));
    }
    Ok(ocean
        .iter()
        .zip(lakes)
        .enumerate()
        .map(|(i, (&o, &l))| {
            o | l
                | if LEGACY_WATER_REGIONS.contains(&region) {
                    legacy.unwrap()[i]
                } else {
                    0
                }
        })
        .collect())
}

fn prepare(root: &Path, region: &str, global: &mut Option<LakeSources>) -> Result<Prepared> {
    let base = root.join("maps").join(region).join("data/terrain-preview");
    let input_bytes = fs::read(base.join("input.npz"))?;
    let files = unzip(&input_bytes)?;
    let elevation = Npy::parse(array(&files, "elevation_m.npy")?)?;
    if elevation.shape.len() != 2 || elevation.shape.iter().any(|n| !(2..=4096).contains(n)) {
        return Err(err(
            "source elevation must be a two dimensional grid between 2 and 4096 cells per axis",
        ));
    }
    if elevation.numbers()?.iter().any(|z| !z.is_finite()) {
        return Err(err("source elevation contains nonfinite values"));
    }
    let shape = &elevation.shape;
    let (height, width) = (shape[0], shape[1]);
    let bounds_array = Npy::parse(array(&files, "bounds_wgs84.npy")?)?;
    let bounds: [f64; 4] = bounds_array
        .numbers()?
        .try_into()
        .map_err(|_| err("source bounds must contain four values"))?;
    if !bounds.iter().all(|n| n.is_finite()) || bounds[0] >= bounds[2] || bounds[1] >= bounds[3] {
        return Err(err("source bounds are invalid"));
    }
    let ocean = mask(&files, "source_ocean_mask.npy", shape)?;
    let legacy = if LEGACY_WATER_REGIONS.contains(&region) {
        Some(mask(&files, "source_water_mask.npy", shape)?)
    } else {
        None
    };
    mask(&files, "inside_province.npy", shape)?;
    let local_path = base.join("raw/source-lakes.geojson");
    let local: Value = if local_path.is_file() {
        serde_json::from_slice(&fs::read(&local_path)?)?
    } else {
        if global.is_none() {
            *global = Some(LakeSources::read(root)?);
        }
        global.as_ref().unwrap().local_collection(bounds)
    };
    let lakes = serde_json::to_vec(&local)?;
    if local["type"] != "FeatureCollection" || local["bounds_wgs84"] != json!(bounds) {
        return Err(err(format!(
            "{region}: cached source lakes do not match the original source bounds"
        )));
    }
    let features = local["features"]
        .as_array()
        .ok_or_else(|| err("cached source lakes have no feature array"))?;
    if local["source_feature_origins"].as_array().map(Vec::len) != Some(features.len())
        || local["source_inputs"].as_array().is_none()
    {
        return Err(err(
            "cached source lakes have incomplete original provenance",
        ));
    }
    let mut lake_mask = vec![0; width * height];
    for feature in features {
        let m = raster(&feature["geometry"], width, height, bounds)?;
        for (cell, sampled) in lake_mask.iter_mut().zip(m) {
            *cell |= sampled;
        }
    }
    let water = combine_water(region, &ocean, &lake_mask, legacy.as_deref())?;
    let reference_arrays = vec![
        (
            "elevation_m.npy".into(),
            array(&files, "elevation_m.npy")?.to_vec(),
        ),
        (
            "inside_province.npy".into(),
            array(&files, "inside_province.npy")?.to_vec(),
        ),
        (
            "bounds_wgs84.npy".into(),
            array(&files, "bounds_wgs84.npy")?.to_vec(),
        ),
        ("water_mask.npy".into(), encode("|u1", shape, &water)),
    ];
    let reference = zip(&reference_arrays)?;
    let dem_crc = crc32fast::hash(array(&files, "elevation_m.npy")?);
    let water_cells: usize = water.iter().map(|&v| v as usize).sum();
    let report = json!({
        "schema_version":1,
        "region":region,
        "reference":"source-reference.npz",
        "reference_crc32":crc32fast::hash(&reference),
        "input":"input.npz",
        "input_crc32":crc32fast::hash(&input_bytes),
        "shape":shape,
        "bounds_wgs84":bounds,
        "north_up":true,
        "elevation":{
            "array":"input.npz:elevation_m.npy",
            "npy_crc32":dem_crc,
            "raw_array_bytes_crc32":crc32fast::hash(&elevation.bytes),
            "npy_preserved_byte_for_byte":true,
            "smoothing":false,"game_layering":false,"height_edits":false,
        },
        "preserved_arrays":["elevation_m.npy","inside_province.npy","bounds_wgs84.npy"],
        "water":{
            "ocean_array":"input.npz:source_ocean_mask.npy",
            "ocean_npy_crc32":crc32fast::hash(array(&files,"source_ocean_mask.npy")?),
            "ocean_source":if matches!(region,"香港"|"澳门") {"preserved detailed OpenStreetMap coastline mask with Natural Earth context fallback"} else {"preserved original geographic ocean mask"},
            "lake_features":"raw/source-lakes.geojson",
            "lake_features_crc32":crc32fast::hash(&lakes),
            "lake_feature_count":features.len(),
            "original_global_lake_inputs":local["source_inputs"],
            "legacy_source_water_array":if legacy.is_some(){json!("input.npz:source_water_mask.npy")}else{Value::Null},
            "legacy_source_water_reason":if legacy.is_some(){json!("preserve all pre-filter water including existing river references from the five original regions")}else{Value::Null},
            "ocean_cells":ocean.iter().map(|&v|v as usize).sum::<usize>(),
            "lake_cells":lake_mask.iter().map(|&v|v as usize).sum::<usize>(),
            "water_cells":water_cells,
            "rasterization":"original polygon interiors sampled at original input pixel centers; even-odd rings preserve holes; lake polygons and original geographic masks are unioned",
            "minimum_lake_area_km2":Value::Null,
            "area_filtering":false,"morphological_opening":false,"morphological_closing":false,
            "game_shoreline_shaping":false,"small_water_removal":false,"game_layer_enclosure":false,
            "coverage_note":"water coverage follows the preserved original ocean, full available lake/reservoir polygon datasets, and legacy water references where present; no river geometry is invented for regions without a river source",
        },
        "rendering_scope":"source reference only; game preview arrays, parameters and images are not edited by this tool",
    });
    Ok(Prepared {
        base,
        lakes,
        reference,
        report: serde_json::to_vec_pretty(&report)?,
        summary: format!(
            "{region}: {width}x{height}, {} original lake features, {water_cells} source water cells",
            features.len()
        ),
    })
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut root = std::env::current_dir()?;
    let mut requested = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                root = PathBuf::from(args.next().ok_or_else(|| err("--root requires a path"))?)
            }
            "--map" => {
                requested = Some(
                    args.next()
                        .ok_or_else(|| err("--map requires a region name"))?,
                )
            }
            "--help" | "-h" => {
                println!(
                    "preview-source-reference [--root REPOSITORY] [--map REGION]\nPreserve original DEM arrays and derive source water from unfiltered geographic references.\nExisting map-local raw/source-lakes.geojson files allow offline repetition."
                );
                return Ok(());
            }
            _ => return Err(err(format!("unknown argument {arg}"))),
        }
    }
    let root = root.canonicalize()?;
    let mut regions = Vec::new();
    if let Some(region) = requested {
        if region.is_empty()
            || Path::new(&region).components().count() != 1
            || !matches!(
                Path::new(&region).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err(err("--map must be a single region directory name"));
        }
        regions.push(region);
    } else {
        for entry in fs::read_dir(root.join("maps"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && entry
                    .path()
                    .join("data/terrain-preview/input.npz")
                    .is_file()
            {
                regions.push(
                    entry
                        .file_name()
                        .into_string()
                        .map_err(|_| err("region name is not UTF-8"))?,
                );
            }
        }
        regions.sort();
    }
    if regions.is_empty() {
        return Err(err("no source terrain inputs found"));
    }
    // Resolve all inputs before changing any map-local reference artifacts.
    let mut global = None;
    let prepared = regions
        .iter()
        .map(|region| prepare(&root, region, &mut global))
        .collect::<Result<Vec<_>>>()?;
    for item in &prepared {
        fs::create_dir_all(item.base.join("raw"))?;
        write_if_changed(&item.base.join("raw/source-lakes.geojson"), &item.lakes)?;
        write_if_changed(&item.base.join("source-reference.npz"), &item.reference)?;
        write_if_changed(
            &item.base.join("source-reference-report.json"),
            &item.report,
        )?;
        println!("{}", item.summary);
    }
    println!(
        "Prepared {} source references; existing input, terrain and game preview files preserved",
        prepared.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_source_bounds_roundtrip_without_changing_coordinate_bits() {
        let bounds = [
            97.27999877929688_f64,
            20.899999618530273,
            106.44000244140625,
            29.479999542236328,
        ];
        let original = json!({"bounds_wgs84":bounds});
        let parsed: Value =
            serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
        assert_eq!(parsed, original);
        for (expected, actual) in bounds
            .iter()
            .zip(parsed["bounds_wgs84"].as_array().unwrap())
        {
            assert_eq!(expected.to_bits(), actual.as_f64().unwrap().to_bits());
        }
    }

    #[test]
    fn raster_preserves_holes_without_dropping_small_polygons() {
        let geometry = json!({"type":"Polygon","coordinates":[
            [[0,0],[5,0],[5,5],[0,5],[0,0]],
            [[1,1],[4,1],[4,4],[1,4],[1,1]],
        ]});
        let mask = raster(&geometry, 5, 5, [0., 0., 5., 5.]).unwrap();
        assert_eq!(mask.iter().filter(|&&v| v == 1).count(), 16);
        assert_eq!(mask[2 * 5 + 2], 0);
        let tiny = json!({"type":"Polygon","coordinates":[[[2.1,2.1],[2.9,2.1],[2.9,2.9],[2.1,2.9],[2.1,2.1]]]});
        let tiny = raster(&tiny, 5, 5, [0., 0., 5., 5.]).unwrap();
        assert_eq!(tiny.iter().filter(|&&v| v == 1).count(), 1);
    }

    #[test]
    fn multipolygon_components_union_instead_of_cancelling() {
        let geometry = json!({"type":"MultiPolygon","coordinates":[
            [[[0,0],[2,0],[2,2],[0,2],[0,0]]],
            [[[1,1],[3,1],[3,3],[1,3],[1,1]]],
        ]});
        let mask = raster(&geometry, 3, 3, [0., 0., 3., 3.]).unwrap();
        assert_eq!(mask.iter().filter(|&&v| v == 1).count(), 7);
        assert_eq!(mask[4], 1);
    }

    #[test]
    fn legacy_water_survives_only_in_original_five_regions() {
        let ocean = [1, 0, 0, 0];
        let lakes = [0, 1, 0, 0];
        let old = [0, 0, 1, 0];
        assert_eq!(
            combine_water("广东", &ocean, &lakes, Some(&old)).unwrap(),
            [1, 1, 1, 0]
        );
        assert_eq!(
            combine_water("香港", &ocean, &lakes, Some(&old)).unwrap(),
            [1, 1, 0, 0]
        );
        assert!(combine_water("广东", &ocean, &lakes, None).is_err());
    }
}
