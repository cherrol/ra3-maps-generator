//! 高程底图、分层预览与公共图例绘制。
use super::{COLORS, Frame};
use crate::{
    render::{self, Image},
    terrain::grid::neighbors_rect as neighbors,
};

fn rect(im: &mut Image, x: usize, y: usize, w: usize, h: usize, c: [u8; 3]) {
    for yy in y..(y + h).min(im.height) {
        for xx in x..(x + w).min(im.width) {
            im.pixel(xx as isize, yy as isize, c);
        }
    }
}
fn outline(mask: &[bool], w: usize, h: usize) -> Vec<f64> {
    let mut out = vec![f64::INFINITY; w * h];
    // Draw precisely on the transition between inside and outside cell centers.
    for y in 0..h {
        for x in 0..w {
            for (dx, dy) in [(1usize, 0usize), (0, 1)] {
                if x + dx >= w || y + dy >= h || mask[y * w + x] == mask[(y + dy) * w + x + dx] {
                    continue;
                }
                let cx = x as f64 + dx as f64 * 0.5;
                let cy = y as f64 + dy as f64 * 0.5;
                for yy in y.saturating_sub(2)..=(y + 3).min(h - 1) {
                    for xx in x.saturating_sub(2)..=(x + 3).min(w - 1) {
                        let (ax, ay) = if dx == 1 {
                            (cx, cy - 0.5)
                        } else {
                            (cx - 0.5, cy)
                        };
                        let (bx, by) = if dx == 1 {
                            (cx, cy + 0.5)
                        } else {
                            (cx + 0.5, cy)
                        };
                        let distance = (xx as f64 - (xx as f64).clamp(ax, bx))
                            .hypot(yy as f64 - (yy as f64).clamp(ay, by));
                        let i = yy * w + xx;
                        out[i] = out[i].min(distance);
                    }
                }
            }
        }
    }
    out
}
fn relief(z: f64, dx: f64, dy: f64) -> [u8; 3] {
    let stops = [
        (0., [88, 132, 81]),
        (1000., [141, 168, 85]),
        (1800., [188, 181, 105]),
        (2600., [170, 138, 100]),
        (4000., [151, 141, 131]),
        (6000., [232, 235, 230]),
    ];
    let mut j = 0;
    while j + 2 < stops.len() && z > stops[j + 1].0 {
        j += 1;
    }
    let t = ((z - stops[j].0) / (stops[j + 1].0 - stops[j].0)).clamp(0., 1.);
    let shade = (0.91 + (-dx - dy) / 480.).clamp(0.48, 1.22);
    std::array::from_fn(|c| {
        ((stops[j].1[c] as f64 * (1. - t) + stops[j + 1].1[c] as f64 * t) * shade).clamp(0., 255.)
            as u8
    })
}
pub(super) struct RenderInput<'a> {
    pub(super) name: &'a str,
    pub(super) dem: &'a [f64],
    pub(super) layers: &'a [u8],
    pub(super) water: &'a [bool],
    pub(super) province: &'a [bool],
    pub(super) frame: &'a Frame,
    pub(super) border: usize,
    pub(super) heights: &'a [f64],
    pub(super) thresholds: &'a [f64],
}

pub(super) fn render_image(input: &RenderInput<'_>, mode: u8) -> Image {
    let RenderInput {
        name,
        dem,
        layers,
        water,
        province,
        frame: f,
        border: b,
        heights,
        thresholds,
    } = *input;
    let (w, h) = (f.w - 2 * b, f.h - 2 * b);
    let (ox, oy) = (28, 112);
    let mut image = Image::new(w + 304, (h + 202).max(802), [26, 32, 37]);
    let font = render::font();
    image.text(
        font.as_ref(),
        28,
        18,
        &format!("九州大地·{name}"),
        32.,
        [238, 236, 224],
    );
    image.text(
        font.as_ref(),
        29,
        65,
        match mode {
            0 => "真实高程 · 未做游戏分层",
            1 => "地形预览 · 全图统一高度分层",
            _ => "地形预览 · 全图统一分层与边界",
        },
        18.,
        [168, 183, 184],
    );
    let boundary = if mode == 2 {
        outline(province, f.w, f.h)
    } else {
        Vec::new()
    };
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x + b, y + b);
            let i = sy * f.w + sx;
            let mut color = if water[i] {
                COLORS[0]
            } else if mode == 0 {
                relief(
                    dem[i],
                    (dem[sy * f.w + (sx + 1).min(f.w - 1)] - dem[sy * f.w + sx.saturating_sub(1)])
                        / 2.,
                    (dem[(sy + 1).min(f.h - 1) * f.w + sx] - dem[sy.saturating_sub(1) * f.w + sx])
                        / 2.,
                )
            } else {
                let c = COLORS[layers[i] as usize];
                if neighbors(i, f.w, f.h).any(|k| !water[k] && layers[k] != layers[i]) {
                    c.map(|v| (v as f64 * 0.77) as u8)
                } else {
                    c
                }
            };
            if mode == 2 {
                for (ink, a) in [
                    ([24, 28, 29], (2. - boundary[i]).clamp(0., 1.)),
                    ([255, 222, 112], (1.4 - boundary[i]).clamp(0., 1.)),
                ] {
                    color = std::array::from_fn(|c| {
                        (color[c] as f64 * (1. - a) + ink[c] as f64 * a).round() as u8
                    });
                }
                if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                    color = [255, 111, 99];
                }
            }
            image.pixel((ox + x) as isize, (oy + y) as isize, color);
        }
    }
    let left = w + 54;
    image.text(
        font.as_ref(),
        left as isize,
        120,
        if mode == 0 {
            "海拔 / 米"
        } else {
            "游戏高度层"
        },
        21.,
        [232, 231, 220],
    );
    if mode == 0 {
        for (j, (height, color)) in [
            (0, [88, 132, 81]),
            (1000, [141, 168, 85]),
            (1800, [188, 181, 105]),
            (2600, [170, 138, 100]),
            (4000, [151, 141, 131]),
            (6000, [232, 235, 230]),
        ]
        .iter()
        .enumerate()
        {
            rect(&mut image, left, 166 + j * 45, 22, 22, *color);
            image.text(
                font.as_ref(),
                (left + 35) as isize,
                (163 + j * 45) as isize,
                &height.to_string(),
                18.,
                [215, 221, 215],
            );
        }
    } else {
        let active: Vec<_> = (1..=5)
            .filter(|&l| (b..f.h - b).any(|y| (b..f.w - b).any(|x| layers[y * f.w + x] == l as u8)))
            .collect();
        for (j, &l) in active.iter().enumerate() {
            let y = 166 + j * 80;
            rect(&mut image, left, y, 22, 22, COLORS[l]);
            image.text(
                font.as_ref(),
                (left + 35) as isize,
                (y - 4) as isize,
                &format!("第 {l} 层"),
                19.,
                [227, 229, 217],
            );
            image.text(
                font.as_ref(),
                (left + 35) as isize,
                (y + 22) as isize,
                &format!(
                    "高度 {}{}",
                    heights[l - 1],
                    if l == 5 { " · 禁行设计" } else { "" }
                ),
                14.,
                [160, 176, 176],
            );
            let altitude = match l {
                1 => format!("海拔 < {:.0} 米", thresholds[0]),
                5 => format!("海拔 ≥ {:.0} 米", thresholds[3]),
                _ => format!("海拔 {:.0}–{:.0} 米", thresholds[l - 2], thresholds[l - 1]),
            };
            image.text(
                font.as_ref(),
                (left + 35) as isize,
                (y + 43) as isize,
                &altitude,
                12.,
                [160, 176, 176],
            );
        }
    }
    let active_count = (1..=5)
        .filter(|&l| (b..f.h - b).any(|y| (b..f.w - b).any(|x| layers[y * f.w + x] == l)))
        .count();
    let ly = if mode == 0 {
        466
    } else {
        192 + active_count * 80
    };
    rect(&mut image, left, ly, 22, 22, COLORS[0]);
    image.text(
        font.as_ref(),
        (left + 35) as isize,
        (ly - 3) as isize,
        "水域",
        18.,
        [215, 224, 220],
    );
    image.text(
        font.as_ref(),
        left as isize,
        (ly + 59) as isize,
        "N ↑ 北",
        23.,
        [226, 229, 217],
    );
    image.text(
        font.as_ref(),
        left as isize,
        (ly + 109) as isize,
        "整张地图统一取景",
        14.,
        [160, 176, 176],
    );
    if mode == 2 {
        rect(&mut image, left, ly + 156, 24, 2, [255, 222, 112]);
        image.text(
            font.as_ref(),
            (left + 35) as isize,
            (ly + 145) as isize,
            "蓝图主体边界",
            15.,
            [210, 217, 208],
        );
        rect(&mut image, left, ly + 194, 24, 2, [255, 111, 99]);
        image.text(
            font.as_ref(),
            (left + 35) as isize,
            (ly + 183) as isize,
            "游戏地图边界",
            15.,
            [210, 217, 208],
        );
    }
    image.text(
        font.as_ref(),
        28,
        (h + 126) as isize,
        "北朝上 · 同一取景 · 全图地形",
        17.,
        [205, 215, 207],
    );
    image.text(
        font.as_ref(),
        28,
        (h + 155) as isize,
        "地形设计预览 · 地理参考资料由本次制作提供",
        13.,
        [142, 160, 162],
    );
    image
}
