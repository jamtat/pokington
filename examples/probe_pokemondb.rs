//! Cached, source-linked artwork investigation; not a bootstrap implementation.
use pokington::source::http_client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, fs, path::PathBuf, time::Duration};

type ProbeResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

fn select(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}

async fn fetch(client: &wreq::Client, url: &str, path: &PathBuf) -> ProbeResult<String> {
    if path.exists() {
        return Ok(fs::read_to_string(path)?);
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    let response = client.get(url).send().await?.error_for_status()?;
    let content_type = response.headers().get("content-type").unwrap().to_str()?;
    if !content_type.starts_with("text/html") {
        return Err(format!("unexpected content type: {url}: {content_type}").into());
    }
    let body = response.text().await?;
    if body.len() > 4_000_000 || !body.trim_end().ends_with("</html>") {
        return Err(format!("invalid HTML: {url}").into());
    }
    fs::write(path, &body)?;
    Ok(body)
}

async fn species(
    client: wreq::Client,
    cache: PathBuf,
    item: Value,
    home: bool,
) -> ProbeResult<Value> {
    let path = item["path"].as_str().unwrap();
    let slug = path.rsplit('/').next().unwrap();
    let detail = fetch(
        &client,
        &format!("https://pokemondb.net{path}"),
        &cache.join(format!("{slug}-species.html")),
    )
    .await?;
    let gallery_path = {
        let doc = Html::parse_document(&detail);
        doc.select(&select(if home {
            "a[href^='/sprites/']"
        } else {
            "a[href^='/artwork/']"
        }))
        .next()
        .and_then(|a| a.value().attr("href"))
        .map(str::to_owned)
    };
    let Some(gallery_path) = gallery_path else {
        return Ok(json!({"species":item,"error":"no source-linked artwork gallery"}));
    };
    let gallery_url = format!("https://pokemondb.net{gallery_path}");
    let gallery = fetch(
        &client,
        &gallery_url,
        &cache.join(format!(
            "{slug}-{}.html",
            if home { "sprites" } else { "artwork" }
        )),
    )
    .await?;
    let doc = Html::parse_document(&gallery);
    let mut artwork = vec![];
    if home {
        for cell in doc.select(&select(".sprites-table-card")) {
            let Some(link) = cell
                .select(&select("a[href*='/sprites/home/normal/']"))
                .next()
            else {
                continue;
            };
            let labels: Vec<_> = cell
                .select(&select("small"))
                .map(|s| s.text().collect::<String>())
                .collect();
            let form = if labels.is_empty() {
                None
            } else {
                Some(labels.join("; "))
            };
            artwork.push(json!({"kind":"HOME normal render", "form":form,"labels":labels, "url":link.value().attr("href")}));
        }
        return Ok(json!({"species":item,"gallery":gallery_url,"artwork":artwork}));
    }
    for cell in doc.select(&select("main .grid-col")) {
        let Some(kind) = cell.select(&select("strong")).next() else {
            continue;
        };
        let Some(img) = cell.select(&select("img[src]")).next() else {
            continue;
        };
        let url = cell
            .select(&select("a[rel=lightbox]"))
            .next()
            .and_then(|a| a.value().attr("href"))
            .unwrap_or_else(|| img.value().attr("src").unwrap());
        artwork.push(json!({
            "kind":kind.text().collect::<String>().trim(),
            "form":cell.select(&select("span.text-muted")).next().map(|s|s.text().collect::<String>()),
            "alt":img.value().attr("alt"),
            "url":url,
        }));
    }
    Ok(json!({"species":item,"gallery":gallery_url,"artwork":artwork}))
}

async fn png(client: wreq::Client, cache: PathBuf, url: String) -> ProbeResult<Value> {
    let relative = url
        .strip_prefix("https://img.pokemondb.net/")
        .or_else(|| url.strip_prefix("https://www.serebii.net/"))
        .ok_or("unexpected image host")?;
    if relative.contains("..") {
        return Err("invalid image path".into());
    }
    let path = cache.join("images").join(relative);
    let metadata_path = path.with_extension("metadata.json");
    let metadata: Value;
    let bytes = if path.exists() && metadata_path.exists() {
        metadata = serde_json::from_slice(&fs::read(&metadata_path)?)?;
        fs::read(&path)?
    } else {
        tokio::time::sleep(Duration::from_millis(150)).await;
        let response = client.get(&url).send().await?.error_for_status()?;
        let content_type = response
            .headers()
            .get("content-type")
            .ok_or("no content type")?
            .to_str()?
            .to_owned();
        if content_type != "image/png" {
            return Err(format!("not PNG: {url}: {content_type}").into());
        }
        metadata = json!({"url":url,"final_url":response.uri().to_string(),"status":response.status().as_u16(),"content_type":content_type});
        let body = response.bytes().await?;
        if body.len() > 20_000_000 {
            return Err("image exceeds bound".into());
        }
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, &body)?;
        fs::write(&metadata_path, serde_json::to_vec(&metadata)?)?;
        body.to_vec()
    };
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("invalid PNG signature".into());
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    decoder.ignore_checksums(false);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or("PNG exceeds address space")?;
    if size > 64 * 1024 * 1024 {
        return Err("PNG exceeds decode bound".into());
    }
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    reader.finish()?;
    let stride = match info.color_type {
        png::ColorType::Rgba => Some(4),
        png::ColorType::GrayscaleAlpha => Some(2),
        _ => None,
    };
    let transparent = stride
        .map(|s| {
            pixels[..info.buffer_size()]
                .chunks_exact(s)
                .filter(|p| p[s - 1] == 0)
                .count()
        })
        .unwrap_or(0);
    let total_pixels = info.width as usize * info.height as usize;
    if transparent == total_pixels {
        return Err(format!("fully transparent image: {url}").into());
    }
    let mut pixel_values = pixels[..info.buffer_size()].chunks_exact(info.color_type.samples());
    let first = pixel_values.next().ok_or("empty decoded image")?;
    if pixel_values.all(|pixel| pixel == first) {
        return Err(format!("uniform blank image: {url}").into());
    }
    Ok(
        json!({"http":metadata,"path":path,"bytes":bytes.len(),"width":info.width,"height":info.height,"color":format!("{:?}",info.color_type),"transparent_pixels":transparent,"visible_pixels":total_pixels-transparent,"uniform_blank":false,"complete_decode":true}),
    )
}

#[tokio::main]
async fn main() -> ProbeResult<()> {
    let cache = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: probe_pokemondb CACHE_DIRECTORY")?,
    );
    fs::create_dir_all(&cache)?;
    let client = http_client()?;
    let mode = std::env::args().nth(2).unwrap_or_default();
    if mode == "png" || mode == "png-list" {
        let mut urls = std::collections::BTreeSet::new();
        if mode == "png-list" {
            let list = std::env::args()
                .nth(3)
                .ok_or("png-list requires a source-linked candidate JSON path")?;
            let records: Vec<Value> = serde_json::from_slice(&fs::read(list)?)?;
            for record in records {
                urls.insert(
                    record["url"]
                        .as_str()
                        .ok_or("candidate missing URL")?
                        .to_owned(),
                );
            }
        } else {
            for name in ["coverage.json", "home-coverage.json"] {
                let records: Vec<Value> = serde_json::from_slice(&fs::read(cache.join(name))?)?;
                for row in records {
                    for art in row["artwork"].as_array().into_iter().flatten() {
                        let url = art["url"].as_str().unwrap();
                        if url.ends_with(".png") {
                            urls.insert(url.to_owned());
                        }
                    }
                }
            }
        }
        eprintln!("source-linked PNGs: {}", urls.len());
        let mut tasks = tokio::task::JoinSet::new();
        let mut results = vec![];
        for url in urls {
            let c = client.clone();
            let dir = cache.clone();
            tasks.spawn(async move {
                match png(c, dir, url.clone()).await {
                    Ok(v) => v,
                    Err(e) => json!({"url":url,"error":e.to_string()}),
                }
            });
            if tasks.len() >= 3 {
                results.push(tasks.join_next().await.unwrap()?);
                if results.len() % 100 == 0 {
                    eprintln!("PNGs checked: {}", results.len());
                }
            }
        }
        while let Some(r) = tasks.join_next().await {
            results.push(r?);
        }
        fs::write(
            cache.join("png-validation.json"),
            serde_json::to_vec_pretty(&results)?,
        )?;
        if results.iter().any(|result| result.get("error").is_some()) {
            return Err("source image validation incomplete; see png-validation.json".into());
        }
        return Ok(());
    }
    let index = fetch(
        &client,
        "https://pokemondb.net/pokedex/all",
        &cache.join("all.html"),
    )
    .await?;
    let entries = {
        let doc = Html::parse_document(&index);
        let mut entries = BTreeMap::new();
        for row in doc.select(&select("#pokedex tbody tr")) {
            let name = row
                .select(&select("a.ent-name"))
                .next()
                .ok_or("missing species link")?;
            let path = name.value().attr("href").ok_or("missing detail path")?;
            let number = row
                .select(&select(".infocard-cell-data"))
                .next()
                .ok_or("missing number")?
                .text()
                .collect::<String>();
            let id: u32 = number.trim().parse()?;
            entries.entry(id).or_insert_with(
                || json!({"id":id,"name":name.text().collect::<String>(),"path":path}),
            );
        }
        entries
    };
    if entries.is_empty() {
        return Err("empty source catalogue".into());
    }
    eprintln!("source species: {}", entries.len());
    let mut tasks = tokio::task::JoinSet::new();
    let mut results = vec![];
    for item in entries.into_values() {
        tasks.spawn(species(client.clone(), cache.clone(), item, mode == "home"));
        if tasks.len() >= 3 {
            results.push(tasks.join_next().await.unwrap()??);
            if results.len() % 50 == 0 {
                eprintln!("galleries checked: {}", results.len());
            }
        }
    }
    while let Some(result) = tasks.join_next().await {
        results.push(result??);
    }
    results.sort_by_key(|r| r["species"]["id"].as_u64());
    fs::write(
        cache.join(if mode == "home" {
            "home-coverage.json"
        } else {
            "coverage.json"
        }),
        serde_json::to_vec_pretty(&results)?,
    )?;
    eprintln!("completed galleries: {}", results.len());
    Ok(())
}
