use ethereum_types::H256;
use kvdb::KeyValueDB;
use kvdb_rocksdb::{Database, DatabaseConfig};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, default, sync::Arc};
use trie_db::{Trie, TrieDB, TrieDBMut, TrieMut};

use self::{db::DB, trie_layout::ExtensionLayout};
use error::Result;

mod trie_layout;

pub mod data_types;
pub mod db;
pub mod error;

pub const KEY_ROOT: &[u8; 4] = b"root";

pub fn open_database(client_path: &str) -> Result<Arc<dyn KeyValueDB>> {
    let db_config = DatabaseConfig::with_columns(db::NUM_COLUMNS);

    Ok(Arc::new(Database::open(&db_config, client_path)?))
}

#[derive(Clone)]
pub struct AppDB {
    db: DB,
    root: [u8; 32],
}

impl AppDB {
    pub fn new(db_backend: Arc<dyn KeyValueDB>, column: u32, root: [u8; 32]) -> Result<AppDB> {
        let db = DB::new(db_backend, column)?;

        let db = AppDB { root, db };
        Ok(db)
    }

    // get data from block
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let db = TrieDB::<ExtensionLayout>::new(&self.db, &self.root)?;
        let value = db.get(key)?;
        Ok(value)
    }

    // insert data to block
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut db = TrieDBMut::<ExtensionLayout>::new(&mut self.db, &mut self.root);
        db.insert(key, value)?;
        db.commit();
        Ok(())
    }

    // multi insert data to block
    pub fn multi_insert(&mut self, data: Vec<(&[u8], &[u8])>) -> Result<()> {
        let mut db = TrieDBMut::<ExtensionLayout>::new(&mut self.db, &mut self.root);
        for (key, value) in data {
            db.insert(key, value)?;
        }

        db.commit();
        Ok(())
    }

    // remove data from block
    pub fn remove(&mut self, key: &[u8]) -> Result<()> {
        let mut db = TrieDBMut::<ExtensionLayout>::new(&mut self.db, &mut self.root);
        db.remove(key)?;
        db.commit();
        Ok(())
    }

    // multi remove data from block
    pub fn multi_remove(&mut self, keys: Vec<&[u8]>) -> Result<()> {
        let mut db = TrieDBMut::<ExtensionLayout>::new(&mut self.db, &mut self.root);
        for key in keys {
            db.remove(key)?;
        }

        db.commit();
        Ok(())
    }

    pub fn get_root(&self) -> [u8; 32] {
        self.root
    }
}

pub fn get_root(db: &Arc<dyn KeyValueDB>, column: u32) -> Result<H256> {
    match db.get(column, KEY_ROOT)? {
        Some(h) => Ok(H256::from_slice(h.as_slice())),
        None => Ok(H256::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database() {
        let dir = tempfile::Builder::new()
            .prefix("worker_test")
            .tempdir()
            .unwrap();
        let path = dir.path().join("db");
        let db_backend = open_database(path.to_str().unwrap()).expect("open database failed");
        let mut db = DB::new(db_backend.clone(), 0).unwrap();
        let mut root = [0u8; 32];

        let mut db = TrieDBMut::<ExtensionLayout>::new(&mut db, &mut root);

        db.insert(b"hello", b"world").unwrap();
        println!("root {:?}", db.root());
        db.insert(b"fuck", b"you").unwrap();
        println!("root {:?}", db.root());
        let value = db.get(b"hello").unwrap();
        println!("value {:?}", value);
        let value = db.get(b"fuck").unwrap();
        println!("value2 {:?}", value);

        let back_value = db_backend.iter(0);
        for value in back_value {
            println!("back_value {:?}", value);
        }
    }
}
