use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Global state manager for tracking TTS and system state
pub struct StateManager {
    is_tts_playing: AtomicBool,
    current_tts_id: AtomicU64,
    last_tts_start: AtomicU64,
    interrupt_requested: AtomicBool,
}

impl StateManager {
    fn new() -> Self {
        Self {
            is_tts_playing: AtomicBool::new(false),
            current_tts_id: AtomicU64::new(0),
            last_tts_start: AtomicU64::new(0),
            interrupt_requested: AtomicBool::new(false),
        }
    }

    pub fn instance() -> &'static StateManager {
        static INSTANCE: std::sync::OnceLock<StateManager> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(StateManager::new)
    }

    /// Mark TTS as playing and return a unique ID for this TTS session
    pub fn start_tts(&self) -> u64 {
        let tts_id = chrono::Utc::now().timestamp_millis() as u64;
        self.current_tts_id.store(tts_id, Ordering::Relaxed);
        self.last_tts_start.store(tts_id, Ordering::Relaxed);
        self.is_tts_playing.store(true, Ordering::Relaxed);
        self.interrupt_requested.store(false, Ordering::Relaxed);
        tts_id
    }

    /// Mark TTS as stopped for the given ID
    pub fn stop_tts(&self, tts_id: u64) {
        let current_id = self.current_tts_id.load(Ordering::Relaxed);
        if current_id == tts_id {
            self.is_tts_playing.store(false, Ordering::Relaxed);
            self.interrupt_requested.store(false, Ordering::Relaxed);
        }
    }

    /// Check if TTS is currently playing
    pub fn is_tts_playing(&self) -> bool {
        self.is_tts_playing.load(Ordering::Relaxed)
    }

    /// Request interrupt of current TTS
    pub fn request_interrupt(&self) -> bool {
        if self.is_tts_playing() {
            self.interrupt_requested.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Check if interrupt was requested for the given TTS ID
    pub fn should_interrupt(&self, tts_id: u64) -> bool {
        let current_id = self.current_tts_id.load(Ordering::Relaxed);
        current_id == tts_id && self.interrupt_requested.load(Ordering::Relaxed)
    }

    /// Clear interrupt flag
    pub fn clear_interrupt(&self) {
        self.interrupt_requested.store(false, Ordering::Relaxed);
    }
}
