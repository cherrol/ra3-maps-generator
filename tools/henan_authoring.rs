//! Rebuild Henan's authored blueprint from the approved, unscaled preview.
//! Placement remains explicit in authoring/layout.json; this does not search it.
use ra3maps::{
    Result,
    formats::{archive, npy},
    pipeline::project::{self, Project},
    terrain::{self, Layout},
};
use serde_json::json;
use std::{fs, path::Path};

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = Project::load(root, "河南")?;
    if std::env::args().any(|a| a == "--audit") {
        return audit(&p);
    }
    if std::env::args().any(|a| a == "--check-layout") {
        return check_layout(&p);
    }
    if p.cfg["COORDINATE_PIPELINE"].is_object() {
        return Err(ra3maps::err(
            "Henan is frozen; use build --map 河南. Initial blueprint inputs remain in histories/v1/ and histories/T0.0.1/blueprint-config.json.",
        ));
    }
    let source = p.path("data/terrain-preview/terrain.npz")?;
    let source_bytes = fs::read(source)?;
    let input = archive::unzip(&source_bytes)?;
    let heights = npy::Npy::parse(&input["game_heights.npy"])?;
    let frame = ra3maps::coordinates::Frame::from_project(&p);
    assert_eq!(heights.shape, [frame.height, frame.width]);
    let original = heights.numbers()?;
    let wet: Vec<_> = npy::Npy::parse(&input["water_mask.npy"])?
        .numbers()?
        .iter()
        .map(|v| *v != 0.)
        .collect();
    let to_water = terrain::distance(&wet, frame.width);
    let to_land = terrain::distance(&wet.iter().map(|v| !v).collect::<Vec<_>>(), frame.width);
    let smooth = |t: f64| {
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t)
    };
    let water = p.number("WATER_Z", 200.);
    let mut z = original.clone();
    for i in 0..z.len() {
        z[i] = if wet[i] {
            water - 1. - (water - 1. - 130.) * smooth((to_land[i] - 1.) / 18.)
        } else {
            let width = ((original[i] - water) / 3.).max(8.);
            water + 1. + (original[i] - water - 1.) * smooth((to_water[i] - 1.) / width)
        };
    }
    let layout = Layout::load(&p)?;
    terrain::carve(&mut z, frame.width, &layout.ramps, &p)?;
    let raw: Vec<_> = z
        .iter()
        .map(|v| (v / frame.z_scale).round_ties_even() as u16)
        .collect();
    // Shoreline interpolation must preserve every accepted water/land sample.
    assert!(
        raw.iter()
            .zip(&wet)
            .all(|(&v, &w)| (f64::from(v) * frame.z_scale < water) == w)
    );
    let mut output: Vec<(String, Vec<u8>)> = vec![
        (
            "base_z.npy".into(),
            npy::encode(
                "<u2",
                &heights.shape,
                &raw.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
            ),
        ),
        (
            "z_scale.npy".into(),
            npy::encode("<f8", &[], &frame.z_scale.to_le_bytes()),
        ),
        (
            "approved_preview_heights.npy".into(),
            input["game_heights.npy"].clone(),
        ),
        (
            "approved_water_mask.npy".into(),
            input["water_mask.npy"].clone(),
        ),
    ];
    output.sort_by(|a, b| a.0.cmp(&b.0));
    let bytes = archive::zip(&output)?;
    fs::write(p.path(p.string("LAYERS", "layers.npz"))?, &bytes)?;
    fs::write(p.path("layers.npz")?, &bytes)?;
    fs::copy(
        p.path("authoring/layout.json")?,
        p.path("histories/v1/layout.json")?,
    )?;
    project::write_json(
        &p.path("authoring/terrain-design.json")?,
        &json!({
            "schema_version":1,"source":"data/terrain-preview/terrain.npz",
            "source_crc32":crc32fast::hash(&source_bytes),"blueprint_crc32":crc32fast::hash(&bytes),
            "frame":frame,"playable_cells":[800,768],"resampled":false,
            "shorelines":"continuous full-perimeter bidirectional transitions; accepted water mask unchanged",
            "shore_water_depth":130,"shore_water_width_cells":18,
            "shore_land_width_cells":"max((approved_height-200)/3, 8)",
            "ramp_count":layout.ramps.len(),"ramps":"explicit authored positions; mountain profile, full width 24, span 40",
            "changed_height_samples":raw.iter().zip(&original).filter(|(v,z)| (f64::from(**v)*frame.z_scale-**z).abs()>frame.z_scale*0.5).count(),
            "command":"cargo run --release --locked --example henan-authoring"
        }),
    )?;
    println!(
        "河南 blueprint: {}×{}, {} authored ramps",
        frame.width,
        frame.height,
        layout.ramps.len()
    );
    Ok(())
}

fn positive_area(x: f64, y: f64, center: [f64; 2], normal: [f64; 2]) -> f64 {
    let value = |p: [f64; 2]| normal[0] * (p[0] - center[0]) + normal[1] * (p[1] - center[1]);
    let corners = [
        [x - 0.5, y - 0.5],
        [x + 0.5, y - 0.5],
        [x + 0.5, y + 0.5],
        [x - 0.5, y + 0.5],
    ];
    let mut out = Vec::new();
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let va = value(a);
        let vb = value(b);
        if va >= 0. {
            out.push(a);
        }
        if (va >= 0.) != (vb >= 0.) {
            let t = va / (va - vb);
            out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    if out.len() < 3 {
        return 0.;
    }
    (0..out.len())
        .map(|i| {
            let a = out[i];
            let b = out[(i + 1) % out.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        .abs()
        * 0.5
}

fn check_layout(p: &Project) -> Result<()> {
    use ra3maps::formats::map::{self, BlendTiles, HeightMap, MapFile};
    let t = terrain::Terrain::generate(p)?;
    let s = terrain::surface::Surface::generate(&t, p)?;
    let mut map = MapFile::load(p.root.join("templates/8p_template.map"))?;
    let hm = HeightMap::parse(map.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(map.get("BlendTileData")?, hm.width, hm.height)?;
    map.get_mut("HeightMapData")?.payload = t.heightmap(p).encode()?;
    map.get_mut("BlendTileData")?.payload = s.encode(t.n, &bt.blend_descriptions)?;
    let objects: Vec<_> = t.layout.objects.iter().map(|o| o.object(p)).collect();
    for o in &objects {
        for pr in &o.props {
            map.id(&pr.key)?;
        }
    }
    map.get_mut("ObjectsList")?.payload = map::write_objects(&objects, &map)?;
    let spawns = terrain::spawn_placement::report(
        p,
        &map,
        &t.layout,
        &terrain::spawn_placement::policy(p)?,
    )?;
    let clearance = terrain::ramp_clearance::validate(p, &map, &t.layout);
    let policy = serde_json::from_slice(&fs::read(
        p.root.join("templates/rules/coordinate-pipeline.json"),
    )?)?;
    let scene = terrain::spatial::Scene::load(p, &map, policy)?;
    let mut issues = scene.inspect(&t.layout)?;
    let (ramps, ramp_issues) = scene.ramp_profiles(&t.layout)?;
    issues.extend(ramp_issues);
    let (routes, route_issues) = scene.routes(&map)?;
    issues.extend(route_issues);
    project::write_json(
        &p.path("authoring/layout-review.json")?,
        &json!({"scope":"blueprint layout preflight; excludes generated gameplay waypoints and packaging", "spawn_placement":spawns,"ramp_clearance":clearance.as_ref().map_err(|e|e.to_string()),"issues":issues,"ramp_profiles":ramps,"routes":routes}),
    )?;
    println!(
        "layout review: {} spawn findings, {} spatial findings, ramp clearance {}",
        spawns["findings"].as_array().unwrap().len(),
        issues.len(),
        clearance.is_ok()
    );
    Ok(())
}

fn audit(p: &Project) -> Result<()> {
    use ra3maps::formats::map::{BlendTiles, HeightMap, MapFile};
    let native = MapFile::load(p.assets.join(format!("{}.map", p.name)))?;
    let hm = HeightMap::parse(native.get("HeightMapData")?)?;
    let bt = BlendTiles::parse(native.get("BlendTileData")?, hm.width, hm.height)?;
    let frame = ra3maps::coordinates::Frame::from_heightmap(p, &hm)?;
    let z: Vec<_> = terrain::flip(&hm.elevations, hm.width)
        .iter()
        .map(|v| f64::from(*v) * frame.z_scale)
        .collect();
    let imp = terrain::flip(&bt.impassability, hm.width);
    let center = [
        (frame.width - 1) as f64 / 2.,
        (frame.height - 1) as f64 / 2.,
    ];
    let user_layout = p.string("TEAM_AREA_LAYOUT", "") == "south_north_user_marked";
    let normal = if user_layout { [0., 1.] } else { [0.56, 1.] };
    let mut area = [[0_f64; 2]; 2];
    let mut shore_edges = 0;
    let mut shore_findings = 0;
    for y in frame.border..frame.height - frame.border {
        for x in frame.border..frame.width - frame.border {
            let i = y * frame.width + x;
            let wet = z[i] < 200.;
            let positive = positive_area(x as f64, y as f64, center, normal);
            area[wet as usize][0] += positive;
            area[wet as usize][1] += 1. - positive;
            for j in [i + 1, i + frame.width] {
                if (z[i] < 200.) != (z[j] < 200.) {
                    shore_edges += 1;
                    shore_findings += usize::from(imp[i] || imp[j] || (z[i] - z[j]).abs() > 12.);
                }
            }
        }
    }
    let ratios = area.map(|a| (a[0] - a[1]).abs() / a[0].min(a[1]));
    let layout: Layout = serde_json::from_slice(&fs::read(p.source().join("layout.json"))?)?;
    assert!(
        layout
            .base_cells
            .iter()
            .enumerate()
            .all(
                |(i, b)| (normal[0] * (b[0] - center[0]) + normal[1] * (b[1] - center[1]) > 0.)
                    == (i < 4)
            )
    );
    let balance = json!({"schema_version":1,"frame":frame,"orientation":if user_layout {"south_north"} else {"southeast_northwest"},
        "dividing_line":if user_layout {"y - 403.5 = 0; playable geometric center"} else {"0.56 * (x - 419.5) + (y - 403.5) = 0; playable geometric center"},
        "positive_side":if user_layout {"P1-P4 south"} else {"P1-P4 southeast"},"negative_side":if user_layout {"P5-P8 north"} else {"P5-P8 northwest"},
        "explicit_user_positions_override_default_area_rule":user_layout,
        "method":"exact polygon clipping of unit terrain cells centered on full-grid samples; excludes border",
        "land_cells_by_team":area[0],"water_cells_by_team":area[1],"land_difference_ratio":ratios[0],"water_difference_ratio":ratios[1],
        "maximum_difference_ratio":0.2,"passed":ratios.iter().all(|v| *v<=0.2)});
    project::write_json(&p.assets.join("team-area-balance.json"), &balance)?;
    project::write_json(&p.path("authoring/team-area-balance.json")?, &balance)?;
    let policy = serde_json::from_slice(&fs::read(
        p.root.join("templates/rules/coordinate-pipeline.json"),
    )?)?;
    let scene = terrain::spatial::Scene::load(p, &native, policy)?;
    let mut issues = scene.inspect(&layout)?;
    let (profiles, ramps) = scene.ramp_profiles(&layout)?;
    issues.extend(ramps);
    let (routes, route_issues) = scene.routes(&native)?;
    issues.extend(route_issues);
    project::write_json(
        &p.assets.join("authoring-audit.json"),
        &json!({"schema_version":1,"map":p.name,
        "native_crc32":crc32fast::hash(&native.encode()?),"frame":frame,
        "issues":issues,"ramp_profiles":profiles,"routes":routes,
        "shore_boundary_edges":shore_edges,"shore_boundary_findings":shore_findings,
        "team_area_balance":balance,"passed":issues.is_empty() && shore_findings==0 && (user_layout || ratios.iter().all(|v| *v<=0.2))}),
    )?;
    assert!(
        issues.is_empty(),
        "native authoring issues: {}",
        serde_json::to_string(&issues)?
    );
    assert_eq!(
        shore_findings, 0,
        "all shoreline edges must be bidirectional"
    );
    assert!(user_layout || ratios.iter().all(|v| *v <= 0.2));
    println!(
        "河南 native audit: {} routes, {} shoreline edges, land/water imbalance {:.3}% / {:.3}%",
        routes.len(),
        shore_edges,
        ratios[0] * 100.,
        ratios[1] * 100.
    );
    Ok(())
}
