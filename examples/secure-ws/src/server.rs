use open_xiaoai::base::AppError;
use open_xiaoai::services::connect::data::{Event, Stream};
use open_xiaoai::services::connect::handler::MessageHandler;
use open_xiaoai::services::connect::message::{MessageManager, WsStream};
use open_xiaoai::services::connect::rpc::RPC;
use open_xiaoai::utils::task::TaskManager;
use serde_json::json;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_async;

use crate::tls::{build_rustls, get_tls, set_tls, TlsConfig};

pub struct AppServer;

impl AppServer {
    pub async fn run() {
        let ws_addr: SocketAddr = "0.0.0.0:4399".parse().unwrap();
        let ws_task: JoinHandle<()> = tokio::spawn(async move {
            let listener = TcpListener::bind(ws_addr).await.expect("bind ws");
            println!("✅ ws server: {} (TLS configurable)", ws_addr);
            loop {
                let (stream, addr) = listener.accept().await.expect("accept ws");
                tokio::spawn(Self::handle_ws(stream, addr));
            }
        });
        TaskManager::instance().add("secure-ws", ws_task).await;
    }

    async fn handle_ws(stream: TcpStream, addr: SocketAddr) {
        let tls = get_tls();
        if let Some(cfg) = build_rustls(&tls).ok().flatten() {
            // TLS / mTLS
            let acceptor = TlsAcceptor::from(cfg);
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    let ws_stream = accept_async(tls_stream).await;
                    match ws_stream {
                        Ok(ws) => {
                            println!("✅ wss connection: {}", addr);
                            Self::serve_ws(WsStream::Server(ws)).await;
                        }
                        Err(e) => eprintln!("wss handshake error: {e}"),
                    }
                }
                Err(e) => eprintln!("tls accept error: {e}"),
            }
        } else {
            // Plain WS
            match accept_async(stream).await {
                Ok(ws) => {
                    println!("✅ ws connection: {}", addr);
                    Self::serve_ws(WsStream::Server(ws)).await;
                }
                Err(e) => eprintln!("ws handshake error: {e}"),
            }
        }
    }

    async fn serve_ws(ws: WsStream) {
        Self::init(ws).await;
        if let Err(e) = MessageManager::instance().process_messages().await {
            eprintln!("❌ message error: {e}");
        }
        Self::dispose().await;
    }

    async fn init(ws_stream: WsStream) {
        MessageManager::instance().init(ws_stream).await;
        MessageHandler::<Event>::instance().set_handler(on_event).await;
        MessageHandler::<Stream>::instance().set_handler(on_stream).await;
        let rpc = RPC::instance();
        // Keep on-device TTS/STT; expose minimal RPCs as needed.
        rpc.add_command("get_version", get_version).await;
    }

    async fn dispose() {
        MessageManager::instance().dispose().await;
    }
}

async fn get_version(_: open_xiaoai::services::connect::data::Request) -> Result<open_xiaoai::services::connect::data::Response, AppError> {
    Ok(open_xiaoai::services::connect::data::Response::from_data(json!(open_xiaoai::base::VERSION.to_string())))
}

async fn on_stream(stream: Stream) -> Result<(), AppError> {
    match stream.tag.as_str() {
        "record" => {
            // Optional: forward to ASR provider; default is device-side ASR so we ignore here.
        }
        _ => {}
    }
    Ok(())
}

async fn on_event(_event: Event) -> Result<(), AppError> {
    // Default: device generates instruction/playing/kws; handle if needed.
    Ok(())
}
