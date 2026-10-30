use flate2::read::ZlibDecoder;
use ra3maps::{formats::archive, formats::npy::Npy, terrain_preview};
use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "ra3-terrain-preview-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(path.join("docs")).unwrap();
    fs::create_dir_all(path.join("maps")).unwrap();
    fs::write(
        path.join("docs/九州大地命名.md"),
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/九州大地命名.md")).unwrap(),
    )
    .unwrap();
    Fixture(path)
}
fn npy(dtype: &str, shape: &[usize], bytes: Vec<u8>) -> Vec<u8> {
    let dims = shape
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let mut h = format!("{{'descr': '{dtype}', 'fortran_order': False, 'shape': ({dims},), }}");
    while (h.len() + 11) % 64 != 0 {
        h.push(' ');
    }
    h.push('\n');
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend((h.len() as u16).to_le_bytes());
    out.extend(h.as_bytes());
    out.extend(bytes);
    out
}
fn reference(root: &Fixture, broken_water: bool) -> PathBuf {
    let (w, h) = (640usize, 680usize);
    let mut z = vec![];
    let mut water = vec![0u8; w * h];
    let mut province = water.clone();
    for y in 0..h {
        for x in 0..w {
            // A known geographic plane makes interpolation and north-up
            // orientation independently checkable after resizing.
            z.extend((500f32 + x as f32 * 6. + y as f32 * 0.1).to_le_bytes());
            let i = y * w + x;
            province[i] = (x >= 23 && x < w - 23 && y >= 24 && y < h - 24) as u8;
            water[i] = ((300..312).contains(&x) && (300..312).contains(&y)) as u8;
        }
    }
    let water_shape = if broken_water { vec![1, 1] } else { vec![h, w] };
    let arrays = vec![
        ("elevation_m.npy".into(), npy("<f4", &[h, w], z)),
        ("land_mask.npy".into(), npy("|u1", &[h, w], province)),
        (
            "water_mask.npy".into(),
            npy(
                "|u1",
                &water_shape,
                if broken_water { vec![1] } else { water },
            ),
        ),
        (
            "bounds_wgs84.npy".into(),
            npy(
                "<f4",
                &[4],
                [97f32, 21., 106., 30.]
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect(),
            ),
        ),
    ];
    let path = root.0.join("provided.npz");
    fs::write(&path, archive::zip(&arrays).unwrap()).unwrap();
    path
}
fn png(path: &Path) -> (usize, usize, Vec<u8>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let (mut offset, mut w, mut h) = (8, 0, 0);
    let mut packed = vec![];
    while offset < bytes.len() {
        let n = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let tag = &bytes[offset + 4..offset + 8];
        let body = &bytes[offset + 8..offset + 8 + n];
        assert_eq!(
            crc32fast::hash(&bytes[offset + 4..offset + 8 + n]),
            u32::from_be_bytes(bytes[offset + 8 + n..offset + 12 + n].try_into().unwrap())
        );
        if tag == b"IHDR" {
            w = u32::from_be_bytes(body[..4].try_into().unwrap()) as usize;
            h = u32::from_be_bytes(body[4..8].try_into().unwrap()) as usize;
        }
        if tag == b"IDAT" {
            packed.extend_from_slice(body);
        }
        offset += n + 12;
    }
    let mut raw = vec![];
    ZlibDecoder::new(&packed[..]).read_to_end(&mut raw).unwrap();
    assert_eq!(raw.len(), (w * 3 + 1) * h);
    let mut rgb = vec![];
    for row in raw.chunks(w * 3 + 1) {
        assert_eq!(row[0], 0);
        rgb.extend_from_slice(&row[1..]);
    }
    (w, h, rgb)
}
#[test]
fn docs_drive_names_and_invalid_names_cannot_create_output() {
    let f = fixture();
    assert_eq!(
        terrain_preview::map_name(&f.0, "云南省").unwrap(),
        "层峦织彩"
    );
    assert_eq!(
        terrain_preview::map_name(&f.0, "翠屿浮光").unwrap(),
        "翠屿浮光"
    );
    assert_eq!(terrain_preview::map_id(&f.0, "云南省").unwrap(), "云南");
    assert_eq!(terrain_preview::map_id(&f.0, "层峦织彩").unwrap(), "云南");
    assert!(terrain_preview::map_name(&f.0, "地图名").is_err());
    assert!(
        terrain_preview::generate(&f.0, "../../outside", &f.0.join("absent.npz"), None).is_err()
    );
    assert_eq!(fs::read_dir(f.0.join("maps")).unwrap().count(), 0);
}
#[test]
fn bad_reference_leaves_existing_three_images_untouched() {
    let f = fixture();
    let reference = reference(&f, true);
    let out = f.0.join("maps/云南");
    fs::create_dir_all(&out).unwrap();
    for name in ["source.png", "preview.png", "preview--line.png"] {
        fs::write(out.join(name), b"existing preview").unwrap();
    }
    assert!(
        terrain_preview::generate(&f.0, "云南", &reference, None)
            .unwrap_err()
            .to_string()
            .contains("尺寸")
    );
    for name in ["source.png", "preview.png", "preview--line.png"] {
        assert_eq!(fs::read(out.join(name)).unwrap(), b"existing preview");
    }
}
#[test]
fn coordinate_initialization_reads_cache_without_rendering_or_overwriting_playable_frame() {
    let f = fixture();
    let out = f.0.join("maps/云南");
    let internal = out.join("data/terrain-preview");
    fs::create_dir_all(&internal).unwrap();
    let terrain = archive::zip(&[(
        "elevation_m.npy".into(),
        npy("<f4", &[80, 128], vec![0; 80 * 128 * 4]),
    )])
    .unwrap();
    fs::write(internal.join("terrain.npz"), &terrain).unwrap();
    let report = serde_json::to_vec(&serde_json::json!({
        "game_cells":[114,66],"border_cells":7
    }))
    .unwrap();
    fs::write(internal.join("report.json"), &report).unwrap();
    for name in ["source.png", "preview.png", "preview--line.png"] {
        fs::write(out.join(name), b"existing preview image").unwrap();
    }
    let path = terrain_preview::initialize_coordinate_system(&f.0, "云南").unwrap();
    let system: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(system["scope"], "terrain-preview-design");
    assert_eq!(system["frame"]["width"], 128);
    assert_eq!(system["frame"]["height"], 80);
    assert_eq!(system["frame"]["border"], 7);
    assert_eq!(system["playable_cells"], serde_json::json!([114, 66]));
    assert_eq!(system["world_origin_grid"], serde_json::json!([7, 72]));
    let root = out.join("coordinate-system.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&root).unwrap()).unwrap(),
        system
    );
    fs::write(out.join("config.json"), b"existing playable recipe").unwrap();
    fs::write(&root, b"existing playable coordinate system").unwrap();
    terrain_preview::initialize_coordinate_system(&f.0, "云南").unwrap();
    assert_eq!(
        fs::read(root).unwrap(),
        b"existing playable coordinate system"
    );
    assert_eq!(fs::read(internal.join("terrain.npz")).unwrap(), terrain);
    assert_eq!(fs::read(internal.join("report.json")).unwrap(), report);
    for name in ["source.png", "preview.png", "preview--line.png"] {
        assert_eq!(fs::read(out.join(name)).unwrap(), b"existing preview image");
    }
}

#[test]
fn public_command_generates_three_aligned_images_for_an_unregistered_map() {
    let f = fixture();
    let input = reference(&f, false);
    let before = fs::read(&input).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args([
            "terrain-preview",
            "云南",
            input.to_str().unwrap(),
            "--root",
            f.0.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let out = f.0.join("maps/云南");
    assert!(!f.0.join("maps/层峦织彩").exists());
    let mut images = fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|v| v == "png"))
        .collect::<Vec<_>>();
    images.sort();
    assert_eq!(images.len(), 3);
    assert!(!out.join("config.json").exists());
    assert!(out.join("coordinate-system.json").exists());
    let coordinate_system: Value = serde_json::from_slice(
        &fs::read(out.join("data/terrain-preview/coordinate-system.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(coordinate_system["scope"], "terrain-preview-design");
    assert_eq!(coordinate_system["map_id"], "云南");
    assert_eq!(
        coordinate_system["source"]["path"],
        "maps/云南/data/terrain-preview/terrain.npz"
    );
    assert_eq!(coordinate_system["frame"]["width"], 840);
    assert_eq!(coordinate_system["frame"]["height"], 840);
    assert_eq!(coordinate_system["frame"]["border"], 20);
    assert_eq!(
        coordinate_system["playable_cells"],
        serde_json::json!([800, 800])
    );
    let coordinate_bytes = fs::read(out.join("coordinate-system.json")).unwrap();
    assert_eq!(
        coordinate_bytes,
        fs::read(out.join("data/terrain-preview/coordinate-system.json")).unwrap()
    );
    assert!(!f.0.join("maps/.default").exists());
    let (w, h, source) = png(&out.join("source.png"));
    let (pw, ph, plain) = png(&out.join("preview.png"));
    let (lw, lh, line) = png(&out.join("preview--line.png"));
    assert_eq!((w, h), (1104, 1002));
    assert_eq!((w, h), (pw, ph));
    assert_eq!((w, h), (lw, lh));
    assert_ne!(source, plain);
    assert_ne!(plain, line);
    let arrays =
        archive::unzip(&fs::read(out.join("data/terrain-preview/terrain.npz")).unwrap()).unwrap();
    let elevation = Npy::parse(&arrays["elevation_m.npy"]).unwrap();
    assert_eq!(elevation.shape, [840, 840]);
    let raw = elevation.numbers().unwrap();
    let masks = Npy::parse(&arrays["water_mask.npy"])
        .unwrap()
        .numbers()
        .unwrap();
    let layers = Npy::parse(&arrays["layers.npy"])
        .unwrap()
        .numbers()
        .unwrap();
    let heights = Npy::parse(&arrays["game_heights.npy"])
        .unwrap()
        .numbers()
        .unwrap();
    let report: Value =
        serde_json::from_slice(&fs::read(out.join("data/terrain-preview/report.json")).unwrap())
            .unwrap();
    assert_eq!(report["same_view_for_all_images"], true);
    assert_eq!(report["game_cells"], serde_json::json!([800, 800]));
    assert_eq!(report["size_policy"]["shape"], "square");
    assert_eq!(report["size_policy"]["long_side_cells"], 800);
    for y in 0..800 {
        for x in 0..800 {
            let i = (y + 20) * 840 + x + 20;
            let pixel = ((y + 112) * w + x + 28) * 3;
            assert_ne!(&plain[pixel..pixel + 3], &[26, 32, 37]);
            let source_x = 19.5 + (x as f64 + 0.5) * 600. / 800.;
            let source_y = 19.5 + (y as f64 + 0.5) * 640. / 800.;
            let expected_elevation = 500. + source_x * 6. + source_y * 0.1;
            assert!((raw[i] - expected_elevation).abs() < 0.001);
            let expected_water = (300. ..312.).contains(&source_x.round())
                && (300. ..312.).contains(&source_y.round());
            assert_eq!(masks[i] != 0., expected_water);
            if masks[i] != 0. {
                assert_eq!(layers[i], 0.);
                assert_eq!(heights[i], 130.);
                assert_eq!(&plain[pixel..pixel + 3], &[53, 116, 150]);
            } else {
                let l = layers[i] as usize;
                assert!((1..=5).contains(&l));
                assert_eq!(
                    heights[i],
                    report["game_layer_heights"][l - 1].as_f64().unwrap()
                );
            }
        }
    }
    assert_eq!(fs::read(input).unwrap(), before);
    let full = Npy::parse(&arrays["playable_bounds_wgs84.npy"])
        .unwrap()
        .numbers()
        .unwrap();
    for (actual, expected) in full.into_iter().zip([
        97. + 20. / 640. * 9.,
        21. + 20. / 680. * 9.,
        106. - 20. / 640. * 9.,
        30. - 20. / 680. * 9.,
    ]) {
        assert!((actual - expected).abs() < 0.0001);
    }
    assert!(out.join("data/terrain-preview/reference.npz").exists());
}

#[test]
fn center_command_uses_cached_frame_and_preserves_original_artifacts() {
    let f = fixture();
    let input = reference(&f, false);
    let out = terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    let paths = [
        "source.png",
        "preview.png",
        "preview--line.png",
        "data/terrain-preview/terrain.npz",
        "data/terrain-preview/report.json",
        "data/terrain-preview/reference.npz",
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|p| fs::read(out.join(p)).unwrap())
        .collect();
    fs::write(out.join("data/terrain-preview/centers.json"),serde_json::to_vec(&serde_json::json!({
        "map_name":"层峦织彩", "label":"地级行政中心", "coordinate_system":"WGS84", "source":"fixture",
        "points":[{"adcode":"one","name":"中心市","center_wgs84":[101.,25.]},
            {"adcode":"outside","name":"图外市","center_wgs84":[120.,25.]}]
    })).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args([
            "terrain-preview-centers",
            "云南",
            "--root",
            f.0.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for (path, bytes) in paths.iter().zip(before) {
        assert_eq!(fs::read(out.join(path)).unwrap(), bytes);
    }
    let report: Value = serde_json::from_slice(
        &fs::read(out.join("data/terrain-preview/centers-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["visible_centers"], 1);
    assert_eq!(report["outside_frame"][0]["adcode"], "outside");
    let (w, h, rgb) = png(&out.join("preview--center--line.png"));
    let (ow, oh, original) = png(&out.join("preview--line.png"));
    assert_eq!((w, h), (ow, oh));
    assert_ne!(rgb, original);
    let p = &report["points"][0]["image_pixel"];
    let i = (p[1].as_u64().unwrap() as usize * w + p[0].as_u64().unwrap() as usize) * 3;
    assert_eq!(&rgb[i..i + 3], &[236, 74, 65]);
    assert!(!out.join("config.json").exists());
    // Future regeneration includes the overlay whenever center data is present.
    terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    let regenerated: Value =
        serde_json::from_slice(&fs::read(out.join("data/terrain-preview/report.json")).unwrap())
            .unwrap();
    assert!(
        regenerated["final_images"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("preview--center--line.png"))
    );
    assert_eq!(
        fs::read_dir(&out)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|s| s == "png"))
            .count(),
        4
    );
}

fn replace_water_reference(path: &Path, source_water: Option<Vec<u8>>, design_water: Vec<u8>) {
    let mut arrays = archive::unzip(&fs::read(path).unwrap()).unwrap();
    arrays.insert(
        "water_mask.npy".into(),
        npy("|u1", &[680, 640], design_water),
    );
    if let Some(water) = source_water {
        arrays.insert(
            "source_water_mask.npy".into(),
            npy("|u1", &[680, 640], water),
        );
    }
    fs::write(
        path,
        archive::zip(&arrays.into_iter().collect::<Vec<_>>()).unwrap(),
    )
    .unwrap();
}

fn water_square(x0: usize, y0: usize, size: usize) -> Vec<u8> {
    let mut mask = vec![0; 640 * 680];
    for y in y0..y0 + size {
        for x in x0..x0 + size {
            mask[y * 640 + x] = 1;
        }
    }
    mask
}

// The fixture's geographic crop is [20, 20, 620, 660]. Choose a pixel
// whose center samples the requested input location, independently of the
// production renderer's source-water selection.
fn reference_pixel(rgb: &[u8], width: usize, x: f64, y: f64) -> &[u8] {
    let px = (((x + 0.5 - 20.) * 800. / 600. - 0.5).round() as usize) + 28;
    let py = (((y + 0.5 - 20.) * 800. / 640. - 0.5).round() as usize) + 112;
    &rgb[(py * width + px) * 3..(py * width + px) * 3 + 3]
}

#[test]
fn source_preserves_real_water_when_design_water_is_added_or_removed() {
    let f = fixture();
    let input = reference(&f, false);
    replace_water_reference(
        &input,
        Some(water_square(300, 300, 24)),
        water_square(420, 420, 24),
    );
    let out = terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    let source_before = fs::read(out.join("source.png")).unwrap();
    let preview_before = fs::read(out.join("preview.png")).unwrap();
    let (w, _, source) = png(&out.join("source.png"));
    let (_, _, preview) = png(&out.join("preview.png"));
    assert_eq!(reference_pixel(&source, w, 312., 312.), &[53, 116, 150]);
    assert_ne!(reference_pixel(&preview, w, 312., 312.), &[53, 116, 150]);
    assert_ne!(reference_pixel(&source, w, 432., 432.), &[53, 116, 150]);
    assert_eq!(reference_pixel(&preview, w, 432., 432.), &[53, 116, 150]);

    replace_water_reference(&input, None, water_square(500, 500, 32));
    terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    assert_eq!(fs::read(out.join("source.png")).unwrap(), source_before);
    assert_ne!(fs::read(out.join("preview.png")).unwrap(), preview_before);
}

#[test]
fn source_command_uses_real_reference_and_protects_design_and_playable_artifacts() {
    let f = fixture();
    let input = reference(&f, false);
    let out = terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    let internal = out.join("data/terrain-preview");
    let source_path = out.join("source.png");
    let report_path = internal.join("report.json");
    let original_source = fs::read(&source_path).unwrap();
    let original_report = fs::read(&report_path).unwrap();

    // A failed source-only invocation cannot replace the current image or report.
    assert!(terrain_preview::generate_source(&f.0, "云南").is_err());
    assert_eq!(fs::read(&source_path).unwrap(), original_source);
    assert_eq!(fs::read(&report_path).unwrap(), original_report);

    let source_reference = internal.join("source-reference.npz");
    fs::copy(&input, &source_reference).unwrap();
    replace_water_reference(&source_reference, None, water_square(360, 360, 24));
    let valid_reference = fs::read(&source_reference).unwrap();
    let mut invalid = archive::unzip(&valid_reference).unwrap();
    invalid.insert(
        "bounds_wgs84.npy".into(),
        npy(
            "<f4",
            &[4],
            [98f32, 21., 107., 30.]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        ),
    );
    fs::write(
        &source_reference,
        archive::zip(&invalid.into_iter().collect::<Vec<_>>()).unwrap(),
    )
    .unwrap();
    assert!(terrain_preview::generate_source(&f.0, "云南").is_err());
    assert_eq!(fs::read(&source_path).unwrap(), original_source);
    assert_eq!(fs::read(&report_path).unwrap(), original_report);
    fs::write(&source_reference, &valid_reference).unwrap();

    for (path, bytes) in [
        ("config.json", b"playable recipe".as_slice()),
        ("layers.npz", b"playable layers".as_slice()),
        (
            "preview--center--line.png",
            b"existing center overlay".as_slice(),
        ),
        (
            "data/terrain-preview/parameters.json",
            b"{\"preserve_this_configuration\":true}".as_slice(),
        ),
    ] {
        fs::write(out.join(path), bytes).unwrap();
    }
    let protected = [
        "preview.png",
        "preview--line.png",
        "preview--center--line.png",
        "coordinate-system.json",
        "config.json",
        "layers.npz",
        "data/terrain-preview/coordinate-system.json",
        "data/terrain-preview/terrain.npz",
        "data/terrain-preview/reference.npz",
        "data/terrain-preview/source-reference.npz",
        "data/terrain-preview/parameters.json",
    ];
    let before: Vec<_> = protected
        .iter()
        .map(|path| fs::read(out.join(path)).unwrap())
        .collect();
    let result = Command::new(env!("CARGO_BIN_EXE_ra3maps"))
        .args([
            "terrain-preview-source",
            "云南",
            "--root",
            f.0.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for (path, bytes) in protected.iter().zip(before) {
        assert_eq!(fs::read(out.join(path)).unwrap(), bytes, "changed {path}");
    }
    let source_bytes = fs::read(&source_path).unwrap();
    assert_ne!(source_bytes, original_source);
    let (w, h, source) = png(&source_path);
    assert_eq!((w, h), (1104, 1002));
    assert_eq!(reference_pixel(&source, w, 372., 372.), &[53, 116, 150]);
    assert_ne!(reference_pixel(&source, w, 306., 306.), &[53, 116, 150]);
    let mut report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(
        report["image_crc32"]["source.png"],
        crc32fast::hash(&source_bytes)
    );
    assert!(internal.join("source-report.json").is_file());
    let original: Value = serde_json::from_slice(&original_report).unwrap();
    if let Some(source_image) = original.get("source_image") {
        report["source_image"] = source_image.clone();
    } else {
        report.as_object_mut().unwrap().remove("source_image");
    }
    report["image_crc32"]["source.png"] = original["image_crc32"]["source.png"].clone();
    assert_eq!(report, original);

    // Ordinary regeneration must use the same independent source reference.
    fs::remove_file(out.join("config.json")).unwrap();
    terrain_preview::generate(&f.0, "云南", &input, None).unwrap();
    assert_eq!(fs::read(source_path).unwrap(), source_bytes);
}
