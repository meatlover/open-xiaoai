use std::future::Future;
use std::io::Read;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::base::AppError;

const EV_KEY: u16 = 1;
const KEY_VOLUMEUP: u16 = 114;
const KEY_VOLUMEDOWN: u16 = 115;
const KEY_MUTE: u16 = 102;
const KEY_PLAYPAUSE: u16 = 139;

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonEvent {
    VolumeUp,
    VolumeDown,
    Mute,
    PlayPause,
}

pub struct ButtonMonitor {
    task_holder: Option<JoinHandle<()>>,
}

impl Default for ButtonMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonMonitor {
    pub fn new() -> Self {
        Self { task_holder: None }
    }

    pub async fn start<F, Fut>(&mut self, on_event: F)
    where
        F: Fn(ButtonEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<ButtonEvent>(32);

        // Blocking reader thread for /dev/input/event0
        tokio::task::spawn_blocking(move || {
            let mut file = match std::fs::File::open("/dev/input/event0") {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("❌ Failed to open /dev/input/event0: {}", e);
                    return;
                }
            };

            // ARM32 input_event: 8B timeval + 2B type + 2B code + 4B value = 16 bytes
            let mut buf = [0u8; 16];
            loop {
                if file.read_exact(&mut buf).is_err() {
                    break;
                }

                let ev_type = u16::from_le_bytes([buf[8], buf[9]]);
                let code = u16::from_le_bytes([buf[10], buf[11]]);
                let value = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);

                if ev_type != EV_KEY {
                    continue;
                }

                let event = match code {
                    KEY_VOLUMEUP if value == 1 || value == 2 => Some(ButtonEvent::VolumeUp),
                    KEY_VOLUMEDOWN if value == 1 || value == 2 => Some(ButtonEvent::VolumeDown),
                    KEY_MUTE if value == 1 => Some(ButtonEvent::Mute),
                    KEY_PLAYPAUSE if value == 1 => Some(ButtonEvent::PlayPause),
                    _ => None,
                };

                if let Some(event) = event {
                    if tx.blocking_send(event).is_err() {
                        break;
                    }
                }
            }
        });

        // Async task to receive events and invoke callback
        let monitor = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = on_event(event).await {
                    eprintln!("❌ Button handler error: {}", e);
                }
            }
        });

        if let Some(old_task) = self.task_holder.replace(monitor) {
            println!("Aborting old button monitor task");
            old_task.abort();
        };
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.task_holder.take() {
            handle.abort();
        }
    }
}
