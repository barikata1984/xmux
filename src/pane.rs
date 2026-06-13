use std::cell::Cell;
use std::collections::HashMap;
use xmux_core::PaneId;
use xmux_terminal::Terminal;
use iced::widget::canvas::Cache;

pub struct PaneState {
    pub id: PaneId,
    pub terminal: Terminal,
    pub cache: Cache,
    pub title: String,
    /// Last known terminal grid size in (columns, rows).
    /// Used for tracking resize events and triggering terminal.resize().
    last_size: Cell<(u16, u16)>,
}

impl PaneState {
    pub fn new() -> Result<Self, xmux_core::XmuxError> {
        let id = PaneId::new();
        let uid = unsafe { libc::getuid() };

        let mut env = HashMap::new();
        env.insert("XMUX".into(), "1".into());
        env.insert("XMUX_PANE_ID".into(), id.to_string());
        env.insert("XMUX_SOCKET_PATH".into(), format!("/tmp/xmux-{uid}.sock"));
        env.insert("TERM".into(), "xterm-256color".into());
        env.insert("COLORTERM".into(), "truecolor".into());

        let terminal = Terminal::new_with_notifications_and_env(10_000, 80, 24, env)?;
        Ok(Self {
            id,
            terminal,
            cache: Cache::default(),
            title: String::new(),
            last_size: Cell::new((80, 24)),
        })
    }

    /// Get the last known terminal grid size.
    pub fn last_size(&self) -> (u16, u16) {
        self.last_size.get()
    }

    /// Update the terminal grid size if it changed, and store the new size.
    pub fn update_size(&self, cols: u16, rows: u16) {
        let current = self.last_size.get();
        if current != (cols, rows) {
            self.last_size.set((cols, rows));
            // Resize both the terminal and PTY.
            self.terminal.resize(cols as usize, rows as usize);
        }
    }
}
