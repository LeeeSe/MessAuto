use notify::{RecursiveMode, Event, EventKind, Config, RecommendedWatcher, Watcher};
use std::path::{Path, PathBuf};
use log::{info, error, debug};
use tokio::sync::mpsc::{channel, Receiver};

pub trait FileProcessor: Clone + Send + Sync + 'static {
    fn get_watch_path(&self) -> PathBuf;
    fn get_file_pattern(&self) -> &str;
    fn get_recursive_mode(&self) -> RecursiveMode;
    fn process_file(&self, path: &Path, event_kind: &EventKind) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct FileWatcher<P: FileProcessor> {
    processor: P,
    _watcher_task: Option<tokio::task::JoinHandle<()>>,
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
            _watcher_task: None,
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

        self._watcher_task = Some(task);

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(task) = self._watcher_task.take() {
            task.abort();
            debug!("File watcher task stopped");
        }
    }
}

impl<P: FileProcessor> Drop for FileWatcher<P> {
    fn drop(&mut self) {
        self.stop();
    }
}

impl<P: FileProcessor> FileWatcher<P> {
    async fn watch_path(
        path: PathBuf,
        recursive_mode: RecursiveMode,
        pattern: String,
        processor: P
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
