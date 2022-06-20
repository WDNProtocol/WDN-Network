use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::thread;
use std::time::Duration;

use std::collections::HashMap;

use async_std::task;
use futures::{prelude::*, select};
use libp2p::gossipsub::{GossipsubEvent, GossipsubMessage, MessageAuthenticity, ValidationMode};
use libp2p::gossipsub::{IdentTopic, MessageId, Topic};
use libp2p::identity::Keypair;
use libp2p::{gossipsub, swarm::SwarmEvent, Multiaddr, PeerId};

use crate::message::{Caller, InnerMessage, Message, Waiter};

use self::topics::Topics;

pub mod config;
pub mod topics;
use log;

#[derive(Clone)]
pub struct NetworkMessage {
    pub peer_id: Option<PeerId>,
    pub topic: Topics,
    pub message: Vec<u8>,
}

pub trait NetworkModule {
    // send data to network
    fn set_message_caller(&mut self, _: Caller) {}

    // receive data from network
    fn get_message_subscribe(&self) -> Vec<(Topics, Caller)> {
        vec![]
    }
}

pub struct Network {
    conf: config::NetworkConfig,
    key: Keypair,

    module_message_caller: HashMap<topics::Topics, Caller>,
    message_waiter: Waiter,
}

impl Network {
    pub fn new(conf: config::NetworkConfig, key: Keypair) -> Network {
        Network {
            conf,
            key,

            module_message_caller: HashMap::new(),
            message_waiter: Waiter::new(),
        }
    }

    pub fn add_module<T>(&mut self, module: &mut T)
    where
        T: NetworkModule,
    {
        let sub = module.get_message_subscribe();
        for (topic, caller) in sub {
            self.module_message_caller.insert(topic, caller);
        }
        module.set_message_caller(self.message_waiter.get_caller());
    }
}

pub fn run(network: Network) {
    thread::spawn(move || {
        task::block_on(async {
            message_loop(network).await.unwrap();
        })
    });
}

async fn message_loop(mut network: Network) -> Result<(), Box<dyn Error>> {
    // TODO: check whether local_key was exist, if not exist, create one then store it.
    let local_key = network.key;
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    // Set up an encrypted TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::development_transport(local_key.clone()).await?;

    // Create a Swarm to manage peers and events
    let mut swarm = {
        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        // Set a custom gossipsub
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the
            // same content will be propagated.
            .build()
            .expect("Valid config");
        // build a gossipsub network behaviour
        let mut gossipsub: gossipsub::Gossipsub =
            gossipsub::Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
                .expect("Correct configuration");

        // subscribes to sync topic
        gossipsub.subscribe(&topics::sync_topic()).unwrap();

        // subscribe module topic
        for (topic, _) in &network.module_message_caller {
            let t: IdentTopic = Topic::new(topic.clone());
            gossipsub.subscribe(&t).unwrap();
        }

        // build the swarm
        libp2p::Swarm::new(transport, gossipsub, local_peer_id)
    };

    // Listen on all interfaces
    let interface = format!("/ip4/0.0.0.0/tcp/{:?}", network.conf.port);
    swarm.listen_on(interface.parse().unwrap()).unwrap();

    for known_node in &network.conf.known_nodes {
        let known_node_str = known_node.as_str();
        if let Some(s) = known_node_str {
            let address: Multiaddr = s.parse().expect("User to provide valid address.");
            match swarm.dial(address.clone()) {
                Ok(_) => log::info!("Dialed {:?}", address),
                Err(e) => log::info!("Dial {:?} failed: {:?}", address, e),
            };
        }
    }

    // Kick it off
    loop {
        select! {
            msg = network.message_waiter.next() => match msg {
                Some(InnerMessage{ msg: Message::NetworkMessage(NetworkMessage{ peer_id, topic, message }), ..}) => {
                    let t: IdentTopic = Topic::new(topic);
                    if let Err(e) = swarm.behaviour_mut().publish(t, message) {
                        log::info!("Publish error: {:?}", e);
                    }
                },
                _ => {
                    log::info!("none")
                }
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(GossipsubEvent::Message {
                    propagation_source: _,
                    message_id: _,
                    message,
                }) => {
                    log::info!("reveive message {:?}", &message);
                    let topic: topics::Topics = message.clone().topic.into_string().into();
                    let chan = network.module_message_caller.get_mut(&topic);
                    match chan {
                        Some(c) => {
                            log::info!("find dealer");
                            let msg = Message::NetworkMessage(NetworkMessage{
                                peer_id: message.source,
                                topic,
                                message: message.data,
                            });
                            let res = c.notify(msg).await;
                            log::info!("{:?}", res);
                        },
                        None => {
                            log::info!("can not find dealer");
                        }
                    }
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    log::info!("Listening on {:?}", address);
                },
                _ => {
                    log::info!("receive swarm event {:?}", event);

                    // for test
                    // if let Err(e) = swarm.behaviour_mut().publish(topics::sync_topic(), b"hello".to_vec()) {
                    //     log::info!("Publish error: {:?}", e);
                    // }
                },
            },
        }
    }
}
