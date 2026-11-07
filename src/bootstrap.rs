//! Complete source discovery and recoverable artwork downloads. Ordinary data reads use Store.

use std::{
    collections::VecDeque, ffi::OsStr, fmt, io::Write, num::NonZeroUsize, sync::Arc, time::Duration,
};

use tokio::{
    sync::{Mutex, Notify, mpsc},
    task::JoinSet,
};

use crate::{
    catalogue::{ArtworkRef, Catalogue},
    serebii::{self, INDEX_URL, ORIGIN},
    source::{self, SourceError},
    store::{Store, StoreError, Writer, validate_png},
};

const DEFAULT_CONCURRENCY: usize = 4;
const ATTEMPTS: u64 = 3;
const MAX_HTML_BYTES: usize = 4 * 1024 * 1024;
const MAX_PNG_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug)]
pub enum BootstrapError {
    Configuration(String),
    Http { url: String, reason: String },
    Source(SourceError),
    Store(StoreError),
    Progress(std::io::Error),
    Task(tokio::task::JoinError),
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(reason) => f.write_str(reason),
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
    client: reqwest::Client,
    origin: String,
    concurrency: Option<NonZeroUsize>,
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
    pub fn with_http(client: reqwest::Client, origin: &str) -> Self {
        Self {
            client,
            origin: origin.trim_end_matches('/').to_owned(),
            concurrency: None,
        }
    }

    /// Override the worker count; otherwise `POKINGTON_CONCURRENCY` supplies it.
    #[must_use]
    pub const fn with_concurrency(mut self, concurrency: NonZeroUsize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }

    /// The caller supplies stderr (or a capture in tests); this method never writes stdout.
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
        let concurrency = match self.concurrency {
            Some(value) => value.get(),
            None => concurrency(std::env::var_os("POKINGTON_CONCURRENCY").as_deref())?,
        };
        let mut discovery = Discovery {
            writer: store.try_writer()?,
            store,
            force,
            index: None,
            species: Vec::new(),
        };
        let mut progress = Progress(progress);
        let queue = Arc::new(TaskQueue::new());
        let (sender, mut receiver) = mpsc::channel(DEFAULT_CONCURRENCY);
        let mut workers = JoinSet::new();
        for _ in 0..concurrency {
            workers.spawn(worker(self.clone(), queue.clone(), sender.clone()));
        }
        drop(sender);
        let mut failure = None;
        loop {
            let result = tokio::select! {
                event = receiver.recv() => match event {
                    Some(WorkerEvent::Downloading(url)) => progress.download(&url),
                    Some(WorkerEvent::Finished(result)) => {
                        let result = result.and_then(|output| {
                            discovery.accept(output, failure.is_none())
                        });
                        let tasks = match result {
                            Ok(tasks) => tasks,
                            Err(error) => {
                                failure.get_or_insert(error);
                                queue.stop().await;
                                Vec::new()
                            }
                        };
                        queue.complete(tasks).await;
                        Ok(())
                    }
                    None => break,
                },
                result = workers.join_next(), if !workers.is_empty() => {
                    result.transpose().map(|_| ()).map_err(BootstrapError::Task)
                }
            };
            if let Err(error) = result {
                failure.get_or_insert(error);
                queue.stop().await;
            }
        }
        while let Some(result) = workers.join_next().await {
            if let Err(error) = result {
                failure.get_or_insert(BootstrapError::Task(error));
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        let catalogue = discovery.writer.publish()?;
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
        self.fetch_once(&request_url, content)
            .await
            .map_err(|reason| BootstrapError::Http {
                url: url.into(),
                reason,
            })
    }

    async fn fetch_once(&self, url: &str, content: Content) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(format!("HTTP {}", response.status()));
        }
        if !response
            .url()
            .to_string()
            .starts_with(&format!("{}/", self.origin))
        {
            return Err("unexpected redirect origin".into());
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
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
        let bytes = source::read_limited(response, limit)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Err("empty response body".into());
        }
        if matches!(content, Content::Png) {
            validate_png(&bytes)?;
        }
        Ok(bytes)
    }
}

fn concurrency(value: Option<&OsStr>) -> Result<usize, BootstrapError> {
    let Some(value) = value else {
        return Ok(DEFAULT_CONCURRENCY);
    };
    value
        .to_str()
        .and_then(|value| value.parse::<NonZeroUsize>().ok())
        .map(NonZeroUsize::get)
        .ok_or_else(|| {
            BootstrapError::Configuration("POKINGTON_CONCURRENCY must be a positive integer".into())
        })
}

// Task variants describe work only. Retry bookkeeping belongs to the queue entry.
enum BootstrapTask {
    NationalPokedex,
    Pokemon(serebii::SpeciesLink),
    GamePage {
        hub: Arc<serebii::Hub>,
        evidence: Vec<serebii::Enrichment>,
        path: String,
    },
    Artwork {
        reference: ArtworkRef,
        url: String,
    },
}

struct BootstrapTaskResult {
    data: BootstrapData,
    tasks: Vec<BootstrapTask>,
}

enum BootstrapData {
    Index(serebii::Index),
    Species(serebii::Species),
    Artwork(ArtworkRef, Vec<u8>),
    None,
}

impl BootstrapTask {
    fn url(&self) -> String {
        match self {
            Self::NationalPokedex => INDEX_URL.into(),
            Self::Pokemon(link) => format!("{ORIGIN}{}", link.detail_path),
            Self::GamePage { path, .. } => format!("{ORIGIN}{path}"),
            Self::Artwork { url, .. } => url.clone(),
        }
    }

    async fn execute(&mut self, source: &Bootstrap) -> Result<BootstrapTaskResult, BootstrapError> {
        let url = self.url();
        match self {
            Self::NationalPokedex => {
                let index = serebii::parse_index(&source.html(&url).await?)?;
                let tasks = index.species.iter().cloned().map(Self::Pokemon).collect();
                Ok(BootstrapTaskResult {
                    data: BootstrapData::Index(index),
                    tasks,
                })
            }
            Self::Pokemon(link) => {
                let hub = serebii::parse_hub(&source.html(&url).await?, link)?;
                reconcile(Arc::new(hub), Vec::new())
            }
            Self::GamePage {
                hub,
                evidence,
                path,
            } => {
                evidence.push(serebii::parse_enrichment(
                    hub,
                    path,
                    &source.html(&url).await?,
                )?);
                reconcile(hub.clone(), std::mem::take(evidence))
            }
            Self::Artwork { reference, .. } => Ok(BootstrapTaskResult {
                data: BootstrapData::Artwork(
                    reference.clone(),
                    source.fetch(&url, Content::Png).await?,
                ),
                tasks: Vec::new(),
            }),
        }
    }
}

fn reconcile(
    hub: Arc<serebii::Hub>,
    evidence: Vec<serebii::Enrichment>,
) -> Result<BootstrapTaskResult, BootstrapError> {
    match serebii::reconcile(&hub, &evidence) {
        Ok(species) => Ok(BootstrapTaskResult {
            data: BootstrapData::Species(species),
            tasks: Vec::new(),
        }),
        Err(error) => {
            let path = hub.game_links.get(evidence.len()).ok_or(error)?.clone();
            Ok(BootstrapTaskResult {
                data: BootstrapData::None,
                tasks: vec![BootstrapTask::GamePage {
                    hub,
                    evidence,
                    path,
                }],
            })
        }
    }
}

struct QueuedBootstrapTask {
    task: BootstrapTask,
    attempts: u64,
}

impl QueuedBootstrapTask {
    async fn execute(&mut self, source: &Bootstrap) -> Result<BootstrapTaskResult, BootstrapError> {
        loop {
            self.attempts = self.attempts.saturating_add(1);
            match self.task.execute(source).await {
                Err(BootstrapError::Http { url, reason }) => {
                    if self.attempts == ATTEMPTS {
                        return Err(BootstrapError::Http {
                            url,
                            reason: format!("{reason} (after {ATTEMPTS} attempts)"),
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(self.attempts.saturating_mul(100)))
                        .await;
                }
                result => return result,
            }
        }
    }
}

struct TaskQueue {
    state: Mutex<QueueState>,
    ready: Notify,
}

struct QueueState {
    tasks: VecDeque<QueuedBootstrapTask>,
    active: usize,
    stopped: bool,
}

impl TaskQueue {
    fn new() -> Self {
        Self {
            state: Mutex::new(QueueState {
                tasks: VecDeque::from([QueuedBootstrapTask {
                    task: BootstrapTask::NationalPokedex,
                    attempts: 0,
                }]),
                active: 0,
                stopped: false,
            }),
            ready: Notify::new(),
        }
    }

    async fn pop(&self) -> Option<QueuedBootstrapTask> {
        loop {
            // Register before checking the queue so enqueue/stop cannot lose a wakeup.
            let ready = self.ready.notified();
            let mut state = self.state.lock().await;
            if state.stopped {
                return None;
            }
            if let Some(task) = state.tasks.pop_front() {
                state.active = state.active.saturating_add(1);
                return Some(task);
            }
            drop(state);
            ready.await;
        }
    }

    async fn complete(&self, tasks: Vec<BootstrapTask>) {
        let mut state = self.state.lock().await;
        // Include finished tasks until their results and follow-up work have been applied.
        state.active = state.active.saturating_sub(1);
        if !state.stopped {
            state.tasks.extend(
                tasks
                    .into_iter()
                    .map(|task| QueuedBootstrapTask { task, attempts: 0 }),
            );
            state.stopped = state.active == 0 && state.tasks.is_empty();
        }
        drop(state);
        self.ready.notify_waiters();
    }

    async fn stop(&self) {
        let mut state = self.state.lock().await;
        state.stopped = true;
        state.tasks.clear();
        drop(state);
        self.ready.notify_waiters();
    }
}

enum WorkerEvent {
    Downloading(String),
    Finished(Result<BootstrapTaskResult, BootstrapError>),
}

async fn worker(source: Bootstrap, queue: Arc<TaskQueue>, sender: mpsc::Sender<WorkerEvent>) {
    while let Some(mut task) = queue.pop().await {
        if sender
            .send(WorkerEvent::Downloading(task.task.url()))
            .await
            .is_err()
        {
            return;
        }
        let result = task.execute(&source).await;
        if result.is_err() {
            queue.stop().await;
        }
        if sender.send(WorkerEvent::Finished(result)).await.is_err() {
            return;
        }
    }
}

struct Discovery<'a> {
    writer: Writer,
    store: &'a Store,
    force: bool,
    index: Option<serebii::Index>,
    species: Vec<serebii::Species>,
}

impl Discovery<'_> {
    fn accept(
        &mut self,
        output: BootstrapTaskResult,
        admit: bool,
    ) -> Result<Vec<BootstrapTask>, BootstrapError> {
        match output.data {
            BootstrapData::Artwork(reference, bytes) => {
                self.writer.write_artwork(&reference, &bytes)?;
            }
            _ if !admit => return Ok(Vec::new()),
            BootstrapData::Index(index) => self.index = Some(index),
            BootstrapData::Species(species) => {
                self.species.push(species);
                if let Some(index) = &self.index
                    && self.species.len() == index.species.len()
                {
                    self.species.sort_by_key(|species| species.id);
                    self.writer
                        .prepare(index, std::mem::take(&mut self.species), self.force)?;
                    if !self.force
                        && let Ok(Some(active)) = self.store.load_active()
                    {
                        self.writer.reuse_active(&active)?;
                    }
                    let checkpoint = self.writer.checkpoint()?;
                    return Ok(checkpoint
                        .catalogue()
                        .forms()
                        .filter(|form| !checkpoint.completed().contains(&form.local_artwork))
                        .map(|form| BootstrapTask::Artwork {
                            reference: form.local_artwork.clone(),
                            url: form.artwork_url.clone(),
                        })
                        .collect());
                }
            }
            BootstrapData::None => (),
        }
        Ok(output.tasks)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn worker_failure_stops_dequeue_and_rejects_follow_up_tasks() {
        let queue = Arc::new(TaskQueue::new());
        // Replace the seed with two tasks that fail before making any HTTP request.
        let _seed = queue.pop().await.unwrap();
        let task = || BootstrapTask::Artwork {
            reference: ArtworkRef::new("artwork/00000000000000000000000000000000-1.png".into())
                .unwrap(),
            url: "unsupported-source".into(),
        };
        queue.complete(vec![task(), task()]).await;
        let (sender, mut receiver) = mpsc::channel(DEFAULT_CONCURRENCY);
        worker(Bootstrap::new().unwrap(), queue.clone(), sender).await;
        assert!(matches!(
            receiver.recv().await,
            Some(WorkerEvent::Downloading(_))
        ));
        assert!(matches!(
            receiver.recv().await,
            Some(WorkerEvent::Finished(Err(BootstrapError::Source(_))))
        ));
        assert!(
            receiver.recv().await.is_none(),
            "the second task must not start"
        );
        queue.complete(vec![task()]).await;
        assert!(queue.pop().await.is_none());
        assert!(queue.state.lock().await.tasks.is_empty());
    }
}
