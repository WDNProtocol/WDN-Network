use std::error::Error;
use std::io::Error as IoError;
use std::{fmt, result};

use serde::Serialize;

use crate::database;

pub type Result<T> = result::Result<T, BlockchainError>;

#[derive(Serialize, Debug)]
pub struct BlockchainError {
    pub message: String,
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for BlockchainError {}

macro_rules! quick_from {
    ($t: ty) => {
        impl From<$t> for BlockchainError {
            fn from(cause: $t) -> BlockchainError {
                BlockchainError {
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(String);
quick_from!(IoError);
quick_from!(database::error::DatabaseError);
quick_from!(serde_cbor::Error);
