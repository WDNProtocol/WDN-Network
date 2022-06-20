use std::time::Duration;

use garcon::Delay;
use ic_agent::{agent::http_transport::ReqwestHttpReplicaV2Transport, Agent};

use super::{error::Result, wdn_identity::WdnIdentity};

pub mod account_capacity;
pub mod node;

pub fn waiter_with_timeout(duration: Duration) -> Delay {
    Delay::builder().timeout(duration).build()
}

pub fn expiry_duration() -> Duration {
    // 5 minutes is max ingress timeout
    Duration::from_secs(60 * 5)
}
