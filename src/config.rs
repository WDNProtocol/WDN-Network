use serde::Serialize;
use serde_derive::Deserialize;

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

use crate::api::config::ApiConfig;
use crate::network::config::NetworkConfig;
use crate::node::config::NodeConfig;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub base: BaseConfig,
    pub network: NetworkConfig,
    pub node_config: NodeConfig,
    pub api_config: ApiConfig,
}

pub fn load_config(file_path: String) -> Result<Config, ConfigError> {
    let mut file = File::open(&file_path)?;
    let mut str_val = String::new();
    file.read_to_string(&mut str_val)?;
    Ok(toml::from_str(&str_val)?)
}

#[derive(Serialize, Debug)]
pub struct ConfigError {
    message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(cause: std::io::Error) -> ConfigError {
        ConfigError {
            message: format!("{:?}", &cause),
        }
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(cause: toml::de::Error) -> ConfigError {
        ConfigError {
            message: format!("{:?}", &cause),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct BaseConfig {
    pub data_path: String,
}
