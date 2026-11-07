//! Offline replay of source-linked hub captures and their linked type evidence.
use pokington::serebii::{
    parse_enrichment, parse_hub, parse_index, reconcile, validate_artwork_references,
};
use serde::Deserialize;
use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};
#[derive(Deserialize)]
struct Request {
    id: u32,
    path: String,
    file: Option<String>,
}
fn decode(path: &std::path::Path) -> Result<String, std::io::Error> {
    Ok(fs::read(path)?.into_iter().map(char::from).collect())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: reconcile_serebii HUB_CACHE ENRICHMENT_CACHE [PNG_VALIDATION_JSON]".into(),
        );
    }
    let dir = PathBuf::from(&args[0]);
    let extra_dir = PathBuf::from(&args[1]);
    let index = parse_index(&decode(&dir.join("index.html"))?)?;
    let requests: Vec<Request> =
        serde_json::from_slice(&fs::read(extra_dir.join("requests.json"))?)?;
    let mut catalogue = vec![];
    let mut hubs = vec![];
    let mut errors = vec![];
    for species in &index.species {
        let result = (|| -> Result<_, Box<dyn Error>> {
            let hub = parse_hub(&decode(&dir.join(format!("{}.html", species.id)))?, species)?;
            let mut extra = vec![];
            for request in requests.iter().filter(|r| r.id == species.id) {
                extra.push(parse_enrichment(
                    &hub,
                    &request.path,
                    &decode(
                        &extra_dir.join(
                            request
                                .file
                                .clone()
                                .unwrap_or_else(|| format!("{}.html", species.id)),
                        ),
                    )?,
                )?);
            }
            let result = reconcile(&hub, &extra);
            hubs.push(hub);
            Ok(result?)
        })();
        match result {
            Ok(species) => catalogue.push(species),
            Err(e) => errors.push(format!("{}: {e}", species.id)),
        }
        if (catalogue.len() + errors.len()).is_multiple_of(100) {
            eprintln!(
                "replayed {} species; {} unresolved",
                catalogue.len() + errors.len(),
                errors.len()
            );
        }
    }
    fs::write(
        dir.join("hub-inventory.json"),
        serde_json::to_vec_pretty(&hubs)?,
    )?;
    fs::write(
        dir.join("catalogue.json"),
        serde_json::to_vec_pretty(&catalogue)?,
    )?;
    fs::write(
        dir.join("reconciliation-errors.json"),
        serde_json::to_vec_pretty(&errors)?,
    )?;
    println!(
        "{} species; {} raw illustration refs; {} reconciled species; {} forms; {} unresolved species",
        index.species.len(),
        hubs.iter().map(|h| h.artwork.len()).sum::<usize>(),
        catalogue.len(),
        catalogue.iter().map(|s| s.forms.len()).sum::<usize>(),
        errors.len()
    );
    for e in &errors {
        println!("{e}");
    }
    if !errors.is_empty() {
        return Err("catalogue reconciliation incomplete".into());
    }
    if let Some(path) = args.get(2) {
        let records: Vec<serde_json::Value> = serde_json::from_slice(&fs::read(path)?)?;
        let verified: BTreeSet<String> = records
            .iter()
            .filter(|record| {
                record.get("error").is_none()
                    && record["complete_decode"] == true
                    && record["uniform_blank"] == false
                    && record["visible_pixels"].as_u64().is_some_and(|n| n > 0)
                    && record["http"]["status"] == 200
                    && record["http"]["content_type"] == "image/png"
            })
            .filter_map(|record| record["http"]["url"].as_str().map(str::to_owned))
            .collect();
        validate_artwork_references(&catalogue, &verified)?;
        println!("All selected illustration references have successful PNG byte validation");
    }
    Ok(())
}
