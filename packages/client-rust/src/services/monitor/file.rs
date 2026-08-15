use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use crate::base::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub enum FileMonitorEvent {
    NewFile,
    NewLine(String),
}

pub struct FileMonitor {
    task_holder: Option<JoinHandle<()>>,
}

impl Default for FileMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl FileMonitor {
    pub fn new() -> Self {
        Self { task_holder: None }
    }

    pub async fn start<F, Fut>(&mut self, file_path: &str, on_update: F)
    where
        F: Fn(FileMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let file_path_clone = file_path.to_string();

        let monitor = tokio::spawn(async move {
            let _ = Self::start_monitor(file_path_clone.as_str(), on_update).await;
        });

        if let Some(old_task) = self.task_holder.replace(monitor) {
            println!("Aborting old file monitor task");
            old_task.abort();
        }
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.task_holder.take() {
            handle.abort();
        }
    }

    async fn start_monitor<F, Fut>(file_path: &str, on_update: F) -> Result<(), AppError>
    where
        F: Fn(FileMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        while !Path::new(file_path).exists() {
            sleep(Duration::from_millis(10)).await;
        }

        let file = OpenOptions::new().read(true).open(file_path).await?;
        let mut reader = BufReader::new(file);

        let metadata = reader.get_ref().metadata().await.unwrap();
        let mut position = metadata.len();

        loop {
            // Check the PATH's current metadata, not the open handle's —
            // a handle opened before a rename-based rotation (e.g.
            // BusyBox syslogd rotating /var/log/messages) keeps pointing
            // at the old, now-static file forever; its own metadata never
            // shrinks, so the truncation check below never fires and this
            // monitor silently goes blind to all new lines (found during
            // final review). Reopen when the path's file looks smaller
            // than what we've already read — covers both in-place
            // truncation and rename-based rotation (a freshly rotated
            // file starts empty/small).
            if let Ok(path_metadata) = tokio::fs::metadata(file_path).await {
                if path_metadata.len() < position {
                    position = 0;
                    if let Ok(new_file) = OpenOptions::new().read(true).open(file_path).await {
                        reader = BufReader::new(new_file);
                    }
                    let _ = on_update(FileMonitorEvent::NewFile).await;
                }
            }

            if reader.stream_position().await? != position {
                reader.seek(SeekFrom::Start(position)).await?;
            }

            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }

                let trimmed_line = line.trim();
                if !trimmed_line.is_empty() {
                    let _ = on_update(FileMonitorEvent::NewLine(trimmed_line.to_string())).await;
                }

                position = reader.stream_position().await?;
                line.clear();
            }

            sleep(Duration::from_millis(10)).await;
        }
    }
}
