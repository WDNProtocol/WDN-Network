use std::error::Error;
use std::io::Error as IoError;
use std::{fmt, result};

use futures::channel::mpsc::SendError;
use serde::Serialize;

use crate::database;
use crate::ic::error::ICError;
use crate::message::MessageError;

pub type Result<T> = result::Result<T, NodeError>;

#[derive(Serialize, Debug)]
pub struct NodeError {
    pub message: String,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for NodeError {}

macro_rules! quick_from {
    ($t: ty) => {
        impl From<$t> for NodeError {
            fn from(cause: $t) -> NodeError {
                NodeError {
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(IoError);
quick_from!(SendError);
quick_from!(MessageError);
quick_from!(ICError);
quick_from!(String);
quick_from!(database::error::DatabaseError);
quick_from!(serde_cbor::Error);
