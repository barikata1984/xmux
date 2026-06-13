mod terminal_view;

use std::time::Duration;

use iced::widget::canvas::{Cache, Canvas};
use iced::{Element, Length, Size, Subscription, Task};

use terminal_view::TerminalView;

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
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let terminal =
            xmux_terminal::Terminal::new(10_000, 80, 24).expect("failed to create terminal");
        (
            Self {
                terminal,
                cache: Cache::default(),
                cell_width: 8.4,
                cell_height: 16.8,
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
