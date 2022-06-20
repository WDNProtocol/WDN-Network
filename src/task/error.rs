use std::error::Error;
use std::io::Error as IoError;
use std::{fmt, result};

use futures::channel::mpsc::SendError;
use serde::Serialize;

use crate::blockchain::error::BlockchainError;
use crate::database;
use crate::ic::error::ICError;
use crate::message::MessageError;

pub type Result<T> = result::Result<T, TaskError>;

#[derive(Serialize, Debug)]
pub struct TaskError {
    pub message: String,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for TaskError {}

macro_rules! quick_from {
    ($t: ty) => {
        impl From<$t> for TaskError {
            fn from(cause: $t) -> TaskError {
                TaskError {
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(IoError);
quick_from!(SendError);
quick_from!(ICError);
quick_from!(String);
quick_from!(database::error::DatabaseError);
quick_from!(serde_cbor::Error);
quick_from!(BlockchainError);
quick_from!(MessageError);
