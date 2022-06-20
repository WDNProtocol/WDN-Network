use std::sync::Arc;

use ethereum_types::H256;
use hash_db::Hasher;
use keccak_hasher::KeccakHasher;
use kvdb::KeyValueDB;

use crate::{
    blockchain::db::{TaskOperation, TaskResult},
    database::{self, data_types::TaskData, AppDB},
};

use super::error::Result;

#[derive(Clone)]
pub struct TaskDB {
    pub db: Arc<dyn KeyValueDB>,
    pub task_db: AppDB,
    pub task_operation_db: AppDB,
    pub task_result_db: AppDB,
    pub temp_task_operation_db: AppDB,
    pub temp_task_result_db: AppDB,
}

impl TaskDB {
    pub fn new(db: Arc<dyn KeyValueDB>) -> Result<Self> {
        let task_root = database::get_root(&db, database::db::COL_TASK_LIST)?;
        let task_db = AppDB::new(
            db.clone(),
            database::db::COL_TASK_LIST,
            task_root.to_fixed_bytes(),
        )?;

        let task_operation_root = database::get_root(&db, database::db::COL_TASK_OPERATIONS)?;
        let task_operation_db = AppDB::new(
            db.clone(),
            database::db::COL_TASK_OPERATIONS,
            task_operation_root.to_fixed_bytes(),
        )?;

        let task_result_root = database::get_root(&db, database::db::COL_TASK_RESULT)?;
        let task_result_db = AppDB::new(
            db.clone(),
            database::db::COL_TASK_RESULT,
            task_result_root.to_fixed_bytes(),
        )?;

        let temp_task_operation_db = AppDB::new(
            db.clone(),
            database::db::COL_TASK_OPERATIONS,
            H256::zero().to_fixed_bytes(),
        )?;

        let temp_task_result_db = AppDB::new(
            db.clone(),
            database::db::COL_TASK_RESULT,
            H256::zero().to_fixed_bytes(),
        )?;

        Ok(TaskDB {
            db,
            task_db,
            task_operation_db,
            task_result_db,
            temp_task_operation_db,
            temp_task_result_db,
        })
    }

    pub fn insert_task(&mut self, task: TaskData) -> Result<()> {
        let task_bytes = serde_cbor::to_vec(&task)?;
        let hash = KeccakHasher::hash(&task_bytes);
        self.task_db.insert(&hash, &task_bytes)?;
        Ok(())
    }

    pub fn insert_task_operation(&mut self, task_operation: TaskOperation) -> Result<()> {
        let data_bytes = serde_cbor::to_vec(&task_operation)?;
        let hash = KeccakHasher::hash(&data_bytes);
        self.task_operation_db.insert(&hash, &data_bytes)?;
        self.temp_task_operation_db.insert(&hash, &data_bytes)?;
        Ok(())
    }

    pub fn insert_task_result(&mut self, task_result: TaskResult) -> Result<()> {
        let data_bytes = serde_cbor::to_vec(&task_result)?;
        let hash = KeccakHasher::hash(&data_bytes);
        self.task_result_db.insert(&hash, &data_bytes)?;
        self.temp_task_result_db.insert(&hash, &data_bytes)?;
        Ok(())
    }
}
