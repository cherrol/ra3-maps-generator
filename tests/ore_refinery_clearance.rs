use ra3maps::{
    coordinates::Frame,
    formats::map::{self, BlendTiles, HeightMap, MapObject},
    terrain,
};
mod support;

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
fn all_current_ores_have_complete_refinery_foundations_beyond_the_ore_disk() {
    let (p, native) = support::current_map();
    let frame = Frame::from_project(&p);
    let hm = HeightMap::parse(native.get("HeightMapData").unwrap()).unwrap();
    let bt = BlendTiles::parse(native.get("BlendTileData").unwrap(), hm.width, hm.height).unwrap();
    let heights = terrain::flip(&hm.elevations, hm.width);
    let imp = terrain::flip(&bt.impassability, hm.width);
    let build = terrain::flip(&bt.buildability, hm.width);
    let objects = map::objects(native.get("ObjectsList").unwrap(), &native.table).unwrap();
    let ores: Vec<_> = objects
        .iter()
        .filter(|o| o.type_name == "OreNode")
        .collect();
    assert_eq!(ores.len(), 48);
    for ore in &ores {
        assert!(
            refinery_clear(ore, ore.angle, &frame, &heights, &imp, &build),
            "{:?}",
            ore.position
        );
    }
    let ore = ores[0];
    let c = frame.grid([f64::from(ore.position[0]), f64::from(ore.position[1])]);
    let x = (c[0] + 40. * f64::from(ore.angle).cos()).round() as usize;
    let y = (c[1] - 40. * f64::from(ore.angle).sin()).round() as usize;
    assert!((x as f64 - c[0]).hypot(y as f64 - c[1]) > 20.);
    let mut damaged = heights;
    damaged[y * frame.width + x] += 1000;
    assert!(!refinery_clear(
        ore, ore.angle, &frame, &damaged, &imp, &build
    ));
}
