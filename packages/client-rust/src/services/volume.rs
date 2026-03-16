use std::sync::LazyLock;
use tokio::sync::Mutex;

use crate::utils::shell::run_shell;

pub struct VolumeControl {
    volume: Mutex<i32>,
    muted_volume: Mutex<Option<i32>>,
}

static INSTANCE: LazyLock<VolumeControl> = LazyLock::new(VolumeControl::new);

/// 10% of 255 ≈ 26
const STEP: i32 = 26;
const MIN: i32 = 0;
/// Minimum volume that is still audible (~5%)
const MIN_AUDIBLE: i32 = 13;
const MAX: i32 = 255;

const VOLUME_SOUND: &str = "/data/open-xiaoai/sounds/mic_on.wav";

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

    pub async fn is_muted(&self) -> bool {
        self.muted_volume.lock().await.is_some()
    }

    /// Increase volume by 10%. Returns None if muted (no-op).
    pub async fn volume_up(&self) -> Option<i32> {
        if self.is_muted().await {
            return None;
        }
        let mut vol = self.volume.lock().await;
        *vol = (*vol + STEP).min(MAX);
        let value = *vol;
        drop(vol);
        self.set_volume(value).await;
        Some(value)
    }

    /// Decrease volume by 10%, clamped to minimum audible level.
    pub async fn volume_down(&self) -> i32 {
        let mut vol = self.volume.lock().await;
        *vol = (*vol - STEP).max(MIN_AUDIBLE);
        let value = *vol;
        drop(vol);
        self.set_volume(value).await;
        value
    }

    /// Set volume to minimum audible level.
    pub async fn set_to_minimum(&self) -> i32 {
        let mut vol = self.volume.lock().await;
        *vol = MIN_AUDIBLE;
        drop(vol);
        self.set_volume(MIN_AUDIBLE).await;
        MIN_AUDIBLE
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

    /// Play the volume feedback beep (fire-and-forget).
    pub async fn play_volume_sound(&self) {
        let cmd = format!("aplay {} 2>/dev/null &", VOLUME_SOUND);
        let _ = run_shell(&cmd).await;
    }

    async fn set_volume(&self, value: i32) {
        let cmd = format!("amixer -c 0 set mysoftvol {}", value);
        let _ = run_shell(&cmd).await;
    }
}
