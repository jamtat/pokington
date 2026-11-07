use pokington::{
    catalogue::{ArtworkRef, Catalogue, CatalogueError, PokemonType},
    serebii::{self, Index},
};
use serde_json::{Value, json};

const GENERATION: &str = "0123456789abcdef0123456789abcdef";

fn catalogue() -> Catalogue {
    let index = serebii::parse_index(include_str!("fixtures/serebii-index.html")).unwrap();
    let hub = serebii::parse_hub(
        include_str!("fixtures/serebii-charizard.html"),
        &index.species[0],
    )
    .unwrap();
    Catalogue::from_serebii(
        &index,
        vec![serebii::reconcile(&hub, &[]).unwrap()],
        GENERATION,
    )
    .unwrap()
}

#[test]
fn json_roundtrip_preserves_distinct_mega_types_and_alias_provenance() {
    let catalogue = catalogue();
    let encoded = serde_json::to_vec(&catalogue).unwrap();
    assert_eq!(Catalogue::from_json(&encoded).unwrap(), catalogue);
    let forms = &catalogue.species[0].forms;
    let x = forms.iter().find(|f| f.key == "mega x").unwrap();
    let y = forms.iter().find(|f| f.key == "mega y").unwrap();
    assert_eq!(x.primary_type, PokemonType::Fire);
    assert_eq!(x.secondary_type, Some(PokemonType::Dragon));
    assert_eq!(y.secondary_type, Some(PokemonType::Flying));
    assert_ne!(x.local_artwork, y.local_artwork);
    let giant = forms.iter().find(|f| f.key == "gigantamax").unwrap();
    assert_eq!(giant.artwork_references.len(), 2);
    assert!(!giant.source_labels.is_empty());
    assert!(!giant.type_source.is_empty());
    assert_eq!(catalogue.species[0].original_form, "original");
}

#[test]
fn complete_reviewed_source_snapshot_converts_without_losing_forms_or_exclusions() {
    let species: Vec<serebii::Species> = serde_json::from_str(include_str!(
        "../docs/logs/20261107-serebii-catalogue-catalogue.json"
    ))
    .unwrap();
    let index = Index {
        min: species.iter().map(|s| s.id).min().unwrap(),
        max: species.iter().map(|s| s.id).max().unwrap(),
        species: species
            .iter()
            .map(|s| serebii::SpeciesLink {
                id: s.id,
                name: s.name.clone(),
                detail_path: s.source_url.strip_prefix(serebii::ORIGIN).unwrap().into(),
            })
            .collect(),
    };
    let catalogue = Catalogue::from_serebii(&index, species.clone(), GENERATION).unwrap();
    assert_eq!(catalogue.species.len(), species.len());
    for source in &species {
        let stored = catalogue
            .species
            .iter()
            .find(|s| s.id == source.id)
            .unwrap();
        assert_eq!(stored.forms.len(), source.forms.len());
        assert_eq!(stored.excluded_illustrations, source.excluded_illustrations);
        for form in &source.forms {
            let stored = stored.forms.iter().find(|f| f.key == form.key).unwrap();
            assert_eq!(stored.source_id, form.source_id);
            assert_eq!(stored.artwork_url, form.artwork_url);
            assert_eq!(stored.artwork_references, form.artwork_references);
            assert_eq!(stored.source_labels, form.source_labels);
            assert_eq!(stored.type_source, form.type_source);
            assert_eq!(stored.type_label, form.type_label);
            let types: Vec<_> = std::iter::once(stored.primary_type)
                .chain(stored.secondary_type)
                .map(|t| {
                    serde_json::to_value(t)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned()
                })
                .collect();
            assert_eq!(types, form.types);
        }
    }
    assert_eq!(
        Catalogue::from_json(&serde_json::to_vec(&catalogue).unwrap()).unwrap(),
        catalogue
    );
}

#[test]
fn unknown_schema_is_reported_before_deserializing_future_record_shapes() {
    assert_eq!(
        Catalogue::from_json(br#"{"schema_version":99,"future":"shape"}"#).unwrap_err(),
        CatalogueError::UnsupportedSchema(99)
    );
    let mut catalogue = catalogue();
    catalogue.schema_version += 1;
    assert!(matches!(
        catalogue.validate(),
        Err(CatalogueError::UnsupportedSchema(_))
    ));
}

#[test]
fn rejects_invalid_models_and_references_in_persisted_json() {
    type Case = (&'static str, fn(&mut Value));
    let cases: &[Case] = &[
        ("empty species", |v| v["species"] = json!([])),
        ("incomplete bounds", |v| v["source"]["max_id"] = json!(7)),
        ("bad source", |v| {
            v["source"]["index_url"] = json!("https://example.org")
        }),
        ("duplicate id", |v| {
            v["source"]["max_id"] = json!(7);
            let copy = v["species"][0].clone();
            v["species"].as_array_mut().unwrap().push(copy);
        }),
        ("empty name", |v| v["species"][0]["name"] = json!("")),
        ("missing original", |v| {
            v["species"][0]["original_form"] = json!("mega x")
        }),
        ("duplicate form", |v| {
            let copy = v["species"][0]["forms"][0].clone();
            v["species"][0]["forms"].as_array_mut().unwrap().push(copy);
        }),
        ("noncanonical key", |v| {
            v["species"][0]["forms"][0]["key"] = json!("MEGA  X")
        }),
        ("empty key", |v| {
            v["species"][0]["forms"][0]["key"] = json!("")
        }),
        ("unknown type", |v| {
            v["species"][0]["forms"][0]["primary_type"] = json!("unknown")
        }),
        ("duplicate types", |v| {
            let first = v["species"][0]["forms"][0]["primary_type"].clone();
            v["species"][0]["forms"][0]["secondary_type"] = first;
        }),
        ("missing artwork provenance", |v| {
            v["species"][0]["forms"][0]["artwork_references"] = json!([])
        }),
        ("unsafe local path", |v| {
            v["species"][0]["forms"][0]["local_artwork"] = json!("../escape.png")
        }),
        ("duplicate local ref", |v| {
            v["species"][0]["forms"][0]["local_artwork"] =
                v["species"][0]["forms"][1]["local_artwork"].clone();
        }),
        ("foreign artwork", |v| {
            v["species"][0]["forms"][0]["artwork_url"] = json!("https://example.org/a.png")
        }),
        ("missing primary type", |v| {
            v["species"][0]["forms"][0]
                .as_object_mut()
                .unwrap()
                .remove("primary_type");
        }),
        ("unknown fields", |v| v["unexpected"] = json!(true)),
    ];
    let base = serde_json::to_value(catalogue()).unwrap();
    for (name, mutate) in cases {
        let mut value = base.clone();
        mutate(&mut value);
        assert!(
            Catalogue::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{name}"
        );
    }
}

#[test]
fn local_artwork_paths_reject_absolute_traversal_encoding_and_non_png_variants() {
    for path in [
        "/tmp/escape.png",
        "../escape.png",
        "artwork/../escape.png",
        "artwork/0123456789abcdef0123456789abcdef/1.png",
        "artwork/0123456789abcdef0123456789abcdef-01.png",
        "artwork/0123456789abcdef0123456789abcdef-0.png",
        "artwork/0123456789abcdef0123456789abcdef-1.jpg",
        "artwork/0123456789abcdef0123456789abcdef-%2e.png",
        "artwork/0123456789ABCDEF0123456789abcdef-1.png",
        "artwork\\0123456789abcdef0123456789abcdef-1.png",
    ] {
        assert!(ArtworkRef::new(path.into()).is_err(), "{path}");
    }
}

#[test]
fn source_conversion_rejects_missing_index_species_identity_and_type_drift() {
    let index = serebii::parse_index(include_str!("fixtures/serebii-index.html")).unwrap();
    let hub = serebii::parse_hub(
        include_str!("fixtures/serebii-charizard.html"),
        &index.species[0],
    )
    .unwrap();
    let source = serebii::reconcile(&hub, &[]).unwrap();
    assert!(Catalogue::from_serebii(&index, vec![], GENERATION).is_err());
    for types in [
        vec![],
        vec!["unknown".into()],
        vec!["fire".into(), "flying".into(), "dragon".into()],
    ] {
        let mut altered = source.clone();
        altered.forms[0].types = types;
        assert!(Catalogue::from_serebii(&index, vec![altered], GENERATION).is_err());
    }
    let mut altered = source;
    altered.name = "Wrong species".into();
    assert!(Catalogue::from_serebii(&index, vec![altered], GENERATION).is_err());
}
