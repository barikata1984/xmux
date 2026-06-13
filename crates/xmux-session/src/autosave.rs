use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct AutoSaver {
    dirty: Arc<AtomicBool>,
    interval: Duration,
}

impl AutoSaver {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            dirty: Arc::new(AtomicBool::new(false)),
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    pub fn clear_dirty(&self) {
        self.dirty.store(false, Ordering::Relaxed);
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn dirty_flag(&self) -> Arc<AtomicBool> {
        self.dirty.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_flag() {
        let autosaver = AutoSaver::new(10);
        assert!(!autosaver.is_dirty(), "should not be dirty initially");

        autosaver.mark_dirty();
        assert!(autosaver.is_dirty(), "should be dirty after mark_dirty");

        autosaver.clear_dirty();
        assert!(!autosaver.is_dirty(), "should not be dirty after clear_dirty");
    }

    #[test]
    fn test_interval() {
        let autosaver = AutoSaver::new(30);
        assert_eq!(autosaver.interval(), Duration::from_secs(30));
    }
}
