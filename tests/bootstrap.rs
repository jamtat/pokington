#![cfg(unix)]

use std::{
    collections::{BTreeMap, VecDeque},
    env,
    error::Error,
    ffi::OsString,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use pokington::{
    bootstrap::Bootstrap,
    catalogue::Catalogue,
    source::http_client_builder,
    store::{Store, validate_png},
};

const INDEX: &str = include_str!("fixtures/serebii-index.html");
const HUB: &str = include_str!("fixtures/serebii-charizard.html");
const INDEX_PATH: &str = "/pokemon/nationalpokedex.shtml";
const IMAGES: [&str; 4] = ["006.png", "006-mx.png", "006-my.png", "006-gi.png"];
static NEXT: AtomicUsize = AtomicUsize::new(0);
type Failure = Box<dyn Error + Send + Sync>;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Result<Self, Failure> {
        let path = env::temp_dir().join(format!(
            "pokington-bootstrap-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    fn store(&self) -> Result<Store, Failure> {
        Ok(Store::at(self.0.clone())?)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct Response {
    status: u16,
    mime: String,
    body: Vec<u8>,
    length: usize,
    delay: Duration,
    gate: Option<Arc<Gate>>,
}
impl Response {
    fn new(status: u16, mime: &str, body: Vec<u8>) -> Self {
        Self {
            status,
            mime: mime.into(),
            length: body.len(),
            body,
            delay: Duration::ZERO,
            gate: None,
        }
    }
    fn html(value: &str) -> Result<Self, Failure> {
        Ok(Self::new(
            200,
            "text/html; charset=iso-8859-1",
            value
                .chars()
                .map(u8::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
    fn png() -> Result<Self, Failure> {
        let mut body = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut body, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()?
                .write_image_data(&[255, 0, 0, 255, 0, 0, 0, 0])?;
        }
        Ok(Self::new(200, "image/png", body))
    }
}

#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) -> Result<(), Failure> {
        drop(
            self.changed
                .wait_while(self.open.lock().map_err(|_| "gate poisoned")?, |open| {
                    !*open
                })
                .map_err(|_| "gate poisoned")?,
        );
        Ok(())
    }

    fn release(&self) {
        *self
            .open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        self.changed.notify_all();
    }
}

#[derive(Default)]
struct State {
    routes: BTreeMap<String, VecDeque<Response>>,
    hits: Vec<String>,
    active_images: usize,
    max_images: usize,
    active: usize,
    max_active: usize,
}
struct Server {
    origin: String,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<Result<(), Failure>>>,
}
impl Server {
    fn new() -> Result<Self, Failure> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let origin = format!("http://{}", listener.local_addr()?);
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = state.clone();
        let worker_stop = stop.clone();
        let thread = thread::spawn(move || -> Result<(), Failure> {
            let mut workers = Vec::new();
            let mut listener_error = None;
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let state = worker_state.clone();
                        workers.push(thread::spawn(move || serve(stream, &state)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => {
                        listener_error = Some(error);
                        break;
                    }
                }
            }
            for worker in workers {
                worker.join().map_err(|_| "fixture worker panicked")??;
            }
            if let Some(error) = listener_error {
                return Err(error.into());
            }
            Ok(())
        });
        Ok(Self {
            origin,
            state,
            stop,
            thread: Some(thread),
        })
    }
    fn route(&self, path: &str, responses: Vec<Response>) -> Result<(), Failure> {
        if responses.is_empty() {
            return Err("missing fixture route responses".into());
        }
        self.state
            .lock()
            .map_err(|_| "fixture state poisoned")?
            .routes
            .insert(path.into(), responses.into());
        Ok(())
    }
    fn fixture() -> Result<Self, Failure> {
        let server = Self::new()?;
        server.route(INDEX_PATH, vec![Response::html(INDEX)?])?;
        server.route("/pokemon/charizard", vec![Response::html(HUB)?])?;
        for image in IMAGES {
            server.route(&format!("/pokemon/art/{image}"), vec![Response::png()?])?;
        }
        Ok(server)
    }
    fn source(&self) -> Result<Bootstrap, Failure> {
        Ok(Bootstrap::with_http(
            http_client_builder().https_only(false).build()?,
            &self.origin,
        ))
    }
    fn many_species() -> Result<Self, Failure> {
        let server = Self::new()?;
        let row = INDEX
            .get(
                INDEX.find("<tr>\n").ok_or("missing index row")?
                    ..INDEX.rfind("</table></body>").ok_or("missing table end")?,
            )
            .ok_or("invalid row")?;
        let mut rows = String::new();
        for id in 6..=11 {
            rows.push_str(
                &row.replace("0006", &format!("{id:04}"))
                    .replace("Charizard", &format!("Pokemon{id}"))
                    .replace("charizard", &format!("pokemon{id}")),
            );
            let hub = original_hub()?
                .replace("006", &format!("{id:03}"))
                .replace("Charizard", &format!("Pokemon{id}"))
                .replace("charizard", &format!("pokemon{id}"));
            server.route(
                &format!("/pokemon/pokemon{id}"),
                vec![Response::html(&hub)?],
            )?;
            server.route(&format!("/pokemon/art/{id:03}.png"), vec![Response::png()?])?;
        }
        server.route(
            INDEX_PATH,
            vec![Response::html(
                &INDEX.replace(row, &rows).replace("006-006", "006-011"),
            )?],
        )?;
        Ok(server)
    }

    fn hold(&self, path: &str) -> Result<Arc<Gate>, Failure> {
        let gate = Arc::new(Gate::default());
        self.state
            .lock()
            .map_err(|_| "fixture state poisoned")?
            .routes
            .get_mut(path)
            .ok_or("missing held route")?
            .back_mut()
            .ok_or("missing held response")?
            .gate = Some(gate.clone());
        Ok(gate)
    }
    fn hits(&self, path: &str) -> Result<usize, Failure> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "fixture state poisoned")?
            .hits
            .iter()
            .filter(|hit| hit.as_str() == path)
            .count())
    }
    fn image_hits(&self) -> Result<usize, Failure> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "fixture state poisoned")?
            .hits
            .iter()
            .filter(|hit| hit.starts_with("/pokemon/art/"))
            .count())
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Unblock held fixture responses even when an ordering assertion panics.
        for response in self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .routes
            .values()
            .flatten()
        {
            if let Some(gate) = &response.gate {
                gate.release();
            }
        }
        if let Some(thread) = self.thread.take() {
            assert!(matches!(thread.join(), Ok(Ok(()))), "fixture server failed");
        }
    }
}
fn serve(mut stream: TcpStream, state: &Mutex<State>) -> Result<(), Failure> {
    // Accepted sockets inherit the listener's nonblocking mode on macOS.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::new();
    let mut bytes = [0; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut bytes) {
            Ok(0) | Err(_) => return Ok(()),
            Ok(n) => request.extend_from_slice(bytes.get(..n).ok_or("invalid read length")?),
        }
    }
    let request = String::from_utf8(request)?;
    let path = request
        .split_whitespace()
        .nth(1)
        .ok_or("missing request path")?
        .to_owned();
    let is_image = path.starts_with("/pokemon/art/");
    let response = {
        let mut state = state.lock().map_err(|_| "fixture state poisoned")?;
        state.hits.push(path.clone());
        state.active = state
            .active
            .checked_add(1)
            .ok_or("active request count overflow")?;
        state.max_active = state.max_active.max(state.active);
        if is_image {
            state.active_images = state
                .active_images
                .checked_add(1)
                .ok_or("active image count overflow")?;
            state.max_images = state.max_images.max(state.active_images);
        }
        match state.routes.get_mut(&path) {
            Some(responses) if responses.len() > 1 => {
                responses.pop_front().ok_or("missing queued response")?
            }
            Some(responses) => responses.front().ok_or("missing response")?.clone(),
            None => Response::new(404, "text/html", b"missing fixture route".to_vec()),
        }
    };
    if let Some(gate) = &response.gate {
        gate.wait()?;
    }
    thread::sleep(response.delay);
    let header = format!(
        "HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status, response.mime, response.length
    );
    let mut state = state.lock().map_err(|_| "fixture state poisoned")?;
    let _ = stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(&response.body));
    state.active = state
        .active
        .checked_sub(1)
        .ok_or("active request count underflow")?;
    if is_image {
        state.active_images = state
            .active_images
            .checked_sub(1)
            .ok_or("active image count underflow")?;
    }
    drop(state);
    Ok(())
}

fn installed(store: &Store, expected: &Catalogue) -> Result<(), Failure> {
    if store.load_active()?.as_ref() != Some(expected) {
        return Err("installed catalogue mismatch".into());
    }
    for form in expected.forms() {
        validate_png(&store.read_artwork(&form.local_artwork)?)?;
    }
    Ok(())
}

fn original_hub() -> Result<String, Failure> {
    let (prefix, _) = HUB
        .split_once("</table></div><table")
        .ok_or("missing form table")?;
    Ok(format!(
        "{prefix}</table><div class=\"hubheader\"><a href=\"/pokedex-sv/charizard/\">Game database</a></div></body></html>"
    ))
}

#[tokio::test]
async fn complete_initial_and_refresh_skip_validate_artwork_and_log_downloads() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let source = server.source().unwrap();
    let mut output = Vec::new();
    let catalogue = source
        .run(&temp.store().unwrap(), false, &mut output)
        .await
        .unwrap();
    assert_eq!(catalogue.species.len(), 1);
    assert_eq!(catalogue.forms().count(), 4);
    installed(&temp.store().unwrap(), &catalogue).unwrap();
    assert!(!temp.0.join("pending.json").exists());
    assert_eq!(server.image_hits().unwrap(), 4);
    assert_eq!(
        server.hits("/pokemon/art/006-g.png").unwrap(),
        0,
        "unselected alias must not be guessed or fetched"
    );
    let refreshed = source
        .run(&temp.store().unwrap(), false, &mut output)
        .await
        .unwrap();
    assert_eq!(refreshed, catalogue);
    assert_eq!(server.image_hits().unwrap(), 4);
    assert_eq!(server.hits(INDEX_PATH).unwrap(), 2);
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Downloading https://www.serebii.net/pokemon/nationalpokedex.shtml"));
    assert!(!output.contains("[####"));
    assert!(output.contains("Complete: 1 species, 4 forms, 0 excluded illustrations"));
    assert!(!output.contains(['\x1b', '\r']));
}

#[tokio::test]
async fn failed_initial_reuses_completed_files_on_retry() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let missing = "/pokemon/art/006-my.png";
    server
        .route(
            missing,
            vec![Response::new(503, "text/html", b"unavailable".to_vec())],
        )
        .unwrap();
    let source = server.source().unwrap();
    let error = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("503"));
    assert!(temp.store().unwrap().load_active().unwrap().is_none());
    assert!(temp.0.join("pending.json").exists());
    assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 3);
    assert_eq!(server.hits(missing).unwrap(), 3);
    server
        .route(missing, vec![Response::png().unwrap()])
        .unwrap();
    let catalogue = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    installed(&temp.store().unwrap(), &catalogue).unwrap();
    assert_eq!(
        server.image_hits().unwrap(),
        7,
        "three complete images are not downloaded again"
    );
}

#[tokio::test]
async fn force_failure_preserves_active_retry_resumes_and_next_force_is_fresh() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let source = server.source().unwrap();
    let store = temp.store().unwrap();
    let original = source.run(&store, false, &mut Vec::new()).await.unwrap();
    let broken = "/pokemon/art/006.png";
    server
        .route(
            broken,
            vec![Response::new(200, "image/png", b"broken PNG".to_vec())],
        )
        .unwrap();
    assert!(source.run(&store, true, &mut Vec::new()).await.is_err());
    installed(&store, &original).unwrap();
    server
        .route(broken, vec![Response::png().unwrap()])
        .unwrap();
    let replacement = source.run(&store, true, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits().unwrap(), 11);
    for (old, new) in original.forms().zip(replacement.forms()) {
        assert_ne!(old.local_artwork, new.local_artwork);
    }
    for form in original.forms() {
        validate_png(&store.read_artwork(&form.local_artwork).unwrap()).unwrap();
    }
    assert_eq!(store.load_active().unwrap(), Some(replacement.clone()));
    let fresh = source.run(&store, true, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits().unwrap(), 15);
    for (old, new) in replacement.forms().zip(fresh.forms()) {
        assert_ne!(old.local_artwork, new.local_artwork);
    }
    installed(&store, &fresh).unwrap();
}

#[tokio::test]
async fn refresh_discovers_new_forms_and_repairs_only_missing_or_corrupt_images() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    server
        .route(
            "/pokemon/charizard",
            vec![Response::html(&original_hub().unwrap()).unwrap()],
        )
        .unwrap();
    let source = server.source().unwrap();
    let store = temp.store().unwrap();
    let first = source.run(&store, false, &mut Vec::new()).await.unwrap();
    assert_eq!(first.forms().count(), 1);
    server
        .route("/pokemon/charizard", vec![Response::html(HUB).unwrap()])
        .unwrap();
    let second = source.run(&store, false, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits().unwrap(), 4);
    assert_eq!(
        first.forms().next().unwrap().local_artwork,
        second
            .forms()
            .find(|f| f.key == "original")
            .unwrap()
            .local_artwork
    );
    let forms: Vec<_> = second.forms().collect();
    fs::remove_file(store.root().join(forms[0].local_artwork.as_str())).unwrap();
    fs::write(
        store.root().join(forms[1].local_artwork.as_str()),
        b"corrupt",
    )
    .unwrap();
    let third = source.run(&store, false, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits().unwrap(), 6);
    installed(&store, &third).unwrap();
}

#[tokio::test]
async fn refresh_discovers_new_species_from_the_extended_source_index() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    server
        .route(
            "/pokemon/charizard",
            vec![Response::html(&original_hub().unwrap()).unwrap()],
        )
        .unwrap();
    let source = server.source().unwrap();
    let first = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    let row = INDEX
        .get(INDEX.find("<tr>\n").unwrap()..INDEX.rfind("</table></body>").unwrap())
        .unwrap();
    let new_row = row
        .replace("#0006", "#0007")
        .replace("/pokemon/charizard", "/pokemon/squirtle")
        .replace("Charizard", "Squirtle");
    let extended = INDEX
        .replace("006-006", "006-007")
        .replace("</table></body>", &format!("{new_row}</table></body>"));
    let new_hub = original_hub()
        .unwrap()
        .replace("#0006", "#0007")
        .replace("Charizard", "Squirtle")
        .replace("charizard", "squirtle")
        .replace("006.png", "007.png");
    server
        .route(INDEX_PATH, vec![Response::html(&extended).unwrap()])
        .unwrap();
    server
        .route("/pokemon/squirtle", vec![Response::html(&new_hub).unwrap()])
        .unwrap();
    server
        .route("/pokemon/art/007.png", vec![Response::png().unwrap()])
        .unwrap();
    let second = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(second.source.max_id, 7);
    assert_eq!(second.species.len(), 2);
    assert_eq!(second.species[1].name, "Squirtle");
    assert_eq!(server.image_hits().unwrap(), 2);
    assert_eq!(first.species[0].forms, second.species[0].forms);
    installed(&temp.store().unwrap(), &second).unwrap();
}

#[tokio::test]
async fn transient_http_errors_are_retried_with_bounded_concurrency() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let mut slow = Response::png().unwrap();
    slow.delay = Duration::from_millis(40);
    for image in IMAGES {
        server
            .route(&format!("/pokemon/art/{image}"), vec![slow.clone()])
            .unwrap();
    }
    server
        .route(
            "/pokemon/art/006.png",
            vec![
                Response::new(429, "text/html", b"busy".to_vec()),
                Response::new(500, "text/html", b"busy".to_vec()),
                slow,
            ],
        )
        .unwrap();
    let catalogue = server
        .source()
        .unwrap()
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    installed(&temp.store().unwrap(), &catalogue).unwrap();
    assert_eq!(server.hits("/pokemon/art/006.png").unwrap(), 3);
    let max = server.state.lock().unwrap().max_images;
    assert!(
        (2..=4).contains(&max),
        "observed {max} concurrent image requests"
    );
}

#[tokio::test]
async fn free_workers_start_the_fifth_species_and_artwork_without_waiting_for_the_first() {
    for (held, fifth) in [
        ("/pokemon/pokemon6", "/pokemon/pokemon10"),
        ("/pokemon/art/006.png", "/pokemon/art/010.png"),
    ] {
        let temp = Temp::new().unwrap();
        let server = Server::many_species().unwrap();
        let gate = server.hold(held).unwrap();
        let source = server.source().unwrap();
        let store = temp.store().unwrap();
        let mut progress = Vec::new();
        let run = source.run(&store, false, &mut progress);
        tokio::pin!(run);
        let observed = tokio::select! {
            result = &mut run => {
                gate.release();
                panic!("bootstrap finished before releasing {held}: {result:?}");
            }
            observed = tokio::time::timeout(Duration::from_secs(5), async {
                while server.hits(fifth).unwrap() == 0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }) => observed,
        };
        gate.release();
        let catalogue = run.await.unwrap();
        assert!(observed.is_ok(), "{fifth} waited for held task {held}");
        installed(&store, &catalogue).unwrap();
        assert_eq!(catalogue.species.len(), 6);
        for id in 6..=11 {
            assert_eq!(server.hits(&format!("/pokemon/pokemon{id}")).unwrap(), 1);
            assert_eq!(
                server.hits(&format!("/pokemon/art/{id:03}.png")).unwrap(),
                1
            );
        }
        let max = server.state.lock().unwrap().max_active;
        assert!((2..=4).contains(&max), "observed {max} concurrent requests");
    }
}

#[test]
fn concurrency_env_configures_workers_in_isolated_processes() {
    for (value, expected) in [
        (None, "4"),
        (Some("1"), "1"),
        (Some("2"), "2"),
        (Some("8"), "6"),
    ] {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args(["--exact", "configured_concurrency_child", "--nocapture"])
            .env("POKINGTON_CONCURRENCY_TEST_EXPECTED", expected)
            .env_remove("POKINGTON_CONCURRENCY");
        if let Some(value) = value {
            command.env("POKINGTON_CONCURRENCY", value);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "concurrency {value:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
async fn configured_concurrency_child() {
    let Ok(expected) = env::var("POKINGTON_CONCURRENCY_TEST_EXPECTED") else {
        return;
    };
    let expected = expected.parse::<usize>().unwrap();
    let temp = Temp::new().unwrap();
    let server = Server::many_species().unwrap();
    let gates: Vec<_> = (6..=11)
        .map(|id| server.hold(&format!("/pokemon/pokemon{id}")).unwrap())
        .collect();
    let source = server.source().unwrap();
    let store = temp.store().unwrap();
    let mut progress = Vec::new();
    let run = source.run(&store, false, &mut progress);
    tokio::pin!(run);
    let observed = tokio::select! {
        result = &mut run => panic!("bootstrap finished with held species pages: {result:?}"),
        observed = tokio::time::timeout(Duration::from_secs(5), async {
            while server.hits("/pokemon/pokemon6").unwrap() == 0
                || server.state.lock().unwrap().active != expected {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }) => observed,
    };
    for gate in gates {
        gate.release();
    }
    let catalogue = run.await.unwrap();
    assert!(
        observed.is_ok(),
        "did not reach {expected} concurrent requests"
    );
    installed(&store, &catalogue).unwrap();
    assert_eq!(server.state.lock().unwrap().max_active, expected);
}

#[test]
fn invalid_concurrency_env_fails_before_creating_store_files() {
    for value in [
        "0",
        "-1",
        "1.5",
        "",
        "no",
        "99999999999999999999999999999999999999",
    ]
    .map(OsString::from)
    .into_iter()
    .chain([OsString::from_vec(vec![0xff])])
    {
        let temp = Temp::new().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_pokington"))
            .arg("bootstrap")
            .env("HOME", &temp.0)
            .env("XDG_DATA_HOME", temp.0.join("data"))
            .env("POKINGTON_CONCURRENCY", &value)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{value:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("POKINGTON_CONCURRENCY must be a positive integer")
        );
        assert_eq!(output.stdout, b"");
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
}

struct FailArtworkLog(Arc<AtomicBool>);

impl Write for FailArtworkLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if String::from_utf8_lossy(bytes).contains("/pokemon/art/") {
            self.0.store(true, Ordering::Relaxed);
            return Err(std::io::Error::other("simulated log failure"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn coordinator_failure_stops_queued_work_and_drains_successful_transfers() {
    let temp = Temp::new().unwrap();
    let server = Server::many_species().unwrap();
    let gates: Vec<_> = (6..=9)
        .map(|id| server.hold(&format!("/pokemon/art/{id:03}.png")).unwrap())
        .collect();
    let failed = Arc::new(AtomicBool::new(false));
    let mut progress = FailArtworkLog(failed.clone());
    let source = server.source().unwrap();
    let store = temp.store().unwrap();
    let run = source.run(&store, false, &mut progress);
    tokio::pin!(run);
    let observed = tokio::select! {
        result = &mut run => panic!("bootstrap finished before releasing transfers: {result:?}"),
        observed = tokio::time::timeout(Duration::from_secs(5), async {
            while !failed.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }) => observed,
    };
    for gate in gates {
        gate.release();
    }
    assert!(observed.is_ok());
    let error = run.await.unwrap_err();
    assert!(error.to_string().contains("simulated log failure"));
    assert_eq!(server.hits("/pokemon/art/010.png").unwrap(), 0);
    assert_eq!(server.hits("/pokemon/art/011.png").unwrap(), 0);
    let saved = fs::read_dir(store.root().join("artwork")).unwrap().count();
    assert!((1..=4).contains(&saved));
    assert_eq!(server.image_hits().unwrap(), saved);
    assert!(store.load_active().unwrap().is_none());
    let catalogue = source.run(&store, false, &mut Vec::new()).await.unwrap();
    installed(&store, &catalogue).unwrap();
    assert_eq!(
        server.image_hits().unwrap(),
        6,
        "saved transfers must be reused"
    );
}

#[tokio::test]
async fn bad_status_mime_empty_truncated_and_invalid_png_never_publish() {
    let mut truncated = Response::png().unwrap();
    truncated.length += 10;
    let mut checksum = Response::png().unwrap();
    checksum.body[50] ^= 0xff;
    for response in [
        Response::new(404, "image/png", Response::png().unwrap().body),
        Response::new(200, "text/html", Response::png().unwrap().body),
        Response::new(200, "image/png", Vec::new()),
        Response::new(200, "image/png", b"not PNG".to_vec()),
        truncated,
        checksum,
    ] {
        let temp = Temp::new().unwrap();
        let server = Server::fixture().unwrap();
        server
            .route(
                "/pokemon/charizard",
                vec![Response::html(&original_hub().unwrap()).unwrap()],
            )
            .unwrap();
        server
            .route("/pokemon/art/006.png", vec![response])
            .unwrap();
        assert!(
            server
                .source()
                .unwrap()
                .run(&temp.store().unwrap(), false, &mut Vec::new())
                .await
                .is_err()
        );
        // A broken connection can fail an attempt before the server receives its request.
        assert!((1..=3).contains(&server.hits("/pokemon/art/006.png").unwrap()));
        assert!(temp.store().unwrap().load_active().unwrap().is_none());
        assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn discovery_drift_or_unresolved_forms_preserves_the_installed_dataset() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let source = server.source().unwrap();
    let old = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    server
        .route(
            "/pokedex-sv/charizard/",
            vec![Response::html(&original_hub().unwrap()).unwrap()],
        )
        .unwrap();
    for (index, hub) in [
        (INDEX.replace("006-006", "006-007"), HUB.to_owned()),
        (
            "<html><title>Challenge</title></html>".into(),
            HUB.to_owned(),
        ),
        (
            INDEX.into(),
            HUB.replace("<title>Charizard", "<title>Wrong"),
        ),
        (
            INDEX.into(),
            HUB.replace("</body>", "<table class=\"dextable\"><tr><td class=\"fooevo\">Alternate Forms</td></tr><tr><td><img src=\"/pokemon/art/006-new.png\" alt=\"New Charizard\"></td></tr></table></body>"),
        ),
    ] {
        server.route(INDEX_PATH, vec![Response::html(&index).unwrap()]).unwrap();
        server.route("/pokemon/charizard", vec![Response::html(&hub).unwrap()]).unwrap();
        assert!(
            source
                .run(&temp.store().unwrap(), true, &mut Vec::new())
                .await
                .is_err()
        );
        installed(&temp.store().unwrap(), &old).unwrap();
    }
    assert_eq!(server.image_hits().unwrap(), 4);
}

#[tokio::test]
async fn linked_type_evidence_is_discovered_without_guessing_artwork_urls() {
    let temp = Temp::new().unwrap();
    let server = Server::new().unwrap();
    let index = INDEX
        .replace("006-006", "128-128")
        .replace("#0006", "#0128")
        .replace("/pokemon/charizard", "/pokemon/tauros")
        .replace("Charizard", "Tauros");
    server
        .route(INDEX_PATH, vec![Response::html(&index).unwrap()])
        .unwrap();
    server
        .route(
            "/pokemon/tauros",
            vec![Response::html(include_str!("fixtures/serebii-tauros.html")).unwrap()],
        )
        .unwrap();
    server
        .route(
            "/pokedex-sv/tauros/",
            vec![Response::html(include_str!("fixtures/serebii-tauros-types.html")).unwrap()],
        )
        .unwrap();
    for image in ["128.png", "128-p.png", "128-b.png", "128-a.png"] {
        server
            .route(
                &format!("/pokemon/art/{image}"),
                vec![Response::png().unwrap()],
            )
            .unwrap();
    }
    let catalogue = server
        .source()
        .unwrap()
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(catalogue.forms().count(), 4);
    assert_eq!(server.hits("/pokedex-sv/tauros/").unwrap(), 1);
    installed(&temp.store().unwrap(), &catalogue).unwrap();
}

#[tokio::test]
async fn unresolved_forms_follow_an_older_actual_link_until_types_are_supported() {
    let temp = Temp::new().unwrap();
    let server = Server::new().unwrap();
    let index = INDEX
        .replace("006-006", "172-172")
        .replace("#0006", "#0172")
        .replace("/pokemon/charizard", "/pokemon/pichu")
        .replace("Charizard", "Pichu");
    let hub = include_str!("fixtures/serebii-pichu.html").replace(
        "<div class=\"hubheader\">",
        "<div class=\"hubheader\"><a href=\"/pokedex-sv/pichu/\">Current game</a>",
    );
    let older = include_str!("fixtures/serebii-pichu-types.html");
    let current = older.replace("<b>Spiky-eared</b>", "<b>Not illustrated</b>");
    server
        .route(INDEX_PATH, vec![Response::html(&index).unwrap()])
        .unwrap();
    server
        .route("/pokemon/pichu", vec![Response::html(&hub).unwrap()])
        .unwrap();
    server
        .route(
            "/pokedex-sv/pichu/",
            vec![Response::html(&current).unwrap()],
        )
        .unwrap();
    server
        .route(
            "/pokedex-dp/172.shtml",
            vec![Response::html(older).unwrap()],
        )
        .unwrap();
    for image in ["172.png", "172-s.png", "172-se.png"] {
        server
            .route(
                &format!("/pokemon/art/{image}"),
                vec![Response::png().unwrap()],
            )
            .unwrap();
    }
    let catalogue = server
        .source()
        .unwrap()
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(server.hits("/pokedex-sv/pichu/").unwrap(), 1);
    assert_eq!(server.hits("/pokedex-dp/172.shtml").unwrap(), 1);
    assert_eq!(catalogue.forms().count(), 2);
    assert!(
        catalogue
            .forms()
            .any(|form| form.type_source.ends_with("/pokedex-dp/172.shtml"))
    );
    installed(&temp.store().unwrap(), &catalogue).unwrap();
}

#[tokio::test]
async fn cancellation_during_transfers_keeps_only_complete_resumable_images() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    for image in IMAGES.into_iter().skip(1) {
        let mut slow = Response::png().unwrap();
        slow.delay = Duration::from_secs(2);
        server
            .route(&format!("/pokemon/art/{image}"), vec![slow])
            .unwrap();
    }
    let source = server.source().unwrap();
    let mut output = Vec::new();
    let store = temp.store().unwrap();
    {
        let run = source.run(&store, false, &mut output);
        tokio::pin!(run);
        let wait_for_image = async {
            loop {
                if fs::read_dir(temp.0.join("artwork")).is_ok_and(|entries| entries.count() == 1) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };
        tokio::select! {
            result = &mut run => panic!("bootstrap completed before cancellation: {result:?}"),
            result = tokio::time::timeout(Duration::from_secs(3), wait_for_image) => result.unwrap(),
        }
        // Dropping the pending future cancels remaining transfers and releases the writer.
    }
    assert!(temp.store().unwrap().load_active().unwrap().is_none());
    assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 1);
    for image in IMAGES.into_iter().skip(1) {
        server
            .route(
                &format!("/pokemon/art/{image}"),
                vec![Response::png().unwrap()],
            )
            .unwrap();
    }
    let catalogue = source
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(server.hits("/pokemon/art/006.png").unwrap(), 1);
    installed(&temp.store().unwrap(), &catalogue).unwrap();
}

#[tokio::test]
async fn request_timeout_is_bounded_and_leaves_the_active_catalogue_usable() {
    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    server
        .route(
            "/pokemon/charizard",
            vec![Response::html(&original_hub().unwrap()).unwrap()],
        )
        .unwrap();
    let old = server
        .source()
        .unwrap()
        .run(&temp.store().unwrap(), false, &mut Vec::new())
        .await
        .unwrap();
    let mut slow = Response::png().unwrap();
    slow.delay = Duration::from_millis(800);
    server.route("/pokemon/art/006.png", vec![slow]).unwrap();
    let client = http_client_builder()
        .https_only(false)
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();
    let source = Bootstrap::with_http(client, &server.origin);
    let started = Instant::now();
    assert!(
        source
            .run(&temp.store().unwrap(), true, &mut Vec::new())
            .await
            .is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(server.hits("/pokemon/art/006.png").unwrap(), 4);
    installed(&temp.store().unwrap(), &old).unwrap();
}

#[tokio::test]
async fn choose_first_run_completes_bootstrap_with_names_only_on_stdout() {
    use pokington::command::{self, Runtime};
    use rand::{SeedableRng, rngs::StdRng};

    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let store = temp.store().unwrap();
    let source = server.source().unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = command::run(
        ["pokington", "choose", "-c", "1"],
        || Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(3)).with_bootstrap(source.clone())),
        &mut stdout,
        &mut stderr,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(stdout, b"Charizard\n");
    let progress = String::from_utf8(stderr).unwrap();
    assert!(progress.contains("Downloading"));
    assert!(progress.contains("Complete: 1 species, 4 forms"));
    let catalogue = store
        .load_active()
        .unwrap()
        .ok_or("first run did not install data")
        .unwrap();
    installed(&store, &catalogue).unwrap();
    assert_eq!(server.image_hits().unwrap(), 4);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = command::run(
        ["pokington", "choose", "-c", "1"],
        || Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(3)).with_bootstrap(source)),
        &mut stdout,
        &mut stderr,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(stdout, b"Charizard\n");
    assert_eq!(stderr, b"");
    assert_eq!(server.hits(INDEX_PATH).unwrap(), 1);
    assert_eq!(server.image_hits().unwrap(), 4);
}

#[tokio::test]
async fn bootstrap_command_refreshes_skips_and_forces_through_the_existing_engine() {
    use pokington::command::{self, Runtime};
    use rand::{SeedableRng, rngs::StdRng};

    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    let store = temp.store().unwrap();
    let source = server.source().unwrap();
    for (args, expected_images) in [
        (vec!["pokington", "bootstrap"], 4),
        (vec!["pokington", "bootstrap"], 4),
        (vec!["pokington", "bootstrap", "--force"], 8),
    ] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = command::run(
            args,
            || {
                Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(3))
                    .with_bootstrap(source.clone()))
            },
            &mut stdout,
            &mut stderr,
        )
        .await;
        assert_eq!(code, 0);
        assert_eq!(stdout, b"");
        assert!(
            String::from_utf8(stderr)
                .unwrap()
                .contains("Complete: 1 species, 4 forms")
        );
        assert_eq!(server.image_hits().unwrap(), expected_images);
    }
    assert_eq!(server.hits(INDEX_PATH).unwrap(), 3);
}

#[tokio::test]
async fn failed_first_run_reports_bootstrap_error_without_printing_names() {
    use pokington::command::{self, Runtime};
    use rand::{SeedableRng, rngs::StdRng};

    let temp = Temp::new().unwrap();
    let server = Server::fixture().unwrap();
    server
        .route(
            "/pokemon/art/006.png",
            vec![Response::new(503, "text/html", b"unavailable".to_vec())],
        )
        .unwrap();
    let store = temp.store().unwrap();
    let source = server.source().unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = command::run(
        ["pokington", "choose", "-c", "1"],
        || Ok(Runtime::new(store.clone(), StdRng::seed_from_u64(3)).with_bootstrap(source)),
        &mut stdout,
        &mut stderr,
    )
    .await;
    assert_eq!(code, 1);
    assert_eq!(stdout, b"");
    let stderr = String::from_utf8(stderr).unwrap();
    assert!(stderr.contains("Downloading"));
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("503"));
    assert!(store.load_active().unwrap().is_none());
    assert_eq!(server.hits("/pokemon/art/006.png").unwrap(), 3);
}
