//! Full metadata feasibility evidence. This is not the production bootstrap command.
use std::{error::Error, fs, path::Path};

use http_body_util::{BodyExt, Limited};
#[allow(dead_code)]
#[path = "support/official_source.rs"]
mod official_source;
use official_source::{
    INDEX_URL, ORIGIN, catalogue_url, http_client, parse_index_page, parse_species_detail,
    parse_species_links,
};
use pokington::source;

type Failure = Box<dyn Error + Send + Sync>;

async fn read(
    client: &wreq::Client,
    url: &str,
    path: &Path,
    offline: bool,
) -> Result<String, Failure> {
    if offline {
        return Ok(fs::read_to_string(path)?);
    }
    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = Limited::new(response, 4 * 1024 * 1024)
        .collect()
        .await?
        .to_bytes();
    let text = String::from_utf8(bytes.to_vec())?;
    fs::write(path, &text)?;
    Ok(text)
}

#[tokio::main]
async fn main() -> Result<(), Failure> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || args.len() > 2 || (args.len() == 2 && args[1] != "--offline") {
        return Err("usage: discover_official OUTPUT_DIRECTORY [--offline]".into());
    }
    let dir = Path::new(&args[0]);
    let offline = args.len() == 2;
    fs::create_dir_all(dir)?;
    let client = http_client()?;
    let index_html = read(&client, INDEX_URL, &dir.join("index.html"), offline).await?;
    let page = parse_index_page(&index_html)?;
    let script = read(&client, &page.script_url, &dir.join("pokedex.js"), offline).await?;
    let url = catalogue_url(&page, &script)?;
    let json = read(&client, &url, &dir.join("species.json"), offline).await?;
    let links = parse_species_links(&json, &page)?;
    let mut species = Vec::new();
    for batch in links.chunks(4) {
        let mut jobs = tokio::task::JoinSet::new();
        for link in batch {
            let link = link.clone();
            let client = client.clone();
            let path = dir.join(format!("{}.html", link.id));
            jobs.spawn(async move {
                let html = read(
                    &client,
                    &format!("{ORIGIN}{}", link.detail_path),
                    &path,
                    offline,
                )
                .await?;
                Ok::<_, Failure>(
                    parse_species_detail(&html, &link)
                        .map_err(|error| format!("{}: {error}", link.detail_path))?,
                )
            });
        }
        while let Some(result) = jobs.join_next().await {
            species.push(result??);
        }
        if species.len() % 100 == 0 || species.len() == links.len() {
            eprintln!(
                "parsed {}/{} species detail pages",
                species.len(),
                links.len()
            );
        }
    }
    species.sort_by_key(|entry| entry.id);
    fs::write(
        dir.join("metadata.json"),
        serde_json::to_vec_pretty(&species)?,
    )?;
    let form_count: usize = species.iter().map(|entry| entry.forms.len()).sum();
    println!(
        "{} species, {} forms; source bounds {}–{}; metadata saved to {}",
        species.len(),
        form_count,
        page.min,
        page.max,
        dir.join("metadata.json").display()
    );
    Ok(())
}
