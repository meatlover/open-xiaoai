use std::sync::LazyLock;
use tokio::sync::Mutex;

use crate::utils::shell::run_shell;

pub struct VolumeControl {
    volume: Mutex<i32>,
    muted_volume: Mutex<Option<i32>>,
}

static INSTANCE: LazyLock<VolumeControl> = LazyLock::new(VolumeControl::new);

const STEP: i32 = 25;
const MIN: i32 = 0;
const MAX: i32 = 255;

impl VolumeControl {
    fn new() -> Self {
        Self {
            volume: Mutex::new(128),
            muted_volume: Mutex::new(None),
        }
    }

    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    pub async fn init(&self) {
        let res = run_shell("amixer -c 0 get mysoftvol").await;
        if let Ok(res) = res {
            // Parse value from output like "Mono: Playback 128 [50%]"
            if let Some(val) = res.stdout.split_whitespace()
                .filter_map(|s| s.parse::<i32>().ok())
                .find(|&v| v >= MIN && v <= MAX)
            {
                *self.volume.lock().await = val;
                println!("🔊 Initial volume: {}", val);
            }
        }
    }

    pub async fn volume_up(&self) -> i32 {
        let mut vol = self.volume.lock().await;
        *vol = (*vol + STEP).min(MAX);
        let value = *vol;
        drop(vol);
        self.set_volume(value).await;
        value
    }

    pub async fn volume_down(&self) -> i32 {
        let mut vol = self.volume.lock().await;
        *vol = (*vol - STEP).max(MIN);
        let value = *vol;
        drop(vol);
        self.set_volume(value).await;
        value
    }

    pub async fn toggle_mute(&self) -> bool {
        let mut muted = self.muted_volume.lock().await;
        if let Some(saved) = muted.take() {
            // Unmute: restore saved volume
            drop(muted);
            *self.volume.lock().await = saved;
            self.set_volume(saved).await;
            println!("🔊 Unmuted, restored volume: {}", saved);
            false
        } else {
            // Mute: save current volume, set to 0
            let vol = *self.volume.lock().await;
            *muted = Some(vol);
            drop(muted);
            self.set_volume(0).await;
            println!("🔇 Muted, saved volume: {}", vol);
            true
        }
    }

    async fn set_volume(&self, value: i32) {
        let cmd = format!("amixer -c 0 set mysoftvol {}", value);
        let _ = run_shell(&cmd).await;
    }
}
