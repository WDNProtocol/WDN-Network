use std::{sync::Arc, thread, time::Duration};

use ethereum_types::H256;
use kvdb::KeyValueDB;

use crate::{
    ic::wdn_identity::WdnIdentity,
    message::{Caller, LocalMessage, LocalMessageModule, Message, Waiter},
    network::{
        topics::{TopicMessage, Topics},
        NetworkModule,
    },
};
use async_std::task;

use self::db::{Block, BlockchainDB, NeedSignData, NodeActivation, TaskOperation};
use self::error::Result;

pub mod db;
pub mod error;

pub struct BlockchainModule {
    db: BlockchainDB,
    message_waiter: Option<Waiter>,
    network_caller: Caller,
    message_subscribe: Vec<(Topics, Caller)>,
    current_block: Block,
}

impl BlockchainModule {
    pub fn new(db_backend: Arc<dyn KeyValueDB>) -> Result<BlockchainModule> {
        let db = BlockchainDB::new(db_backend)?;
        let message_waiter = Waiter::new();
        let message_subscribe = vec![];
        let block = Block::default();
        Ok(BlockchainModule {
            db: db,
            network_caller: message_waiter.get_caller(),
            message_waiter: Some(message_waiter),
            message_subscribe: message_subscribe,
            current_block: block,
        })
    }

    fn save_node_activation(
        &mut self,
        node_activation: NeedSignData<NodeActivation>,
        node_root: H256,
        node_activation_root: H256,
        current_activation_root: H256,
    ) -> Result<()> {
        self.current_block
            .body
            .node_activation
            .push(node_activation);
        self.current_block.header.node_root = node_root;
        self.current_block.header.node_activation_root = node_activation_root;
        self.current_block.header.current_node_activation_root = current_activation_root;
        Ok(())
    }

    fn save_task_operation(
        &mut self,
        mut task_operation_list: Vec<TaskOperation>,
        task_root: H256,
        task_operation_root: H256,
        current_task_operation_root: H256,
    ) -> Result<()> {
        self.current_block
            .body
            .tasks
            .append(&mut task_operation_list);
        self.current_block.header.task_root = task_root;
        self.current_block.header.task_operation_root = task_operation_root;
        self.current_block.header.current_task_operation_root = current_task_operation_root;
        Ok(())
    }

    fn start_tick(&mut self) -> Result<()> {
        log::info!("start blockchain tick!");
        let caller = self.network_caller.clone();
        thread::spawn(|| async move {
            loop {
                thread::sleep(Duration::from_millis(1000));
                let res = caller
                    .clone()
                    .call(Message::LocalMessage(LocalMessage::ReqBlockPack()))
                    .await;
                if res.is_ok() {
                    log::info!("blockchain send pack message success!")
                } else {
                    log::info!("blockchain send pack message fail!")
                }
            }
        });
        Ok(())
    }

    /// `pack_block` will pack a block append to the blockchain
    pub fn pack_block(&mut self) -> Result<()> {
        log::info!("Pack Block!");
        let need_pack_block = self.current_block.clone();
        // Distribute reward here
        let total_task_weight: u64 = need_pack_block.body.task_results.iter().map(|x| x.id).sum();
        let res = self.db.insert_block(need_pack_block);
        let last_index = self.current_block.header.index.clone();
        self.current_block = Block::default();
        self.current_block.header.index = last_index + 1;
        Ok(())
    }
}

impl NetworkModule for BlockchainModule {
    fn set_message_caller(&mut self, caller: Caller) {
        self.network_caller = caller;
    }

    fn get_message_subscribe(&self) -> Vec<(Topics, Caller)> {
        self.message_subscribe.clone()
    }
}

impl LocalMessageModule for BlockchainModule {
    fn get_message_caller(&self) -> Caller {
        self.message_waiter.as_ref().unwrap().get_caller()
    }
}

pub fn run(blockchain_module: BlockchainModule) {
    thread::spawn(move || task::block_on(watch_msg(blockchain_module)));
}

async fn watch_msg(mut blockchain_module: BlockchainModule) {
    log::info!("watch msg!");
    let waiter = blockchain_module.message_waiter;
    blockchain_module.message_waiter = None;
    let mut waiter = match waiter {
        Some(waiter) => waiter,
        None => return,
    };

    waiter
        .wait(|msg| match msg {
            crate::message::Message::NetworkMessage(network_msg) => {
                log::info!("Receive peer msg!");
                let topic_msg: TopicMessage = serde_cbor::from_slice(&network_msg.message).unwrap();
                task::block_on(deal_peer_message(&mut blockchain_module, &topic_msg));
                None
            }
            crate::message::Message::LocalMessage(local_msg) => {
                task::block_on(deal_local_message(&mut blockchain_module, &local_msg))
            }
        })
        .await;
}

async fn deal_peer_message(blockchain_module: &mut BlockchainModule, msg: &TopicMessage) {
    match msg {
        _ => {}
    }
}

async fn deal_local_message(
    blockchain_module: &mut BlockchainModule,
    msg: &LocalMessage,
) -> Option<Message> {
    match msg {
        LocalMessage::ReqBlockCurrent() => Some(Message::LocalMessage(
            LocalMessage::AckBlockCurrent(blockchain_module.current_block.clone()),
        )),
        LocalMessage::ReqBlockSaveNodeActivation(
            node_activation,
            node_root,
            node_activation_root,
            current_node_activation_root,
        ) => {
            let res = blockchain_module.save_node_activation(
                node_activation.clone(),
                node_root.clone(),
                node_activation_root.clone(),
                current_node_activation_root.clone(),
            );
            match res {
                Ok(_) => Some(Message::LocalMessage(
                    LocalMessage::AckBlockSaveNodeActivation(true),
                )),
                Err(_) => Some(Message::LocalMessage(
                    LocalMessage::AckBlockSaveNodeActivation(false),
                )),
            }
        }
        LocalMessage::ReqBlockSaveTaskOperation(
            task_operation_list,
            task_root,
            task_operation_root,
            current_task_operation_root,
        ) => {
            let res = blockchain_module.save_task_operation(
                task_operation_list.clone(),
                task_root.clone(),
                task_operation_root.clone(),
                current_task_operation_root.clone(),
            );
            match res {
                Ok(_) => Some(Message::LocalMessage(
                    LocalMessage::AckBlockSaveTaskOperation(true),
                )),
                Err(_) => Some(Message::LocalMessage(
                    LocalMessage::AckBlockSaveTaskOperation(false),
                )),
            }
        }
        LocalMessage::ReqBlockStartTick() => {
            let res = blockchain_module.start_tick();
            if res.is_ok() {
                Some(Message::LocalMessage(LocalMessage::AckBlockStartTick(true)))
            } else {
                Some(Message::LocalMessage(LocalMessage::AckBlockStartTick(
                    false,
                )))
            }
        }
        LocalMessage::ReqBlockPack() => {
            let res = blockchain_module.pack_block();
            if res.is_ok() {
                log::info!("block packed success!");
            } else {
                log::error!("block packed fail!");
            }
            let res = blockchain_module
                .network_caller
                .notify(Message::LocalMessage(LocalMessage::ReqNodeDistributeTask(
                    blockchain_module.current_block.header.index,
                )))
                .await;
            log::info!("notify node distribute task");
            None
        }
        _ => None,
    }
}
