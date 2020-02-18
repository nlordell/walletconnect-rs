use crate::qr::image::{Dot, Grid};
use atty::Stream;
use std::io::{Result as IoResult, Write};
use termcolor::Color;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
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

    #[cfg(unix)]
    pub fn from_env() -> Self {
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
    pub fn from_env() -> Self {
        Colors::standard()
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

pub trait Print {
    fn print(&self, output: Output, colors: Colors) -> IoResult<()>;
}

impl Print for Grid {
    fn print(&self, output: Output, colors: Colors) -> IoResult<()> {
        let stream = output.stream();
        let mut w = stream.lock();
        let Colors { black, white } = colors;

        let (lines, last_row) = self.lines();

        w.reset()?;
        for line in lines {
            w.set_color(
                ColorSpec::new()
                    .set_reset(false)
                    .set_fg(white)
                    .set_bg(black),
            )?;
            for point in line {
                write!(
                    w,
                    "{}",
                    match point {
                        (Dot::Black, Dot::Black) => ' ',
                        (Dot::Black, Dot::White) => '▄',
                        (Dot::White, Dot::Black) => '▀',
                        (Dot::White, Dot::White) => '█',
                    }
                )?;
            }
            w.reset()?;
            writeln!(w)?;
        }

        if let Some(row) = last_row {
            w.set_color(ColorSpec::new().set_reset(false).set_fg(white))?;
            let mut current_fg = white;

            for dot in row {
                if black.is_some() && w.supports_color() {
                    let fg = match dot {
                        Dot::Black => black,
                        Dot::White => white,
                    };
                    if fg != current_fg {
                        w.set_color(ColorSpec::new().set_reset(false).set_fg(fg))?;
                        current_fg = fg;
                    }
                    write!(w, "▀")?;
                } else {
                    write!(
                        w,
                        "{}",
                        match dot {
                            Dot::White => '▀',
                            Dot::Black => ' ',
                        }
                    )?;
                }
            }
            writeln!(w)?;
        }

        Ok(())
    }
}
