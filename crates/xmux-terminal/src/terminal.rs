use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;

use alacritty_terminal::event::{Event, WindowSize};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::tty;

use crate::event::EventProxy;
use crate::pty::{create_term, PtyManager, TerminalSize};
use xmux_core::XmuxError;

pub struct Terminal {
    term: Arc<FairMutex<Term<EventProxy>>>,
    pty: PtyManager,
    event_rx: mpsc::Receiver<Event>,
    title: RefCell<String>,
}

impl Terminal {
    pub fn new(scrollback: usize, columns: usize, lines: usize) -> Result<Self, XmuxError> {
        let (event_tx, event_rx) = mpsc::channel();
        let event_proxy = EventProxy::new(event_tx);

        let size = TerminalSize::new(columns, lines);
        let term = create_term(event_proxy.clone(), &size, scrollback);

        let window_size = WindowSize {
            num_lines: lines as u16,
            num_cols: columns as u16,
            cell_width: 8,
            cell_height: 16,
        };

        let options = tty::Options::default();
        let pty = PtyManager::new(term.clone(), event_proxy, &options, window_size, 1)?;

        Ok(Self {
            term,
            pty,
            event_rx,
            title: RefCell::new(String::new()),
        })
    }

    pub fn write(&self, data: impl Into<Cow<'static, [u8]>>) {
        self.pty.write(data);
    }

    pub fn with_term<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Term<EventProxy>) -> R,
    {
        let t = self.term.lock();
        f(&t)
    }

    pub fn with_term_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Term<EventProxy>) -> R,
    {
        let mut t = self.term.lock();
        f(&mut t)
    }

    pub fn resize(&self, columns: usize, lines: usize) {
        let size = TerminalSize::new(columns, lines);
        self.with_term_mut(|t| t.resize(size));

        let window_size = WindowSize {
            num_lines: lines as u16,
            num_cols: columns as u16,
            cell_width: 8,
            cell_height: 16,
        };
        self.pty.resize(window_size);
    }

    pub fn scroll_display(&self, scroll: Scroll) {
        self.with_term_mut(|t| t.scroll_display(scroll));
    }

    pub fn process_events(&self) -> bool {
        let mut wakeup = false;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::Wakeup => wakeup = true,
                Event::Title(t) => *self.title.borrow_mut() = t,
                _ => {}
            }
        }
        wakeup
    }

    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }

    pub fn shutdown(&self) {
        self.pty.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::grid::Dimensions;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn terminal_echo() {
        let terminal = Terminal::new(1000, 80, 24).expect("failed to create Terminal");

        terminal.write(b"echo test123\r".to_vec());
        thread::sleep(Duration::from_millis(500));

        let found = terminal.with_term(|t| {
            let grid = t.grid();
            for line_idx in 0..grid.screen_lines() {
                let mut line_text = String::new();
                for col_idx in 0..grid.columns() {
                    let cell = &grid[alacritty_terminal::index::Line(line_idx as i32)]
                        [alacritty_terminal::index::Column(col_idx)];
                    line_text.push(cell.c);
                }
                if line_text.contains("test123") {
                    return true;
                }
            }
            false
        });

        terminal.shutdown();

        assert!(found, "expected 'test123' in terminal output");
    }
}
