# Pokémon Database artwork investigation — 2026-11-06

**Result:** Pokémon Database provides a coherent native PNG render tier for **all 1,025 original species** through its HOME galleries. Every discovered HOME image was downloaded and fully decoded. These are transparent 256×256 game renders. Its larger Global Link illustration tier covers only 807 species. Neither tier covers every currently listed alternate form: HOME lacks 45 of the catalogue's 93 Mega forms.

The investigation began on 2026-11-06. Full PNG validation completed at **2026-11-06T23:59:38.385572Z**; the report was finalized after midnight on **2026-11-07 UTC**. The filename retains the investigation's start date.

## Scope and method

- Fresh source investigation requested by the user: official-looking PNG artwork for every Pokémon, trying Pokémon Database.
- Started at <https://pokemondb.net/pokedex/all>. Parsed the actual table: **1,025 unique species and 1,261 stat rows**. These counts are observations, not configured limits.
- Followed each actual species link and its artwork/sprite-gallery links. No image suffix probing, browser control, imported credentials, challenge solving, or image conversion.
- Used the existing `official_source::http_client()` Chrome 147/macOS HTTP profile, five-redirect limit, three concurrent requests, a 150 ms delay before each uncached request, and local caching.
- Reusable investigation entry point: `examples/probe_pokemondb.rs`. Raw captures and downloaded images are in `/tmp/pokemondb-investigation`. The probe is separate from production bootstrap and CLI code.
- Species coverage and alternate-form coverage are reported separately. A species being represented does not establish complete coverage of its forms.

## Native illustration PNGs

The source labels its outlined, flat-colour PNG illustrations **Global Link artwork**. These are full-body illustrations rather than pixel battle sprites. The source-linked galleries provide them for **807 of 1,025 species**. The missing 218 species are precisely the source-listed sequence from **Meltan (#808) through Pecharunt (#1025)**. No other native illustration-PNG tier was found in those galleries. This is a finding about the pages and links examined, not a claim that undisclosed files cannot exist.

The linked galleries contain **1,066 Global Link artwork entries**, including alternate forms and alternate poses. Of all species, 176 have no artwork-gallery link; their species pages were still inspected. Standard Sugimori artwork is linked as JPEG, with AVIF used for some displayed versions.

Representative exact links:

| Source page | Image | Observation |
| --- | --- | --- |
| [Charizard artwork](https://pokemondb.net/artwork/charizard) | [Global Link original](https://img.pokemondb.net/artwork/vector/large/charizard.png) | Visually inspected full-body flat-colour illustration; 800×782 RGBA with 321,494 fully transparent pixels. |
| [Charizard artwork](https://pokemondb.net/artwork/charizard) | [Mega Charizard X](https://img.pokemondb.net/artwork/vector/charizard-mega-x.png), [Mega Charizard Y](https://img.pokemondb.net/artwork/vector/charizard-mega-y.png) | Separate source-linked PNG illustrations. |
| [Charizard artwork](https://pokemondb.net/artwork/charizard) | [Sugimori original](https://img.pokemondb.net/artwork/large/charizard.jpg), [Gigantamax](https://img.pokemondb.net/artwork/large/charizard-gigantamax.jpg) | Source links JPEG illustrations, not native PNG equivalents. |
| [Pecharunt species](https://pokemondb.net/pokedex/pecharunt) | [Sugimori artwork](https://img.pokemondb.net/artwork/large/pecharunt.jpg) | Source-linked JPEG for the last enumerated species. |

Global Link also includes separate Mega Mewtwo X/Y, Alolan forms, and all 28 Unown symbols. It does not provide the Charizard Gigantamax, Galarian Meowth/Weezing, or Paldean Tauros illustrations exposed as JPEGs. Later-generation form coverage cannot be inferred from the older examples.

## HOME render candidate

The source places HOME artwork under its sprite galleries, but the actual files are smooth full-body game renders, not pixel battle sprites or tiny icons. Charizard's [source gallery](https://pokemondb.net/sprites/charizard) links [this original PNG](https://img.pokemondb.net/sprites/home/normal/charizard.png): a visually inspected 256×256 RGBA image with 39,795 fully transparent pixels. It is lower resolution and visually different from the 800-pixel Global Link illustration. Its gallery also exposes distinct Mega X, Mega Y and Gigantamax PNG links.

All **1,540 source-linked normal HOME render PNGs** were subsequently downloaded and checked, including a verified candidate for each of the **1,025 species**. Every response was HTTP 200 with `image/png`; every image passed PNG signature, CRC, Adler32, full frame decode and stream-end checks. All 1,540 are **256×256 RGBA and contain fully transparent pixels**. There were **zero HTTP or decode failures** and **zero missing species**. Total HOME image bytes: 52,205,082.

Additional visually inspected examples were [Bulbasaur](https://img.pokemondb.net/sprites/home/normal/bulbasaur.png), [Gigantamax Charizard](https://img.pokemondb.net/sprites/home/normal/charizard-gigantamax.png), [Teal Mask Ogerpon](https://img.pokemondb.net/sprites/home/normal/ogerpon-teal.png) and [Pecharunt](https://img.pokemondb.net/sprites/home/normal/pecharunt.png). The latter two confirm the same render style at the recent end of the catalogue. Individual sources are [Ogerpon's gallery](https://pokemondb.net/sprites/ogerpon) and [Pecharunt's gallery](https://pokemondb.net/sprites/pecharunt).

All **1,066 Global Link PNGs** were also downloaded and passed the same HTTP and complete-decode checks, with zero failures. Dimensions vary from 101 to 800 pixels wide and 112 to 800 pixels high. Of these, 1,065 are RGBA with transparency; [Rhydon](https://img.pokemondb.net/artwork/vector/large/rhydon.png) is a valid **787×800 RGB opaque PNG**. Thus even the incomplete Global Link tier is not uniformly transparent. Total Global Link image bytes: 119,834,102.

## Original-form association

- The manifest associates each image with the species page and its actual HOME gallery; image names were never guessed.
- **872 species** have an unlabelled original HOME entry.
- **40 species** have an explicitly named original form in the catalogue. Each has exactly one matching HOME label after normalizing the spelling `Form`/`Forme`. These include Normal Forme Deoxys, Altered Forme Giratina, 50% Forme Zygarde, Zero Form Palafin, Family of Four Maushold, Teal Mask Ogerpon and Normal Form Terapagos.
- **113 species** have an unqualified catalogue original while HOME names gender, cosmetic or default variants, such as Female/Male, Unown letters, Vivillon patterns and Alcremie combinations. A normal variant is recorded and validated for each species. This proves available original-species artwork; it does not establish a universal production default-selection policy for those variants.
- Gallery order is not a safe default rule: HOME lists Hero Form before Zero Form for Palafin and Family of Three before Family of Four for Maushold. The verified manifest uses the explicitly matching original forms instead.

## Alternate-form coverage

| Category | Source-derived comparison | Result |
| --- | --- | --- |
| Mega forms | 93 Mega-labelled rows in the live catalogue | HOME has 48 matching forms and lacks 45, including Mega Raichu X/Y. The complete 45-name gap list is in the summary JSON. Existing Charizard X/Y and Mewtwo X/Y remain separate. |
| Regional forms | 55 explicitly Alolan/Galarian/Hisuian/Paldean-labelled catalogue rows | Every row has a HOME entry. Hisuian Sneasel has separate gender labels; all labels were retained rather than taking only the first. |
| Paldean Tauros | Three named breeds in the catalogue | Combat, Blaze and Aqua all have separate HOME entries. |
| Gigantamax | 33 Sugimori-gallery entries across 32 species | HOME has the corresponding 33 entries across the same 32 species, including both Urshifu styles. |

All HOME entries in this comparison passed the complete PNG checks. This does not establish coverage of every possible alternate form: the newer Mega gap is real, and no claim of universal alternate-form completeness is made. The HOME gallery for Raichu, for example, does not expose its new Mega X/Y PNGs. No guessed alternatives were tested.

## Evidence and limits

- [Summary and complete missing-species/form lists](logs/20261106-pokemondb-summary.json): source-derived counts, 218 Global Link species gaps, 45 HOME Mega gaps, and original-association counts.
- [Per-species manifest](logs/20261106-pokemondb-manifest.json): all 1,025 species, exact original candidates, every Global Link/HOME image URL, source-gallery URLs and labels.
- [PNG validation results](logs/20261106-pokemondb-png-validation.json): all 2,606 HTTP responses, dimensions, colour model, transparent-pixel count, full-decode result, SHA-256 and local cache path. No failures.
- Raw HTML/images remain in `/tmp/pokemondb-investigation` (about 331 MiB). Binary images were not added to the repository.
- `cargo build --quiet --example probe_pokemondb` and `cargo clippy --quiet --example probe_pokemondb -- -D warnings` passed. Build output includes the existing btls linker-directory warning; it did not prevent execution. Live enumeration and full-image decoding are the meaningful checks for this investigation; no sample/fixture test is presented as proof of all-species coverage.
- Reproduce with `cargo run --example probe_pokemondb -- CACHE_DIRECTORY`, then the same command with `home` and `png` as the final argument. The cache preserves successful downloads. Manifest/summary aggregation was performed offline against these captures.
- The complete native option is a **256-pixel HOME render tier**, not universal large Sugimori/Global Link PNG illustration coverage. The visual style and resolution need the user's acceptance. Standard source-provided JPEG illustration links were recorded as an alternative; no JPEG-to-PNG conversion or background removal was performed.
- No mixed-tier fallback, provider adoption, production bootstrap change, commit, push, PR, or independent review was performed.
