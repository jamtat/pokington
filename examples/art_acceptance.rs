//! Run the real command runner in the current terminal and capture acceptance evidence.
//! Usage: `cargo run --example art_acceptance -- ABSOLUTE_STORE ABSOLUTE_EVIDENCE_DIR`

use pokington::{
    command::{self, Runtime},
    store::Store,
    terminal::Terminal,
};
use rand::{SeedableRng, rngs::StdRng};
use serde_json::json;
use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

const CASES: [(&str, &[&str]); 5] = [
    ("auto-fit", &["pokington", "art", "Charizard"]),
    (
        "img-numeric",
        &[
            "pokington",
            "art",
            "Charizard",
            "--mode",
            "img",
            "--width",
            "12",
        ],
    ),
    (
        "img-original",
        &[
            "pokington",
            "art",
            "Charizard",
            "--mode",
            "img",
            "--width",
            "original",
        ],
    ),
    (
        "blocks",
        &[
            "pokington",
            "art",
            "Meowth",
            "--form",
            "galarian",
            "--mode",
            "blocks",
            "--width",
            "40",
            "--color",
            "never",
        ],
    ),
    (
        "ascii",
        &[
            "pokington",
            "art",
            "Charizard",
            "--form",
            "mega x",
            "--mode",
            "ascii",
            "--width",
            "40",
            "--color",
            "never",
        ],
    ),
];

struct Capture<W> {
    output: W,
    bytes: Vec<u8>,
}
impl<W: Write> Write for Capture<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.output.write(bytes)?;
        self.bytes.extend_from_slice(
            bytes
                .get(..written)
                .ok_or_else(|| io::Error::other("invalid writer length"))?,
        );
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let store = Store::at(PathBuf::from(args.next().ok_or("missing store path")?))?;
    let record = PathBuf::from(args.next().ok_or("missing evidence directory")?);
    if args.next().is_some() {
        return Err("expected STORE EVIDENCE_DIR".into());
    }
    store
        .load_active()?
        .ok_or("acceptance requires an already installed complete catalogue")?;
    fs::create_dir_all(&record)?;
    let terminal = Terminal::from_stdio();
    let stdin = io::stdin();
    let before = nix::sys::termios::tcgetattr(&stdin).ok();
    let picker = if terminal.is_terminal {
        Some(pokington::inline_image::query_picker()?)
    } else {
        None
    };
    let mut results = Vec::new();
    let mut failed = false;
    for (label, words) in CASES {
        eprintln!("\nAcceptance case: {label}");
        let mut output = Capture {
            output: io::stdout(),
            bytes: Vec::new(),
        };
        let mut stderr = Vec::new();
        let code = command::run(
            words.iter().copied(),
            || {
                let runtime =
                    Runtime::new(store.clone(), StdRng::seed_from_u64(23)).with_terminal(terminal);
                Ok(if let Some(picker) = &picker {
                    runtime.with_picker(picker.clone())
                } else {
                    runtime
                })
            },
            &mut output,
            &mut stderr,
        )
        .await;
        output.flush()?;
        io::stderr().write_all(&stderr)?;
        fs::write(record.join(format!("{label}.bin")), &output.bytes)?;
        fs::write(record.join(format!("{label}.stderr")), &stderr)?;
        results.push(json!({"case": label, "args": words, "exit": code, "stdout_bytes": output.bytes.len(), "stderr_bytes": stderr.len()}));
        failed |= code != 0;
    }
    let after = nix::sys::termios::tcgetattr(&stdin).ok();
    let report = json!({
        "platform": std::env::consts::OS, "terminal": format!("{terminal:?}"),
        "term": std::env::var("TERM").ok(), "term_program": std::env::var("TERM_PROGRAM").ok(),
        "native_detection": picker.as_ref().map(|p| format!("{:?} {:?}",p.protocol_type(),p.font_size())), "termios_equal_before_after": before.zip(after).map(|(a,b)| a == b),
        "results": results, "visual_confirmation": "pending user observation", "store": store.root(),
    });
    fs::write(
        record.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    eprintln!(
        "\nEvidence: {}. Confirm images remain inline, sizes are sensible, and the prompt/input still works.",
        record.display()
    );
    Ok(ExitCode::from(u8::from(failed)))
}
