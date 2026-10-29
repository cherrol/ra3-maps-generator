//! Geographic center overlays. Positions remain fixed; only labels move.
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;

pub const FILE: &str = "preview--center--line.png";
const INK: [u8; 3] = [28, 34, 39];
const PAPER: [u8; 3] = [255, 249, 225];
const DOT: [u8; 3] = [236, 74, 65];

#[derive(Deserialize)]
pub struct Centers {
    pub map_name: String,
    pub label: String,
    pub coordinate_system: String,
    pub source: String,
    pub points: Vec<Center>,
}
#[derive(Deserialize)]
pub struct Center {
    pub adcode: String,
    pub name: String,
    pub center_wgs84: [f64; 2],
}

pub fn read(map_dir: &Path, name: &str) -> Result<Centers> {
    let centers: Centers = serde_json::from_slice(&fs::read(
        map_dir.join("data/terrain-preview/centers.json"),
    )?)?;
    if centers.map_name != name || centers.coordinate_system != "WGS84" || centers.points.is_empty()
    {
        return Err(err("中心标点数据须属于本图、使用 WGS84 且包含点位"));
    }
    let mut codes = BTreeSet::new();
    for p in &centers.points {
        let [lon, lat] = p.center_wgs84;
        if p.name.trim().is_empty()
            || p.adcode.is_empty()
            || !codes.insert(&p.adcode)
            || !lon.is_finite()
            || !lat.is_finite()
            || !(-180. ..=180.).contains(&lon)
            || !(-90. ..=90.).contains(&lat)
        {
            return Err(err("行政中心名称、代码或经纬度无效／重复"));
        }
    }
    Ok(centers)
}

fn project(p: [f64; 2], bounds: [f64; 4], w: usize, h: usize) -> Option<[isize; 2]> {
    let [west, south, east, north] = bounds;
    if p[0] < west || p[0] > east || p[1] < south || p[1] > north {
        return None;
    }
    // Bounds describe pixel edges, while marker positions refer to pixel centers.
    Some([
        (28. + (p[0] - west) / (east - west) * w as f64 - 0.5)
            .round()
            .clamp(28., (28 + w - 1) as f64) as isize,
        (112. + (north - p[1]) / (north - south) * h as f64 - 0.5)
            .round()
            .clamp(112., (112 + h - 1) as f64) as isize,
    ])
}

#[derive(Clone, Copy, Debug)]
struct Box2 {
    x: isize,
    y: isize,
    w: isize,
    h: isize,
}
impl Box2 {
    fn overlap(self, b: Self) -> isize {
        (self.x + self.w)
            .min(b.x + b.w)
            .saturating_sub(self.x.max(b.x))
            .max(0)
            * (self.y + self.h)
                .min(b.y + b.h)
                .saturating_sub(self.y.max(b.y))
                .max(0)
    }
    fn contains(self, p: [isize; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
}
fn line(im: &mut Image, a: [isize; 2], b: [isize; 2], c: [u8; 3]) {
    let n = (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).max(1);
    for j in 0..=n {
        im.pixel(
            a[0] + (b[0] - a[0]) * j / n,
            a[1] + (b[1] - a[1]) * j / n,
            c,
        );
    }
}
fn dot(im: &mut Image, p: [isize; 2]) {
    for y in -5isize..=5 {
        for x in -5isize..=5 {
            let r = x * x + y * y;
            if r <= 25 {
                im.pixel(
                    p[0] + x,
                    p[1] + y,
                    if r <= 8 {
                        DOT
                    } else if r <= 16 {
                        PAPER
                    } else {
                        INK
                    },
                );
            }
        }
    }
}

pub fn draw(
    im: &mut Image,
    centers: &Centers,
    bounds: [f64; 4],
    w: usize,
    h: usize,
) -> Result<Value> {
    let font = render::font().ok_or_else(|| err("中心标点需要可用的中文字体（RA3MAP_FONT）"))?;
    let size = 13.;
    let mut visible = Vec::new();
    let mut outside = Vec::new();
    for p in &centers.points {
        if let Some(pixel) = project(p.center_wgs84, bounds, w, h) {
            visible.push((p, pixel));
        } else {
            outside.push(json!({"adcode":p.adcode,"name":p.name,"center_wgs84":p.center_wgs84}));
        }
    }
    let mut boxes: Vec<Box2> = Vec::new();
    let mut placements = Vec::new();
    // Most constrained (crowded) labels first, with stable code order as tie-break.
    visible.sort_by_key(|(p, a)| {
        let nearest = centers
            .points
            .iter()
            .filter(|q| q.adcode != p.adcode)
            .filter_map(|q| project(q.center_wgs84, bounds, w, h))
            .map(|b| (a[0] - b[0]).pow(2) + (a[1] - b[1]).pow(2))
            .min()
            .unwrap_or(isize::MAX);
        (nearest, p.adcode.clone())
    });
    for (p, pixel) in &visible {
        let tw: isize = p
            .name
            .chars()
            .map(|c| font.metrics(c, size).advance_width.round() as isize)
            .sum();
        let (bw, bh) = (tw + 6, 20);
        if bw > w as isize - 4 || bh > h as isize - 4 {
            return Err(err("预览取景太窄，无法容纳行政中心名称"));
        }
        let mut best: Option<(isize, Box2)> = None;
        // Expand the label search around its true coordinate, keeping all labels
        // inside the terrain rectangle and clear of every other marker.
        for r in [0, 14, 28, 44, 64, 90, 120, 160, 210, 270] {
            for (dx, dy) in [
                (8 + r, -10),
                (8 + r, -24 - r),
                (8 + r, 4 + r),
                (-bw - 8 - r, -10),
                (-bw - 8 - r, -24 - r),
                (-bw - 8 - r, 4 + r),
                (-bw / 2, -24 - r),
                (-bw / 2, 8 + r),
            ] {
                let b = Box2 {
                    x: (pixel[0] + dx).clamp(30, (28 + w) as isize - bw - 2),
                    y: (pixel[1] + dy).clamp(114, (112 + h) as isize - bh - 2),
                    w: bw,
                    h: bh,
                };
                let overlap: isize = boxes.iter().map(|&other| b.overlap(other)).sum();
                let markers = visible
                    .iter()
                    .filter(|(_, q)| {
                        b.overlap(Box2 {
                            x: q[0] - 6,
                            y: q[1] - 6,
                            w: 13,
                            h: 13,
                        }) > 0
                    })
                    .count();
                let distance =
                    (b.x + b.w / 2 - pixel[0]).pow(2) + (b.y + b.h / 2 - pixel[1]).pow(2);
                let cost = overlap * 1_000_000 + markers as isize * 100_000_000 + distance;
                if best.is_none_or(|(score, _)| cost < score) {
                    best = Some((cost, b));
                }
            }
        }
        let b = best.unwrap().1;
        let conflicts = boxes.iter().any(|&other| b.overlap(other) > 0)
            || visible.iter().any(|(_, q)| b.contains(*q));
        if conflicts {
            return Err(err(format!(
                "{} 的中心名称无法避让；请调整标注布局",
                p.name
            )));
        }
        boxes.push(b);
        placements.push((p, *pixel, b));
    }
    for (_, pixel, b) in &placements {
        let end = [
            pixel[0].clamp(b.x, b.x + b.w),
            pixel[1].clamp(b.y, b.y + b.h),
        ];
        line(im, *pixel, end, INK);
    }
    for (p, _, b) in &placements {
        // Subtle dark backing ensures long labels remain readable on every layer.
        for y in b.y..b.y + b.h {
            for x in b.x..b.x + b.w {
                let i = (y as usize * im.width + x as usize) * 3;
                for (c, &ink) in INK.iter().enumerate() {
                    im.rgb[i + c] = ((im.rgb[i + c] as u16 + ink as u16 * 3) / 4) as u8;
                }
            }
        }
        im.text(Some(&font), b.x + 3, b.y + 2, &p.name, size, PAPER);
    }
    // Render markers last so leaders can never erase the true positions.
    for (_, pixel, _) in &placements {
        dot(im, *pixel);
    }
    let left = (w + 54) as isize;
    // Existing legend space remains intact; the center key uses the footer.
    dot(im, [36, (h + 181) as isize]);
    im.text(
        Some(&font),
        48,
        (h + 171) as isize,
        &format!("{} · {} 处", centers.label, visible.len()),
        14.,
        PAPER,
    );
    im.text(Some(&font), left, 85, "中心标点 + 边界", 14., PAPER);
    Ok(
        json!({"schema_version":1,"map_name":centers.map_name,"image":FILE,
        "coordinate_system":"WGS84","source":centers.source,"label":centers.label,
        "bounds_wgs84":bounds,"game_cells":[w,h],"total_centers":centers.points.len(),
        "visible_centers":visible.len(),"outside_frame":outside,
        "points":placements.iter().map(|(p,pixel,b)|json!({"adcode":p.adcode,"name":p.name,
            "center_wgs84":p.center_wgs84,"image_pixel":pixel,"label_box":[b.x,b.y,b.w,b.h]})).collect::<Vec<_>>() }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn north_up_projection_uses_pixel_centers_and_does_not_clamp_offscreen_points() {
        let bounds = [100., 20., 110., 30.];
        assert_eq!(project([105., 25.], bounds, 800, 640), Some([428, 432]));
        assert_eq!(project([100., 30.], bounds, 800, 640), Some([28, 112]));
        assert_eq!(project([110., 20.], bounds, 800, 640), Some([827, 751]));
        assert_eq!(project([110.001, 25.], bounds, 800, 640), None);
    }
    #[test]
    fn crowded_labels_move_without_moving_or_hiding_centers() {
        let centers = Centers {
            map_name: "test".into(),
            label: "区中心".into(),
            coordinate_system: "WGS84".into(),
            source: "fixture".into(),
            points: (0..16)
                .map(|i| Center {
                    adcode: i.to_string(),
                    name: format!("中心{i}区"),
                    center_wgs84: [105. + (i % 4) as f64 * 0.15, 25. + (i / 4) as f64 * 0.15],
                })
                .collect(),
        };
        let mut im = Image::new(1104, 1002, [104, 151, 89]);
        let report = draw(&mut im, &centers, [100., 20., 110., 30.], 800, 800).unwrap();
        assert_eq!(report["visible_centers"], 16);
        let boxes = report["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let b = p["label_box"].as_array().unwrap();
                Box2 {
                    x: b[0].as_i64().unwrap() as isize,
                    y: b[1].as_i64().unwrap() as isize,
                    w: b[2].as_i64().unwrap() as isize,
                    h: b[3].as_i64().unwrap() as isize,
                }
            })
            .collect::<Vec<_>>();
        for (i, &a) in boxes.iter().enumerate() {
            for &b in &boxes[..i] {
                assert_eq!(a.overlap(b), 0);
            }
        }
        for p in centers.points {
            let [x, y] = project(p.center_wgs84, [100., 20., 110., 30.], 800, 800).unwrap();
            let i = (y as usize * im.width + x as usize) * 3;
            assert_eq!(&im.rgb[i..i + 3], &DOT);
        }
    }
}
