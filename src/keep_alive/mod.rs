use std::{thread, time::Duration};

use async_std::task;
use chrono::Local;

use crate::{
    message::{Caller, Message},
    network::{
        topics::{PingMessage, SubTopics, TopicMessage, Topics},
        NetworkMessage,
    },
};

pub fn run(network_caller: Caller, principal_id: String, peer_id: String) {
    thread::spawn(move || loop {
        task::block_on(keep_alive(
            network_caller.clone(),
            principal_id.clone(),
            peer_id.clone(),
        ));
    });
}

// `keep_alive` send keep alive message to verify node, verify node check the node is online, append reward to the node.
async fn keep_alive(mut network_caller: Caller, principal_id: String, peer_id: String) {
    log::info!("node start send keep alive");
    thread::sleep(Duration::from_millis(1000));
    let ping_message = PingMessage {
        principal_id,
        timestamp: Local::now().timestamp(),
        peer_id,
    };
    let topic_message = TopicMessage {
        sub_topic: SubTopics::Ping,
        data: serde_cbor::to_vec(&ping_message).unwrap(),
    };

    let peer_message = Message::NetworkMessage(NetworkMessage {
        peer_id: None,
        topic: Topics::KeepAlive,
        message: serde_cbor::to_vec(&topic_message).unwrap(),
    });
    let res = network_caller.notify(peer_message).await;
    if res.is_err() {
        log::error!("send keep alive msg error : {:?}", &res);
    }
}
