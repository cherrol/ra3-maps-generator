use ra3maps::{
    coordinates::Frame,
    formats::map::{self, BlendTiles, HeightMap, MapFile, MapObject},
    pipeline::project::Project,
    terrain,
};
use std::path::Path;

// Conservative planning envelope for the ore approach and refinery placement:
// 5 cells behind the ore, 48 ahead, and 18 to either side. This is a clearance
// reservation, not a claim to have extracted any faction's exact game mesh.
fn refinery_clear(
    object: &MapObject,
    heading: f32,
    frame: &Frame,
    heights: &[u16],
    imp: &[bool],
    build: &[bool],
) -> bool {
    let point = frame.grid([object.position[0] as f64, object.position[1] as f64]);
    let angle = -(heading as f64);
    let (uy, ux) = angle.sin_cos();
    let center = heights[point[1].round() as usize * frame.width + point[0].round() as usize]
        as f64
        * frame.z_scale;
    for y in (point[1] - 53.).floor() as isize..=(point[1] + 53.).ceil() as isize {
        for x in (point[0] - 53.).floor() as isize..=(point[0] + 53.).ceil() as isize {
            let dx = x as f64 - point[0];
            let dy = y as f64 - point[1];
            let forward = dx * ux + dy * uy;
            let lateral = -dx * uy + dy * ux;
            // Include every grid cell touched by the rotated rectangle.
            if !(-5.75..=48.75).contains(&forward) || lateral.abs() > 18.75 {
                continue;
            }
            if !frame.playable([x as f64, y as f64]) {
                return false;
            }
            let i = y as usize * frame.width + x as usize;
            let z = heights[i] as f64 * frame.z_scale;
            if imp[i] || build[i] || (z - center).abs() > 12. {
                return false;
            }
        }
    }
    true
}

#[test]
fn reported_ores_leave_room_for_refineries_beyond_the_old_twenty_cell_disk() {
    let p = Project::load(Path::new(env!("CARGO_MANIFEST_DIR")), "陕西").unwrap();
    let m = MapFile::load(p.assets.join(format!("{}.map", p.name))).unwrap();
    let frame = Frame::from_project(&p);
    let hm = HeightMap::parse(m.get("HeightMapData").unwrap()).unwrap();
    let bt = BlendTiles::parse(m.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    let heights = terrain::flip(&hm.elevations, hm.width);
    let imp = terrain::flip(&bt.impassability, hm.width);
    let build = terrain::flip(&bt.buildability, hm.width);
    let objects = map::objects(m.get("ObjectsList").unwrap(), &m.table).unwrap();
    let historical = p.release_assets("[CHAI]千壑横原T0.7.6").unwrap();
    let historical = MapFile::load(historical.join("[CHAI]千壑横原T0.7.6.map")).unwrap();
    let historical_objects =
        map::objects(historical.get("ObjectsList").unwrap(), &historical.table).unwrap();
    let historical_hm = HeightMap::parse(historical.get("HeightMapData").unwrap()).unwrap();
    let historical_bt = BlendTiles::parse(
        historical.get("BlendTileData").unwrap(),
        historical_hm.width,
        historical_hm.height,
    )
    .unwrap();
    let historical_heights = terrain::flip(&historical_hm.elevations, historical_hm.width);
    let historical_imp = terrain::flip(&historical_bt.impassability, historical_hm.width);
    let historical_build = terrain::flip(&historical_bt.buildability, historical_hm.width);
    for (identity, old_heading) in [
        ("OreNode 324", -std::f32::consts::FRAC_PI_4),
        ("OreNode 319", std::f32::consts::FRAC_PI_4),
    ] {
        let ore = objects
            .iter()
            .find(|o| {
                o.type_name == "OreNode"
                    && map::prop(&o.props, "uniqueID").and_then(map::Value::text) == Some(identity)
            })
            .expect("reported ore identity must be preserved");
        let old_ore = historical_objects
            .iter()
            .find(|o| map::prop(&o.props, "uniqueID").and_then(map::Value::text) == Some(identity))
            .unwrap();
        assert!(
            !refinery_clear(
                old_ore,
                old_heading,
                &frame,
                &historical_heights,
                &historical_imp,
                &historical_build
            ),
            "the in-game reported heading must reproduce the cliff collision: {identity}"
        );
        assert!(
            refinery_clear(ore, ore.angle, &frame, &heights, &imp, &build),
            "the complete refinery reservation must fit: {identity}"
        );
    }
}
