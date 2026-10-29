//! Compiled fallback values come from the same public JSON used by projects.
use serde_json::Value;
use std::sync::OnceLock;

fn defaults() -> &'static Value {
    static VALUE: OnceLock<Value> = OnceLock::new();
    VALUE.get_or_init(|| {
        serde_json::from_str(include_str!("../templates/rules/coordinate-pipeline.json"))
            .expect("valid bundled coordinate-pipeline rules")
    })
}

pub(crate) fn number(section: &str, key: &str) -> f64 {
    defaults()[section][key]
        .as_f64()
        .expect("numeric bundled rule")
}

pub(crate) fn count(section: &str, key: &str) -> usize {
    defaults()[section][key]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .expect("integer bundled rule")
}

pub(crate) fn range(section: &str, key: &str) -> [f64; 2] {
    serde_json::from_value(defaults()[section][key].clone()).expect("two-number bundled rule")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_project_fallbacks_match_the_public_parameter_file() {
        for (section, actual) in [
            (
                "spawn_placement",
                serde_json::to_value(crate::terrain::spawn_placement::Policy::default()).unwrap(),
            ),
            (
                "ramp_clearance",
                serde_json::to_value(crate::terrain::ramp_clearance::Policy::default()).unwrap(),
            ),
            (
                "resource_distribution",
                serde_json::to_value(crate::terrain::resource_routes::Policy::default()).unwrap(),
            ),
        ] {
            assert_eq!(actual, defaults()[section], "{section}");
        }
    }
}
