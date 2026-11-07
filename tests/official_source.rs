#[allow(dead_code)]
#[path = "../examples/support/official_source.rs"]
mod official_source;
use official_source::{catalogue_url, parse_index_page, parse_species_detail, parse_species_links};
use pokington::source;
use serde_json::{Value, json};

const INDEX: &str = r#"<html lang="en"><head><script data-main="https://assets.pokemon.com/static2/_ui/js/pokedex.js"></script></head><body><input id="minRangeBox" value="149"><input id="maxRangeBox" value="149"><script>window.pokemonUI.api["pokedex"] = "/api/1/us/kalos/";</script></body></html>"#;
const SPECIES: &str = include_str!("fixtures/main-species.json");
const DETAIL: &str = include_str!("fixtures/main-dragonite.html");

#[test]
fn discovers_source_routes_bounds_and_real_new_mega_form() {
    let page = parse_index_page(INDEX).unwrap();
    let script = r#"urlBase:window.pokemonUI.api.pokedex;this.url=this.urlBase+"kalos""#;
    assert_eq!(
        catalogue_url(&page, script).unwrap(),
        "https://www.pokemon.com/api/1/us/kalos/kalos"
    );
    let links = parse_species_links(SPECIES, &page).unwrap();
    let species = parse_species_detail(DETAIL, &links[0]).unwrap();
    assert_eq!(species.id, 149);
    assert_eq!(species.name, "Dragonite");
    assert_eq!(species.original_key, "original");
    assert_eq!(species.forms.len(), 2);
    assert_eq!(species.forms[1].key, "mega");
    assert_eq!(species.forms[1].display_name, "Mega Dragonite");
    assert_eq!(species.forms[1].source_id, "/us/pokedex/dragonite#1");
    assert_eq!(species.forms[1].types, ["dragon", "flying"]);
    assert!(species.forms[1].artwork_url.ends_with("/149_f2.png"));
}

#[test]
fn rejects_missing_duplicate_filtered_and_malformed_species_metadata() {
    let page = parse_index_page(INDEX).unwrap();
    let original: Value = serde_json::from_str(SPECIES).unwrap();
    let mut duplicate = original.clone();
    duplicate.as_array_mut().unwrap().push(original[0].clone());
    for (label, input) in [
        ("empty", "[]".into()),
        ("malformed", "{broken".into()),
        ("duplicate", duplicate.to_string()),
        ("anti-bot", "<html><iframe></iframe></html>".into()),
    ] {
        assert!(parse_species_links(&input, &page).is_err(), "{label}");
    }
    let mut missing = original.clone();
    missing[0].as_object_mut().unwrap().remove("detailPageURL");
    assert!(parse_species_links(&missing.to_string(), &page).is_err());
    let mut wrong = original;
    wrong[0]["id"] = json!(150);
    assert!(parse_species_links(&wrong.to_string(), &page).is_err());
    let mut wider = page;
    wider.max = 150;
    assert!(parse_species_links(SPECIES, &wider).is_err());
}

#[test]
fn rejects_challenge_truncation_and_source_discovery_drift() {
    for input in [
        "<html><iframe src=\"/_Incapsula_Resource\"></iframe></html>",
        "<html><title>Pardon Our Interruption</title></html>",
        "<html lang=en></html>",
        INDEX.trim_end_matches("</html>"),
    ] {
        assert!(parse_index_page(input).is_err());
    }
    let page = parse_index_page(INDEX).unwrap();
    assert!(catalogue_url(&page, "different script").is_err());
    assert!(
        parse_index_page(&INDEX.replace("/api/1/us/kalos/", "https://other.example/api/")).is_err()
    );
}

#[test]
fn rejects_partial_duplicate_and_misaligned_form_references() {
    let page = parse_index_page(INDEX).unwrap();
    let links = parse_species_links(SPECIES, &page).unwrap();
    let cases = [
        (
            "wrong species",
            DETAIL.replace("content=\"149\"", "content=\"150\""),
        ),
        (
            "missing image",
            DETAIL.replace("profile-images", "unrelated-images"),
        ),
        (
            "missing type panel",
            DETAIL.replacen("pokedex-pokemon-attributes", "unrelated", 1),
        ),
        (
            "missing selector",
            DETAIL.replace("id=\"formes\"", "id=\"unrelated\""),
        ),
        (
            "duplicate selector",
            DETAIL.replace("value=\"1\"", "value=\"0\""),
        ),
        ("duplicate artwork", DETAIL.replace("149_f2.png", "149.png")),
        (
            "duplicate key",
            DETAIL.replace("Mega Dragonite", "Dragonite"),
        ),
        (
            "mismatched label",
            DETAIL.replace("alt=\"Mega Dragonite\"", "alt=\"other\""),
        ),
        ("unsafe artwork", DETAIL.replace("149_f2.png", "../149.png")),
        ("missing type", DETAIL.replace("dtm-type", "unrelated")),
    ];
    for (label, html) in cases {
        assert!(parse_species_detail(&html, &links[0]).is_err(), "{label}");
    }
}
