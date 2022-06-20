use std::error::Error;
use std::{fmt, result};

use serde::Serialize;

pub type Result<T> = result::Result<T, ICError>;

#[derive(Serialize, Debug)]
pub struct ICError {
    pub message: String,
}

impl fmt::Display for ICError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ICError {}

macro_rules! quick_from {
    ($t: ty) => {
        impl From<$t> for ICError {
            fn from(cause: $t) -> ICError {
                ICError {
                    message: format!("{:?}", &cause),
                }
            }
        }
    };
}

quick_from!(String);
quick_from!(ic_agent::export::PrincipalError);
quick_from!(candid::Error);
quick_from!(ic_agent::AgentError);
