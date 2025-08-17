use log::{debug, error, info};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{Receiver, channel};
use tokio::task::JoinHandle;

pub trait FileProcessor: Clone + Send + Sync + 'static {
    fn get_watch_path(&self) -> PathBuf;
    fn get_file_pattern(&self) -> &str;
    fn get_recursive_mode(&self) -> RecursiveMode;
    fn process_file(
        &self,
        path: &Path,
        event_kind: &EventKind,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct FileWatcher<P: FileProcessor> {
    processor: P,
    watcher_task: Option<JoinHandle<()>>,
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel(100);

    let watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.blocking_send(res);
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

impl<P: FileProcessor> FileWatcher<P> {
    pub fn new(processor: P) -> Self {
        Self {
            processor,
            watcher_task: None,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = self.processor.get_watch_path();
        let recursive_mode = self.processor.get_recursive_mode();
        let pattern = self.processor.get_file_pattern().to_string();
        let processor = self.processor.clone();

        info!("Starting watcher for: {:?}", path);

        let task = tokio::spawn(async move {
            if let Err(e) = Self::watch_path(path, recursive_mode, pattern, processor).await {
                error!("Error in file watcher: {}", e);
            }
        });

        self.watcher_task = Some(task);

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(task) = self.watcher_task.take() {
            debug!("Requesting to stop file watcher task...");
            task.abort();
            match task.await {
                Ok(_) => debug!("File watcher task stopped gracefully."),
                Err(e) if e.is_cancelled() => {
                    debug!("File watcher task was cancelled and has shut down.");
                }
                Err(e) => error!("Error waiting for file watcher task to stop: {:?}", e),
            }
        }
    }
}

impl<P: FileProcessor> Drop for FileWatcher<P> {
    fn drop(&mut self) {
        if let Some(task) = self.watcher_task.take() {
            task.abort();
            debug!("File watcher task aborted during drop");
        }
    }
}

impl<P: FileProcessor> FileWatcher<P> {
    async fn watch_path(
        path: PathBuf,
        recursive_mode: RecursiveMode,
        pattern: String,
        processor: P,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (mut watcher, mut rx) = async_watcher()?;

        watcher.watch(&path, recursive_mode)?;
        debug!("Watcher started with recursive mode: {:?}", recursive_mode);
        debug!("Watching for files matching pattern: {}", pattern);

        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) => {
                    debug!("File event detected: {:?}", event);

                    for path in event.paths {
                        let path_str = path.to_string_lossy();
                        debug!("Checking path: {}", path_str);
                        if path_str.contains(&pattern) {
                            debug!("Detected event in watched file: {}", path_str);

                            if let Err(e) = processor.process_file(&path, &event.kind) {
                                error!("Error processing file: {}", e);
                            }
                        } else {
                            debug!("Path does not match pattern, ignoring");
                        }
                    }
                }
                Err(e) => error!("Watch error: {:?}", e),
            }
        }

        Ok(())
    }
}
