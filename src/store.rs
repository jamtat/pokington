//! One bootstrap writer, immutable PNG files, and atomic JSON publication on macOS/Linux.
//!
//! Readers never lock. A loaded catalogue remains usable after replacement because artwork
//! references are never overwritten or collected. Temporary files and abandoned artwork may
//! remain after a crash; cleanup is deliberately outside this stage.

use std::{
    collections::{BTreeMap, BTreeSet, hash_map::RandomState},
    env, fmt,
    fs::{self, File, OpenOptions, TryLockError},
    hash::{BuildHasher, Hasher},
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    catalogue::{ArtworkRef, Catalogue, CatalogueError},
    serebii,
};

const CHECKPOINT_SCHEMA: u32 = 1;
const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PNG_BYTES: u64 = 20 * 1024 * 1024;
const MAX_DECODE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub enum StoreError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Catalogue(CatalogueError),
    Busy(PathBuf),
    InvalidCheckpoint(String),
    InvalidArtwork {
        reference: ArtworkRef,
        reason: String,
    },
    RepairRequired {
        path: PathBuf,
        reason: String,
    },
    DataDirectory(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Catalogue(error) => error.fmt(f),
            Self::Busy(path) => write!(f, "another bootstrap writer holds {}", path.display()),
            Self::InvalidCheckpoint(reason) => write!(f, "invalid pending checkpoint: {reason}"),
            Self::InvalidArtwork { reference, reason } => {
                write!(f, "{}: {reason}", reference.as_str())
            }
            Self::RepairRequired { path, reason } => write!(
                f,
                "{}: {reason}; run explicit `pokington bootstrap` to repair local data",
                path.display()
            ),
            Self::DataDirectory(reason) => write!(f, "application data directory: {reason}"),
        }
    }
}
impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Catalogue(source) => Some(source),
            _ => None,
        }
    }
}
impl From<CatalogueError> for StoreError {
    fn from(value: CatalogueError) -> Self {
        Self::Catalogue(value)
    }
}
fn at(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.into(),
        source,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DataPlatform {
    MacOs,
    Linux,
}

/// Explicit environment inputs keep directory tests independent of the real user environment.
pub fn application_data_dir(
    platform: DataPlatform,
    home: Option<&Path>,
    xdg_data_home: Option<&Path>,
) -> Result<PathBuf, StoreError> {
    let home = || {
        home.filter(|p| p.is_absolute())
            .ok_or_else(|| StoreError::DataDirectory("HOME must be an absolute path".into()))
    };
    Ok(match platform {
        DataPlatform::MacOs => home()?.join("Library/Application Support/pokington"),
        DataPlatform::Linux => match xdg_data_home.filter(|p| p.is_absolute()) {
            Some(path) => path.join("pokington"),
            None => home()?.join(".local/share/pokington"),
        },
    })
}

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// No filesystem writes occur until `try_writer`.
    pub fn at(root: PathBuf) -> Result<Self, StoreError> {
        if !root.is_absolute() {
            return Err(StoreError::DataDirectory(
                "store root must be absolute".into(),
            ));
        }
        Ok(Self { root })
    }

    pub fn platform() -> Result<Self, StoreError> {
        let platform = if cfg!(target_os = "macos") {
            DataPlatform::MacOs
        } else if cfg!(target_os = "linux") {
            DataPlatform::Linux
        } else {
            return Err(StoreError::DataDirectory(
                "only macOS and Linux are supported".into(),
            ));
        };
        Self::at(application_data_dir(
            platform,
            env::var_os("HOME").as_deref().map(Path::new),
            env::var_os("XDG_DATA_HOME").as_deref().map(Path::new),
        )?)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Validate metadata only; rendering validates the selected PNG, not every image on `choose`.
    pub fn load_active(&self) -> Result<Option<Catalogue>, StoreError> {
        let installed = self.root.join("installed");
        let path = installed.join("catalogue.json");
        let result = (|| {
            self.check_root()?;
            match fs::symlink_metadata(&installed) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(at(&installed, error)),
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(at(
                        &installed,
                        io::Error::other("installed data is not a directory"),
                    ));
                }
                Ok(_) => (),
            }
            Catalogue::from_json(&read_regular(&path, MAX_JSON_BYTES)?)
                .map(Some)
                .map_err(StoreError::from)
        })();
        result.map_err(|error| StoreError::RepairRequired {
            path,
            reason: error.to_string(),
        })
    }

    pub fn read_artwork(&self, reference: &ArtworkRef) -> Result<Vec<u8>, StoreError> {
        let result = self.artwork_bytes(reference);
        result.map_err(|error| StoreError::RepairRequired {
            path: self.root.join(reference.as_str()),
            reason: error.to_string(),
        })
    }

    pub fn try_writer(&self) -> Result<Writer, StoreError> {
        fs::create_dir_all(&self.root).map_err(|e| at(&self.root, e))?;
        self.check_root()?;
        let lock_path = self.root.join("writer.lock");
        let lock = regular_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| at(&lock_path, e))?;
        match lock.try_lock() {
            Ok(()) => (),
            Err(TryLockError::WouldBlock) => return Err(StoreError::Busy(lock_path)),
            Err(TryLockError::Error(error)) => return Err(at(&lock_path, error)),
        }
        ensure_directory(&self.root.join("artwork"))?;
        Ok(Writer {
            store: self.clone(),
            _lock: lock,
            pending: None,
        })
    }

    fn check_root(&self) -> Result<(), StoreError> {
        match fs::symlink_metadata(&self.root) {
            Ok(metadata) if metadata.is_dir() => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(at(
                &self.root,
                io::Error::other("store root is not a directory"),
            )),
            Err(error) => Err(at(&self.root, error)),
        }
    }

    fn artwork_bytes(&self, reference: &ArtworkRef) -> Result<Vec<u8>, StoreError> {
        self.check_root()?;
        check_directory(&self.root.join("artwork"))?;
        read_regular(&self.root.join(reference.as_str()), MAX_PNG_BYTES)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    schema_version: u32,
    force: bool,
    catalogue: Catalogue,
    #[serde(skip)]
    completed: BTreeSet<ArtworkRef>,
}

impl Checkpoint {
    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }
    pub fn force(&self) -> bool {
        self.force
    }
    pub fn completed(&self) -> &BTreeSet<ArtworkRef> {
        &self.completed
    }

    fn validate(&self) -> Result<(), StoreError> {
        if self.schema_version != CHECKPOINT_SCHEMA {
            return Err(StoreError::InvalidCheckpoint(format!(
                "unsupported checkpoint schema {}",
                self.schema_version
            )));
        }
        self.catalogue.validate().map_err(StoreError::from)
    }
}

/// Keep this value alive for the whole bootstrap. Never remove the permanent lock file.
pub struct Writer {
    store: Store,
    _lock: File,
    pending: Option<Checkpoint>,
}

impl Writer {
    /// Resume only an identical source discovery and force policy; other refreshes get fresh slots.
    pub fn prepare(
        &mut self,
        index: &serebii::Index,
        species: Vec<serebii::Species>,
        force: bool,
    ) -> Result<&Checkpoint, StoreError> {
        let catalogue = Catalogue::from_serebii(index, species, &fresh_id())?;
        let path = self.store.root.join("pending.json");
        let existing = match read_regular(&path, MAX_JSON_BYTES) {
            Ok(bytes) => {
                let checkpoint: Checkpoint = serde_json::from_slice(&bytes)
                    .map_err(|e| StoreError::InvalidCheckpoint(e.to_string()))?;
                checkpoint.validate()?;
                Some(checkpoint)
            }
            Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        // Publication may have committed just before a crash prevented checkpoint removal.
        // An unreadable installed catalogue is not reusable; an explicit bootstrap can repair it.
        let active = self.store.load_active().ok().flatten();
        let mut pending = existing
            .filter(|p| active.as_ref() != Some(&p.catalogue))
            .filter(|p| p.force == force && p.catalogue.same_source(&catalogue))
            .unwrap_or(Checkpoint {
                schema_version: CHECKPOINT_SCHEMA,
                force,
                catalogue,
                completed: BTreeSet::new(),
            });
        // Final files can precede the checkpoint after a crash. Revalidate them, never partial temps.
        for form in pending
            .catalogue
            .species
            .iter_mut()
            .flat_map(|s| &mut s.forms)
        {
            match self.store.artwork_bytes(&form.local_artwork) {
                Ok(bytes) if validate_png(&bytes).is_ok() => {
                    pending.completed.insert(form.local_artwork.clone());
                }
                Ok(_) => {
                    // A damaged immutable slot is abandoned, including when also referenced by active.
                    form.local_artwork = fresh_artwork_ref();
                }
                Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        replace_json(&path, &pending)?;
        self.pending = Some(pending);
        self.checkpoint()
    }

    pub fn checkpoint(&self) -> Result<&Checkpoint, StoreError> {
        self.pending
            .as_ref()
            .ok_or_else(|| StoreError::InvalidCheckpoint("prepare a refresh first".into()))
    }

    /// Non-force bootstrap can reuse matching validated active artwork. Force must write fresh files.
    pub fn reuse_active(&mut self, active: &Catalogue) -> Result<usize, StoreError> {
        active.validate()?;
        let mut pending = self.checkpoint()?.clone();
        if pending.force {
            return Err(StoreError::InvalidCheckpoint(
                "forced refresh cannot reuse active artwork".into(),
            ));
        }
        let old: BTreeMap<_, _> = active
            .species
            .iter()
            .flat_map(|s| s.forms.iter().map(move |f| ((s.id, f.key.as_str()), f)))
            .collect();
        let mut reused = 0;
        for species in &mut pending.catalogue.species {
            for form in &mut species.forms {
                if pending.completed.contains(&form.local_artwork) {
                    continue;
                }
                if let Some(previous) = old.get(&(species.id, form.key.as_str()))
                    && previous.source_id == form.source_id
                    && previous.artwork_url == form.artwork_url
                {
                    match self.store.artwork_bytes(&previous.local_artwork) {
                        Ok(bytes) if validate_png(&bytes).is_ok() => {
                            form.local_artwork = previous.local_artwork.clone();
                            pending.completed.insert(form.local_artwork.clone());
                            reused += 1;
                        }
                        Ok(_) => (),
                        Err(StoreError::Io { source, .. })
                            if source.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        pending.validate()?;
        replace_json(&self.store.root.join("pending.json"), &pending)?;
        self.pending = Some(pending);
        Ok(reused)
    }

    /// Validate before completing an immutable slot. Never overwrite an existing artwork file.
    pub fn write_artwork(
        &mut self,
        reference: &ArtworkRef,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        if !self
            .checkpoint()?
            .catalogue
            .forms()
            .any(|f| &f.local_artwork == reference)
        {
            return Err(StoreError::InvalidCheckpoint(
                "artwork is outside pending catalogue".into(),
            ));
        }
        validate_png(bytes).map_err(|reason| StoreError::InvalidArtwork {
            reference: reference.clone(),
            reason,
        })?;
        let directory = self.store.root.join("artwork");
        check_directory(&directory)?;
        let (temporary, mut file) = temporary_file(&directory, "png")?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| at(&temporary.0, e))?;
        let destination = self.store.root.join(reference.as_str());
        fs::hard_link(&temporary.0, &destination).map_err(|e| at(&destination, e))?;
        sync_directory(&directory)?;
        self.pending
            .as_mut()
            .expect("checkpoint checked above")
            .completed
            .insert(reference.clone());
        Ok(())
    }

    /// The installed directory / catalogue JSON rename is the commit point. Errors after it may mean publication succeeded;
    /// a retry is safe. No active image or previous catalogue's artwork is ever removed.
    pub fn publish(&mut self) -> Result<Catalogue, StoreError> {
        let pending = self.checkpoint()?;
        pending.validate()?;
        for form in pending.catalogue.forms() {
            if !pending.completed.contains(&form.local_artwork) {
                return Err(StoreError::InvalidCheckpoint(format!(
                    "artwork is incomplete: {}",
                    form.local_artwork.as_str()
                )));
            }
            let bytes = self.store.artwork_bytes(&form.local_artwork)?;
            validate_png(&bytes).map_err(|reason| StoreError::InvalidArtwork {
                reference: form.local_artwork.clone(),
                reason,
            })?;
        }
        let catalogue = pending.catalogue.clone();
        let installed = self.store.root.join("installed");
        match fs::symlink_metadata(&installed) {
            Ok(_) => {
                check_directory(&installed)?;
                replace_json(&installed.join("catalogue.json"), &catalogue)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let staging = self.store.root.join(format!(".install-{}", fresh_id()));
                fs::create_dir(&staging).map_err(|e| at(&staging, e))?;
                let result = (|| {
                    replace_json(&staging.join("catalogue.json"), &catalogue)?;
                    fs::rename(&staging, &installed).map_err(|e| at(&installed, e))?;
                    sync_directory(&self.store.root)
                })();
                // An interrupted process can leave this directory. It is never treated as installed.
                let _ = fs::remove_dir_all(&staging);
                result?;
            }
            Err(error) => return Err(at(&installed, error)),
        }
        fs::remove_file(self.store.root.join("pending.json"))
            .map_err(|e| at(&self.store.root.join("pending.json"), e))?;
        sync_directory(&self.store.root)?;
        self.pending = None;
        Ok(catalogue)
    }

    /// Explicit repair of an unreadable checkpoint; retain all immutable files and active data.
    pub fn discard_pending(&mut self) -> Result<(), StoreError> {
        let path = self.store.root.join("pending.json");
        match fs::remove_file(&path) {
            Ok(()) => sync_directory(&self.store.root)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(at(&path, e)),
        }
        self.pending = None;
        Ok(())
    }
}

fn fresh_id() -> String {
    format!(
        "{:016x}{:016x}",
        RandomState::new().build_hasher().finish(),
        RandomState::new().build_hasher().finish()
    )
}
fn fresh_artwork_ref() -> ArtworkRef {
    ArtworkRef::new(format!("artwork/{}-1.png", fresh_id())).expect("generated artwork reference")
}

fn regular_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    options
}

fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>, StoreError> {
    let file = regular_options()
        .read(true)
        .open(path)
        .map_err(|e| at(path, e))?;
    if !file.metadata().map_err(|e| at(path, e))?.is_file() {
        return Err(at(path, io::Error::other("not a regular file")));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| at(path, e))?;
    if bytes.len() as u64 > limit {
        return Err(at(path, io::Error::other("file exceeds size limit")));
    }
    Ok(bytes)
}

fn check_directory(path: &Path) -> Result<(), StoreError> {
    if !fs::symlink_metadata(path)
        .map_err(|e| at(path, e))?
        .is_dir()
    {
        return Err(at(path, io::Error::other("not a directory")));
    }
    Ok(())
}
fn ensure_directory(path: &Path) -> Result<(), StoreError> {
    match fs::create_dir(path) {
        Ok(()) => sync_directory(path.parent().expect("data directory parent"))?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(at(path, e)),
    }
    check_directory(path)
}
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| at(path, e))
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn temporary_file(directory: &Path, label: &str) -> Result<(Temporary, File), StoreError> {
    loop {
        let path = directory.join(format!(".{label}-{}.tmp", fresh_id()));
        match regular_options().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((Temporary(path), file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(at(&path, e)),
        }
    }
}

fn replace_json(path: &Path, value: &impl Serialize) -> Result<(), StoreError> {
    atomic_replace(path, |file| {
        serde_json::to_writer(file, value).map_err(io::Error::other)
    })
}
fn atomic_replace(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), StoreError> {
    let directory = path.parent().expect("metadata parent");
    let (temporary, mut file) = temporary_file(directory, "json")?;
    write(&mut file)
        .and_then(|()| file.sync_all())
        .map_err(|e| at(&temporary.0, e))?;
    fs::rename(&temporary.0, path).map_err(|e| at(path, e))?;
    sync_directory(directory)
}

/// Full PNG checksum/frame/end validation, with bounded input and allocation. Match S01's
/// visible, non-uniform illustration requirements; network status/MIME checks belong to bootstrap.
pub fn validate_png(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_PNG_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("not a PNG within the input size limit".into());
    }
    let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
    decoder.ignore_checksums(false);
    decoder.set_limits(png::Limits {
        bytes: MAX_DECODE_BYTES,
    });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    if reader.info().animation_control.is_some() {
        return Err("animated artwork is unsupported".into());
    }
    let size = reader
        .output_buffer_size()
        .filter(|n| *n <= MAX_DECODE_BYTES)
        .ok_or("PNG exceeds decode size limit")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
    reader.finish().map_err(|e| e.to_string())?;
    let samples = info.color_type.samples();
    let pixels = &pixels[..info.buffer_size()];
    let mut values = pixels.chunks_exact(samples);
    let first = values.next().ok_or("empty PNG")?;
    if values.all(|pixel| pixel == first) {
        return Err("uniform blank PNG".into());
    }
    if matches!(
        info.color_type,
        png::ColorType::Rgba | png::ColorType::GrayscaleAlpha
    ) && pixels.chunks_exact(samples).all(|p| p[samples - 1] == 0)
    {
        return Err("fully transparent PNG".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_metadata_write_retains_old_bytes_and_removes_temporary() {
        let root = env::temp_dir().join(format!("pokington-atomic-{}", fresh_id()));
        fs::create_dir(&root).unwrap();
        let path = root.join("active.json");
        fs::write(&path, b"old complete catalogue").unwrap();
        let result = atomic_replace(&path, |file| {
            file.write_all(b"new partial catalogue")?;
            Err(io::Error::other("simulated interruption before rename"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old complete catalogue");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
