use std::borrow::Cow;
use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::{EventLoop, Msg, State};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{self, Term};
use alacritty_terminal::tty::{self, Options, Pty};

use crate::EventProxy;
use xmux_core::XmuxError;

pub struct TerminalSize {
    pub columns: usize,
    pub screen_lines: usize,
}

impl TerminalSize {
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        Self {
            columns,
            screen_lines,
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

pub struct PtyManager {
    sender: alacritty_terminal::event_loop::EventLoopSender,
    _handle: JoinHandle<(EventLoop<Pty, EventProxy>, State)>,
}

impl PtyManager {
    pub fn new(
        term: Arc<FairMutex<Term<EventProxy>>>,
        event_proxy: EventProxy,
        options: &Options,
        window_size: WindowSize,
        window_id: u64,
    ) -> Result<Self, XmuxError> {
        let pty =
            tty::new(options, window_size, window_id).map_err(|e| XmuxError::Pty(e.to_string()))?;

        let event_loop = EventLoop::new(term, event_proxy, pty, false, false)
            .map_err(|e| XmuxError::Pty(e.to_string()))?;

        let sender = event_loop.channel();
        let handle = event_loop.spawn();

        Ok(Self {
            sender,
            _handle: handle,
        })
    }

    pub fn write(&self, data: impl Into<Cow<'static, [u8]>>) {
        let _ = self.sender.send(Msg::Input(data.into()));
    }

    pub fn resize(&self, size: WindowSize) {
        let _ = self.sender.send(Msg::Resize(size));
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

pub fn create_term(
    event_proxy: EventProxy,
    size: &TerminalSize,
    scrollback: usize,
) -> Arc<FairMutex<Term<EventProxy>>> {
    let config = term::Config {
        scrolling_history: scrollback,
        ..Default::default()
    };
    let term = Term::new(config, size, event_proxy);
    Arc::new(FairMutex::new(term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn pty_echo() {
        let (event_tx, _event_rx) = mpsc::channel();
        let event_proxy = EventProxy::new(event_tx);

        let size = TerminalSize::new(80, 24);
        let term = create_term(event_proxy.clone(), &size, 1000);

        let window_size = WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 8,
            cell_height: 16,
        };

        let options = Options::default();
        let pty = PtyManager::new(term.clone(), event_proxy, &options, window_size, 1)
            .expect("failed to create PTY");

        pty.write(b"echo hello\r".to_vec());
        thread::sleep(Duration::from_millis(500));

        let t = term.lock();
        let grid = t.grid();
        let mut found = false;
        for line_idx in 0..grid.screen_lines() {
            let mut line_text = String::new();
            for col_idx in 0..grid.columns() {
                let cell =
                    &grid[alacritty_terminal::index::Line(line_idx as i32)][alacritty_terminal::index::Column(col_idx)];
                line_text.push(cell.c);
            }
            if line_text.contains("hello") {
                found = true;
                break;
            }
        }
        drop(t);
        pty.shutdown();

        assert!(found, "expected 'hello' in terminal output");
    }
}
