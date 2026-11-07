#[allow(dead_code)]
#[path = "../examples/support/portal_source.rs"]
mod portal_source;
use portal_source::parse_portal_index;
use serde_json::{Value, json};

fn fixture() -> Result<Value, serde_json::Error> {
    serde_json::from_str(include_str!("fixtures/portal-charizard.json"))
}

fn document(value: &Value) -> String {
    let record = format!("a:[\"$\",\"$L1\",null,{value}]\n");
    // Deliberately split within the record, as the real Next.js response does.
    let split = record
        .char_indices()
        .nth(record.chars().count() / 2)
        .map_or(record.len(), |(index, _)| index);
    let (first, second) = record.split_at(split);
    let chunks: [&str; 2] = (first, second).into();
    format!(
        "<html lang=\"en-SG\"><body>{}</body></html>",
        chunks
            .map(|chunk| format!("<script>self.__next_f.push({})</script>", json!([1, chunk])))
            .join("")
    )
}

#[test]
fn discovers_source_counts_types_and_distinct_mega_keys() {
    let index = parse_portal_index(&document(&fixture().unwrap())).unwrap();
    assert_eq!(index.species_count(), 1);
    assert_eq!(index.entries.len(), 4);
    assert_eq!(index.bounds(), (6, 6));
    let keys = index.form_keys().unwrap();
    assert_eq!(keys["0006"], "original");
    assert_eq!(keys["0006_1"], "mega x");
    assert_eq!(keys["0006_2"], "mega y");
    assert_eq!(keys["0006_3"], "gigantamax");
    assert_eq!(index.entries[1].pokemon_type_id, "fire,dragon");
    assert_eq!(index.entries[2].pokemon_type_id, "fire,flying");
}

#[test]
fn rejects_antibot_and_truncated_transport() {
    let antibot = "<html><body><iframe src=\"/_Incapsula_Resource\"></iframe></body></html>";
    assert!(parse_portal_index(antibot).is_err());
    let html = document(&fixture().unwrap());
    assert!(parse_portal_index(html.trim_end_matches("</html>")).is_err());
    assert!(
        parse_portal_index(
            "<html lang=\"en-SG\"><script>self.__next_f.push([1,\"a:{broken}\\n\"])</script></html>"
        )
        .is_err()
    );
}

#[test]
fn rejects_partial_duplicate_and_missing_references() {
    type FailureCase = (&'static str, fn(&mut Value));
    let cases: &[FailureCase] = &[
        ("empty index", |v| v["initialPokemons"] = json!([])),
        ("missing original", |v| {
            v["initialPokemons"].as_array_mut().unwrap().remove(0);
        }),
        ("missing species", |v| {
            v["searchItems"]["pokemonMaxId"] = json!(7);
        }),
        ("duplicate id", |v| {
            let copy = v["initialPokemons"][0].clone();
            v["initialPokemons"].as_array_mut().unwrap().push(copy);
        }),
        ("missing image", |v| {
            v["initialPokemons"][0]
                .as_object_mut()
                .unwrap()
                .remove("file_name");
        }),
        ("unsafe image", |v| {
            v["initialPokemons"][0]["file_name"] = json!("/images/pokedex/../x.png");
        }),
        ("unknown type", |v| {
            v["initialPokemons"][0]["pokemon_type_id"] = json!("unknown,flying");
        }),
        ("mismatched type", |v| {
            v["initialPokemons"][0]["pokemon_type_name"] = json!("Water,Flying");
        }),
        ("missing bounds", |v| {
            v["searchItems"]
                .as_object_mut()
                .unwrap()
                .remove("pokemonMaxId");
        }),
        ("filtered index", |v| v["variant"] = json!("searchResult")),
        ("external image base", |v| {
            v["imageBaseUrl"] = json!("https://example.com");
        }),
    ];
    for (name, mutate) in cases {
        let mut value = fixture().unwrap();
        mutate(&mut value);
        assert!(parse_portal_index(&document(&value)).is_err(), "{name}");
    }
}

#[test]
fn rejects_duplicate_descriptive_keys_without_inventing_suffixes() {
    let mut value = fixture().unwrap();
    value["initialPokemons"][1]["pokemon_sub_name"] = json!("Paldean Form");
    value["initialPokemons"][2]["pokemon_sub_name"] = json!(" paldean   form ");
    let index = parse_portal_index(&document(&value)).unwrap();
    assert!(
        index
            .form_keys()
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
}

#[test]
fn rejects_duplicate_index_payloads() {
    assert!(
        parse_portal_index(&document(&json!([fixture().unwrap(), fixture().unwrap()]))).is_err()
    );
}
