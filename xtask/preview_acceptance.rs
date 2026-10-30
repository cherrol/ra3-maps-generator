//! Read-only design-width screening. Does not edit terrain, images or playable maps.
use ra3maps::{
    Result, err,
    formats::{archive, npy::Npy},
    pipeline::project::write_json,
    render::Image,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, fs, path::Path};

const COLORS: [[u8; 3]; 6] = [
    [53, 116, 150],
    [104, 151, 89],
    [160, 178, 99],
    [202, 180, 119],
    [162, 128, 103],
    [218, 221, 211],
];

fn label(v: u8) -> String {
    match v {
        0 => "水域".into(),
        6 => "陆地整体".into(),
        _ => format!("第{v}层"),
    }
}

/// Exact lengths of contiguous cell-square intersections along four sampled directions.
/// These are width witnesses, not a complete arbitrary-angle intrusion classifier.
fn sections(a: &[u8], w: usize, h: usize, minimum: f64) -> Vec<Value> {
    let mut out = Vec::new();
    for (dx, dy) in [(1isize, 0isize), (0, 1), (1, 1), (-1, 1)] {
        let step = ((dx * dx + dy * dy) as f64).sqrt();
        for sy in 0..h {
            for sx in 0..w {
                let (px, py) = (sx as isize - dx, sy as isize - dy);
                if px >= 0 && py >= 0 && px < w as isize && py < h as isize {
                    continue;
                }
                let (mut x, mut y) = (sx as isize, sy as isize);
                let mut first = [x, y];
                let mut old = a[sy * w + sx];
                let mut count = 0;
                loop {
                    let inside = x >= 0 && y >= 0 && x < w as isize && y < h as isize;
                    let next = if inside {
                        a[y as usize * w + x as usize]
                    } else {
                        255
                    };
                    if next != old {
                        let length = count as f64 * step;
                        if old != 255 && length + 1e-9 < minimum {
                            let last = [x - dx, y - dy];
                            out.push(json!({"class":old,"first":first,"last":last,
                                "center":[(first[0]+last[0]) as f64/2.,(first[1]+last[1]) as f64/2.],
                                "direction":[dx,dy],"width_cells":length,"intersected_cells":count}));
                        }
                        first = [x, y];
                        old = next;
                        count = 0;
                    }
                    if !inside {
                        break;
                    }
                    count += 1;
                    x += dx;
                    y += dy;
                }
            }
        }
    }
    out
}

/// Group nearby cross-section centers, keeping tiny rounded caps separate from long candidates.
fn group(rows: Vec<Value>, w: usize, h: usize, minimum: f64, offset: usize) -> Vec<Value> {
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); w * h];
    for (i, r) in rows.iter().enumerate() {
        let x = r["center"][0].as_f64().unwrap().round() as usize;
        let y = r["center"][1].as_f64().unwrap().round() as usize;
        bins[y * w + x].push(i);
    }
    let mut seen = vec![false; rows.len()];
    let mut out = Vec::new();
    for s in 0..rows.len() {
        if seen[s] {
            continue;
        }
        let class = rows[s]["class"].as_u64().unwrap() as u8;
        let mut q = VecDeque::from([s]);
        seen[s] = true;
        let mut items = Vec::new();
        let mut bounds = [w, h, 0, 0];
        while let Some(i) = q.pop_front() {
            let x = rows[i]["center"][0].as_f64().unwrap().round() as usize;
            let y = rows[i]["center"][1].as_f64().unwrap().round() as usize;
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(y);
            items.push(rows[i].clone());
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    for &j in &bins[yy * w + xx] {
                        if !seen[j] && rows[j]["class"] == class {
                            seen[j] = true;
                            q.push_back(j);
                        }
                    }
                }
            }
        }
        let span = (bounds[2] - bounds[0]).max(bounds[3] - bounds[1]) as f64;
        // Length is only a triage heuristic. Short sections remain recorded, never silently passed.
        let extended = span >= minimum;
        let minimum_width = items
            .iter()
            .map(|v| v["width_cells"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min);
        for r in &mut items {
            for k in ["first", "last", "center"] {
                for j in 0..2 {
                    r[k][j] = json!(r[k][j].as_f64().unwrap() + offset as f64);
                }
            }
        }
        out.push(json!({"id":out.len()+1,"class":class,"kind":label(class),
            "bounds_full_grid":[bounds[0]+offset,bounds[1]+offset,bounds[2]+offset+1,bounds[3]+offset+1],
            "center_extent_cells":span,"minimum_sampled_width_cells":minimum_width,
            "triage":if extended {"延伸窄段，须核对伸入形态与用途"} else {"短断面或封端，须区别圆角与细尖尾"},
            "extended_candidate":extended,"status":"待核验","sections":items}));
    }
    out
}

fn scan(root: &Path, id: &str, output: &Path, minimum: f64) -> Result<Value> {
    let map = root.join("maps").join(id);
    let base = map.join("data/terrain-preview");
    let input = fs::read(base.join("terrain.npz"))?;
    let files = archive::unzip(&input)?;
    let a = Npy::parse(
        files
            .get("layers.npy")
            .ok_or_else(|| err("missing layers"))?,
    )?;
    let report: Value = serde_json::from_slice(&fs::read(base.join("report.json"))?)?;
    let (fw, fh) = (a.shape[1], a.shape[0]);
    let b = report["border_cells"].as_u64().unwrap_or(20) as usize;
    let (w, h) = (fw - 2 * b, fh - 2 * b);
    let full: Vec<u8> = a.numbers()?.iter().map(|v| *v as u8).collect();
    let cells: Vec<u8> = (0..h)
        .flat_map(|y| full[(y + b) * fw + b..(y + b) * fw + b + w].iter().copied())
        .collect();
    let mut candidates = group(sections(&cells, w, h, minimum), w, h, minimum, b);
    // Land/water union can have different bottlenecks from individual tiers.
    let land: Vec<_> = cells
        .iter()
        .map(|&v| if v == 0 { 255 } else { 6 })
        .collect();
    let mut land_groups = group(sections(&land, w, h, minimum), w, h, minimum, b);
    candidates.append(&mut land_groups);
    for (i, v) in candidates.iter_mut().enumerate() {
        v["id"] = json!(i + 1);
    }
    let mut image = Image::new(w, h, [0, 0, 0]);
    for y in 0..h {
        for x in 0..w {
            image.pixel(x as isize, y as isize, COLORS[cells[y * w + x] as usize]);
        }
    }
    for c in &candidates {
        let color = if c["extended_candidate"] == true {
            [255, 32, 67]
        } else {
            [255, 176, 30]
        };
        for r in c["sections"].as_array().unwrap() {
            let x = r["center"][0].as_f64().unwrap() - b as f64;
            let y = r["center"][1].as_f64().unwrap() - b as f64;
            image.pixel(x.round() as isize, y.round() as isize, color);
        }
    }
    fs::create_dir_all(output)?;
    image.png(&output.join(format!("{id}.png")))?;
    let long = candidates
        .iter()
        .filter(|v| v["extended_candidate"] == true)
        .count();
    let result = json!({"map_id":id,"input_crc32":crc32fast::hash(&input),"frame":[fw,fh,b],
        "minimum_width_cells":minimum,"extended_candidates":long,"short_candidates":candidates.len()-long,
        "method":"four-direction cell-square cross-section witnesses; connected center grouping; >= minimum-width spatial extent is triage only",
        "limitations":"No automatic arbitrary-angle intrusion classification; rounded caps and map-clipped boundaries require review. Positive short-width witnesses do not on their own prove an intrusion. No inferred combat routes or building placements.",
        "candidates":candidates,"diagnostic_png":format!("{id}.png")});
    write_json(&output.join(format!("{id}.json")), &result)?;
    println!(
        "{id}: {long} extended, {} short candidates",
        result["short_candidates"]
    );
    Ok(result)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err(err(
            "usage: preview-acceptance <project-root> <output-directory>",
        ));
    }
    let root = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    let rules: Value =
        serde_json::from_slice(&fs::read(root.join("rules/coordinate-pipeline.json"))?)?;
    let minimum = rules["terrain_shape"]["intrusion_minimum_width_cells"]
        .as_f64()
        .ok_or_else(|| err("missing minimum intrusion width"))?;
    if !minimum.is_finite() || minimum <= 0. {
        return Err(err("invalid minimum intrusion width"));
    }
    let mut ids = Vec::new();
    for e in fs::read_dir(root.join("maps"))? {
        let e = e?;
        if e.path().join("data/terrain-preview/terrain.npz").is_file() {
            ids.push(e.file_name().to_string_lossy().to_string());
        }
    }
    ids.sort();
    let mut maps = Vec::new();
    for id in ids {
        maps.push(scan(root, &id, output, minimum)?);
    }
    gallery(root, &maps, output)?;
    write_json(
        &output.join("summary.json"),
        &json!({"schema_version":1,"rules":rules["terrain_shape"],"maps":maps}),
    )?;
    Ok(())
}

fn gallery(root: &Path, maps: &[Value], output: &Path) -> Result<()> {
    let font = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ]
    .into_iter()
    .find_map(|p| {
        fs::read(p)
            .ok()
            .and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
    });
    let mut panels = Vec::new();
    for map in maps {
        let id = map["map_id"].as_str().unwrap();
        let files = archive::unzip(&fs::read(
            root.join("maps")
                .join(id)
                .join("data/terrain-preview/terrain.npz"),
        )?)?;
        let a = Npy::parse(&files["layers.npy"])?;
        let data = a.numbers()?;
        let fw = a.shape[1];
        let fh = a.shape[0];
        let b = map["frame"][2].as_u64().unwrap() as isize;
        for candidate in map["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["extended_candidate"] == true)
        {
            let bounds = candidate["bounds_full_grid"].as_array().unwrap();
            let cx = (bounds[0].as_i64().unwrap() + bounds[2].as_i64().unwrap()) as isize / 2;
            let cy = (bounds[1].as_i64().unwrap() + bounds[3].as_i64().unwrap()) as isize / 2;
            let mut tile = Image::new(48, 48, [26, 32, 37]);
            for yy in 0..48 {
                for xx in 0..48 {
                    let x = cx - 24 + xx;
                    let y = cy - 24 + yy;
                    if x >= b && y >= b && x < fw as isize - b && y < fh as isize - b {
                        tile.pixel(xx, yy, COLORS[data[y as usize * fw + x as usize] as usize]);
                    }
                }
            }
            for witness in candidate["sections"].as_array().unwrap() {
                let x = witness["center"][0].as_f64().unwrap().round() as isize;
                let y = witness["center"][1].as_f64().unwrap().round() as isize;
                tile.pixel(x - (cx - 24), y - (cy - 24), [255, 32, 67]);
            }
            panels.push((
                format!(
                    "{} #{} {}",
                    id,
                    candidate["id"],
                    candidate["kind"].as_str().unwrap()
                ),
                tile.resize(192, 192),
            ));
        }
    }
    for (batch, group) in panels.chunks(18).enumerate() {
        let mut sheet = Image::new(840, group.len().div_ceil(4) * 230, [26, 32, 37]);
        for (index, (title, tile)) in group.iter().enumerate() {
            let x = (index % 4) * 210;
            let y = (index / 4) * 230;
            sheet.text(
                font.as_ref(),
                x as isize + 5,
                y as isize + 2,
                title,
                15.,
                [240, 240, 240],
            );
            for yy in 0..192 {
                for xx in 0..192 {
                    let i = (yy * 192 + xx) * 3;
                    sheet.pixel(
                        (x + xx + 5) as isize,
                        (y + yy + 28) as isize,
                        tile.rgb[i..i + 3].try_into().unwrap(),
                    );
                }
            }
        }
        sheet.png(&output.join(format!("review-sheet-{}.png", batch + 1)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(width: usize) -> (Vec<u8>, usize, usize) {
        let (w, h) = (60, 30);
        let mut a = vec![255; w * h];
        for y in 10..10 + width {
            for x in 5..55 {
                a[y * w + x] = 1;
            }
        }
        (a, w, h)
    }
    #[test]
    fn four_cell_corridor_has_long_width_witnesses() {
        let (a, w, h) = rectangle(4);
        let g = group(sections(&a, w, h, 5.), w, h, 5., 0);
        assert!(
            g.iter()
                .any(|v| v["extended_candidate"] == true && v["kind"] == "第1层")
        );
    }
    #[test]
    fn five_cell_corridor_is_not_reported_as_long_sub_five_strip() {
        let (a, w, h) = rectangle(5);
        let g = group(sections(&a, w, h, 5.), w, h, 5., 0);
        assert!(!g.iter().any(|v| v["extended_candidate"] == true));
    }
    #[test]
    fn ten_cell_route_requirement_is_read_as_a_distinct_threshold() {
        let (a, w, h) = rectangle(9);
        let g = group(sections(&a, w, h, 10.), w, h, 10., 0);
        assert!(g.iter().any(|v| v["extended_candidate"] == true));
        let (a, w, h) = rectangle(10);
        let g = group(sections(&a, w, h, 10.), w, h, 10., 0);
        assert!(!g.iter().any(|v| v["extended_candidate"] == true));
    }
    #[test]
    fn diagonal_narrow_band_is_detected() {
        let (w, h) = (60, 60);
        let mut a = vec![255; w * h];
        for y in 8..52 {
            for x in 8..52 {
                if (x as isize - y as isize).abs() <= 2 {
                    a[y * w + x] = 1;
                }
            }
        }
        let g = group(sections(&a, w, h, 5.), w, h, 5., 0);
        assert!(g.iter().any(|v| v["extended_candidate"] == true));
    }
    #[test]
    fn wide_circle_does_not_turn_caps_into_long_intrusions() {
        let (w, h) = (80, 80);
        let mut a = vec![255; w * h];
        for y in 0..h {
            for x in 0..w {
                if (x as isize - 40).pow(2) + (y as isize - 40).pow(2) <= 30 * 30 {
                    a[y * w + x] = 1;
                }
            }
        }
        let g = group(sections(&a, w, h, 5.), w, h, 5., 0);
        assert!(!g.iter().any(|v| v["extended_candidate"] == true));
    }
}
