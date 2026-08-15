use open_xiaoai::services::ai_brain::{extract_asr_text, new_session_id, send_user_input};
use open_xiaoai::services::audio::config::AudioConfig;
use open_xiaoai::services::monitor::file::{FileMonitor, FileMonitorEvent};
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

/// Saved Master Playback Volume (numid=1) value while muted for a
/// kws-triggered cycle, so start_play can force-restore it before our own
/// answer plays. Master Volume is hardware-level and mutes ALL sources
/// uniformly, including our own aplay-based playback — unlike mysoftvol,
/// which only affects factory's own gain stage and was confirmed live
/// (fix round 14) not to gate factory's actual TTS output at all.
static MASTER_VOL_SAVED: std::sync::LazyLock<Mutex<Option<i32>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

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
    /// Set true when the KWS handler pauses mphelper to suppress factory's
    /// TTS. Cleared by the instruction_monitor handler on the real
    /// Dialog.Finish signal (not a guessed timeout — factory's cloud
    /// round-trip latency is highly variable).
    mphelper_paused: Arc<Mutex<bool>>,
    /// Set true when the syslog watcher sees the real answer's
    /// speech_synthesizer marker for the current paused cycle. Gates the
    /// syslog unpause so it can't fire on the spurious Dialog::onTtsFinish!
    /// that our own wake-time interrupt (mediaplayer stop) triggers as a
    /// side effect, ~5s before the real one (confirmed live via syslog).
    answer_synthesis_started: Arc<Mutex<bool>>,
    /// Watches /var/log/messages for the real Dialog::onTtsFinish! line —
    /// the actual acoustic-completion signal, distinct from and later than
    /// Dialog.Finish in instruction.log (confirmed live: ~306ms later).
    syslog_monitor: FileMonitor,
}

impl AppClient {
    pub fn new() -> Self {
        Self {
            kws_monitor: KwsMonitor::new(),
            instruction_monitor: InstructionMonitor::new(),
            button_monitor: ButtonMonitor::new(),
            session_id: Arc::new(Mutex::new(new_session_id())),
            kws_triggered: Arc::new(Mutex::new(false)),
            mphelper_paused: Arc::new(Mutex::new(false)),
            answer_synthesis_started: Arc::new(Mutex::new(false)),
            syslog_monitor: FileMonitor::new(),
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
                        // Consume and clear the kws-triggered flag unconditionally,
                        // for every ASR final this handler observes — including ones
                        // the debounce check below discards as duplicates. Clearing
                        // it here (before the debounce early-return) prevents a
                        // debounced kws-triggered result from leaving a stale `true`
                        // that would misroute a later, unrelated real 小爱同学
                        // utterance to the brain.
                        let from_kws = {
                            let mut flag = kws_triggered_clone.lock().await;
                            let was_set = *flag;
                            *flag = false;
                            was_set
                        };

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

        let mphelper_paused_syslog_clone = Arc::clone(&self.mphelper_paused);
        let answer_synthesis_started_syslog_clone = Arc::clone(&self.answer_synthesis_started);
        self.syslog_monitor
            .start("/var/log/messages", move |event| {
                let mphelper_paused_syslog_clone = Arc::clone(&mphelper_paused_syslog_clone);
                let answer_synthesis_started_syslog_clone =
                    Arc::clone(&answer_synthesis_started_syslog_clone);
                async move {
                    if let FileMonitorEvent::NewLine(ref line) = event {
                        // Marks the moment the real answer's TTS content has
                        // been generated — logged exactly once per dialog
                        // cycle, always after the spurious cancel-triggered
                        // Dialog::onTtsFinish! and always before the real
                        // one (confirmed live across 6 dialog cycles).
                        if line.contains("speech_synthesizer.dialog_id=") {
                            *answer_synthesis_started_syslog_clone.lock().await = true;
                        }

                        // The real acoustic-completion signal — factory's
                        // TTS audio has actually finished playing. Only
                        // honored once the marker above has been seen for
                        // this cycle, since our own wake-time mediaplayer
                        // stop triggers a spurious Dialog::onTtsFinish!
                        // within ~30ms of every wake, well before any real
                        // answer exists (confirmed live via syslog).
                        if line.contains("Dialog::onTtsFinish!") {
                            let started = *answer_synthesis_started_syslog_clone.lock().await;
                            if started {
                                let mut paused = mphelper_paused_syslog_clone.lock().await;
                                if *paused {
                                    *paused = false;
                                    drop(paused);
                                    *answer_synthesis_started_syslog_clone.lock().await = false;
                                    let _ = open_xiaoai::utils::shell::run_shell("mphelper play").await;
                                    // Restore factory's ALSA gain to the
                                    // current volume level (mirrors
                                    // VolumeControl::set_volume's existing
                                    // notifyvol/mysoftvol split) — undoes
                                    // Step 1's mute for the next real 小爱
                                    // 同学 wake.
                                    let vol = VolumeControl::instance().get_volume().await;
                                    let _ = open_xiaoai::utils::shell::run_shell(&format!(
                                        "amixer -c 0 set mysoftvol {} 2>/dev/null",
                                        vol
                                    )).await;
                                    // Restore Master Volume (fix round 15).
                                    // take() clears MASTER_VOL_SAVED so
                                    // start_play's own safety-restore
                                    // becomes a no-op if it runs after this.
                                    if let Some(mv) = MASTER_VOL_SAVED.lock().await.take() {
                                        let _ = open_xiaoai::utils::shell::run_shell(&format!(
                                            "amixer -c 0 cset numid=1 {},{} 2>/dev/null",
                                            mv, mv
                                        )).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                }
            })
            .await;

        let kws_triggered_clone = Arc::clone(&self.kws_triggered);
        let mphelper_paused_clone = Arc::clone(&self.mphelper_paused);
        let answer_synthesis_started_clone = Arc::clone(&self.answer_synthesis_started);
        self.kws_monitor
            .start(move |event| {
                let kws_triggered_clone = Arc::clone(&kws_triggered_clone);
                let mphelper_paused_clone = Arc::clone(&mphelper_paused_clone);
                let answer_synthesis_started_clone = Arc::clone(&answer_synthesis_started_clone);
                async move {
                    // Forward the raw event for observability (existing behavior).
                    MessageManager::instance()
                        .send_event("kws", Some(json!(event)))
                        .await?;

                    if let KwsMonitorEvent::Keyword(ref keyword) = event {
                        if is_custom_wake_word(keyword) {
                            // Pause factory's playback path immediately, before anything else in
                            // this branch — every prior step (AudioPlayer stop, the WS interrupt
                            // send, the factory-mediaplayer-stop call) adds latency before this
                            // otherwise would run, giving factory's own audio a small head start.
                            // Live testing showed this reduced (but did not eliminate) a residual
                            // bleed-through window even with pause already running before the
                            // pnshelper trigger fires — moving it first shaves off whatever
                            // cumulative delay those other steps introduced.
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "mphelper pause"
                            ).await;

                            // Hedge against mediaplayer's own internal
                            // per-track reset, which silently undoes both a
                            // one-shot pause (confirmed live via syslog:
                            // PauseDummy(1) immediately followed by
                            // PauseDummy(0), ~15ms apart) and a one-shot
                            // mysoftvol mute (confirmed live: mediaplayer's
                            // own AUDIO_VOLUME_SET writes mysoftvol's
                            // resting value back ~190ms after our mute,
                            // fix round 12's finding) — both happen well
                            // after any fixed-duration burst would have
                            // ended. Re-assert both every 100ms until the
                            // real completion signal (fix round 10's
                            // syslog watcher) flips mphelper_paused false.
                            let hammer_paused_clone = Arc::clone(&mphelper_paused_clone);
                            tokio::spawn(async move {
                                // Keep re-asserting the pause for as long as
                                // mphelper_paused stays true — mediaplayer
                                // performs its own internal pause/unpause
                                // dance when it opens factory's answer as a
                                // new track (confirmed live via syslog:
                                // PauseDummy(1) immediately followed by
                                // PauseDummy(0), ~15ms apart), which silently
                                // undoes a one-shot pause with nobody to
                                // re-assert it. The 300-iteration cap (30s)
                                // is a runaway safety net only, not the
                                // intended stopping condition — the real
                                // stop condition is the flag going false via
                                // the syslog watcher's real-completion
                                // unpause.
                                for _ in 0..300 {
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    if !*hammer_paused_clone.lock().await {
                                        break;
                                    }
                                    let _ = open_xiaoai::utils::shell::run_shell(
                                        "mphelper pause; amixer -c 0 set mysoftvol 0 2>/dev/null"
                                    ).await;
                                    // Only re-mute Master Volume while a
                                    // restore is still owed. Without this
                                    // check, this loop would re-mute the
                                    // hardware on its next 100ms tick even
                                    // after start_play's safety backstop
                                    // (Step 5) has already restored it to
                                    // play our own answer — silencing that
                                    // answer mid-playback. mphelper_paused
                                    // alone can't gate this: it's only
                                    // cleared by factory's completion
                                    // signal, unrelated to when our own
                                    // answer actually arrives.
                                    if MASTER_VOL_SAVED.lock().await.is_some() {
                                        let _ = open_xiaoai::utils::shell::run_shell(
                                            "amixer -c 0 cset numid=1 0,0 2>/dev/null"
                                        ).await;
                                    }
                                }
                            });

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

                            // Always route to brain: mute factory's ALSA
                            // playback path so its own spoken response
                            // never plays. mysoftvol (not factorygatevol,
                            // which doesn't exist as a real ALSA control on
                            // this device — confirmed live, every prior
                            // call to it silently failed) is the real gate:
                            // /etc/asound.conf routes pcm.!default (factory)
                            // through a softvol control named "mysoftvol"
                            // before the shared dmixer, while our own
                            // aplay -D notify path uses an independent
                            // softvol ("notifyvol") — muting mysoftvol
                            // can't affect our own playback. Also, unlike
                            // mphelper pause, this is a persistent ALSA
                            // mixer value, not app-level playback state, so
                            // it isn't reset by mediaplayer's internal
                            // pause/unpause dance when it opens a new track
                            // (fix round 11's finding).
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 set mysoftvol 0 2>/dev/null"
                            ).await;

                            // Acknowledgment sound — plays immediately, using
                            // our own aplay/notify path, unaffected by the
                            // mphelper pause above.
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "aplay -D notify /data/open-xiaoai/sounds/notice.wav 2>/dev/null &"
                            ).await;

                            // Mute the real hardware gain (fix round 15).
                            // mysoftvol above does not actually gate
                            // factory's TTS output (confirmed live, fix
                            // round 14: held at 0 for the full answer
                            // window, factory still audible) — Master
                            // Playback Volume is downstream of every
                            // software mixing path and does gate it
                            // (confirmed live: silenced factory AND our own
                            // ack beep). Placed after the beep call above
                            // so the beep is already queued before this
                            // mutes everything. Read-and-restore, not
                            // hardcoded — this is a hardware-calibrated
                            // baseline, not a user-adjustable level.
                            let master_vol_read = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 cget numid=1"
                            ).await;
                            // Extracted into an owned Option<i32> (Send)
                            // before the lock().await below — the raw
                            // Result<_, Box<dyn Error>> from run_shell is
                            // not Send, and holding it across an await
                            // point here breaks the Send bound required by
                            // KwsMonitor::start's callback future.
                            let master_vol_parsed = master_vol_read.ok().and_then(|res| {
                                res.stdout
                                    .lines()
                                    .find(|l| l.trim_start().starts_with(": values="))
                                    .and_then(|l| l.trim_start().strip_prefix(": values="))
                                    .and_then(|v| v.split(',').next())
                                    .and_then(|v| v.trim().parse::<i32>().ok())
                            });
                            if let Some(v) = master_vol_parsed {
                                *MASTER_VOL_SAVED.lock().await = Some(v);
                            }
                            let _ = open_xiaoai::utils::shell::run_shell(
                                "amixer -c 0 cset numid=1 0,0 2>/dev/null"
                            ).await;

                            // Mark the next ASR result as kws-originated
                            // (Task 4 consumes and clears this).
                            *kws_triggered_clone.lock().await = true;
                            *mphelper_paused_clone.lock().await = true;
                            *answer_synthesis_started_clone.lock().await = false;

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
        self.syslog_monitor.stop().await;
    }
}

async fn get_version(_: Request) -> Result<Response, AppError> {
    let data = json!(VERSION.to_string());
    Ok(Response::from_data(data))
}

async fn start_play(request: Request) -> Result<Response, AppError> {
    // Safety backstop (fix round 15): Master Playback Volume mutes ALL
    // sources uniformly, including our own playback. If a kws-triggered
    // cycle muted it and the real-completion signal hasn't restored it
    // yet, force-restore it now — our own answer must never be silenced
    // by a timing race between factory's completion signal and our own
    // answer's arrival. take() is idempotent with the syslog watcher's
    // own restore: whichever runs first wins, the other is a no-op.
    if let Some(mv) = MASTER_VOL_SAVED.lock().await.take() {
        let _ = open_xiaoai::utils::shell::run_shell(&format!(
            "amixer -c 0 cset numid=1 {},{} 2>/dev/null",
            mv, mv
        )).await;
    }
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
