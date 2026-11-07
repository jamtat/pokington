//! Parser for the observed official English portal index, not a completeness guarantee.
//!
//! The source omits known official Mega forms. Its form labels can also collide.
//! Keep this separate from production bootstrap until those gates are resolved.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError(pub String);

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SourceError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortalEntry {
    pub zukan_id: String,
    pub zukan_sub_id: u32,
    pub pokemon_name: String,
    pub pokemon_sub_name: String,
    pub file_name: String,
    pub pokemon_type_id: String,
    pub pokemon_type_name: String,
}

impl PortalEntry {
    /// The exact route identifier constructed by the official site's card component.
    pub fn source_id(&self) -> String {
        match self.zukan_sub_id {
            0 => self.zukan_id.clone(),
            id => format!("{}_{id}", self.zukan_id),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchItems {
    search: String,
    #[serde(rename = "searchType")]
    search_type: String,
    #[serde(rename = "pokemonMinId")]
    min_id: u32,
    #[serde(rename = "pokemonMaxId")]
    max_id: u32,
    #[serde(rename = "type")]
    types: Vec<PortalType>,
}

#[derive(Debug, Deserialize)]
struct PortalType {
    pokemon_type_id: String,
    pokemon_type_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalIndex {
    variant: String,
    #[serde(rename = "initialPokemons")]
    pub entries: Vec<PortalEntry>,
    #[serde(rename = "searchItems")]
    search_items: SearchItems,
    #[serde(rename = "imageBaseUrl")]
    image_base_url: String,
}

impl PortalIndex {
    pub fn species_count(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| &entry.zukan_id)
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn bounds(&self) -> (u32, u32) {
        (self.search_items.min_id, self.search_items.max_id)
    }

    fn validate(&self) -> Result<(), SourceError> {
        let fail = |message: &str| SourceError(message.into());
        let (min, max) = self.bounds();
        if self.variant != "top"
            || !self.search_items.search.is_empty()
            || !self.search_items.search_type.is_empty()
        {
            return Err(fail("filtered index is not a full discovery response"));
        }
        if min == 0 || min > max || self.entries.is_empty() {
            return Err(fail("empty index or invalid source bounds"));
        }
        // The observed source uses same-origin relative artwork paths. Fail on drift.
        if !self.image_base_url.is_empty() {
            return Err(fail("unsupported imageBaseUrl"));
        }
        let mut types = BTreeMap::new();
        for kind in &self.search_items.types {
            if kind.pokemon_type_id.trim().is_empty()
                || kind.pokemon_type_name.trim().is_empty()
                || types
                    .insert(&kind.pokemon_type_id, &kind.pokemon_type_name)
                    .is_some()
            {
                return Err(fail("invalid or duplicate type definition"));
            }
        }
        let mut identities = BTreeSet::new();
        let mut originals = BTreeSet::new();
        for entry in &self.entries {
            let number = entry
                .zukan_id
                .parse::<u32>()
                .map_err(|_| fail("invalid Pokédex number"))?;
            if number < min || number > max || entry.zukan_id != format!("{number:04}") {
                return Err(fail(
                    "Pokédex number outside source bounds or noncanonical ID",
                ));
            }
            if !identities.insert((number, entry.zukan_sub_id)) {
                return Err(fail("duplicate source identity"));
            }
            if entry.zukan_sub_id == 0 {
                originals.insert(number);
            }
            if entry.pokemon_name.trim().is_empty() {
                return Err(fail("missing Pokémon name"));
            }
            let filename = entry
                .file_name
                .strip_prefix("/images/pokedex/")
                .and_then(|name| name.strip_suffix(".png"))
                .ok_or_else(|| fail("invalid artwork reference"))?;
            if filename.is_empty()
                || !filename
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                return Err(fail("invalid artwork filename"));
            }
            let ids: Vec<_> = entry.pokemon_type_id.split(',').collect();
            let names: Vec<_> = entry.pokemon_type_name.split(',').collect();
            if !(1..=2).contains(&ids.len())
                || ids.len() != names.len()
                || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
                || ids.iter().zip(names).any(|(id, name)| {
                    types.get(&id.to_string()).map(|value| value.as_str()) != Some(name)
                })
            {
                return Err(fail("invalid per-form type reference"));
            }
        }
        // Bounds come from the source, not a compiled Pokémon count. Compare counts
        // after range validation to avoid allocating an attacker-controlled range.
        if u64::try_from(originals.len()).ok() != Some(u64::from(max) - u64::from(min) + 1) {
            return Err(fail(
                "partial index: missing original species within source bounds",
            ));
        }
        Ok(())
    }

    /// Candidate descriptive keys. Ambiguity is an error, never a numeric suffix.
    /// This currently fails for live Tauros's three identically labelled forms.
    pub fn form_keys(&self) -> Result<BTreeMap<String, String>, SourceError> {
        let originals: BTreeMap<_, _> = self
            .entries
            .iter()
            .filter(|entry| entry.zukan_sub_id == 0)
            .map(|entry| (&entry.zukan_id, entry.pokemon_name.as_str()))
            .collect();
        let mut seen = BTreeSet::new();
        let mut keys = BTreeMap::new();
        for entry in &self.entries {
            let original = originals.get(&entry.zukan_id).ok_or_else(|| {
                SourceError(format!("missing original for {}", entry.source_id()))
            })?;
            let label = if !entry.pokemon_sub_name.trim().is_empty() {
                entry.pokemon_sub_name.clone()
            } else if entry.zukan_sub_id == 0 {
                "original".into()
            } else {
                entry.pokemon_name.replacen(original, "", 1)
            };
            let key = label
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if key.is_empty() || !seen.insert((&entry.zukan_id, key.clone())) {
                return Err(SourceError(format!(
                    "ambiguous descriptive form key for {}: {key:?}",
                    entry.source_id()
                )));
            }
            keys.insert(entry.source_id(), key);
        }
        Ok(keys)
    }
}

/// Extract the one unfiltered index from observed Next.js inline Flight chunks.
/// Handles records split across chunks, without executing any source JavaScript.
/// Structural validity does not prove the source includes every official form.
pub fn parse_portal_index(html: &str) -> Result<PortalIndex, SourceError> {
    if !html.trim_end().ends_with("</html>") {
        return Err(SourceError("truncated HTML document".into()));
    }
    let document = Html::parse_document(html);
    let html_selector = Selector::parse("html").expect("constant selector");
    let language = document
        .select(&html_selector)
        .next()
        .and_then(|element| element.value().attr("lang"))
        .unwrap_or_default();
    if !matches!(language, "en-SG" | "en-PH" | "en-IN") {
        return Err(SourceError(
            "not an observed English portal document".into(),
        ));
    }
    let scripts = Selector::parse("script").expect("constant selector");
    let mut flight = String::new();
    for script in document.select(&scripts) {
        let text = script.inner_html();
        if let Some(push) = text.strip_prefix("self.__next_f.push(") {
            let json = push
                .strip_suffix(')')
                .ok_or_else(|| SourceError("malformed Flight push".into()))?;
            let (kind, payload): (u32, String) = serde_json::from_str(json)
                .map_err(|error| SourceError(format!("invalid Flight chunk: {error}")))?;
            if kind != 1 {
                return Err(SourceError("unsupported Flight chunk kind".into()));
            }
            flight.push_str(&payload);
        }
    }
    let mut candidates = Vec::new();
    for line in flight.lines() {
        if let Some((_, json)) = line.split_once(':')
            && (json.starts_with('[') || json.starts_with('{'))
        {
            let value = serde_json::from_str(json)
                .map_err(|error| SourceError(format!("invalid Flight record: {error}")))?;
            find_indices(value, &mut candidates);
        }
    }
    if candidates.len() != 1 {
        return Err(SourceError(format!(
            "expected one index, found {} (anti-bot, partial response, or source drift)",
            candidates.len()
        )));
    }
    let index: PortalIndex = serde_json::from_value(candidates.pop().expect("one candidate"))
        .map_err(|error| SourceError(format!("invalid index fields: {error}")))?;
    index.validate()?;
    Ok(index)
}

fn find_indices(value: Value, indices: &mut Vec<Value>) {
    match value {
        Value::Object(object) if object.contains_key("initialPokemons") => {
            indices.push(Value::Object(object))
        }
        Value::Object(object) => {
            for value in object.into_values() {
                find_indices(value, indices);
            }
        }
        Value::Array(array) => {
            for value in array {
                find_indices(value, indices);
            }
        }
        _ => {}
    }
}
