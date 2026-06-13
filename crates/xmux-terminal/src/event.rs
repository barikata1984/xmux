use std::sync::mpsc;

use alacritty_terminal::event::{Event, EventListener};

#[derive(Clone)]
pub struct EventProxy(mpsc::Sender<Event>);

impl EventProxy {
    pub fn new(sender: mpsc::Sender<Event>) -> Self {
        Self(sender)
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event);
    }
}
