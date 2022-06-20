use std::{
    env,
    io::{BufRead, BufReader, Write},
    ops::Add,
    process::{ChildStdin, ChildStdout, Command, Stdio},
    sync::Arc,
};

use futures::{channel::mpsc::Sender, SinkExt};

use crate::{
    database::{data_types::TaskData, AppDB},
    message::{Caller, Message},
    network::{
        topics::{SubTopics, TopicMessage, Topics},
        NetworkMessage,
    },
};

use self::task_process::{
    TaskCommanResult, TaskCommand, TaskCommandInvoke, TaskProcessMsg, TaskProcessMsgType,
};

pub mod task_process;

pub struct TaskProcessServer {
    pub env_status: u8,
    pub node_caller: Caller,
    pub task: Option<TaskData>,
    pub p_stdin: Option<ChildStdin>,
}

impl TaskProcessServer {
    pub fn new(task: TaskData, node_caller: Caller) -> TaskProcessServer {
        TaskProcessServer {
            env_status: 0,
            node_caller,
            task: Some(task),
            p_stdin: None,
        }
    }

    pub async fn start(&mut self) {
        if self.task == None {
            return;
        }
        let path = env::current_dir().unwrap();
        let task = self.task.as_ref().unwrap().clone();
        let invoke_path = path.join("task_bin").join(task.id.to_string());
        if !invoke_path.exists() || !invoke_path.is_file() {
            log::error!("TaskProcessModule : task invoke path not exist!");
            return;
        }
        let mut p = Command::new(invoke_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        self.p_stdin = p.stdin;
        let mut p_stdout = BufReader::new(p.stdout.as_mut().unwrap());

        let mut is_env_checked = false;
        let check_env_command = TaskCommandInvoke {
            command: task_process::TaskCommand::CheckEnv,
            data: vec![],
        };
        let check_env_process_msg = TaskProcessMsg {
            msg_type: TaskProcessMsgType::CommandInvoke,
            data: serde_cbor::to_vec(&check_env_command).unwrap(),
        };
        self.send_command(check_env_process_msg);
        let mut index = 0;
        loop {
            log::info!("start watch child process msg!");
            let mut line = String::new();
            log::info!("start watch child process msg! before");
            p_stdout.read_line(&mut line).unwrap();
            log::info!("start watch child process msg! after");
            if !line.is_empty() {
                log::info!("get child process msg : {}", line);
                let parse_result: Result<TaskProcessMsg, serde_json::Error> =
                    serde_json::from_str(&line);
                match parse_result {
                    Ok(task_process_msg) => match task_process_msg.msg_type {
                        TaskProcessMsgType::CommandInvoke => {
                            let command_invoke: TaskCommandInvoke =
                                serde_cbor::from_slice(&task_process_msg.data).unwrap();
                            self.deal_command_invoke(command_invoke).await;
                        }
                        TaskProcessMsgType::CommandResponse => {
                            let command_result: TaskCommanResult =
                                serde_cbor::from_slice(&task_process_msg.data).unwrap();
                            self.deal_command_response(command_result).await;
                        }
                    },
                    Err(_) => {
                        log::info!("can not parse child process msg");
                    }
                }
            }
            if index < 10 {
                index = index + 1;
            } else {
                break;
            }
        }
    }

    pub async fn deal_command_invoke(&mut self, command_invoke: TaskCommandInvoke) {
        match command_invoke.command {
            TaskCommand::UploadData => {
                let topic_msg = TopicMessage {
                    sub_topic: SubTopics::UploadTaskData,
                    data: serde_cbor::to_vec(&command_invoke.data).unwrap(),
                };
                let peer_msg = Message::NetworkMessage(NetworkMessage {
                    peer_id: None,
                    topic: Topics::TaskResult,
                    message: serde_cbor::to_vec(&topic_msg).unwrap(),
                });
                let res = self.node_caller.notify(peer_msg).await;
                if res.is_ok() {
                    log::info!("task result upload success!");
                } else {
                    log::error!("task result upload fail!");
                }
            }
            _ => {}
        }
    }

    pub async fn deal_command_response(&mut self, command_result: TaskCommanResult) {
        match command_result.command {
            TaskCommand::CheckEnv => {
                if command_result.code == "200" {
                    let invoke_command = TaskCommandInvoke {
                        command: TaskCommand::Invoke,
                        data: vec![],
                    };
                    let process_msg = TaskProcessMsg {
                        msg_type: TaskProcessMsgType::CommandInvoke,
                        data: serde_cbor::to_vec(&invoke_command).unwrap(),
                    };

                    self.send_command(process_msg);
                } else {
                    log::info!("check env fail!");
                }
            }
            TaskCommand::Invoke => {
                if command_result.code == "200" {
                    log::info!("task invoke success!");
                } else {
                    log::info!("task invoke fail!")
                }
            }
            _ => {}
        }
    }

    fn send_command(&mut self, command: TaskProcessMsg) {
        let p_stdin = self.p_stdin.as_mut().expect("no task child process");
        let msg = serde_json::to_string(&command).unwrap();
        log::info!("send command : {:?}", msg);
        p_stdin.write(msg.add("\n").as_bytes()).unwrap();
        let res = p_stdin.flush();
        log::info!("send command result : {:?}", res);
    }
}
