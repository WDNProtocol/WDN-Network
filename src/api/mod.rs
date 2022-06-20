use actix_web::{
    dev::Server,
    get, post,
    web::{self, Data},
    App, Error, HttpResponse, HttpServer,
};
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};

use crate::{
    database::data_types::TaskData,
    message::{self, Caller, LocalMessage, LocalMessageModule, Message},
};

use self::config::ApiConfig;
pub mod config;

#[derive(Clone)]
pub struct ApiModule {
    caller: Caller,
    conf: ApiConfig,
}

impl ApiModule {
    pub fn new(caller: Caller, conf: ApiConfig) -> ApiModule {
        ApiModule {
            caller: caller,
            conf,
        }
    }
}

/// start api
pub fn run(api: ApiModule) -> Server {
    let api_config = api.conf.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(api.clone()))
            .service(keeper_init)
            .service(worker_active)
            .service(get_keeper_node_list)
    })
    .bind((api_config.host, api_config.port))
    .unwrap()
    .system_exit()
    .workers(1)
    .run()
}

#[post("/keeper/init")]
async fn keeper_init(api_module: Data<ApiModule>) -> Result<HttpResponse, Error> {
    log::info!("keeper/init");
    let result = api_module
        .caller
        .clone()
        .call(Message::LocalMessage(LocalMessage::ReqKeeperInit()))
        .await;
    if result.is_err() {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    let msg = result.unwrap();
    if msg.is_none() {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    let init_res =
        if let Message::LocalMessage(LocalMessage::AckKeeperInit(init_res)) = msg.unwrap() {
            init_res
        } else {
            false
        };
    if !init_res {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success()))
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeActiveDto {
    peer_id: String,
}

#[post("/worker/active")]
async fn worker_active(
    api_module: Data<ApiModule>,
    form: web::Json<NodeActiveDto>,
) -> Result<HttpResponse, Error> {
    log::info!("worker/active");
    let active_res_msg = api_module
        .caller
        .clone()
        .call(Message::LocalMessage(LocalMessage::ReqWorkerActive()))
        .await;
    if active_res_msg.is_err() {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    let active_res_msg = active_res_msg.unwrap();
    if active_res_msg.is_none() {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    let active_res = if let Message::LocalMessage(LocalMessage::AckWorkerActive(active_res)) =
        active_res_msg.unwrap()
    {
        active_res
    } else {
        false
    };
    if active_res {
        return Ok(HttpResponse::Ok().json(ApiResponse::error_default()));
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success()))
}

#[get("/keeper/node_list")]
async fn get_keeper_node_list(api_module: Data<ApiModule>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(ApiResponse::success()))
}

#[derive(Serialize, Deserialize)]
struct ApiResponse<T>
where
    T: Serialize,
{
    code: String,
    msg: String,
    data: Option<T>,
}

impl ApiResponse<()> {
    pub fn success() -> Self {
        ApiResponse {
            code: "200".to_owned(),
            msg: "success".to_owned(),
            data: None,
        }
    }

    pub fn error_default() -> Self {
        ApiResponse {
            code: "201".to_owned(),
            msg: "fail".to_owned(),
            data: None,
        }
    }

    pub fn error(code: String, msg: String) -> Self {
        ApiResponse {
            code,
            msg: msg,
            data: None,
        }
    }
}
