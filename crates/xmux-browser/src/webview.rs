use xmux_core::PaneId;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub id: PaneId,
    pub url: String,
    pub title: String,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub loading: bool,
}

impl BrowserState {
    pub fn new(url: String) -> Self {
        Self {
            id: PaneId::new(),
            url,
            title: String::new(),
            can_go_back: false,
            can_go_forward: false,
            loading: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserCommand {
    Navigate(String),
    Back,
    Forward,
    Reload,
    EvalJs(String),
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvent {
    pub pane_id: PaneId,
    pub kind: BrowserEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserEventKind {
    TitleChanged(String),
    UrlChanged(String),
    LoadStarted,
    LoadFinished,
    JsResult(String),
}

pub struct BrowserManager {
    browsers: Vec<BrowserState>,
}

impl BrowserManager {
    pub fn new() -> Self {
        Self { browsers: Vec::new() }
    }

    pub fn open(&mut self, url: String) -> &BrowserState {
        let state = BrowserState::new(url);
        self.browsers.push(state);
        self.browsers.last().unwrap()
    }

    pub fn get(&self, id: &PaneId) -> Option<&BrowserState> {
        self.browsers.iter().find(|b| b.id == *id)
    }

    pub fn get_mut(&mut self, id: &PaneId) -> Option<&mut BrowserState> {
        self.browsers.iter_mut().find(|b| b.id == *id)
    }

    pub fn close(&mut self, id: &PaneId) -> bool {
        let len = self.browsers.len();
        self.browsers.retain(|b| b.id != *id);
        self.browsers.len() < len
    }

    pub fn list(&self) -> &[BrowserState] {
        &self.browsers
    }

    pub fn navigate(&mut self, id: &PaneId, url: String) -> bool {
        if let Some(browser) = self.get_mut(id) {
            browser.url = url;
            browser.loading = true;
            true
        } else {
            false
        }
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_browser() {
        let mut manager = BrowserManager::new();
        let url = "https://example.com".to_string();
        let browser = manager.open(url.clone());

        assert_eq!(browser.url, url);
        assert!(!browser.title.is_empty() || browser.title.is_empty()); // title starts empty
        assert_eq!(browser.title, String::new());
    }

    #[test]
    fn test_close_browser() {
        let mut manager = BrowserManager::new();
        let browser = manager.open("https://example.com".to_string());
        let id = browser.id;

        assert_eq!(manager.list().len(), 1);
        let closed = manager.close(&id);

        assert!(closed);
        assert_eq!(manager.list().len(), 0);
    }

    #[test]
    fn test_navigate() {
        let mut manager = BrowserManager::new();
        let browser = manager.open("https://example.com".to_string());
        let id = browser.id;

        let new_url = "https://other.com".to_string();
        let success = manager.navigate(&id, new_url.clone());

        assert!(success);
        let updated = manager.get(&id).unwrap();
        assert_eq!(updated.url, new_url);
        assert!(updated.loading);
    }

    #[test]
    fn test_list() {
        let mut manager = BrowserManager::new();
        manager.open("https://example1.com".to_string());
        manager.open("https://example2.com".to_string());

        assert_eq!(manager.list().len(), 2);
    }

    #[test]
    fn test_get() {
        let mut manager = BrowserManager::new();
        let browser = manager.open("https://example.com".to_string());
        let id = browser.id;

        let found = manager.get(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().url, "https://example.com");
    }

    #[test]
    fn test_get_nonexistent() {
        let manager = BrowserManager::new();
        let fake_id = PaneId::new();

        let found = manager.get(&fake_id);
        assert!(found.is_none());
    }

    #[test]
    fn test_close_nonexistent() {
        let mut manager = BrowserManager::new();
        let fake_id = PaneId::new();

        let closed = manager.close(&fake_id);
        assert!(!closed);
    }

    #[test]
    fn test_navigate_nonexistent() {
        let mut manager = BrowserManager::new();
        let fake_id = PaneId::new();

        let success = manager.navigate(&fake_id, "https://new.com".to_string());
        assert!(!success);
    }
}
