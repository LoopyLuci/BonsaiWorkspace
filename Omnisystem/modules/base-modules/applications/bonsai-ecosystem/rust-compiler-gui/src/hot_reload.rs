use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub struct HotReloadWatcher {
    watching: Arc<AtomicBool>,
    last_change: Option<Instant>,
    debounce_ms: u64,
    watched_path: Option<PathBuf>,
}

impl HotReloadWatcher {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            watching: Arc::new(AtomicBool::new(false)),
            last_change: None,
            debounce_ms,
            watched_path: None,
        }
    }

    pub fn start_watching(&mut self, project_path: &Path) -> NotifyResult<mpsc::Receiver<PathBuf>> {
        let (tx, rx) = mpsc::channel();
        let watched_path = project_path.to_path_buf();
        self.watched_path = Some(watched_path.clone());

        let tx_clone = tx.clone();
        let debounce = self.debounce_ms;

        std::thread::spawn(move || {
            let mut watcher = match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        use notify::EventKind;
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) => {
                                // Only watch .rs files
                                if event.paths.iter().any(|p| {
                                    p.extension().and_then(|e| e.to_str()) == Some("rs")
                                }) {
                                    let _ = tx_clone.send(event.paths[0].clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {}
                }
            }) {
                Ok(w) => w,
                Err(_) => return,
            };

            let _ = watcher.watch(&watched_path, RecursiveMode::Recursive);

            // Keep the watcher alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        });

        self.watching.store(true, Ordering::SeqCst);
        Ok(rx)
    }

    pub fn stop_watching(&mut self) {
        self.watching.store(false, Ordering::SeqCst);
        self.watched_path = None;
    }

    pub fn is_watching(&self) -> bool {
        self.watching.load(Ordering::SeqCst)
    }

    pub fn should_rebuild(&mut self) -> bool {
        if let Some(last) = self.last_change {
            let elapsed = last.elapsed().as_millis() as u64;
            if elapsed >= self.debounce_ms {
                self.last_change = None;
                return true;
            }
        }
        false
    }

    pub fn record_change(&mut self) {
        self.last_change = Some(Instant::now());
    }

    pub fn get_watched_path(&self) -> Option<&PathBuf> {
        self.watched_path.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let watcher = HotReloadWatcher::new(500);
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_change_debounce() {
        let mut watcher = HotReloadWatcher::new(100);
        assert!(!watcher.should_rebuild());

        watcher.record_change();
        assert!(!watcher.should_rebuild());

        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(watcher.should_rebuild());
    }
}
