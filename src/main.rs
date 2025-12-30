#![deny(dead_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(unsafe_code)]

use std::time::{Duration, SystemTime};

use iced::Element;
use iced::Length::Fill;
use iced::theme::Theme;
use iced::widget::{button, column, container, row, text};
use iced::window::{self, Position};

extern crate iced;

#[derive(Debug, Clone)]
struct Config {
    warn_after_minutes: u64,
    window_size_x: f32,
    window_size_y: f32,
    window_position_x: f32,
    window_position_y: f32,
    always_on_top: bool,
}

#[derive(Debug, Clone, Copy)]
struct State {
    paused: bool,
    start: SystemTime,
    warn_after_minutes: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Toggle,
    Refresh,
}

pub fn main() -> iced::Result {
    let config = Config::from_env();
    let state = State::new(config.warn_after_minutes);
    let window = create_window_config(config);

    iced::application::application(move || state, State::update, State::view)
        .window(window)
        .antialiasing(true)
        .theme(create_theme)
        .subscription(State::subscription)
        .run()
}

impl State {
    fn update(&mut self, message: Message) {
        if let Message::Toggle = message {
            self.toggle_pause()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let time_passed_seconds = SystemTime::now()
            .duration_since(self.start)
            .unwrap()
            .as_secs();
        let highlight_color =
            highlight_col(time_passed_seconds, self.paused, self.warn_after_minutes);
        let highlight_color =
            iced::Color::from_rgb8(highlight_color[0], highlight_color[1], highlight_color[2]);

        let timer = button(
            text(format_text(time_passed_seconds))
                .color(highlight_color)
                .font(iced::Font::MONOSPACE)
                .size(32),
        )
        .on_press(Message::Toggle);

        container(column![row![timer]])
            .padding(10)
            .center_x(Fill)
            .center_y(Fill)
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(vec![
            iced::time::every(Duration::from_millis(if self.paused { 1000 } else { 500 }))
                .map(|_| Message::Refresh),
        ])
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            self.start = SystemTime::now();
        }
        self.paused = !self.paused;
    }
}

impl State {
    fn new(warn_after_minutes: u64) -> Self {
        Self {
            paused: false,
            start: SystemTime::now(),
            warn_after_minutes,
        }
    }
}

impl Config {
    fn from_env() -> Self {
        Self {
            warn_after_minutes: from_env("STOPWATCH_WARN_AFTER_MINUTES", "20")
                .parse()
                .unwrap_or(20),
            window_size_x: from_env("STOPWATCH_WINDOW_SIZE_X", "180")
                .parse()
                .unwrap_or(180.),
            window_size_y: from_env("STOPWATCH_WINDOW_SIZE_Y", "80")
                .parse()
                .unwrap_or(80.),
            window_position_x: from_env("STOPWATCH_WINDOW_POSITION_X", "40")
                .parse()
                .unwrap_or(40.),
            window_position_y: from_env("STOPWATCH_WINDOW_POSITION_Y", "40")
                .parse()
                .unwrap_or(40.),
            always_on_top: from_env("STOPWATCH_ALWAYS_ON_TOP", "false")
                .parse()
                .unwrap_or(false),
        }
    }
}

fn from_env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn format_text(seconds: u64) -> String {
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{:02}:{:02}", minutes, seconds)
}

const fn highlight_col(seconds: u64, paused: bool, warn_after_minutes: u64) -> [u8; 3] {
    if paused {
        return [200, 200, 200];
    }

    [
        if seconds / 60 >= warn_after_minutes {
            255
        } else {
            0
        },
        255,
        0,
    ]
}

fn create_window_config(config: Config) -> window::Settings {
    let icon = match window::icon::from_file_data(include_bytes!("../resource/icon.png"), None) {
        Ok(icon) => Some(icon),
        Err(e) => {
            eprintln!("Failed to load icon: {}", e);
            None
        }
    };

    window::Settings {
        size: iced::Size::from([config.window_size_x, config.window_size_y]),
        position: Position::Specific(iced::Point::from([
            config.window_position_x,
            config.window_position_y,
        ])),
        visible: true,
        resizable: true,
        transparent: true,
        level: if config.always_on_top {
            window::Level::AlwaysOnTop
        } else {
            window::Level::Normal
        },
        icon,
        exit_on_close_request: true,
        ..Default::default()
    }
}

fn create_theme(_state: &State) -> Theme {
    let theme = Theme::Dark;

    Theme::custom(
        "Custom".to_owned(),
        iced::theme::palette::Palette {
            background: theme.palette().background,
            text: theme.palette().text,
            primary: theme.palette().background.scale_alpha(0.),
            success: theme.palette().success,
            warning: theme.palette().warning,
            danger: theme.palette().danger,
        },
    )
}
