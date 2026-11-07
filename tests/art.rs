#![cfg(unix)]

use image::{Rgba, RgbaImage};
use pokington::{
    art::{self, ArtError, Geometry, Width},
    catalogue::Catalogue,
    inline_image::crop_visible,
    serebii,
    text_art::{self, Depth},
};
use rand::{SeedableRng, rngs::StdRng};
use std::collections::{BTreeMap, BTreeSet};

fn catalogue() -> Result<Catalogue, Box<dyn std::error::Error>> {
    let index = serebii::parse_index(include_str!("fixtures/serebii-index.html"))?;
    let hub = serebii::parse_hub(
        include_str!("fixtures/serebii-charizard.html"),
        index.species.first().ok_or("missing species")?,
    )?;
    let mut catalogue = Catalogue::from_serebii(
        &index,
        vec![serebii::reconcile(&hub, &[])?],
        "00000000000000000000000000000001",
    )?;
    let mut other = catalogue.species.first().ok_or("missing species")?.clone();
    other.id = 2;
    other.name = "Flabébé".into();
    other.forms.retain(|f| f.key == "original");
    catalogue.species.push(other);
    Ok(catalogue)
}

#[test]
fn artwork_labels_use_form_display_names_without_repeating_species() {
    let catalogue = catalogue().unwrap();
    let mut species = catalogue.species.first().unwrap().clone();
    let mut form = species.forms.first().unwrap().clone();
    for (name, display_name, expected) in [
        ("Dragonite", "Mega Dragonite", "Mega Dragonite"),
        ("Dragonite", "Mega dRaGoNiTe", "Mega dRaGoNiTe"),
        ("Flabébé", "Mega FLABÉBÉ", "Mega FLABÉBÉ"),
        ("Sandshrew", "Alola Form", "Sandshrew (Alola)"),
        ("Sandshrew", "Alola fOrM", "Sandshrew (Alola)"),
        ("Deoxys", "Attack Forme", "Deoxys (Attack Forme)"),
        ("Mew", "Original", "Mew (Original)"),
        ("Mew", "Form", "Mew"),
        ("Castform", "Rainy Form", "Castform (Rainy)"),
        ("Castform", "Castform Rainy Form", "Castform Rainy Form"),
    ] {
        species.name = name.into();
        form.display_name = display_name.into();
        assert_eq!(
            art::display_name(&species, &form),
            expected,
            "{name}: {display_name}"
        );
    }
}

#[test]
fn named_lookup_and_random_filter_use_identical_case_insensitive_keys() {
    let catalogue = catalogue().unwrap();
    let species = catalogue.species.first().unwrap();
    let mut rng = StdRng::seed_from_u64(3);
    for name in [species.name.to_uppercase(), species.id.to_string()] {
        assert_eq!(
            art::select(&catalogue, Some(&name), None, &mut rng)
                .unwrap()
                .1
                .key,
            "original"
        );
        for form in &species.forms {
            let named = art::select(
                &catalogue,
                Some(&name),
                Some(&form.key.to_uppercase()),
                &mut rng,
            )
            .unwrap();
            let random =
                art::select(&catalogue, None, Some(&form.key.to_uppercase()), &mut rng).unwrap();
            assert_eq!(named.1.key, random.1.key);
        }
    }
    assert_eq!(
        art::select(&catalogue, Some("FLABÉBÉ"), None, &mut rng)
            .unwrap()
            .0
            .id,
        2
    );
    assert!(matches!(
        art::select(&catalogue, Some("missing"), None, &mut rng),
        Err(ArtError::UnknownSpecies(_))
    ));
    let Err(ArtError::UnknownForm { available, .. }) =
        art::select(&catalogue, Some("Charizard"), Some("missing"), &mut rng)
    else {
        panic!("expected form error")
    };
    assert_eq!(
        available.into_iter().collect::<BTreeSet<_>>(),
        species.forms.iter().map(|f| f.key.clone()).collect()
    );
    assert!(matches!(
        art::select(&catalogue, None, Some("missing"), &mut rng),
        Err(ArtError::NoMatchingForm(_))
    ));
}

#[test]
fn random_sampling_weights_entries_instead_of_species() {
    let catalogue = catalogue().unwrap();
    let expected = catalogue.forms().count();
    let mut rng = StdRng::seed_from_u64(23);
    let mut counts = BTreeMap::<(u32, String), usize>::new();
    for _ in 0..10000 {
        let (species, form) = art::select(&catalogue, None, None, &mut rng).unwrap();
        *counts.entry((species.id, form.key.clone())).or_insert(0) += 1;
    }
    assert_eq!(counts.len(), expected);
    let mean = 10000 / expected;
    assert!(counts.values().all(|n| n.abs_diff(mean) < mean / 4));
}

#[test]
fn crop_retains_partial_alpha_and_original_pixels_for_both_text_modes() {
    let mut image = RgbaImage::new(8, 9);
    image.put_pixel(2, 3, Rgba([100, 50, 25, 1]));
    image.put_pixel(5, 7, Rgba([5, 6, 7, 255]));
    let (crop, bounds) = crop_visible(&image).unwrap();
    assert_eq!(
        (bounds.left, bounds.top, bounds.width, bounds.height),
        (2, 3, 4, 5)
    );
    for blocks in [false, true] {
        assert_eq!(
            art::prepare(&image, Width::Original, Geometry::default(), blocks).unwrap(),
            crop
        );
    }
    assert!(
        art::prepare(
            &RgbaImage::new(2, 2),
            Width::Fit,
            Geometry::default(),
            false
        )
        .unwrap_err()
        .to_string()
        .contains("transparent")
    );
}

#[test]
fn sizing_respects_cell_aspect_fit_height_and_explicit_overflow() {
    let image = RgbaImage::from_pixel(100, 100, Rgba([255; 4]));
    let geometry = Geometry {
        columns: 80,
        rows: 10,
        cell_width: 8,
        cell_height: 16,
    };
    for (blocks, expected) in [(false, (20, 10)), (true, (20, 20))] {
        assert_eq!(
            art::prepare(&image, Width::Fit, geometry, blocks)
                .unwrap()
                .dimensions(),
            expected
        );
    }
    assert_eq!(
        art::prepare(&image, Width::Columns(100), geometry, false)
            .unwrap()
            .dimensions(),
        (100, 50)
    );
    assert_eq!(
        art::prepare(&image, Width::Columns(100), geometry, true)
            .unwrap()
            .dimensions(),
        (100, 100)
    );
    let wide = RgbaImage::from_pixel(100, 10, Rgba([255; 4]));
    assert_eq!(
        art::prepare(&wide, Width::Fit, geometry, false)
            .unwrap()
            .dimensions(),
        (80, 4)
    );
    assert!(art::prepare(&image, Width::Columns(u32::MAX), geometry, true).is_err());
}

#[test]
fn resampling_ignores_hidden_color_and_preserves_solid_color() {
    let mut image = RgbaImage::from_pixel(2, 1, Rgba([255, 0, 0, 0]));
    image.put_pixel(0, 0, Rgba([0, 0, 255, 255]));
    let scaled = art::resize(&image, 1, 1).unwrap();
    let pixel = scaled.get_pixel(0, 0);
    assert_eq!(pixel[0], 0);
    assert!(pixel[2] >= 250 && (100..=155).contains(&pixel[3]));
    let solid = RgbaImage::from_pixel(2, 2, Rgba([30, 60, 90, 255]));
    assert!(
        art::resize(&solid, 7, 9)
            .unwrap()
            .pixels()
            .all(|p| *p == Rgba([30, 60, 90, 255]))
    );
}

#[test]
fn text_output_has_defined_structure_transparency_and_odd_row_handling() {
    let mut image = RgbaImage::new(3, 3);
    image.put_pixel(1, 0, Rgba([255; 4]));
    image.put_pixel(2, 1, Rgba([255; 4]));
    image.put_pixel(0, 2, Rgba([255; 4]));
    assert_eq!(
        text_art::render(&image, false, Depth::None),
        " @ \n  @\n@  \n"
    );
    let blocks = text_art::render(&image, true, Depth::None);
    assert_eq!(blocks.lines().count(), 2);
    assert!(blocks.lines().all(|line| line.chars().count() == 3));
    assert!(!blocks.contains('\x1b'));
    assert_eq!(blocks, " ▀▄\n▀  \n");
    for depth in [Depth::TrueColor, Depth::Ansi256, Depth::Ansi16] {
        let colored = text_art::render(&image, true, depth);
        assert!(colored.contains('▀') && colored.contains('▄'));
        assert!(colored.ends_with("\x1b[0m\n"));
    }
    let ascii = text_art::render(&image, false, Depth::TrueColor);
    assert!(ascii.contains("\x1b[38;2;255;255;255m@"));
}
