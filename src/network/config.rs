use serde::Deserialize;
use toml::value::*;

#[derive(Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub port: u32,
    pub known_nodes: Array,
}
