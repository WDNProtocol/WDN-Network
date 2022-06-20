use candid::{CandidType, Decode, Deserialize, Encode, Nat};
use ic_agent::{ic_types::Principal, Agent};

use super::super::error::Result;

#[derive(Clone, Debug, Deserialize, CandidType)]
pub struct CapacityInfo {
    pub inviter: Principal,

    // long term capacity
    pub long_term_capacity: u64,
    // reward capacity
    pub reward_capacity: u64,

    pub invitee_count: u64,
    pub invitation_expire: u64,

    pub charged: Nat,
    pub rest_charged: Nat,
}

pub struct AccountCapacity<'agent> {
    agent: &'agent Agent,
    canister_id: Principal,
}

impl<'agent> AccountCapacity<'agent> {
    pub fn create(agent: &Agent, canister_id: String) -> Result<AccountCapacity> {
        Ok(AccountCapacity {
            agent,
            canister_id: Principal::from_text(canister_id)?,
        })
    }

    pub async fn get_capacity_info(&self, account: Principal) -> Result<CapacityInfo> {
        let res = self
            .agent
            .query(&self.canister_id, "getCapacityInfo")
            .with_arg(Encode!(&account)?)
            .call()
            .await?;

        let res = Decode!(&res, core::result::Result<CapacityInfo, String>)??;

        Ok(res)
    }

    pub async fn get_all_capacity_info(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<Vec<CapacityInfo>> {
        let res = self
            .agent
            .query(&self.canister_id, "getAllCapacityInfo")
            .with_arg(Encode!(&start, &limit)?)
            .call()
            .await?;

        Ok(Decode!(&res, Vec<CapacityInfo>)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::ic::{create_agent_with_identity, wdn_identity::WdnIdentity};

    use super::*;

    use actix_web::rt::Runtime;
    use libp2p::identity::ed25519::Keypair;

    #[test]
    fn test_get_capacity_info() {
        let runtime = Runtime::new().expect("Unable to create a runtime");
        let res = runtime.block_on(async {
            let temp_local_key = Keypair::generate();
            let identity = WdnIdentity::from_key_pair(temp_local_key);
            let agent = create_agent_with_identity(identity, "http://127.0.0.1:8000").unwrap();
            agent.fetch_root_key().await.unwrap();

            let account =
                AccountCapacity::create(&agent, "rkp4c-7iaaa-aaaaa-aaaca-cai".to_string()).unwrap();
            account.get_capacity_info(Principal::anonymous()).await
        });
        println!("res {:?}", res);
    }

    #[test]
    fn test_get_all_capacity_info() {
        let runtime = Runtime::new().expect("Unable to create a runtime");
        let res = runtime.block_on(async {
            let temp_local_key = Keypair::generate();
            let identity = WdnIdentity::from_key_pair(temp_local_key);
            let agent = create_agent_with_identity(identity, "http://127.0.0.1:8000").unwrap();
            agent.fetch_root_key().await.unwrap();

            let account =
                AccountCapacity::create(&agent, "rkp4c-7iaaa-aaaaa-aaaca-cai".to_string()).unwrap();
            account.get_all_capacity_info(0, 10).await
        });
        println!("res {:?}", res);
    }
}
