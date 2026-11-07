#![cfg(unix)]

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use pokington::{
    catalogue::{ArtworkRef, Catalogue},
    command::{self, CommandError, Runtime},
    serebii,
    store::Store,
    terminal::Terminal,
};
use rand::{SeedableRng, rngs::StdRng};

type Failure = Box<dyn std::error::Error>;
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Result<Self, Failure> {
        loop {
            let path = std::env::temp_dir().join(format!(
                "pokington-command-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error.into()),
            }
        }
    }
    fn store(&self) -> Result<Store, Failure> {
        Ok(Store::at(self.0.join("data"))?)
    }
    fn install(&self) -> Result<Store, Failure> {
        let store = self.store()?;
        let installed = store.root().join("installed");
        fs::create_dir_all(&installed)?;
        fs::write(
            installed.join("catalogue.json"),
            serde_json::to_vec(&catalogue()?)?,
        )?;
        Ok(store)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn catalogue() -> Result<Catalogue, Failure> {
    let index = serebii::parse_index(include_str!("fixtures/serebii-index.html"))?;
    let link = index.species.first().ok_or("missing fixture species")?;
    let hub = serebii::parse_hub(include_str!("fixtures/serebii-charizard.html"), link)?;
    let mut catalogue = Catalogue::from_serebii(
        &index,
        vec![serebii::reconcile(&hub, &[])?],
        "00000000000000000000000000000001",
    )?;
    let template = catalogue
        .species
        .first()
        .ok_or("missing catalogue species")?
        .clone();
    catalogue.species = ["Mew", "Mewtwo", "Eevee", "Flabébé"]
        .into_iter()
        .zip(1_u32..)
        .map(|(name, id)| {
            let mut species = template.clone();
            species.id = id;
            species.name = name.into();
            species
        })
        .collect();
    // Unequal form counts must not change the probability of selecting a species.
    for (species, forms) in catalogue.species.iter_mut().zip(1..) {
        species.forms.sort_by_key(|form| form.key != "original");
        species.forms.truncate(forms);
    }
    catalogue.source.min_id = 1;
    catalogue.source.max_id = 4;
    for (form, ordinal) in catalogue
        .species
        .iter_mut()
        .flat_map(|species| &mut species.forms)
        .zip(1_u64..)
    {
        form.local_artwork = ArtworkRef::new(format!(
            "artwork/00000000000000000000000000000001-{ordinal}.png"
        ))?;
    }
    catalogue.validate()?;
    Ok(catalogue)
}

fn args(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}
async fn invoke(store: &Store, args: Vec<OsString>) -> Result<(u8, String, String), Failure> {
    let source = pokington::bootstrap::Bootstrap::with_http(
        pokington::source::http_client_builder()
            .https_only(false)
            .build()?,
        "http://127.0.0.1:0",
    );
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = command::run(
        args,
        || {
            // The injected output buffers represent redirected streams, even under a TTY.
            Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(23))
                .with_terminal(Terminal::default())
                .with_bootstrap(source))
        },
        &mut stdout,
        &mut stderr,
    )
    .await;
    Ok((code, String::from_utf8(stdout)?, String::from_utf8(stderr)?))
}

#[test]
fn selection_is_repeatable_distinct_and_uses_species_names_only() {
    let catalogue = catalogue().unwrap();
    let mut first = StdRng::seed_from_u64(7);
    let mut second = StdRng::seed_from_u64(7);
    let selected = command::choose(&catalogue, 3, &[], &[], &mut first).unwrap();
    assert_eq!(
        selected,
        command::choose(&catalogue, 3, &[], &[], &mut second).unwrap()
    );
    assert_eq!(selected.len(), 3);
    assert_eq!(selected.iter().collect::<BTreeSet<_>>().len(), 3);
    let all = command::choose(&catalogue, 4, &[], &[], &mut first).unwrap();
    assert_eq!(
        all.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from(["Mew", "Mewtwo", "Eevee", "Flabébé"])
    );
    assert!(matches!(
        command::choose(&catalogue, 5, &[], &[], &mut first),
        Err(CommandError::Insufficient {
            requested: 5,
            available: 4
        })
    ));
    assert_eq!(
        command::choose(&catalogue, 0, &[], &[], &mut first).unwrap(),
        Vec::<&str>::new()
    );
}

#[test]
fn seeded_sampling_reaches_all_ordered_pairs_without_form_weighting() {
    let catalogue = catalogue().unwrap();
    let mut rng = StdRng::seed_from_u64(64);
    let mut pairs = BTreeMap::<Vec<&str>, usize>::new();
    for _ in 0..2400 {
        let pair = command::choose(&catalogue, 2, &[], &[], &mut rng).unwrap();
        let count = pairs.entry(pair).or_default();
        *count = count.saturating_add(1);
    }
    assert_eq!(pairs.len(), 12);
    assert!(pairs.values().all(|count| (140..=260).contains(count)));
}

#[tokio::test]
async fn parser_handles_help_version_and_invalid_arguments_before_runtime_creation() {
    for (words, expected) in [
        (vec!["pokington", "--help"], 0),
        (vec!["pokington", "--version"], 0),
        (vec!["pokington", "choose", "--help"], 0),
        (vec!["pokington", "bootstrap", "--help"], 0),
        (vec!["pokington", "art", "--help"], 0),
        (vec!["pokington"], 2),
        (vec!["pokington", "choose"], 2),
        (vec!["pokington", "choose", "-c", "-1"], 2),
        (vec!["pokington", "choose", "-c", "no"], 2),
        (
            vec!["pokington", "choose", "-c", "9999999999999999999999999"],
            2,
        ),
        (vec!["pokington", "bootstrap", "--unknown"], 2),
        (vec!["pokington", "art", "--width", "0"], 2),
        (vec!["pokington", "art", "--width", "-1"], 2),
        (vec!["pokington", "art", "--width", "huge"], 2),
        (vec!["pokington", "art", "--mode", "invalid"], 2),
        (vec!["pokington", "art", "--color", "invalid"], 2),
        (vec!["pokington", "art", "--no-newline"], 2),
        (vec!["pokington", "art", "--title", "--no-newline"], 2),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = command::run::<_, _, StdRng>(
            args(&words),
            || panic!("parser accessed runtime: {words:?}"),
            &mut stdout,
            &mut stderr,
        )
        .await;
        assert_eq!(code, expected, "{words:?}");
        if expected == 0 {
            assert_ne!(stdout, b"");
            assert_eq!(stderr, b"");
        } else {
            assert_eq!(stdout, b"");
            assert_ne!(stderr, b"");
        }
    }
}

#[tokio::test]
async fn runner_applies_repeatable_exact_and_unicode_exclusions_and_clean_output() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    let (code, stdout, stderr) = invoke(
        &store,
        args(&["pokington", "choose", "-c", "3", "-e", "mEw"]),
    )
    .await
    .unwrap();
    assert_eq!(code, 0);
    assert_eq!(
        stdout.lines().collect::<BTreeSet<_>>(),
        BTreeSet::from(["Mewtwo", "Eevee", "Flabébé"])
    );
    assert_eq!(stderr, "");
    let (code, stdout, stderr) = invoke(
        &store,
        args(&[
            "pokington",
            "choose",
            "--count",
            "1",
            "-e",
            "mEw",
            "--exclude",
            "MEWTWO",
            "-e",
            "FLABÉBÉ",
            "-e",
            "unknown",
            "-e",
            "Mew",
        ]),
    )
    .await
    .unwrap();
    assert_eq!(code, 0);
    assert_eq!(stdout, "Eevee\n");
    assert_eq!(stderr, "");
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "choose", "-c", "4"]))
        .await
        .unwrap();
    assert_eq!(code, 0);
    assert_eq!(
        stdout.lines().collect::<BTreeSet<_>>(),
        BTreeSet::from(["Mew", "Mewtwo", "Eevee", "Flabébé"])
    );
    assert!(stdout.ends_with('\n'));
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn runner_matches_substrings_in_multiple_files_and_reports_exhaustion() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    let first = temp.0.join("first.txt");
    let second = temp.0.join("second.txt");
    fs::write(&first, "before MEWTWO after").unwrap();
    fs::write(&second, "FLABÉBÉ").unwrap();
    let mut words = args(&["pokington", "choose", "-c", "1"]);
    words.extend([first.into_os_string(), second.into_os_string()]);
    let (code, stdout, stderr) = invoke(&store, words.clone()).await.unwrap();
    assert_eq!(code, 0);
    assert_eq!(stdout, "Eevee\n");
    assert_eq!(stderr, "");
    words.extend(args(&["-e", "EEVEE"]));
    let (code, stdout, stderr) = invoke(&store, words).await.unwrap();
    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("only 0 eligible species"));
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "choose", "-c", "5"]))
        .await
        .unwrap();
    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("cannot choose 5"));
}

#[tokio::test]
async fn runner_validates_every_file_before_output_even_for_zero() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    let valid = temp.0.join("valid.txt");
    let invalid = temp.0.join("invalid-utf8.txt");
    let missing = temp.0.join("missing.txt");
    fs::write(&valid, "unrelated").unwrap();
    fs::write(&invalid, [0xff, 0xfe]).unwrap();
    for file in [&missing, &invalid, &temp.0] {
        for count in ["0", "1"] {
            let mut words = args(&["pokington", "choose", "-c", count]);
            words.extend([
                valid.clone().into_os_string(),
                file.clone().into_os_string(),
            ]);
            let (code, stdout, stderr) = invoke(&store, words).await.unwrap();
            assert_eq!(code, 1);
            assert_eq!(stdout, "");
            assert!(stderr.contains(&file.display().to_string()));
        }
    }
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "choose", "-c", "0"]))
        .await
        .unwrap();
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn invalid_installed_metadata_produces_offline_repair_error() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    fs::write(store.root().join("installed/catalogue.json"), b"broken").unwrap();
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "choose", "-c", "1"]))
        .await
        .unwrap();
    assert_eq!(code, 1);
    assert_eq!(stdout, "");
    assert!(stderr.contains("pokington bootstrap"));
    assert!(!stderr.contains("Downloading"));
}

#[test]
fn executable_returns_clap_status_and_uses_separate_output_streams() {
    for words in [
        vec!["--help"],
        vec!["--version"],
        vec!["choose", "--help"],
        vec!["bootstrap", "--help"],
        vec!["art", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_pokington"))
            .args(words)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_ne!(output.stdout, b"");
        assert_eq!(output.stderr, b"");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_pokington"))
        .args(["choose", "-c", "-1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_ne!(output.stderr, b"");
}

fn install_artwork(store: &Store) -> Result<(), Failure> {
    use image::{DynamicImage, Rgba, RgbaImage};
    let image = RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut bytes, image::ImageFormat::Png)?;
    fs::create_dir_all(store.root().join("artwork"))?;
    for form in catalogue()?.forms() {
        fs::write(
            store.root().join(form.local_artwork.as_str()),
            bytes.get_ref(),
        )?;
    }
    Ok(())
}

#[tokio::test]
async fn art_runner_uses_local_data_and_redirected_color_policy() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    install_artwork(&store).unwrap();
    let (code, stdout, stderr) = invoke(
        &store,
        args(&["pokington", "art", "mEw", "--width", "original"]),
    )
    .await
    .unwrap();
    assert_eq!(code, 0);
    assert_eq!(stdout, "@@@@@@@@\n".repeat(8));
    assert_eq!(stderr, "");
    let (code, stdout, stderr) = invoke(
        &store,
        args(&[
            "pokington",
            "art",
            "1",
            "--mode",
            "ascii",
            "--width",
            "4",
            "--color",
            "always",
        ]),
    )
    .await
    .unwrap();
    assert_eq!(code, 0);
    assert!(stdout.contains("\x1b[38;2;255;255;255m@"));
    assert_eq!(stdout.lines().count(), 2);
    assert_eq!(stderr, "");
    let (code, stdout, stderr) = invoke(
        &store,
        args(&[
            "pokington",
            "art",
            "--form",
            "ORIGINAL",
            "--mode",
            "blocks",
            "--width",
            "original",
            "--color",
            "never",
        ]),
    )
    .await
    .unwrap();
    assert_eq!(code, 0);
    assert_eq!(stdout, "████████\n".repeat(4));
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn art_name_and_title_flags_use_the_selected_form_label() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    install_artwork(&store).unwrap();
    let mut catalogue = catalogue().unwrap();
    let mut alternate = catalogue
        .species
        .get(1)
        .unwrap()
        .forms
        .get(1)
        .unwrap()
        .clone();
    // Use a single species to make random selection deterministic, including an alternate form.
    catalogue.species.truncate(1);
    catalogue.source.max_id = 1;
    let species = catalogue.species.first_mut().unwrap();
    species.name = "Sandshrew".into();
    alternate.key = "alola".into();
    alternate.display_name = "Alola Form".into();
    species.forms.first_mut().unwrap().display_name = "SANDSHREW".into();
    species.forms.push(alternate);
    catalogue.validate().unwrap();
    fs::write(
        store.root().join("installed/catalogue.json"),
        serde_json::to_vec(&catalogue).unwrap(),
    )
    .unwrap();
    for mode in ["auto", "ascii", "blocks"] {
        for (selection, expected) in [
            (vec!["sandshrew"], "SANDSHREW"),
            (vec!["1", "--form", "ALOLA"], "Sandshrew (Alola)"),
            (vec!["--form", "alola"], "Sandshrew (Alola)"),
            (vec![], "SANDSHREW"),
        ] {
            let mut words = vec![
                "pokington",
                "art",
                "--mode",
                mode,
                "--width",
                "original",
                "--color",
                "never",
            ];
            words.extend(selection);
            let (code, image, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert!(image.ends_with('\n'));
            words.push("--title");
            let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert_eq!(stdout, format!("{image}\x1b]2;{expected}\x07"));
            words.pop();
            words.push("--name");
            let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert_eq!(stdout, format!("{image}{expected}\n"));
            words.push("--no-newline");
            let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert_eq!(stdout, format!("{image}{expected}"));
            words.pop();
            words.push("--title");
            let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert_eq!(stdout, format!("{image}{expected}\n\x1b]2;{expected}\x07"));
            words.push("--no-newline");
            let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
            assert_eq!((code, stderr.as_str()), (0, ""));
            assert_eq!(stdout, format!("{image}{expected}\x1b]2;{expected}\x07"));
        }
    }
}

#[tokio::test]
async fn art_runner_reports_lookup_and_selected_png_errors_without_output() {
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    for (words, expected) in [
        (vec!["pokington", "art", "missing"], "unknown Pokémon"),
        (
            vec!["pokington", "art", "Mew", "--form", "missing"],
            "available forms: original",
        ),
        (
            vec!["pokington", "art", "--form", "missing"],
            "no artwork matches",
        ),
        (vec!["pokington", "art", "Mew"], "pokington bootstrap"),
    ] {
        let mut words = words;
        words.push("--name");
        words.push("--title");
        let (code, stdout, stderr) = invoke(&store, args(&words)).await.unwrap();
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("Downloading"));
    }
    install_artwork(&store).unwrap();
    let catalogue = catalogue().unwrap();
    let reference = &catalogue
        .species
        .first()
        .unwrap()
        .forms
        .first()
        .unwrap()
        .local_artwork;
    let path = store.root().join(reference.as_str());
    fs::write(&path, b"invalid PNG").unwrap();
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "art", "Mew"]))
        .await
        .unwrap();
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert!(stderr.contains("pokington bootstrap") && stderr.contains(&path.display().to_string()));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(2, 2))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    fs::write(path, bytes.get_ref()).unwrap();
    let (code, stdout, stderr) = invoke(&store, args(&["pokington", "art", "Mew"]))
        .await
        .unwrap();
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert!(stderr.contains("fully transparent") && stderr.contains("pokington bootstrap"));
}

#[tokio::test]
async fn terminal_context_selects_blocks_and_limits_automatic_color_depth() {
    use pokington::{art::Geometry, terminal::Terminal, text_art::Depth};
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    install_artwork(&store).unwrap();
    for (unicode, depth, expected_escape) in [
        (true, Depth::Ansi16, "\x1b[97m"),
        (true, Depth::Ansi256, "\x1b[38;5;231m"),
        (false, Depth::None, ""),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = command::run(
            ["pokington", "art", "Mew", "--width", "4"],
            || {
                Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(1))
                    .with_terminal(Terminal {
                        geometry: Geometry::default(),
                        is_terminal: true,
                        unicode,
                        depth,
                    })
                    .with_picker(ratatui_image::picker::Picker::halfblocks()))
            },
            &mut stdout,
            &mut stderr,
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(stderr, b"");
        let text = String::from_utf8(stdout).unwrap();
        if unicode {
            assert!(text.contains('▀'));
        } else {
            assert_eq!(text, "@@@@\n".repeat(2));
        }
        assert!(text.contains(expected_escape));
    }
}

#[tokio::test]
async fn terminal_metadata_child() {
    let Some(path) = std::env::var_os("POKINGTON_TEST_ART_STORE") else {
        return;
    };
    let store = Store::at(PathBuf::from(path)).unwrap();
    let code = command::run(
        [
            "pokington",
            "art",
            "Mew",
            "--mode",
            "ascii",
            "--width",
            "8",
            "--color",
            "never",
        ],
        || Ok(Runtime::new(store, StdRng::seed_from_u64(1))),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
    .await;
    assert_eq!(code, 0);
}

#[test]
fn runner_uses_reported_cell_proportions_without_querying_input() {
    use nix::pty::{Winsize, openpty};
    use std::{fs::File, io::Read, process::Stdio};
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    install_artwork(&store).unwrap();
    for (cell_width, cell_height, rows) in [(8_u16, 16_u16, 4), (12, 12, 8)] {
        let pty = openpty(
            Some(&Winsize {
                ws_col: 80,
                ws_row: 24,
                ws_xpixel: 80 * cell_width,
                ws_ypixel: 24 * cell_height,
            }),
            None,
        )
        .unwrap();
        let mut master = File::from(pty.master);
        let slave = File::from(pty.slave);
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            // Drain while the slave is open; macOS can discard unread bytes at its last close.
            if let Err(error) = master.read_to_end(&mut bytes) {
                assert_eq!(error.raw_os_error(), Some(5));
            }
            bytes
        });
        let output = Command::new(std::env::current_exe().unwrap())
            .env("POKINGTON_TEST_ART_STORE", store.root())
            .env("TERM", "dumb")
            .args([
                "--exact",
                "terminal_metadata_child",
                "--quiet",
                "--nocapture",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(slave))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        let bytes = reader.join().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(
            text.lines().filter(|line| *line == "@@@@@@@@").count(),
            rows,
            "{text}"
        );
        assert!(!text.contains('\x1b'));
    }
}

#[tokio::test]
async fn native_modes_use_library_protocol_and_explicit_text_bypasses_it() {
    use pokington::terminal::Terminal;
    use ratatui_image::picker::{Picker, ProtocolType};
    let temp = Temp::new().unwrap();
    let store = temp.install().unwrap();
    install_artwork(&store).unwrap();
    for protocol in [
        ProtocolType::Kitty,
        ProtocolType::Sixel,
        ProtocolType::Iterm2,
    ] {
        for mode in ["auto", "img", "ascii", "blocks"] {
            let mut picker = Picker::halfblocks();
            picker.set_protocol_type(protocol);
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let code = command::run(
                [
                    "pokington",
                    "art",
                    "Mew",
                    "--mode",
                    mode,
                    "--width",
                    "original",
                    "--color",
                    "never",
                    "--name",
                    "--no-newline",
                ],
                || {
                    Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(1))
                        .with_terminal(Terminal {
                            is_terminal: true,
                            unicode: true,
                            ..Terminal::default()
                        })
                        .with_picker(picker))
                },
                &mut stdout,
                &mut stderr,
            )
            .await;
            assert_eq!(code, 0);
            assert_eq!(stderr, b"");
            let text = String::from_utf8(stdout).unwrap();
            assert!(text.ends_with("Mew (Charizard)"), "{text}");
            if mode == "auto" || mode == "img" {
                assert!(text.contains('\x1b'));
                assert!(!text.contains("\x1b[?1049"));
            } else {
                assert!(!text.contains('\x1b'));
            }
        }
    }
    let (code, stdout, stderr) =
        invoke(&store, args(&["pokington", "art", "Mew", "--mode", "img"]))
            .await
            .unwrap();
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert!(
        stderr.contains("terminal stdin and stdout")
            && stderr.contains("--mode auto, blocks, or ascii")
    );
}
