//! S01 inline native-image feasibility. No event loop, input reads, or terminal mode changes.
//!
//! Detection uses only local ioctl metadata and conservative environment hints. Hints are not
//! a graphics-protocol handshake or visual evidence. Unknown contexts must fall back or fail.

use std::{env, fmt, fmt::Write as _, io, io::Write, os::fd::AsFd};

use image::{DynamicImage, RgbaImage};
use ratatui::{
    buffer::Buffer,
    layout::{Rect, Size},
    style::Color,
    widgets::Widget,
};
use ratatui_image::{
    Image,
    protocol::{Protocol, kitty::Kitty},
};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct TerminalHints {
    pub term: String,
    pub program: String,
    pub version: String,
    pub kitty_window: bool,
    pub multiplexed: bool,
    pub remote: bool,
}

impl TerminalHints {
    pub fn from_env() -> Self {
        let value = |key| env::var(key).unwrap_or_default();
        Self {
            term: value("TERM"),
            program: value("TERM_PROGRAM"),
            version: value("TERM_PROGRAM_VERSION"),
            kitty_window: !value("KITTY_WINDOW_ID").is_empty(),
            multiplexed: !value("TMUX").is_empty() || !value("STY").is_empty(),
            remote: !value("SSH_CONNECTION").is_empty() || !value("SSH_TTY").is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Metrics {
    pub columns: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeTerminal {
    pub identity_hint: &'static str,
    pub protocol: &'static str,
    pub cell_measurement: &'static str,
    pub cell_width_interval: [u16; 2],
    pub cell_height_interval: [u16; 2],
    pub viewport_remainder_width: u16,
    pub viewport_remainder_height: u16,
    pub metrics: Metrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Unavailable {
    RedirectedOutput,
    UnavailableInput,
    DifferentTerminals,
    MultiplexerOrRemote,
    UnknownIdentity,
    UnverifiedMeasurementProfile,
    MissingPixelMetrics,
}

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RedirectedOutput => "stdout is not a terminal",
            Self::UnavailableInput => "stdin is not a terminal",
            Self::DifferentTerminals => "stdin and stdout do not identify the same terminal",
            Self::MultiplexerOrRemote => "multiplexer or remote context is not verified",
            Self::UnknownIdentity => "no recognised paired terminal identity hints",
            Self::UnverifiedMeasurementProfile => "terminal pixel measurement profile is unverified; this prototype supports Ghostty 1.4.0",
            Self::MissingPixelMetrics => {
                "terminal ioctl did not provide exact nonzero cell/pixel metrics"
            }
        })
    }
}

/// Constant-count local metadata calls only. Never writes query escapes, waits for responses,
/// reads either descriptor, changes termios, or opens `/dev/tty`.
pub fn detect(
    input: &impl AsFd,
    output: &impl AsFd,
    hints: &TerminalHints,
) -> Result<NativeTerminal, Unavailable> {
    if !rustix::termios::isatty(output) {
        return Err(Unavailable::RedirectedOutput);
    }
    if !rustix::termios::isatty(input) {
        return Err(Unavailable::UnavailableInput);
    }
    let same_terminal = rustix::fs::fstat(input)
        .ok()
        .zip(rustix::fs::fstat(output).ok())
        .is_some_and(|(a, b)| a.st_rdev == b.st_rdev && a.st_dev == b.st_dev);
    if !same_terminal {
        return Err(Unavailable::DifferentTerminals);
    }
    if hints.multiplexed
        || hints.remote
        || hints.term.starts_with("screen")
        || hints.term.starts_with("tmux")
    {
        return Err(Unavailable::MultiplexerOrRemote);
    }
    let identity_hint = match (
        hints.term.as_str(),
        hints.program.as_str(),
        hints.kitty_window,
    ) {
        ("xterm-ghostty", "ghostty", _) => "Ghostty",
        ("xterm-kitty", "kitty" | "", true) => "Kitty",
        _ => return Err(Unavailable::UnknownIdentity),
    };
    if identity_hint != "Ghostty" || hints.version != "1.4.0" {
        return Err(Unavailable::UnverifiedMeasurementProfile);
    }
    let size =
        rustix::termios::tcgetwinsize(output).map_err(|_| Unavailable::MissingPixelMetrics)?;
    let (columns, rows, pixel_width, pixel_height) =
        (size.ws_col, size.ws_row, size.ws_xpixel, size.ws_ypixel);
    if columns == 0 || rows == 0 || pixel_width < columns || pixel_height < rows {
        return Err(Unavailable::MissingPixelMetrics);
    }
    // Ghostty 1.4.0 exports the padding-subtracted viewport, including partial-cell
    // leftovers. Its grid count is floor(viewport / integer cell pixels). Invert that
    // relation only when there is exactly one possible integer cell dimension.
    // See docs/20261107-inline-native-image-evidence.md for pinned upstream sources.
    let cell_width =
        ghostty_140_cell(pixel_width, columns).ok_or(Unavailable::MissingPixelMetrics)?;
    let cell_height =
        ghostty_140_cell(pixel_height, rows).ok_or(Unavailable::MissingPixelMetrics)?;
    Ok(NativeTerminal {
        identity_hint,
        protocol: "Kitty Unicode placeholders (ratatui-image 11.1.0)",
        cell_measurement: "Ghostty 1.4.0: unique integer solution to grid = floor(viewport / cell), after padding",
        cell_width_interval: [cell_width, cell_width],
        cell_height_interval: [cell_height, cell_height],
        viewport_remainder_width: pixel_width - columns * cell_width,
        viewport_remainder_height: pixel_height - rows * cell_height,
        metrics: Metrics {
            columns,
            rows,
            pixel_width,
            pixel_height,
            cell_width,
            cell_height,
        },
    })
}

fn ghostty_140_cell(pixels: u16, cells: u16) -> Option<u16> {
    // Ghostty clamps grid counts to at least one and ioctl fields to u16::MAX. Neither
    // boundary allows inversion of the original dimensions, so reject it explicitly.
    if cells <= 1 || cells == u16::MAX || pixels == u16::MAX {
        return None;
    }
    let pixels = u32::from(pixels);
    let cells = u32::from(cells);
    let smallest = pixels / (cells + 1) + 1;
    let largest = pixels / cells;
    (smallest == largest && largest != 0).then_some(largest as u16)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Mode {
    Auto,
    Img,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Fallback {
    Blocks,
    Ascii,
}

#[derive(Debug, Clone, Serialize)]
pub enum Selection {
    Native(NativeTerminal),
    Fallback {
        renderer: Fallback,
        reason: Unavailable,
    },
}

/// Text renderer selection only; production block/ASCII artwork is S04 work.
pub fn select(
    mode: Mode,
    detection: Result<NativeTerminal, Unavailable>,
    unicode_terminal: bool,
) -> io::Result<Selection> {
    match detection {
        Ok(terminal) => Ok(Selection::Native(terminal)),
        Err(reason) if mode == Mode::Auto => Ok(Selection::Fallback {
            renderer: if unicode_terminal && reason != Unavailable::RedirectedOutput {
                Fallback::Blocks
            } else {
                Fallback::Ascii
            },
            reason,
        }),
        Err(reason) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("native image unavailable: {reason}; use --mode auto, blocks, or ascii"),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Crop {
    pub source_width: u32,
    pub source_height: u32,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

/// Include every nonzero alpha pixel, including partially transparent edge pixels.
pub fn crop_visible(image: &RgbaImage) -> io::Result<(RgbaImage, Crop)> {
    let (mut left, mut top) = image.dimensions();
    let (mut right, mut bottom) = (0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] != 0 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    if right == 0 || bottom == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "image is fully transparent",
        ));
    }
    let crop = Crop {
        source_width: image.width(),
        source_height: image.height(),
        left,
        top,
        width: right - left,
        height: bottom - top,
    };
    Ok((
        image::imageops::crop_imm(image, left, top, crop.width, crop.height).to_image(),
        crop,
    ))
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameReport {
    pub crop: Crop,
    pub payload_width: u32,
    pub payload_height: u32,
    pub image_columns: u16,
    pub image_rows: u16,
    pub printed_columns: u16,
    pub encoded_bytes: usize,
    pub scaling: &'static str,
}

pub struct NativeFrame {
    bytes: Vec<u8>,
    pub report: FrameReport,
}

impl NativeFrame {
    /// Build using ratatui-image's raw Kitty protocol API, bypassing Picker/Resize entirely.
    /// Pad the crop to full cells without resampling: Kitty virtual placement fits into a
    /// rounded cell grid even without `c`/`r` fields. Equal payload/grid pixels prevent scaling.
    /// Oversized images are clipped at the right edge, never resized to terminal width.
    pub fn original(image: &RgbaImage, metrics: Metrics) -> io::Result<Self> {
        if metrics.cell_width == 0 || metrics.cell_height == 0 || metrics.columns == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "zero terminal cell dimensions",
            ));
        }
        let (cropped, crop) = crop_visible(image)?;
        let columns = crop.width.div_ceil(u32::from(metrics.cell_width));
        let rows = crop.height.div_ceil(u32::from(metrics.cell_height));
        // Upstream's fixed Unicode diacritic table has 297 entries per axis. Reject rather
        // than let its internal min() silently omit unsupported rows/columns.
        if columns > 297 || rows > 297 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "image exceeds ratatui-image's 297-cell Kitty placeholder limit",
            ));
        }
        let size = Size::new(columns as u16, rows as u16);
        let payload_width = columns * u32::from(metrics.cell_width);
        let payload_height = rows * u32::from(metrics.cell_height);
        if u64::from(payload_width) * u64::from(payload_height) > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "padded native image exceeds the 16-megapixel prototype limit",
            ));
        }
        let mut padded = RgbaImage::new(payload_width, payload_height);
        image::imageops::replace(&mut padded, &cropped, 0, 0);
        let area = Rect::new(0, 0, size.width.min(metrics.columns), size.height);
        let id = fresh_image_id();
        let protocol = Protocol::Kitty(
            Kitty::new(DynamicImage::ImageRgba8(padded), size, id, false, false)
                .map_err(io::Error::other)?,
        );
        let mut buffer = Buffer::empty(area);
        Image::new(&protocol)
            .allow_clipping(true)
            .render(area, &mut buffer);
        let mut encoded = String::from("\r\x1b[0m");
        for y in 0..area.height {
            // Kitty uses foreground colour to select the image. Every cell on this row
            // belongs to this image; emit style once, and reset before each newline.
            let Color::Rgb(r, g, b) = buffer[(0, y)].fg else {
                return Err(io::Error::other(
                    "ratatui-image did not emit a native Kitty cell",
                ));
            };
            write!(encoded, "\x1b[38;2;{r};{g};{b}m").expect("String writing cannot fail");
            for x in 0..area.width {
                encoded.push_str(buffer[(x, y)].symbol());
            }
            encoded.push_str("\x1b[0m\r\n");
        }
        let report = FrameReport {
            crop,
            payload_width,
            payload_height,
            image_columns: size.width,
            image_rows: size.height,
            printed_columns: area.width,
            encoded_bytes: encoded.len(),
            scaling: "none; one source pixel per protocol screen pixel",
        };
        Ok(Self {
            bytes: encoded.into_bytes(),
            report,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Writes only inline graphics/text. No cursor hiding, screen switching or termios changes.
    /// If a write is interrupted mid-escape, make a best-effort cancellation/reset. A permanently
    /// broken writer cannot accept cleanup; return that limitation with the original failure.
    pub fn write_to(&self, output: &mut impl Write) -> io::Result<()> {
        let mut guard = OutputGuard {
            output,
            complete: false,
        };
        let result = guard
            .output
            .write_all(&self.bytes)
            .and_then(|()| guard.output.flush());
        match result {
            Ok(()) => {
                guard.complete = true;
                Ok(())
            }
            Err(error) => {
                let cleanup = guard.cleanup();
                guard.complete = true;
                match cleanup {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(io::Error::new(
                        error.kind(),
                        format!("{error}; output cleanup also failed: {cleanup}"),
                    )),
                }
            }
        }
    }
}

// IDs must be nonzero and should not collide with other programs' images. Use kernel entropy
// through std's independently seeded hash builder, avoiding another RNG dependency.
fn fresh_image_id() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let id = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish() as u32;
    id.max(1)
}

struct OutputGuard<'a, W: Write> {
    output: &'a mut W,
    complete: bool,
}
impl<W: Write> OutputGuard<'_, W> {
    fn cleanup(&mut self) -> io::Result<()> {
        self.output.write_all(b"\x18\x1b\\\x1b[0m\r\n")?;
        self.output.flush()
    }
}
impl<W: Write> Drop for OutputGuard<'_, W> {
    fn drop(&mut self) {
        if !self.complete {
            let _ = self.cleanup();
        }
    }
}
