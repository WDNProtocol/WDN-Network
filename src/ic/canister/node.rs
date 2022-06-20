use std::collections::HashSet;

use candid::{Decode, Encode, Nat};
use ic_agent::{ic_types::Principal, Agent};

use super::{super::error::Result, expiry_duration, waiter_with_timeout};

pub type Keeper = HashSet<Principal>;
pub type Backer = HashSet<Principal>;

pub struct Node<'agent> {
    agent: &'agent Agent,
    canister_id: Principal,
}

impl<'agent> Node<'agent> {
    pub fn create(agent: &Agent, canister_id: String) -> Result<Node> {
        Ok(Node {
            agent,
            canister_id: Principal::from_text(canister_id)?,
        })
    }

    pub async fn get_keepers(&self) -> Result<Keeper> {
        let res = self
            .agent
            .query(&self.canister_id, "getKeepers")
            .with_arg(Encode!(&())?)
            .call()
            .await?;

        Ok(Decode!(&res, Keeper)?)
    }

    pub async fn get_backers(&self) -> Result<Backer> {
        let res = self
            .agent
            .query(&self.canister_id, "getBackers")
            .with_arg(Encode!(&())?)
            .call()
            .await?;

        Ok(Decode!(&res, Backer)?)
    }

    pub async fn withdraw(&self, worker: Principal, amount: Nat) -> Result<Nat> {
        let res = self
            .agent
            .update(&self.canister_id, "withdraw")
            .with_arg(Encode!(&worker, &amount)?)
            .call_and_wait(waiter_with_timeout(expiry_duration()))
            .await?;

        Ok(Decode!(&res, Nat)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::ic::{create_agent_with_identity, wdn_identity::WdnIdentity};

    use super::*;

    use actix_web::rt::Runtime;
    use libp2p::identity::ed25519::Keypair;

    #[test]
    fn test_keepers() {
        let runtime = Runtime::new().expect("Unable to create a runtime");
        let res = runtime.block_on(async {
            let temp_local_key = Keypair::generate();
            let identity = WdnIdentity::from_key_pair(temp_local_key);
            let agent = create_agent_with_identity(identity, "http://127.0.0.1:8000").unwrap();
            agent.fetch_root_key().await.unwrap();

            let node = Node::create(&agent, "rno2w-sqaaa-aaaaa-aaacq-cai".to_string()).unwrap();
            node.get_keepers().await
        });
        println!("res {:?}", res);
        // assert!(!hash.is_empty(), "wrong hash");
    }
}
