//! Deterministic ASCII and half-block rendering. Alpha is coverage over black.

use image::{Rgba, RgbaImage};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

fn rgb(pixel: Rgba<u8>) -> [u8; 3] {
    let a = u16::from(pixel[3]);
    [pixel[0], pixel[1], pixel[2]]
        .map(|c| u8::try_from(u16::from(c).saturating_mul(a) / 255).unwrap_or(255))
}

fn brightness(rgb: [u8; 3]) -> u16 {
    let [r, g, b] = rgb.map(u16::from);
    r.saturating_mul(54)
        .saturating_add(g.saturating_mul(183))
        .saturating_add(b.saturating_mul(19))
        / 256
}

fn paint(output: &mut String, rgb: [u8; 3], background: bool, depth: Depth) {
    let prefix = if background { 48 } else { 38 };
    let [r, g, b] = rgb;
    match depth {
        Depth::None => (),
        Depth::TrueColor => {
            let _ = write!(output, "\x1b[{prefix};2;{r};{g};{b}m");
        }
        Depth::Ansi256 => {
            // The standard 6x6x6 cube has levels 0,95,135,175,215,255.
            let level = |c: u8| -> u16 {
                [0_u16, 95, 135, 175, 215, 255]
                    .into_iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.abs_diff(u16::from(c)))
                    .and_then(|(i, _)| u16::try_from(i).ok())
                    .unwrap_or(0)
            };
            let index = 16_u16
                .saturating_add(level(r).saturating_mul(36))
                .saturating_add(level(g).saturating_mul(6))
                .saturating_add(level(b));
            let _ = write!(output, "\x1b[{prefix};5;{index}m");
        }
        Depth::Ansi16 => {
            let bright = r.max(g).max(b) >= 192;
            let bits = u16::from(r >= 96)
                .saturating_add(u16::from(g >= 96).saturating_mul(2))
                .saturating_add(u16::from(b >= 96).saturating_mul(4));
            let base: u16 = match (background, bright) {
                (false, false) => 30,
                (true, false) => 40,
                (false, true) => 90,
                (true, true) => 100,
            };
            let _ = write!(output, "\x1b[{}m", base.saturating_add(bits));
        }
    }
}

#[must_use]
pub fn render(image: &RgbaImage, blocks: bool, depth: Depth) -> String {
    let mut output = String::new();
    let step = if blocks { 2 } else { 1 };
    for y in (0..image.height()).step_by(step) {
        for x in 0..image.width() {
            let top = image.get_pixel(x, y);
            let top_rgb = rgb(*top);
            if blocks {
                let bottom = if y.saturating_add(1) < image.height() {
                    *image.get_pixel(x, y.saturating_add(1))
                } else {
                    Rgba([0; 4])
                };
                if depth == Depth::None {
                    output.push(
                        match (brightness(top_rgb) >= 64, brightness(rgb(bottom)) >= 64) {
                            (false, false) => ' ',
                            (true, false) => '▀',
                            (false, true) => '▄',
                            (true, true) => '█',
                        },
                    );
                } else {
                    output.push_str("\x1b[0m");
                    match (top[3] != 0, bottom[3] != 0) {
                        (true, true) => {
                            paint(&mut output, top_rgb, false, depth);
                            paint(&mut output, rgb(bottom), true, depth);
                            output.push('▀');
                        }
                        (true, false) => {
                            paint(&mut output, top_rgb, false, depth);
                            output.push('▀');
                        }
                        (false, true) => {
                            paint(&mut output, rgb(bottom), false, depth);
                            output.push('▄');
                        }
                        (false, false) => output.push(' '),
                    }
                }
            } else {
                if depth != Depth::None {
                    paint(&mut output, top_rgb, false, depth);
                }
                output.push(shade(
                    brightness(top_rgb),
                    &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'],
                ));
            }
        }
        if depth != Depth::None {
            output.push_str("\x1b[0m");
        }
        output.push('\n');
    }
    output
}

fn shade(light: u16, ramp: &[char]) -> char {
    let index = usize::from(light).saturating_mul(ramp.len().saturating_sub(1)) / 255;
    ramp.get(index).copied().unwrap_or(' ')
}
