mod input;
mod pane;
mod terminal_view;
mod workspace;

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::canvas::Canvas;
use iced::widget::pane_grid;
use iced::widget::{button, column, container, row, text};
use iced::{Background, Color, Element, Length, Size, Subscription, Task, Theme};

use pane::PaneState;
use terminal_view::TerminalView;
use workspace::WorkspaceManager;
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
    workspace_manager: WorkspaceManager,
    cell_width: f32,
    cell_height: f32,
    clipboard: Box<dyn PlatformClipboard>,
    sidebar_visible: bool,
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
    NewWorkspace,
    NextWorkspace,
    PrevWorkspace,
    ToggleSidebar,
    SelectWorkspace(usize),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let workspace_manager = WorkspaceManager::new().expect("failed to create workspace manager");
        let platform = create_platform();
        (
            Self {
                workspace_manager,
                cell_width: 8.4,
                cell_height: 16.8,
                clipboard: platform.clipboard,
                sidebar_visible: true,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        let active_ws = self.workspace_manager.active();
        if let Some(pane) = active_ws.focus {
            if let Some(state) = active_ws.panes.get(pane) {
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
                // Process events on all panes in all workspaces and clear cache if needed.
                for workspace in &mut self.workspace_manager.workspaces {
                    for (_, pane_state) in workspace.panes.iter_mut() {
                        if pane_state.terminal.process_events() {
                            pane_state.cache.clear();
                        }
                    }
                }
            }
            Message::Copy(text) => {
                if let Err(e) = self.clipboard.set_text(&text) {
                    eprintln!("clipboard copy failed: {e}");
                }
            }
            Message::Paste => {
                let active_ws = self.workspace_manager.active_mut();
                if let Some(pane) = active_ws.focus {
                    if let Some(state) = active_ws.panes.get_mut(pane) {
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
                let active_ws = self.workspace_manager.active_mut();
                if let Ok(new_state) = PaneState::new() {
                    if let Some((_new_pane, _split)) = active_ws.panes.split(axis, pane, new_state) {
                        // Pane split successful.
                    }
                }
            }
            Message::ClosePane(pane) => {
                let active_ws = self.workspace_manager.active_mut();
                if let Some((state, _surviving_pane)) = active_ws.panes.close(pane) {
                    state.terminal.shutdown();
                    // Update focus if the closed pane was focused.
                    if active_ws.focus == Some(pane) {
                        active_ws.focus = active_ws.panes.iter().next().map(|(p, _)| *p);
                    }
                }
            }
            Message::FocusPane(pane) => {
                let active_ws = self.workspace_manager.active_mut();
                active_ws.focus = Some(pane);
            }
            Message::PaneResized(resize_event) => {
                let active_ws = self.workspace_manager.active_mut();
                active_ws.panes.resize(resize_event.split, resize_event.ratio);
            }
            Message::NewWorkspace => {
                if let Err(e) = self.workspace_manager.create_workspace() {
                    eprintln!("failed to create workspace: {e}");
                }
            }
            Message::NextWorkspace => {
                self.workspace_manager.next_workspace();
            }
            Message::PrevWorkspace => {
                self.workspace_manager.prev_workspace();
            }
            Message::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
            }
            Message::SelectWorkspace(index) => {
                if index < self.workspace_manager.workspaces.len() {
                    self.workspace_manager.active_index = index;
                }
            }
        }
    }

    fn sidebar_view(&self) -> Element<'_, Message> {
        let mut tabs = column(vec![]);

        for (i, ws) in self.workspace_manager.workspaces.iter().enumerate() {
            let is_active = i == self.workspace_manager.active_index;
            let label = text(&ws.name).size(14);
            let btn = button(label)
                .on_press(Message::SelectWorkspace(i))
                .width(Length::Fill)
                .padding(8);

            let btn = if is_active {
                btn.style(button::primary)
            } else {
                btn.style(button::secondary)
            };
            tabs = tabs.push(btn);
        }

        let tabs = tabs.spacing(2).padding(4).width(Length::Fixed(200.0));

        container(tabs)
            .height(Length::Fill)
            .style(|_theme: &Theme| {
                iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.15))),
                    ..Default::default()
                }
            })
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let cell_width = self.cell_width;
        let cell_height = self.cell_height;
        let active_ws = self.workspace_manager.active();

        let pane_grid = pane_grid::PaneGrid::new(&active_ws.panes, |pane, state, _is_focused| {
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

        if self.sidebar_visible {
            row(vec![
                self.sidebar_view(),
                pane_grid.into(),
            ])
            .into()
        } else {
            pane_grid.into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
