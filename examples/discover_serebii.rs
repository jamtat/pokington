//! Bounded metadata investigation, not production bootstrap or accepted catalogue output.
use http_body_util::{BodyExt, Limited};
use pokington::{
    serebii::{INDEX_URL, ORIGIN, parse_hub, parse_index},
    source::http_client,
};
use std::{error::Error, fs, path::Path};

type Failure = Box<dyn Error + Send + Sync>;

fn decode(bytes: &[u8]) -> String {
    // The captured Serebii pages declare ISO-8859-1, not UTF-8.
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

async fn read(
    client: &wreq::Client,
    url: &str,
    path: &Path,
    offline: bool,
) -> Result<String, Failure> {
    if offline {
        return Ok(decode(&fs::read(path)?));
    }
    let response = client.get(url).send().await?.error_for_status()?;
    if !response.uri().to_string().starts_with(ORIGIN) {
        return Err("unexpected redirect origin".into());
    }
    let bytes = Limited::new(response, 4 * 1024 * 1024)
        .collect()
        .await?
        .to_bytes();
    fs::write(path, &bytes)?;
    Ok(decode(&bytes))
}

#[tokio::main]
async fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty()
        || args.len() > 2
        || (args.len() == 2 && args.get(1).map(String::as_str) != Some("--offline"))
    {
        return Err("usage: discover_serebii DIRECTORY [--offline]".into());
    }
    let dir = Path::new(args.first().ok_or("missing output directory")?);
    fs::create_dir_all(dir)?;
    let offline = args.len() == 2;
    let client = http_client()?;
    let html = read(&client, INDEX_URL, &dir.join("index.html"), offline).await?;
    let index = parse_index(&html)?;
    fs::write(dir.join("index.json"), serde_json::to_vec_pretty(&index)?)?;
    println!(
        "index: {} species; advertised bounds {}–{}",
        index.species.len(),
        index.min,
        index.max
    );
    let mut hubs = Vec::new();
    let mut errors = Vec::new();
    for batch in index.species.chunks(4) {
        let mut jobs = tokio::task::JoinSet::new();
        for species in batch {
            let species = species.clone();
            let client = client.clone();
            let path = dir.join(format!("{}.html", species.id));
            jobs.spawn(async move {
                let html = read(
                    &client,
                    &format!("{ORIGIN}{}", species.detail_path),
                    &path,
                    offline,
                )
                .await?;
                Ok::<_, Failure>((species.id, parse_hub(&html, &species)))
            });
        }
        while let Some(result) = jobs.join_next().await {
            let (id, result) = result??;
            match result {
                Ok(hub) => hubs.push(hub),
                Err(error) => errors.push(format!("{id}: {error}")),
            }
        }
        let done = hubs
            .len()
            .checked_add(errors.len())
            .ok_or("hub count overflow")?;
        if done % 100 == 0 || done == index.species.len() {
            println!(
                "inspected {done}/{} hubs; {} parser failures",
                index.species.len(),
                errors.len()
            );
        }
    }
    hubs.sort_by_key(|hub| hub.id);
    fs::write(
        dir.join("hub-inventory.json"),
        serde_json::to_vec_pretty(&hubs)?,
    )?;
    fs::write(
        dir.join("parser-errors.json"),
        serde_json::to_vec_pretty(&errors)?,
    )?;
    println!(
        "{} parsed hubs; {} source artwork references; {} errors. Types and full form coverage not yet established.",
        hubs.len(),
        hubs.iter().map(|hub| hub.artwork.len()).sum::<usize>(),
        errors.len()
    );
    if !errors.is_empty() {
        return Err("source parsing incomplete; see parser-errors.json".into());
    }
    Ok(())
}
