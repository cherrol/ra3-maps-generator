//! Native SAGE map codecs and reproducible RA3 map builds.
pub mod coordinates;
pub mod formats;
pub mod gameplay;
pub mod pipeline;
pub mod render;
pub mod resources;
mod rules;
pub mod terrain;
pub mod terrain_preview;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
pub fn err(message: impl Into<String>) -> Error {
    message.into().into()
}
