use iced::widget::column;
use iced::Element;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window_size(iced::Size::new(1024.0, 768.0))
        .centered()
        .run()
}

struct App;

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> Self {
        Self
    }

    fn title(&self) -> String {
        String::from("xmux")
    }

    fn update(&mut self, _message: Message) {}

    fn view(&self) -> Element<'_, Message> {
        column![].into()
    }
}
