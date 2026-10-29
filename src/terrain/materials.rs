//! Shared, user-approved land palette. Resource names are stock RA3 assets.
use crate::pipeline::project::Project;

pub const REVISION: &str = "five-layer-2026-10-29";
pub const PRIMARY: [&str; 5] = [
    "Grass_NewYork01",
    "Grass_MtRush04",
    "Grass_CapeCod05",
    "Rock_Yucatan03",
    "Rock_Iceland02",
];
pub const STOCK: [[&str; 3]; 7] = [
    [
        "Grass_NewYork01",
        "TGrass_NewYork01",
        "TGrass_NewYork01_NRM",
    ],
    ["Grass_MtRush04", "Grass_MtRush04", "Grass_MtRush04_NRM"],
    [
        "Grass_CapeCod05",
        "TGrass_CapeCod05",
        "TGrass_CapeCod05_nrm",
    ],
    ["Rock_Yucatan03", "TRock_Yucatan03", "TRock_Yucatan03_nrm"],
    ["Rock_Iceland02", "TRock_Iceland02", "TRock_Iceland02_nrm"],
    ["Grass_Geneva01", "TGrass_Geneva01", "TGrass_Geneva01_nrm"],
    ["Dirt_Iceland05", "TDirt_Iceland05", "TDirt_Iceland05_NRM"],
];

pub(crate) fn levels(p: &Project) -> Vec<f64> {
    let mut out: Vec<_> = p.cfg["LAYER_M"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row[1].as_f64())
        .collect();
    if out.is_empty() {
        out = p.cfg["TERRAIN"]["levels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_f64)
            .collect();
    }
    if out.is_empty() {
        out = vec![210., 280., 350., 420.];
    }
    out.sort_by(f64::total_cmp);
    out.dedup();
    out
}

pub(crate) fn layer(height: f64, levels: &[f64], peak: f64) -> usize {
    // Older finished maps configure only playable plateaus; their actual peaks
    // are a separate fifth band. Never turn a configured high plateau into one.
    if levels.len() < PRIMARY.len() && height > peak.max(*levels.last().unwrap()) {
        return 4;
    }
    levels
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (**a - height).abs().total_cmp(&(**b - height).abs()))
        .unwrap()
        .0
        .min(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_high_plateaus_do_not_become_unconfigured_peaks() {
        assert_eq!(layer(525., &[210., 280., 350., 525.], 435.), 3);
        assert_eq!(layer(600., &[210., 280., 350., 525.], 435.), 4);
        assert_eq!(layer(420., &[210., 280., 350., 420., 700.], 435.), 3);
        assert_eq!(layer(700., &[210., 280., 350., 420., 700.], 435.), 4);
    }
}
