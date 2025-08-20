use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::base::AppError;

use super::file::{FileMonitor, FileMonitorEvent};

#[derive(Debug, Serialize, Deserialize)]
pub enum InterruptMonitorEvent {
    Started,
    InterruptDetected(String),
}

pub struct InterruptMonitor;

pub static INTERRUPT_FILE_PATH: &str = "/tmp/open-xiaoai/interrupt.log";

static LAST_INTERRUPT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl InterruptMonitor {
    pub async fn start<F, Fut>(on_update: F, interrupt_words: Vec<String>)
    where
        F: Fn(InterruptMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let on_update = Arc::new(on_update);
        let interrupt_words = Arc::new(interrupt_words);
        
        FileMonitor::instance()
            .start(INTERRUPT_FILE_PATH, move |event| {
                let on_update = Arc::clone(&on_update);
                let interrupt_words = Arc::clone(&interrupt_words);
                async move {
                    if let FileMonitorEvent::NewLine(content) = event {
                        // Parse the instruction data to check for interrupt words
                        if let Ok(log_message) = serde_json::from_str::<crate::services::monitor::instruction::LogMessage>(&content) {
                            if let crate::services::monitor::instruction::Payload::RecognizeResultPayload { results, .. } = log_message.payload {
                                for result in results {
                                    let text = result.text.trim().to_lowercase();
                                    
                                    // Check if any interrupt word is detected
                                    for interrupt_word in interrupt_words.iter() {
                                        if text.contains(&interrupt_word.to_lowercase()) {
                                            let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                                            let last_timestamp = LAST_INTERRUPT_TIMESTAMP.load(Ordering::Relaxed);
                                            
                                            // Avoid duplicate interrupt detection within 2 seconds
                                            if timestamp - last_timestamp > 2000 {
                                                LAST_INTERRUPT_TIMESTAMP.store(timestamp, Ordering::Relaxed);
                                                let _ = on_update(InterruptMonitorEvent::InterruptDetected(text)).await;
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub async fn stop() {
        LAST_INTERRUPT_TIMESTAMP.store(0, Ordering::Relaxed);
        FileMonitor::instance().stop(INTERRUPT_FILE_PATH).await;
    }
}
