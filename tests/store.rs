#![cfg(unix)]

use std::{
    collections::BTreeSet,
    env, fs,
    io::{self, BufRead, Write},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use pokington::{
    catalogue::{ArtworkRef, Catalogue},
    serebii::{self, Index, Species},
    store::{DataPlatform, Store, StoreError, Writer, application_data_dir, validate_png},
};
use serde_json::{Value, json};

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);
type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

struct Temp(PathBuf);
impl Temp {
    fn new() -> io::Result<Self> {
        loop {
            let path = env::temp_dir().join(format!(
                "pokington-store-{}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e),
            }
        }
    }
    fn store(&self) -> Result<Store, StoreError> {
        Store::at(self.0.join("data"))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn source() -> TestResult<(Index, Vec<Species>)> {
    let index = serebii::parse_index(include_str!("fixtures/serebii-index.html"))?;
    let species_link = index.species.first().ok_or("missing species fixture row")?;
    let hub = serebii::parse_hub(
        include_str!("fixtures/serebii-charizard.html"),
        species_link,
    )?;
    let species = serebii::reconcile(&hub, &[])?;
    Ok((index, vec![species]))
}
fn png(value: u8) -> TestResult<Vec<u8>> {
    let mut bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()?
            .write_image_data(&[value, 20, 40, 255, 0, 0, 0, 0])?;
    }
    Ok(bytes)
}
fn refs(writer: &Writer) -> TestResult<Vec<ArtworkRef>> {
    let checkpoint = writer.checkpoint()?;
    Ok(checkpoint
        .catalogue()
        .forms()
        .map(|f| f.local_artwork.clone())
        .collect())
}
fn prepare(writer: &mut Writer, force: bool) -> TestResult<()> {
    let (index, species) = source()?;
    writer.prepare(&index, species, force)?;
    Ok(())
}
fn complete(writer: &mut Writer, color: u8) -> TestResult<()> {
    for reference in refs(writer)? {
        if !writer.checkpoint()?.completed().contains(&reference) {
            writer.write_artwork(&reference, &png(color)?)?;
        }
    }
    Ok(())
}
fn install(store: &Store) -> TestResult<Catalogue> {
    let mut writer = store.try_writer()?;
    prepare(&mut writer, false)?;
    complete(&mut writer, 10)?;
    Ok(writer.publish()?)
}

#[test]
fn data_roots_follow_macos_and_xdg_rules_without_touching_user_directories() {
    let home = Some(Path::new("/home/tester"));
    assert_eq!(
        application_data_dir(DataPlatform::MacOs, home, Some(Path::new("/ignored"))).unwrap(),
        Path::new("/home/tester/Library/Application Support/pokington")
    );
    assert_eq!(
        application_data_dir(DataPlatform::Linux, home, None).unwrap(),
        Path::new("/home/tester/.local/share/pokington")
    );
    assert_eq!(
        application_data_dir(DataPlatform::Linux, None, Some(Path::new("/data"))).unwrap(),
        Path::new("/data/pokington")
    );
    assert_eq!(
        application_data_dir(DataPlatform::Linux, home, Some(Path::new("relative"))).unwrap(),
        Path::new("/home/tester/.local/share/pokington")
    );
    assert!(application_data_dir(DataPlatform::MacOs, None, None).is_err());
    assert!(Store::at(PathBuf::from("relative")).is_err());
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    assert!(store.load_active().unwrap().is_none());
    assert!(!store.root().exists());
}

#[test]
fn interrupted_first_install_resumes_only_complete_validated_assets() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let reference;
    {
        let mut writer = store.try_writer().unwrap();
        prepare(&mut writer, false).unwrap();
        reference = refs(&writer).unwrap()[0].clone();
        writer.write_artwork(&reference, &png(10).unwrap()).unwrap();
        fs::write(
            store.root().join("artwork/.png-interrupted.tmp"),
            b"partial transfer",
        )
        .unwrap();
        let interrupted_install = store.root().join(".install-interrupted");
        fs::create_dir(&interrupted_install).unwrap();
        fs::write(
            interrupted_install.join("catalogue.json"),
            b"partial metadata",
        )
        .unwrap();
        assert!(writer.publish().is_err());
        assert!(store.load_active().unwrap().is_none());
    }
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    assert_eq!(
        writer.checkpoint().unwrap().completed(),
        &BTreeSet::from([reference])
    );
    assert!(store.load_active().unwrap().is_none());
    complete(&mut writer, 10).unwrap();
    let published = writer.publish().unwrap();
    assert_eq!(store.load_active().unwrap().unwrap(), published);
    assert!(!store.root().join("pending.json").exists());
    assert!(store.root().join("artwork/.png-interrupted.tmp").exists());
}

#[test]
fn a_checkpoint_left_after_committed_publication_cannot_bypass_a_fresh_force() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, true).unwrap();
    complete(&mut writer, 10).unwrap();
    let saved = fs::read(store.root().join("pending.json")).unwrap();
    let published = writer.publish().unwrap();
    // Crash after active catalogue rename, before pending.json removal.
    fs::write(store.root().join("pending.json"), saved).unwrap();
    drop(writer);
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, true).unwrap();
    assert!(writer.checkpoint().unwrap().completed().is_empty());
    for reference in refs(&writer).unwrap() {
        assert!(published.forms().all(|f| f.local_artwork != reference));
    }
    assert_eq!(store.load_active().unwrap().unwrap(), published);
}

#[test]
fn failed_force_preserves_active_and_retry_reuses_staged_replacements() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let old = install(&store).unwrap();
    let original_bytes: Vec<_> = old
        .forms()
        .map(|f| store.read_artwork(&f.local_artwork).unwrap())
        .collect();
    let staged;
    {
        let mut writer = store.try_writer().unwrap();
        prepare(&mut writer, true).unwrap();
        assert!(writer.reuse_active(&old).is_err());
        staged = refs(&writer).unwrap()[0].clone();
        assert!(old.forms().all(|f| f.local_artwork != staged));
        writer.write_artwork(&staged, &png(99).unwrap()).unwrap();
        let other = refs(&writer).unwrap()[1].clone();
        assert!(writer.write_artwork(&other, b"not a PNG").is_err());
        assert!(writer.publish().is_err());
        assert_eq!(store.load_active().unwrap().unwrap(), old);
    }
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, true).unwrap();
    assert!(writer.checkpoint().unwrap().force());
    assert_eq!(
        writer.checkpoint().unwrap().completed(),
        &BTreeSet::from([staged])
    );
    complete(&mut writer, 99).unwrap();
    let new = writer.publish().unwrap();
    assert_eq!(store.load_active().unwrap().unwrap(), new);
    for (form, bytes) in old.forms().zip(original_bytes) {
        assert_eq!(store.read_artwork(&form.local_artwork).unwrap(), bytes);
        assert!(new.forms().all(|f| f.local_artwork != form.local_artwork));
    }
    // A new force operation starts again; only retrying the unfinished force may reuse staged bytes.
    prepare(&mut writer, true).unwrap();
    assert!(writer.checkpoint().unwrap().completed().is_empty());
}

#[test]
fn refresh_reuses_matching_active_artwork_and_discovers_additional_forms() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let (index, mut reduced) = source().unwrap();
    reduced[0].forms.retain(|f| f.key == "original");
    let mut writer = store.try_writer().unwrap();
    writer.prepare(&index, reduced, false).unwrap();
    complete(&mut writer, 10).unwrap();
    let old = writer.publish().unwrap();
    prepare(&mut writer, false).unwrap();
    assert_eq!(writer.reuse_active(&old).unwrap(), 1);
    assert_eq!(writer.checkpoint().unwrap().completed().len(), 1);
    assert_eq!(
        writer.checkpoint().unwrap().catalogue().forms().count(),
        source().unwrap().1[0].forms.len()
    );
    complete(&mut writer, 22).unwrap();
    let new = writer.publish().unwrap();
    assert!(
        new.forms()
            .any(|f| f.local_artwork == old.forms().next().unwrap().local_artwork)
    );
}

#[test]
fn changed_discovery_or_force_policy_does_not_resume_an_unrelated_checkpoint() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    let old_ref = refs(&writer).unwrap()[0].clone();
    writer.write_artwork(&old_ref, &png(10).unwrap()).unwrap();
    prepare(&mut writer, true).unwrap();
    assert!(writer.checkpoint().unwrap().completed().is_empty());
    assert!(!refs(&writer).unwrap().contains(&old_ref));
    let (index, mut species) = source().unwrap();
    species[0].forms.retain(|f| f.key == "original");
    writer.prepare(&index, species, true).unwrap();
    assert_eq!(writer.checkpoint().unwrap().catalogue().forms().count(), 1);
    assert!(writer.checkpoint().unwrap().completed().is_empty());
    assert!(store.root().join(old_ref.as_str()).exists());
}

#[test]
fn resume_recovers_complete_files_written_before_checkpoint_and_rejects_later_corruption() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    let first = refs(&writer).unwrap()[0].clone();
    // Crash after durable final PNG link but before metadata checkpoint update.
    fs::write(store.root().join(first.as_str()), png(10).unwrap()).unwrap();
    drop(writer);
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    assert!(writer.checkpoint().unwrap().completed().contains(&first));
    assert!(writer.write_artwork(&first, &png(99).unwrap()).is_err());
    assert_eq!(store.read_artwork(&first).unwrap(), png(10).unwrap());
    fs::write(
        store.root().join(first.as_str()),
        b"corrupted after checkpoint",
    )
    .unwrap();
    complete(&mut writer, 10).unwrap();
    assert!(writer.publish().is_err());
    drop(writer);
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    assert!(!refs(&writer).unwrap().contains(&first));
    assert!(!writer.checkpoint().unwrap().completed().contains(&first));
    assert_eq!(
        fs::read(store.root().join(first.as_str())).unwrap(),
        b"corrupted after checkpoint"
    );
    complete(&mut writer, 33).unwrap();
    writer.publish().unwrap();
}

#[test]
fn metadata_reads_report_missing_corrupt_unknown_schema_without_decoding_all_artwork() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let old = install(&store).unwrap();
    let path = store.root().join("installed/catalogue.json");
    let saved = fs::read(&path).unwrap();
    for bytes in [
        b"{".as_slice(),
        br#"{"schema_version":999,"future":"shape"}"#,
    ] {
        fs::write(&path, bytes).unwrap();
        let error = store.load_active().unwrap_err().to_string();
        assert!(error.contains("bootstrap"));
        assert!(error.contains("catalogue.json"));
    }
    fs::remove_file(&path).unwrap();
    assert!(
        store
            .load_active()
            .unwrap_err()
            .to_string()
            .contains("bootstrap")
    );
    fs::write(&path, &saved).unwrap();
    let selected = &old.forms().next().unwrap().local_artwork;
    fs::write(
        store.root().join(selected.as_str()),
        b"corrupt selected artwork",
    )
    .unwrap();
    assert_eq!(store.load_active().unwrap().unwrap(), old);
    assert!(validate_png(&store.read_artwork(selected).unwrap()).is_err());
    fs::remove_file(store.root().join(selected.as_str())).unwrap();
    assert!(
        store
            .read_artwork(selected)
            .unwrap_err()
            .to_string()
            .contains("bootstrap")
    );
}

#[test]
fn checkpoint_validation_and_explicit_discard_preserve_installed_data() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let old = install(&store).unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, true).unwrap();
    let path = store.root().join("pending.json");
    let mut changed: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    changed["schema_version"] = json!(999);
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    let (index, species) = source().unwrap();
    assert!(writer.prepare(&index, species, true).is_err());
    assert_eq!(store.load_active().unwrap().unwrap(), old);
    writer.discard_pending().unwrap();
    prepare(&mut writer, true).unwrap();
    assert!(writer.checkpoint().unwrap().completed().is_empty());
    assert_eq!(store.load_active().unwrap().unwrap(), old);
}

#[test]
fn safe_local_references_cannot_follow_artwork_or_metadata_symlinks() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let old = install(&store).unwrap();
    let reference = old.forms().next().unwrap().local_artwork.clone();
    let outside = temp.0.join("outside.png");
    fs::write(&outside, png(10).unwrap()).unwrap();
    fs::remove_file(store.root().join(reference.as_str())).unwrap();
    symlink(&outside, store.root().join(reference.as_str())).unwrap();
    assert!(store.read_artwork(&reference).is_err());
    fs::remove_file(store.root().join(reference.as_str())).unwrap();
    fs::remove_dir_all(store.root().join("artwork")).unwrap();
    symlink(&temp.0, store.root().join("artwork")).unwrap();
    assert!(store.read_artwork(&reference).is_err());
    assert!(store.try_writer().is_err());
    let metadata = store.root().join("installed/catalogue.json");
    let outside_json = temp.0.join("outside.json");
    fs::rename(&metadata, &outside_json).unwrap();
    symlink(&outside_json, &metadata).unwrap();
    assert!(store.load_active().is_err());
    assert_eq!(fs::read(outside).unwrap(), png(10).unwrap());
}

#[test]
fn readers_see_only_complete_catalogues_while_force_replacement_is_staged_and_published() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let old = install(&store).unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, true).unwrap();
    let new = writer.checkpoint().unwrap().catalogue().clone();
    let reader_store = store.clone();
    let old_snapshot = old.clone();
    let new_snapshot = new.clone();
    let barrier = Arc::new(Barrier::new(2));
    let reader_barrier = barrier.clone();
    let reader = thread::spawn(move || {
        reader_barrier.wait();
        for _ in 0..100 {
            let current = reader_store.load_active().unwrap().unwrap();
            assert!(current == old_snapshot || current == new_snapshot);
            for form in current.forms() {
                validate_png(&reader_store.read_artwork(&form.local_artwork).unwrap()).unwrap();
            }
        }
    });
    barrier.wait();
    complete(&mut writer, 99).unwrap();
    assert_eq!(store.load_active().unwrap().unwrap(), old);
    writer.publish().unwrap();
    reader.join().unwrap();
    assert_eq!(store.load_active().unwrap().unwrap(), new);
    for form in old.forms() {
        assert_eq!(
            store.read_artwork(&form.local_artwork).unwrap(),
            png(10).unwrap()
        );
    }
}

#[test]
fn writer_lock_child() {
    let Some(root) = env::var_os("POKINGTON_LOCK_TEST_ROOT") else {
        return;
    };
    let store = Store::at(PathBuf::from(root)).unwrap();
    let _writer = store.try_writer().unwrap();
    println!("POKINGTON_WRITER_LOCKED");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
}

#[test]
fn concurrent_writers_are_excluded_and_process_death_releases_the_lock() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    {
        let _writer = store.try_writer().unwrap();
        assert!(matches!(store.try_writer(), Err(StoreError::Busy(_))));
    }
    let mut child = Command::new(env::current_exe().unwrap())
        .args(["--exact", "writer_lock_child", "--nocapture"])
        .env("POKINGTON_LOCK_TEST_ROOT", store.root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = io::BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        assert!(
            output.read_line(&mut line).unwrap() > 0,
            "child exited without acquiring lock"
        );
        if line.contains("POKINGTON_WRITER_LOCKED") {
            break;
        }
        line.clear();
    }
    assert!(matches!(store.try_writer(), Err(StoreError::Busy(_))));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(store.root().join("writer.lock").exists());
    assert!(store.try_writer().is_ok());
}

#[test]
fn invalid_png_signature_checksum_truncation_and_blank_pixels_never_complete_slots() {
    let temp = Temp::new().unwrap();
    let store = temp.store().unwrap();
    let mut writer = store.try_writer().unwrap();
    prepare(&mut writer, false).unwrap();
    let reference = refs(&writer).unwrap()[0].clone();
    let valid = png(10).unwrap();
    let mut bad_crc = valid.clone();
    bad_crc[29] ^= 1;
    let mut transparent = vec![];
    {
        let mut encoder = png::Encoder::new(&mut transparent, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[10, 20, 40, 0, 0, 0, 0, 0])
            .unwrap();
    }
    let mut uniform = vec![];
    {
        let mut encoder = png::Encoder::new(&mut uniform, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[10, 20, 40, 255, 10, 20, 40, 255])
            .unwrap();
    }
    for bytes in [
        b"not png".as_slice(),
        &valid[..valid.len() / 2],
        &bad_crc,
        &transparent,
        &uniform,
    ] {
        assert!(writer.write_artwork(&reference, bytes).is_err());
        assert!(!store.root().join(reference.as_str()).exists());
        assert!(writer.checkpoint().unwrap().completed().is_empty());
    }
    assert!(writer.publish().is_err());
    assert!(store.load_active().unwrap().is_none());
}
