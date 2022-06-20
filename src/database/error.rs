use std::error::Error;
use std::io::Error as IoError;
use std::{fmt, result};

use serde::Serialize;
use trie_db::TrieError;

pub type Result<T> = result::Result<T, DatabaseError>;

#[derive(Serialize, Debug)]
pub struct DatabaseError {
    pub message: String,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DatabaseError {}

macro_rules! quick_from {
    ($t: ty) => {
        impl From<$t> for DatabaseError {
            fn from(cause: $t) -> DatabaseError {
                DatabaseError {
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(IoError);
quick_from!(Box<TrieError<[u8; 32], parity_scale_codec::Error>>);
quick_from!(Vec<u8>);
