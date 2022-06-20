use actix_web::rt::Runtime;
use ic_agent::{agent::http_transport::ReqwestHttpReplicaV2Transport, Agent};
use libp2p::identity::ed25519::Keypair;

use self::canister::node::{Keeper, Node};
use self::error::Result;
use self::wdn_identity::WdnIdentity;

pub mod canister;
pub mod error;
pub mod wdn_identity;

pub const IC_URL: &str = "https://ic0.app";

pub fn create_agent_with_identity(identity: WdnIdentity, url: &str) -> Result<Agent> {
    let agent = Agent::builder()
        .with_transport(ReqwestHttpReplicaV2Transport::create(url)?)
        .with_identity(identity)
        .build()?;

    Ok(agent)
}

/// `get_keepers` get keeper from node
pub fn get_keepers(agent: Agent, local_key: Keypair) -> Result<Keeper> {
    let runtime = Runtime::new().expect("Unable to create a runtime");
    let res = runtime.block_on(async {
        let identity = WdnIdentity::from_key_pair(local_key);
        let agent = create_agent_with_identity(identity, "http://127.0.0.1:8000").unwrap();
        agent.fetch_root_key().await.unwrap();
        let node = Node::create(&agent, "rno2w-sqaaa-aaaaa-aaacq-cai".to_string()).unwrap();
        node.get_keepers().await
    })?;
    Ok(res)
}
