//! Command parsing, constrained species selection, and the shared bootstrap lifecycle.

use std::{ffi::OsString, fmt, fs, io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use rand::{Rng, seq::SliceRandom};

use crate::{
    art::{self, ArtError, Color, Mode, Width},
    bootstrap::{Bootstrap, BootstrapError},
    catalogue::Catalogue,
    store::{Store, StoreError},
    terminal::Terminal,
};

#[derive(Parser)]
#[command(
    version,
    about = "Choose Pokémon names and display locally installed artwork"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Display an original or alternate-form illustration inline.
    Art {
        name: Option<String>,
        #[arg(long)]
        form: Option<String>,
        #[arg(long, value_enum, default_value_t)]
        mode: Mode,
        #[arg(long, value_name = "N|original")]
        width: Option<Width>,
        #[arg(long, value_enum, default_value_t)]
        color: Color,
    },
    /// Choose distinct original species names uniformly at random.
    Choose {
        #[arg(short, long)]
        count: usize,
        /// Exclude an exact species name, ignoring case. May be repeated.
        #[arg(short, long, value_name = "NAME")]
        exclude: Vec<String>,
        /// Exclude names occurring anywhere in these UTF-8 files, ignoring case.
        files: Vec<PathBuf>,
    },
    /// Refresh the catalogue and download missing artwork.
    Bootstrap {
        /// Redownload all artwork, including already downloaded files.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug)]
pub enum CommandError {
    Art(ArtError),
    Store(StoreError),
    Bootstrap(BootstrapError),
    File {
        path: PathBuf,
        source: std::io::Error,
    },
    Insufficient {
        requested: usize,
        available: usize,
    },
    Output(std::io::Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Art(error) => error.fmt(f),
            Self::Store(error) => error.fmt(f),
            Self::Bootstrap(error) => error.fmt(f),
            Self::File { path, source } => write!(f, "exclusion file {}: {source}", path.display()),
            Self::Insufficient {
                requested,
                available,
            } => write!(
                f,
                "cannot choose {requested} Pokémon: only {available} eligible species remain"
            ),
            Self::Output(error) => write!(f, "command output: {error}"),
        }
    }
}
impl std::error::Error for CommandError {}
impl From<ArtError> for CommandError {
    fn from(value: ArtError) -> Self {
        Self::Art(value)
    }
}
impl From<StoreError> for CommandError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}
impl From<BootstrapError> for CommandError {
    fn from(value: BootstrapError) -> Self {
        Self::Bootstrap(value)
    }
}

/// Read every file before selection, including when the requested count is zero.
///
/// # Errors
/// Returns an error identifying any unreadable or invalid UTF-8 file.
pub fn exclusion_text(files: &[PathBuf]) -> Result<Vec<String>, CommandError> {
    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .map(|text| text.to_lowercase())
                .map_err(|source| CommandError::File {
                    path: path.clone(),
                    source,
                })
        })
        .collect()
}

/// Sample species names without replacement; forms never add selection entries.
/// File contents must already be lowercase, as returned by `exclusion_text`.
///
/// # Errors
/// Returns an error when fewer eligible species remain than requested.
pub fn choose<'a>(
    catalogue: &'a Catalogue,
    count: usize,
    exclude: &[String],
    file_text: &[String],
    rng: &mut impl Rng,
) -> Result<Vec<&'a str>, CommandError> {
    let exclude: Vec<_> = exclude.iter().map(|name| name.to_lowercase()).collect();
    let mut candidates: Vec<_> = catalogue
        .species
        .iter()
        .filter_map(|species| {
            let name = species.name.to_lowercase();
            (!exclude.contains(&name) && !file_text.iter().any(|text| text.contains(&name)))
                .then_some(species.name.as_str())
        })
        .collect();
    if count > candidates.len() {
        return Err(CommandError::Insufficient {
            requested: count,
            available: candidates.len(),
        });
    }
    // A uniform shuffle followed by truncation gives every distinct ordered sample equal probability.
    candidates.shuffle(rng);
    candidates.truncate(count);
    Ok(candidates)
}

/// Dependencies are injected for isolated stores, local HTTP fixtures, and seeded sampling.
pub struct Runtime<R> {
    store: Store,
    bootstrap: Option<Bootstrap>,
    rng: R,
    terminal: Terminal,
}
impl<R: Rng + Send> Runtime<R> {
    #[must_use]
    pub fn new(store: Store, rng: R) -> Self {
        Self {
            store,
            bootstrap: None,
            rng,
            terminal: Terminal::from_stdio(),
        }
    }

    #[must_use]
    pub const fn with_terminal(mut self, terminal: Terminal) -> Self {
        self.terminal = terminal;
        self
    }

    #[must_use]
    pub fn with_bootstrap(mut self, bootstrap: Bootstrap) -> Self {
        self.bootstrap = Some(bootstrap);
        self
    }

    async fn execute(
        &mut self,
        command: Command,
        stdout: &mut (impl Write + Send),
        stderr: &mut (impl Write + Send),
    ) -> Result<(), CommandError> {
        match command {
            Command::Art {
                name,
                form,
                mode,
                width,
                color,
            } => {
                let catalogue = catalogue(&self.store, self.bootstrap.as_ref(), stderr).await?;
                let (_, selected) =
                    art::select(&catalogue, name.as_deref(), form.as_deref(), &mut self.rng)?;
                let bytes = self.store.read_artwork(&selected.local_artwork)?;
                let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                    .map_err(|error| StoreError::RepairRequired {
                        path: self.store.root().join(selected.local_artwork.as_str()),
                        reason: error.to_string(),
                    })?
                    .into_rgba8();
                if mode == Mode::Img {
                    return Err(CommandError::Output(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "native image unavailable; use --mode auto, blocks, or ascii",
                    )));
                }
                let blocks = mode == Mode::Blocks
                    || (mode == Mode::Auto && self.terminal.is_terminal && self.terminal.unicode);
                let prepared = art::prepare(
                    &image,
                    width.unwrap_or_default(),
                    self.terminal.geometry,
                    blocks,
                )
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::InvalidData {
                        CommandError::Store(StoreError::RepairRequired {
                            path: self.store.root().join(selected.local_artwork.as_str()),
                            reason: error.to_string(),
                        })
                    } else {
                        CommandError::Art(ArtError::from(error))
                    }
                })?;
                let text = crate::text_art::render(&prepared, blocks, self.terminal.color(color));
                stdout
                    .write_all(text.as_bytes())
                    .map_err(CommandError::Output)?;
            }
            Command::Choose {
                count,
                exclude,
                files,
            } => {
                let text = exclusion_text(&files)?;
                let catalogue = catalogue(&self.store, self.bootstrap.as_ref(), stderr).await?;
                let names = choose(&catalogue, count, &exclude, &text, &mut self.rng)?;
                for name in names {
                    writeln!(stdout, "{name}").map_err(CommandError::Output)?;
                }
            }
            Command::Bootstrap { force } => {
                bootstrap(&self.store, self.bootstrap.as_ref(), force, stderr).await?;
            }
        }
        Ok(())
    }
}

async fn catalogue(
    store: &Store,
    source: Option<&Bootstrap>,
    stderr: &mut (impl Write + Send),
) -> Result<Catalogue, CommandError> {
    match store.load_active()? {
        Some(catalogue) => Ok(catalogue),
        None => bootstrap(store, source, false, stderr).await,
    }
}

async fn bootstrap(
    store: &Store,
    source: Option<&Bootstrap>,
    force: bool,
    stderr: &mut (impl Write + Send),
) -> Result<Catalogue, CommandError> {
    let bootstrap = match source {
        Some(bootstrap) => bootstrap.clone(),
        None => Bootstrap::new()?,
    };
    Ok(bootstrap.run(store, force, stderr).await?)
}

/// Parse and run the product commands, returning their process exit status.
///
/// The factory is called only after parsing succeeds, so help, version, and invalid
/// arguments cannot access the store or create an HTTP client. Diagnostics use stderr.
pub async fn run<I, T, R>(
    args: I,
    runtime: impl FnOnce() -> Result<Runtime<R>, StoreError> + Send,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> u8
where
    I: IntoIterator<Item = T> + Send,
    T: Into<OsString> + Clone,
    R: Rng + Send,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            let code = if error.use_stderr() { 2 } else { 0 };
            let result = if error.use_stderr() {
                write!(stderr, "{error}")
            } else {
                write!(stdout, "{error}")
            };
            return if result.is_ok() { code } else { 1 };
        }
    };
    let result = match runtime() {
        Ok(mut runtime) => runtime.execute(cli.command, stdout, stderr).await,
        Err(error) => Err(CommandError::Store(error)),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}
