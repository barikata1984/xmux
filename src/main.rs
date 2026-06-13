mod input;
mod pane;
mod terminal_view;
mod workspace;

use std::borrow::Cow;
use std::time::Duration;

use iced::widget::canvas::Canvas;
use iced::widget::pane_grid;
use iced::widget::{button, column, container, row, text, scrollable};
use iced::{Background, Color, Element, Length, Size, Subscription, Task, Theme};

use pane::PaneState;
use terminal_view::TerminalView;
use workspace::WorkspaceManager;
use xmux_platform::{PlatformClipboard, create_platform};
use xmux_notification::NotificationManager;

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
    notification_manager: NotificationManager,
    notification_panel_visible: bool,
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
    ToggleNotificationPanel,
    MarkAllNotificationsRead,
    ClearNotifications,
    InjectTestNotification,
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
                notification_manager: NotificationManager::new(1000),
                notification_panel_visible: false,
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
                        // Drain notifications from this terminal
                        for notif in pane_state.terminal.drain_notifications() {
                            self.notification_manager.add(notif, Some(pane_state.id));
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
            Message::ToggleNotificationPanel => {
                self.notification_panel_visible = !self.notification_panel_visible;
            }
            Message::MarkAllNotificationsRead => {
                self.notification_manager.mark_all_read();
            }
            Message::ClearNotifications => {
                self.notification_manager.clear();
            }
            Message::InjectTestNotification => {
                use xmux_notification::{OscNotification, NotificationId, OscProtocol};
                let notif = OscNotification {
                    id: NotificationId::new(),
                    protocol: OscProtocol::Osc9,
                    title: Some("Test".to_string()),
                    body: "Test notification from Ctrl+Shift+I".to_string(),
                };
                let pane_id = self.workspace_manager.active()
                    .focus
                    .and_then(|p| self.workspace_manager.active().panes.get(p))
                    .map(|ps| ps.id);
                self.notification_manager.add(notif, pane_id);
            }
        }
    }

    fn notification_panel_view(&self) -> Element<'_, Message> {
        let mut items = column(vec![]).spacing(4).padding(8);

        // Header with action buttons
        let header = row(vec![
            text("Notifications").size(16).into(),
            button(text("Read All").size(11))
                .on_press(Message::MarkAllNotificationsRead)
                .padding(4)
                .style(button::secondary)
                .into(),
            button(text("Clear").size(11))
                .on_press(Message::ClearNotifications)
                .padding(4)
                .style(button::secondary)
                .into(),
        ]).spacing(8);
        items = items.push(header);

        // Notification list (most recent first)
        for notif in self.notification_manager.list().iter().rev().take(50) {
            let style = if notif.read { Color::from_rgb(0.5, 0.5, 0.5) } else { Color::WHITE };
            let title_text = notif.title.as_deref().unwrap_or("");
            let display = if title_text.is_empty() {
                notif.body.clone()
            } else {
                format!("{}: {}", title_text, notif.body)
            };
            items = items.push(text(display).size(12).color(style));
        }

        let scrollable_items = scrollable(items)
            .height(Length::Fixed(300.0));

        container(scrollable_items)
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(300.0))
            .style(|_theme: &Theme| {
                iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.18))),
                    ..Default::default()
                }
            })
            .into()
    }

    fn sidebar_view(&self) -> Element<'_, Message> {
        let mut tabs = column(vec![]);

        for (i, ws) in self.workspace_manager.workspaces.iter().enumerate() {
            let is_active = i == self.workspace_manager.active_index;

            // Count unread notifications for this workspace's panes
            let unread: usize = ws.panes.iter()
                .map(|(_, ps)| self.notification_manager.unread_count_for_pane(&ps.id))
                .sum();

            let label_text = if unread > 0 {
                format!("{} ({})", ws.name, unread)
            } else {
                ws.name.clone()
            };
            let label = text(label_text).size(14);

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

        // Add notification panel toggle button at the bottom
        let notif_count = self.notification_manager.unread_count();
        let notif_label = if notif_count > 0 {
            format!("Notifications ({})", notif_count)
        } else {
            "Notifications".to_string()
        };
        let notif_btn = button(text(notif_label).size(12))
            .on_press(Message::ToggleNotificationPanel)
            .width(Length::Fill)
            .padding(6)
            .style(button::secondary);

        let tabs = tabs.spacing(2).padding(4).width(Length::Fixed(200.0));

        let sidebar_content = column(vec![
            tabs.into(),
            container(notif_btn)
                .padding(4)
                .width(Length::Fixed(200.0))
                .into(),
        ]);

        container(sidebar_content)
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
                pane_state: state,
            };
            let canvas = Canvas::new(view)
                .width(Length::Fill)
                .height(Length::Fill);

            pane_grid::Content::new(canvas)
        })
        .on_click(|pane| Message::FocusPane(pane))
        .on_resize(10, |event| Message::PaneResized(event));

        if self.sidebar_visible {
            let sidebar = self.sidebar_view();
            if self.notification_panel_visible {
                let panel = self.notification_panel_view();
                let left = column(vec![sidebar, panel]);
                row(vec![left.into(), pane_grid.into()]).into()
            } else {
                row(vec![sidebar, pane_grid.into()]).into()
            }
        } else {
            pane_grid.into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
    }
}
