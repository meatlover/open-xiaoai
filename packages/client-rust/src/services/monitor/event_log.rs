use std::future::Future;

use crate::base::AppError;

use super::file::{FileMonitor, FileMonitorEvent};

static EVENT_FILE_PATH: &str = "/tmp/mico_aivs_lab/event.log";

#[derive(Debug, Clone, PartialEq)]
pub enum EventLogEvent {
    Wakeup,
}

pub struct EventLogMonitor {
    file_monitor: FileMonitor,
}

impl Default for EventLogMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLogMonitor {
    pub fn new() -> Self {
        Self {
            file_monitor: FileMonitor::new(),
        }
    }

    pub async fn start<F, Fut>(&mut self, on_update: F)
    where
        F: Fn(EventLogEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        self.file_monitor
            .start(EVENT_FILE_PATH, move |event| {
                let on_update = &on_update;
                async move {
                    if let FileMonitorEvent::NewLine(ref line) = event {
                        // Detect wake-up word: namespace=SpeechWakeup, name=Wakeup
                        if line.contains("\"namespace\":\"SpeechWakeup\"")
                            && line.contains("\"name\":\"Wakeup\"")
                        {
                            let _ = on_update(EventLogEvent::Wakeup).await;
                        }
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub async fn stop(&mut self) {
        self.file_monitor.stop().await;
    }
}
