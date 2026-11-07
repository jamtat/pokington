//! Versioned installed metadata. Source reconciliation remains in `serebii`.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::serebii;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogueError {
    UnsupportedSchema(u32),
    Invalid(String),
}

impl fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(f, "unsupported catalogue schema {version}"),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for CatalogueError {}

fn invalid(message: impl Into<String>) -> CatalogueError {
    CatalogueError::Invalid(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PokemonType {
    Normal,
    Fire,
    Water,
    Electric,
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
}

/// A relative, immutable file identity, never a source URL or user-supplied path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ArtworkRef(String);

impl ArtworkRef {
    pub fn new(value: String) -> Result<Self, CatalogueError> {
        let Some((generation, number)) = value
            .strip_prefix("artwork/")
            .and_then(|s| s.strip_suffix(".png"))
            .and_then(|s| s.split_once('-'))
        else {
            return Err(invalid(format!("unsafe local artwork reference: {value}")));
        };
        if generation.len() != 32
            || !generation
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || number
                .parse::<u64>()
                .ok()
                .is_none_or(|n| n == 0 || n.to_string() != number)
        {
            return Err(invalid(format!("unsafe local artwork reference: {value}")));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ArtworkRef {
    type Error = CatalogueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ArtworkRef> for String {
    fn from(value: ArtworkRef) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub index_url: String,
    pub min_id: u32,
    pub max_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub schema_version: u32,
    pub source: Source,
    pub species: Vec<Species>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Species {
    pub id: u32,
    pub name: String,
    pub source_url: String,
    pub original_form: String,
    pub forms: Vec<Form>,
    pub excluded_illustrations: Vec<serebii::ExcludedIllustration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub key: String,
    pub display_name: String,
    pub source_id: String,
    pub source_labels: Vec<String>,
    pub primary_type: PokemonType,
    pub secondary_type: Option<PokemonType>,
    pub artwork_url: String,
    pub artwork_references: Vec<String>,
    pub type_source: String,
    pub type_label: String,
    pub local_artwork: ArtworkRef,
}

fn name_valid(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

fn source_url(value: &str) -> bool {
    value.strip_prefix(serebii::ORIGIN).is_some_and(|path| {
        path.starts_with('/')
            && !path.starts_with("//")
            && !path.contains("..")
            && path.bytes().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'\'' | b':')
            })
    })
}

fn artwork_url(value: &str) -> bool {
    source_url(value)
        && value
            .strip_prefix("https://www.serebii.net/pokemon/art/")
            .and_then(|s| s.strip_suffix(".png"))
            .is_some_and(|s| !s.is_empty() && !s.contains('/'))
}

impl Catalogue {
    /// Preserve the reviewed source identities; do not normalize names or infer new aliases here.
    pub fn from_serebii(
        index: &serebii::Index,
        species: Vec<serebii::Species>,
        generation: &str,
    ) -> Result<Self, CatalogueError> {
        if species.len() != index.species.len() {
            return Err(invalid("reconciled species do not cover the source index"));
        }
        let mut records = Vec::with_capacity(species.len());
        let mut ordinal = 0_u64;
        for record in species {
            let link = index.species.iter().find(|s| s.id == record.id);
            if !link.is_some_and(|link| {
                link.name == record.name
                    && record.source_url == format!("{}{}", serebii::ORIGIN, link.detail_path)
            }) {
                return Err(invalid(format!(
                    "species {} differs from source index",
                    record.id
                )));
            }
            let mut forms = Vec::with_capacity(record.forms.len());
            for form in record.forms {
                if !(1..=2).contains(&form.types.len()) {
                    return Err(invalid(format!(
                        "{} {} requires one or two types",
                        record.id, form.key
                    )));
                }
                let parse_type = |kind: &String| {
                    serde_json::from_value(serde_json::Value::String(kind.clone()))
                        .map_err(|_| invalid(format!("unknown Pokémon type: {kind}")))
                };
                ordinal += 1;
                forms.push(Form {
                    key: form.key,
                    display_name: form.display_name,
                    source_id: form.source_id,
                    source_labels: form.source_labels,
                    primary_type: parse_type(&form.types[0])?,
                    secondary_type: form.types.get(1).map(parse_type).transpose()?,
                    artwork_url: form.artwork_url,
                    artwork_references: form.artwork_references,
                    type_source: form.type_source,
                    type_label: form.type_label,
                    local_artwork: ArtworkRef::new(format!("artwork/{generation}-{ordinal}.png"))?,
                });
            }
            forms.sort_by(|a, b| a.key.cmp(&b.key));
            records.push(Species {
                id: record.id,
                name: record.name,
                source_url: record.source_url,
                original_form: record.original_form,
                forms,
                excluded_illustrations: record.excluded_illustrations,
            });
        }
        records.sort_by_key(|s| s.id);
        let catalogue = Self {
            schema_version: SCHEMA_VERSION,
            source: Source {
                index_url: serebii::INDEX_URL.into(),
                min_id: index.min,
                max_id: index.max,
            },
            species: records,
        };
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, CatalogueError> {
        #[derive(Deserialize)]
        struct Header {
            schema_version: u32,
        }
        let header: Header = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        if header.schema_version != SCHEMA_VERSION {
            return Err(CatalogueError::UnsupportedSchema(header.schema_version));
        }
        let catalogue: Self = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<(), CatalogueError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(CatalogueError::UnsupportedSchema(self.schema_version));
        }
        if self.source.index_url != serebii::INDEX_URL
            || self.source.min_id == 0
            || self.source.max_id < self.source.min_id
            || self.species.len() as u64
                != u64::from(self.source.max_id) - u64::from(self.source.min_id) + 1
        {
            return Err(invalid("invalid or incomplete source index coverage"));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut local_refs = BTreeSet::new();
        for species in &self.species {
            if !(self.source.min_id..=self.source.max_id).contains(&species.id)
                || !ids.insert(species.id)
                || !name_valid(&species.name)
                || !names.insert(species.name.to_lowercase())
                || !source_url(&species.source_url)
            {
                return Err(invalid(format!(
                    "invalid or duplicate species {}",
                    species.id
                )));
            }
            let mut keys = BTreeSet::new();
            let mut source_ids = BTreeSet::new();
            if species.original_form != "original"
                || !species.forms.iter().any(|f| f.key == species.original_form)
            {
                return Err(invalid(format!(
                    "species {} has no original form",
                    species.id
                )));
            }
            for form in &species.forms {
                if !name_valid(&form.key)
                    || form.key
                        != form
                            .key
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase()
                    || !keys.insert(&form.key)
                    || !name_valid(&form.display_name)
                    || !artwork_url(&form.artwork_url)
                    || form.source_id != form.artwork_url
                    || !source_ids.insert(&form.source_id)
                    || form.secondary_type == Some(form.primary_type)
                    || form.source_labels.is_empty()
                    || form.source_labels.iter().any(|s| !name_valid(s))
                    || form.artwork_references.is_empty()
                    || form.artwork_references.iter().any(|s| !artwork_url(s))
                    || !form.artwork_references.contains(&form.artwork_url)
                    || !source_url(&form.type_source)
                    || form.type_label.trim().is_empty()
                    || !local_refs.insert(&form.local_artwork)
                {
                    return Err(invalid(format!("invalid form {} {}", species.id, form.key)));
                }
            }
            for excluded in &species.excluded_illustrations {
                if !name_valid(&excluded.label)
                    || !artwork_url(&excluded.artwork_url)
                    || excluded.source_context.trim().is_empty()
                    || species
                        .forms
                        .iter()
                        .any(|f| f.artwork_references.contains(&excluded.artwork_url))
                {
                    return Err(invalid(format!(
                        "invalid exclusion for species {}",
                        species.id
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn forms(&self) -> impl Iterator<Item = &Form> {
        self.species.iter().flat_map(|s| &s.forms)
    }

    pub(crate) fn same_source(&self, other: &Self) -> bool {
        let mut candidate = self.clone();
        for (left, right) in candidate
            .species
            .iter_mut()
            .flat_map(|s| &mut s.forms)
            .zip(other.forms())
        {
            left.local_artwork = right.local_artwork.clone();
        }
        candidate == *other
    }
}
