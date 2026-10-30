//! Mountain passes: a steady central grade, tapered side cuts and local materials.
//! Reference measurements and WorldBuilder views: docs/mountain_ramps.md.
use super::*;

/// A constant central grade with short, tangent-continuous landings. Keeping the
/// declared length avoids both a steep cubic hump and an extended artificial road.
pub(crate) fn progress(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    let ease = 0.15;
    if t < ease {
        t * t / (2. * ease * (1. - ease))
    } else if t > 1. - ease {
        1. - (1. - t).powi(2) / (2. * ease * (1. - ease))
    } else {
        (t - ease * 0.5) / (1. - ease)
    }
}

pub(crate) fn carve(z: &mut [f64], n: usize, r: &Value, scale: f64) -> Result<()> {
    let locks = vec![false; z.len()];
    let patch = sculpt(z, (z, z), n, r, r, &locks, scale)
        .ok_or_else(|| err("mountain ramp crosses a third terrain level"))?;
    for (i, h) in patch {
        z[i] = h;
    }
    Ok(())
}

pub(super) fn sculpt(
    z: &[f64],
    ground: (&[f64], &[f64]),
    n: usize,
    old: &Value,
    new: &Value,
    locks: &[bool],
    scale: f64,
) -> Option<Vec<(usize, f64)>> {
    let (base, platforms) = ground;
    let (_, _, lo, hi) = terrain::ramp_axes(new).ok()?;
    let width = terrain::ramp_width(new).ok()? * 0.5;
    // Bilinear sampling of a diagonal lane needs a one-cell margin around the
    // declared full width, before the shoulders begin to contract.
    let core = width + 1.;
    let shoulder = 4.;
    let mut patch = Vec::new();
    let cx = number(new, "x", 0.);
    let cy = number(new, "y", 0.);
    let radius = (half(new) + core + shoulder + 8.).ceil() as isize;
    for y in (cy as isize - radius).max(0)..=(cy as isize + radius).min((z.len() / n) as isize - 1)
    {
        for x in (cx as isize - radius).max(0)..=(cx as isize + radius).min(n as isize - 1) {
            let i = y as usize * n + x as usize;
            let (u, v) = frame(new, x as f64, y as f64);
            let old_inside = old_foot(old, x as f64, y as f64, 0.);
            let s = ((v.abs() - core) / shoulder).clamp(0., 1.);
            let weight =
                (1. - smooth((s - 0.75) * 4.)) * (1. - smooth((u.abs() - half(new) - 3.) / 2.));
            if !old_inside && weight <= 0. {
                continue;
            }
            if weight > 0. && !terrain::ramp_layer_allowed(platforms[i], lo, hi) {
                return None;
            }
            let bg = if old_inside && (base[i] - lo).abs().min((base[i] - hi).abs()) < 3. {
                base[i]
            } else {
                z[i]
            };
            // Contract the *length* toward the cliff at each side. Multiplying
            // a full-length ramp by a hard plateau step leaves a ridge through
            // each shoulder; a contracting cut gives continuous side faces.
            let local_half = half(new) * (1. - smooth(s)) + 0.8 * smooth(s);
            let h = lo + (hi - lo) * progress((u + local_half) / (2. * local_half));
            let value = ((bg * (1. - weight) + h * weight) / scale).round_ties_even() * scale;
            if (value - z[i]).abs() < scale * 0.5 {
                continue;
            }
            if locks[i] || z[i] < lo - 3. || z[i] > hi + 3. {
                return None;
            }
            patch.push((i, value));
        }
    }
    Some(patch)
}

fn plateau_material(
    tiles: &[u16],
    names: &[String],
    z: &[f64],
    n: usize,
    r: &Value,
    upper: bool,
) -> u16 {
    let (ux, uy, lo, hi) = terrain::ramp_axes(r).unwrap();
    let expected = if upper { hi } else { lo };
    let sign = if upper { 1. } else { -1. };
    let width = terrain::ramp_width(r).unwrap() * 0.5;
    let mut votes = BTreeMap::<u16, usize>::new();
    for u in [half(r) + 7., half(r) + 11., half(r) + 16.] {
        for v in [-width - 9., -width - 5., 0., width + 5., width + 9.] {
            let x = (number(r, "x", 0.) + ux * u * sign - uy * v).round();
            let y = (number(r, "y", 0.) + uy * u * sign + ux * v).round();
            if x < 0. || y < 0. || x >= n as f64 || y >= (z.len() / n) as f64 {
                continue;
            }
            let i = y as usize * n + x as usize;
            let family = tiles[i] / 64;
            let name = &names[family as usize];
            if (z[i] - expected).abs() <= 3.
                && (name.starts_with("Grass_")
                    || name.starts_with("Dirt_")
                    || name.starts_with("Rock_"))
                && ![
                    "Grass_CapeCod01",
                    "Grass_CapeCod03",
                    "Grass_CapeCod07",
                    "Grass_Golf01",
                    "Grass_Golf03",
                    "Grass_Golf05",
                    "Dirt_Yucatan06",
                ]
                .contains(&name.as_str())
            {
                *votes.entry(family).or_default() += 1;
            }
        }
    }
    votes
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(v, _)| v)
        .unwrap_or_else(|| {
            ["Grass_CapeCod05", "Grass_Geneva01", "Grass_NewYork01"]
                .iter()
                .find_map(|s| names.iter().position(|n| n == s))
                .unwrap_or(0) as u16
        })
}

/// Continue the actual plateau materials across the slope. Artificial short
/// soil tracks read as isolated patches; rock belongs only on steep side faces.
pub(crate) fn paint(
    tiles: &mut [u16],
    names: &[String],
    z: &[f64],
    n: usize,
    ramps: (&Value, &Value),
    water: f64,
    changed: &mut [bool],
) {
    let (old, r) = ramps;
    let (_, _, lo, hi) = terrain::ramp_axes(r).unwrap();
    let lower = plateau_material(tiles, names, z, n, r, false);
    let upper = plateau_material(tiles, names, z, n, r, true);
    let pick = |choices: &[&str], fallback| {
        choices
            .iter()
            .find_map(|s| names.iter().position(|n| n == s))
            .map_or(fallback, |i| i as u16)
    };
    let rock = pick(&["Rock_Yucatan01", "Rock_Yucatan02"], lower);
    let sand = pick(&["Sand_Hawaii02"], lower);
    let sea = pick(&["RA3_ShallowSeaFloor", "Reef_Yucatan01"], sand);
    let width = terrain::ramp_width(r).unwrap() * 0.5;
    let cx = number(r, "x", 0.);
    let cy = number(r, "y", 0.);
    let radius = (half(r) + width + 18.).ceil() as isize;
    for y in (cy as isize - radius).max(0)..=(cy as isize + radius).min((z.len() / n) as isize - 1)
    {
        for x in (cx as isize - radius).max(0)..=(cx as isize + radius).min(n as isize - 1) {
            let i = y as usize * n + x as usize;
            let (u, v) = frame(r, x as f64, y as f64);
            let old_material = &names[tiles[i] as usize / 64];
            let apron = old_foot(old, x as f64, y as f64, 12.)
                && ["Dirt_Yucatan06", "Gravel_Yucatan01"].contains(&old_material.as_str());
            let inside = u.abs() < half(r) + 5. && v.abs() < width + 7.;
            if !apron && !inside {
                continue;
            }
            let gx = (sample(z, n, x as f64 + 1., y as f64)
                - sample(z, n, x as f64 - 1., y as f64))
                * 0.5;
            let gy = (sample(z, n, x as f64, y as f64 + 1.)
                - sample(z, n, x as f64, y as f64 - 1.))
                * 0.5;
            let detail = (x as f64 * 0.19 + (y as f64 * 0.13).sin()).sin();
            let mut family = if z[i] < (lo + hi) * 0.5 + detail * 8. {
                lower
            } else {
                upper
            };
            if z[i] < water - 5. {
                family =
                    if old_material.starts_with("Reef_") || old_material == "RA3_ShallowSeaFloor" {
                        tiles[i] / 64
                    } else {
                        sea
                    };
            } else if z[i] < water + 4. {
                family = sand;
            } else if gx.hypot(gy) > 12. {
                family = rock;
            }
            tiles[i] = family * 64 + tiles[i] % 64;
            // Also remove obsolete blends inside the former apron where the
            // base family happens to match the restored plateau already.
            changed[i] = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_profile_defaults_to_grass_slopes_and_preserves_baked_geometry() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut p = Project::load(root, "河南").unwrap();
        p.cfg.as_object_mut().unwrap().remove("RAMP_PROFILE");
        p.cfg["TERRAIN"]["v11_baked"] = json!(false);
        let mut layout:Layout=serde_json::from_value(json!({"schema_version":1,"base_cells":[],"ore_cells":[],"ore_angles":[],"objects":[],"ramps":[{"x":64.,"y":64.,"ux":1.,"uy":0.,"w":25.,"h":25.,"span":25.,"width":25.,"auto":true,"pair":[210.,280.]}]})).unwrap();
        assert!(enabled(&p).unwrap());
        configure_layout(&p, &mut layout).unwrap();
        assert_eq!(layout.ramps[0]["style"], PROFILE);
        assert_eq!(layout.ramps[0]["surface_style"], PROFILE);
        let n = 128;
        let z: Vec<_> = (0..n * n)
            .map(|i| 210. + 70. * progress(((i % n) as f64 - 51.5) / 25.))
            .collect();
        let t = Terrain {
            n,
            raw: z.iter().map(|h| (h / 0.0390625).round() as u16).collect(),
            slope: terrain::gradient(&z, n),
            land: vec![true; n * n],
            z,
            layout: layout.clone(),
        };
        let surface = Surface::generate(&t, &p).unwrap();
        for y in 58..=70 {
            for x in 59..=69 {
                let name = &surface.textures[surface.chosen[y * n + x] as usize][0];
                assert!(
                    name.starts_with("Grass_"),
                    "fresh pipeline painted a road patch: {name}"
                );
            }
        }
        p.cfg["TERRAIN"]["v11_baked"] = json!(true);
        layout.ramps[0]["style"] = json!("natural");
        configure_layout(&p, &mut layout).unwrap();
        assert_eq!(layout.ramps[0]["style"], "natural");
        p.cfg["RAMP_PROFILE"] = json!("legacy");
        assert!(enabled(&p).is_err());
        let mut unmodified = MapFile {
            table: std::collections::BTreeMap::new(),
            order: vec![],
            chunks: vec![],
        };
        let rejected = apply(&p, &mut unmodified, "invalid-profile").err().unwrap();
        assert!(rejected.to_string().contains("RAMP_PROFILE"));
        assert!(unmodified.chunks.is_empty());
        p.cfg["RAMP_PROFILE"] = json!("invalid");
        assert!(enabled(&p).is_err());
    }
    #[test]
    fn grass_slopes_remove_isolated_soil_tracks_and_preserve_tile_phases() {
        let n = 128;
        let r = json!({"x":64.,"y":64.,"ux":1.,"uy":0.,"w":25.,"h":25.,"span":25.,"width":25.,"pair":[210.,280.]});
        let names = vec![
            "Grass_NewYork01".into(),
            "Grass_Geneva01".into(),
            "Dirt_Yucatan02".into(),
            "Rock_Yucatan01".into(),
        ];
        let z: Vec<_> = (0..n * n)
            .map(|i| 210. + 70. * progress(((i % n) as f64 - 51.5) / 25.))
            .collect();
        let mut tiles: Vec<_> = (0..n * n)
            .map(|i| u16::from(i % n >= 64) * 64 + (i % 64) as u16)
            .collect();
        for y in 61..=67 {
            for x in 61..=67 {
                let i = y * n + x;
                tiles[i] = 2 * 64 + tiles[i] % 64;
            }
        }
        let original = tiles.clone();
        let mut changed = vec![false; n * n];
        paint(&mut tiles, &names, &z, n, (&r, &r), 200., &mut changed);
        assert!(
            !tiles.iter().any(|t| t / 64 == 2),
            "isolated dirt track survived"
        );
        assert_eq!(tiles[0], original[0]);
        for (actual, before) in tiles.iter().zip(original) {
            assert_eq!(actual % 64, before % 64);
        }
        for y in 61..=67 {
            for x in 61..=67 {
                assert!(changed[y * n + x]);
            }
        }
    }
    #[test]
    fn steady_grade_has_flat_landings_without_a_central_hump() {
        assert_eq!(progress(-1.), 0.);
        assert_eq!(progress(2.), 1.);
        for i in 0..100 {
            let t = i as f64 / 100.;
            assert!(progress(t + 0.01) >= progress(t));
            assert!((progress(t) + progress(1. - t) - 1.).abs() < 1e-12);
            assert!(70. * (progress(t + 0.01) - progress(t)) / 0.25 < 3.30);
        }
        assert!(progress(0.001) < 0.000004);
        assert!((progress(0.6) - progress(0.5) - (progress(0.5) - progress(0.4))).abs() < 1e-12);
    }
    #[test]
    fn diagonal_mountain_cut_keeps_full_width_flat_mouths_and_protected_ground() {
        let n = 128;
        let r = json!({"x":64.,"y":64.,"ux":0.6,"uy":0.8,"w":25.,"h":25.,"span":25.,"width":25.,"pair":[210.,280.],"style":"mountain"});
        let base: Vec<_> = (0..n * n)
            .map(|i| {
                if frame(&r, (i % n) as f64, (i / n) as f64).0 < 0. {
                    210.
                } else {
                    280.
                }
            })
            .collect();
        let mut locks = vec![false; n * n];
        let patch = sculpt(&base, (&base, &base), n, &r, &r, &locks, 0.0390625).unwrap();
        let mut z = base.clone();
        for &(i, h) in &patch {
            z[i] = h;
        }
        assert!(validate_shape(&z, n, &r, 0).is_some());
        locks[patch[0].0] = true;
        assert!(sculpt(&base, (&base, &base), n, &r, &r, &locks, 0.0390625).is_none());
        let mut third = base.clone();
        third[64 * n + 64] = 350.;
        assert!(
            sculpt(
                &third,
                (&third, &third),
                n,
                &r,
                &r,
                &vec![false; n * n],
                0.0390625
            )
            .is_none()
        );
    }
}
