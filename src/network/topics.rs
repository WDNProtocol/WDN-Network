use libp2p::gossipsub::{IdentTopic, Topic};
use serde_derive::{Deserialize, Serialize};

use crate::database::data_types::{NodeActiveStatus, TaskData, TaskDistributeData};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub enum Topics {
    NodeStatus,
    NodeList,
    TaskList,
    TakeTask,
    TaskResult,
    NewBlock,
    DataSync,
    Vote,
    Election,
    KeepAlive,
    Unknown(String),
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub enum SubTopics {
    ReqNodeList,
    AckNodeList,
    ReqTaskList,
    AckTaskList,
    ReqNodeRunStatus,
    AckNodeRunStatus,
    ReqNodeActiveStatus(String, String),
    AckNodeActiveStatus(NodeActiveStatus, String),
    UploadTaskData,
    DistributeTask(Vec<TaskDistributeData>),
    GetTaskList(Vec<i64>),
    GetTaskListResponse(Vec<TaskData>),
    Ping,
    Pong,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Deserialize, Serialize)]
pub struct TopicMessage {
    pub sub_topic: SubTopics,
    pub data: Vec<u8>,
}

impl From<String> for Topics {
    fn from(str: String) -> Topics {
        match str.as_str() {
            "NodeList" => Topics::NodeList,
            "TaskList" => Topics::TaskList,
            "TakeTask" => Topics::TakeTask,
            "TaskResult" => Topics::TaskResult,
            "NewBlock" => Topics::NewBlock,
            "DataSync" => Topics::DataSync,
            "Vote" => Topics::Vote,
            "Election" => Topics::Election,
            _ => Topics::Unknown(str),
        }
    }
}

impl Into<String> for Topics {
    fn into(self) -> String {
        format!("{:?}", self)
    }
}

pub fn sync_topic() -> IdentTopic {
    Topic::new(Topics::DataSync)
}

pub fn verifier_topics() -> Vec<IdentTopic> {
    vec![
        Topic::new(Topics::TaskList),
        Topic::new(Topics::TakeTask),
        Topic::new(Topics::TaskResult),
        Topic::new(Topics::NewBlock),
        Topic::new(Topics::Vote),
        Topic::new(Topics::Election),
    ]
}

pub fn worker_topics() -> Vec<IdentTopic> {
    vec![
        Topic::new(Topics::TaskList),
        Topic::new(Topics::TakeTask),
        Topic::new(Topics::NewBlock),
        Topic::new(Topics::Vote),
        Topic::new(Topics::NodeList),
    ]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingMessage {
    pub principal_id: String,
    pub peer_id: String,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topics() {
        let t = Topics::DataSync;

        let topic_str: String = t.clone().into();

        let t2: Topics = topic_str.into();

        assert!(t.eq(&t2), "wrong hash");
    }

    #[test]
    fn test_serialize() {
        let t = Topics::DataSync;

        let bytes = serde_cbor::to_vec(&t).unwrap();
        let t2: Topics = serde_cbor::from_slice(&bytes).unwrap();

        assert!(t.eq(&t2), "wrong serialize")
    }
}
