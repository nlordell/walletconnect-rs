use atty::Stream;
use termcolor::Color;
use termcolor::{ColorChoice, StandardStream};
use terminfo::capability::MaxColors;
use terminfo::Database;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Output {
    Stdout,
    Stderr,
}

impl Output {
    pub fn stream(self) -> StandardStream {
        match self {
            Output::Stdout => StandardStream::stdout(default_color_choice(Stream::Stdout)),
            Output::Stderr => StandardStream::stderr(default_color_choice(Stream::Stderr)),
        }
    }
}

fn default_color_choice(stream: Stream) -> ColorChoice {
    if atty::is(stream) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

impl Default for Output {
    fn default() -> Self {
        Output::Stdout
    }
}

#[derive(Clone, Debug)]
pub struct Colors {
    pub black: Option<Color>,
    pub white: Option<Color>,
}

impl Colors {
    pub fn none() -> Self {
        Colors {
            black: None,
            white: None,
        }
    }

    fn standard() -> Self {
        Colors {
            black: Some(Color::Black),
            white: Some(Color::White),
        }
    }

    #[cfg(unix)]
    fn ansi256() -> Self {
        Colors {
            black: Some(Color::Ansi256(16)),
            white: Some(Color::Ansi256(231)),
        }
    }
}

impl Default for Colors {
    #[cfg(unix)]
    fn default() -> Self {
        if let Ok(db) = Database::from_env() {
            match db.get::<MaxColors>() {
                Some(MaxColors(8)) => Colors::standard(),
                Some(MaxColors(256)) => Colors::ansi256(),
                _ => Colors::none(),
            }
        } else {
            Colors::none()
        }
    }

    #[cfg(windows)]
    fn default() -> Self {
        Colors::standard()
    }
}
