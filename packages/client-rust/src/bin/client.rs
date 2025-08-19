use open_xiaoai::services::audio::config::AudioConfig;
use open_xiaoai::services::monitor::kws::KwsMonitor;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue as WsHeaderValue;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use url::Url;

use open_xiaoai::base::AppError;
use open_xiaoai::base::VERSION;
use open_xiaoai::services::audio::play::AudioPlayer;
use open_xiaoai::services::audio::record::AudioRecorder;
use open_xiaoai::services::connect::data::{Event, Request, Response, Stream};
use open_xiaoai::services::connect::handler::MessageHandler;
use open_xiaoai::services::connect::message::{MessageManager, WsStream};
use open_xiaoai::services::connect::rpc::RPC;
use open_xiaoai::services::connect::tls_native::{TlsClientConfig, build_tls_connector};
use open_xiaoai::services::monitor::instruction::InstructionMonitor;
use open_xiaoai::services::monitor::playing::PlayingMonitor;

struct AppClient;

impl AppClient {
    pub async fn connect(url: &str) -> Result<WsStream, AppError> {
        let parsed_url = Url::parse(url)?;
        let tls_config = TlsClientConfig::from_env();
        
        let ws_stream = if parsed_url.scheme() == "wss" {
            // WSS connection with native-tls; prefer our custom connector if enabled
            if let Some(connector) = build_tls_connector(&tls_config)? {
                println!("🔐 Connecting with custom TLS to: {}", url);
                println!("🔍 TLS connector type: native-tls");
                
                // Extract hostname for SNI debugging
                let hostname = parsed_url.host_str().unwrap_or("unknown");
                println!("🔍 Target hostname for SNI: {}", hostname);
                
                let ws_config = WebSocketConfig::default();

                // Allow Cloudflare Access token or custom header if provided
                let mut req = url.into_client_request()?;
                
                // Debug: Show the host header that will be used for SNI
                if let Some(host_header) = req.headers().get("Host") {
                    println!("🔍 Host header for SNI: {:?}", host_header);
                } else {
                    println!("⚠️  No Host header found - SNI might not work correctly");
                }
                
                // Enhanced Service Token debugging
                println!("🔍 Checking Service Token environment variables...");
                match (std::env::var("CF_ACCESS_CLIENT_ID"), std::env::var("CF_ACCESS_CLIENT_SECRET")) {
                    (Ok(client_id), Ok(client_secret)) => {
                        println!("🔍 Service Token ID found: {}...{}", &client_id[..8], &client_id[client_id.len()-8..]);
                        println!("🔍 Service Token Secret found: {}...{}", &client_secret[..8], &client_secret[client_secret.len()-8..]);
                        
                        match WsHeaderValue::from_str(&client_id) {
                            Ok(v) => {
                                req.headers_mut().insert("CF-Access-Client-Id", v);
                                println!("✅ CF-Access-Client-Id header set");
                            }
                            Err(e) => println!("❌ Failed to set CF-Access-Client-Id header: {}", e),
                        }
                        
                        match WsHeaderValue::from_str(&client_secret) {
                            Ok(v) => {
                                req.headers_mut().insert("CF-Access-Client-Secret", v);
                                println!("✅ CF-Access-Client-Secret header set");
                            }
                            Err(e) => println!("❌ Failed to set CF-Access-Client-Secret header: {}", e),
                        }
                        
                        println!("🔍 Total headers in request: {}", req.headers().len());
                    }
                    (Err(_), Err(_)) => {
                        println!("⚠️  No Service Token credentials found - will attempt without authentication");
                    }
                    (Ok(_), Err(_)) => {
                        println!("⚠️  CF_ACCESS_CLIENT_ID found but CF_ACCESS_CLIENT_SECRET missing");
                    }
                    (Err(_), Ok(_)) => {
                        println!("⚠️  CF_ACCESS_CLIENT_SECRET found but CF_ACCESS_CLIENT_ID missing");
                    }
                }

                let (ws_stream, response) = tokio_tungstenite::connect_async_tls_with_config(
                    req,
                    Some(ws_config),
                    false,
                    Some(Connector::NativeTls(connector)),
                ).await?;
                println!("🔍 WebSocket handshake completed, response status: {:?}", response.status());
                ws_stream
            } else {
                println!("🔐 Connecting with default TLS to: {}", url);
                let (ws_stream, _) = connect_async(url).await?;
                ws_stream
            }
        } else {
            // Plain WS connection
            println!("🔓 Connecting without TLS to: {}", url);
            let (ws_stream, _) = connect_async(url).await?;
            ws_stream
        };
        
        Ok(WsStream::Client(ws_stream))
    }

    pub async fn run() {
        let url = std::env::args().nth(1).expect("❌ 请输入服务器地址");
        println!("✅ 已启动");
        loop {
                let ws_stream = match AppClient::connect(&url).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("❌ 连接失败: {}", e);
                    eprintln!("🔍 Error details: {:?}", e);
                    // Small jitter to avoid hot-looping
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            println!("✅ 已连接: {:?}", url);
            AppClient::init(ws_stream).await;
            if let Err(e) = MessageManager::instance().process_messages().await {
                eprintln!("❌ 消息处理异常: {}", e);
            }
            AppClient::dispose().await;
            eprintln!("❌ 已断开连接");
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
        rpc.add_command("run_shell", run_shell).await;
        rpc.add_command("start_play", start_play).await;
        rpc.add_command("stop_play", stop_play).await;
        rpc.add_command("start_recording", start_recording).await;
        rpc.add_command("stop_recording", stop_recording).await;

        InstructionMonitor::start(|event| async move {
            MessageManager::instance()
                .send_event("instruction", Some(json!(event)))
                .await
        })
        .await;

        PlayingMonitor::start(|event| async move {
            MessageManager::instance()
                .send_event("playing", Some(json!(event)))
                .await
        })
        .await;

        KwsMonitor::start(|event| async move {
            MessageManager::instance()
                .send_event("kws", Some(json!(event)))
                .await
        })
        .await;
    }

    async fn dispose() {
        MessageManager::instance().dispose().await;
        let _ = AudioPlayer::instance().stop().await;
        let _ = AudioRecorder::instance().stop_recording().await;
        InstructionMonitor::stop().await;
        PlayingMonitor::stop().await;
        KwsMonitor::stop().await;
    }
}

async fn get_version(_: Request) -> Result<Response, AppError> {
    let data = json!(VERSION.to_string());
    Ok(Response::from_data(data))
}

async fn start_play(request: Request) -> Result<Response, AppError> {
    let config = request
        .payload
        .and_then(|payload| serde_json::from_value::<AudioConfig>(payload).ok());
    AudioPlayer::instance().start(config).await?;
    Ok(Response::success())
}

async fn stop_play(_: Request) -> Result<Response, AppError> {
    AudioPlayer::instance().stop().await?;
    Ok(Response::success())
}

async fn start_recording(request: Request) -> Result<Response, AppError> {
    let config = request
        .payload
        .and_then(|payload| serde_json::from_value::<AudioConfig>(payload).ok());
    AudioRecorder::instance()
        .start_recording(
            |bytes| async {
                MessageManager::instance()
                    .send_stream("record", bytes, None)
                    .await
            },
            config,
        )
        .await?;
    Ok(Response::success())
}

async fn stop_recording(_: Request) -> Result<Response, AppError> {
    AudioRecorder::instance().stop_recording().await?;
    Ok(Response::success())
}

async fn run_shell(request: Request) -> Result<Response, AppError> {
    let script = match request.payload {
        Some(payload) => serde_json::from_value::<String>(payload)?,
        _ => return Err("empty command".into()),
    };
    let res = open_xiaoai::utils::shell::run_shell(script.as_str()).await?;
    Ok(Response::from_data(json!(res)))
}

async fn on_event(event: Event) -> Result<(), AppError> {
    println!("🔥 收到事件: {:?}", event);
    Ok(())
}

async fn on_stream(stream: Stream) -> Result<(), AppError> {
    let Stream { tag, bytes, .. } = stream;
    match tag.as_str() {
        "play" => {
            // 播放接收到的音频流
            let _ = AudioPlayer::instance().play(bytes).await;
        }
        _ => {}
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    AppClient::run().await;
}
