//! 脚本数据模型，以及原生编解码、参数和规则链接入口。
mod arguments;
mod codec;
mod linking;

pub(crate) use arguments::leaf;
pub use arguments::{ScalarArgument, real_argument, scalar_arguments, string_arguments};
pub use codec::{encode_nodes, hex, name, parse_nodes, prefix_end, script_names, unhex, validate};
pub use linking::{apply, load_rules, validate_waypoints};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub asset: String,
    pub version: u16,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,
}
#[derive(Deserialize)]
pub struct RuleChunk {
    version: u16,
    body: String,
}
#[derive(Deserialize)]
pub struct Rules {
    #[serde(rename = "SidesList")]
    sides: RuleChunk,
    #[serde(rename = "Teams")]
    teams: RuleChunk,
    pub names: Vec<(String, u32)>,
    pub scripts: Vec<Node>,
    #[serde(default)]
    pub spawn_waypoints: Vec<crate::gameplay::script_spawns::SpawnWaypoint>,
    #[serde(default)]
    pub periodic_attack: Option<crate::gameplay::periodic_attack::Config>,
}
