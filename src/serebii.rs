//! Source-linked Serebii illustration catalogue discovery and reconciliation.
use std::collections::{BTreeMap, BTreeSet};

use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

use crate::source::SourceError;

pub const ORIGIN: &str = "https://www.serebii.net";
pub const INDEX_URL: &str = "https://www.serebii.net/pokemon/nationalpokedex.shtml";

#[derive(Debug, Clone, Serialize)]
pub struct SpeciesLink {
    pub id: u32,
    pub name: String,
    pub detail_path: String,
}

#[derive(Debug, Serialize)]
pub struct Index {
    pub min: u32,
    pub max: u32,
    pub species: Vec<SpeciesLink>,
}

#[derive(Debug, Serialize)]
pub struct Artwork {
    pub label: String,
    pub url: String,
    pub section: String,
    pub aliases: Vec<String>,
    /// Distinct named slots in the source's alternate-form tables.
    pub form_slots: BTreeSet<String>,
}

#[derive(Debug, Serialize)]
pub struct Hub {
    pub id: u32,
    pub name: String,
    pub source_url: String,
    pub original_artwork: String,
    pub artwork: Vec<Artwork>,
    pub game_links: Vec<String>,
    /// Observed source groups only; joining these to every artwork is a separate gate.
    pub type_groups: Vec<TypeGroup>,
    pub non_battle_device_context: Option<String>,
    pub evolution_conditions: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TypeGroup {
    pub section: String,
    pub label: String,
    pub types: Vec<String>,
    pub artwork_url: Option<String>,
}

fn selector(value: &str) -> Result<Selector, SourceError> {
    Selector::parse(value)
        .map_err(|parse_error| error(format!("invalid CSS selector {value}: {parse_error}")))
}

fn text(element: ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn error(message: impl Into<String>) -> SourceError {
    SourceError(message.into())
}

fn document(html: &str) -> Result<Html, SourceError> {
    if !html.to_ascii_lowercase().contains("</html>") {
        return Err(error("missing HTML end marker"));
    }
    let document = Html::parse_document(html);
    for image in document.select(&selector("img[src]")?) {
        let Some(src) = image.attr("src") else {
            continue;
        };
        for namespace in [
            "/pokemon/art/",
            "/pokedex-bw/type/",
            "/pokedex-dp/type/",
            "/pokedex-sv/teraicon/",
        ] {
            if src.contains(namespace.trim_start_matches('/'))
                && !source_path(src).is_some_and(|path| path.starts_with(namespace))
            {
                return Err(error(format!("unsupported source asset URL: {src}")));
            }
        }
    }
    let title = document
        .select(&selector("title")?)
        .next()
        .map(text)
        .unwrap_or_default();
    if !title.contains("Pokédex") && !title.contains("Pokémon") {
        return Err(error(format!("unexpected source title: {title}")));
    }
    Ok(document)
}

fn source_path(value: &str) -> Option<&str> {
    let path = value
        .strip_prefix(ORIGIN)
        .or_else(|| value.strip_prefix("//www.serebii.net"))
        .unwrap_or(value);
    (path.starts_with('/') && !path.starts_with("//")).then_some(path)
}

fn source_images<'a>(
    element: ElementRef<'a>,
    prefix: &'a str,
) -> Result<impl Iterator<Item = ElementRef<'a>>, SourceError> {
    let selector = selector("img[src]")?;
    Ok(element
        .select(&selector)
        .collect::<Vec<_>>()
        .into_iter()
        .filter(move |img| {
            img.attr("src")
                .and_then(source_path)
                .is_some_and(|path| path.starts_with(prefix))
        }))
}

/// Parse the National Pokédex index and validate its advertised ranges.
///
/// # Errors
///
/// Returns an error for malformed HTML, inconsistent ranges, or incomplete species rows.
pub fn parse_index(html: &str) -> Result<Index, SourceError> {
    let document = document(html)?;
    let mut ranges = Vec::new();
    for a in document.select(&selector("a[href]")?) {
        let href = a.attr("href").unwrap_or_default();
        if href.starts_with("gen") && href.ends_with("pokemon.shtml") {
            let label = text(a);
            let Some((min, max)) = label.split_once('-') else {
                return Err(error("generation range missing"));
            };
            ranges.push((
                min.parse::<u32>()
                    .map_err(|_| error("invalid source lower bound"))?,
                max.parse::<u32>()
                    .map_err(|_| error("invalid source upper bound"))?,
            ));
        }
    }
    if ranges.is_empty()
        || ranges.first().is_some_and(|(lo, _)| *lo == 0)
        || ranges.iter().any(|(lo, hi)| lo > hi)
        || ranges.windows(2).any(|w| {
            w.first()
                .zip(w.get(1))
                .is_none_or(|(left, right)| left.1.checked_add(1) != Some(right.0))
        })
    {
        return Err(error(
            "missing or inconsistent advertised generation ranges",
        ));
    }
    let Some((min, _)) = ranges.first().copied() else {
        return Err(error("missing advertised generation ranges"));
    };
    let Some((_, max)) = ranges.last().copied() else {
        return Err(error("missing advertised generation ranges"));
    };
    let tables: Vec<_> = document.select(&selector("table.dextable")?).collect();
    if tables.len() != 1 {
        return Err(error("expected one National Pokédex table"));
    }
    let mut species = Vec::new();
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut names = BTreeSet::new();
    let table = tables
        .first()
        .ok_or_else(|| error("expected one National Pokédex table"))?;
    for row in table.select(&selector(":scope > tbody > tr")?) {
        if let Some(link) = parse_index_row(row, &mut ids, &mut paths, &mut names)? {
            species.push(link);
        }
    }
    let expected_count = max
        .checked_sub(min)
        .and_then(|range| range.checked_add(1))
        .ok_or_else(|| error("invalid advertised species range"))?;
    if u32::try_from(species.len()).ok() != Some(expected_count)
        || !ids.iter().copied().eq(min..=max)
    {
        return Err(error(
            "partial index: species do not match advertised source ranges",
        ));
    }
    Ok(Index { min, max, species })
}

fn parse_index_row(
    row: ElementRef<'_>,
    ids: &mut BTreeSet<u32>,
    paths: &mut BTreeSet<String>,
    names: &mut BTreeSet<String>,
) -> Result<Option<SpeciesLink>, SourceError> {
    let cells: Vec<_> = row
        .children()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "td")
        .collect();
    if cells.first().is_some_and(|cell| {
        cell.value()
            .has_class("fooevo", scraper::CaseSensitivity::CaseSensitive)
    }) {
        return Ok(None);
    }
    if cells.len() != 11 {
        return Err(error(format!(
            "unexpected index row cell count {}",
            cells.len()
        )));
    }
    let number = text(
        *cells
            .first()
            .ok_or_else(|| error("index row lacks National number"))?,
    );
    let id = number
        .strip_prefix('#')
        .ok_or_else(|| error("missing National number"))?
        .parse::<u32>()
        .map_err(|_| error("invalid National number"))?;
    let name_cell = cells
        .get(2)
        .ok_or_else(|| error("index row lacks species name"))?;
    let links: Vec<_> = name_cell.select(&selector("a[href]")?).collect();
    let [link] = links.as_slice() else {
        return Err(error("ambiguous species detail link"));
    };
    let detail_path = link
        .attr("href")
        .ok_or_else(|| error("species detail link lacks href"))?
        .to_owned();
    let slug = detail_path
        .strip_prefix("/pokemon/")
        .ok_or_else(|| error("non-hub species link"))?;
    if slug.is_empty()
        || slug.contains("..")
        || !slug.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'\'' | b':')
        })
    {
        return Err(error("invalid species path"));
    }
    let name = text(*link);
    if name.is_empty()
        || !ids.insert(id)
        || !paths.insert(detail_path.clone())
        || !names.insert(name.to_lowercase())
    {
        return Err(error("empty or duplicate species identity"));
    }
    Ok(Some(SpeciesLink {
        id,
        name,
        detail_path,
    }))
}

fn cells(row: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    row.children()
        .filter_map(ElementRef::wrap)
        .filter(|el| matches!(el.value().name(), "td" | "th"))
        .collect()
}

fn rows(table: ElementRef<'_>) -> Result<Vec<ElementRef<'_>>, SourceError> {
    Ok(table.select(&selector(":scope > tbody > tr")?).collect())
}

fn span(cell: ElementRef<'_>) -> usize {
    cell.attr("colspan")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn column(cell: ElementRef<'_>) -> usize {
    cell.prev_siblings()
        .filter_map(ElementRef::wrap)
        .filter(|el| matches!(el.value().name(), "td" | "th"))
        .map(span)
        .sum()
}

fn heading(table: ElementRef<'_>, col: usize) -> Result<String, SourceError> {
    let Some(row) = rows(table)?.first().copied() else {
        return Ok(String::new());
    };
    let headers = cells(row);
    if headers.len() > 1 && headers.iter().all(|h| span(*h) > 1) {
        let mut start: usize = 0;
        for h in headers {
            let end = start
                .checked_add(span(h))
                .ok_or_else(|| error("header column span overflow"))?;
            if (start..end).contains(&col) {
                return Ok(text(h));
            }
            start = end;
        }
    }
    let label = table
        .select(&selector("td.fooevo, td.foo")?)
        .next()
        .map(text)
        .unwrap_or_default();
    if label == "Name"
        && let Some(previous) = table.prev_siblings().find_map(ElementRef::wrap)
        && previous.value().name() == "table"
        && let Some(form) = previous.select(&selector("h3")?).next()
    {
        return Ok(text(form));
    }
    Ok(label)
}

fn artwork_url(path: &str) -> Result<String, SourceError> {
    let path = source_path(path).ok_or_else(|| error("unsupported illustration origin"))?;
    let file = path
        .strip_prefix("/pokemon/art/")
        .and_then(|s| s.strip_suffix(".png"))
        .ok_or_else(|| error(format!("non-illustration PNG reference: {path}")))?;
    if file.is_empty()
        || !file
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(error(format!("invalid artwork path: {path}")));
    }
    Ok(format!("{ORIGIN}{path}"))
}

fn clean_label(label: &str) -> String {
    let clean = label.replace("<br />", " ").replace("<br>", " ");
    clean
        .strip_suffix(" Artwork")
        .unwrap_or(&clean)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn record_artwork(
    artwork: &mut Vec<Artwork>,
    label: String,
    url: String,
    section: String,
    slot: Option<String>,
) {
    if let Some(existing) = artwork.iter_mut().find(|a| a.url == url) {
        if existing.section == "Evolutionary Chain" && section != "Evolutionary Chain" {
            existing.label.clone_from(&label);
            existing.section = section;
        }
        if !existing.aliases.contains(&label) {
            existing.aliases.push(label);
        }
        existing.form_slots.extend(slot);
    } else {
        artwork.push(Artwork {
            label: label.clone(),
            url,
            section,
            aliases: vec![label],
            form_slots: slot.into_iter().collect(),
        });
    }
}

/// Parse one species hub, including its illustration and type evidence.
///
/// # Errors
///
/// Returns an error for malformed source markup, unsupported assets, or inconsistent species data.
pub fn parse_hub(html: &str, species: &SpeciesLink) -> Result<Hub, SourceError> {
    let document = document(html)?;
    let title = document
        .select(&selector("title")?)
        .next()
        .map(text)
        .unwrap_or_default();
    if !title.starts_with(&format!("{} -", species.name))
        || !document
            .select(&selector("td b, h1")?)
            .any(|el| text(el) == format!("#{:04} {}", species.id, species.name))
    {
        return Err(error(format!("species title mismatch: {title}")));
    }
    let og: Vec<_> = document
        .select(&selector("meta[property='og:image']")?)
        .collect();
    if og.len() != 1 {
        return Err(error("missing or ambiguous original OG portrait"));
    }
    let og_image = og
        .first()
        .ok_or_else(|| error("missing original OG portrait"))?;
    let original_artwork = artwork_url(og_image.attr("content").unwrap_or_default())?;
    let original_file = original_artwork
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".png"))
        .ok_or_else(|| error("invalid original artwork URL"))?;
    if original_file.parse::<u32>().ok() != Some(species.id) {
        return Err(error("original portrait National identifier mismatch"));
    }
    let (mut artwork, evolution_conditions) = extract_hub_table_artwork(&document, species)?;
    extract_hub_selector_artwork(&document, html, species, &mut artwork)?;
    if !artwork.iter().any(|art| art.url == original_artwork) {
        return Err(error("OG portrait has no displayed artwork reference"));
    }
    let game_links = hub_game_links(&document)?;
    if game_links.is_empty() {
        return Err(error("missing source-linked game databases"));
    }
    let type_groups = extract_hub_type_groups(&document, &artwork)?;
    let non_battle_device_context = document
        .select(&selector("table.dextable table.pkmn td")?)
        .map(text)
        .find(|content| content.contains("not usable as a Pokémon"));
    Ok(Hub {
        id: species.id,
        name: species.name.clone(),
        source_url: format!("{ORIGIN}{}", species.detail_path),
        original_artwork,
        evolution_conditions,
        game_links,
        type_groups,
        artwork,
        non_battle_device_context,
    })
}

fn extract_hub_table_artwork(
    document: &Html,
    species: &SpeciesLink,
) -> Result<(Vec<Artwork>, BTreeMap<String, String>), SourceError> {
    let mut artwork = vec![];
    let mut unlabelled_evolution = BTreeSet::new();
    let mut evolution_conditions = BTreeMap::new();
    for table in document.select(&selector("table.dextable")?) {
        let independently_typed = table
            .select(&selector(
                ":scope > tbody > tr > td.fooevo, :scope > tbody > tr > td.foo",
            )?)
            .any(|header| text(header) == "Type");
        for img in source_images(table, "/pokemon/art/")? {
            let cell = img
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().name() == "td")
                .ok_or_else(|| error("artwork outside source cell"))?;
            let section = heading(table, column(cell))?;
            if section == "Evolutionary Chain" {
                let owner = img
                    .ancestors()
                    .filter_map(ElementRef::wrap)
                    .find(|e| e.value().name() == "a")
                    .and_then(|e| e.attr("href"));
                if owner.map(|s| s.trim_end_matches('/'))
                    != Some(species.detail_path.trim_end_matches('/'))
                {
                    continue;
                }
            }
            let url = artwork_url(
                img.attr("src")
                    .ok_or_else(|| error("illustration lacks src"))?,
            )?;
            let image_species = url
                .rsplit('/')
                .next()
                .ok_or_else(|| error("invalid illustration URL"))?
                .split(['-', '.'])
                .next()
                .ok_or_else(|| error("invalid illustration filename"))?
                .parse::<u32>()
                .ok();
            if image_species != Some(species.id) {
                if section == "Evolutionary Chain" {
                    continue;
                }
                return Err(error(format!(
                    "illustration belongs to another species: {url}"
                )));
            }
            let raw = img.attr("alt").unwrap_or("");
            if section == "Evolutionary Chain"
                && let Some(condition) = evolution_condition(cell)?
            {
                evolution_conditions.insert(url.clone(), condition);
            }
            if raw.is_empty() && section == "Evolutionary Chain" {
                unlabelled_evolution.insert(url);
                continue;
            }
            let label = clean_label(raw);
            // Column captions identify forms even when a copied image alt is wrong.
            let caption = if section == "Alternate Forms" {
                form_caption(cell)?
            } else {
                None
            };
            let (mut label, slot) = if let Some((caption, caption_slot)) = caption {
                (caption, Some(caption_slot))
            } else {
                (
                    label,
                    independently_typed
                        .then(|| format!("typed:{:?}:column:{}", table.id(), column(cell))),
                )
            };
            if let Some((_, name)) = section.split_once(" - ") {
                label.clone_from(&name.to_owned());
            }
            if section.starts_with("Gigantamax ") {
                label.clone_from(&section);
            }
            if label.is_empty() {
                return Err(error(format!("artwork lacks form label: {url}")));
            }
            record_artwork(
                &mut artwork,
                label.clone(),
                url.clone(),
                section.clone(),
                slot,
            );
        }
    }
    add_unlabelled_evolution(&mut artwork, unlabelled_evolution);
    Ok((artwork, evolution_conditions))
}

fn form_caption(cell: ElementRef<'_>) -> Result<Option<(String, String)>, SourceError> {
    let col = column(cell);
    let Some(row) = cell.parent().and_then(ElementRef::wrap) else {
        return Ok(None);
    };
    let bold_selector = selector("b")?;
    let image_selector = selector("img")?;
    for prior in row
        .prev_siblings()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "tr")
    {
        let candidate = cells(prior)
            .into_iter()
            .find(|candidate| column(*candidate) == col);
        if let Some(candidate) = candidate.filter(|candidate| {
            candidate.select(&bold_selector).next().is_some()
                && candidate.select(&image_selector).next().is_none()
        }) {
            let slot = format!("caption:{:?}:column:{col}", prior.id());
            return Ok(Some((text(candidate), slot)));
        }
    }
    Ok(None)
}

fn add_unlabelled_evolution(artwork: &mut Vec<Artwork>, urls: BTreeSet<String>) {
    for url in urls {
        if !artwork.iter().any(|art| art.url == url) {
            record_artwork(
                artwork,
                String::new(),
                url,
                "Evolutionary Chain".into(),
                None,
            );
        }
    }
}

fn extract_hub_selector_artwork(
    document: &Html,
    html: &str,
    species: &SpeciesLink,
    artwork: &mut Vec<Artwork>,
) -> Result<(), SourceError> {
    for button in document.select(&selector("a.sprite-select[data-key]")?) {
        let key = button
            .attr("data-key")
            .ok_or_else(|| error("artwork selector lacks key"))?;
        if !html.contains("src = '//www.serebii.net/pokemon/art/' + key + '.png'") {
            return Err(error("unrecognized artwork selector script"));
        }
        let url = artwork_url(&format!("/pokemon/art/{key}.png"))?;
        if key.split('-').next().and_then(|n| n.parse::<u32>().ok()) != Some(species.id) {
            return Err(error("selector belongs to another species"));
        }
        let selector = selector("img[alt]")?;
        let label = button
            .attr("title")
            .or_else(|| button.select(&selector).next().and_then(|i| i.attr("alt")))
            .ok_or_else(|| error("selector lacks form label"))?;
        record_artwork(
            artwork,
            clean_label(label),
            url,
            "Artwork selector".into(),
            Some(format!("selector:{key}")),
        );
    }
    Ok(())
}

fn hub_game_links(document: &Html) -> Result<Vec<String>, SourceError> {
    let mut game_links = Vec::new();
    for link in document.select(&selector(".hubheader a[href]")?) {
        let Some(path) = link.attr("href") else {
            continue;
        };
        if (path.starts_with("/pokedex") || path.starts_with("/pokemongo/pokemon/"))
            && !path.contains(['?', '#'])
            && !path.contains("..")
            && !game_links.iter().any(|existing| existing == path)
        {
            game_links.push(path.to_owned());
        }
    }
    Ok(game_links)
}

fn extract_hub_type_groups(
    document: &Html,
    artwork: &[Artwork],
) -> Result<Vec<TypeGroup>, SourceError> {
    let mut groups = extract_type_evidence(document)?;
    for art in artwork {
        if !art.label.ends_with(" Tera") {
            continue;
        }
        for img in source_images(document.root_element(), "/pokemon/art/")? {
            let Some(src) = img.attr("src") else {
                continue;
            };
            if artwork_url(src)? != art.url {
                continue;
            }
            let Some(cell) = img
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().name() == "td")
            else {
                continue;
            };
            let Some(table) = cell
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().name() == "table")
            else {
                continue;
            };
            for row in rows(table)? {
                for candidate in cells(row)
                    .into_iter()
                    .filter(|c| column(*c) == column(cell) && text(*c).contains("Tera Type"))
                {
                    let icons: Vec<_> =
                        source_images(candidate, "/pokedex-sv/teraicon/")?.collect();
                    if icons.len() == 1 {
                        let kind = icons
                            .first()
                            .and_then(|icon| icon.attr("src"))
                            .ok_or_else(|| error("Tera type icon lacks src"))?
                            .rsplit('/')
                            .next()
                            .ok_or_else(|| error("invalid Tera type icon URL"))?
                            .strip_suffix(".png")
                            .unwrap_or("");
                        if !TYPES.contains(&kind) {
                            return Err(error("unsupported explicit Tera type"));
                        }
                        let group = TypeGroup {
                            section: "Terastallized form / Tera Type".into(),
                            label: art.label.clone(),
                            types: vec![kind.into()],
                            artwork_url: Some(art.url.clone()),
                        };
                        if !groups.contains(&group) {
                            groups.push(group);
                        }
                    }
                }
            }
        }
    }
    Ok(groups)
}

fn evolution_condition(cell: ElementRef<'_>) -> Result<Option<String>, SourceError> {
    for sibling in [
        cell.prev_siblings()
            .find(|n| ElementRef::wrap(*n).is_some()),
        cell.next_siblings()
            .find(|n| ElementRef::wrap(*n).is_some()),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(cell) = ElementRef::wrap(sibling)
            && let Some(condition) = cell
                .select(&selector("img[title]")?)
                .next()
                .and_then(|img| img.attr("title"))
            && !condition.is_empty()
        {
            return Ok(Some(
                condition.split_whitespace().collect::<Vec<_>>().join(" "),
            ));
        }
    }
    Ok(None)
}

const TYPES: [&str; 18] = [
    "normal", "fire", "water", "electric", "grass", "ice", "fighting", "poison", "ground",
    "flying", "psychic", "bug", "rock", "ghost", "dragon", "dark", "steel", "fairy",
];

fn read_types(panel: ElementRef<'_>) -> Result<Vec<String>, SourceError> {
    let mut types = vec![];
    for img in panel.select(&selector("img[src]")?) {
        let Some(path) = img.attr("src").and_then(source_path) else {
            continue;
        };
        let Some(file) = path
            .strip_prefix("/pokedex-bw/type/")
            .or_else(|| path.strip_prefix("/pokedex-dp/type/"))
        else {
            continue;
        };
        let kind = file
            .strip_suffix(".gif")
            .ok_or_else(|| error("unexpected type icon URL"))?;
        if !TYPES.contains(&kind) {
            return Err(error(format!("unknown source type: {kind}")));
        }
        types.push(kind.to_owned());
    }
    if types.len() > 2 || types.iter().collect::<BTreeSet<_>>().len() != types.len() {
        return Err(error("ambiguous source type panel"));
    }
    Ok(types)
}

/// Explicit source type panels, retaining column identities and direct illustration bindings.
/// Parse explicit type panels from a linked Serebii page.
///
/// # Errors
///
/// Returns an error for malformed HTML, missing panels, or unsupported type icons.
pub fn parse_type_evidence(html: &str) -> Result<Vec<TypeGroup>, SourceError> {
    let document = document(html)?;
    extract_type_evidence(&document)
}

fn extract_type_evidence(document: &Html) -> Result<Vec<TypeGroup>, SourceError> {
    let mut groups = vec![];
    for table in document.select(&selector("table.dextable, table.dextab")?) {
        let headers: Vec<_> = table
            .select(&selector(
                ":scope > tbody > tr > td.fooevo, :scope > tbody > tr > td.foo",
            )?)
            .collect();
        if !headers.iter().any(|h| text(*h) == "Type") {
            continue;
        }
        let section = heading(table, 0)?;
        if !headers
            .iter()
            .any(|h| matches!(text(*h).as_str(), "Numbers" | "No." | "Other Names"))
            && ![
                "Mega Evolution - ",
                "Primal Reversion - ",
                "Ultra Burst - ",
                "Gigantamax ",
            ]
            .iter()
            .any(|prefix| section.starts_with(prefix))
        {
            continue;
        }
        for cell in table.select(&selector(
            ":scope > tbody > tr > td.cen, :scope > tbody > tr > td.fooinfo",
        )?) {
            let section = heading(table, column(cell))?;
            let nested: Vec<_> = cell.select(&selector("table > tbody > tr")?).collect();
            let panels = if nested.is_empty() {
                vec![cell]
            } else {
                nested
            };
            let mut region = None;
            for panel in panels {
                let types = read_types(panel)?;
                if types.is_empty() {
                    continue;
                }
                let raw = text(panel);
                let normalized = raw.to_lowercase();
                let label = if ["alolan", "galarian", "hisuian", "paldean"]
                    .contains(&normalized.as_str())
                {
                    region = Some(raw.clone());
                    raw
                } else if normalized == "normal" {
                    region = None;
                    raw
                } else if let Some(region) = &region {
                    format!("{region} {raw}")
                } else {
                    raw
                };
                let urls: Vec<_> = source_images(table, "/pokemon/art/")?
                    .filter_map(|img| {
                        let c = img
                            .ancestors()
                            .filter_map(ElementRef::wrap)
                            .find(|e| e.value().name() == "td")?;
                        (heading(table, column(c)).ok().as_deref() == Some(section.as_str()))
                            .then(|| img.attr("src"))
                            .flatten()
                    })
                    .collect();
                let artwork_url = if urls.len() == 1 && label.is_empty() {
                    Some(artwork_url(
                        urls.first().ok_or_else(|| error("missing artwork URL"))?,
                    )?)
                } else {
                    None
                };
                let group = TypeGroup {
                    section: section.clone(),
                    label,
                    types,
                    artwork_url,
                };
                if !groups.contains(&group) {
                    groups.push(group);
                }
            }
        }
    }
    if groups.is_empty() {
        return Err(error("missing explicit type panels"));
    }
    Ok(groups)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Species {
    pub id: u32,
    pub name: String,
    pub source_url: String,
    pub original_form: String,
    pub forms: Vec<Form>,
    pub excluded_illustrations: Vec<ExcludedIllustration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExcludedIllustration {
    pub label: String,
    pub artwork_url: String,
    pub reason: ExclusionReason,
    pub source_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    NonBattleDeviceWithoutTypes,
    ApprovedUnavailableIllustration,
    ApprovedUnsupportedTypeEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Form {
    pub key: String,
    pub display_name: String,
    pub source_id: String,
    pub source_labels: Vec<String>,
    pub artwork_url: String,
    pub artwork_references: Vec<String>,
    pub types: Vec<String>,
    pub type_source: String,
    pub type_label: String,
}

#[derive(Debug, Serialize)]
pub struct Enrichment {
    pub source_url: String,
    pub groups: Vec<TypeGroup>,
    pub artwork_labels: BTreeMap<String, String>,
    pub named_forms: BTreeSet<String>,
}

/// Parse linked game data used to enrich labels and type assignments.
///
/// # Errors
///
/// Returns an error for unlinked paths, malformed source pages, or mismatched species identities.
pub fn parse_enrichment(hub: &Hub, path: &str, html: &str) -> Result<Enrichment, SourceError> {
    if !hub.game_links.iter().any(|link| link == path) {
        return Err(error("type evidence is not linked by the species hub"));
    }
    let doc = document(html)?;
    let title = doc
        .select(&selector("title")?)
        .next()
        .map(text)
        .unwrap_or_default();
    if !title.starts_with(&format!("{} -", hub.name))
        && title != format!("Serebii.net Pokédex - #{} {}", hub.id, hub.name)
    {
        return Err(error("type enrichment species title mismatch"));
    }
    let matched = doc.select(&selector("table.dextable")?).any(|table| {
        let content = text(table);
        let words: Vec<_> = content
            .split_whitespace()
            .filter(|word| *word != ":")
            .collect();
        words.windows(2).any(|window| {
            let Some((label, number)) = window.first().zip(window.get(1)) else {
                return false;
            };
            label.trim_end_matches(':') == "National"
                && number.strip_prefix('#').and_then(|s| s.parse::<u32>().ok()) == Some(hub.id)
        })
    });
    let heading_matches = doc
        .select(&selector("td b, h1")?)
        .any(|el| text(el) == format!("#{:04} {}", hub.id, hub.name));
    if !matched && !heading_matches {
        return Err(error("type enrichment National number mismatch"));
    }
    let mut named_forms = BTreeSet::new();
    let mut labelled_images: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for img in doc.select(&selector("img.formpic[src]")?) {
        if let Some(label) = img.attr("alt").or_else(|| img.attr("title")) {
            let label = clean_label(label);
            named_forms.insert(form_key(&label, &hub.name));
            labelled_images
                .entry(
                    img.attr("src")
                        .ok_or_else(|| error("form image lacks src"))?
                        .to_owned(),
                )
                .or_default()
                .insert(label);
        }
    }
    for button in doc.select(&selector("a.sprite-select[title]")?) {
        if let Some(title) = button.attr("title") {
            named_forms.insert(form_key(title, &hub.name));
        }
    }
    for table in doc.select(&selector("table.dextable, table.dextab")?) {
        let section = heading(table, 0)?;
        if section.starts_with("Alternate Forms") {
            for caption in table.select(&selector("td.pkmn > b")?) {
                named_forms.insert(form_key(&text(caption), &hub.name));
            }
        }
        if let Some(label) = section.strip_prefix("Stats - ") {
            named_forms.insert(form_key(label, &hub.name));
        }
    }
    let artwork_labels = enrichment_artwork_labels(&doc, hub, path, &labelled_images)?;
    Ok(Enrichment {
        source_url: format!("{ORIGIN}{path}"),
        groups: extract_type_evidence(&doc)?,
        artwork_labels,
        named_forms,
    })
}

fn enrichment_artwork_labels(
    doc: &Html,
    hub: &Hub,
    path: &str,
    labelled_images: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeMap<String, String>, SourceError> {
    let mut artwork_labels = BTreeMap::new();
    for (art, condition) in &hub.evolution_conditions {
        let mut labels = BTreeSet::new();
        for link in doc.select(&selector("table.evochain a[href]")?) {
            if link.attr("href").map(|href| href.trim_end_matches('/'))
                != Some(path.trim_end_matches('/'))
            {
                continue;
            }
            let Some(cell) = link
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|element| element.value().name() == "td")
            else {
                continue;
            };
            if evolution_condition(cell)?.as_ref() != Some(condition) {
                continue;
            }
            for image in link.select(&selector("img[src]")?) {
                if let Some(names) = image.attr("src").and_then(|src| labelled_images.get(src)) {
                    labels.extend(names.iter().cloned());
                }
            }
        }
        if labels.len() == 1
            && let Some(label) = labels.into_iter().next()
        {
            artwork_labels.insert(art.clone(), label);
        }
    }
    Ok(artwork_labels)
}

/// Lexical source aliases only: no species IDs, image suffixes, or assigned types.
#[must_use]
pub fn form_key(label: &str, species: &str) -> String {
    fn words(s: &str) -> String {
        s.to_lowercase()
            .replace(['(', ')', ':', '.', '-'], " ")
            .replace('é', "e")
            .split_whitespace()
            .filter(|s| !matches!(*s, "form" | "forme"))
            .map(|s| match s {
                "alola" => "alolan",
                "galar" => "galarian",
                "hisui" => "hisuian",
                "paldea" => "paldean",
                _ => s,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    let lower = label.to_lowercase();
    let label = lower
        .strip_prefix("type: ")
        .or_else(|| lower.strip_suffix("-type"))
        .unwrap_or(label);
    let value = words(label);
    let name = words(species);
    // Match complete words even when the species occurs between "Mega" and "X".
    let value = format!(" {value} ")
        .replace(&format!(" {name} "), " ")
        .trim()
        .to_owned();
    let value = value.replace("pokédex", "dex").replace("pokedex", "dex");
    match value.as_str() {
        "" | "normal" | "natural" => "original".into(),
        "unovan zen mode" => "zen mode".into(),
        "paldean combat breed" => "paldean".into(),
        "galarian standard mode" | "galarian normal mode" => "galarian".into(),
        _ => value,
    }
}

fn group_key(group: &TypeGroup, name: &str) -> String {
    if !group.label.is_empty() {
        form_key(&group.label, name)
    } else if let Some((_, label)) = group.section.split_once(" - ") {
        form_key(label, name)
    } else {
        form_key(&group.section, name)
    }
}

fn label_type(label: &str) -> Option<Vec<String>> {
    let label = label.to_lowercase();
    let name = label
        .strip_prefix("type: ")
        .or_else(|| label.strip_suffix("-type"))?;
    TYPES.contains(&name).then(|| vec![name.to_owned()])
}

/// Exact illustration aliases established by source captions and reviewed depictions.
/// These do not imply any relationship between game-render and illustration filenames.
fn supported_original_alias(hub: &Hub, art: &Artwork, label: &str) -> Result<bool, SourceError> {
    const ALIASES: &[(u32, &str, &str, &str)] = &[
        (
            666,
            "https://www.serebii.net/pokemon/art/666.png",
            "https://www.serebii.net/pokemon/art/666-m.png",
            "Meadow Pattern",
        ),
        (
            669,
            "https://www.serebii.net/pokemon/art/669.png",
            "https://www.serebii.net/pokemon/art/669-r.png",
            "Red Flower",
        ),
        (
            670,
            "https://www.serebii.net/pokemon/art/670.png",
            "https://www.serebii.net/pokemon/art/670-r.png",
            "Red Flower",
        ),
        (
            671,
            "https://www.serebii.net/pokemon/art/671.png",
            "https://www.serebii.net/pokemon/art/671-r.png",
            "Red Flower",
        ),
        (
            778,
            "https://www.serebii.net/pokemon/art/778.png",
            "https://www.serebii.net/pokemon/art/778-d.png",
            "Disguised Form",
        ),
    ];
    for (id, original, named, source_label) in ALIASES {
        if hub.id != *id || art.url == hub.original_artwork {
            continue;
        }
        if art.url == *named || form_key(label, &hub.name) == form_key(source_label, &hub.name) {
            if hub.original_artwork == *original && art.url == *named && label == *source_label {
                return Ok(true);
            }
            return Err(error(format!(
                "verified original alias identity changed: {} / {label}",
                art.url
            )));
        }
    }
    Ok(false)
}

/// Join illustration identities to source-linked type evidence.
///
/// # Errors
///
/// Returns an error when an illustration is unlabelled, type assignments conflict, or the original form is lost.
pub fn reconcile(hub: &Hub, enrichment: &[Enrichment]) -> Result<Species, SourceError> {
    let main: Vec<_> = hub
        .type_groups
        .iter()
        .filter(|g| g.section == "Picture")
        .collect();
    let original_types = main
        .first()
        .ok_or_else(|| error("missing original source type panel"))?;
    let mut original_aliases = BTreeSet::from(["original".to_owned()]);
    if !original_types.label.is_empty() {
        original_aliases.insert(form_key(&original_types.label, &hub.name));
    }
    if let Some(original) = hub.artwork.iter().find(|a| a.url == hub.original_artwork) {
        for label in &original.aliases {
            original_aliases.insert(form_key(label, &hub.name));
        }
    }
    let ClassifiedArtwork {
        grouped,
        resolved_labels,
        mut problems,
        excluded_illustrations,
    } = classify_artwork(hub, enrichment, &original_aliases)?;
    let forms = reconcile_forms(
        hub,
        enrichment,
        original_types,
        grouped,
        &resolved_labels,
        &mut problems,
    );
    if !problems.is_empty() {
        return Err(error(format!(
            "{} {}: {}",
            hub.id,
            hub.name,
            problems.join("; ")
        )));
    }
    if !forms
        .iter()
        .any(|f| f.key == "original" && f.artwork_url == hub.original_artwork)
    {
        return Err(error("original form association lost"));
    }
    Ok(Species {
        id: hub.id,
        name: hub.name.clone(),
        source_url: hub.source_url.clone(),
        original_form: "original".into(),
        forms,
        excluded_illustrations,
    })
}

fn reconcile_forms(
    hub: &Hub,
    enrichment: &[Enrichment],
    original_types: &TypeGroup,
    grouped: BTreeMap<String, Vec<&Artwork>>,
    resolved_labels: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) -> Vec<Form> {
    let mut forms = vec![];
    for (key, mut images) in grouped {
        let slots: BTreeSet<_> = images.iter().flat_map(|art| &art.form_slots).collect();
        if images.len() > 1
            && (slots
                .iter()
                .filter(|slot| slot.starts_with("caption:"))
                .count()
                > 1
                || slots
                    .iter()
                    .filter(|slot| slot.starts_with("selector:"))
                    .count()
                    > 1
                || slots
                    .iter()
                    .filter(|slot| slot.starts_with("typed:"))
                    .count()
                    > 1)
        {
            problems.push(format!("{key}: distinct source form slots collide"));
            continue;
        }
        images.sort_by_key(|a| {
            (
                a.url != hub.original_artwork,
                !hub.type_groups
                    .iter()
                    .any(|g| g.artwork_url.as_ref() == Some(&a.url)),
                a.section == "Evolutionary Chain",
                a.section == "Artwork selector",
            )
        });
        let Some(image) = images.first() else {
            problems.push(format!("{key}: no source artwork"));
            continue;
        };
        let TypeAssignment {
            types,
            source: type_source,
            label: type_label,
        } = match form_type_assignment(hub, enrichment, original_types, &key, &images) {
            Ok(assignment) => assignment,
            Err(assignment_count) => {
                problems.push(format!("{key}: {assignment_count} type assignments"));
                continue;
            }
        };
        let mut labels = BTreeSet::new();
        for art in &images {
            labels.extend(art.aliases.iter().cloned());
            if let Some(label) = resolved_labels.get(&art.url) {
                labels.insert(label.clone());
            }
        }
        let Some(display_name) = resolved_labels.get(&image.url) else {
            problems.push(format!("{key}: missing resolved display name"));
            continue;
        };
        forms.push(Form {
            display_name: key.strip_prefix("gigantamax ").map_or_else(
                || display_name.clone(),
                |style| format!("{display_name} ({style})"),
            ),
            key,
            source_id: image.url.clone(),
            source_labels: labels.into_iter().collect(),
            artwork_url: image.url.clone(),
            artwork_references: images.iter().map(|a| a.url.clone()).collect(),
            types,
            type_source,
            type_label,
        });
    }
    forms
}

struct TypeAssignment {
    types: Vec<String>,
    source: String,
    label: String,
}

fn form_type_assignment(
    hub: &Hub,
    enrichment: &[Enrichment],
    original_types: &TypeGroup,
    key: &str,
    images: &[&Artwork],
) -> Result<TypeAssignment, usize> {
    let mut matches: Vec<(Vec<String>, String, String)> = vec![];
    for art in images {
        for group in &hub.type_groups {
            if group.artwork_url.as_ref() == Some(&art.url)
                || (!group.label.is_empty() && group_key(group, &hub.name) == key)
                || (!matches!(group.section.as_str(), "Picture" | "Name")
                    && group_key(group, &hub.name) == key)
            {
                matches.push((
                    group.types.clone(),
                    hub.source_url.clone(),
                    format!("{} / {}", group.section, group.label),
                ));
            }
        }
        if let Some(types) = label_type(&art.label) {
            matches.push((types, hub.source_url.clone(), art.label.clone()));
        }
    }
    if key == "original" {
        matches.push((
            original_types.types.clone(),
            hub.source_url.clone(),
            format!("Picture / {}", original_types.label),
        ));
    }
    if matches.is_empty() {
        for extra in enrichment {
            for group in &extra.groups {
                let candidate = group_key(group, &hub.name);
                let shorter = key.strip_prefix("paldean ").unwrap_or(key);
                if !group.label.is_empty() && (candidate == key || candidate == shorter) {
                    matches.push((
                        group.types.clone(),
                        extra.source_url.clone(),
                        group.label.clone(),
                    ));
                }
            }
        }
    }
    if matches.is_empty() && !key.starts_with("mega") && !key.starts_with("gigantamax") {
        for extra in enrichment {
            let shared: Vec<_> = extra
                .groups
                .iter()
                .filter(|group| matches!(group.section.as_str(), "Name" | "Picture"))
                .filter(|group| !group_key(group, &hub.name).starts_with("mega"))
                .collect();
            if let Some(group) = shared.first().copied().filter(|_| shared.len() == 1)
                && (group.label.is_empty() || group.label == hub.name)
                && extra.named_forms.contains(key)
            {
                matches.push((
                    group.types.clone(),
                    extra.source_url.clone(),
                    format!("Shared type panel for independently named form: {key}"),
                ));
            }
        }
    }
    let distinct: BTreeSet<_> = matches
        .iter()
        .map(|assignment| assignment.0.clone())
        .collect();
    if distinct.len() != 1 {
        return Err(distinct.len());
    }
    let Some((types, source, label)) = matches.into_iter().next() else {
        return Err(0);
    };
    Ok(TypeAssignment {
        types,
        source,
        label,
    })
}

struct ClassifiedArtwork<'a> {
    grouped: BTreeMap<String, Vec<&'a Artwork>>,
    resolved_labels: BTreeMap<String, String>,
    problems: Vec<String>,
    excluded_illustrations: Vec<ExcludedIllustration>,
}

fn classify_artwork<'a>(
    hub: &'a Hub,
    enrichment: &[Enrichment],
    original_aliases: &BTreeSet<String>,
) -> Result<ClassifiedArtwork<'a>, SourceError> {
    let mut grouped: BTreeMap<String, Vec<&Artwork>> = BTreeMap::new();
    let mut resolved_labels = BTreeMap::new();
    let mut problems = vec![];
    let mut excluded_illustrations = vec![];
    for art in &hub.artwork {
        match classify_artwork_item(hub, enrichment, art, original_aliases)? {
            ArtworkDecision::Unlabelled => {
                problems.push(format!("unlabelled illustration {}", art.url));
            }
            ArtworkDecision::Excluded(excluded) => excluded_illustrations.push(excluded),
            ArtworkDecision::Included { key, label } => {
                resolved_labels.insert(art.url.clone(), label);
                grouped.entry(key).or_default().push(art);
            }
        }
    }
    Ok(ClassifiedArtwork {
        grouped,
        resolved_labels,
        problems,
        excluded_illustrations,
    })
}

enum ArtworkDecision {
    Unlabelled,
    Excluded(ExcludedIllustration),
    Included { key: String, label: String },
}

fn classify_artwork_item(
    hub: &Hub,
    enrichment: &[Enrichment],
    art: &Artwork,
    original_aliases: &BTreeSet<String>,
) -> Result<ArtworkDecision, SourceError> {
    let unavailable_label = match (hub.id, art.url.as_str()) {
        (1012, "https://www.serebii.net/pokemon/art/1012-m.png") => Some("Artisan Form"),
        (1013, "https://www.serebii.net/pokemon/art/1013-m.png") => Some("Masterpiece Form"),
        _ => None,
    };
    let label = if art.label.is_empty() || art.section == "Evolutionary Chain" {
        enrichment
            .iter()
            .find_map(|entry| entry.artwork_labels.get(&art.url))
            .map(String::as_str)
            .or(unavailable_label)
            .unwrap_or(&art.label)
    } else {
        &art.label
    };
    if label.is_empty() {
        return Ok(ArtworkDecision::Unlabelled);
    }
    let mut candidate = form_key(label, &hub.name);
    if matches!(
        (hub.id, candidate.as_str(), art.url.as_str()),
        (
            25,
            "captain",
            "https://www.serebii.net/pokemon/art/025-captain.png"
        ) | (
            25,
            "mr windychu",
            "https://www.serebii.net/pokemon/art/025-mrwindychu.png"
        ) | (
            25,
            "ms waveychu",
            "https://www.serebii.net/pokemon/art/025-mswaveychu.png"
        ) | (
            718,
            "cell",
            "https://www.serebii.net/pokemon/art/718-cell.png"
        ) | (
            718,
            "core",
            "https://www.serebii.net/pokemon/art/718-core.png"
        )
    ) {
        return Ok(ArtworkDecision::Excluded(ExcludedIllustration {
            label: label.to_owned(), artwork_url: art.url.clone(),
            reason: ExclusionReason::ApprovedUnsupportedTypeEvidence,
            source_context: "No supported named per-form type join in the inspected source-linked evidence; exact exclusion approved in D32 on 2026-11-07. Source labels and evidence: docs/logs/20261107-serebii-correctness-unresolved.json".into(),
        }));
    }
    if hub.name == "Rotom"
        && ["phone", "drone", "rotomi", "phone paldean ver"].contains(&candidate.as_str())
    {
        let context = hub.non_battle_device_context.as_ref().ok_or_else(|| {
            error("approved device exclusion lacks its source non-battle description")
        })?;
        return Ok(ArtworkDecision::Excluded(ExcludedIllustration {
            label: label.to_owned(),
            artwork_url: art.url.clone(),
            reason: ExclusionReason::NonBattleDeviceWithoutTypes,
            source_context: context.clone(),
        }));
    }
    if matches!(
        (hub.id, candidate.as_str(), art.url.as_str()),
        (
            1012,
            "artisan",
            "https://www.serebii.net/pokemon/art/1012-m.png"
        ) | (
            1013,
            "masterpiece",
            "https://www.serebii.net/pokemon/art/1013-m.png"
        )
    ) {
        return Ok(ArtworkDecision::Excluded(ExcludedIllustration {
            label: label.to_owned(), artwork_url: art.url.clone(),
            reason: ExclusionReason::ApprovedUnavailableIllustration,
            source_context: "Source-linked illustration returned HTTP 404 on 2026-11-07T00:53:46–47Z; exact exclusion approved in D31. Response evidence: docs/logs/20261107-serebii-catalogue-source-verification.json".into(),
        }));
    }
    candidate = gigantamax_key(hub, enrichment, art, candidate);
    let verified_alias = supported_original_alias(hub, art, label)?;
    let key = if art.url == hub.original_artwork
        || original_aliases.contains(&candidate)
        || verified_alias
    {
        "original".into()
    } else {
        candidate
    };
    Ok(ArtworkDecision::Included {
        key,
        label: label.to_owned(),
    })
}

fn gigantamax_key(
    hub: &Hub,
    enrichment: &[Enrichment],
    art: &Artwork,
    candidate: String,
) -> String {
    if candidate != "gigantamax" {
        return candidate;
    }
    let variants: BTreeSet<_> = hub
        .type_groups
        .iter()
        .filter(|group| group.section.starts_with("Gigantamax "))
        .map(|group| group.types.clone())
        .collect();
    let Some(bound) = hub
        .type_groups
        .iter()
        .find(|group| group.artwork_url.as_ref() == Some(&art.url))
    else {
        return candidate;
    };
    if variants.len() <= 1 {
        return candidate;
    }
    let labels: BTreeSet<_> = enrichment
        .iter()
        .flat_map(|entry| &entry.groups)
        .filter(|group| {
            group.section == "Name" && !group.label.is_empty() && group.types == bound.types
        })
        .map(|group| form_key(&group.label, &hub.name))
        .collect();
    labels
        .iter()
        .next()
        .filter(|_| labels.len() == 1)
        .map_or(candidate, |label| format!("gigantamax {label}"))
}

/// Require independent successful image validation for every selected form.
/// The caller supplies URLs whose actual bytes passed its PNG checks, not merely discovered links.
///
/// # Errors
///
/// Returns an error when any selected artwork URL is absent from the verified PNG set.
pub fn validate_artwork_references(
    catalogue: &[Species],
    verified_png_urls: &BTreeSet<String>,
) -> Result<(), SourceError> {
    let missing: Vec<_> = catalogue
        .iter()
        .flat_map(|species| {
            species
                .forms
                .iter()
                .filter(|form| !verified_png_urls.contains(&form.artwork_url))
                .map(|form| format!("{} {}: {}", species.id, form.key, form.artwork_url))
        })
        .collect();
    if !missing.is_empty() {
        return Err(error(format!(
            "unverified selected illustrations: {}",
            missing.join("; ")
        )));
    }
    Ok(())
}
