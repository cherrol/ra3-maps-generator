//! Number every authored object and ramp without changing the playable map.
use super::Image;
use crate::{
    Result,
    map::{self, Value},
    project::{Project, write_json},
    terrain::{Layout, ObjectSpec, Terrain, number},
};
use serde::Serialize;

const BACKGROUND: [u8; 3] = [22, 29, 38];
const TOP: isize = 110;
const LABEL_SIZE: f32 = 18.;
// Category names stay in the index even on machines without a Chinese font.
const KINDS: [(&str, &str, [u8; 3]); 13] = [
    ("矿脉", "Ore", [255, 215, 0]),
    ("油井", "Oil", [255, 160, 60]),
    ("科技建筑", "Tech buildings", [215, 145, 255]),
    ("房屋", "Houses", [235, 195, 140]),
    ("树木", "Trees", [90, 230, 110]),
    ("灌木", "Bushes", [180, 215, 90]),
    ("石头", "Rocks", [210, 215, 225]),
    ("废墟", "Ruins", [210, 130, 100]),
    ("出生点", "Player starts", [80, 185, 255]),
    ("航点", "Waypoints", [125, 205, 215]),
    ("基地建筑", "Construction yards", [255, 130, 175]),
    ("其他对象", "Other objects", [160, 175, 185]),
    ("斜坡", "Ramps", [255, 80, 70]),
];
const RAMP: usize = KINDS.len() - 1;

#[derive(Serialize)]
struct Entry {
    id: usize,
    category: &'static str,
    cell: [f64; 2],
    source_pointer: String,
    /// All matching coordinate fields that should be kept in sync when moving.
    position_pointers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    waypoint_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    angle_radians: Option<f32>,
    #[serde(skip)]
    kind: usize,
}

fn kind(o: &ObjectSpec, layout: &Layout) -> usize {
    let name = o.type_name.as_str();
    if name.contains("Waypoint") {
        let waypoint = map::prop(&o.props, "waypointName").and_then(Value::text);
        return if waypoint.is_some_and(|s| s.starts_with("Player_") && s.contains("_Start")) {
            8
        } else {
            9
        };
    }
    if name.contains("Collapse") {
        7
    } else if name == "OreNode" {
        0
    } else if name == "OilDerrick" {
        1
    } else if name.contains("TechStructure") || name.ends_with("NavalYard") {
        2
    } else if name.starts_with("ST_") || layout.house_cells.contains(&o.cell) {
        3
    } else if name.contains("Tree") {
        4
    } else if name.contains("Bush") {
        5
    } else if name.contains("Rock") {
        6
    } else if name.contains("ConstructionYard") {
        10
    } else {
        11
    }
}

fn entries(layout: &Layout) -> Vec<Entry> {
    let mut entries = Vec::new();
    for (i, o) in layout.objects.iter().enumerate() {
        let kind = kind(o, layout);
        let mut position_pointers = vec![format!("/objects/{i}/cell")];
        for (field, cells) in [
            ("base_cells", &layout.base_cells),
            ("base_pad_cells", &layout.base_pad_cells),
            ("ore_cells", &layout.ore_cells),
            ("derrick_cells", &layout.derrick_cells),
            ("house_cells", &layout.house_cells),
            ("post_cells", &layout.post_cells),
            ("garage_cells", &layout.garage_cells),
            ("airport_cells", &layout.airport_cells),
            ("hospital_cells", &layout.hospital_cells),
            ("academy_cells", &layout.academy_cells),
            ("shipyard_cells", &layout.shipyard_cells),
            ("bridge_cells", &layout.bridge_cells),
        ] {
            for (j, cell) in cells.iter().enumerate() {
                if (cell[0] - o.cell[0]).hypot(cell[1] - o.cell[1]) <= 0.001 {
                    position_pointers.push(format!("/{field}/{j}"));
                }
            }
        }
        entries.push(Entry {
            id: i + 1,
            category: KINDS[kind].0,
            cell: o.cell,
            source_pointer: format!("/objects/{i}"),
            position_pointers,
            object_type: Some(o.type_name.clone()),
            unique_id: map::prop(&o.props, "uniqueID")
                .and_then(Value::text)
                .map(str::to_owned),
            waypoint_name: map::prop(&o.props, "waypointName")
                .and_then(Value::text)
                .map(str::to_owned),
            angle_radians: Some(o.angle),
            kind,
        });
    }
    for (i, ramp) in layout.ramps.iter().enumerate() {
        entries.push(Entry {
            id: layout.objects.len() + i + 1,
            category: KINDS[RAMP].0,
            cell: [number(ramp, "x", 0.), number(ramp, "y", 0.)],
            source_pointer: format!("/ramps/{i}"),
            position_pointers: vec![format!("/ramps/{i}/x"), format!("/ramps/{i}/y")],
            object_type: None,
            unique_id: None,
            waypoint_name: None,
            angle_radians: None,
            kind: RAMP,
        });
    }
    entries
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: isize,
    y: isize,
    w: isize,
    h: isize,
}
impl Rect {
    fn overlap(self, other: Self) -> isize {
        (self.x + self.w)
            .min(other.x + other.w)
            .saturating_sub(self.x.max(other.x))
            .max(0)
            * (self.y + self.h)
                .min(other.y + other.h)
                .saturating_sub(self.y.max(other.y))
                .max(0)
    }
    fn draw(self, image: &mut Image, color: [u8; 3]) {
        for y in self.y..self.y + self.h {
            for x in self.x..self.x + self.w {
                image.pixel(x, y, color);
            }
        }
    }
}

fn line(image: &mut Image, from: (isize, isize), to: (isize, isize), color: [u8; 3]) {
    let steps = (to.0 - from.0).abs().max((to.1 - from.1).abs()).max(1);
    for i in 0..=steps {
        image.pixel(
            from.0 + (to.0 - from.0) * i / steps,
            from.1 + (to.1 - from.1) * i / steps,
            color,
        );
    }
}

fn label_rect(point: (isize, isize), width: isize, side: isize, occupied: &[Rect]) -> Rect {
    let mut best = Rect {
        x: 0,
        y: TOP,
        w: width,
        h: 26,
    };
    let mut best_score = isize::MAX;
    // Search outward deterministically. Include map markers in occupied so a
    // displaced label cannot hide a neighboring tree, rock or resource point.
    for radius in [12, 30, 50, 75, 105, 145, 195] {
        for (dx, dy) in [
            (1, -1),
            (1, 1),
            (-1, -1),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, 0),
            (-1, 0),
        ] {
            let rect = Rect {
                x: (point.0 + dx * radius
                    - if dx < 0 {
                        width
                    } else if dx == 0 {
                        width / 2
                    } else {
                        0
                    })
                .clamp(0, side - width),
                y: (point.1 + dy * radius
                    - if dy < 0 {
                        26
                    } else if dy == 0 {
                        13
                    } else {
                        0
                    })
                .clamp(TOP, TOP + side - 26),
                w: width,
                h: 26,
            };
            let overlap: isize = occupied.iter().map(|r| rect.overlap(*r)).sum();
            let score = overlap * 10000 + radius;
            if score < best_score {
                best = rect;
                best_score = score;
            }
            if overlap == 0 {
                return best;
            }
        }
    }
    best
}

fn viewport(entries: &[Entry], grid: usize) -> ([f64; 2], f64) {
    let mut min = [0_f64; 2];
    let mut max = [grid as f64; 2];
    for entry in entries {
        for axis in 0..2 {
            if entry.cell[axis] < 0. {
                min[axis] = min[axis].min(entry.cell[axis] - 16.);
            }
            if entry.cell[axis] >= grid as f64 {
                max[axis] = max[axis].max(entry.cell[axis] + 16.);
            }
        }
    }
    (min, (max[0] - min[0]).max(max[1] - min[1]))
}

pub(super) fn write(
    base: &Image,
    t: &Terrain,
    p: &Project,
    font: Option<&fontdue::Font>,
) -> Result<()> {
    let entries = entries(&t.layout);
    let side = (t.n * 4).clamp(2048, 4096);
    let (origin, span) = viewport(&entries, t.n);
    let scale = side as f64 / span;
    let off_grid = entries
        .iter()
        .filter(|e| e.cell.iter().any(|v| *v < 0. || *v >= t.n as f64))
        .count();
    let mut image = Image::new(side + 400, side + TOP as usize + 50, BACKGROUND);
    let terrain_side = (t.n as f64 * scale).round() as usize;
    let terrain = base.resize(terrain_side, terrain_side);
    let left = (-origin[0] * scale).round() as usize;
    let top = (-origin[1] * scale).round() as usize + TOP as usize;
    for y in 0..terrain_side {
        let i = ((y + top) * image.width + left) * 3;
        image.rgb[i..i + terrain_side * 3]
            .copy_from_slice(&terrain.rgb[y * terrain_side * 3..(y + 1) * terrain_side * 3]);
    }
    let chinese = font.is_some_and(|f| f.lookup_glyph_index('图') != 0);
    let title = if chinese {
        format!("{} · 全要素编号", p.name)
    } else {
        format!("LAYOUT OVERVIEW - {}", p.version)
    };
    image.text(font, 24, 12, &title, 32., [255, 226, 120]);
    let subtitle = if chinese {
        format!(
            "{} 个对象 + {} 条斜坡 · 北朝上 · 引线连接编号与实际位置",
            t.layout.objects.len(),
            t.layout.ramps.len()
        )
    } else {
        format!(
            "{} objects + {} ramps | North up | Lines connect IDs to exact positions",
            t.layout.objects.len(),
            t.layout.ramps.len()
        )
    };
    image.text(font, 24, 62, &subtitle, 22., [210, 220, 235]);
    let point = |cell: [f64; 2]| {
        (
            ((cell[0] - origin[0]) * scale).round() as isize,
            ((cell[1] - origin[1]) * scale).round() as isize + TOP,
        )
    };
    // The recipe uses an axis-aligned w/h box; ux/uy describe its slope axis.
    for ramp in &t.layout.ramps {
        let (ux, uy, _, _) = crate::terrain::ramp_axes(ramp)?;
        let half_width = number(ramp, "w", 32.) * 0.5;
        let half_height = number(ramp, "h", 32.) * 0.5;
        let x = number(ramp, "x", 0.);
        let y = number(ramp, "y", 0.);
        let corners = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
            .map(|(u, v)| point([x + u * half_width, y + v * half_height]));
        for i in 0..4 {
            line(&mut image, corners[i], corners[(i + 1) % 4], KINDS[RAMP].2);
        }
        let length = half_width.min(half_height);
        line(
            &mut image,
            point([x - ux * length, y - uy * length]),
            point([x + ux * length, y + uy * length]),
            KINDS[RAMP].2,
        );
    }
    let mut occupied: Vec<_> = entries
        .iter()
        .map(|e| {
            let (x, y) = point(e.cell);
            Rect {
                x: x - 7,
                y: y - 7,
                w: 15,
                h: 15,
            }
        })
        .collect();
    let mut labels = Vec::new();
    for entry in &entries {
        let text = entry.id.to_string();
        let width = font.map_or(text.len() as isize * 10, |f| {
            text.chars()
                .map(|c| f.metrics(c, LABEL_SIZE).advance_width.round() as isize)
                .sum()
        }) + 10;
        let rect = label_rect(point(entry.cell), width, side as isize, &occupied);
        occupied.push(Rect {
            x: rect.x - 3,
            y: rect.y - 3,
            w: rect.w + 6,
            h: rect.h + 6,
        });
        let end = (
            point(entry.cell).0.clamp(rect.x, rect.x + rect.w - 1),
            point(entry.cell).1.clamp(rect.y, rect.y + rect.h - 1),
        );
        line(&mut image, point(entry.cell), end, KINDS[entry.kind].2);
        labels.push((rect, text));
    }
    for (entry, (rect, text)) in entries.iter().zip(labels) {
        let color = KINDS[entry.kind].2;
        rect.draw(&mut image, color);
        Rect {
            x: rect.x + 1,
            y: rect.y + 1,
            w: rect.w - 2,
            h: rect.h - 2,
        }
        .draw(&mut image, BACKGROUND);
        image.text(font, rect.x + 5, rect.y + 3, &text, LABEL_SIZE, color);
        let (x, y) = point(entry.cell);
        image.circle(x, y, 5, color);
    }
    let legend_x = side as isize + 24;
    image.text(
        font,
        legend_x,
        TOP + 8,
        if chinese {
            "分类 / 数量"
        } else {
            "CATEGORY / COUNT"
        },
        24.,
        [235, 240, 245],
    );
    for (i, (zh, en, color)) in KINDS.iter().enumerate() {
        let count = entries.iter().filter(|e| e.kind == i).count();
        let y = TOP + 65 + i as isize * 42;
        image.circle(legend_x + 6, y + 12, 6, *color);
        image.text(
            font,
            legend_x + 24,
            y,
            &format!("{}  {}", if chinese { zh } else { en }, count),
            21.,
            *color,
        );
    }
    for (i, text) in [
        "layout-index.json",
        "ID -> source_pointer",
        "cell: x east, y south",
        "objects: 1 .. N",
        "ramps: N+1 .. N+R",
    ]
    .iter()
    .enumerate()
    {
        image.text(
            font,
            legend_x,
            TOP + 680 + i as isize * 34,
            text,
            19.,
            [195, 205, 220],
        );
    }
    if off_grid > 0 {
        let text = if chinese {
            format!("图外对象：{off_grid}（灰底区域）")
        } else {
            format!("Off-grid objects: {off_grid}")
        };
        image.text(font, legend_x, TOP + 890, &text, 19., [235, 195, 140]);
    }
    image.png(&p.assets.join("preview--layout.png"))?;
    write_json(
        &p.assets.join("layout-index.json"),
        &serde_json::json!({
            "schema_version": 1,
            "map_id": p.id,
            "version": p.version,
            "layout": "source/layout.json",
            "recipe_layout": p.string("LAYOUT", "layout.json"),
            "coordinates": "Full-grid cells; x east, y south; row 0 is north. Angles are radians.",
        "viewport": {"origin_cell": origin, "span_cells": span, "grid_size": t.n, "off_grid_entries": off_grid},
            "numbering": "Objects in layout order, then ramps. Moving coordinates preserves IDs; inserting, deleting or reordering entries can change IDs.",
            "entries": entries,
        }),
    )?;
    println!(
        "全要素编号预览：{}（{} 项）；索引：layout-index.json",
        p.assets.join("preview--layout.png").display(),
        entries.len()
    );
    Ok(())
}
