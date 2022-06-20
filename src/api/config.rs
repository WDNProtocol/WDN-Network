use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ApiConfig{
    pub host: String,
    pub port: u16
}