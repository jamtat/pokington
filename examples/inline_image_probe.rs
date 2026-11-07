//! S01 manual feasibility probe, not the production `art` command.
use std::{
    env,
    error::Error,
    fs,
    io::{self, Cursor, IsTerminal},
    path::PathBuf,
    time::Instant,
};

use image::{DynamicImage, ImageFormat, ImageReader};
use pokington::inline_image::{self, Mode, NativeFrame, Selection, TerminalHints};
use serde_json::json;

#[path = "support/calibration.rs"]
mod calibration;
#[path = "support/write_fault.rs"]
mod write_fault;

#[derive(Default)]
struct Options {
    image: Option<PathBuf>,
    record: Option<PathBuf>,
    payload: Option<PathBuf>,
    fixture: Option<PathBuf>,
    explicit_img: bool,
    fail_mid_write: bool,
}

fn options() -> Result<Options, Box<dyn Error>> {
    let mut options = Options::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--fail-mid-write" {
            options.fail_mid_write = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {arg}"))?;
        match arg.as_str() {
            "--image" => options.image = Some(value.into()),
            "--record" => options.record = Some(value.into()),
            "--payload" => options.payload = Some(value.into()),
            "--fixture" => options.fixture = Some(value.into()),
            "--mode" => match value.as_str() { "img" => options.explicit_img = true, "auto" => (), _ => return Err("probe mode must be auto or img".into()) },
            "--width" if value == "original" => (),
            _ => return Err(format!("unknown probe option {arg} {value}; supports --mode auto|img --width original --image PNG --record JSON --payload BIN --fixture PNG --fail-mid-write").into()),
        }
    }
    Ok(options)
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = options()?;
    let hints = TerminalHints::from_env();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let before = nix::sys::termios::tcgetattr(&stdin).ok();
    let started = Instant::now();
    let detection = inline_image::detect(&stdin, &stdout, &hints);
    let detection_micros = started.elapsed().as_micros();
    let mode = if options.explicit_img {
        Mode::Img
    } else {
        Mode::Auto
    };
    let unicode_terminal = stdout.is_terminal()
        && hints.term != "dumb"
        && ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|key| env::var(key).ok().filter(|s| !s.is_empty()))
            .is_some_and(|s| {
                s.to_ascii_lowercase().contains("utf-8") || s.to_ascii_lowercase().contains("utf8")
            });
    let selection = inline_image::select(mode, detection, unicode_terminal);
    let ioctl_window = stdout
        .is_terminal()
        .then(|| rustix::termios::tcgetwinsize(&stdout).ok())
        .flatten()
        .map(|size| {
            json!({ "columns": size.ws_col, "rows": size.ws_row,
            "pixel_width": size.ws_xpixel, "pixel_height": size.ws_ypixel })
        });
    let mut report = json!({ "platform": env::consts::OS, "hints": hints,
        "stdin_is_terminal": stdin.is_terminal(), "stdout_is_terminal": stdout.is_terminal(),
        "ioctl_window": ioctl_window, "native_emitted": false,
        "output_state": "not written",
        "detection_microseconds": detection_micros, "query_bytes": 0, "input_bytes_read": 0,
        "termios_changed_by_probe": false, "alternate_screen_entered": false,
        "cursor_hidden": false, "visual_confirmation": "pending user observation",
        "mode": mode });
    let result = perform(selection, &options, &stdout, &mut report);
    let after = nix::sys::termios::tcgetattr(&stdin).ok();
    insert_report(
        &mut report,
        "termios_equal_before_after",
        json!(before.zip(after).map(|(a, b)| a == b)),
    )?;
    insert_report(
        &mut report,
        "result",
        json!(match &result {
            Ok(()) => "ok".into(),
            Err(error) => error.to_string(),
        }),
    )?;
    if let Some(path) = options.record {
        fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
        eprintln!("Probe report: {}", path.display());
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&report)?);
    }
    result
}

fn perform(
    selection: Result<Selection, io::Error>,
    options: &Options,
    stdout: &io::Stdout,
    report: &mut serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    let selection = selection?;
    insert_report(report, "selection", serde_json::to_value(&selection)?)?;
    match selection {
        Selection::Fallback { renderer, reason } => {
            // Intentionally only demonstrate the decision. S04 implements actual text art.
            println!("fallback={renderer:?}: {reason} (text artwork is outside this probe)");
        }
        Selection::Native(terminal) => {
            let image = if let Some(path) = &options.image {
                insert_report(report, "image", json!(path))?;
                let mut reader = ImageReader::open(path)?;
                reader.set_format(ImageFormat::Png);
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(4096);
                limits.max_image_height = Some(4096);
                limits.max_alloc = Some(64 * 1024 * 1024);
                reader.limits(limits);
                reader.decode()?.to_rgba8()
            } else {
                let mut png = Cursor::new(Vec::new());
                DynamicImage::ImageRgba8(calibration::calibration_image())
                    .write_to(&mut png, ImageFormat::Png)?;
                if let Some(path) = &options.fixture {
                    fs::write(path, png.get_ref())?;
                }
                insert_report(
                    report,
                    "image",
                    json!("generated 151x117 calibration PNG; visible crop 131x97"),
                )?;
                image::load_from_memory_with_format(png.get_ref(), ImageFormat::Png)?.to_rgba8()
            };
            let frame = NativeFrame::original(&image, terminal.metrics)?;
            insert_report(report, "frame", serde_json::to_value(&frame.report)?)?;
            if let Some(path) = &options.payload {
                fs::write(path, frame.bytes())?;
            }
            let output = if options.fail_mid_write {
                let mut writer =
                    write_fault::FailOnce::inside_first_apc(stdout.lock(), frame.bytes())?;
                let output = frame.write_to(&mut writer);
                insert_report(
                    report,
                    "write_fault",
                    serde_json::to_value(writer.report())?,
                )?;
                output
            } else {
                frame.write_to(&mut stdout.lock())
            };
            insert_report(
                report,
                "output_state",
                json!(if output.is_ok() {
                    "inline write completed and styles reset"
                } else {
                    "write failed; escape/style cleanup attempted; see result for any cleanup failure"
                }),
            )?;
            output?;
            insert_report(report, "native_emitted", json!(true))?;
        }
    }
    Ok(())
}

fn insert_report(
    report: &mut serde_json::Value,
    key: &str,
    value: serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    report
        .as_object_mut()
        .ok_or("probe report is not a JSON object")?
        .insert(key.to_owned(), value);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
