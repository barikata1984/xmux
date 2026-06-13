mod input;
mod terminal_view;

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::canvas::{Cache, Canvas};
use iced::{Element, Length, Size, Subscription, Task};

use terminal_view::TerminalView;
use xmux_platform::{PlatformClipboard, create_platform};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .window_size(Size::new(1024.0, 768.0))
        .centered()
        .run()
}

struct App {
    terminal: xmux_terminal::Terminal,
    cache: Cache,
    cell_width: f32,
    cell_height: f32,
    clipboard: Box<dyn PlatformClipboard>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Copy(String),
    Paste,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let terminal =
            xmux_terminal::Terminal::new(10_000, 80, 24).expect("failed to create terminal");
        let platform = create_platform();
        (
            Self {
                terminal,
                cache: Cache::default(),
                cell_width: 8.4,
                cell_height: 16.8,
                clipboard: platform.clipboard,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        let t = self.terminal.title();
        if t.is_empty() {
            String::from("xmux")
        } else {
            format!("xmux — {}", t)
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                if self.terminal.process_events() {
                    self.cache.clear();
                }
            }
            Message::Copy(text) => {
                if let Err(e) = self.clipboard.set_text(&text) {
                    eprintln!("clipboard copy failed: {e}");
                }
            }
            Message::Paste => {
                match self.clipboard.get_text() {
                    Ok(text) if !text.is_empty() => {
                        let is_bracketed = self.terminal.with_term(|t| {
                            t.mode().contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE)
                        });
                        if is_bracketed {
                            self.terminal.write(Cow::Borrowed(&b"\x1b[200~"[..]));
                            self.terminal.write(Cow::Owned(text.into_bytes()));
                            self.terminal.write(Cow::Borrowed(&b"\x1b[201~"[..]));
                        } else {
                            self.terminal.write(Cow::Owned(text.into_bytes()));
                        }
                        self.cache.clear();
                    }
                    Ok(_) => {} // empty clipboard
                    Err(e) => eprintln!("clipboard paste failed: {e}"),
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let view = TerminalView {
            terminal: &self.terminal,
            cache: &self.cache,
            cell_width: self.cell_width,
            cell_height: self.cell_height,
        };
        Canvas::new(view)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
