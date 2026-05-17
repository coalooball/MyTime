mod app;
mod i18n;
mod model;
mod repository;
mod ui;

use app::MyTimeApp;
use iced::{daemon, theme::Palette, Color, Font, Theme};

fn main() -> iced::Result {
    daemon(MyTimeApp::new, MyTimeApp::update, MyTimeApp::view)
        .title(MyTimeApp::title)
        .theme(app_theme)
        .font(include_bytes!("/System/Library/Fonts/PingFang.ttc").as_slice())
        .font(include_bytes!("/System/Library/Fonts/Hiragino Sans GB.ttc").as_slice())
        .default_font(Font::with_name("PingFang SC"))
        .subscription(MyTimeApp::subscription)
        .run()
}

fn app_theme(_app: &MyTimeApp, _window_id: iced::window::Id) -> Theme {
    mytime_theme()
}

fn mytime_theme() -> Theme {
    Theme::custom(
        "MyTime".to_string(),
        Palette {
            background: Color::from_rgb8(247, 248, 250),
            text: Color::from_rgb8(31, 41, 55),
            primary: Color::from_rgb8(15, 118, 110),
            success: Color::from_rgb8(22, 101, 52),
            warning: Color::from_rgb8(217, 119, 6),
            danger: Color::from_rgb8(185, 28, 28),
        },
    )
}
