use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::base::AppError;

use super::config::{AudioConfig, AUDIO_CONFIG};

pub struct AudioPlayer {
    aplay_thread: Arc<Mutex<Option<Child>>>,
    write_thread: Arc<Mutex<Option<ChildStdin>>>,
    sender: Arc<Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
    player_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

static INSTANCE: LazyLock<AudioPlayer> = LazyLock::new(AudioPlayer::new);

impl AudioPlayer {
    fn new() -> Self {
        Self {
            aplay_thread: Arc::new(Mutex::new(None)),
            write_thread: Arc::new(Mutex::new(None)),
            sender: Arc::new(Mutex::new(None)),
            player_task: Arc::new(Mutex::new(None)),
        }
    }

    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    pub async fn stop(&self) -> Result<(), AppError> {
        eprintln!("🛑 AudioPlayer::stop() called");

        // 1. Stop accepting new chunks
        if let Some(sender) = self.sender.lock().await.take() {
            drop(sender);
        }

        // 2. Wait for writer task to flush remaining chunks
        if let Some(task) = self.player_task.lock().await.take() {
            let _ = task.await;
        }

        // 3. Close stdin so aplay sees EOF and plays remaining buffer
        if let Some(mut stdin) = self.write_thread.lock().await.take() {
            let _ = stdin.shutdown().await;
            drop(stdin);
        }

        // 4. Wait for aplay to finish (with timeout), don't kill it
        if let Some(mut child) = self.aplay_thread.lock().await.take() {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                child.wait(),
            ).await;
            let _ = child.kill().await; // kill only if timeout
        }

        Ok(())
    }

    pub async fn start(&self, config: Option<AudioConfig>) -> Result<(), AppError> {
        let is_started = self.sender.lock().await.is_some();
        if is_started {
            self.stop().await?;
        }

        let config = config.unwrap_or_else(|| (*AUDIO_CONFIG).clone());
        eprintln!("🎵 AudioPlayer::start() config: rate={}, bits={}, ch={}", config.sample_rate, config.bits_per_sample, config.channels);

        let mut aplay_thread = Command::new("aplay")
            .args([
                "-t",
                "raw",
                "-f",
                &format!("S{}_LE", config.bits_per_sample),
                "-r",
                &config.sample_rate.to_string(),
                "-c",
                &config.channels.to_string(),
                "-",
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Capture aplay stderr for debugging
        if let Some(stderr) = aplay_thread.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let reader = tokio::io::BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    eprintln!("🔈 aplay: {}", line);
                }
            });
        }

        let stdin = aplay_thread.stdin.take().unwrap();
        self.aplay_thread.lock().await.replace(aplay_thread);
        self.write_thread.lock().await.replace(stdin);

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

        let write_thread_clone = self.write_thread.clone();
        let player_task = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                let mut write_guard = write_thread_clone.lock().await;
                if let Some(write_thread) = write_guard.as_mut() {
                    if let Err(e) = write_thread.write_all(&bytes).await {
                        eprintln!("⚠️ aplay write error: {}", e);
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        self.player_task.lock().await.replace(player_task);
        self.sender.lock().await.replace(tx);

        Ok(())
    }

    pub async fn play(&self, bytes: Vec<u8>) -> Result<(), AppError> {
        let sender_guard = self.sender.lock().await;
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(bytes).await?;
        } else {
            eprintln!("⚠️ AudioPlayer::play() called but no sender (start not called?)");
        }

        Ok(())
    }
}
