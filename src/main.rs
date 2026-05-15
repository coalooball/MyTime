mod app;
mod i18n;
mod model;
mod repository;
mod ui;

use app::MyTimeApp;
use iced::{daemon, Font, Theme};

fn main() -> iced::Result {
    daemon(MyTimeApp::title, MyTimeApp::update, MyTimeApp::view)
        .theme(|_, _| Theme::Light)
        .font(include_bytes!("/System/Library/Fonts/PingFang.ttc").as_slice())
        .font(include_bytes!("/System/Library/Fonts/Hiragino Sans GB.ttc").as_slice())
        .default_font(Font::with_name("PingFang SC"))
        .subscription(MyTimeApp::subscription)
        .run_with(MyTimeApp::new)
}
