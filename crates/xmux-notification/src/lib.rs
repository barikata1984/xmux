pub mod parser;

pub use parser::{NotificationId, OscNotification, OscParser, OscProtocol};

use std::time::Instant;
use xmux_core::PaneId;

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: NotificationId,
    pub pane_id: Option<PaneId>,
    pub title: Option<String>,
    pub body: String,
    pub timestamp: Instant,
    pub read: bool,
    pub protocol: OscProtocol,
}

pub struct NotificationManager {
    notifications: Vec<Notification>,
    max_count: usize,
}

impl NotificationManager {
    pub fn new(max_count: usize) -> Self {
        Self {
            notifications: Vec::new(),
            max_count,
        }
    }

    pub fn add(&mut self, osc: OscNotification, pane_id: Option<PaneId>) -> &Notification {
        if self.notifications.len() >= self.max_count {
            self.notifications.remove(0);
        }
        self.notifications.push(Notification {
            id: osc.id,
            pane_id,
            title: osc.title,
            body: osc.body,
            timestamp: Instant::now(),
            read: false,
            protocol: osc.protocol,
        });
        self.notifications.last().unwrap()
    }

    pub fn list(&self) -> &[Notification] {
        &self.notifications
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }

    pub fn unread_count_for_pane(&self, pane_id: &PaneId) -> usize {
        self.notifications
            .iter()
            .filter(|n| !n.read && n.pane_id.as_ref() == Some(pane_id))
            .count()
    }

    pub fn mark_read(&mut self, id: &NotificationId) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == *id) {
            n.read = true;
        }
    }

    pub fn mark_all_read(&mut self) {
        for n in &mut self.notifications {
            n.read = true;
        }
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    pub fn clear_one(&mut self, id: &NotificationId) {
        self.notifications.retain(|n| n.id != *id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_osc(body: &str) -> OscNotification {
        OscNotification {
            id: NotificationId::new(),
            protocol: OscProtocol::Osc9,
            title: None,
            body: body.to_string(),
        }
    }

    #[test]
    fn test_add_and_list() {
        let mut mgr = NotificationManager::new(100);
        mgr.add(make_osc("hello"), None);
        mgr.add(make_osc("world"), None);
        assert_eq!(mgr.list().len(), 2);
    }

    #[test]
    fn test_unread_count() {
        let mut mgr = NotificationManager::new(100);
        mgr.add(make_osc("a"), None);
        mgr.add(make_osc("b"), None);
        assert_eq!(mgr.unread_count(), 2);
        let id = mgr.list()[0].id.clone();
        mgr.mark_read(&id);
        assert_eq!(mgr.unread_count(), 1);
    }

    #[test]
    fn test_max_count_eviction() {
        let mut mgr = NotificationManager::new(2);
        mgr.add(make_osc("a"), None);
        mgr.add(make_osc("b"), None);
        mgr.add(make_osc("c"), None);
        assert_eq!(mgr.list().len(), 2);
        assert_eq!(mgr.list()[0].body, "b");
        assert_eq!(mgr.list()[1].body, "c");
    }

    #[test]
    fn test_clear() {
        let mut mgr = NotificationManager::new(100);
        mgr.add(make_osc("a"), None);
        mgr.clear();
        assert_eq!(mgr.list().len(), 0);
    }

    #[test]
    fn test_mark_all_read() {
        let mut mgr = NotificationManager::new(100);
        mgr.add(make_osc("a"), None);
        mgr.add(make_osc("b"), None);
        mgr.mark_all_read();
        assert_eq!(mgr.unread_count(), 0);
    }
}
