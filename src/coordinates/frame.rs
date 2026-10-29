//! Per-map coordinate frame, conversions and persisted declarations.
use crate::{
    Result, err,
    formats::map::HeightMap,
    pipeline::project::{self, Project},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{fs, path::Path};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub border: usize,
    pub cell_size: f64,
    pub z_scale: f64,
}
impl Frame {
    pub fn from_project(p: &Project) -> Self {
        Self {
            width: p.usize("GRID", 840),
            height: p.usize("GRID_HEIGHT", p.usize("GRID", 840)),
            border: p.usize("BORDER", 20),
            cell_size: p.number("CELL", 10.),
            z_scale: p.number("Z_SCALE", 0.0390625),
        }
    }
    pub fn from_heightmap(p: &Project, hm: &HeightMap) -> Result<Self> {
        let frame = Self {
            width: hm.width,
            height: hm.height,
            border: hm.border as usize,
            ..Self::from_project(p)
        };
        frame.validate()?;
        Ok(frame)
    }
    pub fn validate(&self) -> Result<()> {
        let border = self
            .border
            .checked_mul(2)
            .ok_or_else(|| err("coordinate border overflow"))?;
        if self.width <= border || self.height <= border {
            return Err(err("coordinate grid must exceed its border on both axes"));
        }
        if !self.cell_size.is_finite()
            || self.cell_size <= 0.
            || !self.z_scale.is_finite()
            || self.z_scale <= 0.
        {
            return Err(err(
                "coordinate CELL and Z_SCALE must be finite and positive",
            ));
        }
        Ok(())
    }
    pub fn descriptor(&self) -> Result<Json> {
        self.validate()?;
        Ok(json!({
            "schema_version":1,
            "frame":self,"origin":"northwest full-grid sample (0,0)",
            "x_direction":"east","y_direction":"south","grid_unit":"cell",
            "cell_size_world_units":self.cell_size,
            "full_grid_sample_bounds":[[0,0],[self.width-1,self.height-1]],
            "playable_sample_bounds":[[self.border,self.border],[self.width-self.border-1,self.height-self.border-1]],
            "playable_cells":[self.width-2*self.border,self.height-2*self.border],
            "world_origin_grid":[self.border,self.height-1-self.border],
            "world_x_direction":"east","world_y_direction":"north","world_unit":"game world unit",
            "world_transform":{"x":"(grid_x-border)*cell_size","y":"(height-1-border-grid_y)*cell_size"},
            "grid_transform":{"x":"world_x/cell_size+border","y":"height-1-border-world_y/cell_size"},
            "height_transform":"world_height=height_raw*z_scale",
            "in_playable":"sample domain excludes authored border; world conversion also preserves off-grid coordinates"
        }))
    }
    pub fn metadata(&self, p: &Project) -> Result<Json> {
        let mut metadata = self.descriptor()?;
        metadata["scope"] = json!("playable-map");
        metadata["map_id"] = json!(p.id);
        metadata["map_name"] = json!(p.group);
        metadata["release_name"] = json!(p.name);
        metadata["version"] = json!(p.version);
        metadata["source"] = json!({"kind":"recipe","path":format!("maps/{}/config.json",p.id)});
        Ok(metadata)
    }
    pub fn write_system(&self, p: &Project, dir: &Path) -> Result<Json> {
        let metadata = self.metadata(p)?;
        fs::create_dir_all(dir)?;
        write_declaration(&dir.join("coordinate-system.json"), &metadata)?;
        Ok(metadata)
    }
    pub fn world(&self, p: [f64; 2]) -> [f64; 2] {
        [
            (p[0] - self.border as f64) * self.cell_size,
            (self.height as f64 - 1. - self.border as f64 - p[1]) * self.cell_size,
        ]
    }
    pub fn grid(&self, p: [f64; 2]) -> [f64; 2] {
        [
            p[0] / self.cell_size + self.border as f64,
            self.height as f64 - 1. - self.border as f64 - p[1] / self.cell_size,
        ]
    }
    /// Rotate an object's local world-unit offset by its native heading, then
    /// convert through the same per-map frame used for every native object.
    pub fn local_to_grid(
        &self,
        grid_center: [f64; 2],
        heading_radians: f64,
        offset: [f64; 2],
    ) -> [f64; 2] {
        let center = self.world(grid_center);
        let (sin, cos) = heading_radians.sin_cos();
        self.grid([
            center[0] + cos * offset[0] - sin * offset[1],
            center[1] + sin * offset[0] + cos * offset[1],
        ])
    }
    pub fn inside(&self, p: [f64; 2]) -> bool {
        p[0] >= 0. && p[1] >= 0. && p[0] < self.width as f64 && p[1] < self.height as f64
    }
    pub fn playable(&self, p: [f64; 2]) -> bool {
        p[0] >= self.border as f64
            && p[1] >= self.border as f64
            && p[0] < (self.width - self.border) as f64
            && p[1] < (self.height - self.border) as f64
    }
}

fn write_declaration(path: &Path, metadata: &Json) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(metadata)?;
    bytes.push(b'\n');
    match fs::read(path) {
        Ok(current) if current == bytes => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    project::atomic_write(path, &bytes)
}

/// Establish the map's coordinate frame before placing resources or objects.
pub fn prepare(p: &Project) -> Result<Frame> {
    let frame = Frame::from_project(p);
    let mut metadata = frame.write_system(p, &p.source())?;
    let current = project::format_version(&project::load_version(&p.root, &p.id)?)?;
    if current == p.version {
        // Old release exports keep their own descriptor without replacing the
        // coordinate system selected by the current map recipe.
        let fields = metadata.as_object_mut().unwrap();
        fields.remove("release_name");
        fields.remove("version");
        write_declaration(&p.map_dir().join("coordinate-system.json"), &metadata)?;
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProject(Project);
    impl Drop for TestProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }
    fn project_fixture() -> TestProject {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "ra3-coordinate-frame-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        TestProject(Project {
            assets: root.join("assets/frame-test/T0.0.1"),
            dist: root.join("dist/frame-test/T0.0.1"),
            root,
            id: "frame-test".into(),
            cfg: json!({"GRID":128,"BORDER":7,"CELL":7.5,"Z_SCALE":0.125}),
            state: json!({"stage":"T","major":0,"minor":0,"patch":1}),
            version: "T0.0.1".into(),
            group: "frame-test".into(),
            name: "frame-testT0.0.1".into(),
        })
    }

    #[test]
    fn rectangular_nondefault_frame_roundtrips_grid_and_world_coordinates() {
        let frame = Frame {
            width: 128,
            height: 80,
            border: 7,
            cell_size: 7.5,
            z_scale: 0.125,
        };
        frame.validate().unwrap();
        assert_eq!(frame.world([7., 72.]), [0., 0.]);
        assert_eq!(frame.world([120., 7.]), [847.5, 487.5]);
        for grid in [[0., 0.], [7., 72.], [38.25, 51.75], [128.5, -3.5]] {
            let roundtrip = frame.grid(frame.world(grid));
            for (actual, expected) in roundtrip.into_iter().zip(grid) {
                assert!((actual - expected).abs() < 1e-9);
            }
        }
        for world in [[0., 0.], [847.5, 487.5], [-127.25, 932.5]] {
            let roundtrip = frame.world(frame.grid(world));
            for (actual, expected) in roundtrip.into_iter().zip(world) {
                assert!((actual - expected).abs() < 1e-9);
            }
        }
        assert!(frame.playable([120., 72.]));
        assert!(!frame.playable([121., 72.]));
        assert!(!frame.inside([70., 80.]));
        let local = frame.local_to_grid([38.25, 51.75], std::f64::consts::FRAC_PI_2, [15., 7.5]);
        assert!((local[0] - 37.25).abs() < 1e-9);
        assert!((local[1] - 49.75).abs() < 1e-9);
    }

    #[test]
    fn repeated_preparation_does_not_rewrite_identical_declarations() {
        let fixture = project_fixture();
        let p = &fixture.0;
        project::write_json(&project::version_path(&p.root, &p.id), &p.state).unwrap();
        prepare(p).unwrap();
        let paths = [
            p.map_dir().join("coordinate-system.json"),
            p.source().join("coordinate-system.json"),
        ];
        let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(123456);
        for path in &paths {
            fs::OpenOptions::new()
                .write(true)
                .open(path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(time))
                .unwrap();
        }
        prepare(p).unwrap();
        for path in &paths {
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), time);
        }
    }

    #[test]
    fn resource_objects_and_script_points_use_the_recipe_frame() {
        let fixture = project_fixture();
        let p = &fixture.0;
        for kind in ["OreNode", "OilDerrick", "HospitalTechStructure"] {
            let spec = crate::terrain::ObjectSpec {
                cell: [12.5, 93.25],
                version: 1,
                angle: 0.,
                road_type: 0,
                type_name: kind.into(),
                props: vec![],
            };
            let object = spec.object(p);
            assert_eq!(object.position, [41.25, 200.625, 0.]);
            assert_eq!(
                crate::gameplay::script_spawns::to_cell(p, &object),
                spec.cell
            );
        }
    }

    #[test]
    fn native_dimensions_and_border_define_rectangular_metadata() {
        let fixture = project_fixture();
        let p = &fixture.0;
        let hm = HeightMap {
            width: 96,
            height: 64,
            border: 5,
            borders: vec![],
            elevations: vec![],
        };
        let frame = Frame::from_heightmap(p, &hm).unwrap();
        let metadata = frame.write_system(p, &p.source()).unwrap();
        assert_eq!(frame.width, 96);
        assert_eq!(frame.height, 64);
        assert_eq!(frame.border, 5);
        assert_eq!(frame.cell_size, 7.5);
        assert_eq!(frame.z_scale, 0.125);
        assert_eq!(
            metadata["full_grid_sample_bounds"],
            json!([[0, 0], [95, 63]])
        );
        assert_eq!(
            metadata["playable_sample_bounds"],
            json!([[5, 5], [90, 58]])
        );
        assert_eq!(metadata["world_origin_grid"], json!([5, 58]));
        assert_eq!(metadata["playable_cells"], json!([86, 54]));
        assert_eq!(metadata["scope"], "playable-map");
        let saved: Json =
            serde_json::from_slice(&fs::read(p.source().join("coordinate-system.json")).unwrap())
                .unwrap();
        assert_eq!(saved, metadata);
        assert_eq!(
            serde_json::from_value::<Frame>(saved["frame"].clone()).unwrap(),
            frame
        );
    }

    #[test]
    fn prepare_persists_current_map_identity_without_old_release_overwrite() {
        let fixture = project_fixture();
        let p = &fixture.0;
        let frame = prepare(p).unwrap();
        let map_path = p.map_dir().join("coordinate-system.json");
        let bytes = fs::read(&map_path).unwrap();
        let metadata: Json = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(metadata["map_id"], "frame-test");
        assert_eq!(metadata["map_name"], "frame-test");
        assert!(metadata.get("version").is_none());
        assert!(metadata.get("source_release").is_none());
        assert_eq!(metadata["frame"], json!(frame));
        let mut old = p.clone();
        old.version = "T0.0.0".into();
        old.name = "frame-testT0.0.0".into();
        old.assets = p.root.join("assets/frame-test/T0.0.0");
        old.cfg["CELL"] = json!(3.);
        prepare(&old).unwrap();
        assert_eq!(fs::read(map_path).unwrap(), bytes);
        assert!(old.source().join("coordinate-system.json").is_file());
    }

    #[test]
    fn malformed_frames_fail_before_writing_a_descriptor() {
        let fixture = project_fixture();
        let p = &fixture.0;
        let mut frame = Frame::from_project(p);
        frame.height = 14;
        assert!(frame.write_system(p, &p.source()).is_err());
        assert!(!p.source().exists());
        frame.height = 80;
        for scale in [0., -1., f64::NAN, f64::INFINITY] {
            frame.cell_size = scale;
            assert!(frame.validate().is_err());
        }
        frame.cell_size = 7.5;
        frame.z_scale = 0.;
        assert!(frame.validate().is_err());
    }
}
