use super::error::Result;
use ethereum_types::H256;
use hash_db::Hasher;
use keccak_hasher::KeccakHasher;
use kvdb::KeyValueDB;
use std::sync::Arc;

use crate::{
    blockchain::db::{NeedSignData, NodeActivation},
    database::{self, data_types::NodeData, AppDB},
};

#[derive(Clone)]
pub struct NodeDB {
    pub db: Arc<dyn KeyValueDB>,
    pub node_db: AppDB,
    pub node_active_db: AppDB,
    pub temp_node_db: AppDB,
    pub temp_node_active_db: AppDB,
}

impl NodeDB {
    pub fn new(db: Arc<dyn KeyValueDB>) -> Result<Self> {
        let node_root = database::get_root(&db, database::db::COL_NODE_LIST)?;
        let node_db = AppDB::new(
            db.clone(),
            database::db::COL_NODE_LIST,
            node_root.to_fixed_bytes(),
        )?;

        let node_active_root = database::get_root(&db, database::db::COL_NODE_LIST_ACTIVATED)?;
        let node_active_db = AppDB::new(
            db.clone(),
            database::db::COL_NODE_LIST_ACTIVATED,
            node_active_root.to_fixed_bytes(),
        )?;

        let temp_node_db = AppDB::new(
            db.clone(),
            database::db::COL_NODE_LIST,
            H256::zero().to_fixed_bytes(),
        )?;

        let temp_node_active_db = AppDB::new(
            db.clone(),
            database::db::COL_NODE_LIST_ACTIVATED,
            H256::zero().to_fixed_bytes(),
        )?;

        Ok(NodeDB {
            db,
            node_db,
            node_active_db,
            temp_node_db,
            temp_node_active_db,
        })
    }

    pub fn insert_node(&mut self, node: NodeData) -> Result<()> {
        let node_bytes = serde_cbor::to_vec(&node)?;
        let node_bytes_hash = KeccakHasher::hash(&node_bytes);
        self.node_db.insert(&node_bytes_hash, &node_bytes)?;
        self.temp_node_db.insert(&node_bytes_hash, &node_bytes)?;
        Ok(())
    }

    pub fn insert_node_activation(
        &mut self,
        node_activation: NeedSignData<NodeActivation>,
    ) -> Result<()> {
        let node_activation_bytes = serde_cbor::to_vec(&node_activation)?;
        let node_activation_hash = KeccakHasher::hash(&node_activation_bytes);
        self.node_active_db
            .insert(&node_activation_hash, &node_activation_bytes)?;
        self.temp_node_active_db
            .insert(&node_activation_hash, &node_activation_bytes)?;
        Ok(())
    }

    /// `reset_temp_db` should be called after block was packed, before a new block id coming.
    /// it's used for calculate current data hash.
    pub fn reset_temp_db(&mut self) -> Result<()> {
        self.temp_node_db = AppDB::new(
            self.db.clone(),
            database::db::COL_NODE_LIST,
            H256::zero().to_fixed_bytes(),
        )?;
        self.temp_node_active_db = AppDB::new(
            self.db.clone(),
            database::db::COL_NODE_LIST_ACTIVATED,
            H256::zero().to_fixed_bytes(),
        )?;
        Ok(())
    }
}
