use std::{str::FromStr, sync::Arc, thread};

use crate::{
    blockchain::db::{TaskOperation, TaskOperationType},
    database::data_types::{TaskData, TaskDistributeData, TaskStatus, TaskType},
    message::{Caller, LocalMessage, LocalMessageModule, Message, Waiter},
    network::{
        topics::{self, SubTopics, TopicMessage},
        NetworkMessage, NetworkModule,
    },
    task_process::TaskProcessServer,
};
use async_std::task;
use error::Result;
use ethereum_types::H256;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    prelude::*,
    select,
};
use kvdb::KeyValueDB;
use log::info;
use topics::Topics;

use self::{db::TaskDB, error::TaskError};

pub mod config;
pub mod db;
pub mod error;

pub struct TaskModule {
    db: TaskDB,
    peer_id: String,
    message_waiter: Option<Waiter>,
    network_caller: Caller,
    message_subscribe: Vec<(Topics, Caller)>,
    all_task_list: Vec<TaskData>,
    running_task_list: Vec<TaskData>,
}

impl TaskModule {
    pub fn new(db: Arc<dyn KeyValueDB>, peer_id: String) -> Result<TaskModule> {
        let message_waiter = Waiter::new();
        let message_subscribe = vec![
            (Topics::TakeTask, message_waiter.get_caller()),
            (Topics::TaskList, message_waiter.get_caller()),
        ];

        let task_db = TaskDB::new(db)?;
        Ok(TaskModule {
            db: task_db,
            peer_id: peer_id,
            network_caller: message_waiter.get_caller(),
            message_waiter: Some(message_waiter),
            message_subscribe,
            all_task_list: vec![],
            running_task_list: vec![],
        })
    }

    async fn genesis_init_task(&mut self) -> Result<()> {
        let task_one_add_operation = TaskOperation {
            id: 1,
            operation: TaskOperationType::Add,
            binary_hash: H256::from_str("one").unwrap(),
            task_type: TaskType::LongTerm,
            node_limit: 100,
            reward_weight: 100,
        };
        self.db
            .insert_task_operation(task_one_add_operation.clone())?;
        let task_one = TaskData {
            id: task_one_add_operation.id.clone(),
            hash: task_one_add_operation.binary_hash.clone(),
            task_type: task_one_add_operation.task_type.clone(),
            node_limit: task_one_add_operation.node_limit.clone(),
            current_node_num: 0,
            status: TaskStatus::Enable,
            reward_weight: task_one_add_operation.reward_weight.clone(),
        };
        self.db.insert_task(task_one.clone())?;
        let task_two_add_operation = TaskOperation {
            id: 2,
            operation: TaskOperationType::Add,
            binary_hash: H256::from_str("two").unwrap(),
            task_type: TaskType::LongTerm,
            node_limit: 100,
            reward_weight: 200,
        };
        self.db
            .insert_task_operation(task_two_add_operation.clone())?;
        let task_two = TaskData {
            id: task_two_add_operation.id.clone(),
            hash: task_two_add_operation.binary_hash.clone(),
            task_type: task_two_add_operation.task_type.clone(),
            node_limit: task_two_add_operation.node_limit.clone(),
            current_node_num: 0,
            status: TaskStatus::Enable,
            reward_weight: task_two_add_operation.reward_weight.clone(),
        };
        self.db.insert_task(task_two.clone())?;
        let task_root = self.db.task_db.get_root();
        let task_operation_root = self.db.task_operation_db.get_root();
        let temp_task_operation_root = self.db.temp_task_operation_db.get_root();
        let res = self
            .network_caller
            .call(Message::LocalMessage(
                LocalMessage::ReqBlockSaveTaskOperation(
                    vec![
                        task_one_add_operation.clone(),
                        task_two_add_operation.clone(),
                    ],
                    H256(task_root),
                    H256(task_operation_root),
                    H256(temp_task_operation_root),
                ),
            ))
            .await?;
        let res = if let Some(Message::LocalMessage(LocalMessage::AckBlockSaveTaskOperation(res))) =
            res
        {
            res
        } else {
            return Err(TaskError {
                message: "init task fail".to_owned(),
            });
        };
        if !res {
            return Err(TaskError {
                message: "init task fail".to_owned(),
            });
        }
        Ok(())
    }

    /// `verify_node_pre_set_task_list` pre set verify node task list into first block
    fn verify_node_pre_set_task_list(&self) {}
}

impl NetworkModule for TaskModule {
    // send data to network
    fn set_message_caller(&mut self, caller: Caller) {
        self.network_caller = caller;
    }

    // receive data from network
    fn get_message_subscribe(&self) -> Vec<(Topics, Caller)> {
        self.message_subscribe.clone()
    }
}

impl LocalMessageModule for TaskModule {
    fn get_message_caller(&self) -> Caller {
        self.message_waiter.as_ref().unwrap().get_caller()
    }
}

pub fn run(task_module: TaskModule) {
    thread::spawn(move || task::block_on(watch_msg(task_module)));
}

async fn check_task_list(task_module: &mut TaskModule) -> Result<()> {
    if task_module.all_task_list.is_empty() {
        let topic_msg = TopicMessage {
            sub_topic: SubTopics::ReqTaskList,
            data: vec![],
        };
        let peer_message = Message::NetworkMessage(NetworkMessage {
            peer_id: None,
            topic: Topics::TaskList,
            message: serde_cbor::to_vec(&topic_msg).unwrap(),
        });
        task_module.network_caller.notify(peer_message).await;
    }
    Ok(())
}

/// `start_invoke_task` start invoke task, pick_tasks then check the every task requirement,
/// if the node can invoke that task, add the task into running_task_list, nofity all_task_list
/// current task curren_node_num add one.
async fn start_invoke_task(task_module: &mut TaskModule) {
    let picked_task_list = pick_task(task_module).await;
    if picked_task_list.is_empty() {
        log::error!("TaskModule : TaskModule can't find excutable task!");
    } else {
        for task in picked_task_list {
            invoke_task(task_module, task).await;
        }
    }
}

async fn pick_task(task_module: &mut TaskModule) -> Vec<TaskData> {
    task_module.all_task_list.clone()
}

async fn invoke_task(task_module: &mut TaskModule, task: TaskData) {
    let mut task_process = TaskProcessServer::new(task.clone(), task_module.network_caller.clone());
    task::block_on(task_process.start());
}

async fn watch_msg(mut task_module: TaskModule) {
    log::info!("TaskModule : watch msg");

    let waiter = task_module.message_waiter;
    task_module.message_waiter = None;
    let mut waiter = match waiter {
        Some(waiter) => waiter,
        None => return,
    };

    waiter
        .wait(|msg| match msg {
            Message::NetworkMessage(network_msg) => {
                log::info!("NodeModule: receive peer msg!");
                let topic_msg: TopicMessage = serde_cbor::from_slice(&network_msg.message).unwrap();
                task::block_on(deal_peer_message(&mut task_module, &topic_msg));

                None
            }
            Message::LocalMessage(local_msg) => {
                task::block_on(deal_local_message(&mut task_module, &local_msg))
            }
        })
        .await;
}

pub async fn deal_peer_message(task_module: &mut TaskModule, msg: &TopicMessage) {
    let sub_topic = msg.sub_topic.clone();
    match sub_topic {
        topics::SubTopics::AckTaskList => {
            log::info!("peer receive ack task list");
            let task_list: Vec<TaskData> =
                serde_cbor::from_slice(&msg.data).expect("parse data error");
            task_module.all_task_list = task_list;
        }
        topics::SubTopics::DistributeTask(task_list) => {
            log::info!("peer receive distribute task list");
            let current_worker_task_list: Vec<TaskDistributeData> = task_list
                .into_iter()
                .filter(|x| x.peer_id == task_module.peer_id)
                .collect();
        }
        _ => {}
    }
    // TODO: "need to judge whether the node type is verify node"
    let sub_topic = msg.sub_topic.clone();
    match sub_topic {
        topics::SubTopics::ReqTaskList => {
            log::info!("peer receive require task list");
            // TODO: modify this response
            let task_list = vec![TaskData {
                id: 0,
                hash: H256::from_slice(&[0u8; 32]),
                task_type: TaskType::LongTerm,
                node_limit: 1000,
                current_node_num: 0,
                status: TaskStatus::Disable,
                reward_weight: 100,
            }];
            let topic_msg = TopicMessage {
                sub_topic: SubTopics::AckTaskList,
                data: serde_cbor::to_vec(&task_list).unwrap(),
            };
            let peer_msg = Message::NetworkMessage(NetworkMessage {
                peer_id: None,
                topic: Topics::TaskList,
                message: serde_cbor::to_vec(&topic_msg).unwrap(),
            });
            task_module.network_caller.notify(peer_msg).await.unwrap();
        }
        topics::SubTopics::GetTaskList(task_id_list) => {}
        _ => {}
    }
}

/// `deal_local_message` will deal the local message comunication
async fn deal_local_message(task_module: &mut TaskModule, msg: &LocalMessage) -> Option<Message> {
    match msg {
        LocalMessage::RequireTask() => {
            info!("require task list");
            let topic_message = TopicMessage {
                sub_topic: SubTopics::ReqTaskList,
                data: vec![],
            };
            Some(Message::NetworkMessage(NetworkMessage {
                peer_id: None,
                topic: Topics::TaskList,
                message: serde_cbor::to_vec(&topic_message).unwrap(),
            }))
        }
        LocalMessage::InvokeTask() => {
            info!("task_module : invoke task");
            start_invoke_task(task_module).await;
            None
        }
        LocalMessage::GetTaskList() => {
            log::info!("TaskModule: get task list.");
            Some(Message::LocalMessage(LocalMessage::GetTaskListResponse(
                task_module.all_task_list.clone(),
            )))
        }
        LocalMessage::ReqTaskInitGenesis() => {
            log::info!("init genesis");
            Some(Message::LocalMessage(LocalMessage::AckTaskInitGenesis(
                true,
            )))
        }
        _ => None,
    }
}
