use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NodeConfig{
    pub principal_id: String
}