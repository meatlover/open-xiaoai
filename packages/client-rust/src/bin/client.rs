use open_xiaoai::services::ai_brain::{extract_asr_text, new_session_id, send_user_input};
use open_xiaoai::services::audio::config::AudioConfig;
use open_xiaoai::services::monitor::file::FileMonitorEvent;
use open_xiaoai::services::monitor::kws::{KwsMonitor, KwsMonitorEvent};
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

/// Custom wake words that bypass factory ASR intent classification entirely
/// and always route to our own AI brain.
const CUSTOM_WAKE_WORDS: &[&str] = &["小虎同学", "小虎儿同学"];

fn is_custom_wake_word(keyword: &str) -> bool {
    CUSTOM_WAKE_WORDS.contains(&keyword)
}

struct AppClient {
    kws_monitor: KwsMonitor,
    instruction_monitor: InstructionMonitor,
    button_monitor: ButtonMonitor,
    session_id: Arc<Mutex<String>>,
    /// Set true when a custom wake word (小虎同学/小虎儿同学) fires a
    /// synthetic ASR trigger. The next ASR final result consumes and
    /// clears this flag to skip factory-intent routing (Task 4).
    kws_triggered: Arc<Mutex<bool>>,
}

impl AppClient {
    pub fn new() -> Self {
        Self {
            kws_monitor: KwsMonitor::new(),
            instruction_monitor: InstructionMonitor::new(),
            button_monitor: ButtonMonitor::new(),
            session_id: Arc::new(Mutex::new(new_session_id())),
            kws_triggered: Arc::new(Mutex::new(false)),
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
                            // Unmuted: turn off mute LEDs (9=our orange, 7=firmware purple)
                            led::shut(9).await;
                            led::shut(7).await;
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
        let kws_triggered_clone = Arc::clone(&self.kws_triggered);
        self.instruction_monitor
            .start(move |event| {
                let session_id_clone = Arc::clone(&session_id_clone);
                let last_asr = Arc::clone(&last_asr);
                let kws_triggered_clone = Arc::clone(&kws_triggered_clone);
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

                        // Stop any current brain response (kills aplay mid-stream)
                        let _ = AudioPlayer::instance().stop().await;
                        // Tell server to cancel current LLM generation
                        let interrupt_msg = json!({"type": "interrupt"});
                        let _ = MessageManager::instance()
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                interrupt_msg.to_string().into(),
                            ))
                            .await;
                        // Stop factory AI playback
                        let _ = open_xiaoai::utils::shell::run_shell(
                            "ubus -t 1 call mediaplayer player_play_operation '{\"action\":\"stop\"}' 2>/dev/null"
                        ).await;

                        // Play "prompt received" confirmation sound
                        let _ = open_xiaoai::utils::shell::run_shell(
                            "aplay -D notify /data/open-xiaoai/sounds/notice.wav 2>/dev/null &"
                        ).await;

                        let from_kws = {
                            let mut flag = kws_triggered_clone.lock().await;
                            let was_set = *flag;
                            *flag = false;
                            was_set
                        };

                        if !from_kws && is_factory_ai_query(&text) {
                            // Real 小爱同学 wake, simple query → open factory
                            // gate, let factory AI answer.
                            println!("🏭 Factory AI query: {}", text);
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 set factorygatevol 255 2>/dev/null"
                            ).await;
                        } else {
                            // Real 小爱同学 wake with a complex query, OR any
                            // kws-triggered utterance → close factory gate,
                            // send to brain.
                            if from_kws {
                                println!("🐯 Custom wake word query: {}", text);
                            }
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

        let kws_triggered_clone = Arc::clone(&self.kws_triggered);
        self.kws_monitor
            .start(move |event| {
                let kws_triggered_clone = Arc::clone(&kws_triggered_clone);
                async move {
                    // Forward the raw event for observability (existing behavior).
                    MessageManager::instance()
                        .send_event("kws", Some(json!(event)))
                        .await?;

                    if let KwsMonitorEvent::Keyword(ref keyword) = event {
                        if is_custom_wake_word(keyword) {
                            println!("🐯 Custom wake word: {}", keyword);

                            // Interrupt any in-progress playback (brain or
                            // factory), mirroring the existing ASR-final
                            // interrupt behavior below.
                            let _ = AudioPlayer::instance().stop().await;
                            let interrupt_msg = json!({"type": "interrupt"});
                            let _ = MessageManager::instance()
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    interrupt_msg.to_string().into(),
                                ))
                                .await;
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "ubus -t 1 call mediaplayer player_play_operation '{\"action\":\"stop\"}' 2>/dev/null"
                            ).await;

                            // Always route to brain: close factory's audio
                            // gate so its own spoken response never plays.
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 set factorygatevol 0 2>/dev/null"
                            ).await;

                            // Acknowledgment sound.
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "aplay -D notify /data/open-xiaoai/sounds/notice.wav 2>/dev/null &"
                            ).await;

                            // Mark the next ASR result as kws-originated
                            // (Task 4 consumes and clears this).
                            *kws_triggered_clone.lock().await = true;

                            // Trigger a real factory ASR cycle without the
                            // real wake word.
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "ubus -t 1 call pnshelper event_notify '{\"src\":1,\"event\":0}' 2>/dev/null"
                            ).await;
                        }
                    }

                    Ok(())
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_custom_wake_word_matches_both_phrases() {
        assert!(is_custom_wake_word("小虎同学"));
        assert!(is_custom_wake_word("小虎儿同学"));
    }

    #[test]
    fn test_is_custom_wake_word_rejects_factory_wake_word() {
        assert!(!is_custom_wake_word("小爱同学"));
    }

    #[test]
    fn test_is_custom_wake_word_rejects_unrelated_text() {
        assert!(!is_custom_wake_word(""));
        assert!(!is_custom_wake_word("天气怎么样"));
        assert!(!is_custom_wake_word("小虎"));
    }
}

#[tokio::main]
async fn main() {
    AppClient::new().run().await;
}
