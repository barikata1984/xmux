use xmux_core::PaneId;
use xmux_terminal::Terminal;
use iced::widget::canvas::Cache;

pub struct PaneState {
    pub id: PaneId,
    pub terminal: Terminal,
    pub cache: Cache,
    pub title: String,
}

impl PaneState {
    pub fn new() -> Result<Self, xmux_core::XmuxError> {
        let terminal = Terminal::new(10_000, 80, 24)?;
        Ok(Self {
            id: PaneId::new(),
            terminal,
            cache: Cache::default(),
            title: String::new(),
        })
    }
}
