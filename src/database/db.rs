use std::sync::Arc;

use hash_db::{AsHashDB, HashDB, HashDBRef, Hasher, Prefix};
use keccak_hasher::KeccakHasher;
use kvdb::KeyValueDB;
use kvdb_rocksdb::{Database, DatabaseConfig};
use trie_db::DBValue;
use trie_db::{NodeCodec, TrieLayout};

use super::{error::Result, trie_layout::ExtensionLayout};

// Database column indexes.
pub const COL_EXTRA: u32 = 0;
pub const COL_ACCOUNT: u32 = 1;
pub const COL_BLOCK_HEADERS: u32 = 2;
pub const COL_BLOCK_BODIES: u32 = 3;
pub const COL_NODE_LIST: u32 = 4;
pub const COL_NODE_LIST_ACTIVATED: u32 = 5;
pub const COL_TASK_LIST: u32 = 6;
pub const COL_TASK_RESULT: u32 = 7;
pub const COL_TASK_OPERATIONS: u32 = 8;

pub const NUM_COLUMNS: u32 = 9;

#[derive(Clone)]
pub struct DB {
    pub data: Arc<dyn KeyValueDB>,
    column: u32,
    pub hashed_null_node: [u8; 32],
    null_node_data: [u8; 1],
}

impl DB {
    pub fn new(data: Arc<dyn KeyValueDB>, column: u32) -> Result<DB> {
        let db = DB {
            data,
            column,
            hashed_null_node: <ExtensionLayout as TrieLayout>::Codec::hashed_null_node(),
            null_node_data: [0u8],
        };

        Ok(db)
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(self.column, key)?)
    }

    pub fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut tx = self.data.transaction();
        tx.put(self.column, &key, &value);
        Ok(self.data.write(tx)?)
    }

    pub fn remove(&mut self, key: &[u8]) -> Result<()> {
        let mut tx = self.data.transaction();
        tx.delete(self.column, &key);
        Ok(self.data.write(tx)?)
    }
}

impl HashDB<KeccakHasher, DBValue> for DB {
    fn get(&self, key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) -> Option<DBValue> {
        if key.as_ref() == &self.hashed_null_node {
            return Some(self.null_node_data.to_vec());
        }

        let key = prefixed_key(key, prefix);
        match self.data.get(self.column, &key) {
            Ok(Some(value)) => Some(value.clone()),
            _ => None,
        }
    }

    fn contains(&self, key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) -> bool {
        if key.as_ref() == &self.hashed_null_node {
            return true;
        }

        let key = prefixed_key(key, prefix);
        match self.data.has_key(self.column, &key) {
            Ok(r) => r,
            _ => false,
        }
    }

    fn emplace(&mut self, key: <KeccakHasher as Hasher>::Out, prefix: Prefix, value: DBValue) {
        if value == self.null_node_data {
            return;
        }

        let key = prefixed_key(&key, prefix);
        let mut tx = self.data.transaction();
        tx.put(self.column, &key, &value);
        match self.data.write(tx) {
            Ok(_) => {}
            Err(e) => {}
        }
    }

    fn insert(&mut self, prefix: Prefix, value: &[u8]) -> <KeccakHasher as Hasher>::Out {
        if value == self.null_node_data {
            return KeccakHasher::hash(value);
        }
        let key = KeccakHasher::hash(value);
        HashDB::<KeccakHasher, DBValue>::emplace(self, key, prefix, value.into());
        key
    }

    fn remove(&mut self, key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) {
        if key.as_ref() == &self.hashed_null_node {
            return;
        }

        let key = prefixed_key(key, prefix);
        let mut tx = self.data.transaction();
        tx.delete(self.column, &key);
        match self.data.write(tx) {
            Ok(_) => {}
            Err(e) => {}
        }
    }
}

impl HashDBRef<KeccakHasher, DBValue> for DB {
    fn get(&self, key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) -> Option<DBValue> {
        HashDB::<KeccakHasher, DBValue>::get(self, key, prefix)
    }
    fn contains(&self, key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) -> bool {
        HashDB::<KeccakHasher, DBValue>::contains(self, key, prefix)
    }
}

impl AsHashDB<KeccakHasher, DBValue> for DB {
    fn as_hash_db(&self) -> &dyn HashDB<KeccakHasher, DBValue> {
        self
    }
    fn as_hash_db_mut(&mut self) -> &mut dyn HashDB<KeccakHasher, DBValue> {
        self
    }
}

/// Derive a database key from hash value of the node (key) and the node prefix.
pub fn prefixed_key(key: &<KeccakHasher as Hasher>::Out, prefix: Prefix) -> Vec<u8> {
    let mut prefixed_key = Vec::with_capacity(key.as_ref().len() + prefix.0.len() + 1);
    prefixed_key.extend_from_slice(prefix.0);
    if let Some(last) = prefix.1 {
        prefixed_key.push(last);
    }
    prefixed_key.extend_from_slice(key.as_ref());
    prefixed_key
}
