//! Main-site discovery and form parsing, independent of HTTP and local storage.
use std::collections::BTreeSet;
use std::time::Duration;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::source::SourceError;

pub const ORIGIN: &str = "https://www.pokemon.com";
pub const INDEX_URL: &str = "https://www.pokemon.com/us/pokedex";
/// Browser impersonation is the default for every official-source request.
/// No browser process, imported cookies, proxy, or challenge solving is involved.
pub fn http_client() -> Result<wreq::Client, wreq::Error> {
    wreq::Client::builder()
        .emulation(
            wreq_util::Emulation::builder()
                .profile(wreq_util::Profile::Chrome147)
                .platform(wreq_util::Platform::MacOS)
                .build(),
        )
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(wreq::redirect::Policy::limited(5))
        .no_proxy()
        .build()
}

#[derive(Debug)]
pub struct IndexPage {
    pub min: u32,
    pub max: u32,
    pub api_base: String,
    pub script_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpeciesLink {
    pub id: u32,
    pub number: String,
    pub name: String,
    #[serde(rename = "detailPageURL")]
    pub detail_path: String,
}

#[derive(Debug, Serialize)]
pub struct Species {
    pub id: u32,
    pub name: String,
    pub detail_path: String,
    pub original_key: String,
    pub forms: Vec<Form>,
}

#[derive(Debug, Serialize)]
pub struct Form {
    pub key: String,
    pub display_name: String,
    /// Detail URL plus the source selector value, not an artwork suffix.
    pub source_id: String,
    pub types: Vec<String>,
    pub artwork_url: String,
}

fn error(message: impl Into<String>) -> SourceError {
    SourceError(message.into())
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("constant selector")
}

fn document(text: &str) -> Result<Html, SourceError> {
    if !text.trim_end().ends_with("</html>") {
        return Err(error("truncated HTML"));
    }
    let html = Html::parse_document(text);
    if html.select(&selector("html[lang=en]")).count() != 1 {
        return Err(error(
            "not an English main-site document (anti-bot or source drift)",
        ));
    }
    Ok(html)
}

pub fn parse_index_page(text: &str) -> Result<IndexPage, SourceError> {
    let html = document(text)?;
    let bound = |css| -> Result<u32, SourceError> {
        let values: Vec<_> = html.select(&selector(css)).collect();
        if values.len() != 1 {
            return Err(error("missing or duplicate index bound"));
        }
        values[0]
            .value()
            .attr("value")
            .unwrap_or_default()
            .parse()
            .map_err(|_| error("invalid index bound"))
    };
    let min = bound("#minRangeBox")?;
    let max = bound("#maxRangeBox")?;
    if min == 0 || max < min {
        return Err(error("invalid source species range"));
    }
    let assignments: Vec<_> = text
        .split("window.pokemonUI.api[\"pokedex\"] = ")
        .skip(1)
        .collect();
    if assignments.len() != 1 {
        return Err(error("missing or duplicate Pokédex API declaration"));
    }
    let api_base: String =
        serde_json::from_str(assignments[0].split(';').next().unwrap_or_default())
            .map_err(|_| error("invalid Pokédex API declaration"))?;
    if !api_base.starts_with("/api/")
        || !api_base.ends_with('/')
        || api_base.contains(['?', '#', '\\'])
        || api_base.contains("..")
    {
        return Err(error("unexpected Pokédex API path"));
    }
    let scripts: Vec<_> = html.select(&selector("script[data-main]")).collect();
    if scripts.len() != 1 {
        return Err(error("missing or duplicate Pokédex script"));
    }
    let script_url = scripts[0].value().attr("data-main").unwrap_or_default();
    if script_url != "https://assets.pokemon.com/static2/_ui/js/pokedex.js" {
        return Err(error("unexpected Pokédex script"));
    }
    Ok(IndexPage {
        min,
        max,
        api_base,
        script_url: script_url.into(),
    })
}

/// The observed official script appends `kalos` to its page-configured base URL.
/// Require that source behavior instead of silently assuming a candidate API.
pub fn catalogue_url(page: &IndexPage, script: &str) -> Result<String, SourceError> {
    if !script.contains("urlBase:window.pokemonUI.api.pokedex")
        || !script.contains("this.url=this.urlBase+\"kalos\"")
    {
        return Err(error("Pokédex script discovery contract changed"));
    }
    Ok(format!("{ORIGIN}{}kalos", page.api_base))
}

pub fn parse_species_links(json: &str, page: &IndexPage) -> Result<Vec<SpeciesLink>, SourceError> {
    let links: Vec<SpeciesLink> =
        serde_json::from_str(json).map_err(|e| error(format!("invalid species JSON: {e}")))?;
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for link in &links {
        let slug = link
            .detail_path
            .strip_prefix("/us/pokedex/")
            .unwrap_or_default();
        if link.id < page.min
            || link.id > page.max
            || link.number != format!("{:04}", link.id)
            || link.name.trim().is_empty()
            || slug.is_empty()
            || !slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !ids.insert(link.id)
            || !paths.insert(&link.detail_path)
        {
            return Err(error(format!(
                "invalid or duplicate species identity: {}",
                link.id
            )));
        }
    }
    if u64::try_from(ids.len()).ok() != Some(u64::from(page.max) - u64::from(page.min) + 1) {
        return Err(error(
            "partial species catalogue: source bounds not covered",
        ));
    }
    Ok(links)
}

pub fn parse_species_detail(text: &str, link: &SpeciesLink) -> Result<Species, SourceError> {
    let html = document(text)?;
    let meta: Vec<_> = html.select(&selector("meta[name=pkm-id]")).collect();
    if meta.len() != 1
        || meta[0]
            .value()
            .attr("content")
            .and_then(|v| v.parse::<u32>().ok())
            != Some(link.id)
    {
        return Err(error(format!(
            "detail species identity mismatch for {}",
            link.id
        )));
    }
    let images: Vec<_> = html.select(&selector(".profile-images img")).collect();
    let panels: Vec<_> = html
        .select(&selector(".pokedex-pokemon-attributes"))
        .collect();
    let options: Vec<_> = html.select(&selector("#formes option")).collect();
    if images.is_empty()
        || images.len() != panels.len()
        || (!options.is_empty() && options.len() != images.len())
        || (options.is_empty() && images.len() != 1)
    {
        return Err(error(format!("partial form references for {}", link.id)));
    }
    let mut forms = Vec::new();
    let mut keys = BTreeSet::new();
    let mut artwork = BTreeSet::new();
    for (index, (image, panel)) in images.iter().zip(panels).enumerate() {
        let display_name = if let Some(option) = options.get(index) {
            if option
                .value()
                .attr("value")
                .and_then(|v| v.parse::<usize>().ok())
                != Some(index)
            {
                return Err(error("noncontiguous or duplicate form selector values"));
            }
            option.text().collect::<String>().trim().to_owned()
        } else {
            image
                .value()
                .attr("alt")
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        if display_name.is_empty() || image.value().attr("alt") != Some(display_name.as_str()) {
            return Err(error(format!(
                "missing or misaligned form label for {}:{index}",
                link.id
            )));
        }
        let label = display_name.replacen(&link.name, "", 1);
        let key = if index == 0 && label.trim().is_empty() {
            "original".into()
        } else {
            label
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        if key.is_empty() || !keys.insert(key.clone()) {
            return Err(error(format!(
                "duplicate or empty descriptive key for {}:{index}",
                link.id
            )));
        }
        let artwork_url = image.value().attr("src").unwrap_or_default();
        let filename = artwork_url
            .strip_prefix(
                "https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/",
            )
            .and_then(|v| v.strip_suffix(".png"))
            .unwrap_or_default();
        if filename.is_empty()
            || !filename
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !artwork.insert(artwork_url)
        {
            return Err(error("invalid or duplicate form artwork reference"));
        }
        let mut types = Vec::new();
        for element in panel.select(&selector(".dtm-type li a")) {
            let kind = element
                .value()
                .attr("href")
                .and_then(|v| v.strip_prefix("/us/pokedex?type="))
                .unwrap_or_default();
            if kind.is_empty()
                || !kind.bytes().all(|b| b.is_ascii_lowercase())
                || types.iter().any(|v| v == kind)
            {
                return Err(error("invalid or duplicate per-form type"));
            }
            types.push(kind.into());
        }
        if !(1..=2).contains(&types.len()) {
            return Err(error("missing or excessive form types"));
        }
        forms.push(Form {
            key,
            display_name,
            source_id: format!("{}#{index}", link.detail_path),
            types,
            artwork_url: artwork_url.into(),
        });
    }
    Ok(Species {
        id: link.id,
        name: link.name.clone(),
        detail_path: link.detail_path.clone(),
        original_key: forms[0].key.clone(),
        forms,
    })
}
