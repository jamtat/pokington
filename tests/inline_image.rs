#![cfg(unix)]

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{Rgba, RgbaImage};
use pokington::{
    art::{Geometry, Width},
    inline_image::NativeFrame,
};
use ratatui_image::picker::{Picker, ProtocolType};
use std::collections::BTreeMap;

#[path = "../examples/support/calibration.rs"]
mod calibration;
#[path = "../examples/support/write_fault.rs"]
mod write_fault;

fn picker(protocol: ProtocolType) -> Picker {
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(protocol);
    picker
}

fn kitty_pixels(frame: &NativeFrame) -> Option<(BTreeMap<String, String>, Vec<u8>)> {
    let text = std::str::from_utf8(frame.bytes()).ok()?;
    let mut header = None;
    let mut bytes = Vec::new();
    for part in text.split("\x1b_G").skip(1) {
        let (command, _) = part.split_once("\x1b\\")?;
        let (fields, encoded) = command.split_once(';')?;
        let fields: BTreeMap<_, _> = fields
            .split(',')
            .map(|s| s.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
            .collect::<Option<_>>()?;
        header.get_or_insert(fields);
        bytes.extend(STANDARD.decode(encoded).ok()?);
    }
    Some((header?, bytes))
}

#[test]
fn library_kitty_original_preserves_every_cropped_pixel_and_transparent_padding() {
    let image = calibration::calibration_image();
    let frame = NativeFrame::render(
        &image,
        Width::Original,
        Geometry::default(),
        &picker(ProtocolType::Kitty),
    )
    .unwrap();
    let (header, bytes) = kitty_pixels(&frame).unwrap();
    assert_eq!(header["s"], "140");
    assert_eq!(header["v"], "100");
    assert_eq!(header["q"], "2");
    let decoded = RgbaImage::from_raw(140, 100, bytes).unwrap();
    for (x, y, pixel) in decoded.enumerate_pixels() {
        if x < 131 && y < 97 {
            assert_eq!(pixel, image.get_pixel(x + 10, y + 10));
        } else {
            assert_eq!(*pixel, Rgba([0; 4]));
        }
    }
    assert_eq!(
        (frame.report.image_columns, frame.report.image_rows),
        (14, 5)
    );
    assert!(
        !frame
            .bytes()
            .windows(8)
            .any(|bytes| bytes == b"\x1b[?1049h")
    );
}

#[test]
fn library_protocols_all_render_inline_with_fit_and_numeric_sizes() {
    let image = calibration::calibration_image();
    let geometry = Geometry {
        columns: 10,
        rows: 4,
        ..Geometry::default()
    };
    for protocol in [
        ProtocolType::Kitty,
        ProtocolType::Sixel,
        ProtocolType::Iterm2,
    ] {
        for (width, pixels) in [
            (Width::Columns(8), (80, 60)),
            (Width::Fit, (100, 80)),
            (Width::Original, (140, 100)),
        ] {
            let frame = NativeFrame::render(&image, width, geometry, &picker(protocol)).unwrap();
            assert_eq!(
                (frame.report.payload_width, frame.report.payload_height),
                pixels
            );
            let text = std::str::from_utf8(frame.bytes()).unwrap();
            match protocol {
                ProtocolType::Kitty => assert!(text.contains("\x1b_G")),
                ProtocolType::Sixel => assert!(text.contains("\x1bP")),
                ProtocolType::Iterm2 => assert!(text.contains("\x1b]1337;File=")),
                ProtocolType::Halfblocks => panic!("native case unexpectedly selected text"),
            }
            if protocol == ProtocolType::Sixel {
                // Only reserve the visible viewport and let Sixel perform its own scrolling.
                assert_eq!(
                    text.matches("\r\n").count(),
                    usize::from(frame.report.image_rows.min(4))
                );
                assert!(text.ends_with("\x1b[0m\r"));
                assert!(!text.contains("\x1b7") && !text.contains("\x1b8"));
            } else {
                assert!(text.ends_with("\x1b[0m\r\n"));
                assert_eq!(
                    text.matches("\x1b7").count(),
                    usize::from(frame.report.image_rows)
                );
                assert_eq!(
                    text.matches("\x1b8\x1b[0m\r\n").count(),
                    usize::from(frame.report.image_rows)
                );
            }
        }
    }
}

#[test]
fn unsupported_or_transparent_images_fail_before_output() {
    let image = calibration::calibration_image();
    assert!(
        NativeFrame::render(
            &image,
            Width::Original,
            Geometry::default(),
            &Picker::halfblocks()
        )
        .unwrap_err()
        .to_string()
        .contains("no detected image protocol")
    );
    assert!(
        NativeFrame::render(
            &RgbaImage::new(2, 2),
            Width::Original,
            Geometry::default(),
            &picker(ProtocolType::Kitty)
        )
        .unwrap_err()
        .to_string()
        .contains("fully transparent")
    );
    let wide = RgbaImage::from_pixel(2980, 1, Rgba([255; 4]));
    assert!(
        NativeFrame::render(
            &wide,
            Width::Original,
            Geometry::default(),
            &picker(ProtocolType::Kitty)
        )
        .unwrap_err()
        .to_string()
        .contains("297-cell")
    );
}

#[test]
fn interrupted_inline_output_cancels_escape_and_restores_style() {
    let frame = NativeFrame::render(
        &calibration::calibration_image(),
        Width::Original,
        Geometry::default(),
        &picker(ProtocolType::Kitty),
    )
    .unwrap();
    let mut writer = write_fault::FailOnce::inside_first_apc(Vec::new(), frame.bytes()).unwrap();
    assert!(frame.write_to(&mut writer).is_err());
    assert!(writer.report().injected && writer.report().cleanup_complete);
}

#[test]
fn library_query_child() {
    let Ok(expected) = std::env::var("POKINGTON_TEST_PROTOCOL") else {
        return;
    };
    let picker = pokington::inline_image::query_picker().unwrap();
    assert_eq!(format!("{:?}", picker.protocol_type()), expected);
    assert_eq!(
        (picker.font_size().width, picker.font_size().height),
        (10, 20)
    );
}

#[test]
fn library_detects_protocol_responses_over_ssh_and_restores_terminal_on_timeout() {
    use nix::pty::{Winsize, openpty};
    use rustix::termios;
    use std::{
        fs::File,
        io::{Read, Write},
        process::{Command, Stdio},
    };
    for (expected, program, reply) in [
        (
            "Kitty",
            "unrecognised-client",
            "\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[0n",
        ),
        (
            "Sixel",
            "unrecognised-client",
            "\x1b[?62;4c\x1b[6;20;10t\x1b[0n",
        ),
        ("Iterm2", "iTerm.app", "\x1b[6;20;10t\x1b[0n"),
        ("Halfblocks", "unrecognised-client", "\x1b[0n"),
        ("Halfblocks", "unrecognised-client", ""),
    ] {
        let pty = openpty(
            Some(&Winsize {
                ws_col: 80,
                ws_row: 24,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .unwrap();
        let mut master = File::from(pty.master);
        let slave = File::from(pty.slave);
        let before = termios::tcgetattr(&slave).unwrap();
        let input = slave.try_clone().unwrap();
        let output = slave.try_clone().unwrap();
        let peer = std::thread::spawn(move || {
            let mut captured = Vec::new();
            let mut replied = false;
            let mut buffer = [0_u8; 8192];
            loop {
                match master.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        captured.extend_from_slice(&buffer[..n]);
                        if !replied && captured.windows(4).any(|part| part == b"\x1b[5n") {
                            master.write_all(reply.as_bytes()).unwrap();
                            replied = true;
                        }
                    }
                    Err(error) if error.raw_os_error() == Some(5) => break,
                    Err(error) => panic!("terminal peer: {error}"),
                }
            }
            captured
        });
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "library_query_child", "--quiet", "--nocapture"])
            .env("POKINGTON_TEST_PROTOCOL", expected)
            .env("TERM", "xterm-256color")
            .env("TERM_PROGRAM", program)
            .env("SSH_CONNECTION", "test-session")
            .env_remove("TMUX")
            .env_remove("STY")
            .env_remove("WEZTERM_EXECUTABLE")
            .env_remove("KONSOLE_VERSION")
            .env_remove("LC_TERMINAL")
            .stdin(Stdio::from(input))
            .stdout(Stdio::from(output))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        let after = termios::tcgetattr(&slave).unwrap();
        assert_terminal_restored(before, after);
        drop(slave);
        let captured = peer.join().unwrap();
        assert!(
            result.status.success(),
            "{expected}: {} {}",
            String::from_utf8_lossy(&result.stderr),
            String::from_utf8_lossy(&captured)
        );
        assert!(captured.windows(4).any(|part| part == b"\x1b[5n"));
    }
}

fn assert_terminal_restored(
    mut before: rustix::termios::Termios,
    mut after: rustix::termios::Termios,
) {
    use rustix::termios::{LocalModes, SpecialCodeIndex as Code};

    // PENDIN is a kernel input-reprocessing marker, not an application terminal setting.
    before.local_modes.remove(LocalModes::PENDIN);
    after.local_modes.remove(LocalModes::PENDIN);
    // Compare terminal settings, not libc's unused termios storage on musl.
    assert_eq!(after.input_modes, before.input_modes);
    assert_eq!(after.output_modes, before.output_modes);
    assert_eq!(after.control_modes, before.control_modes);
    assert_eq!(after.local_modes, before.local_modes);
    for code in [
        Code::VINTR,
        Code::VQUIT,
        Code::VERASE,
        Code::VKILL,
        Code::VEOF,
        Code::VTIME,
        Code::VMIN,
        Code::VSTART,
        Code::VSTOP,
        Code::VSUSP,
        Code::VEOL,
        Code::VREPRINT,
        Code::VDISCARD,
        Code::VWERASE,
        Code::VLNEXT,
        Code::VEOL2,
        #[cfg(target_os = "linux")]
        Code::VSWTC,
        #[cfg(target_os = "macos")]
        Code::VDSUSP,
        #[cfg(target_os = "macos")]
        Code::VSTATUS,
    ] {
        assert_eq!(
            after.special_codes[code], before.special_codes[code],
            "{code:?}"
        );
    }
    assert_eq!(after.input_speed(), before.input_speed());
    assert_eq!(after.output_speed(), before.output_speed());
    #[cfg(target_os = "linux")]
    assert_eq!(after.line_discipline, before.line_discipline);
}
