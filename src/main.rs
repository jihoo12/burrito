mod app;
mod canvas;
mod types;

use iced::{Size, Theme};

use crate::app::Whiteboard;

fn new() -> Whiteboard {
    Whiteboard::new()
}

fn theme(_app: &Whiteboard) -> Theme {
    Theme::Light
}

fn main() -> iced::Result {
    if cfg!(target_os = "linux") && std::env::var("WAYLAND_DISPLAY").is_ok() {
        if std::env::var("WINIT_UNIX_BACKEND").is_err() {
            unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "x11") };
        }
    }
    iced::application(new, app::update, app::view)
        .theme(theme)
        .window_size(Size::new(1200.0, 800.0))
        .centered()
        .run()
}
