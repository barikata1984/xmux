mod input;
mod pane;
mod terminal_view;

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::canvas::Canvas;
use iced::widget::pane_grid;
use iced::{Element, Length, Size, Subscription, Task};

use pane::PaneState;
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
    panes: pane_grid::State<PaneState>,
    focus: Option<pane_grid::Pane>,
    cell_width: f32,
    cell_height: f32,
    clipboard: Box<dyn PlatformClipboard>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Copy(String),
    Paste,
    Split(pane_grid::Axis, pane_grid::Pane),
    ClosePane(pane_grid::Pane),
    FocusPane(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let pane_state = PaneState::new().expect("failed to create initial pane");
        let (panes, first_pane) = pane_grid::State::new(pane_state);
        let platform = create_platform();
        (
            Self {
                panes,
                focus: Some(first_pane),
                cell_width: 8.4,
                cell_height: 16.8,
                clipboard: platform.clipboard,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        if let Some(pane) = self.focus {
            if let Some(state) = self.panes.get(pane) {
                let t = state.terminal.title();
                if t.is_empty() {
                    String::from("xmux")
                } else {
                    format!("xmux — {}", t)
                }
            } else {
                String::from("xmux")
            }
        } else {
            String::from("xmux")
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                // Process events on all panes and clear cache if needed.
                for (_, pane_state) in self.panes.iter_mut() {
                    if pane_state.terminal.process_events() {
                        pane_state.cache.clear();
                    }
                }
            }
            Message::Copy(text) => {
                if let Err(e) = self.clipboard.set_text(&text) {
                    eprintln!("clipboard copy failed: {e}");
                }
            }
            Message::Paste => {
                if let Some(pane) = self.focus {
                    if let Some(state) = self.panes.get_mut(pane) {
                        match self.clipboard.get_text() {
                            Ok(text) if !text.is_empty() => {
                                let is_bracketed = state.terminal.with_term(|t| {
                                    t.mode().contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE)
                                });
                                if is_bracketed {
                                    state.terminal.write(Cow::Borrowed(&b"\x1b[200~"[..]));
                                    state.terminal.write(Cow::Owned(text.into_bytes()));
                                    state.terminal.write(Cow::Borrowed(&b"\x1b[201~"[..]));
                                } else {
                                    state.terminal.write(Cow::Owned(text.into_bytes()));
                                }
                                state.cache.clear();
                            }
                            Ok(_) => {} // empty clipboard
                            Err(e) => eprintln!("clipboard paste failed: {e}"),
                        }
                    }
                }
            }
            Message::Split(axis, pane) => {
                if let Ok(new_state) = PaneState::new() {
                    if let Some((_new_pane, _split)) = self.panes.split(axis, pane, new_state) {
                        // Pane split successful.
                    }
                }
            }
            Message::ClosePane(pane) => {
                if let Some((state, _surviving_pane)) = self.panes.close(pane) {
                    state.terminal.shutdown();
                    // Update focus if the closed pane was focused.
                    if self.focus == Some(pane) {
                        self.focus = self.panes.iter().next().map(|(p, _)| *p);
                    }
                }
            }
            Message::FocusPane(pane) => {
                self.focus = Some(pane);
            }
            Message::PaneResized(resize_event) => {
                self.panes.resize(resize_event.split, resize_event.ratio);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let cell_width = self.cell_width;
        let cell_height = self.cell_height;

        let pane_grid = pane_grid::PaneGrid::new(&self.panes, |pane, state, _is_focused| {
            let view = TerminalView {
                terminal: &state.terminal,
                cache: &state.cache,
                cell_width,
                cell_height,
                pane,
            };
            let canvas = Canvas::new(view)
                .width(Length::Fill)
                .height(Length::Fill);

            pane_grid::Content::new(canvas)
        })
        .on_click(|pane| Message::FocusPane(pane))
        .on_resize(10, |event| Message::PaneResized(event));

        pane_grid.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
