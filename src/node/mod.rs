use std::{
    collections::{hash_map, HashMap},
    str::FromStr,
    sync::Arc,
    thread,
    time::Duration,
    vec,
};

use self::{config::NodeConfig, db::NodeDB, error::NodeError};
use crate::{
    blockchain::db::{
        Block, BlockchainDB, Body, Header, NeedSignData, NodeActivation, TaskOperation,
    },
    database::{
        self,
        data_types::{NodeActiveStatus, NodeData, NodeStatus, NodeType, TaskDistributeData},
        AppDB,
    },
    ic::{self, wdn_identity::WdnIdentity},
    message::{Caller, LocalMessage, LocalMessageModule, Message, Waiter},
    network::{
        topics::{self, PingMessage, SubTopics, TopicMessage},
        NetworkMessage, NetworkModule,
    },
};
use async_std::task;
use chrono::Local;
use error::Result;
use ethereum_types::H256;
use futures::{
    channel::mpsc::{channel, Receiver, SendError, Sender},
    prelude::*,
    select,
};
use hash_db::Hasher;
use ic_agent::{self, identity};
use ic_agent::{ic_types::Principal, Identity, Signature};
use keccak_hasher::KeccakHasher;
use kvdb::KeyValueDB;
use libp2p::{gossipsub::GossipsubMessage, identity::Keypair, PeerId};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use topics::Topics;

pub mod config;
pub mod db;
pub mod error;

pub struct NodeModule {
    config: NodeConfig,
    local_key: Keypair,
    peer_id: PeerId,
    pub wdn_indentity: WdnIdentity,
    network_caller: Caller,
    message_waiter: Option<Waiter>,
    message_subscribe: Vec<(Topics, Caller)>,
    pub task_caller: Option<Caller>,
    pub blockchain_caller: Option<Caller>,
    node_db: NodeDB,
    status: NodeStatus,
    active_status: NodeActiveStatus,
    node_type: NodeType,
    node_list: Vec<NodeData>,
    last_distribute_block: u64,
    task_distribute_list: Vec<TaskDistributeData>,
    agent: ic_agent::Agent,
}

impl NodeModule {
    pub fn new(
        config: NodeConfig,
        local_key: Keypair,
        db_backend: Arc<dyn KeyValueDB>,
    ) -> Result<NodeModule> {
        let message_waiter = Waiter::new();
        let message_subscribe = vec![
            (Topics::NodeList, message_waiter.get_caller()),
            (Topics::NewBlock, message_waiter.get_caller()),
            (Topics::KeepAlive, message_waiter.get_caller()),
        ];

        let node_db = NodeDB::new(db_backend.clone())?;
        let peer_id = PeerId::from_public_key(&local_key.public());

        // agent
        let mut identity: Option<ic::wdn_identity::WdnIdentity> = None;
        if let Keypair::Ed25519(temp_key) = local_key.clone() {
            identity = Some(ic::wdn_identity::WdnIdentity::from_key_pair(temp_key));
        }
        let agent = ic::create_agent_with_identity(
            identity.clone().expect("can't get indentity!"),
            ic::IC_URL,
        )
        .expect("create agent fail!");

        Ok(NodeModule {
            config,
            local_key: local_key,
            peer_id: peer_id,
            wdn_indentity: identity.unwrap(),
            message_subscribe,
            network_caller: message_waiter.get_caller(),
            message_waiter: Some(message_waiter),
            task_caller: None,
            blockchain_caller: None,
            node_db: node_db,
            status: NodeStatus::Online,
            active_status: NodeActiveStatus::Inactived,
            node_type: NodeType::Work,
            node_list: vec![],
            last_distribute_block: 0,
            task_distribute_list: vec![],
            agent: agent,
        })
    }

    pub async fn require_active_status_from_verify_node(
        &mut self,
        timestamp: String,
    ) -> Result<()> {
        log::info!("require_active_status_from_verify_node");
        let topic_message = TopicMessage {
            sub_topic: SubTopics::ReqNodeActiveStatus(self.config.principal_id.clone(), timestamp),
            data: vec![],
        };
        let peer_message = Message::NetworkMessage(NetworkMessage {
            peer_id: None,
            topic: Topics::NodeStatus,
            message: serde_cbor::to_vec(&topic_message).unwrap(),
        });
        self.network_caller.notify(peer_message).await?;
        Ok(())
    }

    async fn verify_node_init(&mut self) -> Result<()> {
        // Check current block is genesis block.
        let current_block_res = self
            .blockchain_caller
            .clone()
            .unwrap()
            .call(Message::LocalMessage(LocalMessage::ReqBlockCurrent()))
            .await?;
        let current_block =
            if let Some(Message::LocalMessage(LocalMessage::AckBlockCurrent(current_block))) =
                current_block_res
            {
                current_block
            } else {
                return Err(NodeError {
                    message: "Can't get block info now!".to_owned(),
                });
            };
        if current_block.header.index != 0 {
            return Err(NodeError {
                message: "node had inited".to_owned(),
            });
        }

        // check is keeper
        let local_key = match self.local_key.clone() {
            Keypair::Ed25519(ed25519key) => ed25519key,
            _ => {
                return Err(NodeError {
                    message: "verify node init can not get local key".to_owned(),
                });
            }
        };
        let keeper = ic::get_keepers(self.agent.clone(), local_key.clone())?;
        let principal = self.wdn_indentity.sender()?;
        if !keeper.contains(&principal) {
            return Err(NodeError {
                message: "You are not verify node!".to_owned(),
            });
        }

        // Add current node active operation into genesis block
        let node_active_operation_data = NodeActivation {
            operation: crate::blockchain::db::ActivationOperation::Activate,
            peer_id: self.peer_id.to_base58(),
            account: serde_cbor::to_vec(&self.config.principal_id)?,
            pub_key: self.local_key.public().to_protobuf_encoding(),
        };
        let node_active_operation_data_bytes = serde_cbor::to_vec(&node_active_operation_data)?;
        let node_active_operation_signature = self
            .wdn_indentity
            .sign(&node_active_operation_data_bytes)?
            .signature;
        if node_active_operation_signature.is_none() {
            return Err(NodeError {
                message: "Verify node init sign node active message fail!".to_owned(),
            });
        }
        let node_active_operation = NeedSignData {
            data: node_active_operation_data.clone(),
            signature: node_active_operation_signature.unwrap(),
        };
        let mut node = NodeData::default();
        node.peer_id = node_active_operation_data.clone().peer_id;
        node.bind_address = serde_cbor::from_slice(&node_active_operation_data.account)?;
        node.status = NodeStatus::Online;
        node.node_type = NodeType::Verify;
        self.node_db.insert_node(node.clone())?;
        self.node_list.push(node.clone());
        self.node_db
            .insert_node_activation(node_active_operation.clone())?;

        // save node active operation.
        let node_root = self.node_db.node_db.get_root();
        let node_activation_root = self.node_db.node_active_db.get_root();
        let temp_node_activation_root = self.node_db.temp_node_active_db.get_root();
        let node_active_res = self
            .network_caller
            .clone()
            .call(Message::LocalMessage(
                LocalMessage::ReqBlockSaveNodeActivation(
                    node_active_operation,
                    H256(node_root),
                    H256(node_activation_root),
                    H256(temp_node_activation_root),
                ),
            ))
            .await?;
        let res =
            if let Some(Message::LocalMessage(LocalMessage::AckBlockSaveNodeActivation(res))) =
                node_active_res
            {
                res
            } else {
                false
            };
        if !res {
            return Err(NodeError {
                message: "Init genesis block fail!".to_owned(),
            });
        }

        let req_task_init_genesis_res = self
            .task_caller
            .clone()
            .expect("can't get task caller")
            .call(Message::LocalMessage(LocalMessage::ReqTaskInitGenesis()))
            .await?;
        let res = if let Some(Message::LocalMessage(LocalMessage::AckTaskInitGenesis(res))) =
            req_task_init_genesis_res
        {
            res
        } else {
            false
        };
        if !res {
            return Err(NodeError {
                message: "init genesis block fail!".to_owned(),
            });
        }

        // Start block tick thread
        let req_block_start_tick_res = self
            .blockchain_caller
            .clone()
            .expect("can't get blockchain caller")
            .call(Message::LocalMessage(LocalMessage::ReqBlockStartTick()))
            .await?;
        let res = if let Some(Message::LocalMessage(LocalMessage::AckBlockStartTick(res))) =
            req_block_start_tick_res
        {
            res
        } else {
            false
        };
        if !res {
            return Err(NodeError {
                message: "start block tick fail!".to_owned(),
            });
        }
        Ok(())
    }

    async fn verify_node_ack_node_active_status(
        &mut self,
        pricipal_id: &String,
        timestamp: &String,
    ) -> Result<()> {
        // TODO: Get node active status from contract and db, then back to require node.
        log::info!("verify_node_ack_node_active_status");
        let topic_message = TopicMessage {
            sub_topic: SubTopics::AckNodeActiveStatus(NodeActiveStatus::Actived, timestamp.clone()),
            data: vec![],
        };
        let peer_message = Message::NetworkMessage(NetworkMessage {
            peer_id: None,
            topic: Topics::NodeStatus,
            message: serde_cbor::to_vec(&topic_message).unwrap(),
        });
        self.network_caller.notify(peer_message).await?;
        Ok(())
    }

    async fn distribute_task(&mut self) -> Result<()> {
        let mut node_list = self.node_list.clone();
        node_list.shuffle(&mut rand::thread_rng());
        let task_list = self
            .task_caller
            .clone()
            .expect("can't get task caller")
            .call(Message::LocalMessage(LocalMessage::GetTaskList()))
            .await?;
        if task_list.is_none() {
            return Err(NodeError {
                message: "Task list is empty!".to_owned(),
            });
        }
        let task_list = match task_list.unwrap() {
            Message::LocalMessage(LocalMessage::GetTaskListResponse(task_list)) => task_list,
            _ => vec![],
        };
        if task_list.is_empty() {
            return Err(NodeError {
                message: "Task list is empty!".to_owned(),
            });
        }
        self.task_distribute_list.clear();
        let mut node_take_task_num_map: HashMap<String, i32> = HashMap::new();
        let node_take_task_total_limit = 2;
        for mut task in task_list {
            for node in node_list.clone() {
                if node.active_status == NodeActiveStatus::Inactived {
                    continue;
                }
                let current_node_taked_task_num = node_take_task_num_map.get(&node.peer_id);
                if current_node_taked_task_num.is_none() {
                    let current_node_taked_task_num = 1;
                    node_take_task_num_map
                        .insert(node.peer_id.clone(), current_node_taked_task_num);
                    self.task_distribute_list.push(TaskDistributeData {
                        task_id: task.id,
                        peer_id: node.peer_id.clone(),
                    });
                } else {
                    if *current_node_taked_task_num.unwrap() < node_take_task_total_limit {
                        let current_node_taked_task_num = current_node_taked_task_num.unwrap() + 1;
                        node_take_task_num_map
                            .insert(node.peer_id.clone(), current_node_taked_task_num);
                        self.task_distribute_list.push(TaskDistributeData {
                            task_id: task.id,
                            peer_id: node.peer_id.clone(),
                        });
                    } else {
                        continue;
                    }
                }
                task.current_node_num = task.current_node_num + 1;
                if task.current_node_num == task.node_limit {
                    break;
                }
            }
        }
        let topic_msg = TopicMessage {
            data: vec![],
            sub_topic: SubTopics::DistributeTask(self.task_distribute_list.clone()),
        };
        let res = self
            .network_caller
            .notify(Message::NetworkMessage(NetworkMessage {
                peer_id: Some(self.peer_id),
                topic: Topics::TaskList,
                message: serde_cbor::to_vec(&topic_msg)?,
            }))
            .await;
        if res.is_err() {
            return Err(NodeError {
                message: "Distribute task list fail!".to_owned(),
            });
        }
        Ok(())
    }
}

/// `run` function to watch peer msg or local msg, build the connection with other node or module
pub fn run(node: NodeModule) {
    thread::spawn(move || task::block_on(watch_message(node)));
}

// `watch_message` will watch the message from peer or local module, then deal the message.
async fn watch_message(node: NodeModule) {
    log::info!("node watch message");
    let mut node = node;

    let waiter = node.message_waiter;
    node.message_waiter = None;
    let mut waiter = match waiter {
        Some(waiter) => waiter,
        None => return,
    };

    waiter
        .wait(|msg| match msg {
            Message::NetworkMessage(network_msg) => {
                log::info!("NodeModule: receive peer msg!");
                let topic_msg: TopicMessage = serde_cbor::from_slice(&network_msg.message).unwrap();
                deal_peer_message(&mut node, &topic_msg);
                None
            }
            Message::LocalMessage(local_msg) => {
                task::block_on(deal_local_message(&mut node, &local_msg))
            }
        })
        .await;
}

pub fn deal_peer_message(node: &mut NodeModule, msg: &TopicMessage) {
    let sub_topic = msg.sub_topic.clone();
    match sub_topic {
        SubTopics::Ping => {
            let ping_msg: PingMessage = serde_cbor::from_slice(&msg.data).unwrap();
            let mut exist = false;
            for node_data in node.node_list.clone() {
                if ping_msg.peer_id == node_data.peer_id {
                    exist = true;
                    break;
                }
            }
            if !exist {
                node.node_list.push(NodeData {
                    peer_id: ping_msg.peer_id,
                    bind_address: ping_msg.principal_id,
                    status: NodeStatus::Online,
                    active_status: NodeActiveStatus::Inactived,
                    node_type: NodeType::Work,
                    stake_amount: 0,
                    worker_stake_amount: HashMap::new(),
                    vote_amount: 0,
                    voting_rights: 0,
                    worker_vote_amount: HashMap::new(),
                    online_blocks: 0,
                })
            };
        }
        _ => {}
    }
}

pub async fn deal_local_message(node: &mut NodeModule, msg: &LocalMessage) -> Option<Message> {
    match msg {
        LocalMessage::ReqNodeDistributeTask(current_block_index) => {
            let offset_block = current_block_index - node.last_distribute_block;
            if offset_block == 10 {
                let res = node.distribute_task().await;
                if res.is_ok() {
                    node.last_distribute_block = *current_block_index;
                }
            }
            None
        }
        LocalMessage::ReqKeeperInit() => {
            let res = node.verify_node_init().await;
            if res.is_ok() {
                Some(Message::LocalMessage(LocalMessage::AckKeeperInit(true)))
            } else {
                Some(Message::LocalMessage(LocalMessage::AckKeeperInit(false)))
            }
        }
        _ => None,
    }
}

impl NetworkModule for NodeModule {
    // send data to network
    fn set_message_caller(&mut self, caller: Caller) {
        self.network_caller = caller;
    }

    // receive data from network
    fn get_message_subscribe(&self) -> Vec<(Topics, Caller)> {
        self.message_subscribe.clone()
    }
}

impl LocalMessageModule for NodeModule {
    fn get_message_caller(&self) -> Caller {
        self.message_waiter.as_ref().unwrap().get_caller()
    }
}
