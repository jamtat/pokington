use pokington::serebii::{
    Species, SpeciesLink, form_key, parse_enrichment, parse_hub, parse_index, parse_type_evidence,
    reconcile,
};

const INDEX: &str = include_str!("fixtures/serebii-index.html");
const DRAGONITE: &str = include_str!("fixtures/serebii-dragonite.html");
const CHARIZARD: &str = include_str!("fixtures/serebii-charizard.html");
const TAUROS: &str = include_str!("fixtures/serebii-tauros.html");

fn species(id: u32, name: &str) -> SpeciesLink {
    SpeciesLink {
        id,
        name: name.into(),
        detail_path: format!("/pokemon/{}", name.to_lowercase()),
    }
}

#[test]
fn source_bounds_and_links_are_discovered() {
    let index = parse_index(INDEX).unwrap();
    assert_eq!((index.min, index.max), (6, 6));
    assert_eq!(index.species.len(), 1);
    assert_eq!(index.species[0].name, "Charizard");
    assert_eq!(index.species[0].detail_path, "/pokemon/charizard");
}

#[test]
fn partial_duplicate_malformed_and_challenge_indices_fail() {
    for (case, html) in [
        ("partial", INDEX.replace("006-006", "006-007")),
        (
            "duplicate",
            INDEX.replace(
                "</table></body>",
                &format!(
                    "{}</table></body>",
                    &INDEX[INDEX.find("<tr>\n").unwrap()..INDEX.rfind("</table>").unwrap()]
                ),
            ),
        ),
        (
            "unknown path",
            INDEX.replace("/pokemon/charizard", "https://example.org/charizard"),
        ),
        ("truncated", INDEX.replace("</html>", "")),
        (
            "challenge",
            "<html><title>Pardon Our Interruption</title></html>".into(),
        ),
    ] {
        assert!(parse_index(&html).is_err(), "{case}");
    }
}

#[test]
fn hub_inventory_preserves_distinct_megas_and_source_breed_names() {
    let dragonite = parse_hub(DRAGONITE, &species(149, "Dragonite")).unwrap();
    assert_eq!(dragonite.artwork.len(), 2);
    assert_eq!(dragonite.artwork[1].label, "Mega Dragonite");
    let charizard = parse_hub(CHARIZARD, &species(6, "Charizard")).unwrap();
    let catalogue = reconcile(&charizard, &[]).unwrap();
    assert_eq!(
        catalogue
            .forms
            .iter()
            .map(|f| f.key.as_str())
            .collect::<Vec<_>>(),
        ["gigantamax", "mega x", "mega y", "original"]
    );
    assert_eq!(form(&catalogue, "mega x").types, ["fire", "dragon"]);
    assert_eq!(form(&catalogue, "mega y").types, ["fire", "flying"]);
    assert!(
        form(&catalogue, "gigantamax")
            .artwork_url
            .ends_with("006-gi.png")
    );
    assert_eq!(form(&catalogue, "gigantamax").artwork_references.len(), 2);
    let tauros = parse_hub(TAUROS, &species(128, "Tauros")).unwrap();
    assert_eq!(tauros.artwork.len(), 4);
    assert_eq!(tauros.artwork[2].label, "Paldean Form Blaze Breed");
    assert_eq!(tauros.artwork[3].label, "Paldean Form Aqua Breed");
    // Inventory does not claim canonical completeness: the captured Unown hub has one image.
    let unown = parse_hub(
        include_str!("fixtures/serebii-unown.html"),
        &species(201, "Unown"),
    )
    .unwrap();
    assert_eq!(unown.artwork.len(), 1);
}

#[test]
fn explicit_named_types_are_preserved_without_guessing_inheritance() {
    let types = parse_type_evidence(include_str!("fixtures/serebii-tauros-types.html")).unwrap();
    let blaze = types
        .iter()
        .find(|group| group.label == "Paldean Blaze Breed")
        .unwrap();
    assert_eq!(blaze.types, ["fighting", "fire"]);
    let aqua = types
        .iter()
        .find(|group| group.label == "Paldean Aqua Breed")
        .unwrap();
    assert_eq!(aqua.types, ["fighting", "water"]);
    let hub = parse_hub(TAUROS, &species(128, "Tauros")).unwrap();
    assert!(
        !hub.type_groups
            .iter()
            .any(|group| group.label == "Blaze Breed")
    );
}

fn form<'a>(catalogue: &'a Species, key: &str) -> &'a pokington::serebii::Form {
    catalogue
        .forms
        .iter()
        .find(|f| f.key == key)
        .unwrap_or_else(|| panic!("missing form {key}"))
}

fn enriched(id: u32, name: &str, hub_html: &str, game_html: &str) -> Species {
    let hub = parse_hub(hub_html, &species(id, name)).unwrap();
    let extra = parse_enrichment(&hub, &hub.game_links[0], game_html).unwrap();
    reconcile(&hub, &[extra]).unwrap()
}

#[test]
fn linked_regional_panels_supply_ordered_types() {
    let tauros = enriched(
        128,
        "Tauros",
        TAUROS,
        include_str!("fixtures/serebii-tauros-types.html"),
    );
    assert_eq!(
        form(&tauros, "paldean blaze breed").types,
        ["fighting", "fire"]
    );
    assert_eq!(
        form(&tauros, "paldean aqua breed").types,
        ["fighting", "water"]
    );
    let darmanitan = enriched(
        555,
        "Darmanitan",
        include_str!("fixtures/serebii-darmanitan.html"),
        include_str!("fixtures/serebii-darmanitan-types.html"),
    );
    assert_eq!(
        form(&darmanitan, "galarian zen mode").types,
        ["ice", "fire"]
    );
    assert_eq!(form(&darmanitan, "zen mode").types, ["fire", "psychic"]);
}

#[test]
fn misleading_and_missing_image_labels_use_linked_exact_identifiers() {
    let slowking = enriched(
        199,
        "Slowking",
        include_str!("fixtures/serebii-slowking.html"),
        include_str!("fixtures/serebii-slowking-types.html"),
    );
    assert_eq!(form(&slowking, "galarian").types, ["poison", "psychic"]);
    assert!(!form(&slowking, "galarian").display_name.contains("Slowbro"));
    let tea = enriched(
        1012,
        "Poltchageist",
        include_str!("fixtures/serebii-poltchageist.html"),
        include_str!("fixtures/serebii-poltchageist-types.html"),
    );
    assert_eq!(tea.forms.len(), 1);
    let artisan = &tea.excluded_illustrations[0];
    assert_eq!(artisan.label, "Artisan Form");
    assert!(artisan.artwork_url.ends_with("1012-m.png"));
    assert_eq!(
        artisan.reason,
        pokington::serebii::ExclusionReason::ApprovedUnavailableIllustration
    );
    assert!(artisan.source_context.contains("HTTP 404"));
}

#[test]
fn arbitrary_unverified_selected_images_fail_instead_of_being_omitted() {
    use pokington::serebii::validate_artwork_references;
    use std::collections::BTreeSet;
    let hub = parse_hub(DRAGONITE, &species(149, "Dragonite")).unwrap();
    let catalogue = vec![reconcile(&hub, &[]).unwrap()];
    let mut verified: BTreeSet<_> = catalogue[0]
        .forms
        .iter()
        .map(|f| f.artwork_url.clone())
        .collect();
    assert!(validate_artwork_references(&catalogue, &verified).is_ok());
    verified.remove(&form(&catalogue[0], "mega").artwork_url);
    assert!(validate_artwork_references(&catalogue, &verified).is_err());
    assert_eq!(catalogue[0].forms.len(), 2);
    assert!(validate_artwork_references(&catalogue, &BTreeSet::new()).is_err());
}

#[test]
fn table_captions_and_explicit_tera_column_override_copied_alt_text() {
    let ogerpon = enriched(
        1017,
        "Ogerpon",
        include_str!("fixtures/serebii-ogerpon.html"),
        include_str!("fixtures/serebii-ogerpon-types.html"),
    );
    assert_eq!(ogerpon.forms.len(), 8);
    assert_eq!(form(&ogerpon, "wellspring mask").types, ["grass", "water"]);
    assert_eq!(form(&ogerpon, "cornerstone mask tera").types, ["rock"]);
    assert_eq!(form(&ogerpon, "hearthflame mask tera").types, ["fire"]);
    assert!(
        form(&ogerpon, "cornerstone mask tera")
            .type_label
            .contains("Tera Type")
    );
}

#[test]
fn two_gigantamax_styles_keep_distinct_descriptive_keys_and_portraits() {
    let urshifu = enriched(
        892,
        "Urshifu",
        include_str!("fixtures/serebii-urshifu.html"),
        include_str!("fixtures/serebii-urshifu-types.html"),
    );
    assert_eq!(urshifu.forms.len(), 4);
    let single = form(&urshifu, "gigantamax single strike style");
    let rapid = form(&urshifu, "gigantamax rapid strike style");
    assert_eq!(single.types, ["fighting", "dark"]);
    assert_eq!(rapid.types, ["fighting", "water"]);
    assert_ne!(single.artwork_url, rapid.artwork_url);
}

#[test]
fn only_four_approved_untyped_devices_are_excluded_with_source_context() {
    let rotom = enriched(
        479,
        "Rotom",
        include_str!("fixtures/serebii-rotom.html"),
        include_str!("fixtures/serebii-rotom-types.html"),
    );
    assert_eq!(rotom.excluded_illustrations.len(), 4);
    assert_eq!(rotom.forms.len(), 8);
    for excluded in &rotom.excluded_illustrations {
        assert!(excluded.source_context.contains("not usable as a Pok"));
    }
    assert_eq!(form(&rotom, "dex").types, ["electric"]);
    assert_eq!(form(&rotom, "stereo").types, ["electric", "normal"]);
    let mut hub = parse_hub(
        include_str!("fixtures/serebii-rotom.html"),
        &species(479, "Rotom"),
    )
    .unwrap();
    hub.non_battle_device_context = None;
    assert!(reconcile(&hub, &[]).is_err());
}

#[test]
fn original_portraits_and_illustrated_only_scope_are_preserved() {
    for (id, name, html, count) in [
        (201, "Unown", include_str!("fixtures/serebii-unown.html"), 1),
        (
            869,
            "Alcremie",
            include_str!("fixtures/serebii-alcremie.html"),
            2,
        ),
        (
            841,
            "Flapple",
            include_str!("fixtures/serebii-flapple.html"),
            2,
        ),
    ] {
        let hub = parse_hub(html, &species(id, name)).unwrap();
        let catalogue = reconcile(&hub, &[]).unwrap();
        assert_eq!(catalogue.forms.len(), count, "{name}");
        assert_eq!(
            form(&catalogue, "original").artwork_url,
            hub.original_artwork
        );
        assert!(
            catalogue
                .forms
                .iter()
                .all(|f| !f.artwork_url.ends_with("842.png"))
        );
    }
}

#[test]
fn game_mega_sections_do_not_pollute_shared_original_type_panel() {
    let groups = parse_type_evidence(include_str!("fixtures/serebii-mewtwo-types.html")).unwrap();
    let main: Vec<_> = groups.iter().filter(|g| g.section == "Name").collect();
    assert_eq!(main.len(), 1);
    assert_eq!(main[0].types, ["psychic"]);
    assert!(
        groups
            .iter()
            .any(|g| g.section.contains("Mega Mewtwo X") && g.types == ["psychic", "fighting"])
    );
}

#[test]
fn enrichment_rejects_unlinked_pages_and_mismatched_species() {
    let hub = parse_hub(TAUROS, &species(128, "Tauros")).unwrap();
    let html = include_str!("fixtures/serebii-tauros-types.html");
    assert!(parse_enrichment(&hub, "/pokedex-sv/guessed/", html).is_err());
    assert!(parse_enrichment(&hub, &hub.game_links[0], &html.replace("#128", "#129")).is_err());
    assert!(
        parse_enrichment(
            &hub,
            &hub.game_links[0],
            &html.replace("Tauros -", "Pikachu -")
        )
        .is_err()
    );
    assert!(
        reconcile(&hub, &[]).is_err(),
        "missing breed types must not inherit original types"
    );
}

#[test]
fn conflicting_duplicate_form_keys_and_missing_original_fail() {
    let mut hub = parse_hub(CHARIZARD, &species(6, "Charizard")).unwrap();
    for art in &mut hub.artwork {
        if art.label == "Mega Charizard Y" {
            art.label = "Mega Charizard X".into();
        }
    }
    assert!(reconcile(&hub, &[]).is_err());
    let mut hub = parse_hub(DRAGONITE, &species(149, "Dragonite")).unwrap();
    hub.artwork.retain(|art| art.url != hub.original_artwork);
    assert!(reconcile(&hub, &[]).is_err());
}

#[test]
fn form_keys_are_shared_across_species_and_retain_meaningful_distinctions() {
    assert_eq!(form_key("Mega Charizard X", "Charizard"), "mega x");
    assert_eq!(form_key("Mega Mewtwo X", "Mewtwo"), "mega x");
    assert_eq!(form_key("Alola Form", "Raichu"), "alolan");
    assert_eq!(
        form_key("Paldean Form Blaze Breed", "Tauros"),
        "paldean blaze breed"
    );
    assert_ne!(
        form_key("Mega Charizard X", "Charizard"),
        form_key("Mega Charizard Y", "Charizard")
    );
}

#[test]
fn game_sprite_basenames_cannot_merge_burmy_plant_with_bare_original() {
    let burmy = enriched(
        412,
        "Burmy",
        include_str!("fixtures/serebii-burmy.html"),
        include_str!("fixtures/serebii-burmy-types.html"),
    );
    assert_eq!(burmy.forms.len(), 4);
    let original = form(&burmy, "original");
    assert!(original.artwork_url.ends_with("/412.png"));
    assert!(
        !original
            .source_labels
            .iter()
            .any(|label| label == "Plant Cloak")
    );
    assert!(
        !original
            .artwork_references
            .iter()
            .any(|url| url.ends_with("412-p.png"))
    );
    let plant = form(&burmy, "plant cloak");
    assert!(plant.artwork_url.ends_with("412-p.png"));
    assert_eq!(plant.types, ["bug"]);
}

#[test]
fn supported_absolute_and_protocol_relative_assets_preserve_forms_and_types() {
    for origin in ["https://www.serebii.net", "//www.serebii.net"] {
        let html = DRAGONITE
            .replace(
                "src=\"/pokemon/art/",
                &format!("src=\"{origin}/pokemon/art/"),
            )
            .replace(
                "src=\"/pokedex-bw/type/",
                &format!("src=\"{origin}/pokedex-bw/type/"),
            );
        let hub = parse_hub(&html, &species(149, "Dragonite")).unwrap();
        let catalogue = reconcile(&hub, &[]).unwrap();
        assert_eq!(catalogue.forms.len(), 2);
        assert_eq!(form(&catalogue, "original").types, ["dragon", "flying"]);
        assert_eq!(form(&catalogue, "mega").types, ["dragon", "flying"]);
    }
    let foreign = DRAGONITE.replace(
        "src=\"/pokemon/art/149-m.png",
        "src=\"https://example.org/pokemon/art/149-m.png",
    );
    assert!(parse_hub(&foreign, &species(149, "Dragonite")).is_err());
}

#[test]
fn different_deoxys_slots_cannot_merge_even_when_their_types_match() {
    let html = include_str!("fixtures/serebii-deoxys.html");
    let game = include_str!("fixtures/serebii-deoxys-types.html");
    let baseline = enriched(386, "Deoxys", html, game);
    assert_eq!(baseline.forms.len(), 4);
    assert_eq!(
        form(&baseline, "attack").types,
        form(&baseline, "defense").types
    );
    let changed = html.replace("Defense Forme", "Attack Forme");
    let hub = parse_hub(&changed, &species(386, "Deoxys")).unwrap();
    let extra = parse_enrichment(&hub, &hub.game_links[0], game).unwrap();
    let error = reconcile(&hub, &[extra]).unwrap_err();
    assert!(error.0.contains("distinct source form slots collide"));
}

#[test]
fn unknown_form_names_do_not_receive_unqualified_shared_types() {
    let html =
        include_str!("fixtures/serebii-deoxys.html").replace("Defense Forme", "Unknown Forme");
    let hub = parse_hub(&html, &species(386, "Deoxys")).unwrap();
    let extra = parse_enrichment(
        &hub,
        &hub.game_links[0],
        include_str!("fixtures/serebii-deoxys-types.html"),
    )
    .unwrap();
    assert!(
        reconcile(&hub, &[extra])
            .unwrap_err()
            .0
            .contains("unknown: 0 type assignments")
    );
    let html =
        include_str!("fixtures/serebii-pikachu.html").replace("Pikachu Belle", "Unknown Form");
    let hub = parse_hub(&html, &species(25, "Pikachu")).unwrap();
    let extra = parse_enrichment(
        &hub,
        &hub.game_links[0],
        include_str!("fixtures/serebii-pikachu-types.html"),
    )
    .unwrap();
    assert!(
        reconcile(&hub, &[extra])
            .unwrap_err()
            .0
            .contains("unknown: 0 type assignments")
    );
}

#[test]
fn older_linked_game_layout_names_and_types_spiky_eared_pichu() {
    let pichu = enriched(
        172,
        "Pichu",
        include_str!("fixtures/serebii-pichu.html"),
        include_str!("fixtures/serebii-pichu-types.html"),
    );
    assert_eq!(form(&pichu, "spiky eared").types, ["electric"]);
    assert!(
        form(&pichu, "spiky eared")
            .type_source
            .ends_with("/pokedex-dp/172.shtml")
    );
}

#[test]
fn d32_excludes_only_five_exact_unsupported_references() {
    use pokington::serebii::ExclusionReason;
    let hub = parse_hub(
        include_str!("fixtures/serebii-pikachu.html"),
        &species(25, "Pikachu"),
    )
    .unwrap();
    let sv = parse_enrichment(
        &hub,
        "/pokedex-sv/pikachu/",
        include_str!("fixtures/serebii-pikachu-types.html"),
    )
    .unwrap();
    let xy = parse_enrichment(
        &hub,
        "/pokedex-xy/025.shtml",
        include_str!("fixtures/serebii-pikachu-xy-types.html"),
    )
    .unwrap();
    let pikachu = reconcile(&hub, &[sv, xy]).unwrap();
    assert_eq!(pikachu.excluded_illustrations.len(), 3);
    assert_eq!(form(&pikachu, "belle").types, ["electric"]);
    assert_eq!(form(&pikachu, "original").artwork_url, hub.original_artwork);
    for excluded in &pikachu.excluded_illustrations {
        assert_eq!(
            excluded.reason,
            ExclusionReason::ApprovedUnsupportedTypeEvidence
        );
        assert!(excluded.source_context.contains("D32"));
    }
    let html = include_str!("fixtures/serebii-zygarde.html");
    let game = include_str!("fixtures/serebii-zygarde-types.html");
    let zygarde = enriched(718, "Zygarde", html, game);
    assert_eq!(zygarde.excluded_illustrations.len(), 2);
    assert_eq!(form(&zygarde, "10%").types, ["dragon", "ground"]);
    assert!(form(&zygarde, "original").artwork_url.ends_with("/718.png"));
    let changed = html
        .replace("718-cell.png", "718-unknown.png")
        .replace("data-key=\"718-cell\"", "data-key=\"718-unknown\"");
    let hub = parse_hub(&changed, &species(718, "Zygarde")).unwrap();
    let extra = parse_enrichment(&hub, &hub.game_links[0], game).unwrap();
    assert!(
        reconcile(&hub, &[extra])
            .unwrap_err()
            .0
            .contains("cell: 0 type assignments")
    );
}

#[test]
fn reviewed_original_aliases_require_exact_source_identity() {
    for (id, name, suffix, label, html, game) in [
        (
            666,
            "Vivillon",
            "m",
            "Meadow Pattern",
            include_str!("fixtures/serebii-vivillon-original-alias.html"),
            include_str!("fixtures/serebii-vivillon-original-alias-types.html"),
        ),
        (
            669,
            "Flabébé",
            "r",
            "Red Flower",
            include_str!("fixtures/serebii-flabebe-original-alias.html"),
            include_str!("fixtures/serebii-flabebe-original-alias-types.html"),
        ),
        (
            670,
            "Floette",
            "r",
            "Red Flower",
            include_str!("fixtures/serebii-floette-original-alias.html"),
            include_str!("fixtures/serebii-floette-original-alias-types.html"),
        ),
        (
            671,
            "Florges",
            "r",
            "Red Flower",
            include_str!("fixtures/serebii-florges-original-alias.html"),
            include_str!("fixtures/serebii-florges-original-alias-types.html"),
        ),
        (
            778,
            "Mimikyu",
            "d",
            "Disguised Form",
            include_str!("fixtures/serebii-mimikyu-original-alias.html"),
            include_str!("fixtures/serebii-mimikyu-original-alias-types.html"),
        ),
    ] {
        let hub = parse_hub(html, &species(id, name)).unwrap();
        let extra = parse_enrichment(&hub, &hub.game_links[0], game).unwrap();
        assert!(extra.named_forms.contains(&form_key(label, name)));
        let result = reconcile(&hub, std::slice::from_ref(&extra)).unwrap();
        assert_eq!(result.forms.len(), 1, "{name}");
        let original = form(&result, "original");
        assert_eq!(original.artwork_url, hub.original_artwork);
        assert_eq!(original.artwork_references.len(), 2);
        assert!(
            original
                .source_labels
                .iter()
                .any(|source_label| source_label == label)
        );
        let unknown_url = html.replace(
            &format!("{id}-{suffix}.png"),
            &format!("{id}-unverified.png"),
        );
        let unknown_label = html.replace(label, "Unverified Form");
        let bare_species_label = html.replace(label, name);
        let changed_url_and_caption = unknown_url.replace(label, &format!("{label} Form"));
        for changed in [
            unknown_url,
            unknown_label,
            bare_species_label,
            changed_url_and_caption,
        ] {
            let hub = parse_hub(&changed, &species(id, name)).unwrap();
            let error = reconcile(&hub, std::slice::from_ref(&extra)).unwrap_err();
            assert!(
                error.0.contains("verified original alias identity changed"),
                "{name}: {error}"
            );
        }
        let mut wrong_id = hub;
        wrong_id.id += 1;
        assert!(
            reconcile(&wrong_id, &[]).is_err(),
            "wrong species cannot reuse {name} alias"
        );
    }
}

#[test]
fn independently_typed_sections_cannot_merge_with_original_even_with_same_types() {
    for section in [
        "Mega Evolution - Dragonite",
        "Primal Reversion - Dragonite",
        "Ultra Burst - Dragonite",
    ] {
        let changed = DRAGONITE.replace("Mega Evolution - Mega Dragonite", section);
        let hub = parse_hub(&changed, &species(149, "Dragonite")).unwrap();
        assert!(
            hub.type_groups
                .iter()
                .all(|group| group.types == ["dragon", "flying"])
        );
        let error = reconcile(&hub, &[]).unwrap_err();
        assert!(
            error.0.contains("distinct source form slots collide"),
            "{section}: {error}"
        );
    }
}

#[test]
fn partial_identity_artwork_and_type_failures_are_rejected() {
    for (case, html) in [
        (
            "identity",
            DRAGONITE.replace("#0149 Dragonite", "#0001 Dragonite"),
        ),
        (
            "missing original",
            DRAGONITE.replace("/pokemon/art/149.png", "/missing.png"),
        ),
        (
            "format drift",
            DRAGONITE.replace("/pokemon/art/149-m.png", "/pokemon/art/149-m.jpg"),
        ),
        (
            "unknown type",
            DRAGONITE.replace(
                "/pokedex-bw/type/dragon.gif",
                "/pokedex-bw/type/unknown.gif",
            ),
        ),
        ("truncated", DRAGONITE.replace("</html>", "")),
    ] {
        assert!(
            parse_hub(&html, &species(149, "Dragonite")).is_err(),
            "{case}"
        );
    }
}
