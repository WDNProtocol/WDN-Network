use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub enum NodeType {
    Work,
    Verify,
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Work
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub enum NodeStatus {
    Online,
    Offline,
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Offline
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub enum NodeActiveStatus {
    Inactived,
    Actived,
}

impl Default for NodeActiveStatus {
    fn default() -> Self {
        NodeActiveStatus::Inactived
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize, Default)]
pub struct NodeData {
    pub peer_id: String,
    pub bind_address: String,
    pub status: NodeStatus,
    pub active_status: NodeActiveStatus,
    pub node_type: NodeType,
    pub stake_amount: u128,
    pub worker_stake_amount: HashMap<String, u128>,
    pub vote_amount: u128,
    pub voting_rights: u128,
    pub worker_vote_amount: HashMap<String, u128>,
    pub online_blocks: u128,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize, Default)]
pub struct TaskDistributeData {
    pub task_id: u64,
    pub peer_id: String,
}
