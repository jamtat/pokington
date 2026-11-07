# Pokington

A cli application written in rust for working with pokemon names and images. Bootstrap data on first run. Do not hardcode the number of pokemon always reference the source data where possible.

On first run, download the complete catalogue and all original and alternate-form artwork before completing the requested command. Once bootstrap completes, ordinary `choose` and `art` commands must work offline using local data.

The first release supports macOS and Linux. Windows support is outside the first-release scope.

## Commands

### choose

`choose -c/--count n [-e/--exclude name]... [files]`

picks n pokemon from the list. -e will exclude pokemon with that name (match case insensitive). If the name of a pokemon occurs inside any of [files] then it can not be chosen. Exit 1 and print to stderr when it is not possible to choose n pokemon given constraints. Output one per line.

Names found inside supplied files are matched case-insensitively as substrings. For example, a file containing `MEWTWO` excludes both `Mew` and `Mewtwo`.

If any supplied exclusion file cannot be read, fail before printing any names and report the file in the error.

Only original Pokémon names are eligible; alternate forms do not add entries to name selection.

Sample uniformly without replacement. `--exclude` is repeatable, and an unknown excluded name has no effect. `--count` is required; zero succeeds without printing names, while negative or malformed counts are argument errors. Read supplied files as UTF-8; invalid encoding fails before printing names, with the file identified.

### art

`art [name/id] [--form form] [--mode auto|img|blocks|ascii] [--width n|original] [--color auto|always|never]`

Print art for pokemon with id or name matching (name case insensitive). For a specified Pokémon, use the original form unless `--form` explicitly selects an alternate form. No specified pokemon produces random artwork, with equal probability for every species/form artwork entry, including originals and alternate forms. Species with more forms therefore have a greater combined probability. Transparent pixels should be trimmed off the source png before display (crop to only visible pixels).

2026-11-07 artwork preference: use illustration-style artwork. HOME's 3D game-render style was inspected and rejected by the user; do not select it as the default merely because it has complete PNG coverage. Local artwork remains PNG. See [D27](20261106-design.md#d27--use-illustration-style-artwork--2026-11-07).

`--form` accepts descriptive, case-insensitive names rather than numeric form indexes. Invalid-form errors list the available form names for the selected Pokémon. Alternate artwork includes Mega Evolutions as well as regional forms, Gigantamax, and other forms exposed by the source. Distinct Mega forms must remain separately selectable.

2026-11-07 eligibility decision: include alternate forms only when a matching illustration PNG is available from Serebii. Every original species remains included. Variations available only as game renders or sprites are omitted; there is no render/sprite fallback or substitution of an original image for a distinct form. Preserve all illustrated Mega, regional, Gigantamax and other forms. See [D29](20261106-design.md#d29--illustration-only-form-eligibility--2026-11-07).

Exclude Rotom Phone, Rotom Drone, Rotomi and Rotom Phone Paldea Ver.: these non-battle device illustrations lack supported type data. Original Rotom and its battle forms remain included. See [D30](20261106-design.md#d30--exclude-four-untyped-non-battle-device-entries--2026-11-07).

Exclude Artisan Poltchageist and Masterpiece Sinistcha: their source illustration links are confirmed missing, with no linked illustration alternatives. Preserve their unavailable-artwork evidence; other download failures remain errors. See [D31](20261106-design.md#d31--exclude-two-unavailable-source-illustrations--2026-11-07).

Exclude Captain Pikachu, Mr. Windychu, Ms. Waveychu, Zygarde Cell and Zygarde Core because the checked source-linked pages do not support their form types. Preserve their exclusion evidence and retain original Pikachu/Zygarde and other supported forms. See [D32](20261106-design.md#d32--exclude-five-unsupported-non-battle-depictions--2026-11-07).

When `--form` is provided without a Pokémon name or ID, choose uniformly from all species/form artwork entries matching that form name. Report an error if no entries match.

modes (default auto):

- auto: Choose the most natural one for the terminal. Default to img, then blocks, then ascii.

If explicit `--mode img` is unsupported by the terminal, fail with a clear error suggesting `--mode auto`, `blocks`, or `ascii`. Do not silently fall back; automatic fallback belongs to `--mode auto`.

With no `--width`, fit the terminal's available width and height while preserving aspect ratio; redirected ASCII defaults to 80 columns. `--width n` requests output n character columns wide. `--width original` disables resizing after trimming transparent margins: native image mode uses one source pixel per screen pixel; block mode uses one source pixel per half-block, packing two pixel rows per character row; ASCII mode uses one source pixel per character. Original-size output may overflow the terminal, and ASCII character proportions may stretch it vertically.

Artwork prints inline and the command exits, leaving the result visible. Numeric widths must be positive. Unknown names/forms, invalid local artwork references, and unsupported local catalogue schemas are errors.

When stdout is piped or redirected, `--mode auto` emits plain, uncoloured ASCII without terminal control codes by default.

`--color auto|always|never` controls ANSI colour in block and ASCII rendering and defaults to `auto`. `auto` uses colour in a terminal and plain text when piped or redirected; `always` emits ANSI colour even when redirected; `never` renders monochrome text. Native image mode retains the PNG's colours. With redirected stdout, `--mode auto` still selects ASCII, and an explicit `--color always` can colour it.

Art should be printed with ratatui-image if img used. Try and autodetect the best output type from the terminal. For blocks use unicode blocks to match the colours. For ascii convert the image to coloured ascii art, subject to the colour policy above.

When downscaling/upscaling use appropriate sampling in every display mode.

### Shared command behavior

Operational failures exit 1; argument errors are reported by `clap` with exit 2. Errors and bootstrap progress go to stderr so stdout contains only requested names or artwork. Help/version and argument validation do not trigger bootstrap.

Data commands trigger initial bootstrap when no completed dataset exists. After installation, missing/corrupt local data produces an offline error suggesting explicit bootstrap repair rather than an automatic network update.

### bootstrap

`bootstrap [--force]`

Run the bootstrap process. Skip already downloaded assets. `--force` forces redownloading all assets. Bootstrap should always show a measure of progress in some way with both a progress bar and a log of what is being downloaded.

Each explicit `bootstrap` refreshes the catalogue from the source, discovering new Pokémon and forms, then downloads new or missing artwork while skipping existing files. `--force` also redownloads existing artwork. After successful first-run bootstrap, ordinary `choose` and `art` commands use local data without checking for catalogue updates.

Keep successfully downloaded artwork so interrupted bootstrap can resume without downloading completed assets again. Initial bootstrap is complete only when the catalogue and all artwork are ready. During updates, keep the previous complete dataset usable until its replacement is ready, including during `--force`. Download failures print to stderr and exit nonzero.

## Data

Source species, forms, types and artwork from [Serebii's National Pokédex](https://www.serebii.net/pokemon/nationalpokedex.shtml) and its source-linked detail pages. Verify complete discovery and required artwork coverage before adoption; derive counts and form identities from the source.

2026-11-06 source correction: the user instructed switching to Serebii after main-site anti-bot challenges blocked complete discovery. This supersedes D06's official-only restriction and D22's main-site priority. Complete species/form coverage remains required. See [D25](20261106-design.md#d25--switch-to-serebiis-national-pokedex--2026-11-06). The earlier official-only and main-site decisions remain in the design history.

2026-11-06 current investigation update: the user now requests official-looking PNG artwork for all Pokémon from Pokémon Database and a fresh investigator. Verify native PNG coverage and actual image quality for every original species; report alternate-form coverage separately. Production source adoption and any image conversion/fallback policy remain subject to the resulting evidence. See [D26](20261106-design.md#d26--fresh-pokemon-database-artwork-investigation--2026-11-06).

2026-11-07 artwork source decision: after inspecting the render style and native illustration evidence, the user selected Serebii's illustration PNGs. All 1,025 original species and the 45 newer Mega candidates were downloaded and fully verified as visible, transparent native PNGs. Continue with Serebii; no JPEG conversion or HOME default is adopted. D29 subsequently settled alternate eligibility as illustration-only, and D30 excludes exactly four untyped Rotom devices. See [D28](20261106-design.md#d28--select-serebiis-native-illustration-pngs--2026-11-07).

All data requests impersonate a browser by default using an appropriate, consistent browser HTTP request profile. This uses HTTP clients and does not require browser automation. See [D23](20261106-design.md#d23--browser-impersonation-for-all-data-requests--2026-11-06).

Bootstrap needs to pull in data about all the pokemon by fetching from the website. Save the pokedex number, the name, the primary and secondary type. Also save the image. Save in a format that's simple and fast to read - used on every call. Also need to save all the png artwork for every Pokemon.

Keep one species entry per National Pokédex number for name selection, with original and alternate artwork forms associated with that species.

Store a versioned JSON catalogue using `serde` and `serde_json`, alongside PNG artwork files, in the platform's standard per-user application data directory.

The catalogue contains its schema version, source information, and species list. Each species stores its Pokédex number, original name, original-form reference, and forms. Each form stores its descriptive key, display name, source identifier, primary and optional secondary type, artwork URL, and local file reference. Types are stored per form rather than duplicated on the species record. `choose` uses species names; `art` resolves a form.

### Historical image URLs

These original Pokémon.com examples are retained as history. Current Serebii artwork URLs must be discovered from source-linked pages rather than generated from these patterns.

- Artwork under 1000 padded to 3 digits with 0: https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/001.png
- Artwork 1000 and over not padded: https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/1000.png
- Alternate forms have a suffix: https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/019_f2.png

## Dependencies

- clap: cmdline handling
- wreq 0.16.3 and wreq-util 0.2.0: browser-profile HTTP, replacing the originally requested reqwest under D24
- ratatui-image: for displaying images on the cmdline
- serde and serde_json: versioned local JSON catalogue

## Design clarifications

These entries preserve discussion history. Current command and data sections above govern behavior; later entries resolve earlier open notes.

- 2026-11-06: Name selection uses original Pokémon names only. This clarifies the earlier unspecified treatment of alternate forms: artwork includes alternate forms, named artwork defaults to the original, `--form` selects an explicit form, and random artwork includes all forms. See [D01](20261106-design.md#d01--species-names-and-artwork-forms--2026-11-06).
- 2026-11-06: Random artwork has equal probability per species/form artwork entry, rather than choosing species uniformly first. See [D02](20261106-design.md#d02--random-artwork-distribution--2026-11-06).
- 2026-11-06: Explicitly include Mega Evolutions and use descriptive, case-insensitive form names with available names reported on invalid-form errors. See [D03](20261106-design.md#d03--descriptive-form-names-and-mega-evolutions--2026-11-06).
- 2026-11-06: Without a Pokémon name or ID, `--form` filters the random artwork selection to matching forms. See [D04](20261106-design.md#d04--form-filter-for-random-artwork--2026-11-06).
- 2026-11-06: First-run bootstrap downloads the complete catalogue and all artwork before the requested command completes; ordinary commands subsequently work offline. See [D05](20261106-design.md#d05--complete-first-run-bootstrap--2026-11-06).
- 2026-11-06: The source requirement now allows other official English-language Pokémon websites after coverage verification, superseding the earlier US-only source wording. See [D06](20261106-design.md#d06--official-english-language-source-policy--2026-11-06).
- 2026-11-06: Each explicit bootstrap refreshes the catalogue and downloads new or missing artwork; `--force` also redownloads existing artwork. Ordinary commands do not check for updates. See [D07](20261106-design.md#d07--explicit-bootstrap-refresh-policy--2026-11-06).
- 2026-11-06: Store a versioned JSON catalogue using `serde` and `serde_json`, with PNG artwork files, in the platform's standard per-user application data directory. See [D08](20261106-design.md#d08--local-json-catalogue-and-png-storage--2026-11-06).
- 2026-11-06: Bootstrap retains completed downloads for retries and publishes a replacement dataset only after completion; failed updates preserve the previous complete dataset. See [D09](20261106-design.md#d09--bootstrap-recovery-and-dataset-publication--2026-11-06).
- 2026-11-06: Names found inside supplied files are excluded using case-insensitive substring matching. See [D10](20261106-design.md#d10--case-insensitive-file-exclusions--2026-11-06).
- 2026-11-06: The first release supports macOS and Linux; Windows support is outside this release's scope. See [D11](20261106-design.md#d11--first-release-platforms--2026-11-06).
- 2026-11-06: With piped or redirected stdout, automatic artwork mode defaults to plain, uncoloured ASCII without terminal control codes. An explicit colour override is being designed separately. See [D12](20261106-design.md#d12--automatic-artwork-with-redirected-stdout--2026-11-06).
- 2026-11-06: The colour override is now settled: `--color auto|always|never` controls ANSI colour for block and ASCII rendering, while native images retain PNG colours. Explicit `always` overrides the default plain redirected output from D12. See [D13](20261106-design.md#d13--artwork-colour-control--2026-11-06).
- 2026-11-06: Omitted width fits the terminal, with an 80-column default for redirected ASCII. Add `--width original` for original-size output; exact mode-specific semantics remain open. See [D14](20261106-design.md#d14--default-sizing-and-original-width-option--2026-11-06).
- 2026-11-06: Original-size semantics are settled: crop transparent margins, then render without resizing using the mode-specific pixel mappings above, even if output exceeds the terminal. This resolves the open semantics in D14. See [D15](20261106-design.md#d15--original-size-rendering-semantics--2026-11-06).
- 2026-11-06: An explicitly requested image mode fails clearly when unsupported instead of silently falling back. Automatic mode retains fallback behavior. See [D16](20261106-design.md#d16--unsupported-explicit-image-mode--2026-11-06).
- 2026-11-06: The catalogue shares species identity across form records and stores types and artwork references per form. See [D17](20261106-design.md#d17--catalogue-species-and-form-records--2026-11-06).
- 2026-11-06: An unreadable exclusion file causes `choose` to fail before printing names, with an error identifying the file. See [D18](20261106-design.md#d18--unreadable-exclusion-files--2026-11-06).
- 2026-11-06: Plan approval accepts distinct sampling, repeatable exclusions, UTF-8 files, count and exit-code rules, offline help/version, repair errors and inline artwork output. Implementation uses task goals and two ordered independent review passes under the plan; those workflow rules do not change application behavior. See [D19](20261106-design.md#d19--plan-approval-task-goals-and-two-review-passes--2026-11-06).
