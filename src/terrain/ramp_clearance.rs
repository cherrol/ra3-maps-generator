//! Full-width ramp approaches and future refinery reservations.
use crate::{
    Result,
    coordinates::Frame,
    err,
    formats::map::{self, HeightMap, MapFile},
    pipeline::project::Project,
    terrain::{self, Layout},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Policy {
    pub approach_extension: f64,
    pub gap: f64,
    pub refinery_back: f64,
    pub refinery_forward: f64,
    pub refinery_half_width: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            approach_extension: crate::rules::number("ramp_clearance", "approach_extension"),
            gap: crate::rules::number("ramp_clearance", "gap"),
            refinery_back: crate::rules::number("ramp_clearance", "refinery_back"),
            refinery_forward: crate::rules::number("ramp_clearance", "refinery_forward"),
            refinery_half_width: crate::rules::number("ramp_clearance", "refinery_half_width"),
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        if [
            self.approach_extension,
            self.gap,
            self.refinery_back,
            self.refinery_forward,
            self.refinery_half_width,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err(err("invalid ramp approach clearance policy"));
        }
        Ok(())
    }
}
pub fn policy(p: &Project) -> Result<Option<Policy>> {
    let mut fields = serde_json::to_value(Policy::default())?;
    let path = p.root.join("templates/rules/coordinate-pipeline.json");
    let common: Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.into()),
    };
    for overrides in [common.get("ramp_clearance"), p.cfg.get("RAMP_CLEARANCE")]
        .into_iter()
        .flatten()
    {
        let object = overrides
            .as_object()
            .ok_or_else(|| err("ramp clearance overrides must be an object"))?;
        for (key, value) in object {
            fields[key] = value.clone();
        }
    }
    let policy: Policy = serde_json::from_value(fields)?;
    policy.validate()?;
    Ok(Some(policy))
}
pub type Polygon = [[f64; 2]; 4];
pub fn rectangle(
    center: [f64; 2],
    unit: [f64; 2],
    start: f64,
    end: f64,
    half_width: f64,
) -> Polygon {
    [
        (start, -half_width),
        (end, -half_width),
        (end, half_width),
        (start, half_width),
    ]
    .map(|(t, v)| {
        [
            center[0] + unit[0] * t - unit[1] * v,
            center[1] + unit[1] * t + unit[0] * v,
        ]
    })
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub fn intersects(a: &Polygon, b: &Polygon) -> bool {
    for poly in [a, b] {
        for i in 0..4 {
            let p = poly[i];
            let q = poly[(i + 1) % 4];
            let axis = [q[1] - p[1], p[0] - q[0]];
            let aa = a.map(|p| dot(p, axis));
            let bb = b.map(|p| dot(p, axis));
            let lo = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
            let hi = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
            if hi(aa) < lo(bb) - 1e-8 || hi(bb) < lo(aa) - 1e-8 {
                return false;
            }
        }
    }
    true
}
fn point_segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (dot([p[0] - a[0], p[1] - a[1]], d) / dot(d, d)).clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
pub fn polygon_distance(a: &Polygon, b: &Polygon) -> f64 {
    if intersects(a, b) {
        return 0.;
    }
    let mut best = f64::INFINITY;
    for (points, edges) in [(a, b), (b, a)] {
        for p in points {
            for i in 0..4 {
                best = best.min(point_segment_distance(*p, edges[i], edges[(i + 1) % 4]));
            }
        }
    }
    best
}
#[derive(Clone, Debug)]
pub struct Ramp {
    pub id: usize,
    pub center: [f64; 2],
    pub unit: [f64; 2],
    pub half: f64,
    pub width: f64,
    pub pair: [f64; 2],
    pub approach: Polygon,
}
pub fn ramps(layout: &Layout, policy: &Policy) -> Result<Vec<Ramp>> {
    layout
        .ramps
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (ux, uy, lo, hi) = terrain::ramp_axes(r)?;
            let center = [terrain::number(r, "x", 0.), terrain::number(r, "y", 0.)];
            let half = terrain::number(r, "span", 35.) / 2. + terrain::number(r, "landing", 0.);
            let width = terrain::ramp_width(r)?;
            Ok(Ramp {
                id: layout.objects.len() + i + 1,
                center,
                unit: [ux, uy],
                half,
                width,
                pair: [lo, hi],
                approach: rectangle(
                    center,
                    [ux, uy],
                    -half - policy.approach_extension,
                    half + policy.approach_extension,
                    width / 2.,
                ),
            })
        })
        .collect()
}
pub fn radius(kind: &str) -> Option<f64> {
    if kind == "OreNode" {
        Some(20.)
    } else if kind == "OilDerrick" {
        Some(6.)
    } else if kind.contains("TechStructure") || kind.starts_with("EI_Structure_") {
        Some(8.)
    } else {
        None
    }
}
#[derive(Debug, Serialize)]
pub struct Conflict {
    pub ramp_id: usize,
    pub body_gap: f64,
    pub approach_gap: f64,
    pub refinery_gap: Option<f64>,
}
pub fn conflicts(
    point: [f64; 2],
    angle: f64,
    kind: &str,
    height: f64,
    ramps: &[Ramp],
    policy: &Policy,
) -> Vec<Conflict> {
    let Some(radius) = radius(kind) else {
        return Vec::new();
    };
    let field = (kind == "OreNode").then(|| {
        rectangle(
            point,
            [angle.cos(), -angle.sin()],
            -policy.refinery_back,
            policy.refinery_forward,
            policy.refinery_half_width,
        )
    });
    let mut out = Vec::new();
    for r in ramps {
        let delta = [point[0] - r.center[0], point[1] - r.center[1]];
        let u = dot(delta, r.unit);
        let v = dot(delta, [-r.unit[1], r.unit[0]]);
        if delta[0].hypot(delta[1])
            > r.half
                + policy.approach_extension
                + r.width
                + policy.refinery_forward
                + radius
                + policy.gap
        {
            continue;
        }
        let expected = if u >= 0. { r.pair[1] } else { r.pair[0] };
        if (u.abs() > r.half && (height - expected).abs() > 12.)
            || height < r.pair[0] - 12.
            || height > r.pair[1] + 12.
        {
            continue;
        }
        let distance = |extension: f64| {
            (u.abs() - r.half - extension)
                .max(0.)
                .hypot((v.abs() - r.width / 2.).max(0.))
                - radius
        };
        let body_gap = distance(0.);
        let approach_gap = distance(policy.approach_extension);
        let refinery_gap = field.as_ref().map(|p| polygon_distance(p, &r.approach));
        if body_gap + 1e-6 < policy.gap
            || approach_gap + 1e-6 < policy.gap
            || refinery_gap.is_some_and(|g| g + 1e-6 < policy.gap)
        {
            out.push(Conflict {
                ramp_id: r.id,
                body_gap,
                approach_gap,
                refinery_gap,
            });
        }
    }
    out
}
pub fn report(p: &Project, m: &MapFile, layout: &Layout, policy: &Policy) -> Result<Value> {
    policy.validate()?;
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let z = terrain::flip(&hm.elevations, hm.width);
    let ramps = ramps(layout, policy)?;
    let mut findings = Vec::new();
    let mut checked = 0usize;
    for (i, o) in map::objects(m.get("ObjectsList")?, &m.table)?
        .iter()
        .enumerate()
    {
        if radius(&o.type_name).is_none() {
            continue;
        }
        let point = frame.grid([o.position[0] as f64, o.position[1] as f64]);
        if !frame.playable(point) {
            continue;
        }
        checked += 1;
        let height = z[point[1].round() as usize * hm.width + point[0].round() as usize] as f64
            * frame.z_scale;
        for conflict in conflicts(point, o.angle as f64, &o.type_name, height, &ramps, policy) {
            findings.push(json!({"object_id":i+1,"type":o.type_name,"cell":point,"height":height,"conflict":conflict}));
        }
    }
    Ok(
        json!({"passed":findings.is_empty(),"checked_objects":checked,"ramps":ramps.len(),"policy":policy,"geometry_model":"conservative planning reservations, not exact faction meshes","findings":findings}),
    )
}

fn ramp_geometry(ramp: &Ramp) -> Value {
    // Authored f64 values can move by one ULP when layout JSON is read back.
    // Encode the shared stamp at 1e-9 precision as integers so the stamp itself
    // never depends on another JSON floating-point round trip. Collision tests
    // keep using the original values and their unchanged clearance thresholds.
    let fixed = |value: f64| (value * 1_000_000_000.).round() as i64;
    json!({"scale":1_000_000_000_i64,"center":ramp.center.map(fixed),
        "unit":ramp.unit.map(fixed),"half":fixed(ramp.half),
        "width":fixed(ramp.width),"pair":ramp.pair.map(fixed)})
}

/// Bind geometry diagnostics to the native data this check observes.
/// Textures, scripts and release labels are intentionally outside this binding.
pub fn geometry_crc32(p: &Project, m: &MapFile, layout: &Layout) -> Result<u32> {
    let policy = policy(p)?.expect("the common clearance policy is always enabled");
    let hm = HeightMap::parse(m.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(m.get("ObjectsList")?, &m.table)?
        .into_iter()
        .filter(|o| radius(&o.type_name).is_some())
        .collect::<Vec<_>>();
    let geometry = ramps(layout, &policy)?
        .iter()
        .map(ramp_geometry)
        .collect::<Vec<_>>();
    let mut hash = crc32fast::Hasher::new();
    hash.update(b"ramp-clearance-geometry-v2");
    hash.update(&serde_json::to_vec(&json!({"frame":frame,
        "policy":policy,"water":p.number("WATER_Z",200.),"objects":objects,"ramps":geometry}))?);
    hash.update(&m.get("HeightMapData")?.payload);
    Ok(hash.finalize())
}

/// Read-only diagnostic output; it cannot authorize placement acceptance.
pub fn reference(p: &Project, m: &MapFile, layout: &Layout) -> Result<Value> {
    let policy = policy(p)?.expect("the common clearance policy is always enabled");
    Ok(json!({"schema_version":1,"map_id":p.id,
        "geometry_crc32":geometry_crc32(p,m,layout)?,"policy":policy,
        "report":report(p,m,layout,&policy)?}))
}

fn finish(mut checked: Value) -> Result<Option<Value>> {
    checked["raw_passed"] = checked["passed"].clone();
    checked["inherited"] = json!(0);
    checked["regressions"] = checked["findings"].clone();
    checked["status"] = json!("strict");
    if checked["passed"] != true {
        return Err(err(format!(
            "ramp approach clearance rejected: {}",
            checked["regressions"]
        )));
    }
    Ok(Some(checked))
}

/// Every map requires zero current ramp-approach conflicts.
pub fn validate(p: &Project, m: &MapFile, layout: &Layout) -> Result<Option<Value>> {
    if p.cfg.get("RAMP_CLEARANCE_REFERENCE").is_some() {
        return Err(err(
            "RAMP_CLEARANCE_REFERENCE is no longer supported; all maps require current strict ramp clearance",
        ));
    }
    let policy = policy(p)?.expect("the common clearance policy is always enabled");
    finish(report(p, m, layout, &policy)?)
}

/// Incremental edits use the same strict check as initial publication.
pub fn validate_transition(
    p: &Project,
    _before_map: &MapFile,
    _before_layout: &Layout,
    after_map: &MapFile,
    after_layout: &Layout,
) -> Result<Option<Value>> {
    validate(p, after_map, after_layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::ObjectSpec;
    use std::{
        collections::BTreeMap,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(Project);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ra3-ramp-common-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("maps/test/authoring")).unwrap();
            Self(Project {
                root: root.clone(),
                id: "test".into(),
                cfg: json!({"GRID":256,"BORDER":8,"CELL":10.,"Z_SCALE":1.,"WATER_Z":200.}),
                state: json!({}),
                version: "T0.0.1".into(),
                group: "Test".into(),
                name: "TestT0.0.1".into(),
                assets: root.join("assets/Test/TestT0.0.1"),
                dist: root.join("dist/Test/TestT0.0.1"),
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    fn sample(p: &Project, kind: &str, point: [f64; 2], angle: f32) -> (MapFile, Layout) {
        let props = vec![map::Property {
            key: "uniqueID".into(),
            value: map::Value::Ascii("fixed-resource".into()),
        }];
        let spec = ObjectSpec {
            cell: point,
            version: 3,
            angle,
            road_type: 0,
            type_name: kind.into(),
            props,
        };
        let layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[spec],"ramps":[{"x":100.,"y":100.,"ux":1.,"uy":0.,"span":25.,"width":25.,"pair":[210.,280.]}]})).unwrap();
        let hm = HeightMap {
            width: 256,
            height: 256,
            border: 8,
            borders: vec![],
            elevations: vec![280; 256 * 256],
        };
        let mut m = MapFile {
            table: BTreeMap::from([
                (1, "HeightMapData".into()),
                (2, "ObjectsList".into()),
                (3, "Object".into()),
                (4, "uniqueID".into()),
            ]),
            order: vec![],
            chunks: vec![
                map::Chunk {
                    name_idx: 1,
                    version: 6,
                    payload: hm.encode().unwrap(),
                },
                map::Chunk {
                    name_idx: 2,
                    version: 1,
                    payload: vec![],
                },
            ],
        };
        m.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&[layout.objects[0].object(p)], &m).unwrap();
        (m, layout)
    }
    fn freeze(p: &mut Project, m: &MapFile, layout: &Layout) {
        let bytes = crate::formats::refpack::pack(&m.encode().unwrap()).unwrap();
        fs::write(
            p.path("authoring/source.zip").unwrap(),
            crate::formats::archive::zip(&[
                (format!("{}.map", p.name), bytes.clone()),
                ("layout.json".into(), serde_json::to_vec(layout).unwrap()),
            ])
            .unwrap(),
        )
        .unwrap();
        p.cfg["COORDINATE_PIPELINE"] = json!({"source_name":p.name,"source_package":"authoring/source.zip","source_crc32":crc32fast::hash(&bytes)});
    }

    #[test]
    fn common_policy_is_enabled_and_merges_shared_and_local_fields() {
        let mut f = Fixture::new();
        let directory = f.0.root.join("templates/rules");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("coordinate-pipeline.json"),
            br#"{"ramp_clearance":{"gap":10.0}}"#,
        )
        .unwrap();
        f.0.cfg["RAMP_CLEARANCE"] = json!({"approach_extension":30.});
        let p = policy(&f.0).unwrap().unwrap();
        assert_eq!(p.gap, 10.);
        assert_eq!(p.approach_extension, 30.);
        assert_eq!(p.refinery_forward, 48.);
        f.0.cfg["RAMP_CLEARANCE"] = json!(false);
        assert!(policy(&f.0).is_err());
    }

    #[test]
    fn frozen_source_findings_are_rejected_even_when_unchanged() {
        let mut f = Fixture::new();
        let (m, l) = sample(&f.0, "HospitalTechStructure", [145., 100.], 0.);
        assert!(
            validate(&f.0, &m, &l).is_err(),
            "new maps must be strict by default"
        );
        freeze(&mut f.0, &m, &l);
        assert!(validate(&f.0, &m, &l).is_err());
        assert!(validate_transition(&f.0, &m, &l, &m, &l).is_err());
        let (moved, ml) = sample(&f.0, "HospitalTechStructure", [144., 100.], 0.);
        assert!(validate(&f.0, &moved, &ml).is_err());
        assert!(validate_transition(&f.0, &m, &l, &moved, &ml).is_err());
        let mut changed = l.clone();
        changed.ramps[0]["x"] = json!(101.);
        assert!(validate_transition(&f.0, &m, &l, &m, &changed).is_err());
        let mut duplicated = m.clone();
        let mut objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
        objects.push(objects[0].clone());
        duplicated.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &duplicated).unwrap();
        assert!(validate_transition(&f.0, &m, &l, &duplicated, &l).is_err());
        f.0.cfg["RAMP_CLEARANCE"] = json!({});
        assert!(
            validate(&f.0, &m, &l).is_err(),
            "explicit per-map strictness must remain strict"
        );
    }

    #[test]
    fn native_ore_heading_controls_the_complete_refinery_reserve() {
        let mut f = Fixture::new();
        let (m, l) = sample(&f.0, "OreNode", [180., 100.], 0.);
        freeze(&mut f.0, &m, &l);
        assert!(validate(&f.0, &m, &l).is_ok());
        let (turned, tl) = sample(&f.0, "OreNode", [180., 100.], std::f32::consts::PI);
        assert!(validate_transition(&f.0, &m, &l, &turned, &tl).is_err());
    }

    #[test]
    fn removed_reference_parameter_cannot_authorize_a_conflict() {
        let mut f = Fixture::new();
        let (m, l) = sample(&f.0, "HospitalTechStructure", [145., 100.], 0.);
        let expected = reference(&f.0, &m, &l).unwrap();
        let path: PathBuf = f.0.path("reference.json").unwrap();
        let bytes = serde_json::to_vec(&expected).unwrap();
        fs::write(&path, &bytes).unwrap();
        f.0.cfg["RAMP_CLEARANCE_REFERENCE"] = json!("reference.json");
        assert!(
            validate(&f.0, &m, &l)
                .unwrap_err()
                .to_string()
                .contains("no longer supported")
        );
        let mut changed = m.clone();
        let mut hm = HeightMap::parse(changed.get("HeightMapData").unwrap()).unwrap();
        hm.elevations[0] += 1;
        changed.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        assert!(validate(&f.0, &changed, &l).is_err());
        assert!(validate_transition(&f.0, &m, &l, &changed, &l).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn diagnostic_reference_output_does_not_change_strict_acceptance() {
        let f = Fixture::new();
        for step in 0..32 {
            let (m, layout) = sample(
                &f.0,
                "HospitalTechStructure",
                [145.3, 114. + step as f64 * 0.1],
                0.,
            );
            let saved = reference(&f.0, &m, &layout).unwrap();
            fs::write(
                f.0.path("reference.json").unwrap(),
                serde_json::to_vec_pretty(&saved).unwrap(),
            )
            .unwrap();
            assert_eq!(saved["report"]["passed"], false);
            assert!(validate(&f.0, &m, &layout).is_err());
        }
    }

    #[test]
    fn fixed_ramp_stamps_survive_layout_json_without_weakening_native_bindings() {
        let f = Fixture::new();
        let (m, mut layout) = sample(&f.0, "HospitalTechStructure", [55., 100.], 0.);
        // This actual f64 is rounded by one ULP by the configured JSON reader.
        layout.ramps[0]["x"] = json!(9.789168638004107);
        let saved: Layout =
            serde_json::from_slice(&serde_json::to_vec_pretty(&layout).unwrap()).unwrap();
        let before = geometry_crc32(&f.0, &m, &layout).unwrap();
        assert_eq!(before, geometry_crc32(&f.0, &m, &saved).unwrap());
        assert!(validate_transition(&f.0, &m, &layout, &m, &saved).is_err());

        let mut edited = saved.clone();
        edited.ramps[0]["x"] = json!(9.789168638004107 + 0.001);
        assert_ne!(before, geometry_crc32(&f.0, &m, &edited).unwrap());
        assert!(validate_transition(&f.0, &m, &layout, &m, &edited).is_err());

        let mut native = m.clone();
        let mut hm = HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap();
        hm.elevations[0] += 1;
        native.get_mut("HeightMapData").unwrap().payload = hm.encode().unwrap();
        assert_ne!(before, geometry_crc32(&f.0, &native, &layout).unwrap());
        let mut objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
        objects[0].position[0] = f32::from_bits(objects[0].position[0].to_bits() + 1);
        native = m.clone();
        native.get_mut("ObjectsList").unwrap().payload =
            map::write_objects(&objects, &native).unwrap();
        assert_ne!(before, geometry_crc32(&f.0, &native, &layout).unwrap());
    }

    #[test]
    fn current_henan_geometry_crc_survives_real_layout_json_round_trips() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        {
            let id = "河南";
            let p = Project::load(root, id).unwrap();
            let source = p.cfg["COORDINATE_PIPELINE"]["source_package"]
                .as_str()
                .unwrap();
            let package =
                crate::formats::archive::unzip(&fs::read(p.path(source).unwrap()).unwrap())
                    .unwrap();
            let name = p.cfg["COORDINATE_PIPELINE"]["source_name"]
                .as_str()
                .unwrap();
            let m = MapFile::load_bytes(&package[&format!("{name}.map")]).unwrap();
            let original: Layout = serde_json::from_slice(&package["layout.json"]).unwrap();
            let decoded: Layout =
                serde_json::from_slice(&serde_json::to_vec_pretty(&original).unwrap()).unwrap();
            assert_eq!(
                geometry_crc32(&p, &m, &original).unwrap(),
                geometry_crc32(&p, &m, &decoded).unwrap(),
                "{id} geometry must survive layout persistence"
            );
        }
    }

    #[test]
    fn rotated_polygons_include_corners_and_exact_gaps() {
        let a = rectangle([0., 0.], [1., 0.], 0., 10., 2.);
        let b = rectangle([18., 0.], [1., 0.], 0., 10., 2.);
        assert!(!intersects(&a, &b));
        assert_eq!(polygon_distance(&a, &b), 8.);
        assert!(intersects(
            &a,
            &rectangle([5., 0.], [0.6, 0.8], -1., 1., 1.)
        ));
    }
    #[test]
    fn approach_and_refinery_corners_are_checked_beyond_ramp_ends() {
        let p = Policy::default();
        let r = Ramp {
            id: 1,
            center: [100., 100.],
            unit: [1., 0.],
            half: 12.5,
            width: 25.,
            pair: [212., 280.],
            approach: rectangle([100., 100.], [1., 0.], -37.5, 37.5, 12.5),
        };
        assert!(
            !conflicts(
                [145., 100.],
                0.,
                "HospitalTechStructure",
                280.,
                std::slice::from_ref(&r),
                &p
            )
            .is_empty()
        );
        assert!(
            !conflicts(
                [180., 100.],
                std::f64::consts::PI,
                "OreNode",
                280.,
                std::slice::from_ref(&r),
                &p
            )
            .is_empty()
        );
        assert!(
            conflicts(
                [180., 100.],
                0.,
                "OreNode",
                280.,
                std::slice::from_ref(&r),
                &p
            )
            .is_empty()
        );
        assert!(conflicts([145., 100.], 0., "HospitalTechStructure", 420., &[r], &p).is_empty());
    }
}
