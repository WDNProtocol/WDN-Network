use std::{io::{self, Write}, env::args, ops::Add};

use serde_derive::{Deserialize, Serialize};

pub trait TaskTemplate{

    fn check_env(&self);

    fn invoke(&self);

    /// `check_result` let verify node to check the result uploaded by work node.
    fn check_result(&self, data: Vec<u8>);
    
}

pub trait TaskCommandDealer<T : TaskTemplate>{
    fn deal_command_invoke(&self, task: &T, command_invoke: TaskCommandInvoke);
    fn deal_command_response(&self, task: &T, command_invoke: TaskCommanResult);
}


pub async fn task_watch<T, F>(task: T, task_dealer: F)
where
    T : TaskTemplate,
    F : TaskCommandDealer<T>
{
    let mut args = args();
    let stdin = io::stdin();
    loop {
        let mut s = String::new();
        stdin.read_line(&mut s).unwrap();
        log::info!("{}", s.trim());
        if !s.is_empty() {
            let task_process_msg : TaskProcessMsg = serde_json::from_str(&s).unwrap();
            match task_process_msg.msg_type {
                TaskProcessMsgType::CommandInvoke => {
                    let command_invoke: TaskCommandInvoke = serde_cbor::from_slice(&task_process_msg.data).unwrap();
                    task_dealer.deal_command_invoke(&task, command_invoke);
                }
                TaskProcessMsgType:: CommandResponse => {
                    let command_result: TaskCommanResult = serde_cbor::from_slice(&task_process_msg.data).unwrap();
                    task_dealer.deal_command_response(&task, command_result)
                }
            }
        }
    }
}

pub fn check_env_success() {
    let msg = TaskCommanResult{
        command: TaskCommand::CheckEnv,
        code: String::from("200"),
        msg: String::from("success"),
        data: vec![]
    };
    let process_msg = TaskProcessMsg{
        msg_type: TaskProcessMsgType::CommandResponse,
        data: serde_cbor::to_vec(&msg).unwrap(),
    };
    child_process_send_msg(process_msg);
}

pub fn invoke_success() {
    let msg = TaskCommanResult{
        command: TaskCommand::Invoke,
        code: String::from("200"),
        msg: String::from("success"),
        data: vec![]
    };
    let process_msg = TaskProcessMsg{
        msg_type: TaskProcessMsgType::CommandResponse,
        data: serde_cbor::to_vec(&msg).unwrap(),
    };
    child_process_send_msg(process_msg);
}

pub fn upload_data(data: Vec<u8>) {
    let data = serde_json::to_vec(&data).unwrap();
}

pub fn child_process_send_msg(msg: TaskProcessMsg) {
    let mut stdout = io::stdout();
    stdout.write(&serde_json::to_string(&msg).unwrap().add("\n").as_bytes()).unwrap();
    stdout.flush();
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskProcessMsg{
    pub msg_type: TaskProcessMsgType,
    pub data: Vec<u8>
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TaskProcessMsgType {
    CommandInvoke,
    CommandResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TaskCommand {
    CheckEnv,
    Invoke,
    CheckResult,
    UploadData
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskCommandInvoke{
    pub command: TaskCommand,
    pub data: Vec<u8>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskCommanResult{
    pub command: TaskCommand,
    pub code: String,
    pub msg: String,
    pub data: Vec<u8>
}

pub struct DefaultTaskCommandDealer{}

impl DefaultTaskCommandDealer{
    pub fn new() -> DefaultTaskCommandDealer {
        DefaultTaskCommandDealer {  }
    }
}

impl<T> TaskCommandDealer<T> for DefaultTaskCommandDealer 
where
    T : TaskTemplate
{
    fn deal_command_invoke(&self, task: &T, command_invoke: TaskCommandInvoke) {
        match command_invoke.command {
            TaskCommand::CheckEnv => {
                task.check_env();
            },
            TaskCommand::Invoke => {
                task.invoke()
            },
            TaskCommand::CheckResult => {
                task.check_result(command_invoke.data)
            },
            _ => {}
        }
    }

    fn deal_command_response(&self, task: &T, command_invoke: TaskCommanResult) {
        todo!()
    }
}