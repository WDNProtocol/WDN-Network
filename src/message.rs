use ethereum_types::H256;
use futures::{channel::mpsc, stream::Next, Future, SinkExt, StreamExt};
use serde::Serialize;
use std::{collections::HashMap, error::Error, fmt, pin::Pin, result, thread};

use crate::{
    blockchain::db::{ActivationOperation, Block, NeedSignData, NodeActivation, TaskOperation},
    database::data_types::{NodeActiveStatus, TaskData},
    module_quick_from,
    network::NetworkMessage,
};
use futures::channel::mpsc::{channel, Receiver, Sender};

pub enum Message {
    NetworkMessage(NetworkMessage),
    LocalMessage(LocalMessage),
}

#[derive(Debug)]
pub enum LocalMessage {
    RequireTask(),
    RequireNodeList(),
    RequireNodeStart(String),
    AckNodeStart(bool, String),
    InvokeTask(),
    GetTaskList(),
    GetTaskListResponse(Vec<TaskData>),
    ReqKeeperInit(),
    AckKeeperInit(bool),
    ReqWorkerActive(),
    AckWorkerActive(bool),
    ReqWorkerActiveStatus(String),
    AckWorkerActiveStatus(NodeActiveStatus),
    ReqNodeDistributeTask(u64),
    ReqBlockSaveNodeActivation(NeedSignData<NodeActivation>, H256, H256, H256),
    AckBlockSaveNodeActivation(bool),
    ReqBlockSaveTaskOperation(Vec<TaskOperation>, H256, H256, H256),
    AckBlockSaveTaskOperation(bool),
    ReqBlockStartTick(),
    AckBlockStartTick(bool),
    ReqBlockCurrent(),
    AckBlockCurrent(Block),
    ReqBlockPack(),
    ReqTaskInitGenesis(),
    AckTaskInitGenesis(bool),
    BlockTick(),
}

pub trait LocalMessageModule {
    fn get_message_caller(&self) -> Caller;
}

pub type Result<T> = result::Result<T, MessageError>;

#[derive(Serialize, Debug, Default)]
pub struct MessageError {
    pub message: String,
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MessageError {}

module_quick_from!(futures::channel::mpsc::SendError, MessageError);

pub struct InnerMessage<Message> {
    pub msg: Message,
    sender: Option<mpsc::Sender<Message>>,
}

#[derive(Clone)]
pub struct Caller {
    remote_sender: mpsc::Sender<InnerMessage<Message>>,
}

impl Caller {
    pub async fn call(&mut self, msg: Message) -> Result<Option<Message>> {
        let (local_sender, mut local_receiver) = mpsc::channel::<Message>(100);

        self.remote_sender
            .send(InnerMessage {
                msg,
                sender: Some(local_sender.clone()),
            })
            .await?;
        Ok(local_receiver.next().await)
    }

    pub async fn notify(&mut self, msg: Message) -> Result<()> {
        self.remote_sender
            .send(InnerMessage { msg, sender: None })
            .await?;
        Ok(())
    }
}

pub struct Waiter {
    local_receiver: mpsc::Receiver<InnerMessage<Message>>,
    local_sender: mpsc::Sender<InnerMessage<Message>>,
}

impl Waiter {
    pub fn new() -> Waiter {
        let (tx, rx) = mpsc::channel::<InnerMessage<Message>>(100);
        Waiter {
            local_receiver: rx,
            local_sender: tx,
        }
    }

    pub fn get_caller(&self) -> Caller {
        Caller {
            remote_sender: self.local_sender.clone(),
        }
    }

    pub async fn wait<F>(&mut self, mut func: F)
    where
        F: FnMut(Message) -> Option<Message>,
    {
        loop {
            let msg = self.local_receiver.next().await;
            let msg = match msg {
                Some(m) => m,
                None => continue,
            };

            let res = func(msg.msg);

            if let Some(mut sender) = msg.sender {
                if let Some(res) = res {
                    let err = sender.send(res).await;
                    if err.is_err() {
                        log::error!("response msg faield. {:?}", err);
                    }
                }
            }
        }
    }

    pub fn next(&mut self) -> Next<'_, mpsc::Receiver<InnerMessage<Message>>> {
        self.local_receiver.next()
    }
}
