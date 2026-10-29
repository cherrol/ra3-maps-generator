//! Read-only spatial inventory of the actual saved map, in native grid/world frames.
mod export;
mod frame;
mod points;
mod raster;
use crate::{
    Result, err,
    formats::archive,
    formats::map::{self, BlendTiles, HeightMap, MapFile, Value, prop},
    pipeline::project::{self, Project},
    terrain::{Layout, flip},
};
use export::{csv_row, overview, write_points, write_readme};
pub use frame::{Frame, prepare};
use points::points;
#[cfg(test)]
use points::{category, neutral, owner_seat};
#[cfg(test)]
use raster::{Edge, trace};
use raster::{Layer, Raster, boundaries, classify, layer_regions, terrain_runs};
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, fs};

/// Preserve native ordering and import generated marker waypoints omitted by older caches.
pub fn native_layout(p: &Project, map: &MapFile, saved: &Layout) -> Result<(Layout, Json)> {
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    let mut used = std::collections::BTreeSet::new();
    let mut ids = Vec::new();
    for (i, a) in saved.objects.iter().enumerate() {
        let matching = objects
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                let c = frame.grid([b.position[0] as f64, b.position[1] as f64]);
                a.type_name == b.type_name
                    && a.angle == b.angle
                    && (a.cell[0] - c[0]).hypot(a.cell[1] - c[1]) < 0.001
                    && ["uniqueID", "waypointName"].iter().all(|key| {
                        prop(&a.props, key).and_then(Value::text)
                            == prop(&b.props, key).and_then(Value::text)
                    })
            })
            .map(|(j, _)| j)
            .collect::<Vec<_>>();
        if matching.len() != 1 || !used.insert(matching[0]) {
            return Err(err(
                "saved coordinate layout is stale or ambiguous relative to native objects",
            ));
        }
        ids.push(json!({"old_preview_id":i+1,"preview_id":matching[0]+1,"kind":"object"}));
    }
    let rules = crate::gameplay::scripts::load_rules(p)?;
    for (i, o) in objects
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
    {
        let name = prop(&o.props, "waypointName")
            .and_then(Value::text)
            .unwrap_or("");
        if !o.type_name.contains("Waypoint")
            || !rules.spawn_waypoints.iter().any(|s| s.name == name)
        {
            return Err(err(format!(
                "native object {} is absent from saved coordinate layout and is not a registered generated waypoint",
                i + 1
            )));
        }
        ids.push(json!({"old_preview_id":null,"preview_id":i+1,"kind":"generated_waypoint"}));
    }
    for i in 0..saved.ramps.len() {
        ids.push(json!({"old_preview_id":saved.objects.len()+i+1,"preview_id":objects.len()+i+1,"kind":"ramp"}));
    }
    let mut layout = saved.clone();
    layout.objects = objects
        .iter()
        .map(|o| crate::terrain::ObjectSpec {
            cell: frame.grid([o.position[0] as f64, o.position[1] as f64]),
            version: o.version,
            angle: o.angle,
            road_type: o.road_type,
            type_name: o.type_name.clone(),
            props: o.props.clone(),
        })
        .collect();
    Ok((
        layout,
        json!({"schema_version":1,"map":p.name,"object_id_basis":"native object order","entries":ids}),
    ))
}

pub fn generate(p: &Project) -> Result<Json> {
    let source = p.assets.join(format!("{}.map", p.name));
    let bytes = fs::read(&source)?;
    let map = MapFile::load_bytes(&bytes)?;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    let frame = Frame::from_heightmap(p, &hm)?;
    let layout_path = p.source().join("layout.json");
    let saved: Layout = serde_json::from_slice(&fs::read(&layout_path)?)?;
    let (layout, mut id_map) = native_layout(p, &map, &saved)?;
    let mut paths = match fs::read_dir(p.map_dir().join("histories")) {
        Ok(entries) => entries
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    paths.sort();
    let mut previous = Vec::<Json>::new();
    for path in paths {
        if path.is_dir()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| project::parse_version(name).is_ok())
        {
            let snapshot = path.join("coordinate-id-map.json");
            if snapshot.is_file() {
                previous.push(serde_json::from_slice(&fs::read(snapshot)?)?);
            }
        }
    }
    id_map["previous_versions"] = json!(previous);
    let actual_objects = map::objects(map.get("ObjectsList")?, &map.table)?;
    if layout.objects.len() != actual_objects.len() {
        return Err(err(
            "coordinate report requires complete current source layout",
        ));
    }
    for (a, b) in layout.objects.iter().zip(&actual_objects) {
        let c = frame.grid([b.position[0] as f64, b.position[1] as f64]);
        if a.type_name != b.type_name || (a.cell[0] - c[0]).hypot(a.cell[1] - c[1]) > 0.001 {
            return Err(err(
                "coordinate source layout differs from current native map",
            ));
        }
    }
    let raw = flip(&hm.elevations, hm.width);
    let z: Vec<_> = raw.iter().map(|v| *v as f64 * frame.z_scale).collect();
    let water = p.number("WATER_Z", 200.);
    let peak = water + p.number("PEAK_IMPASSABLE", 235.);
    let mut levels = p.cfg["TERRAIN"]["levels"]
        .as_array()
        .ok_or_else(|| err("coordinate report requires terrain levels"))?
        .iter()
        .filter_map(Json::as_f64)
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    if levels.is_empty() {
        return Err(err("terrain layer list is empty"));
    }
    let palette = [
        [85, 147, 79],
        [156, 177, 92],
        [191, 160, 103],
        [195, 196, 193],
    ];
    let mut layers = vec![Layer {
        id: 0,
        label: "水域".into(),
        nominal_height: None,
        color: [55, 119, 164],
        cells: 0,
        playable_cells: 0,
        flat_cells: 0,
        slope_cells: 0,
    }];
    for (i, h) in levels.iter().enumerate() {
        layers.push(Layer {
            id: i + 1,
            label: format!("第{}层 · {}", i + 1, h),
            nominal_height: Some(*h),
            color: palette[i % palette.len()],
            cells: 0,
            playable_cells: 0,
            flat_cells: 0,
            slope_cells: 0,
        });
    }
    layers.push(Layer {
        id: layers.len(),
        label: format!("高山 · >{peak}"),
        nominal_height: None,
        color: [223, 225, 227],
        cells: 0,
        playable_cells: 0,
        flat_cells: 0,
        slope_cells: 0,
    });
    let imp = flip(&bt.impassability, hm.width);
    let build = flip(&bt.buildability, hm.width);
    let mut class = Vec::new();
    let mut flags = Vec::new();
    for i in 0..z.len() {
        let layer = classify(z[i], water, peak, &levels);
        let x = i % frame.width;
        let y = i / frame.width;
        let gx = (z[y * frame.width + (x + 1).min(frame.width - 1)]
            - z[y * frame.width + x.saturating_sub(1)])
            * 0.5;
        let gy = (z[(y + 1).min(frame.height - 1) * frame.width + x]
            - z[y.saturating_sub(1) * frame.width + x])
            * 0.5;
        let sloped = gx.hypot(gy) > 1.5
            || layers[layer]
                .nominal_height
                .is_some_and(|h| (z[i] - h).abs() > 2.);
        let flat = !sloped && layer > 0 && layer <= levels.len();
        class.push(layer);
        flags.push(
            u8::from(imp[i])
                | u8::from(build[i]) << 1
                | u8::from(sloped) << 2
                | u8::from(flat) << 3,
        );
    }
    let mut raster = Raster {
        frame,
        raw,
        z,
        class,
        regions: vec![0; hm.width * hm.height],
        flags,
    };
    let regions = layer_regions(&mut raster, &mut layers);
    let boundaries = boundaries(&raster, &regions, p.number("SLOPE_IMPASSABLE", 24.));
    let points = points(&map, &raster, &layout)?;
    let rows = terrain_runs(&raster);
    let mut categories = BTreeMap::<String, usize>::new();
    for point in &points {
        *categories.entry(point.category.clone()).or_default() += 1;
    }
    let mut boundary_counts = BTreeMap::from([
        ("shoreline".to_string(), 0usize),
        ("cliff".to_string(), 0),
        ("slope_transition".to_string(), 0),
    ]);
    let mut boundary_lengths = boundary_counts.clone();
    for b in &boundaries {
        *boundary_counts.entry(b.kind.clone()).or_default() += 1;
        *boundary_lengths.entry(b.kind.clone()).or_default() += b.segments;
    }
    let statistics = json!({"map_objects":actual_objects.len(),"ramps":layout.ramps.len(),"script_building_candidates":points.iter().filter(|p|p.conditional).count(),"off_grid_objects":points.iter().filter(|p|p.category!="斜坡"&&!p.conditional&&!p.in_grid).count(),"buildings":points.iter().filter(|p|p.category=="建筑").count(),"neutral_buildings":points.iter().filter(|p|p.category=="建筑"&&p.neutral).count(),"resource_points":points.iter().filter(|p|p.category=="资源").count(),"regions":regions.len(),"boundary_paths":boundaries.len(),"boundary_counts":boundary_counts,"boundary_unit_segments":boundary_lengths,"water_cells":layers[0].cells,"categories":categories,"terrain_grid_points":hm.width*hm.height,"terrain_runs":rows.iter().map(Vec::len).sum::<usize>()});
    let mut coordinate_system = raster.frame.write_system(p, &p.source())?;
    let report_fields = json!({"layer_assignment":"nearest configured nominal land height; slopes are flagged separately and ramp properties give actual endpoint levels","flat_rule":"within 2 height units of nominal and central gradient <=1.5","boundary_rule":"complete shared cell edges at half-grid coordinates; no invented shoreline outside terrain","rle_fields":["x_begin","x_end_inclusive","height_raw","layer_id","region_id","flags"],"flags":{"1":"impassable","2":"unbuildable","4":"sloped or transition","8":"flat platform"},"object_z":"raw native object Z; surface_z is bilinear terrain height, null outside raster"});
    coordinate_system
        .as_object_mut()
        .unwrap()
        .extend(report_fields.as_object().unwrap().clone());
    let data = json!({"schema_version":1,"map_id":p.id,"map_name":p.name,"version":p.version,"source_map":source.strip_prefix(&p.root).unwrap_or(&source),"source_crc32":crc32fast::hash(&bytes),"frame":raster.frame,"coordinate_system":coordinate_system,"statistics":statistics,"layers":layers,"regions":regions,"boundaries":boundaries,"points":points,"terrain_rows":rows});
    let dir = p.assets.join("coordinates");
    fs::create_dir_all(&dir)?;
    raster.frame.write_system(p, &dir)?;
    project::write_json(&dir.join("coordinates.json"), &data)?;
    project::write_json(&dir.join("coordinate-id-map.json"), &id_map)?;
    write_points(&dir.join("objects.csv"), &points, false)?;
    write_points(&dir.join("script-buildings.csv"), &points, true)?;
    let mut csv = String::from(
        "\u{feff}layer_id,label,nominal_height,full_grid_cells,playable_cells,flat_cells,slope_cells\n",
    );
    for l in &layers {
        csv_row(
            &mut csv,
            &[
                l.id.to_string(),
                l.label.clone(),
                l.nominal_height.map(|v| v.to_string()).unwrap_or_default(),
                l.cells.to_string(),
                l.playable_cells.to_string(),
                l.flat_cells.to_string(),
                l.slope_cells.to_string(),
            ],
        );
    }
    fs::write(dir.join("layers.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}region_id,layer_id,cells,playable_cells,flat_cells,slope_cells,min_x,min_y,max_x,max_y,centroid_x,centroid_y,seed_x,seed_y,min_height,max_height\n",
    );
    for r in &regions {
        csv_row(
            &mut csv,
            &[
                r.id.to_string(),
                r.layer.to_string(),
                r.cells.to_string(),
                r.playable_cells.to_string(),
                r.flat_cells.to_string(),
                r.slope_cells.to_string(),
                r.bounds[0].to_string(),
                r.bounds[1].to_string(),
                r.bounds[2].to_string(),
                r.bounds[3].to_string(),
                r.centroid[0].to_string(),
                r.centroid[1].to_string(),
                r.seed[0].to_string(),
                r.seed[1].to_string(),
                r.height_range[0].to_string(),
                r.height_range[1].to_string(),
            ],
        );
    }
    fs::write(dir.join("regions.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}boundary_id,kind,region_a,region_b,layer_a,layer_b,point_index,x,y,world_x,world_y,closed\n",
    );
    for b in &boundaries {
        for (i, pt) in b.points.iter().enumerate() {
            let world = raster.frame.world(*pt);
            csv_row(
                &mut csv,
                &[
                    b.id.to_string(),
                    b.kind.clone(),
                    b.regions[0].to_string(),
                    b.regions[1].to_string(),
                    b.layers[0].to_string(),
                    b.layers[1].to_string(),
                    i.to_string(),
                    pt[0].to_string(),
                    pt[1].to_string(),
                    world[0].to_string(),
                    world[1].to_string(),
                    b.closed.to_string(),
                ],
            );
        }
    }
    fs::write(dir.join("boundary-points.csv"), csv)?;
    let mut csv = String::from(
        "\u{feff}y,x_begin,x_end_inclusive,height_raw,height,layer_id,region_id,flags,world_x_begin,world_x_end,world_y\n",
    );
    for (y, row) in rows.iter().enumerate() {
        for run in row {
            let a = raster.frame.world([run[0] as f64, y as f64]);
            let b = raster.frame.world([run[1] as f64, y as f64]);
            csv_row(
                &mut csv,
                &[
                    y.to_string(),
                    run[0].to_string(),
                    run[1].to_string(),
                    run[2].to_string(),
                    (run[2] as f64 * raster.frame.z_scale).to_string(),
                    run[3].to_string(),
                    run[4].to_string(),
                    run[5].to_string(),
                    a[0].to_string(),
                    b[0].to_string(),
                    a[1].to_string(),
                ],
            );
        }
    }
    fs::write(dir.join("terrain-runs.csv"), csv)?;
    let encoded = serde_json::to_string(&data)?.replace('<', "\\u003c");
    fs::write(
        dir.join("index.html"),
        include_str!("view.html").replace("__COORDINATE_DATA__", &encoded),
    )?;
    overview(
        &dir.join("overview.png"),
        &raster,
        &layers,
        &boundaries,
        &points,
    )?;
    write_readme(&dir, p, &data)?;
    let mut files = Vec::new();
    for f in [
        "index.html",
        "coordinates.json",
        "coordinate-system.json",
        "coordinate-id-map.json",
        "objects.csv",
        "script-buildings.csv",
        "layers.csv",
        "regions.csv",
        "boundary-points.csv",
        "terrain-runs.csv",
        "overview.png",
        "README.md",
    ] {
        files.push((format!("coordinates/{f}"), fs::read(dir.join(f))?));
    }
    fs::write(p.assets.join("coordinates.zip"), archive::zip(&files)?)?;
    if fs::read(&source)? != bytes {
        return Err(err("native map changed during coordinate generation"));
    }
    println!("坐标报告：{}；{}", dir.display(), statistics);
    Ok(statistics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(class: Vec<usize>, w: usize) -> (Raster, Vec<Layer>) {
        let h = class.len() / w;
        let z: Vec<_> = class.iter().map(|i| [190., 212., 280.][*i]).collect();
        let layers = (0..3)
            .map(|id| Layer {
                id,
                label: id.to_string(),
                nominal_height: None,
                color: [0; 3],
                cells: 0,
                playable_cells: 0,
                flat_cells: 0,
                slope_cells: 0,
            })
            .collect();
        (
            Raster {
                frame: Frame {
                    width: w,
                    height: h,
                    border: 0,
                    cell_size: 10.,
                    z_scale: 1.,
                },
                raw: z.iter().map(|v| *v as u16).collect(),
                z,
                regions: vec![0; class.len()],
                flags: vec![8; class.len()],
                class,
            },
            layers,
        )
    }

    #[test]
    fn land_edges_do_not_invent_shorelines_and_diagonal_islands_stay_separate() {
        let (mut r, mut layers) = fixture(vec![1, 2, 2, 1], 2);
        let regions = layer_regions(&mut r, &mut layers);
        assert_eq!(regions.len(), 4); // Diagonal contact is not a four-neighbor connection.
        let paths = boundaries(&r, &regions, 24.);
        assert_eq!(paths.iter().map(|p| p.segments).sum::<usize>(), 4);
        assert!(paths.iter().all(|p| p.kind == "cliff"));
        assert_eq!(regions.iter().map(|r| r.cells).sum::<usize>(), 4);
    }

    #[test]
    fn shorelines_form_closed_complete_unit_edge_loops() {
        let (mut r, mut layers) = fixture(vec![1, 1, 1, 1, 0, 1, 1, 1, 1], 3);
        let regions = layer_regions(&mut r, &mut layers);
        let paths = boundaries(&r, &regions, 24.);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].kind, "shoreline");
        assert!(paths[0].closed);
        assert_eq!(paths[0].segments, 4);
        assert_eq!(paths[0].points.first(), paths[0].points.last());
        for pair in paths[0].points.windows(2) {
            assert_eq!(
                (pair[0][0] - pair[1][0]).abs() + (pair[0][1] - pair[1][1]).abs(),
                1.
            );
        }
    }

    #[test]
    fn terrain_runs_preserve_every_field_and_row_boundary() {
        let (mut r, mut layers) = fixture(vec![1, 1, 1, 1, 1, 1, 2, 2], 4);
        r.flags[1] = 4;
        r.raw[2] += 1;
        layer_regions(&mut r, &mut layers);
        let runs = terrain_runs(&r);
        let mut decoded = Vec::new();
        for row in runs {
            assert_eq!(row.first().unwrap()[0], 0);
            assert_eq!(row.last().unwrap()[1], 3);
            for pair in row.windows(2) {
                assert_eq!(pair[0][1] + 1, pair[1][0]);
            }
            for run in row {
                for _ in run[0]..=run[1] {
                    decoded.push([run[2], run[3], run[4], run[5]]);
                }
            }
        }
        for (i, value) in decoded.iter().enumerate() {
            assert_eq!(
                *value,
                [
                    r.raw[i] as usize,
                    r.class[i],
                    r.regions[i],
                    r.flags[i] as usize
                ]
            );
        }
        assert_eq!(flip(&[1, 2, 3, 4, 5, 6], 3), [4, 5, 6, 1, 2, 3]);
    }
    #[test]
    fn grid_world_roundtrip_includes_offmap_coordinates() {
        let f = Frame {
            width: 840,
            height: 840,
            border: 20,
            cell_size: 10.,
            z_scale: 0.0390625,
        };
        for p in [[20., 819.], [781., 411.], [801.8, 871.2], [-1., 15.]] {
            let q = f.grid(f.world(p));
            assert!((p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9);
        }
        assert_eq!(f.world([20., 819.]), [0., 0.]);
        assert!(!f.inside([801., 871.]));
    }
    #[test]
    fn fixed_ai_is_distinguished_from_neutral_buildings() {
        assert_eq!(owner_seat("PlyrCivilian/teamPlyrCivilian"), Some(7));
        assert!(!neutral("PlyrCivilian/teamPlyrCivilian"));
        assert!(neutral("SkirmishCivilian/teamSkirmishCivilian"));
        assert_eq!(
            category("HospitalTechStructure", ""),
            ("建筑", "中立科技建筑")
        );
    }
    #[test]
    fn boundary_traces_cover_every_edge_without_fake_connections() {
        let edges = vec![
            Edge {
                a: (1, 1),
                b: (3, 1),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 1),
                b: (3, 3),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 3),
                b: (1, 3),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (1, 3),
                b: (1, 1),
                playable: true,
                heights: [212., 280.],
            },
            Edge {
                a: (3, 1),
                b: (5, 1),
                playable: true,
                heights: [212., 280.],
            },
        ];
        let paths = trace(&edges);
        let all = paths.iter().flatten().copied().collect::<Vec<_>>();
        assert_eq!(all.len(), 5);
        assert_eq!(
            all.iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            5
        );
        assert!(paths.len() > 1);
    }
}
