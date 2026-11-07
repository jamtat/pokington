//! Local terminal metadata; no terminal queries or input reads.

use crate::{
    art::{Color, Geometry},
    text_art::Depth,
};
use std::{
    env,
    io::{self, IsTerminal},
};

#[derive(Debug, Clone, Copy)]
pub struct Terminal {
    pub geometry: Geometry,
    pub is_terminal: bool,
    pub unicode: bool,
    pub depth: Depth,
}
impl Default for Terminal {
    fn default() -> Self {
        Self {
            geometry: Geometry {
                rows: u32::MAX,
                ..Geometry::default()
            },
            is_terminal: false,
            unicode: false,
            depth: Depth::None,
        }
    }
}
impl Terminal {
    #[must_use]
    pub fn from_stdio() -> Self {
        let output = io::stdout();
        if !output.is_terminal() {
            return Self::default();
        }
        let term = env::var("TERM").unwrap_or_default();
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .find_map(|key| env::var(key).ok().filter(|s| !s.is_empty()))
            .unwrap_or_default()
            .to_lowercase();
        let unicode = term != "dumb" && (locale.contains("utf-8") || locale.contains("utf8"));
        let color = env::var("COLORTERM").unwrap_or_default().to_lowercase();
        let depth = if term.is_empty() || term == "dumb" {
            Depth::None
        } else if color == "truecolor" || color == "24bit" {
            Depth::TrueColor
        } else if term.contains("256color") {
            Depth::Ansi256
        } else {
            Depth::Ansi16
        };
        let mut geometry = Geometry::default();
        if let Ok(size) = rustix::termios::tcgetwinsize(&output) {
            if size.ws_col != 0 {
                geometry.columns = u32::from(size.ws_col);
            }
            if size.ws_row != 0 {
                geometry.rows = u32::from(size.ws_row).saturating_sub(1).max(1);
            }
            if size.ws_col != 0
                && size.ws_row != 0
                && size.ws_xpixel >= size.ws_col
                && size.ws_ypixel >= size.ws_row
            {
                geometry.cell_width =
                    u32::from(size.ws_xpixel.checked_div(size.ws_col).unwrap_or(1));
                geometry.cell_height =
                    u32::from(size.ws_ypixel.checked_div(size.ws_row).unwrap_or(2));
            }
        }
        Self {
            geometry,
            is_terminal: true,
            unicode,
            depth,
        }
    }

    #[must_use]
    pub fn color(self, policy: Color) -> Depth {
        match policy {
            Color::Never => Depth::None,
            Color::Auto => {
                if self.is_terminal {
                    self.depth
                } else {
                    Depth::None
                }
            }
            Color::Always => {
                if self.depth == Depth::None {
                    Depth::TrueColor
                } else {
                    self.depth
                }
            }
        }
    }
}
