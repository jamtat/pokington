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
};
use rand::{SeedableRng, rngs::StdRng};

type Failure = Box<dyn std::error::Error>;
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Result<Self, Failure> {
        let path = std::env::temp_dir().join(format!(
            "pokington-command-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
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
        || Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(23)).with_bootstrap(source)),
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
        (vec!["pokington"], 2),
        (vec!["pokington", "choose"], 2),
        (vec!["pokington", "choose", "-c", "-1"], 2),
        (vec!["pokington", "choose", "-c", "no"], 2),
        (
            vec!["pokington", "choose", "-c", "9999999999999999999999999"],
            2,
        ),
        (vec!["pokington", "bootstrap", "--unknown"], 2),
        (vec!["pokington", "art"], 2),
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
