//! Height-field sampling, row orientation and grid traversal.

// Preserve the established chamfer weight exactly for blueprint compatibility.
#[allow(clippy::approx_constant)]
pub fn distance(mask: &[bool], n: usize) -> Vec<f64> {
    let h = mask.len() / n;
    let mut d = mask
        .iter()
        .map(|&v| if v { 0. } else { 1e9 })
        .collect::<Vec<f64>>();
    for y in 0..h {
        for x in 0..n {
            let i = y * n + x;
            if y > 0 {
                d[i] = d[i].min(d[i - n] + 1.);
                if x > 0 {
                    d[i] = d[i].min(d[i - n - 1] + 1.4142);
                }
                if x + 1 < n {
                    d[i] = d[i].min(d[i - n + 1] + 1.4142);
                }
            }
        }
        for x in 1..n {
            let i = y * n + x;
            d[i] = d[i].min(d[i - 1] + 1.);
        }
    }
    for y in (0..h).rev() {
        for x in 0..n {
            let i = y * n + x;
            if y + 1 < h {
                d[i] = d[i].min(d[i + n] + 1.);
                if x > 0 {
                    d[i] = d[i].min(d[i + n - 1] + 1.4142);
                }
                if x + 1 < n {
                    d[i] = d[i].min(d[i + n + 1] + 1.4142);
                }
            }
        }
        for x in (0..n - 1).rev() {
            let i = y * n + x;
            d[i] = d[i].min(d[i + 1] + 1.);
        }
    }
    d
}
pub fn gradient(z: &[f64], n: usize) -> Vec<f64> {
    let h = z.len() / n;
    (0..z.len())
        .map(|i| {
            let x = i % n;
            let y = i / n;
            let gx = if x == 0 {
                z[i + 1] - z[i]
            } else if x == n - 1 {
                z[i] - z[i - 1]
            } else {
                (z[i + 1] - z[i - 1]) * 0.5
            };
            let gy = if y == 0 {
                z[i + n] - z[i]
            } else if y == h - 1 {
                z[i] - z[i - n]
            } else {
                (z[i + n] - z[i - n]) * 0.5
            };
            (gx * gx + gy * gy).sqrt()
        })
        .collect()
}
pub fn edge(i: usize, n: usize, border: usize) -> f64 {
    edge_rect(i, n, n, border)
}
pub fn edge_rect(i: usize, n: usize, h: usize, border: usize) -> f64 {
    let x = (i % n) as isize;
    let y = (i / n) as isize;
    let b = border as isize;
    [x - b, y - b, n as isize - 1 - b - x, h as isize - 1 - b - y]
        .into_iter()
        .min()
        .unwrap() as f64
}
pub fn disk(i: usize, n: usize, c: [f64; 2], r: f64) -> bool {
    ((i % n) as f64 - c[0]).powi(2) + ((i / n) as f64 - c[1]).powi(2) <= r * r
}
pub fn flip<T: Clone>(data: &[T], n: usize) -> Vec<T> {
    data.chunks(n)
        .rev()
        .flat_map(|r| r.iter().cloned())
        .collect()
}

/// Sample a rectangular height field with clamped bilinear interpolation.
pub(crate) fn sample(z: &[f64], n: usize, x: f64, y: f64) -> f64 {
    let x = x.clamp(0., (n - 1) as f64);
    let h = z.len() / n;
    let y = y.clamp(0., (h - 1) as f64);
    let ix = x.floor() as usize;
    let iy = y.floor() as usize;
    let jx = (ix + 1).min(n - 1);
    let jy = (iy + 1).min(h - 1);
    let ax = x - ix as f64;
    let ay = y - iy as f64;
    (z[iy * n + ix] * (1. - ax) + z[iy * n + jx] * ax) * (1. - ay)
        + (z[jy * n + ix] * (1. - ax) + z[jy * n + jx] * ax) * ay
}

/// Visit adjacent square-grid cells in left, right, up, down order.
pub fn neighbors(i: usize, n: usize) -> impl Iterator<Item = usize> {
    neighbors_rect(i, n, n)
}

/// Visit a rectangular grid without wrapping across the end of a row.
pub(crate) fn neighbors_rect(i: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let x = i % width;
    let y = i / width;
    [
        (x > 0).then(|| i - 1),
        (x + 1 < width).then_some(i + 1),
        (y > 0).then(|| i - width),
        (y + 1 < height).then_some(i + width),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangular_neighbors_keep_axis_bounds_and_visit_order() {
        for (index, expected) in [
            (0, vec![1, 3]),
            (1, vec![0, 2, 4]),
            (2, vec![1, 5]),
            (3, vec![4, 0]),
            (4, vec![3, 5, 1]),
            (5, vec![4, 2]),
        ] {
            assert_eq!(neighbors_rect(index, 3, 2).collect::<Vec<_>>(), expected);
        }
        assert_eq!(neighbors_rect(3, 1, 5).collect::<Vec<_>>(), vec![2, 4]);
        assert_eq!(neighbors_rect(3, 5, 1).collect::<Vec<_>>(), vec![2, 4]);
        assert_eq!(neighbors(4, 3).collect::<Vec<_>>(), vec![3, 5, 1, 7]);
    }

    #[test]
    fn height_sampling_interpolates_both_axes_and_clamps_outside_edges() {
        let heights = [0., 10., 20., 40.];
        assert_eq!(sample(&heights, 2, 0.25, 0.75), 19.375);
        assert_eq!(sample(&heights, 2, 0.5, 0.5), 17.5);
        assert_eq!(sample(&heights, 2, -4., 0.5), 10.);
        assert_eq!(sample(&heights, 2, 3., 2.), 40.);
    }
}
