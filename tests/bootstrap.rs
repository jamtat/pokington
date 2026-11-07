#![cfg(unix)]

use std::{
    collections::{BTreeMap, VecDeque},
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
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

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = env::temp_dir().join(format!(
            "pokington-bootstrap-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn store(&self) -> Store {
        Store::at(self.0.clone()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Clone)]
struct Response {
    status: u16,
    mime: String,
    body: Vec<u8>,
    length: usize,
    delay: Duration,
}
impl Response {
    fn new(status: u16, mime: &str, body: Vec<u8>) -> Self {
        Self {
            status,
            mime: mime.into(),
            length: body.len(),
            body,
            delay: Duration::ZERO,
        }
    }
    fn html(value: &str) -> Self {
        Self::new(
            200,
            "text/html; charset=iso-8859-1",
            value
                .chars()
                .map(|ch| u8::try_from(ch).expect("ISO-8859-1 fixture"))
                .collect(),
        )
    }
    fn png() -> Self {
        let mut body = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut body, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[255, 0, 0, 255, 0, 0, 0, 0])
                .unwrap();
        }
        Self::new(200, "image/png", body)
    }
}

#[derive(Default)]
struct State {
    routes: BTreeMap<String, VecDeque<Response>>,
    hits: Vec<String>,
    active_images: usize,
    max_images: usize,
}
struct Server {
    origin: String,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = state.clone();
        let worker_stop = stop.clone();
        let thread = thread::spawn(move || {
            let mut workers = Vec::new();
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let state = worker_state.clone();
                        workers.push(thread::spawn(move || serve(stream, &state)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(error) => panic!("fixture listener: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            origin,
            state,
            stop,
            thread: Some(thread),
        }
    }
    fn route(&self, path: &str, responses: Vec<Response>) {
        assert!(!responses.is_empty());
        self.state
            .lock()
            .unwrap()
            .routes
            .insert(path.into(), responses.into());
    }
    fn fixture() -> Self {
        let server = Self::new();
        server.route(INDEX_PATH, vec![Response::html(INDEX)]);
        server.route("/pokemon/charizard", vec![Response::html(HUB)]);
        for image in IMAGES {
            server.route(&format!("/pokemon/art/{image}"), vec![Response::png()]);
        }
        server
    }
    fn source(&self) -> Bootstrap {
        Bootstrap::with_http(
            http_client_builder().https_only(false).build().unwrap(),
            self.origin.clone(),
        )
    }
    fn hits(&self, path: &str) -> usize {
        self.state
            .lock()
            .unwrap()
            .hits
            .iter()
            .filter(|hit| hit.as_str() == path)
            .count()
    }
    fn image_hits(&self) -> usize {
        self.state
            .lock()
            .unwrap()
            .hits
            .iter()
            .filter(|hit| hit.starts_with("/pokemon/art/"))
            .count()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn serve(mut stream: TcpStream, state: &Mutex<State>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request = Vec::new();
    let mut bytes = [0; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut bytes) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&bytes[..n]),
        }
    }
    let request = String::from_utf8(request).unwrap();
    let path = request.split_whitespace().nth(1).unwrap().to_owned();
    let is_image = path.starts_with("/pokemon/art/");
    let response = {
        let mut state = state.lock().unwrap();
        state.hits.push(path.clone());
        if is_image {
            state.active_images += 1;
            state.max_images = state.max_images.max(state.active_images);
        }
        match state.routes.get_mut(&path) {
            Some(responses) if responses.len() > 1 => responses.pop_front().unwrap(),
            Some(responses) => responses[0].clone(),
            None => Response::new(404, "text/html", b"missing fixture route".to_vec()),
        }
    };
    thread::sleep(response.delay);
    let header = format!(
        "HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status, response.mime, response.length
    );
    let _ = stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(&response.body));
    if is_image {
        state.lock().unwrap().active_images -= 1;
    }
}

fn installed(store: &Store, expected: &Catalogue) {
    assert_eq!(store.load_active().unwrap().as_ref(), Some(expected));
    for form in expected.forms() {
        validate_png(&store.read_artwork(&form.local_artwork).unwrap()).unwrap();
    }
}

fn original_hub() -> String {
    let start = HUB.find("</table></div><table").unwrap();
    format!(
        "{}<div class=\"hubheader\"><a href=\"/pokedex-sv/charizard/\">Game database</a></div></body></html>",
        &HUB[..start + "</table>".len()]
    )
}

#[tokio::test]
async fn complete_initial_and_refresh_skip_validate_artwork_and_show_plain_progress() {
    let temp = Temp::new();
    let server = Server::fixture();
    let source = server.source();
    let mut output = Vec::new();
    let catalogue = source.run(&temp.store(), false, &mut output).await.unwrap();
    assert_eq!(catalogue.species.len(), 1);
    assert_eq!(catalogue.forms().count(), 4);
    installed(&temp.store(), &catalogue);
    assert!(!temp.0.join("pending.json").exists());
    assert_eq!(server.image_hits(), 4);
    assert_eq!(
        server.hits("/pokemon/art/006-g.png"),
        0,
        "unselected alias must not be guessed or fetched"
    );
    let refreshed = source.run(&temp.store(), false, &mut output).await.unwrap();
    assert_eq!(refreshed, catalogue);
    assert_eq!(server.image_hits(), 4);
    assert_eq!(server.hits(INDEX_PATH), 2);
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Downloading https://www.serebii.net/pokemon/nationalpokedex.shtml"));
    assert!(output.contains("Artwork [########################] 4/4"));
    assert!(output.contains("Complete: 1 species, 4 forms, 0 excluded illustrations"));
    assert!(!output.contains(['\x1b', '\r']));
}

#[tokio::test]
async fn failed_initial_reuses_completed_files_on_retry() {
    let temp = Temp::new();
    let server = Server::fixture();
    let missing = "/pokemon/art/006-my.png";
    server.route(
        missing,
        vec![Response::new(503, "text/html", b"unavailable".to_vec())],
    );
    let source = server.source();
    let error = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("503"));
    assert!(temp.store().load_active().unwrap().is_none());
    assert!(temp.0.join("pending.json").exists());
    assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 3);
    assert_eq!(server.hits(missing), 3);
    server.route(missing, vec![Response::png()]);
    let catalogue = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    installed(&temp.store(), &catalogue);
    assert_eq!(
        server.image_hits(),
        7,
        "three complete images are not downloaded again"
    );
}

#[tokio::test]
async fn force_failure_preserves_active_retry_resumes_and_next_force_is_fresh() {
    let temp = Temp::new();
    let server = Server::fixture();
    let source = server.source();
    let store = temp.store();
    let original = source.run(&store, false, &mut Vec::new()).await.unwrap();
    let broken = "/pokemon/art/006.png";
    server.route(
        broken,
        vec![Response::new(200, "image/png", b"broken PNG".to_vec())],
    );
    assert!(source.run(&store, true, &mut Vec::new()).await.is_err());
    installed(&store, &original);
    server.route(broken, vec![Response::png()]);
    let replacement = source.run(&store, true, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits(), 11);
    for (old, new) in original.forms().zip(replacement.forms()) {
        assert_ne!(old.local_artwork, new.local_artwork);
    }
    for form in original.forms() {
        validate_png(&store.read_artwork(&form.local_artwork).unwrap()).unwrap();
    }
    assert_eq!(store.load_active().unwrap(), Some(replacement.clone()));
    let fresh = source.run(&store, true, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits(), 15);
    for (old, new) in replacement.forms().zip(fresh.forms()) {
        assert_ne!(old.local_artwork, new.local_artwork);
    }
    installed(&store, &fresh);
}

#[tokio::test]
async fn refresh_discovers_new_forms_and_repairs_only_missing_or_corrupt_images() {
    let temp = Temp::new();
    let server = Server::fixture();
    server.route("/pokemon/charizard", vec![Response::html(&original_hub())]);
    let source = server.source();
    let store = temp.store();
    let first = source.run(&store, false, &mut Vec::new()).await.unwrap();
    assert_eq!(first.forms().count(), 1);
    server.route("/pokemon/charizard", vec![Response::html(HUB)]);
    let second = source.run(&store, false, &mut Vec::new()).await.unwrap();
    assert_eq!(server.image_hits(), 4);
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
    assert_eq!(server.image_hits(), 6);
    installed(&store, &third);
}

#[tokio::test]
async fn refresh_discovers_new_species_from_the_extended_source_index() {
    let temp = Temp::new();
    let server = Server::fixture();
    server.route("/pokemon/charizard", vec![Response::html(&original_hub())]);
    let source = server.source();
    let first = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    let row = &INDEX[INDEX.find("<tr>\n").unwrap()..INDEX.rfind("</table></body>").unwrap()];
    let new_row = row
        .replace("#0006", "#0007")
        .replace("/pokemon/charizard", "/pokemon/squirtle")
        .replace("Charizard", "Squirtle");
    let extended = INDEX
        .replace("006-006", "006-007")
        .replace("</table></body>", &format!("{new_row}</table></body>"));
    let new_hub = original_hub()
        .replace("#0006", "#0007")
        .replace("Charizard", "Squirtle")
        .replace("charizard", "squirtle")
        .replace("006.png", "007.png");
    server.route(INDEX_PATH, vec![Response::html(&extended)]);
    server.route("/pokemon/squirtle", vec![Response::html(&new_hub)]);
    server.route("/pokemon/art/007.png", vec![Response::png()]);
    let second = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(second.source.max_id, 7);
    assert_eq!(second.species.len(), 2);
    assert_eq!(second.species[1].name, "Squirtle");
    assert_eq!(server.image_hits(), 2);
    assert_eq!(first.species[0].forms, second.species[0].forms);
    installed(&temp.store(), &second);
}

#[tokio::test]
async fn transient_http_errors_are_retried_with_bounded_concurrency() {
    let temp = Temp::new();
    let server = Server::fixture();
    let mut slow = Response::png();
    slow.delay = Duration::from_millis(40);
    for image in IMAGES {
        server.route(&format!("/pokemon/art/{image}"), vec![slow.clone()]);
    }
    server.route(
        "/pokemon/art/006.png",
        vec![
            Response::new(429, "text/html", b"busy".to_vec()),
            Response::new(500, "text/html", b"busy".to_vec()),
            slow,
        ],
    );
    let catalogue = server
        .source()
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    installed(&temp.store(), &catalogue);
    assert_eq!(server.hits("/pokemon/art/006.png"), 3);
    let max = server.state.lock().unwrap().max_images;
    assert!(
        (2..=4).contains(&max),
        "observed {max} concurrent image requests"
    );
}

#[tokio::test]
async fn bad_status_mime_empty_truncated_and_invalid_png_never_publish() {
    let mut truncated = Response::png();
    truncated.length += 10;
    let mut checksum = Response::png();
    checksum.body[50] ^= 0xff;
    for response in [
        Response::new(404, "image/png", Response::png().body),
        Response::new(200, "text/html", Response::png().body),
        Response::new(200, "image/png", Vec::new()),
        Response::new(200, "image/png", b"not PNG".to_vec()),
        truncated,
        checksum,
    ] {
        let temp = Temp::new();
        let server = Server::fixture();
        server.route("/pokemon/charizard", vec![Response::html(&original_hub())]);
        server.route("/pokemon/art/006.png", vec![response]);
        assert!(
            server
                .source()
                .run(&temp.store(), false, &mut Vec::new())
                .await
                .is_err()
        );
        // A broken connection can fail an attempt before the server receives its request.
        assert!((1..=3).contains(&server.hits("/pokemon/art/006.png")));
        assert!(temp.store().load_active().unwrap().is_none());
        assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn discovery_drift_or_unresolved_forms_preserves_the_installed_dataset() {
    let temp = Temp::new();
    let server = Server::fixture();
    let source = server.source();
    let old = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    server.route(
        "/pokedex-sv/charizard/",
        vec![Response::html(&original_hub())],
    );
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
        server.route(INDEX_PATH, vec![Response::html(&index)]);
        server.route("/pokemon/charizard", vec![Response::html(&hub)]);
        assert!(
            source
                .run(&temp.store(), true, &mut Vec::new())
                .await
                .is_err()
        );
        installed(&temp.store(), &old);
    }
    assert_eq!(server.image_hits(), 4);
}

#[tokio::test]
async fn linked_type_evidence_is_discovered_without_guessing_artwork_urls() {
    let temp = Temp::new();
    let server = Server::new();
    let index = INDEX
        .replace("006-006", "128-128")
        .replace("#0006", "#0128")
        .replace("/pokemon/charizard", "/pokemon/tauros")
        .replace("Charizard", "Tauros");
    server.route(INDEX_PATH, vec![Response::html(&index)]);
    server.route(
        "/pokemon/tauros",
        vec![Response::html(include_str!("fixtures/serebii-tauros.html"))],
    );
    server.route(
        "/pokedex-sv/tauros/",
        vec![Response::html(include_str!(
            "fixtures/serebii-tauros-types.html"
        ))],
    );
    for image in ["128.png", "128-p.png", "128-b.png", "128-a.png"] {
        server.route(&format!("/pokemon/art/{image}"), vec![Response::png()]);
    }
    let catalogue = server
        .source()
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(catalogue.forms().count(), 4);
    assert_eq!(server.hits("/pokedex-sv/tauros/"), 1);
    installed(&temp.store(), &catalogue);
}

#[tokio::test]
async fn unresolved_forms_follow_an_older_actual_link_until_types_are_supported() {
    let temp = Temp::new();
    let server = Server::new();
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
    server.route(INDEX_PATH, vec![Response::html(&index)]);
    server.route("/pokemon/pichu", vec![Response::html(&hub)]);
    server.route("/pokedex-sv/pichu/", vec![Response::html(&current)]);
    server.route("/pokedex-dp/172.shtml", vec![Response::html(older)]);
    for image in ["172.png", "172-s.png", "172-se.png"] {
        server.route(&format!("/pokemon/art/{image}"), vec![Response::png()]);
    }
    let catalogue = server
        .source()
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(server.hits("/pokedex-sv/pichu/"), 1);
    assert_eq!(server.hits("/pokedex-dp/172.shtml"), 1);
    assert_eq!(catalogue.forms().count(), 2);
    assert!(
        catalogue
            .forms()
            .any(|form| form.type_source.ends_with("/pokedex-dp/172.shtml"))
    );
    installed(&temp.store(), &catalogue);
}

#[tokio::test]
async fn cancellation_during_transfers_keeps_only_complete_resumable_images() {
    let temp = Temp::new();
    let server = Server::fixture();
    for image in IMAGES.into_iter().skip(1) {
        let mut slow = Response::png();
        slow.delay = Duration::from_secs(2);
        server.route(&format!("/pokemon/art/{image}"), vec![slow]);
    }
    let source = server.source();
    let mut output = Vec::new();
    let store = temp.store();
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
    assert!(temp.store().load_active().unwrap().is_none());
    assert_eq!(fs::read_dir(temp.0.join("artwork")).unwrap().count(), 1);
    for image in IMAGES.into_iter().skip(1) {
        server.route(&format!("/pokemon/art/{image}"), vec![Response::png()]);
    }
    let catalogue = source
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(server.hits("/pokemon/art/006.png"), 1);
    installed(&temp.store(), &catalogue);
}

#[tokio::test]
async fn request_timeout_is_bounded_and_leaves_the_active_catalogue_usable() {
    let temp = Temp::new();
    let server = Server::fixture();
    server.route("/pokemon/charizard", vec![Response::html(&original_hub())]);
    let old = server
        .source()
        .run(&temp.store(), false, &mut Vec::new())
        .await
        .unwrap();
    let mut slow = Response::png();
    slow.delay = Duration::from_millis(800);
    server.route("/pokemon/art/006.png", vec![slow]);
    let client = http_client_builder()
        .https_only(false)
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();
    let source = Bootstrap::with_http(client, server.origin.clone());
    let started = Instant::now();
    assert!(
        source
            .run(&temp.store(), true, &mut Vec::new())
            .await
            .is_err()
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(server.hits("/pokemon/art/006.png"), 4);
    installed(&temp.store(), &old);
}
