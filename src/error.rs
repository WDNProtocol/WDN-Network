use std::error::Error;
use std::fmt;

use serde::Serialize;

use crate::{blockchain, database, key_pair, node, task::error::TaskError};

#[derive(Serialize, Debug, Clone, Copy)]
pub enum ErrorCode {
    BindError = 10000,
    DatabaseError = 10001,
    NodeError = 10002,
    BlockChainError = 10003,
    KeyError = 10004,
    TaskError = 10005,
}

#[derive(Serialize, Debug)]
pub struct WError {
    pub code: ErrorCode,
    pub message: String,
}

impl fmt::Display for WError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for WError {}

macro_rules! quick_from {
    ($t: ty, $code: expr) => {
        impl From<$t> for WError {
            fn from(cause: $t) -> WError {
                WError {
                    code: $code,
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(database::error::DatabaseError, ErrorCode::DatabaseError);
quick_from!(
    blockchain::error::BlockchainError,
    ErrorCode::BlockChainError
);
quick_from!(key_pair::KeyPairError, ErrorCode::KeyError);
quick_from!(node::error::NodeError, ErrorCode::NodeError);
quick_from!(TaskError, ErrorCode::TaskError);
