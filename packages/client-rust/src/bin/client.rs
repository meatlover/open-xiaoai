use open_xiaoai::services::ai_brain::{extract_asr_text, new_session_id, send_user_input};
use open_xiaoai::services::audio::config::AudioConfig;
use open_xiaoai::services::monitor::file::FileMonitorEvent;
use open_xiaoai::services::monitor::kws::KwsMonitor;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;

use open_xiaoai::base::AppError;
use open_xiaoai::base::VERSION;
use open_xiaoai::services::audio::play::AudioPlayer;
use open_xiaoai::services::audio::record::AudioRecorder;
use open_xiaoai::services::connect::data::{Event, Request, Response, Stream};
use open_xiaoai::services::connect::handler::MessageHandler;
use open_xiaoai::services::connect::message::{MessageManager, WsStream};
use open_xiaoai::services::connect::rpc::RPC;
use open_xiaoai::services::led;
use open_xiaoai::services::monitor::button::{ButtonEvent, ButtonMonitor};
use open_xiaoai::services::monitor::instruction::InstructionMonitor;
use open_xiaoai::services::volume::VolumeControl;

/// Check if the query should be handled by factory AI (simple daily questions).
fn is_factory_ai_query(text: &str) -> bool {
    const PATTERNS: &[&str] = &[
        // Weather
        "天气", "气温", "下雨", "下雪", "温度", "多少度", "冷不冷", "热不热",
        // Date/Time
        "几点", "几号", "星期几", "日期", "什么时候",
        // Timers/Alarms
        "闹钟", "定时", "倒计时", "提醒我",
    ];
    PATTERNS.iter().any(|p| text.contains(p))
}

struct AppClient {
    kws_monitor: KwsMonitor,
    instruction_monitor: InstructionMonitor,
    button_monitor: ButtonMonitor,
    session_id: Arc<Mutex<String>>,
}

impl AppClient {
    pub fn new() -> Self {
        Self {
            kws_monitor: KwsMonitor::new(),
            instruction_monitor: InstructionMonitor::new(),
            button_monitor: ButtonMonitor::new(),
            session_id: Arc::new(Mutex::new(new_session_id())),
        }
    }

    pub async fn connect(&self, url: &str) -> Result<WsStream, AppError> {
        let (ws_stream, _) = connect_async(url).await?;
        Ok(WsStream::Client(ws_stream))
    }

    pub async fn run(&mut self) {
        let url = std::env::args().nth(1).expect("❌ 请输入服务器地址");
        println!("✅ 已启动");
        loop {
            let Ok(ws_stream) = self.connect(&url).await else {
                sleep(Duration::from_secs(1)).await;
                continue;
            };
            println!("✅ 已连接: {:?}", url);
            self.init(ws_stream).await;
            if let Err(e) = MessageManager::instance().process_messages().await {
                eprintln!("❌ 消息处理异常: {}", e);
            }
            self.dispose().await;
            eprintln!("❌ 已断开连接");
        }
    }

    async fn init(&mut self, ws_stream: WsStream) {
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

        // Stop touchpad daemon — we handle buttons directly via /dev/input/event0
        let _ = open_xiaoai::utils::shell::run_shell(
            "/etc/init.d/touchpad stop"
        ).await;
        println!("🔇 Stopped touchpad daemon");

        // Initialize volume control
        VolumeControl::instance().init().await;

        // Start button monitor
        self.button_monitor
            .start(|event| async move {
                let vc = VolumeControl::instance();
                match event {
                    ButtonEvent::VolumeUp => {
                        // Do nothing if muted
                        if let Some(vol) = vc.volume_up().await {
                            println!("🔊 Volume up: {}", vol);
                            vc.play_volume_sound().await;
                        }
                    }
                    ButtonEvent::VolumeDown => {
                        let vol = vc.volume_down().await;
                        println!("🔉 Volume down: {}", vol);
                        vc.play_volume_sound().await;
                    }
                    ButtonEvent::VolumeDownLong => {
                        let vol = vc.set_to_minimum().await;
                        println!("🔉 Volume minimum: {}", vol);
                        vc.play_volume_sound().await;
                    }
                    ButtonEvent::Mute => {
                        let muted = vc.toggle_mute().await;
                        if muted {
                            // Muted: play beep confirmation, then orange LED
                            vc.play_volume_sound().await;
                            led::show(9).await;
                        } else {
                            // Unmuted: turn off mute LED, play beep
                            led::shut(9).await;
                            vc.play_volume_sound().await;
                        }
                    }
                    ButtonEvent::PlayPause => {
                        println!("⏹️ Stop playback");
                        let _ = AudioPlayer::instance().stop().await;
                    }
                }
                Ok(())
            })
            .await;

        let session_id_clone = Arc::clone(&self.session_id);
        let last_asr = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10)));
        self.instruction_monitor
            .start(move |event| {
                let session_id_clone = Arc::clone(&session_id_clone);
                let last_asr = Arc::clone(&last_asr);
                async move {
                    // Log firmware TTS attempts for debugging
                    if let FileMonitorEvent::NewLine(ref line) = event {
                        if line.contains("\"namespace\":\"SpeechSynthesizer\"") {
                            eprintln!("📡 Firmware TTS: {}", line);
                        }
                    }

                    // Send original instruction event for backward compatibility
                    MessageManager::instance()
                        .send_event("instruction", Some(json!(event)))
                        .await?;

                    // Check if this is an ASR final result and send to AI-Brain
                    if let Some(text) = extract_asr_text(&json!(event)) {
                        // Debounce: skip duplicate ASR finals within 2s
                        let mut last = last_asr.lock().await;
                        if last.elapsed() < Duration::from_secs(2) {
                            println!("⏭️ Skipping duplicate ASR final: {}", text);
                            return Ok(());
                        }
                        *last = Instant::now();
                        drop(last);

                        let session_id = session_id_clone.lock().await.clone();
                        println!("🔥 ASR final result: {}", text);
                        // Play "prompt received" confirmation sound
                        let _ = open_xiaoai::utils::shell::run_shell(
                            "aplay -D notify /data/open-xiaoai/sounds/notice.wav 2>/dev/null &"
                        ).await;

                        if is_factory_ai_query(&text) {
                            // Simple query → open factory gate, let factory AI answer
                            println!("🏭 Factory AI query: {}", text);
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 set factorygatevol 255 2>/dev/null"
                            ).await;
                        } else {
                            // Complex query → close factory gate, send to brain
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 set factorygatevol 0 2>/dev/null"
                            ).await;
                            if let Err(e) = send_user_input(session_id, text).await {
                                eprintln!("❌ Failed to send user_input: {}", e);
                            }
                        }
                    }

                    Ok(())
                }
            })
            .await;

        self.kws_monitor
            .start(|event| async move {
                MessageManager::instance()
                    .send_event("kws", Some(json!(event)))
                    .await
            })
            .await;
    }

    async fn dispose(&mut self) {
        MessageManager::instance().dispose().await;
        let _ = AudioPlayer::instance().stop().await;
        let _ = AudioRecorder::instance().stop_recording().await;
        self.instruction_monitor.stop().await;
        self.kws_monitor.stop().await;
        self.button_monitor.stop().await;
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
    if tag.as_str() == "play" {
        let _ = AudioPlayer::instance().play(bytes).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    AppClient::new().run().await;
}
