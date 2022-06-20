use std::sync::Arc;

use ethereum_types::H256;
use futures::future::ok;
use hash_db::Hasher;
use keccak_hasher::KeccakHasher;
use kvdb::KeyValueDB;
use serde::{Deserialize, Serialize};

use super::error::Result;
use crate::database::{self, data_types::TaskType, AppDB};

pub const KEY_LAST_HASH: &[u8; 11] = b"latest_hash";

#[derive(Clone)]
pub struct BlockchainDB {
    pub db: Arc<dyn KeyValueDB>,
    pub header_db: AppDB,
    pub body_db: AppDB,
}

impl BlockchainDB {
    pub fn new(db: Arc<dyn KeyValueDB>) -> Result<Self> {
        let root = database::get_root(&db, database::db::COL_BLOCK_HEADERS)?;
        let header_db = AppDB::new(
            db.clone(),
            database::db::COL_BLOCK_HEADERS,
            root.to_fixed_bytes(),
        )?;
        let root = database::get_root(&db, database::db::COL_BLOCK_BODIES)?;
        let body_db = AppDB::new(
            db.clone(),
            database::db::COL_BLOCK_BODIES,
            root.to_fixed_bytes(),
        )?;

        Ok(BlockchainDB {
            db,
            header_db,
            body_db,
        })
    }

    pub fn insert_block(&mut self, block: Block) -> Result<()> {
        let header_index_bytes = serde_cbor::to_vec(&block.header.index)?;

        let header_bytes = serde_cbor::to_vec(&block.header)?;
        let body_bytes = serde_cbor::to_vec(&block.body)?;

        let block_bytes = serde_cbor::to_vec(&block)?;

        let hash = KeccakHasher::hash(&block_bytes);

        let mut tx = self.db.transaction();
        tx.put(database::db::COL_EXTRA, &header_index_bytes, &hash);
        tx.put(database::db::COL_EXTRA, KEY_LAST_HASH, &hash);
        self.db.write(tx)?;

        self.header_db.insert(&hash, &header_bytes)?;
        self.body_db.insert(&hash, &body_bytes)?;

        Ok(())
    }

    pub fn get_block_by_index(&self, index: u64) -> Result<Block> {
        let hash = self.db.get(
            database::db::COL_EXTRA,
            serde_cbor::to_vec(&index)?.as_slice(),
        )?;
        let hash = match hash {
            Some(h) => h,
            None => return Err("block index not found".to_string().into()),
        };

        self.get_block_by_hash(H256::from_slice(hash.as_slice()))
    }

    pub fn get_block_by_hash(&self, hash: H256) -> Result<Block> {
        let header = self.header_db.get(&hash.as_bytes())?;
        let header = match header {
            Some(h) => h,
            None => return Err("block not found".to_string().into()),
        };

        let body = self.body_db.get(&hash.as_bytes())?;
        let body = match body {
            Some(h) => h,
            None => return Err("block not found".to_string().into()),
        };

        Ok(Block {
            header: serde_cbor::from_slice(header.as_slice())?,
            body: serde_cbor::from_slice(body.as_slice())?,
        })
    }
}

pub fn get_latest_hash(db: Arc<dyn KeyValueDB>) -> Result<Option<H256>> {
    match db.get(database::db::COL_EXTRA, KEY_LAST_HASH)? {
        Some(h) => Ok(Some(H256::from_slice(h.as_slice()))),
        None => Ok(None),
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize, Default)]
pub struct Block {
    pub header: Header,
    pub body: Body,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize, Default)]
pub struct Header {
    pub index: u64,
    pub previous_hash: H256,
    pub account_root: H256,
    pub reward_root: H256,
    pub task_root: H256,
    pub task_operation_root: H256,
    pub task_result_root: H256,
    pub node_root: H256,
    pub node_activation_root: H256,
    pub current_reward_root: H256,
    pub current_task_operation_root: H256,
    pub current_task_result_root: H256,
    pub current_node_activation_root: H256,
    pub timestamp: u64,
    pub version: u64,
    pub minter: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize, Default)]
pub struct Body {
    pub reward: Vec<Reward>,
    pub tasks: Vec<TaskOperation>,
    pub task_results: Vec<TaskResult>,
    pub node_activation: Vec<NeedSignData<NodeActivation>>,
}

impl Body {
    pub fn new() -> Self {
        Body {
            reward: vec![],
            tasks: vec![],
            task_results: vec![],
            node_activation: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub enum TaskOperationType {
    Add,
    Remove,
    Disable,
    Enable,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub struct TaskOperation {
    pub id: u64,
    pub operation: TaskOperationType,
    pub binary_hash: H256,
    pub task_type: TaskType,
    pub node_limit: u64,
    pub reward_weight: u64,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub struct TaskResult {
    pub id: u64,
    pub timestamp: u64,
    pub result: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub enum ActivationOperation {
    Activate,
    Deactivate,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub struct NodeActivation {
    pub operation: ActivationOperation,
    pub peer_id: String,
    pub account: Vec<u8>,
    pub pub_key: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub struct NeedSignData<T> {
    pub data: T,
    pub signature: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Eq, Deserialize, Serialize)]
pub struct Reward {
    pub account: Vec<u8>,
    pub amount: u64,
}
