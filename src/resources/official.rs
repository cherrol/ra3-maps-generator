//! Versioned offline EA data. The production pipeline reads only the embedded index.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogBinding {
    pub schema_version: u32,
    pub extractor_version: String,
    pub revision: String,
    pub fingerprint: String,
    pub inputs_sha256: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TerrainDefinition {
    pub id: String,
    pub texture: String,
    pub bump_texture: Option<String>,
    pub class: String,
    pub wb_folder: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Shape {
    pub kind: String,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub height: f64,
    pub offset: [f64; 3],
    pub active: bool,
    pub name: Option<String>,
    pub front_angle_radians: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductionExit {
    pub create: [f64; 3],
    pub rally: [f64; 3],
    pub no_exit_path: bool,
    pub placement_view_angle_radians: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocomotorBinding {
    pub condition: String,
    pub locomotor: String,
    pub speed: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssistedPlacement {
    pub suggested_offset: [f64; 3],
    pub filter_radius: f64,
    pub filter_rule: String,
    pub relationships: Vec<String>,
    pub restricted_kind_of: Vec<String>,
    pub excluded_kind_of: Vec<String>,
    pub restricted_objects: Vec<String>,
    pub excluded_objects: Vec<String>,
    pub hint_objects: Vec<String>,
    pub pending: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectDefinition {
    pub id: String,
    pub source: String,
    pub lineage: Vec<String>,
    pub kind_of: Vec<String>,
    pub shapes: Vec<Shape>,
    pub placement_shape: Option<Shape>,
    pub rotation_anchor: [f64; 2],
    pub placement_view_angle_radians: f64,
    pub production_exit: Option<ProductionExit>,
    pub restricted_area: Option<Shape>,
    pub locomotors: Vec<LocomotorBinding>,
    pub assisted_placement: Vec<AssistedPlacement>,
    /// Membership from WBStatic_12.big, independently of SDK directory presence.
    pub preloaded: bool,
    pub pending: Vec<String>,
}

/// Only facts shared by every reachable candidate; no unique geometry/source.
#[derive(Debug, Deserialize, Serialize)]
pub struct ClassificationFact {
    pub id: String,
    pub kind_of: Vec<String>,
    pub equivalent_sources: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocomotorDefinition {
    pub id: String,
    pub source: String,
    pub surfaces: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub pending: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WaterDepths {
    pub wade: f64,
    pub deep: f64,
    pub source: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dependency {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub resolved: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Catalog {
    pub binding: CatalogBinding,
    pub terrain: BTreeMap<String, TerrainDefinition>,
    pub objects: BTreeMap<String, ObjectDefinition>,
    #[serde(default)]
    pub classifications: BTreeMap<String, ClassificationFact>,
    pub locomotors: BTreeMap<String, LocomotorDefinition>,
    pub water_depths: WaterDepths,
    pub dependencies: Vec<Dependency>,
    pub global_membership: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub fn catalog() -> &'static Catalog {
    static INDEX: OnceLock<Catalog> = OnceLock::new();
    INDEX.get_or_init(|| {
        serde_json::from_str(include_str!("../../templates/resources/ea-catalog.json"))
            .expect("the versioned EA resource index must be valid")
    })
}

pub fn terrain(name: &str) -> Option<&'static TerrainDefinition> {
    catalog().terrain.get(&name.to_ascii_lowercase())
}

/// Recipe-compatible logical name/diffuse/normal row; paths stay authoritative.
pub(crate) fn terrain_row(name: &str) -> Option<[String; 3]> {
    let definition = terrain(name)?;
    Some([
        definition.id.clone(),
        definition.texture.strip_suffix(".tga")?.to_owned(),
        definition
            .bump_texture
            .as_deref()?
            .strip_suffix(".tga")?
            .to_owned(),
    ])
}

pub fn object(name: &str) -> Option<&'static ObjectDefinition> {
    catalog().objects.get(&name.to_ascii_lowercase())
}

pub fn classification(name: &str) -> Option<&'static ClassificationFact> {
    catalog().classifications.get(&name.to_ascii_lowercase())
}

pub fn certainly_noncolliding_marker(name: &str) -> bool {
    let bits = match object(name) {
        Some(object) if object.pending.is_empty() => Some(&object.kind_of),
        Some(_) => None,
        None => classification(name).map(|fact| &fact.kind_of),
    };
    bits.is_some_and(|bits| {
        let has = |flag: &str| bits.iter().any(|bit| bit == flag);
        has("BEACON") && has("NO_COLLIDE") && !has("STRUCTURE")
    })
}

pub fn locomotor(name: &str) -> Option<&'static LocomotorDefinition> {
    catalog().locomotors.get(&name.to_ascii_lowercase())
}

pub fn objects() -> impl Iterator<Item = &'static ObjectDefinition> {
    catalog().objects.values()
}

pub fn preloaded(name: &str) -> bool {
    catalog()
        .global_membership
        .iter()
        .any(|id| id.eq_ignore_ascii_case(name))
}

pub fn binding() -> &'static CatalogBinding {
    &catalog().binding
}

pub fn water_depths() -> &'static WaterDepths {
    &catalog().water_depths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_has_the_fixed_origin_and_complete_terrain_names() {
        assert_eq!(
            binding().revision,
            "3aad8e412961ab543192ae70f8503dc8e178e095"
        );
        assert_eq!(catalog().terrain.len(), 406);
        assert!(!binding().inputs_sha256.is_empty());
        assert_eq!(binding().fingerprint.len(), 64);
        let rock = terrain("ROCK_YUCATAN03").unwrap();
        assert_eq!(rock.texture, "TRock_Yucatan03.tga");
        assert_eq!(
            rock.bump_texture.as_deref(),
            Some("TRock_Yucatan03_nrm.tga")
        );
        assert!(std::ptr::eq(catalog(), catalog()));
        assert!(objects().all(|object| {
            object
                .kind_of
                .iter()
                .all(|flag| !flag.contains(['$', '=', '\'', '"']))
        }));
        assert!(
            object("CC_Tree01")
                .unwrap()
                .kind_of
                .iter()
                .any(|flag| flag == "IMMOBILE")
        );
        assert!(
            object("HA_GardenRock01")
                .unwrap()
                .kind_of
                .iter()
                .any(|flag| flag == "STRUCTURE")
        );
    }

    #[test]
    fn placement_shapes_anchors_and_factory_exits_remain_distinct() {
        let factory = object("AlliedWarFactory").unwrap();
        assert!(factory.pending.is_empty(), "{:?}", factory.pending);
        assert_eq!(
            factory.production_exit.as_ref().unwrap().create,
            [-20., 20., 0.]
        );
        assert_eq!(
            factory.production_exit.as_ref().unwrap().rally,
            [80., 20., 0.]
        );
        assert_eq!(factory.shapes[0].major_radius, 60.);
        let refinery = object("AlliedRefinery").unwrap();
        let restricted = refinery.restricted_area.as_ref().unwrap();
        assert_eq!(
            (restricted.major_radius, restricted.minor_radius),
            (30., 45.)
        );
        assert_eq!(restricted.offset, [90., 0., 0.]);
        assert_eq!(
            object("OreNode").unwrap().assisted_placement[0].suggested_offset,
            [180., 0., 0.]
        );
        let yard = object("SovietNavalYard").unwrap();
        assert_eq!(yard.placement_shape.as_ref().unwrap().major_radius, 90.);
        assert!(yard.shapes.iter().any(|s| s.major_radius != 90.));
    }

    #[test]
    fn mobility_uses_effective_sdk_water_values_and_keeps_states_separate() {
        assert_eq!((water_depths().wade, water_depths().deep), (0., 35.));
        let unit = object("AlliedMCV").unwrap();
        assert_eq!(unit.locomotors.len(), 2);
        assert!(
            unit.locomotors
                .iter()
                .all(|state| state.condition == "NORMAL")
        );
        assert_ne!(unit.locomotors[0].locomotor, unit.locomotors[1].locomotor);
        for state in &unit.locomotors {
            let movement = locomotor(&state.locomotor).unwrap();
            assert!(!movement.surfaces.is_empty());
        }
        assert_eq!(catalog().global_membership.len(), 507);
        assert!(object("OreNode").unwrap().preloaded);
        assert!(!object("HospitalTechStructure").unwrap().preloaded);
        assert!(preloaded("MultiplayerBeacon"));
        assert!(object("MultiplayerBeacon").is_none());
    }

    #[test]
    fn ambiguous_beacons_expose_consensus_without_selecting_geometry() {
        for name in ["MultiplayerBeacon", "MultiplayerBeaconAttached"] {
            assert!(object(name).is_none());
            let fact = classification(name).unwrap();
            assert_eq!(fact.equivalent_sources.len(), 2);
            assert!(fact.scope.contains("classification-only"));
            assert!(certainly_noncolliding_marker(name));
            let json = serde_json::to_value(fact).unwrap();
            assert!(json.get("source").is_none());
            assert!(json.get("shapes").is_none());
        }
        assert!(!certainly_noncolliding_marker("UnknownBuilding"));
        assert!(!certainly_noncolliding_marker("AlliedWarFactory"));
        assert!(
            catalog()
                .diagnostics
                .iter()
                .any(|issue| issue.contains("multiplayerbeacon"))
        );
    }

    #[test]
    fn complete_material_catalog_retains_explicit_recipe_overrides() {
        let mut project = crate::pipeline::project::Project::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            "河南",
        )
        .unwrap();
        project.cfg["TEX_SET"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!([
                "Rock_Yucatan03",
                "Custom_Diffuse",
                "Custom_Normal"
            ]));
        let materials = super::super::atlas::terrain_catalog(&project).unwrap();
        assert_eq!(materials["Rock_Yucatan03"][1], "Custom_Diffuse");
        assert_eq!(materials["Pavement_Romania01"][1], "Pavement_Romania01");
        project.cfg["TEX_SET"] = serde_json::json!([
            "Rock_Yucatan03",
            ["Custom", "Custom_Diffuse", "Custom_Normal"]
        ]);
        let rows = crate::terrain::surface::textures(&project).unwrap();
        assert_eq!(
            rows[0],
            ["Rock_Yucatan03", "TRock_Yucatan03", "TRock_Yucatan03_nrm"]
        );
        assert_eq!(rows[1], ["Custom", "Custom_Diffuse", "Custom_Normal"]);
        project.cfg["TEX_SET"] = serde_json::json!(["UnknownUnofficialTerrain"]);
        assert!(crate::terrain::surface::textures(&project).is_err());
    }
}
