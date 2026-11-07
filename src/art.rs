//! Artwork lookup and shared image preparation.

use std::{fmt, io, str::FromStr};

use clap::ValueEnum;
use image::{RgbaImage, imageops::FilterType};
use rand::{Rng, seq::IndexedRandom};

use crate::{
    catalogue::{Catalogue, Form, Species},
    inline_image::crop_visible,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    #[default]
    Auto,
    Img,
    Blocks,
    Ascii,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum Color {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Width {
    #[default]
    Fit,
    Columns(u32),
    Original,
}

impl FromStr for Width {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "original" {
            return Ok(Self::Original);
        }
        value
            .parse::<u32>()
            .ok()
            .filter(|n| *n != 0)
            .map(Self::Columns)
            .ok_or_else(|| "width must be a positive integer or original".into())
    }
}

#[derive(Debug)]
pub enum ArtError {
    UnknownSpecies(String),
    UnknownForm {
        species: String,
        requested: String,
        available: Vec<String>,
    },
    NoMatchingForm(String),
    EmptyCatalogue,
    Image(io::Error),
}

impl fmt::Display for ArtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSpecies(name) => write!(f, "unknown Pokémon: {name}"),
            Self::UnknownForm {
                species,
                requested,
                available,
            } => write!(
                f,
                "unknown form {requested:?} for {species}; available forms: {}",
                available.join(", ")
            ),
            Self::NoMatchingForm(form) => write!(f, "no artwork matches form {form:?}"),
            Self::EmptyCatalogue => f.write_str("no eligible artwork remains"),
            Self::Image(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ArtError {}
impl From<io::Error> for ArtError {
    fn from(value: io::Error) -> Self {
        Self::Image(value)
    }
}

/// Resolve named/default forms or uniformly sample matching artwork entries.
///
/// # Errors
/// Returns a lookup error for unknown names/forms or an empty selection.
pub fn select<'a>(
    catalogue: &'a Catalogue,
    name: Option<&str>,
    form: Option<&str>,
    rng: &mut impl Rng,
) -> Result<(&'a Species, &'a Form), ArtError> {
    let form = form.map(str::to_lowercase);
    if let Some(name) = name {
        let lowered = name.to_lowercase();
        let id = name.parse::<u32>().ok();
        let species = catalogue
            .species
            .iter()
            .find(|s| Some(s.id) == id || s.name.to_lowercase() == lowered)
            .ok_or_else(|| ArtError::UnknownSpecies(name.into()))?;
        let key = form.as_deref().unwrap_or(&species.original_form);
        let selected =
            species
                .forms
                .iter()
                .find(|f| f.key == key)
                .ok_or_else(|| ArtError::UnknownForm {
                    species: species.name.clone(),
                    requested: key.into(),
                    available: species.forms.iter().map(|f| f.key.clone()).collect(),
                })?;
        return Ok((species, selected));
    }
    let candidates: Vec<_> = catalogue
        .species
        .iter()
        .flat_map(|s| s.forms.iter().map(move |f| (s, f)))
        .filter(|(_, f)| form.as_ref().is_none_or(|key| f.key == *key))
        .collect();
    candidates
        .choose(rng)
        .copied()
        .ok_or_else(|| form.map_or(ArtError::EmptyCatalogue, ArtError::NoMatchingForm))
}

/// Label artwork using its form display name without repeating the species name.
#[must_use]
pub fn display_name(species: &Species, form: &Form) -> String {
    if form
        .display_name
        .to_lowercase()
        .contains(&species.name.to_lowercase())
    {
        return form.display_name.clone();
    }
    let description = form
        .display_name
        .split_whitespace()
        .filter(|word| !word.eq_ignore_ascii_case("form"))
        .collect::<Vec<_>>()
        .join(" ");
    if description.is_empty() {
        species.name.clone()
    } else {
        format!("{} ({description})", species.name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub columns: u32,
    pub rows: u32,
    pub cell_width: u32,
    pub cell_height: u32,
}
impl Default for Geometry {
    fn default() -> Self {
        Self {
            columns: 80,
            rows: 24,
            cell_width: 1,
            cell_height: 2,
        }
    }
}

/// Crop and size a text image. Blocks use two sample rows per character row.
///
/// # Errors
/// Rejects transparent images, invalid geometry, or excessively large output.
pub fn prepare(
    image: &RgbaImage,
    width: Width,
    geometry: Geometry,
    blocks: bool,
) -> io::Result<RgbaImage> {
    let (cropped, _) = crop_visible(image)?;
    if width == Width::Original {
        return Ok(cropped);
    }
    let factor = if blocks { 2_u64 } else { 1 };
    if geometry.columns == 0
        || geometry.rows == 0
        || geometry.cell_width == 0
        || geometry.cell_height == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "zero terminal dimensions",
        ));
    }
    let columns = match width {
        Width::Columns(n) => n,
        _ => geometry.columns,
    };
    let numerator = u64::from(cropped.height())
        .saturating_mul(u64::from(geometry.cell_width))
        .saturating_mul(factor);
    let denominator = u64::from(cropped.width()).saturating_mul(u64::from(geometry.cell_height));
    let mut w = u64::from(columns);
    let mut h = rounded_ratio(w, numerator, denominator);
    if width == Width::Fit {
        let max_height = u64::from(geometry.rows).saturating_mul(factor);
        if h > max_height {
            h = max_height;
            w = rounded_ratio(h, denominator, numerator).min(w);
        }
    }
    resize(
        &cropped,
        u32::try_from(w).map_err(io::Error::other)?,
        u32::try_from(h).map_err(io::Error::other)?,
    )
}

fn rounded_ratio(value: u64, numerator: u64, denominator: u64) -> u64 {
    value
        .saturating_mul(numerator)
        .saturating_add(denominator / 2)
        .checked_div(denominator)
        .unwrap_or(0)
        .max(1)
}

/// Resize premultiplied RGBA to prevent invisible RGB values bleeding into edges.
///
/// # Errors
/// Rejects empty or oversized output dimensions.
pub fn resize(image: &RgbaImage, width: u32, height: u32) -> io::Result<RgbaImage> {
    if width == 0
        || height == 0
        || u64::from(width).saturating_mul(u64::from(height)) > 16 * 1024 * 1024
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "render dimensions must contain 1–16777216 pixels",
        ));
    }
    if image.dimensions() == (width, height) {
        return Ok(image.clone());
    }
    let mut premultiplied = image.clone();
    for pixel in premultiplied.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in pixel.0.iter_mut().take(3) {
            *channel = u8::try_from(u16::from(*channel).saturating_mul(alpha) / 255).unwrap_or(255);
        }
    }
    let filter = if width < image.width() || height < image.height() {
        FilterType::Lanczos3
    } else {
        FilterType::Triangle
    };
    let mut resized = image::imageops::resize(&premultiplied, width, height, filter);
    for pixel in resized.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in pixel.0.iter_mut().take(3) {
            *channel = u8::try_from(
                u16::from(*channel)
                    .saturating_mul(255)
                    .checked_div(alpha)
                    .unwrap_or(0),
            )
            .unwrap_or(255);
        }
    }
    Ok(resized)
}
