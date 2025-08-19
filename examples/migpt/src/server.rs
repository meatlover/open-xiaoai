use neon::prelude::Context;
use neon::types::JsUint8Array;
use open_xiaoai::base::{AppError, VERSION};
use open_xiaoai::services::connect::data::{Event, Request, Response, Stream};
use open_xiaoai::services::connect::handler::MessageHandler;
use open_xiaoai::services::connect::message::{MessageManager, WsStream};
use open_xiaoai::services::connect::rpc::RPC;
use open_xiaoai::services::speaker::SpeakerManager;
use open_xiaoai::utils::task::TaskManager;

use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, accept_hdr_async};
use tokio_tungstenite::tungstenite::handshake::server::{Request as WsRequest, Response as WsResponse};

use crate::node::NodeManager;

pub struct AppServer;

async fn test() -> Result<(), AppError> {
    SpeakerManager::play_text("已连接").await?;

    // let _ = RPC::instance()
    //     .call_remote("start_recording", None, None)
    //     .await;

    // let _ = RPC::instance().call_remote("start_play", None, None).await;

    Ok(())
}

impl AppServer {
    pub async fn connect(stream: TcpStream) -> Result<WsStream, AppError> {
        // Log handshake request to help debug Cloudflare/headers
        let cb = |req: &WsRequest, resp: WsResponse| -> Result<WsResponse, tokio_tungstenite::tungstenite::Error> {
            let path = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
            println!(
                "🛰️  WS handshake: method={} path={} host={:?}",
                req.method(),
                path,
                req.headers().get("host").and_then(|v| v.to_str().ok())
            );
            Ok(resp)
        };
        let ws_stream = accept_hdr_async(stream, cb)
            .await
            .map_err(|e| AppError::from(format!("websocket accept failed: {e}")))?;
        Ok(WsStream::Server(ws_stream))
    }

    pub async fn run() {
        let addr = "0.0.0.0:4399";
        let listener = TcpListener::bind(&addr)
            .await
            .expect(format!("❌ 绑定地址失败: {}", &addr).as_str());
        println!("✅ 已启动: {:?}", addr);
        while let Ok((stream, addr)) = listener.accept().await {
            // 同一时刻只处理一个连接
            AppServer::handle_connection(stream, addr).await;
        }
    }

    async fn handle_connection(stream: TcpStream, addr: std::net::SocketAddr) {
        match AppServer::connect(stream).await {
            Ok(ws_stream) => {
                println!("✅ 已连接: {:?}", addr);
                AppServer::init(ws_stream).await;
                if let Err(e) = MessageManager::instance().process_messages().await {
                    println!("❌ 消息处理异常: {}", e);
                }
                AppServer::dispose().await;
                println!("❌ 已断开连接");
            }
            Err(e) => {
                println!("❌ 连接异常: {} - {}", addr, e);
            }
        }
    }

    async fn init(ws_stream: WsStream) {
        MessageManager::instance().init(ws_stream).await;
        MessageHandler::<Event>::instance()
            .set_handler(on_event)
            .await;
        MessageHandler::<Stream>::instance()
            .set_handler(on_stream)
            .await;

        let rpc = RPC::instance();
        rpc.add_command("get_version", get_version).await;

        let test = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = test().await;
        });
        TaskManager::instance().add("test", test).await;
    }

    async fn dispose() {
        MessageManager::instance().dispose().await;
        TaskManager::instance().dispose("test").await;
    }
}

async fn get_version(_: Request) -> Result<Response, AppError> {
    let data = json!(VERSION.to_string());
    Ok(Response::from_data(data))
}

async fn on_stream(stream: Stream) -> Result<(), AppError> {
    let Stream { tag, bytes, .. } = stream;
    match tag.as_str() {
        "record" => {
            NodeManager::instance()
                .call_fn::<(), _, _>(
                    "on_input_data",
                    move |cx| JsUint8Array::from_slice(cx, &bytes).unwrap().upcast(),
                    |_, _| Ok(()),
                )
                .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn on_event(event: Event) -> Result<(), AppError> {
    let event_json = serde_json::to_string(&event)?;
    NodeManager::instance()
        .call_fn::<(), _, _>(
            "on_event",
            move |cx| cx.string(&event_json).upcast(),
            |_, _| Ok(()),
        )
        .await?;
    Ok(())
}
