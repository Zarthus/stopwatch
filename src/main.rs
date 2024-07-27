#![deny(dead_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(unsafe_code)]

use std::io::{Read, Write};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use iced::widget::{button, column, container, row, text};
use iced::Length::Fill;
use iced::{Center, Element, Task};

extern crate iced;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Config {
    warn_after_minutes: u16,
    danger_after_minutes: u16,
    window_size: [f32; 2],
    window_position: [f32; 2],
    always_on_top: bool,
    start_unpaused: bool,
    /// Only supported if feature store_sessions enabled
    store_last_session: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            warn_after_minutes: 45,
            danger_after_minutes: 60,
            window_size: [150., 80.],
            window_position: [40., 40.],
            always_on_top: false,
            start_unpaused: false,
            store_last_session: true,
        }
    }
}

pub fn main() -> iced::Result {
    let settings = load_config();
    let state = State::default();

    WARN_SETTINGS
        .set(WarnSettings {
            warn_after: settings.warn_after_minutes as u64 * 60,
            danger_after: settings.danger_after_minutes as u64 * 60,
        })
        .expect("Failed to set warn settings");

    iced::application::application("Counter", State::update, State::view)
        .antialiasing(true)
        .centered()
        .exit_on_close_request(true)
        .theme(|_| {
            let theme = iced::theme::Theme::Dark;

            iced::theme::Theme::custom(
                "Custom".to_owned(),
                iced::theme::palette::Palette {
                    background: theme.palette().background,
                    text: theme.palette().text,
                    primary: {
                        // todo ensure same bg
                        let col = theme.palette().background.scale_alpha(0.);

                        col
                    },
                    success: theme.palette().success,
                    danger: theme.palette().danger,
                },
            )
        })
        .resizable(true)
        .transparent(true)
        .window_size(iced::Size::from(settings.window_size))
        .position(iced::window::Position::Specific(iced::Point::from(
            settings.window_position,
        )))
        .subscription(State::subscription)
        .run_with(|| (state, Task::none()))
}

#[derive(Debug, Clone)]
struct State {
    paused: bool,
    start: SystemTime,
    sessions: Vec<Session>,
}

#[derive(Debug)]
struct WarnSettings {
    warn_after: u64,
    danger_after: u64,
}

static WARN_SETTINGS: OnceLock<WarnSettings> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct Session {
    pause: bool,
    start: u64,
    end: u64,
}

impl Session {
    fn new(pause: bool, start: SystemTime) -> Self {
        Self {
            pause,
            start: start
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            end: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Toggle,
    Refresh,
}

impl Default for State {
    fn default() -> Self {
        Self {
            paused: true,
            start: SystemTime::now(),
            sessions: vec![],
        }
    }
}

impl State {
    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.toggle_pause(),
            _ => {}
        }
    }

    fn view(&self) -> Element<Message> {
        let timer = if self.paused {
            button("PAUSED")
        } else {
            let time_passed_seconds = SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_secs();

            let highlight_color = highlight_col(&time_passed_seconds);

            button(
                text(format_text(time_passed_seconds, false))
                    .color(highlight_color)
                    .font(iced::Font::MONOSPACE)
                    .size(32),
            )
        }
        .on_press(Message::Toggle);

        let bottom_row = if !self.paused {
            row![]
        } else {
            let pauses = text(format!(
                "breaks: {}",
                self.sessions.iter().filter(|s| s.pause).count()
            ))
            .size(20)
            .width(100);

            row![pauses]
        };

        container(column![row![timer], bottom_row].align_x(Center))
            .padding(10)
            .center_x(Fill)
            .center_y(Fill)
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        if self.paused {
            return iced::Subscription::none();
        }

        iced::Subscription::batch(vec![
            iced::time::every(Duration::from_millis(500)).map(|_| Message::Refresh)
        ])
    }

    fn toggle_pause(&mut self) {
        self.sessions.push(Session::new(self.paused, self.start));
        if self.paused {
            self.start = SystemTime::now();
        }
        self.paused = !self.paused;
        match store_sessions(&self.sessions) {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to store sessions: {}", e),
        }
    }
}

#[inline]
fn format_text(seconds: u64, full: bool) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if hours != 0 || full {
        return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    }
    format!("{:02}:{:02}", minutes, seconds)
}

#[inline]
fn highlight_col(seconds: &u64) -> iced::Color {
    let warn_settings = WARN_SETTINGS.get().unwrap();

    if warn_settings.danger_after == 0 && warn_settings.warn_after == 0 {
        return iced::Color::from_rgb8(0, 0, 0);
    }

    if *seconds > warn_settings.danger_after {
        return iced::Color::from_rgb8(255, 0, 0);
    }

    if *seconds > warn_settings.warn_after {
        return iced::Color::from_rgb8(255, 255, 0);
    }

    iced::Color::from_rgb8(0, 255, 0)
}

fn load_config() -> Config {
    let config_path = dirs::config_dir().unwrap().join("zarthus_counter.toml");

    let config = if !config_path.exists() {
        let config = Config::default();

        let file = std::fs::File::create(config_path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let toml = toml::to_string_pretty(&config).unwrap();

        writer.write_all(toml.as_bytes()).unwrap();

        config
    } else {
        let file = std::fs::File::open(&config_path).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();

        match toml::from_str(&buf) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to parse config: {}\n", e);
                std::process::exit(1);
            }
        }
    };

    config
}

#[cfg(not(feature = "store_sessions"))]
fn store_sessions(_: &Vec<Session>) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "store_sessions")]
fn store_sessions(sessions: &Vec<Session>) -> Result<(), String> {
    let sessions_path = dirs::config_dir().unwrap().join("zarthus_counter.log");

    let sessdata = sessions
        .iter()
        .map(|sess| {
            format!(
                "{} {}",
                format_text(sess.end - sess.start, true),
                if sess.pause { "pause" } else { "active" }
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let file = std::fs::File::create(sessions_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);

    writer
        .write_all(sessdata.as_bytes())
        .map_err(|e| format!("Failed to write to file: {}", e))?;

    Ok(())
}
