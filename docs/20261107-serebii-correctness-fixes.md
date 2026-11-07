# Serebii correctness fixes — 2026-11-07

Latest correction, 2026-11-07T01:50:21Z: the [targeted-review fixes](20261107-serebii-targeted-review-fixes.md) resolve the five original/named duplicates using accepted source captions and visual evidence, and reject same-type collisions across independently typed sections. Final replay passes for 1,025 species and **1,427 canonical forms**, with 11 unchanged approved exclusions and no unresolved original/named identity pairs. The earlier 1,432-entry and uncertainty statements below are historical.

Current update at **2026-11-07T01:35:18Z**: D32's five exact exclusions are applied. The complete offline metadata/image-reference gate passes for all 1,025 species and **1,432 illustrated source entries**, with 11 approved exclusions and no unresolved type joins. Five generic OG/named illustration relationships remain explicit identity limitations for the authorized targeted review; 1,432 is not a claim of proven distinct biological forms. The pre-D32 failure account below is retained as history; the final update is at the end.

The authorized correctness review invalidated the initial 1,431-form readiness claim. Four parser fixes and their regression tests are implemented. Complete catalogue acceptance remains blocked by five unsupported type joins and an explicit identity limitation for five generic original/named illustration pairs. No additional exclusions, commits, or reviews were performed.

## Fixes and validation

- C1: game-render basenames no longer identify illustration assets. The hub's explicit No Cloak/Plant Cloak slots retain Burmy Plant Cloak separately from its bare original. Missing/copy-pasted evolutionary labels require the same source evolution condition and species target, followed by an exact image-URL match within the linked game's named-form table. The two D31 exceptions retain their previously approved labels and exact URLs as explicit policy records.
- C2: artwork and type images accept source root-relative, same-host HTTPS, and same-host protocol-relative URLs. Unsupported relevant asset URL forms fail explicitly. Regression mutations preserve both Dragonite forms and both ordered types when URLs become absolute, and reject a foreign-host illustration.
- C3: distinct alternate-form caption slots or distinct artwork-selector options cannot merge into one key merely because their types match. Repeated references to one exact image remain valid. The Deoxys Defense→Attack mutation now fails even though both have Psychic typing; Ogerpon's repeated poses within a single named column remain supported.
- C4: a shared type panel requires an independently named form on that same source page. Unknown Deoxys and Pikachu labels no longer receive a species type by fallback. Six already-linked game captures resolve legitimate older costume, size, battle-transformation, and Spiky-eared Pichu names; the previously cached Mewtwo GO capture supplies named Armored coverage. Source types remain ordered and are not assigned through per-species constants.
- Older source formatting is explicitly supported for Pichu's `Serebii.net Pokédex - #172 Pichu` title, `foo`/`fooinfo` metadata table, and Gen IV type-icon paths. Its regression passes with actual reduced source fixtures.

After the final selector-slot change, `cargo fmt --check`, `cargo test --locked` (30 integration tests: 21 Serebii and nine archived-provider tests), `cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check` passed. No broad recrawl or redownload was performed. Source changes are limited to six additional actual linked game pages and six source-linked PNGs needed by corrected identities; all six PNGs fully decode as nonblank transparent RGBA images. The original 1,431 successful image validations remain reused evidence.

## Identity changes

| Previous state | Corrected source representation |
| --- | --- |
| Burmy Plant Cloak was merged into original/No Cloak through game sprite key `412` | Separate `plant cloak` using the explicitly labelled `412-p.png`; original remains `412.png`, with its source No Cloak alias |
| Vivillon Meadow Pattern merged into generic OG original | Retain named `meadow pattern` candidate (`666-m.png`) separately until an explicit OG identity join is supported |
| Flabébé, Floette, Florges Red Flower merged into their generic OG originals | Retain each named `red flower` candidate (`669-r.png`, `670-r.png`, `671-r.png`) separately pending supported OG identity joins |
| Mimikyu Disguised Form merged into generic OG original | Retain named `disguised` candidate (`778-d.png`) separately pending a supported OG identity join |

The five default-name candidates are **not proven additional biological forms**. Their labels/URLs are explicit source slots, but shared game-render basenames did not establish their relationship to the generic OG illustrations. Different dimensions or file bytes do not prove biological distinction, and visual similarity was not used to restore those aliases. All six newly selected source files passed actual PNG validation. The complete [identity diff](logs/20261107-serebii-correctness-identity-diff.json) records six added source candidates and six alias changes, with no original portrait changes, no existing type-value changes, and 131 type-evidence changes among fully reconciled species. Pikachu and Zygarde are listed separately as atomic reconciliation failures, not silently removed species.

## Exactly five unsupported type joins

| Source species | Source illustration | Missing evidence |
| --- | --- | --- |
| [Pikachu](https://www.serebii.net/pokemon/pikachu/) | [Captain Pikachu](https://www.serebii.net/pokemon/art/025-captain.png) | No independently named type coverage in the actual linked SV/XY captures |
| Pikachu | [Mr. Windychu](https://www.serebii.net/pokemon/art/025-mrwindychu.png) | Same named-character coverage gap |
| Pikachu | [Ms. Waveychu](https://www.serebii.net/pokemon/art/025-mswaveychu.png) | Same named-character coverage gap |
| [Zygarde](https://www.serebii.net/pokemon/zygarde/) | [Cell](https://www.serebii.net/pokemon/art/718-cell.png) | Source calls Core/Cells item-like variants rather than game forms; no supported named type join |
| Zygarde | [Core](https://www.serebii.net/pokemon/art/718-core.png) | Same source limitation |

These are metadata evidence gaps, not PNG download failures. The [machine-readable gap list](logs/20261107-serebii-correctness-unresolved.json) retains every exact source URL and explicitly records that no exclusion approval was applied. D30's four-device exception and D31's two unavailable images remain unchanged. The source's ordinary Pikachu and Zygarde original portraits/types are available, but `reconcile` rejects the whole species while any included form has unresolved types; a partial output must not be treated as a complete catalogue.

## Evidence and replay

The pre-fix [catalogue](logs/20261107-serebii-catalogue-before-correctness-fixes.json) and [summary](logs/20261107-serebii-catalogue-summary-before-correctness-fixes.json) preserve the reviewed result. New evidence consists of the [identity diff](logs/20261107-serebii-correctness-identity-diff.json), [six PNG checks](logs/20261107-serebii-correctness-new-png-validation.json), [138 linked game capture references/hashes](logs/20261107-serebii-correctness-enrichment-captures.json), and [five unresolved joins](logs/20261107-serebii-correctness-unresolved.json). The existing 1,025 hub captures and National index are reused.

```sh
cargo run --locked --example reconcile_serebii -- /private/tmp/pokington-serebii/crawl /private/tmp/pokington-serebii/enrichment /private/tmp/pokington-illustration-validation/corrected-png-validation.json
```

This command must fail while the five type joins remain unresolved. The combined PNG report reuses prior checks and adds the six newly required source images; a metadata failure prevents the overall catalogue gate from passing. Temporary caches remain under `/private/tmp/pokington-serebii` and `/private/tmp/pokington-illustration-validation`. No production bootstrap or CLI behavior was added.

Final replay completed by **2026-11-07T01:29:12Z**, after the final selector-slot change: all 1,025 hubs parsed, 1,458 owned references, 1,023 fully reconciled species and 1,413 forms in the partial output; Pikachu and Zygarde failed atomically for exactly the five joins above. The command exited 1. Every form in that partial output maps to an already validated PNG, but this does not make the complete catalogue gate pass. Six earlier approved exclusions remain unchanged. The [final bounded summary](logs/20261107-serebii-correctness-summary.json) and [explicitly partial catalogue](logs/20261107-serebii-correctness-partial-catalogue.json) supersede the initial readiness claim.

## D32 implementation and final replay

Root recorded the user's explicit approval of the five exclusions at 2026-11-07T01:31:51Z. The implementation matches each approved National ID, normalized source label, and exact illustration URL; it records the source label/reference and `approved_unsupported_type_evidence` reason. Original Pikachu/Zygarde and supported variants remain. A regression checks all five exclusions, retained original/supported forms, and failure for an unapproved changed Cell URL. No general unresolved-type filter was added, and no further network requests were made for this change.

The final full offline command above completed by 2026-11-07T01:35:18Z with exit 0: 1,025 species, 1,458 owned source references, 1,432 selected illustrated source entries, 11 approved exclusions, 15 additional same-identity references, and zero unresolved metadata. Every selected reference passes the recorded actual-PNG gate; all 1,432 selected images are nonblank transparent PNGs with distinct SHA-256 hashes. All original portraits remain unchanged. Ordered types come from hub evidence for 1,270 entries and linked game evidence for 162. Five unresolved OG/named identity relationships remain candidates for review, as explained above; no biological distinction was inferred from file bytes or dimensions.

Post-D32 `cargo fmt --check`, `cargo test --locked` (31 integration tests: 22 Serebii and nine archived providers), `cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check` passed. Compared with the reviewed 1,431-entry snapshot, the complete [current identity diff](logs/20261107-serebii-correctness-identity-diff.json) records six additions, exactly five D32 removals, six alias changes, no original portrait or existing type-value changes, and 146 type-evidence changes.

Current artifacts: [catalogue](logs/20261107-serebii-catalogue-catalogue.json), [summary](logs/20261107-serebii-correctness-summary.json), [current selected PNG validation](logs/20261107-serebii-correctness-current-png-validation.json), [15 alias groups](logs/20261107-serebii-correctness-aliases.json), and [five D32 exclusion records](logs/20261107-serebii-d32-exclusions.json). The pre-D32 [summary](logs/20261107-serebii-correctness-summary-before-d32.json), [identity diff](logs/20261107-serebii-correctness-identity-diff-before-d32.json), and partial catalogue preserve the prior failure state. Ready for the specifically authorized targeted review; no review or commit was performed by the implementation agent.
