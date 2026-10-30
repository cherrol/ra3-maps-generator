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

/// Exact squared Euclidean distance to blocked sample centres on a rectangular grid.
/// Existing `distance` remains the compatibility chamfer transform used by authoring.
/// No obstacle means infinity; callers measuring occupied grid squares must subtract
/// their half diagonal from the square root to obtain a conservative radius.
pub fn exact_squared_distance(mask: &[bool], width: usize) -> crate::Result<Vec<f64>> {
    if width == 0 || mask.is_empty() || !mask.len().is_multiple_of(width) {
        return Err(crate::err(
            "exact distance requires a nonempty rectangular grid",
        ));
    }
    let height = mask.len() / width;
    let mut horizontal = vec![f64::INFINITY; mask.len()];
    for (source, target) in mask.chunks(width).zip(horizontal.chunks_mut(width)) {
        let values = source
            .iter()
            .map(|&blocked| if blocked { 0. } else { f64::INFINITY })
            .collect::<Vec<_>>();
        squared_distance_line(&values, target);
    }
    let mut result = vec![f64::INFINITY; mask.len()];
    let mut column = vec![0.; height];
    let mut transformed = vec![0.; height];
    for x in 0..width {
        for y in 0..height {
            column[y] = horizontal[y * width + x];
        }
        squared_distance_line(&column, &mut transformed);
        for y in 0..height {
            result[y * width + x] = transformed[y];
        }
    }
    Ok(result)
}

// Lower envelope of parabolas f[i] + (x-i)^2; finite sites only avoid INF-INF.
fn squared_distance_line(values: &[f64], out: &mut [f64]) {
    let mut sites = Vec::<usize>::with_capacity(values.len());
    let mut boundaries = Vec::<f64>::with_capacity(values.len() + 1);
    for (q, &value) in values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
    {
        let mut boundary = f64::NEG_INFINITY;
        while let Some(&last) = sites.last() {
            let qf = q as f64;
            let lastf = last as f64;
            boundary = ((value + qf * qf) - (values[last] + lastf * lastf)) / (2. * (qf - lastf));
            if boundary > *boundaries.last().expect("each site has a boundary") {
                break;
            }
            sites.pop();
            boundaries.pop();
        }
        if sites.is_empty() {
            boundary = f64::NEG_INFINITY;
        }
        sites.push(q);
        boundaries.push(boundary);
    }
    if sites.is_empty() {
        out.fill(f64::INFINITY);
        return;
    }
    let mut site = 0;
    for (x, value) in out.iter_mut().enumerate() {
        while site + 1 < sites.len() && boundaries[site + 1] < x as f64 {
            site += 1;
        }
        *value = (x as f64 - sites[site] as f64).powi(2) + values[sites[site]];
    }
}

#[cfg(test)]
mod exact_distance_tests {
    use super::*;
    #[test]
    fn exact_transform_matches_brute_force_on_rectangular_and_single_axis_grids() {
        for (width, height) in [(1, 9), (9, 1), (7, 5), (5, 7), (11, 8)] {
            for seed in 0..8 {
                let mask = (0..width * height)
                    .map(|i| (i * 13 + seed * 7) % 17 < 3)
                    .collect::<Vec<_>>();
                let exact = exact_squared_distance(&mask, width).unwrap();
                for (i, &actual) in exact.iter().enumerate() {
                    let expected = mask
                        .iter()
                        .enumerate()
                        .filter(|(_, yes)| **yes)
                        .map(|(j, _)| {
                            let dx = (i % width) as f64 - (j % width) as f64;
                            let dy = (i / width) as f64 - (j / width) as f64;
                            dx * dx + dy * dy
                        })
                        .fold(f64::INFINITY, f64::min);
                    assert_eq!(actual, expected, "{width}x{height}, seed {seed}, cell {i}");
                }
            }
        }
    }
    #[test]
    fn no_obstacles_and_all_obstacles_are_explicit_and_invalid_shapes_fail() {
        assert!(
            exact_squared_distance(&[false; 15], 5)
                .unwrap()
                .iter()
                .all(|v| v.is_infinite())
        );
        assert_eq!(
            exact_squared_distance(&[true; 15], 5).unwrap(),
            vec![0.; 15]
        );
        assert!(exact_squared_distance(&[], 1).is_err());
        assert!(exact_squared_distance(&[true], 0).is_err());
        assert!(exact_squared_distance(&[true; 5], 2).is_err());
    }
    #[test]
    fn diagonal_direction_uses_true_euclidean_distance_instead_of_chamfer() {
        let mut mask = vec![false; 100];
        mask[0] = true;
        let exact = exact_squared_distance(&mask, 10).unwrap();
        assert_eq!(exact[2 * 10 + 5], 29.);
        assert!(distance(&mask, 10)[2 * 10 + 5] > exact[2 * 10 + 5].sqrt());
    }
}
