use ethereum_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub struct TaskData {
    pub id: u64,
    pub hash: H256,
    pub task_type: TaskType,
    pub node_limit: u64,
    pub current_node_num: u64,
    pub status: TaskStatus,
    pub reward_weight: u64,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub enum TaskStatus {
    Enable,
    Disable,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub enum TaskType {
    LongTerm,
    Single,
}
