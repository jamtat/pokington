#![cfg(unix)]

use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read, Write},
    os::fd::AsFd,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{Rgba, RgbaImage};
use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    pty::{OpenptyResult, Winsize, openpty},
    sys::termios::{self, LocalFlags, OutputFlags, SetArg},
};
use pokington::inline_image::{
    self, Fallback, Metrics, Mode, NativeFrame, Selection, TerminalHints, Unavailable,
};

#[path = "../examples/support/calibration.rs"]
mod calibration;
#[path = "../examples/support/write_fault.rs"]
mod write_fault;

fn hints() -> TerminalHints {
    TerminalHints {
        term: "xterm-ghostty".into(),
        program: "ghostty".into(),
        version: "1.4.0".into(),
        ..Default::default()
    }
}

const fn metrics() -> Metrics {
    Metrics {
        columns: 80,
        rows: 24,
        pixel_width: 640,
        pixel_height: 384,
        cell_width: 8,
        cell_height: 16,
    }
}

fn pty(size: Winsize) -> Result<OpenptyResult, nix::errno::Errno> {
    let pty = openpty(Some(&size), None)?;
    // This is our own test PTY. Suppress input echo and output newline translation so
    // assertions can distinguish queries from terminal echo, and inspect exact bytes.
    let mut state = termios::tcgetattr(&pty.slave)?;
    state.local_flags.remove(LocalFlags::ECHO);
    state.output_flags.remove(OutputFlags::OPOST);
    termios::tcsetattr(&pty.slave, SetArg::TCSANOW, &state)?;
    Ok(pty)
}

const fn size() -> Winsize {
    Winsize {
        ws_col: 80,
        ws_row: 24,
        ws_xpixel: 640,
        ws_ypixel: 384,
    }
}

fn nonblocking(fd: &impl AsFd) -> Result<(), nix::errno::Errno> {
    let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL)?);
    fcntl(fd, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK))?;
    Ok(())
}

#[test]
fn paired_hints_and_ioctl_are_bounded_without_queries_or_consuming_queued_input() {
    let pty = pty(size()).unwrap();
    let mut master = File::from(pty.master);
    let mut slave = File::from(pty.slave);
    nonblocking(&master).unwrap();
    nonblocking(&slave).unwrap();
    master.write_all(b"pending user input\n").unwrap();
    let before = termios::tcgetattr(&slave).unwrap();
    let started = Instant::now();
    let terminal = inline_image::detect(&slave, &slave, &hints()).unwrap();
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(terminal.metrics, metrics());
    assert_eq!(termios::tcgetattr(&slave).unwrap(), before);
    let mut bytes = [0; 128];
    assert_eq!(
        master.read(&mut bytes).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    let count = slave.read(&mut bytes).unwrap();
    assert_eq!(&bytes[..count], b"pending user input\n");
}

#[test]
fn unavailable_contexts_do_not_query_or_read_and_choose_explicit_error_or_safe_fallback() {
    let other = pty(size()).unwrap();
    let pty = pty(size()).unwrap();
    let mut master = File::from(pty.master);
    let mut slave = File::from(pty.slave);
    let null = File::open("/dev/null").unwrap();
    nonblocking(&master).unwrap();
    nonblocking(&slave).unwrap();
    master.write_all(b"untouched\n").unwrap();
    let before = termios::tcgetattr(&slave).unwrap();
    let checks = [
        (
            inline_image::detect(&slave, &null, &hints()),
            Unavailable::RedirectedOutput,
        ),
        (
            inline_image::detect(&null, &slave, &hints()),
            Unavailable::UnavailableInput,
        ),
        (
            inline_image::detect(&other.slave, &slave, &hints()),
            Unavailable::DifferentTerminals,
        ),
        (
            inline_image::detect(
                &slave,
                &slave,
                &TerminalHints {
                    program: String::new(),
                    ..hints()
                },
            ),
            Unavailable::UnknownIdentity,
        ),
        (
            inline_image::detect(
                &slave,
                &slave,
                &TerminalHints {
                    multiplexed: true,
                    ..hints()
                },
            ),
            Unavailable::MultiplexerOrRemote,
        ),
        (
            inline_image::detect(
                &slave,
                &slave,
                &TerminalHints {
                    remote: true,
                    ..hints()
                },
            ),
            Unavailable::MultiplexerOrRemote,
        ),
    ];
    for (detection, expected) in checks {
        assert_eq!(detection.unwrap_err(), expected);
        let error = inline_image::select(Mode::Img, Err(expected), true).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(error.to_string().contains("--mode auto, blocks, or ascii"));
        let selection = inline_image::select(Mode::Auto, Err(expected), true).unwrap();
        let Selection::Fallback { renderer, reason } = selection else {
            panic!("expected fallback")
        };
        assert_eq!(reason, expected);
        assert_eq!(
            renderer,
            if expected == Unavailable::RedirectedOutput {
                Fallback::Ascii
            } else {
                Fallback::Blocks
            }
        );
    }
    assert!(matches!(
        inline_image::select(Mode::Auto, Err(Unavailable::UnknownIdentity), false).unwrap(),
        Selection::Fallback {
            renderer: Fallback::Ascii,
            ..
        }
    ));
    assert_eq!(termios::tcgetattr(&slave).unwrap(), before);
    let mut bytes = [0; 128];
    assert_eq!(
        master.read(&mut bytes).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(slave.read(&mut bytes).unwrap(), 10);
    assert_eq!(&bytes[..10], b"untouched\n");
}

#[test]
fn missing_metrics_fail_without_waiting_for_terminal_replies() {
    let cases = [
        Winsize {
            ws_xpixel: 0,
            ..size()
        },
        Winsize {
            ws_ypixel: 0,
            ..size()
        },
        Winsize {
            ws_col: 0,
            ..size()
        },
        Winsize {
            ws_row: 0,
            ..size()
        },
        Winsize {
            ws_xpixel: 40,
            ..size()
        },
    ];
    for size in cases {
        let pty = pty(size).unwrap();
        let started = Instant::now();
        assert_eq!(
            inline_image::detect(&pty.slave, &pty.slave, &hints()).unwrap_err(),
            Unavailable::MissingPixelMetrics
        );
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}

#[test]
fn ghostty_140_observed_viewport_has_unique_cells_and_ambiguous_or_clamped_sizes_fail() {
    let hints = TerminalHints {
        version: "1.4.0".into(),
        ..hints()
    };
    let observed = Winsize {
        ws_col: 105,
        ws_row: 59,
        ws_xpixel: 1801,
        ws_ypixel: 2196,
    };
    let pty = pty(observed).unwrap();
    let terminal = inline_image::detect(&pty.slave, &pty.slave, &hints).unwrap();
    assert_eq!(
        (terminal.metrics.cell_width, terminal.metrics.cell_height),
        (17, 37)
    );
    assert!(
        terminal
            .cell_measurement
            .contains("unique integer solution")
    );
    assert_eq!(terminal.cell_width_interval, [17, 17]);
    assert_eq!(terminal.cell_height_interval, [37, 37]);
    assert_eq!(
        (
            terminal.viewport_remainder_width,
            terminal.viewport_remainder_height
        ),
        (16, 13)
    );
    let image = calibration::calibration_image();
    let frame = NativeFrame::original(&image, terminal.metrics).unwrap();
    let (header, payload) = transmitted_pixels(&frame).unwrap();
    assert_eq!((header["s"].as_str(), header["v"].as_str()), ("136", "111"));
    assert_eq!(
        (frame.report.image_columns, frame.report.image_rows),
        (8, 3)
    );
    let decoded = RgbaImage::from_raw(136, 111, payload).unwrap();
    for (x, y, pixel) in decoded.enumerate_pixels() {
        if x < 131 && y < 97 {
            assert_eq!(pixel, image.get_pixel(x + 10, y + 10));
        } else {
            assert_eq!(pixel, &Rgba([0, 0, 0, 0]));
        }
    }
    // Even exact division is insufficient when multiple cell dimensions could yield
    // the reported grid. One-cell minimums and saturated ioctl fields are also ambiguous.
    for invalid in [
        Winsize {
            ws_col: 2,
            ws_xpixel: 300,
            ..observed
        },
        Winsize {
            ws_row: 2,
            ws_ypixel: 300,
            ..observed
        },
        Winsize {
            ws_col: 1,
            ..observed
        },
        Winsize {
            ws_row: 1,
            ..observed
        },
        Winsize {
            ws_xpixel: u16::MAX,
            ..observed
        },
        Winsize {
            ws_ypixel: u16::MAX,
            ..observed
        },
    ] {
        let other = openpty(Some(&invalid), None).unwrap();
        assert_eq!(
            inline_image::detect(&other.slave, &other.slave, &hints).unwrap_err(),
            Unavailable::MissingPixelMetrics
        );
    }
    let unverified = TerminalHints {
        version: "1.4.1".into(),
        ..hints
    };
    assert_eq!(
        inline_image::detect(&pty.slave, &pty.slave, &unverified).unwrap_err(),
        Unavailable::UnverifiedMeasurementProfile
    );
}

#[test]
fn unverified_profiles_cannot_turn_divisible_ambiguous_metrics_into_native_sizing() {
    let pty = pty(Winsize {
        ws_col: 105,
        ws_row: 2,
        ws_xpixel: 1785,
        ws_ypixel: 100,
    })
    .unwrap();
    let mut master = File::from(pty.master);
    let mut slave = File::from(pty.slave);
    nonblocking(&master).unwrap();
    nonblocking(&slave).unwrap();
    master.write_all(b"preserved profile input\n").unwrap();
    let before = termios::tcgetattr(&slave).unwrap();
    for profile in [
        TerminalHints {
            version: String::new(),
            ..hints()
        },
        TerminalHints {
            version: "1.4.1".into(),
            ..hints()
        },
        TerminalHints {
            version: "unknown".into(),
            ..hints()
        },
        TerminalHints {
            term: "xterm-kitty".into(),
            program: "kitty".into(),
            version: "0.45.0".into(),
            kitty_window: true,
            ..Default::default()
        },
    ] {
        let reason = inline_image::detect(&slave, &slave, &profile).unwrap_err();
        assert_eq!(reason, Unavailable::UnverifiedMeasurementProfile);
        assert!(inline_image::select(Mode::Img, Err(reason), true).is_err());
        assert!(matches!(
            inline_image::select(Mode::Auto, Err(reason), true).unwrap(),
            Selection::Fallback {
                renderer: Fallback::Blocks,
                ..
            }
        ));
    }
    // The verified profile also rejects this tuple: 100 / 2 allows cell heights 34..=50.
    assert_eq!(
        inline_image::detect(&slave, &slave, &hints()).unwrap_err(),
        Unavailable::MissingPixelMetrics
    );
    assert_eq!(termios::tcgetattr(&slave).unwrap(), before);
    let mut bytes = [0; 128];
    assert_eq!(
        master.read(&mut bytes).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    let count = slave.read(&mut bytes).unwrap();
    assert_eq!(&bytes[..count], b"preserved profile input\n");
}

fn transmitted_pixels(frame: &NativeFrame) -> Option<(BTreeMap<String, String>, Vec<u8>)> {
    let text = std::str::from_utf8(frame.bytes()).ok()?;
    let mut first = None;
    let mut payload = Vec::new();
    let mut parts = text.split("\x1b_G").skip(1).peekable();
    while let Some(part) = parts.next() {
        let (command, _) = part.split_once("\x1b\\")?;
        let (header, encoded) = command.split_once(';')?;
        let fields: BTreeMap<_, _> = header
            .split(',')
            .map(|f| {
                f.split_once('=')
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
            })
            .collect::<Option<_>>()?;
        assert_eq!(
            fields.get("q").map(String::as_str),
            Some("2"),
            "terminal must not send replies"
        );
        assert_eq!(
            fields.get("m").map(String::as_str),
            Some(if parts.peek().is_some() { "1" } else { "0" })
        );
        assert!(encoded.len() <= 4096);
        assert!(!fields.contains_key("c") && !fields.contains_key("r"));
        assert!(
            !fields.contains_key("o"),
            "compression must not require a capability probe"
        );
        payload.extend(STANDARD.decode(encoded).ok()?);
        first.get_or_insert(fields);
    }
    (first?, payload).into()
}

#[test]
fn odd_crop_rgba_payload_matches_every_source_pixel_and_full_cell_placement_without_resampling() {
    let mut png = io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(calibration::calibration_image())
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let image = image::load_from_memory_with_format(png.get_ref(), image::ImageFormat::Png)
        .unwrap()
        .to_rgba8();
    for (cell_width, cell_height) in [(8, 16), (11, 23), (20, 40), (1, 1)] {
        let metrics = Metrics {
            cell_width,
            cell_height,
            ..metrics()
        };
        let frame = NativeFrame::original(&image, metrics).unwrap();
        assert_eq!(
            (
                frame.report.crop.left,
                frame.report.crop.top,
                frame.report.crop.width,
                frame.report.crop.height
            ),
            (10, 10, 131, 97)
        );
        let (header, raw) = transmitted_pixels(&frame).unwrap();
        assert_eq!(header["a"], "T");
        assert_eq!(header["U"], "1");
        assert_eq!(header["f"], "32");
        assert_eq!(header["t"], "d");
        assert_ne!(header["i"], "0");
        let width: u32 = header["s"].parse().unwrap();
        let height: u32 = header["v"].parse().unwrap();
        assert_eq!(
            width,
            u32::from(frame.report.image_columns) * u32::from(cell_width)
        );
        assert_eq!(
            height,
            u32::from(frame.report.image_rows) * u32::from(cell_height)
        );
        let expected_len = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|area| area.checked_mul(4))
            .expect("decoded PNG dimensions fit in memory");
        assert_eq!(raw.len(), expected_len);
        let decoded = RgbaImage::from_raw(width, height, raw).unwrap();
        for (x, y, pixel) in decoded.enumerate_pixels() {
            if x < 131 && y < 97 {
                assert_eq!(pixel, image.get_pixel(x + 10, y + 10));
            } else {
                assert_eq!(pixel, &Rgba([0, 0, 0, 0]));
            }
        }
    }
}

#[test]
fn original_overflow_clips_placeholders_without_reducing_payload_or_scaling() {
    let frame = NativeFrame::original(
        &calibration::calibration_image(),
        Metrics {
            columns: 5,
            rows: 2,
            ..metrics()
        },
    )
    .unwrap();
    assert_eq!(
        (
            frame.report.image_columns,
            frame.report.image_rows,
            frame.report.printed_columns
        ),
        (17, 7, 5)
    );
    let (header, _) = transmitted_pixels(&frame).unwrap();
    assert_eq!((header["s"].as_str(), header["v"].as_str()), ("136", "112"));
    let text = std::str::from_utf8(frame.bytes()).unwrap();
    assert_eq!(text.matches('\u{10eeee}').count(), 5 * 7);
    assert_eq!(text.matches("\x1b[0m\r\n").count(), 7);
    assert!(!text.contains("\x1b[?")); // no alternate-screen, cursor hiding, or mode commands
}

#[test]
fn partial_alpha_is_preserved_and_transparent_or_unsupported_extents_fail_before_output() {
    let mut image = RgbaImage::new(9, 9);
    image.put_pixel(2, 3, Rgba([17, 42, 99, 1]));
    image.put_pixel(4, 6, Rgba([200, 0, 9, 128]));
    let frame = NativeFrame::original(&image, metrics()).unwrap();
    let (_, payload) = transmitted_pixels(&frame).unwrap();
    let decoded = RgbaImage::from_raw(8, 16, payload).unwrap();
    assert_eq!(decoded.get_pixel(0, 0), image.get_pixel(2, 3));
    assert_eq!(decoded.get_pixel(2, 3), image.get_pixel(4, 6));
    assert!(NativeFrame::original(&RgbaImage::new(9, 9), metrics()).is_err());
    let oversized = RgbaImage::from_pixel(2384, 1, Rgba([1, 2, 3, 255]));
    assert!(NativeFrame::original(&oversized, metrics()).is_err());
    assert!(
        NativeFrame::original(
            &image,
            Metrics {
                cell_width: u16::MAX,
                cell_height: u16::MAX,
                ..metrics()
            }
        )
        .is_err()
    );
}

#[test]
fn success_and_partial_write_failure_preserve_pty_modes_input_and_restore_output_parser() {
    let image = RgbaImage::from_pixel(3, 3, Rgba([255, 0, 0, 255]));
    let frame = NativeFrame::original(&image, metrics()).unwrap();
    let apc_fault = write_fault::FailOnce::inside_first_apc(io::sink(), frame.bytes())
        .unwrap()
        .report()
        .prefix_bytes;
    for fail_after in [
        None,
        Some(1),
        Some(apc_fault),
        Some(frame.bytes().len() - 2),
    ] {
        let pty = pty(size()).unwrap();
        let mut master = File::from(pty.master);
        let mut slave = File::from(pty.slave);
        nonblocking(&master).unwrap();
        nonblocking(&slave).unwrap();
        master.write_all(b"next command\n").unwrap();
        let before = termios::tcgetattr(&slave).unwrap();
        inline_image::detect(&slave, &slave, &hints()).unwrap();
        if let Some(remaining) = fail_after {
            let mut writer = write_fault::FailOnce::after_bytes(&mut slave, remaining);
            assert!(frame.write_to(&mut writer).is_err());
            let report = writer.report();
            assert!(report.injected && report.cleanup_complete);
            assert_eq!(report.forwarded_before_failure, remaining);
            assert_eq!(report.forwarded_after_failure, 9);
        } else {
            frame.write_to(&mut slave).unwrap();
        }
        assert_eq!(termios::tcgetattr(&slave).unwrap(), before);
        let mut input = [0; 128];
        let count = slave.read(&mut input).unwrap();
        assert_eq!(&input[..count], b"next command\n");
        let mut output = [0; 4096];
        let count = master.read(&mut output).unwrap();
        let output = &output[..count];
        if fail_after.is_some() {
            assert!(output.ends_with(b"\x18\x1b\\\x1b[0m\r\n"));
        } else {
            assert_eq!(output, frame.bytes());
        }
        assert!(!output.windows(3).any(|x| x == b"\x1b[?"));
    }
}

#[test]
fn permanently_broken_output_reports_cleanup_failure() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }
    let frame = NativeFrame::original(&calibration::calibration_image(), metrics()).unwrap();
    let error = frame.write_to(&mut Broken).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert!(error.to_string().contains("cleanup also failed"));
}
