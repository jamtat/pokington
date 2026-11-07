# Serebii targeted review fixes — 2026-11-07

The authorized targeted correctness review accepted C1/C2/C4 and the exact D32 exclusions, then identified two remaining blockers: five duplicate original/named forms and a same-type collision across independently typed form sections. Both received bounded source-adapter fixes. No network requests, new exclusions, commits, additional reviews, or generic filename inference were introduced.

## Verified original aliases

The targeted reviewer independently compared the source captions and actual depictions. These findings supersede the earlier “unresolved identity” disclaimer; different file bytes or dimensions had not established distinct biological forms.

| Source | Original and named illustration | Verified identity |
| --- | --- | --- |
| [Vivillon](https://www.serebii.net/pokemon/vivillon/), captured hub line 2895 | `666.png` / `666-m.png`, “Meadow Pattern” | Same pink Meadow pattern, including white/turquoise bands |
| [Flabébé](https://www.serebii.net/pokemon/flabebe/), line 2904 | `669.png` / `669-r.png`, “Red Flower” | Both carry a red flower |
| [Floette](https://www.serebii.net/pokemon/floette/), line 2904 | `670.png` / `670-r.png`, “Red Flower” | Both are ordinary Red Flower; neither is Eternal Flower |
| [Florges](https://www.serebii.net/pokemon/florges/), line 2904 | `671.png` / `671-r.png`, “Red Flower” | Both are Red Flower |
| [Mimikyu](https://www.serebii.net/pokemon/mimikyu/), line 2893 | `778.png` / `778-d.png`, “Disguised Form” | Both are intact Disguised, unlike the collapsed `778-b.png` Busted depiction |

Each alias record requires the exact National ID, source OG illustration URL, named illustration URL and caption. Reconciliation retains the OG as the selected `original`, retains both artwork references and labels, and removes the duplicate named canonical entry. It does not infer aliases between game-render and illustration directories or from a shared type. Burmy's independently established Plant Cloak remains separate from No Cloak.

Reduced source fixtures test all five aliases, preservation of both references, and rejection of wrong URLs, labels or species identities. Negative URL/caption tests include actual linked game evidence independently naming and typing the form, and assert the specific alias-identity error rather than incidental missing-type failure. A verified named URL or normalized known-family identity triggers validation; the complete exact tuple is still required to merge. Changing both the URL and a minor “Form” caption spelling therefore fails explicitly. Validation runs before generic original-key matching; changing a known named caption to the bare species name also fails, with a regression covering all five families. Existing Burmy regression coverage remains passing. The prior 1,432-entry catalogue, summary and identity diff are preserved in `docs/logs/*-before-targeted-fixes.json`.

## Independently typed sections

Artwork now retains a slot for its independently typed source table and column, including the original metadata panel. Canonical grouping rejects collisions across these slots even when the types are equal. This extends the existing caption/selector slot checks to Mega, Primal, Ultra and other dedicated typed sections without a Dragonite-specific rule.

The exact review mutation `DRAGONITE.replace("Mega Evolution - Mega Dragonite", "Mega Evolution - Dragonite")` now fails reconciliation rather than merging `149.png` and `149-m.png` into `original`. Regression variants using Primal Reversion and Ultra Burst headings also fail while preserving the same Dragon/Flying type values. No type constants were introduced.

## Checks

The checks are `cargo fmt --check`, `cargo test --locked` (33 integration tests: 24 Serebii and nine archived-provider tests), `cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check`. A first complete replay passed at 1,427 forms before root requested the stronger known-family drift guard; that follow-up prompted a necessary fresh check/replay after the final guard and enriched regression changes. The final result is recorded below. The source PNG bodies and previously verified hashes were reused.

Final full replay completed by **2026-11-07T01:50:21Z**, exit 0: **1,025 species, 1,427 canonical forms, 11 unchanged approved exclusions, 20 extra same-form references, and zero unresolved joins**. Every selected image reference passes the existing full-PNG validation report. All 1,025 original portraits are unchanged. Ordered type evidence comes from hub panels for 1,270 forms and linked game pages for 157. The five original/named identity limitations are now resolved by the documented source/visual evidence.

All required checks above passed after the final guard changes; the final bare-species-caption addition was test-only and passed the complete test suite without changing the runtime already covered by this replay. Compared with the preserved 1,432-entry snapshot, exactly five named entries merged into their existing `original` forms, retaining all five named URLs and captions. There are no new forms, original portrait changes, existing type-value changes, or exclusion changes. Burmy Plant remains separate.

Current artifacts: [canonical catalogue](logs/20261107-serebii-catalogue-catalogue.json), [final summary](logs/20261107-serebii-targeted-summary.json), [exact targeted identity diff](logs/20261107-serebii-targeted-identity-diff.json), [current aliases](logs/20261107-serebii-correctness-aliases.json), and [selected PNG validation](logs/20261107-serebii-correctness-current-png-validation.json). Historical snapshots remain preserved. Ready for root assessment and the already approved simplification review; no additional correctness review or commit was performed.

## Simplification validation — 2026-11-07

The approved simplification review found no blockers. Its two accepted optional changes are now applied: hub and enrichment parsing reuse their already validated `Html` documents through a private type-evidence extractor, while the public wrapper still validates input; text extraction splits each text node directly before collecting and joining. No ingestion validation was removed.

The single post-change offline replay wrote its complete catalogue at **2026-11-07T01:55:53Z**, with zero reconciliation errors. Its output is byte-identical to the preserved pre-simplification cache and structurally identical to the repository catalogue and [preserved repository snapshot](logs/20261107-serebii-catalogue-before-simplification.json), including keys, ordered types, provenance, references and exclusions. Both cache SHA-256 values are `f289a88004887a49af20ac3069c25cee3e16694e9eb68abb5c41d5211e0ba7d2`. Counts remain **1,025 species, 1,427 forms and 11 exclusions**. Every selected reference again passes the existing successful full-PNG decode report; no network requests or image downloads occurred.

`cargo fmt --check`, `cargo test --locked` (33 integration tests), `cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check` all passed after the changes. Tests emit existing btls linker search-directory warnings but finish successfully. The [simplification validation record](logs/20261107-serebii-simplification-validation.json) records the exact source hash, catalogue hashes and check results. No runtime behavior, source identity or policy changed. Ready for root's concrete commit proposal; no commit or additional review was performed.
