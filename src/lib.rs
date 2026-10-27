//! Native SAGE map codecs and reproducible RA3 map builds.
pub mod archive;
pub mod binary;
pub mod build;
pub mod coordinate_pipeline;
pub mod coordinates;
pub mod gift_clearance;
pub mod map;
pub mod periodic_attack;
pub mod production_facing;
pub mod production_spawns;
pub mod project;
pub mod ramps;
pub mod refpack;
pub mod release;
pub mod render;
pub mod resources;
pub mod script_spawns;
pub mod scripts;
pub mod spatial;
pub mod surface;
pub mod terrain;
pub mod texture_polish;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
pub fn err(message: impl Into<String>) -> Error {
    message.into().into()
}
