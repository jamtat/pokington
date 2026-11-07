//! Complete source discovery and recoverable artwork downloads. Ordinary data reads use Store.

use std::{fmt, io::Write, time::Duration};

use http_body_util::{BodyExt, Limited};
use tokio::task::JoinSet;

use crate::{
    catalogue::Catalogue,
    serebii::{self, INDEX_URL, ORIGIN},
    source::{self, SourceError},
    store::{Store, StoreError, validate_png},
};

const CONCURRENCY: usize = 4;
const ATTEMPTS: usize = 3;
const MAX_HTML_BYTES: usize = 4 * 1024 * 1024;
const MAX_PNG_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug)]
pub enum BootstrapError {
    Http { url: String, reason: String },
    Source(SourceError),
    Store(StoreError),
    Progress(std::io::Error),
    Task(tokio::task::JoinError),
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { url, reason } => write!(f, "{url}: {reason}"),
            Self::Source(error) => error.fmt(f),
            Self::Store(error) => error.fmt(f),
            Self::Progress(error) => write!(f, "bootstrap progress: {error}"),
            Self::Task(error) => write!(f, "bootstrap download task: {error}"),
        }
    }
}
impl std::error::Error for BootstrapError {}
impl From<SourceError> for BootstrapError {
    fn from(value: SourceError) -> Self {
        Self::Source(value)
    }
}
impl From<StoreError> for BootstrapError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

#[derive(Clone, Copy)]
enum Content {
    Html,
    Png,
}

#[derive(Clone)]
pub struct Bootstrap {
    client: wreq::Client,
    origin: String,
}

impl Bootstrap {
    /// Create a bootstrap client using the canonical Serebii origin.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new() -> Result<Self, BootstrapError> {
        Ok(Self::with_http(
            source::http_client().map_err(|error| BootstrapError::Http {
                url: ORIGIN.into(),
                reason: error.to_string(),
            })?,
            ORIGIN,
        ))
    }

    /// Map canonical source paths onto an injected server, for deterministic HTTP fixtures.
    /// Stored URLs and source identities always remain the canonical HTTPS Serebii URLs.
    #[must_use]
    pub fn with_http(client: wreq::Client, origin: &str) -> Self {
        Self {
            client,
            origin: origin.trim_end_matches('/').to_owned(),
        }
    }

    /// The caller supplies stderr (or a capture in tests); this method never writes stdout.
    /// Plain bar snapshots also work when stderr is redirected, without terminal controls.
    ///
    /// # Errors
    ///
    /// Returns an error if source requests, catalogue reconciliation, storage operations, or progress writes fail.
    pub async fn run(
        &self,
        store: &Store,
        force: bool,
        progress: &mut impl Write,
    ) -> Result<Catalogue, BootstrapError> {
        let mut writer = store.try_writer()?;
        let mut progress = Progress(progress);
        progress.download(INDEX_URL)?;
        let index = serebii::parse_index(&self.html(INDEX_URL).await?)?;
        progress.bar("Index", 1, 1)?;
        let mut records = Vec::with_capacity(index.species.len());
        progress.bar("Species", 0, index.species.len())?;
        for batch in index.species.chunks(CONCURRENCY) {
            let mut jobs = JoinSet::new();
            for species in batch {
                let url = format!("{ORIGIN}{}", species.detail_path);
                progress.download(&url)?;
                let source = self.clone();
                let species = species.clone();
                jobs.spawn(
                    async move { Ok::<_, BootstrapError>((species, source.html(&url).await?)) },
                );
            }
            while let Some(result) = jobs.join_next().await {
                let (link, html) = result.map_err(BootstrapError::Task)??;
                let hub = serebii::parse_hub(&html, &link)?;
                let mut evidence = Vec::new();
                let mut record = serebii::reconcile(&hub, &evidence);
                // Older linked games can independently identify forms absent from current games.
                for path in &hub.game_links {
                    if record.is_ok() {
                        break;
                    }
                    let url = format!("{ORIGIN}{path}");
                    progress.download(&url)?;
                    evidence.push(serebii::parse_enrichment(
                        &hub,
                        path,
                        &self.html(&url).await?,
                    )?);
                    record = serebii::reconcile(&hub, &evidence);
                }
                records.push(record?);
                progress.bar("Species", records.len(), index.species.len())?;
            }
        }
        records.sort_by_key(|species| species.id);
        writer.prepare(&index, records, force)?;
        if !force && let Ok(Some(active)) = store.load_active() {
            writer.reuse_active(&active)?;
        }
        let checkpoint = writer.checkpoint()?;
        let total = checkpoint.catalogue().forms().count();
        let mut done = checkpoint.completed().len();
        let downloads: Vec<_> = checkpoint
            .catalogue()
            .forms()
            .filter(|form| !checkpoint.completed().contains(&form.local_artwork))
            .map(|form| (form.local_artwork.clone(), form.artwork_url.clone()))
            .collect();
        progress.bar("Artwork", done, total)?;
        for batch in downloads.chunks(CONCURRENCY) {
            let mut jobs = JoinSet::new();
            for (reference, url) in batch {
                progress.download(url)?;
                let source = self.clone();
                let reference = reference.clone();
                let url = url.clone();
                jobs.spawn(async move { (reference, source.fetch(&url, Content::Png).await) });
            }
            // Finish successful downloads in this batch even if another request fails.
            let mut failure = None;
            while let Some(result) = jobs.join_next().await {
                let saved = match result {
                    Ok((reference, Ok(bytes))) => writer
                        .write_artwork(&reference, &bytes)
                        .map_err(BootstrapError::from),
                    Ok((_, Err(error))) => Err(error),
                    Err(error) => Err(BootstrapError::Task(error)),
                };
                match saved {
                    Ok(()) => {
                        done = done.saturating_add(1);
                        progress.bar("Artwork", done, total)?;
                    }
                    Err(error) => {
                        failure.get_or_insert(error);
                    }
                }
            }
            if let Some(error) = failure {
                return Err(error);
            }
        }
        let catalogue = writer.publish()?;
        report_complete(&mut progress, &catalogue)?;
        Ok(catalogue)
    }

    async fn html(&self, url: &str) -> Result<String, BootstrapError> {
        // Serebii's source pages use ISO-8859-1; HTML entities are handled by the parser.
        Ok(self
            .fetch(url, Content::Html)
            .await?
            .into_iter()
            .map(char::from)
            .collect())
    }

    async fn fetch(&self, url: &str, content: Content) -> Result<Vec<u8>, BootstrapError> {
        let path = url
            .strip_prefix(ORIGIN)
            .filter(|path| path.starts_with('/'))
            .ok_or_else(|| {
                BootstrapError::Source(SourceError(format!("unsupported source URL: {url}")))
            })?;
        let request_url = format!("{}{path}", self.origin);
        let mut reason = String::new();
        for attempt in 1..=ATTEMPTS {
            match self.fetch_once(&request_url, content).await {
                Ok(bytes) => return Ok(bytes),
                Err(error) => reason = error,
            }
            if attempt < ATTEMPTS {
                let delay = u64::try_from(attempt)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(100);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }
        Err(BootstrapError::Http {
            url: url.into(),
            reason: format!("{reason} (after {ATTEMPTS} attempts)"),
        })
    }

    async fn fetch_once(&self, url: &str, content: Content) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if response.status() != wreq::StatusCode::OK {
            return Err(format!("HTTP {}", response.status()));
        }
        if !response
            .uri()
            .to_string()
            .starts_with(&format!("{}/", self.origin))
        {
            return Err("unexpected redirect origin".into());
        }
        let mime = response
            .headers()
            .get(wreq::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let expected = match content {
            Content::Html => "text/html",
            Content::Png => "image/png",
        };
        if !mime.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "expected {expected}, received Content-Type {mime:?}"
            ));
        }
        let limit = match content {
            Content::Html => MAX_HTML_BYTES,
            Content::Png => MAX_PNG_BYTES,
        };
        let bytes = Limited::new(response, limit)
            .collect()
            .await
            .map_err(|e| e.to_string())?
            .to_bytes()
            .to_vec();
        if bytes.is_empty() {
            return Err("empty response body".into());
        }
        if matches!(content, Content::Png) {
            validate_png(&bytes)?;
        }
        Ok(bytes)
    }
}

struct Progress<'a, W>(&'a mut W);
impl<W: Write> Progress<'_, W> {
    fn line(&mut self, message: &str) -> Result<(), BootstrapError> {
        writeln!(self.0, "{message}")
            .and_then(|()| self.0.flush())
            .map_err(BootstrapError::Progress)
    }
    fn download(&mut self, url: &str) -> Result<(), BootstrapError> {
        self.line(&format!("Downloading {url}"))
    }
    fn bar(&mut self, label: &str, done: usize, total: usize) -> Result<(), BootstrapError> {
        let filled = 24_usize
            .saturating_mul(done)
            .checked_div(total)
            .unwrap_or(24)
            .min(24);
        self.line(&format!(
            "{label} [{}{}] {done}/{total}",
            "#".repeat(filled),
            "-".repeat(24_usize.saturating_sub(filled))
        ))
    }
}

fn report_complete<W: Write>(
    progress: &mut Progress<'_, W>,
    catalogue: &Catalogue,
) -> Result<(), BootstrapError> {
    let excluded = catalogue
        .species
        .iter()
        .map(|species| species.excluded_illustrations.len())
        .sum::<usize>();
    progress.line(&format!(
        "Complete: {} species, {} forms, {excluded} excluded illustrations",
        catalogue.species.len(),
        catalogue.forms().count(),
    ))
}
