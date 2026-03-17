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

const VOLUME_SOUND: &str = "/data/open-xiaoai/sounds/volume.wav";

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
        // Default volume (~30%)
        const DEFAULT_VOL: i32 = 80;

        let mut vol = DEFAULT_VOL;
        let res = run_shell("amixer -c 0 get notifyvol").await;
        if let Ok(res) = res {
            // Parse value from output like "Front Left: 80 [31%]"
            if let Some(val) = res.stdout.split_whitespace()
                .filter_map(|s| s.parse::<i32>().ok())
                .find(|&v| v >= MIN && v <= MAX)
            {
                if val > 0 {
                    vol = val;
                }
            }
        }
        *self.volume.lock().await = vol;
        // Ensure notifyvol and mysoftvol are at the resolved level
        let _ = run_shell(&format!(
            "amixer -c 0 set notifyvol {} && amixer -c 0 set mysoftvol {}",
            vol, vol
        )).await;
        println!("🔊 Initial volume: {}", vol);
        // Close factory gate by default (brain mode). Create the control first
        // by playing a silent frame through the default PCM.
        let _ = run_shell("aplay -d 0 /dev/null 2>/dev/null; amixer -c 0 set factorygatevol 0 2>/dev/null").await;
    }

    pub async fn get_volume(&self) -> i32 {
        *self.volume.lock().await
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
            // Unmute: restore volume and re-enable microphone
            drop(muted);
            *self.volume.lock().await = saved;
            self.set_volume(saved).await;
            let _ = run_shell("/etc/init.d/pns mic_on").await;
            println!("🔊 Unmuted, restored volume: {}, mic on", saved);
            false
        } else {
            // Mute: save volume, silence output, disable microphone
            let vol = *self.volume.lock().await;
            *muted = Some(vol);
            drop(muted);
            self.set_volume(0).await;
            let _ = run_shell("/etc/init.d/pns mic_off").await;
            println!("🔇 Muted, saved volume: {}, mic off", vol);
            true
        }
    }

    /// Play the volume feedback beep (fire-and-forget).
    pub async fn play_volume_sound(&self) {
        let cmd = format!("aplay -D notify {} 2>/dev/null &", VOLUME_SOUND);
        let _ = run_shell(&cmd).await;
    }

    async fn set_volume(&self, value: i32) {
        // Set both notifyvol (our AI) and mysoftvol (factory AI).
        let cmd = format!(
            "amixer -c 0 set notifyvol {} && amixer -c 0 set mysoftvol {}",
            value, value
        );
        let _ = run_shell(&cmd).await;
    }
}
