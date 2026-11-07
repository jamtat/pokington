//! Inline output using ratatui-image's capability detection and protocol encoders.

use crate::art::{Geometry, Width};
use image::{DynamicImage, RgbaImage};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::{Rect, Size},
    style::Color,
    widgets::Widget,
};
use ratatui_image::{
    Image, Resize,
    picker::{Picker, ProtocolType, cap_parser::QueryStdioOptions},
    protocol::Protocol,
};
use serde::Serialize;
use std::{
    fmt::Write as _,
    io::{self, IsTerminal, Write},
    time::Duration,
};

/// Query capabilities through ratatui-image only when both streams are the same terminal.
///
/// # Errors
/// Returns unsupported for pipelines or missing terminal input, or a library/query error.
pub fn query_picker() -> io::Result<Picker> {
    let input = io::stdin();
    let output = io::stdout();
    if !input.is_terminal() || !output.is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native images require terminal stdin and stdout",
        ));
    }
    let a = rustix::fs::fstat(&input)?;
    let b = rustix::fs::fstat(&output)?;
    if a.st_rdev != b.st_rdev || a.st_dev != b.st_dev {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "stdin and stdout are different terminals",
        ));
    }
    let before = rustix::termios::tcgetattr(&input)?;
    let result = Picker::from_query_stdio_with_options(QueryStdioOptions {
        timeout: Duration::from_millis(500),
        ..QueryStdioOptions::default()
    });
    // Also restore at the caller boundary when the library's query times out.
    rustix::termios::tcsetattr(&input, rustix::termios::OptionalActions::Now, &before)?;
    result.map_err(io::Error::other)
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
///
/// # Errors
///
/// Returns invalid data when the image is fully transparent or dimensions cannot be represented.
pub fn crop_visible(image: &RgbaImage) -> io::Result<(RgbaImage, Crop)> {
    let (mut left, mut top) = image.dimensions();
    let (mut right, mut bottom) = (0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] != 0 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x.checked_add(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "image width overflow")
            })?);
            bottom = bottom.max(y.checked_add(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "image height overflow")
            })?);
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
        width: right.checked_sub(left).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid image crop width")
        })?,
        height: bottom.checked_sub(top).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid image crop height")
        })?,
    };
    Ok((
        image::imageops::crop_imm(image, left, top, crop.width, crop.height).to_image(),
        crop,
    ))
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameReport {
    pub protocol: String,
    pub payload_width: u32,
    pub payload_height: u32,
    pub image_columns: u16,
    pub image_rows: u16,
    pub encoded_bytes: usize,
}

#[derive(Debug)]
pub struct NativeFrame {
    bytes: Vec<u8>,
    pub report: FrameReport,
}
impl NativeFrame {
    /// Let Picker create the protocol; serialize its widget cells as inline rows.
    ///
    /// # Errors
    /// Returns unsupported when Picker selected text, or image/encoding/output-size errors.
    pub fn render(
        image: &RgbaImage,
        width: Width,
        geometry: Geometry,
        picker: &Picker,
    ) -> io::Result<Self> {
        if picker.protocol_type() == ProtocolType::Halfblocks {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal has no detected image protocol; use --mode auto, blocks, or ascii",
            ));
        }
        let font = picker.font_size();
        let width = match width {
            Width::Columns(columns) => {
                Width::Columns(columns.checked_mul(u32::from(font.width)).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "native width overflow")
                })?)
            }
            other => other,
        };
        let prepared = crate::art::prepare(
            image,
            width,
            Geometry {
                columns: geometry.columns.saturating_mul(u32::from(font.width)),
                rows: geometry.rows.saturating_mul(u32::from(font.height)),
                cell_width: 1,
                cell_height: 1,
            },
            false,
        )?;
        let size = Size::new(
            u16::try_from(prepared.width().div_ceil(u32::from(font.width)))
                .map_err(io::Error::other)?,
            u16::try_from(prepared.height().div_ceil(u32::from(font.height)))
                .map_err(io::Error::other)?,
        );
        if picker.protocol_type() == ProtocolType::Kitty && (size.width > 297 || size.height > 297)
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "image exceeds ratatui-image's 297-cell Kitty placeholder limit; use blocks or ascii",
            ));
        }
        let prepared = DynamicImage::ImageRgba8(prepared);
        // Library crop/padding keeps original pixels exact and prevents cell rounding from scaling them.
        let padded = Resize::Crop(None).resize(&prepared, font, size, None);
        let protocol = picker
            .new_protocol(padded, size, Resize::Crop(None))
            .map_err(io::Error::other)?;
        let reserved = size
            .height
            .min(u16::try_from(geometry.rows).unwrap_or(u16::MAX));
        let mut encoded = "\r\n".repeat(usize::from(reserved));
        let _ = write!(encoded, "\x1b[{reserved}A\r");
        if let Protocol::Sixel(sixel) = &protocol {
            // Sixel advances below its own image, including images taller than the viewport.
            encoded.push_str(&sixel.data);
            encoded.push_str("\x1b[0m\r");
        } else {
            let area = Rect::new(0, 0, size.width, size.height);
            let mut buffer = Buffer::empty(area);
            Image::new(&protocol).render(area, &mut buffer);
            for y in 0..size.height {
                // Native protocols differ in cursor movement; restore the row origin before advancing.
                encoded.push_str("\x1b7");
                for x in 0..size
                    .width
                    .min(u16::try_from(geometry.columns).unwrap_or(u16::MAX))
                {
                    let cell = &buffer[(x, y)];
                    if cell.diff_option == CellDiffOption::Skip {
                        continue;
                    }
                    if let Color::Rgb(r, g, b) = cell.fg {
                        let _ = write!(encoded, "\x1b[38;2;{r};{g};{b}m");
                    }
                    encoded.push_str(cell.symbol());
                }
                encoded.push_str("\x1b8\x1b[0m\r\n");
            }
        }
        let report = FrameReport {
            protocol: format!("{:?}", picker.protocol_type()),
            payload_width: u32::from(size.width).saturating_mul(u32::from(font.width)),
            payload_height: u32::from(size.height).saturating_mul(u32::from(font.height)),
            image_columns: size.width,
            image_rows: size.height,
            encoded_bytes: encoded.len(),
        };
        Ok(Self {
            bytes: encoded.into_bytes(),
            report,
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Write inline output, cancelling an incomplete escape and resetting style on failure.
    ///
    /// # Errors
    /// Returns the output error, including any failed cleanup.
    pub fn write_to(&self, output: &mut impl Write) -> io::Result<()> {
        if let Err(error) = output.write_all(&self.bytes).and_then(|()| output.flush()) {
            if let Err(cleanup) = output
                .write_all(b"\x18\x1b\\\x1b[0m\r\n")
                .and_then(|()| output.flush())
            {
                return Err(io::Error::new(
                    error.kind(),
                    format!("{error}; output cleanup also failed: {cleanup}"),
                ));
            }
            return Err(error);
        }
        Ok(())
    }
}
