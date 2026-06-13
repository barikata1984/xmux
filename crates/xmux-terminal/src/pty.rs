use std::borrow::Cow;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
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
use xmux_notification::{OscNotification, OscParser};

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
    orig_pty_fd: Option<libc::c_int>,
}

impl PtyManager {
    pub fn new(
        term: Arc<FairMutex<Term<EventProxy>>>,
        event_proxy: EventProxy,
        options: &Options,
        window_size: WindowSize,
        window_id: u64,
        notif_tx: Option<std::sync::mpsc::Sender<OscNotification>>,
    ) -> Result<Self, XmuxError> {
        let pty =
            tty::new(options, window_size, window_id).map_err(|e| XmuxError::Pty(e.to_string()))?;

        let orig_pty_fd = if let Some(tx) = notif_tx {
            Some(Self::setup_interception(&pty, tx)?)
        } else {
            None
        };

        let event_loop = EventLoop::new(term, event_proxy, pty, false, false)
            .map_err(|e| XmuxError::Pty(e.to_string()))?;

        let sender = event_loop.channel();
        let handle = event_loop.spawn();

        Ok(Self {
            sender,
            _handle: handle,
            orig_pty_fd,
        })
    }

    fn setup_interception(
        pty: &Pty,
        notif_tx: std::sync::mpsc::Sender<OscNotification>,
    ) -> Result<libc::c_int, XmuxError> {
        let pty_fd = pty.file().as_raw_fd();

        unsafe {
            // 1. Create a Unix socketpair for bidirectional proxy
            let (our_end, el_end) = std::os::unix::net::UnixStream::pair()
                .map_err(|e| XmuxError::Pty(format!("socketpair: {e}")))?;

            // 2. Save original PTY master fd
            let orig_fd = libc::dup(pty_fd);
            if orig_fd < 0 {
                return Err(XmuxError::Pty("dup failed".into()));
            }

            // 3. Replace PTY fd with EventLoop-facing end of socketpair
            let el_end_fd = el_end.into_raw_fd();
            if libc::dup2(el_end_fd, pty_fd) < 0 {
                libc::close(orig_fd);
                libc::close(el_end_fd);
                return Err(XmuxError::Pty("dup2 failed".into()));
            }
            libc::close(el_end_fd); // pty_fd now IS the socketpair end

            // 4. Set socketpair fd (at pty_fd) to non-blocking (EventLoop requires this)
            let flags = libc::fcntl(pty_fd, libc::F_GETFL);
            libc::fcntl(pty_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);

            // 5. Set our_end to non-blocking too
            our_end.set_nonblocking(true)
                .map_err(|e| XmuxError::Pty(format!("set_nonblocking: {e}")))?;

            // 6. Spawn relay thread
            std::thread::Builder::new()
                .name("osc-interceptor".into())
                .spawn(move || {
                    osc_relay(orig_fd, our_end, notif_tx);
                })
                .map_err(|e| XmuxError::Pty(format!("spawn relay: {e}")))?;

            Ok(orig_fd)
        }
    }

    pub fn write(&self, data: impl Into<Cow<'static, [u8]>>) {
        let _ = self.sender.send(Msg::Input(data.into()));
    }

    pub fn resize(&self, size: WindowSize) {
        if let Some(orig_fd) = self.orig_pty_fd {
            // When intercepting, call ioctl directly on the real PTY fd.
            // Do NOT send Msg::Resize — it would call on_resize() on the socketpair
            // which would die!() because ioctl(TIOCSWINSZ) fails on sockets.
            let winsize = libc::winsize {
                ws_row: size.num_lines as libc::c_ushort,
                ws_col: size.num_cols as libc::c_ushort,
                ws_xpixel: (size.num_cols * size.cell_width) as libc::c_ushort,
                ws_ypixel: (size.num_lines * size.cell_height) as libc::c_ushort,
            };
            unsafe {
                libc::ioctl(orig_fd, libc::TIOCSWINSZ, &winsize as *const _);
            }
        } else {
            let _ = self.sender.send(Msg::Resize(size));
        }
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

fn osc_relay(
    orig_fd: libc::c_int,
    our_end: std::os::unix::net::UnixStream,
    notif_tx: std::sync::mpsc::Sender<OscNotification>,
) {
    let mut pty_file = unsafe { std::fs::File::from_raw_fd(orig_fd) };
    let mut stream = our_end;
    let our_fd = stream.as_raw_fd();
    let mut parser = OscParser::new();
    let mut buf = [0u8; 8192];

    loop {
        let mut fds = [
            libc::pollfd {
                fd: orig_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: our_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let ret = unsafe { libc::poll(fds.as_mut_ptr(), 2, 1000) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }

        // PTY output -> OscParser + forward to EventLoop
        if fds[0].revents & libc::POLLIN != 0 {
            match pty_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for notif in parser.feed(&buf[..n]) {
                        let _ = notif_tx.send(notif);
                    }
                    let mut written = 0;
                    while written < n {
                        match stream.write(&buf[written..n]) {
                            Ok(0) => break,
                            Ok(w) => written += w,
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::yield_now();
                            }
                            Err(_) => return,
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }
        }

        // EventLoop writes -> forward to PTY
        if fds[1].revents & libc::POLLIN != 0 {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let mut written = 0;
                    while written < n {
                        match pty_file.write(&buf[written..n]) {
                            Ok(0) => break,
                            Ok(w) => written += w,
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::yield_now();
                            }
                            Err(_) => return,
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }
        }

        if fds[0].revents & libc::POLLHUP != 0 && fds[0].revents & libc::POLLIN == 0 {
            break;
        }
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
        let pty = PtyManager::new(term.clone(), event_proxy, &options, window_size, 1, None)
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
