use chrono::Local;
use database::data_types::NodeActiveStatus;
use database::AppDB;
use env_logger::{Builder, Env};
use error::WError;
use futures::channel::mpsc::{channel, SendError, Sender};
use futures::{SinkExt, StreamExt};
use libp2p::identity::Keypair;
use message::{Caller, LocalMessage, Message};
use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;
use crate::{message::LocalMessageModule, task::TaskModule};

mod api;
mod blockchain;
mod config;
mod console;
mod dao;
mod database;
mod dir;
mod encrypt;
mod error;
mod ic;
mod keep_alive;
mod key_pair;
mod message;
mod network;
mod node;
mod task;
mod task_process;

#[macro_use]
mod utils;

#[actix_web::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let res = init().await;
    match res {
        Ok(()) => {}
        Err(e) => {
            log::error!("{}", e)
        }
    }
}

async fn init() -> Result<(), WError> {
    let conf = config::load_config("config.toml".to_string()).unwrap();

    if conf.node_config.principal_id.is_empty() {
        println!("Please set principal id of network config on config.toml");
        return Err(WError {
            code: ErrorCode::BindError,
            message: "Please set principal id of network config on config.toml".to_string(),
        });
    }

    let d = dir::Directories::new(conf.base.data_path.clone());
    d.create_dirs().expect("create directories failed");

    let local_key = key_pair::new(conf.base.data_path.clone())?;

    // database
    let db_backend = database::open_database(d.db.as_str()).expect("open database failed");

    // blockchain module
    let mut blockchain_module = blockchain::BlockchainModule::new(db_backend.clone())?;
    let blockchain_module_caller = blockchain_module.get_message_caller();

    // node module
    let mut node_module = node::NodeModule::new(
        conf.node_config.clone(),
        local_key.clone(),
        db_backend.clone(),
    )?;
    let node_caller = node_module.get_message_caller();
    node_module.blockchain_caller = Some(blockchain_module_caller.clone());

    // task module
    let mut task_module: TaskModule = task::TaskModule::new(
        db_backend.clone(),
        local_key.public().to_peer_id().to_base58(),
    )?;
    let task_caller = task_module.get_message_caller();
    node_module.task_caller = Some(task_caller.clone());

    // Join P2P network.
    let mut net_moudle = network::Network::new(conf.network, local_key.clone());
    net_moudle.add_module(&mut node_module);
    net_moudle.add_module(&mut task_module);
    net_moudle.add_module(&mut blockchain_module);
    network::run(net_moudle);

    // Build the connection between local modules. Watch messages from other peer.
    node::run(node_module);
    task::run(task_module);
    blockchain::run(blockchain_module);

    // upload keep alive
    keep_alive::run(
        node_caller.clone(),
        conf.node_config.principal_id.clone(),
        local_key.public().to_peer_id().to_base58(),
    );

    // Check node active status.
    let check_node_active_status_res = check_node_active_status(node_caller.clone()).await?;
    match check_node_active_status_res {
        NodeActiveStatus::Inactived => {
            // Do nothing now!
        }
        NodeActiveStatus::Actived => {
            // Start sync task list and do task.
        }
    }

    // api module
    let api_module = api::ApiModule::new(node_caller.clone(), conf.api_config.clone());
    let _ = api::run(api_module).await;
    Ok(())
}

// `check_node_active_status`
async fn check_node_active_status(mut node_caller: Caller) -> Result<NodeActiveStatus, WError> {
    let node_check_active_status_msg =
        LocalMessage::ReqWorkerActiveStatus(Local::now().timestamp_millis().to_string());
    let res = node_caller
        .call(message::Message::LocalMessage(node_check_active_status_msg))
        .await;
    let node_check_active_status_res = match res {
        Ok(r) => r,
        Err(e) => {
            log::error!("Check node active status msg send fail!");
            return Err(WError {
                code: ErrorCode::NodeError,
                message: "Check node active status msg send fail!".to_owned(),
            });
        }
    };
    if node_check_active_status_res.is_none() {
        log::error!("Check node active status msg receive none!");
        return Err(WError {
            code: ErrorCode::NodeError,
            message: "Check node active status msg receive none!".to_owned(),
        });
    }
    match node_check_active_status_res {
        Some(Message::LocalMessage(LocalMessage::AckWorkerActiveStatus(node_active_status))) => {
            log::info!("Current node active status is : {:?}", node_active_status);
            return Ok(node_active_status);
        }
        _ => {
            log::error!("Get node active status fail!");
            return Err(WError {
                code: ErrorCode::NodeError,
                message: "Get node active status fail!".to_owned(),
            });
        }
    }
}

async fn check_task_list() {}

async fn start_do_task(task_sender: &mut Sender<LocalMessage>) -> Result<(), SendError> {
    let msg = LocalMessage::InvokeTask();
    task_sender.send(msg).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalKey {
    iv: Vec<u8>,
    local_key: Vec<u8>,
}
