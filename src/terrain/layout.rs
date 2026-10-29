//! Declared object positions and validated map layouts.
use super::{number, ramp_axes, ramp_width};
use crate::{
    Result, err,
    formats::map::{self, HeightMap, MapFile, MapObject, Property},
    pipeline::project::Project,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectSpec {
    pub cell: [f64; 2],
    pub version: u16,
    pub angle: f32,
    pub road_type: u32,
    pub type_name: String,
    pub props: Vec<Property>,
}
impl ObjectSpec {
    pub fn object(&self, p: &Project) -> MapObject {
        let position = crate::coordinates::Frame::from_project(p).world(self.cell);
        MapObject {
            version: self.version,
            position: [position[0] as f32, position[1] as f32, 0.],
            angle: self.angle,
            road_type: self.road_type,
            type_name: self.type_name.clone(),
            props: self.props.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    pub schema_version: u32,
    pub base_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub base_pad_cells: Vec<[f64; 2]>,
    pub ore_cells: Vec<[f64; 2]>,
    pub ore_angles: Vec<f64>,
    #[serde(default)]
    pub ore_kind: Vec<i8>,
    #[serde(default)]
    pub ore_owner: Vec<i32>,
    #[serde(default)]
    pub derrick_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub house_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub post_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub garage_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub airport_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub hospital_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub academy_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub shipyard_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub bridge_cells: Vec<[f64; 2]>,
    #[serde(default)]
    pub ramps: Vec<Value>,
    pub objects: Vec<ObjectSpec>,
    #[serde(default)]
    pub city_cells: Vec<[f64; 2]>,
}
impl Layout {
    /// Only a snapshot belonging to this exact native release declares its
    /// starts. A recipe blueprint may describe a different historical layout.
    pub(crate) fn release_snapshot(p: &Project, source_name: &str) -> Result<Option<Self>> {
        let path = p.release_assets(source_name)?.join("source/layout.json");
        match fs::read(path) {
            Ok(bytes) => return Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        if p.cfg["COORDINATE_PIPELINE"]["source_name"].as_str() == Some(source_name) {
            return Ok(Some(
                crate::pipeline::frozen_source::FrozenSource::load(p)?.layout()?,
            ));
        }
        Ok(None)
    }

    pub(crate) fn native_base_cells(p: &Project, native: &MapFile) -> Result<Vec<[f64; 2]>> {
        let hm = HeightMap::parse(native.get("HeightMapData")?)?;
        let frame = crate::coordinates::Frame::from_heightmap(p, &hm)?;
        let objects = map::objects(native.get("ObjectsList")?, &native.table)?;
        let mut bases = Vec::new();
        for seat in 1..=8 {
            let ordinary = format!("Player_{seat}_Start");
            let imported = format!("Player_{seat}_Start (imported)");
            let starts = objects
                .iter()
                .filter(|o| {
                    o.type_name.contains("Waypoint")
                        && map::prop(&o.props, "waypointName")
                            .and_then(map::Value::text)
                            .is_some_and(|name| name == ordinary || name == imported)
                })
                .collect::<Vec<_>>();
            if starts.len() != 1 {
                return Err(err(format!(
                    "native spawn layout requires exactly one start for P{seat}"
                )));
            }
            bases.push(frame.grid([starts[0].position[0] as f64, starts[0].position[1] as f64]));
        }
        Ok(bases)
    }

    /// Import the native seats only when no same-version declaration exists;
    /// a conflicting published declaration must still fail validation.
    pub(crate) fn sync_imported_starts(
        &mut self,
        p: &Project,
        source_name: &str,
        native: &MapFile,
    ) -> Result<()> {
        if Self::release_snapshot(p, source_name)?.is_none() {
            self.base_cells = Self::native_base_cells(p, native)?;
        }
        Ok(())
    }

    /// Reuse a release's actual layout, falling back to the selected recipe.
    pub(crate) fn load_release(p: &Project, source_name: &str) -> Result<Self> {
        match Self::release_snapshot(p, source_name)? {
            Some(layout) => Ok(layout),
            None => Self::load(p),
        }
    }
    pub fn load(p: &Project) -> Result<Self> {
        if crate::pipeline::coordinate_pipeline::enabled(p) {
            return crate::pipeline::coordinate_pipeline::layout(p);
        }
        let mut layout: Self =
            serde_json::from_slice(&fs::read(p.path(p.string("LAYOUT", "layout.json"))?)?)?;
        if layout.schema_version != 1 {
            return Err(err("unsupported layout schema"));
        }
        let n = p.number("GRID", 0.);
        let h = p.number("GRID_HEIGHT", n);
        for c in layout
            .base_cells
            .iter()
            .chain(&layout.ore_cells)
            .chain(&layout.base_pad_cells)
            .chain(&layout.derrick_cells)
            .chain(&layout.house_cells)
            .chain(&layout.post_cells)
            .chain(&layout.garage_cells)
            .chain(&layout.airport_cells)
            .chain(&layout.hospital_cells)
            .chain(&layout.academy_cells)
            .chain(&layout.shipyard_cells)
            .chain(&layout.bridge_cells)
            .chain(layout.objects.iter().map(|o| &o.cell))
        {
            if c.iter()
                .enumerate()
                .any(|(axis, v)| !v.is_finite() || *v < 0. || *v >= [n, h][axis])
            {
                return Err(err("layout coordinate outside grid"));
            }
        }
        if layout.base_cells.len() != 8 {
            return Err(err("8-player template requires eight base cells"));
        }
        if layout.ore_cells.len() != layout.ore_angles.len() {
            return Err(err("ore positions and angles differ in length"));
        }
        for ramp in &layout.ramps {
            let x = ramp["x"].as_f64().ok_or_else(|| err("ramp missing x"))?;
            let y = ramp["y"].as_f64().ok_or_else(|| err("ramp missing y"))?;
            if x < 0.
                || y < 0.
                || x >= n
                || y >= h
                || number(ramp, "w", 0.) <= 0.
                || number(ramp, "h", 0.) <= 0.
            {
                return Err(err("invalid ramp center or size"));
            }
            ramp_axes(ramp)?;
            ramp_width(ramp)?;
        }
        if !p.flag("TERRAIN_ONLY") {
            for (i, base) in layout.base_cells.iter().enumerate() {
                let name = if i < 6 {
                    format!("Player_{}_Start", i + 1)
                } else {
                    format!("Player_{}_Start (imported)", i + 1)
                };
                let object = layout
                    .objects
                    .iter()
                    .find(|o| {
                        crate::formats::map::prop(&o.props, "waypointName")
                            .and_then(crate::formats::map::Value::text)
                            == Some(name.as_str())
                    })
                    .ok_or_else(|| err(format!("layout missing start object {name}")))?;
                if (object.cell[0] - base[0]).hypot(object.cell[1] - base[1]) > 0.001 {
                    return Err(err(format!(
                        "{name}: base_cells and object coordinates differ; update both"
                    )));
                }
            }
            let ores = layout
                .objects
                .iter()
                .filter(|o| o.type_name == "OreNode")
                .collect::<Vec<_>>();
            if ores.len() != layout.ore_cells.len() {
                return Err(err("ore_cells and OreNode object counts differ"));
            }
            for ore in &layout.ore_cells {
                if ores
                    .iter()
                    .filter(|o| (o.cell[0] - ore[0]).hypot(o.cell[1] - ore[1]) <= 0.001)
                    .count()
                    != 1
                {
                    return Err(err(
                        "ore_cells and OreNode object coordinates differ; update both",
                    ));
                }
            }
        }
        crate::terrain::ramps::configure_layout(p, &mut layout)?;
        Ok(layout)
    }
}
