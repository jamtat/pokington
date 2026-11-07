# S01.T01 official-source evidence

2026-11-06T23:02:10Z root update: the approved wreq browser-profile client still received challenge HTML on one quiet-interval retry of the main index (HTTP 200, 5,871 bytes, `Pardon Our Interruption`, SHA-256 `40bf64cbe54043a17eaaecce2dbc2085d918b8fa7e462e3cf9ada8a95c85bbaa`). All fetching then stopped. Nine tests, formatting and Clippy pass; complete main-site form discovery, both independent reviews and the approved task commit remain unfinished. Earlier successful index/API/detail observations below remain valid historical evidence, not current repeatable full-source acceptance. Browser impersonation is the default; summon and computer/browser control are disabled by user instruction. Canonical permissions and current status: [Run 01](logs/20261106-s01-implementation-log.md).

Investigation date: 2026-11-06. Task began at 22:27:38Z. Checks completed before the clock observation at 22:34:05Z. Source selection remains unapproved and S01.T01 remains incomplete. The prototype is an offline inspection tool, not bootstrap implementation.

## Result and decision needed

- The official English Singapore index contains a discoverable, internally consistent snapshot: **1,025 species, 1,251 species/form entries, 1,251 distinct PNG references**, with source-provided bounds **1–1,025**. These are observed values, never application constants. There are 226 alternate records, 48 names beginning with `Mega `, and 29 records labelled `Gigantamax`.
- The linked English Philippines and India indices expose exactly the same metadata entries. Each contains only original Dragonite. The official English Legends: Z-A website separately describes Mega Dragonite. The regional index therefore does not include every form described on other official websites.
- D06 requires equivalent coverage; the brief also describes forms exposed by the source. Whether a complete snapshot of this official regional catalogue satisfies that boundary despite known cross-site omissions needs an explicit decision. This investigation does not resolve that policy by adopting the regional source silently.
- Independently, descriptive form naming remains unfinished: Tauros source IDs `0128_1`, `0128_2`, and `0128_3` all have `pokemon_sub_name = "Paldean Form"`, with types Fighting, Fighting/Fire, and Fighting/Water. The prototype rejects that key collision. Detail-page enrichment or an agreed descriptive normalization may resolve it; no numeric name or undocumented breed label was invented.
- No complete, currently equivalent alternative was established by the bounded checks below. This is not proof that no official source exists. No unofficial data was used, no production source was selected, and no task/stage acceptance is claimed.

## Discovery mechanism

The [Singapore index](https://sg.portal-pokemon.com/pokedex/) is the redirect destination of the previously documented `/play/pokedex/` route. Its HTML declares `lang="en-SG"`. Inline Next.js `self.__next_f.push([1, ...])` records jointly contain one unfiltered `PokedexList` props object with `variant="top"`, `initialPokemons`, `searchItems`, and `imageBaseUrl=""`.

The referenced [index UI script](https://sg.portal-pokemon.com/_next/static/chunks/3tt_xtvoyqw-l.js) receives `initialPokemons`, sorts that array, and displays `P.slice(0,j)`. Its load-more button changes `j` by 16; it does not fetch a further page. Thus the embedded array, rather than the first 16 rendered cards, is this source's complete unfiltered list. The source's `searchItems.pokemonMinId` and `pokemonMaxId` define its current species bounds. The parser verifies one original record for each number within those bounds without a compiled maximum or guessed suffix requests.

| Source field | Observed meaning and prototype handling |
| --- | --- |
| `zukan_id` | Zero-padded National Pokédex number; retained and validated against source bounds |
| `zukan_sub_id` | Source form identifier; zero identifies the original; never used as the user-facing form name |
| `pokemon_name` | English displayed name; original record supplies species name; some Mega names include the species |
| `pokemon_sub_name` | Descriptive form label where supplied; originals can also have labels, such as Terapagos `Normal Form` |
| `pokemon_type_id`, `pokemon_type_name` | One or two comma-separated type IDs/names, validated against the index's type dictionary for every form |
| `file_name` | Actual PNG path, e.g. `/images/pokedex/<hash>.png`; no numeric artwork/suffix enumeration |
| `imageBaseUrl` | Empty in captures; artwork is resolved at the source origin; prototype rejects an unexpected nonempty base |

The official card component builds detail routes from `zukan_id`, appending `_zukan_sub_id` only for nonzero sub-IDs. `PortalEntry::source_id()` reproduces that observed identity rule. Candidate user keys use the explicit sub-name when present, otherwise `original` for the base or the displayed alternate name with the species name removed; whitespace is collapsed and case lowered. This gives `mega x`, `mega y`, `alola form`, `galarian form`, and `gigantamax` in the samples. It intentionally fails on ambiguous keys instead of silently overwriting forms.

## Coverage checks and bounded alternatives

| Official source / access path | Observation |
| --- | --- |
| [US index](https://www.pokemon.com/us/pokedex) | Direct HTTPS retrieval returned 1,042 bytes of Incapsula/iframe HTML, not catalogue data. Web retrieval also exposed only an iframe. HTTP success alone is not accepted. |
| [US candidate API](https://www.pokemon.com/us/api/pokedex/kalos), [US Dragonite](https://www.pokemon.com/us/pokedex/dragonite), [UK index](https://www.pokemon.com/uk/pokedex) | Web retrieval exposed iframe content. Candidate API remains unverified, not an accepted source contract. |
| Normal browser attempt on the US index | `cua.createBrowserTab("iab", ...)` returned “The browser access host is unavailable”; no browser inspection was completed and no bypass was attempted. |
| [Singapore](https://sg.portal-pokemon.com/pokedex/), [Philippines](https://ph.portal-pokemon.com/pokedex/), [India](https://in.portal-pokemon.com/pokedex/) | Full HTML captures parsed successfully. Each has 1,025 species / 1,251 entries and identical canonical entry payload hashes. PH and IN are English alternatives linked by the SG page itself. |
| [Singapore Dragonite](https://sg.portal-pokemon.com/pokedex/0149/) | Detail page says “This Pokémon does not have different form.” Full index has only `(0149, 0)`. |
| [Official English Mega Dragonite](https://legends.pokemon.com/en-us/story-world/pokemon/mega-dragonite) | Direct full HTML retrieval succeeded; title identifies Mega Dragonite, description calls it a newly discovered Mega-Evolved Pokémon, and page gives Dragon/Flying typing. Establishes a concrete form present elsewhere officially but absent from the regional catalogue. |
| [Legends: Z-A Pokémon listing](https://legends.pokemon.com/en-us/story-world/pokemon) | Links Mega Dragonite and other new Megas, but introduces only “some of the Pokémon” and says more are coming. It does not establish a complete national species/form index or a complete replacement catalogue. No supplemental merge was implemented. |
| [Singapore Pecharunt](https://sg.portal-pokemon.com/pokedex/1025/) | Matches the highest species number in the captured source: Pecharunt, Poison/Ghost. The source also lists all three Terapagos entries at 1024. This proves the source's upper end, not global freshness. |

## Representative artwork

All URLs below came directly from the index `file_name` field. Nine downloads completed at 22:33:37–22:33:38Z. `png` 0.18.1 decoded each complete frame and finished reading the PNG stream without disabling checksum verification. All measured **630×630, 8-bit**; the first seven are indexed-colour PNGs, and the last two are RGBA. This samples artwork feasibility; it is not a download or validation of all 1,251 files.

| Source ID / label | Source types | PNG bytes | Source PNG |
| --- | --- | ---: | --- |
| 0001 Bulbasaur / original | Grass, Poison | 73,668 | [PNG](https://sg.portal-pokemon.com/images/pokedex/cf47f9fac4ed3037ff2a8ea83204e32aff8fb5f3.png) |
| 0006_1 Mega Charizard X / mega x | Fire, Dragon | 93,613 | [PNG](https://sg.portal-pokemon.com/images/pokedex/ca3db4aad5c85a525d9be86852b26db1db7a22c0.png) |
| 0006_2 Mega Charizard Y / mega y | Fire, Flying | 91,905 | [PNG](https://sg.portal-pokemon.com/images/pokedex/0aa78a0061bda9d88cbb0bbf739cd9cc56522fe9.png) |
| 0052 Meowth / original | Normal | 80,729 | [PNG](https://sg.portal-pokemon.com/images/pokedex/6ea01871238908780334293e6407033650d803a9.png) |
| 0052_1 Meowth / alola form | Dark | 80,431 | [PNG](https://sg.portal-pokemon.com/images/pokedex/57a0c88227aa0fe2327d79af31eb9516d4728752.png) |
| 0052_2 Meowth / galarian form | Steel | 60,777 | [PNG](https://sg.portal-pokemon.com/images/pokedex/704fc0cde74c862f2063faefcf40eb67752a8637.png) |
| 0052_3 Meowth / gigantamax | Normal | 35,703 | [PNG](https://sg.portal-pokemon.com/images/pokedex/bff57f3b31012cef1da149224a84180492f90ed4.png) |
| 1024_2 Terapagos / stellar form | Normal | 364,713 | [PNG](https://sg.portal-pokemon.com/images/pokedex/5f6667b0f6280e1f0149da8cd946ad0569ae90f3.png) |
| 1025 Pecharunt / original | Poison, Ghost | 196,733 | [PNG](https://sg.portal-pokemon.com/images/pokedex/f2f8e759f5f72e9ed8ec1496bc6a1e4822f11009.png) |

## Captured evidence

Raw responses and PNGs remain in `/tmp/pokington-source/` for this run; these temporary files are not durable repository artifacts. Timestamps below are observed file modification times after retrieval, rounded to seconds, not remote publication dates. SHA-256 values identify the exact captured bytes. The reduced Charizard metadata fixture in `tests/fixtures/portal-charizard.json` retains the four source records and necessary dictionary fields; its bounds are deliberately reduced to species 6 for isolated tests.

| Capture | UTC time | SHA-256 |
| --- | --- | --- |
| `index.html` (SG) | 22:28:39 | `50f60a077a05bf9be91ff614047ce42974be53ac3e0dbb61cac675a7730c49e3` |
| `catalogue-ui.js` | 22:29:06 | `914f80dbe5586fe8670093f598bdf9da2c1b4a43b012d00e1332c88c8ee88480` |
| `mega-dragonite.html` | 22:29:50 | `cc27f9aeaecac3227ea6dac32f568d9c0c5b0bf540e96a4e4cafed7baad26b0b` |
| `ph-index.html` | 22:30:06 | `1054fb3588f5fb44df5885f3f8c603cd1f6cf48916765cf06a744975b47ea0e7` |
| `us-index.html` | 22:30:15 | `b83de0a26948f2527bbd84bf8fb09128265ddbd079cbd0b886640704b2ba96cd` |
| `in-index.html` | 22:30:19 | `b4c9eae1c8d8a412c2f5e79dfe5e1585e4496f4883c9d0855e17e8de553e119c` |

The three portal entry arrays all hash to `23cf6c7771b4c125cd113ea9aba022e424bedf2c94fbfec5938b982449980a76` after Python `json.dumps(entries, sort_keys=True).encode()` serialization. No count comparison alone was used to infer identical coverage.

## Prototype and validation

- `src/source.rs`: typed source rows, inline-chunk extraction without JavaScript execution, source-derived counts/bounds, type/artwork/original validation, and candidate descriptive key validation. Unknown source structure fails; no fallback or source adoption occurs.
- `examples/inspect_source.rs`: reads captured HTML and optional downloaded PNGs, prints counts and form-key blockers, validates full PNG decoding. Its successful process exit means inspection completed; its warning explicitly prevents interpreting that as source acceptance.
- `tests/source.rs`: five offline tests cover split Flight chunks, distinct Mega identities/types/keys, anti-bot and malformed/truncated input, empty/partial data, duplicate identities/index payloads/keys, missing image/bounds, unsafe paths, invalid types, filtering and image-base drift. The grouped failure test contains 11 cases. Tests never use live networking.
- Checks passed: `cargo fmt --check`; `cargo test --locked` (5 tests); `cargo clippy --locked --all-targets -- -D warnings`. First clippy attempt found a complex test-case type annotation; a type alias fixed it and the full required checks passed afterward.
- Live replay: `cargo run --locked --example inspect_source -- /tmp/pokington-source/index.html /tmp/pokington-source/*.png`; separate SG/PH/IN HTML runs all gave the same counts and Tauros collision. No production command was changed; `src/main.rs` remains the original scaffold.
- Dependencies resolved from installed cache: `scraper` 0.29.0, `serde` 1.0.229, `serde_json` 1.0.154, and example-only `png` 0.18.1. The existing untracked Cargo.lock was updated through Cargo and preserved. No Rust tooling installation or network dependency download was necessary.

Structural checks can detect missing originals against source bounds, but they cannot prove that the source did not omit an alternate form while still returning valid data. The source supplies no independent total-form assertion in the inspected props. A missing alternate record, stale-but-consistent index, or globally incomplete source is therefore outside what these fixture checks prove. Completeness acceptance and descriptive-key enrichment remain separate unfinished work; no review, commit, source-policy change, S01.T02 work, or production bootstrap is included.

## 2026-11-06 continuation: main-site HTTP investigation

Later user direction resolves the earlier coverage question: “I want all the official ones can we jsut get pulling from the main website working?” The regional snapshot is therefore not adopted. The user subsequently declined computer control. The remaining investigation used ordinary HTTP and official public source inspection only; no further browser or native-app control was attempted by the implementation agent. The earlier observations above remain historical evidence, not a pending proposal to weaken coverage.

The primary site's English index, its trailing-slash variant, `/us/`, the previously identified candidate `/us/api/pokedex/kalos`, and English UK Dragonite returned HTTP 200 `text/html` with Incapsula failure/iframe bodies instead of catalogue content. Ordinary HTML/English content-negotiation headers did not fix that. A normal HTTP cookie session started from an accessible public site page and following its Pokédex link also returned the same failure. No session cookies are copied into documentation or repository fixtures.

The [main site's robots path](https://www.pokemon.com/robots.txt) returned an actual HTTP 404 Pokémon site shell rather than a robots file. That shell linked the accessible [official main.js](https://assets.pokemon.com/static2/_ui/js/main.js), configured `window.pokemonUI.site_search_api.url` as `https://api.search.prod.pokemon.com/v1/search`, and linked `/us/pokedex`. These were legitimate source discoveries, not assumed API contracts.

The current `main.js` search collection specifies GET JSON requests with `q`, `locale`, `offset`, `limit`, `sort`, and optional `filter`; its Pokédex category filter is `pkm-category:pokedex`. No required custom authentication header appears in that request code. A first query with `limit=20` returned a JSON validation error reporting that allowed limits are 5, 10, or 25. Subsequent supported requests all returned HTTP 200 `text/html` containing an Imperva “Pardon Our Interruption” page, not JSON:

- `q=dragonite&locale=us&offset=0&limit=25&sort=relevance&filter=pkm-category%3Apokedex`
- `q=dragonite&locale=us&offset=0&limit=25&sort=relevance`
- `q=dragonite&locale=us`
- `q=dragonite&locale=us&offset=0&limit=5&sort=relevance`

Inspection of the accessible generic main.js did not establish a Pokédex-specific metadata endpoint or full catalogue/form resource. Its search endpoint remains insufficient even if access is later restored: ordinary search results would still require completeness and form-metadata verification. No challenge JavaScript was executed and no challenge was solved.

Further source-discovered official routes also failed direct HTTP extraction: [unprefixed English index](https://www.pokemon.com/pokedex/), [Latias](https://www.pokemon.com/us/pokedex/latias), and [Golem](https://www.pokemon.com/us/pokedex/golem) each returned a 924–932-byte Incapsula body. Search-indexed official Spanish Dragonite text includes Mega Dragonite, but fresh raw retrieval failed too; it is evidence of main-site coverage, not an English data fallback or repeatable runtime source. Cached search content is not live bootstrap acceptance.

| Additional capture | UTC time | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `robots-response` (genuine 404 shell) | 22:36:03 | 15,096 | `6c06a36b923fe2400c3affcae28ec9c54c04aa150c0d1ad3eb13ee38c856f15e` |
| `us-main.js` | 22:36:22 | 859,715 | `b88ab85575d3219944740cc05e1486bc80d94d06079f8b3d2a32fe391814db95` |
| `us-search.json` (HTML, despite capture filename) | 22:37:30 | 5,996 | `394b0ac2faeeb9a42402a09b4df3b8253d454618655c96a22c0bef20efd68f24` |
| `us-search-minimal-response` | 22:38:28 | 5,996 | `ef9258611757497865458fd63843f0da55212246c78b888202515f94309d968c` |
| `new-route-0.html` (unprefixed index) | 22:38:39 | 928 | `f0af57b70a904906b70db19003bb7bab9b66b8d18b5e48bb4334c933adb31513` |

The clock read at 22:39:26Z followed these HTTP checks. The earlier Rust checks remain valid because the continuation changed only evidence documentation. No live main-site catalogue payload, complete form discovery, or unattended download solution has been established.

Minimum useful next evidence would be user-saved raw HTML source from a normally loaded public English Pokédex index, and optionally the English Dragonite detail page. Those may expose page-specific script/data URLs and form mappings that the generic 404 shell lacks. Cookies, HAR files, private headers, account content, and computer control are unnecessary. One manual capture would permit further source investigation; it would not by itself prove repeatable unattended bootstrap or authorise transferring browser session credentials.

Transport follow-up used installed curl 8.7.1 (SecureTransport / LibreSSL 3.3.6). Both its default HTTP/2 path and explicit `--http1.1` returned Incapsula HTML for the index. No `curl-impersonate` executable or Python `curl_cffi` client was installed; no browser-profile client was added or tested. This does not exhaust TLS-client compatibility possibilities. Restoring one response through an alternate client would still need reproducible Rust/runtime access and complete discovery evidence.

## 2026-11-06 correction: browser requests enable some main-site access

The prior failed-access observations remain valid for their request configurations. A later bounded experiment installed `curl_cffi` 0.17.0 only in `/private/tmp/pokington-http-check`, with registry cache in `/private/tmp/pokington-uv-cache`. Its standard Chrome profile obtained genuine main-index HTML (134,651 bytes, SHA-256 `f1f79815a3a6e2fc114d0f9bdebffb7d67894c6b66dee1ea096706dd01ddbebb`) and official search JSON (19,861 bytes). No browser process, imported cookies, credentials, proxies, or challenge solving was used. The root agent independently succeeded with ordinary curl and a complete Chrome 142 user-agent string; the shorter earlier `Mozilla/5.0` string had been insufficient. The user then expressly required browser impersonation by default for all data requests. The manual source capture became unnecessary.

The genuine main-index HTML supplies bounds 1–1,025, `window.pokemonUI.api["pokedex"] = "/api/1/us/kalos/"`, and `data-main="https://assets.pokemon.com/static2/_ui/js/pokedex.js"`. That [official script](https://assets.pokemon.com/static2/_ui/js/pokedex.js) initializes its collection URL as `this.url=this.urlBase+"kalos"`. Therefore the actual source-derived [catalogue endpoint](https://www.pokemon.com/api/1/us/kalos/kalos) is distinct from the previously guessed `/us/api/pokedex/kalos` route. It returned **1,025 original species records**, each with its actual `detailPageURL`, National number, English name and base-form metadata. This is complete species discovery against the page's bounds, not full form discovery.

Genuine [English Dragonite HTML](https://www.pokemon.com/us/pokedex/dragonite) contains a form selector with value 0 `Dragonite` and value 1 `Mega Dragonite`. Its `.profile-images` references distinct `149.png` and `149_f2.png` full PNGs. This is direct form evidence, separate from TCG card references. The [official detail script](https://assets.pokemon.com/static2/_ui/js/pokemon-detail.js) uses the same selector order for `.profile-images img` and `.pokedex-pokemon-attributes`, whose `.dtm-type` links supply per-form types.

`src/official_source.rs` now contains a draft main-site parser and shared reqwest client, with all requests using browser headers. `examples/fetch_official.rs` probes a supplied URL, and `examples/discover_official.rs` uses the source-derived index/API/detail links, at most four concurrent requests, 30-second timeouts, a 4 MiB response limit, and offline replay of captures. These are feasibility tools, not production bootstrap. Full main-site parser fixture coverage and form normalization remain unfinished.

Observed request comparisons:

| Request configuration | Result |
| --- | --- |
| reqwest 0.13.5, Chrome 142 user-agent, gzip | Genuine index, 1,025-species API and Bulbasaur; Ivysaur/Venusaur/Charmander return anti-bot HTML. A sequential Ivysaur retry also fails. |
| Ordinary curl, same Chrome 142 user-agent | Same known-blocked Ivysaur detail fails. |
| Temporary curl_cffi, standard Chrome profile, no imported state | Genuine Ivysaur HTML, 48,220 bytes. |
| Ordinary curl, exact noncredential headers observed from that Chrome 152 Windows profile | Genuine Ivysaur and Venusaur HTML, demonstrating that header changes alone can help some requests. |
| reqwest with those browser headers, initially gzip-only encoding negotiation | Genuine Ivysaur; full crawl stops at Venusaur. |
| reqwest with full matching `Accept-Encoding: gzip, deflate, br, zstd` and enabled decoders | Full discovery attempt fails at the index with a 5,871-byte anti-bot page, SHA-256 `7200e9df9639cb41a298d201c929a6d643b8dbaab073520982c54539bb9cfeca`. |

The full metadata crawl has **not passed**. Only the first four source-listed detail requests were attempted before validation stopped the first runs; the final attempt stopped at its index. No total main-site form count or main-site representative PNG decode is claimed yet. Temporary captures are in `/private/tmp/pokington-source/` and `/private/tmp/pokington-main-discovery/`; later attempted responses overwrite that evidence runner's same-named local files. Separate named captures above preserve the significant successful/failed comparisons.

The compression build initially encountered an unavailable zstd version in the cached registry index. Refreshing the registry resolved zstd 0.14.3; the temporary async-compression downgrade was reversed, leaving 0.4.52 and compression-codecs 0.4.47. No older-version workaround remains.

The bounded header comparison ended after the full reqwest attempt. A documented next option is [wreq 0.16.3](https://docs.rs/wreq/latest/wreq/) with [wreq-util 0.2.0](https://docs.rs/wreq-util/latest/wreq_util/), whose `Client::builder().emulation(...)` configures headers, TLS and HTTP behavior together. This changes the earlier reqwest transport choice and has not been installed, compiled, or adopted. Upstream documents Linux/macOS builds, BoringSSL prerequisites and MSRV 1.98; installed Rust 1.99.1 meets that version requirement. CMake, Perl, Clang and Git are present; `pkg-config` is not on PATH and complete native build prerequisites remain unverified. The root agent owns the pending transport decision and canonical permission record.

## 2026-11-06 continuation: approved Rust browser-profile client

The user approved Q06 at the parent agent's recorded time 22:53:31Z. This supersedes the preceding pending dependency proposal. `reqwest` was replaced by pinned `wreq` 0.16.3 and `wreq-util` 0.2.0; Tokio drives the async evidence examples. The shared `http_client()` applies an explicit browser profile for all requests, retains certificate verification, sets 10-second connect / 30-second total timeouts, and uses no browser process, imported cookies, proxy or challenge-solving code. Production bootstrap remains outside this task.

Native compilation succeeded with the installed tooling, without system packages or `sudo`. The macOS linker warns that two `btls-sys` search directories (`out/build/lib` and `out/build/crypto`) do not exist; linking nevertheless succeeds. This warning remains recorded rather than suppressed. Rust formatting, nine tests (four main-source tests and five earlier portal tests), and `cargo clippy --locked --all-targets -- -D warnings` all pass after the change. The clock observation after these checks was 22:58:37Z.

New main-source fixtures reduce genuine Dragonite form-selector/image/type markup and its source API row. The fixture's index bounds are deliberately narrowed to 149–149. Tests verify the real Mega Dragonite form, source-derived endpoint construction, duplicate/missing species IDs, malformed JSON and challenge HTML, source bounds, selector/image/type alignment, missing references, unsafe artwork, duplicate keys/artwork and mismatched identity. These tests prove parser behavior for those fixtures, not complete live coverage.

The approved HTTP client still has unresolved live access failures:

| Supported profile and configuration | Live outcome and retained capture |
| --- | --- |
| Chrome 147, default macOS platform | Genuine index/API, then Bulbasaur challenge; `/private/tmp/pokington-main-wreq/1.html`, 5,871 bytes, SHA-256 `9880f41b64d6a780a46775d133f8d38d15f398d1b643c3e5522f25ce4b361293`, captured 22:55:55Z |
| Chrome 149, Windows (newest profile bundled in wreq-util 0.2.0; closest available to successful curl_cffi Chrome 152 Windows) | Index challenge; `/private/tmp/pokington-main-wreq149/index.html`, 5,871 bytes, SHA-256 `a50b4adbb77ca2b2ba9ab2c6ebba467fedbe26eced5c47795a5f1a8644e92c98`, captured 22:56:38Z |
| Chrome 149 Windows, with `emulation-compression` enabled | Index challenge; `/private/tmp/pokington-main-wreq149-compression/index.html`, SHA-256 `7a4915eb85c35e52fdcb595086fd35804686277b4873d238084388302f522ea6`, captured 22:57:42Z |
| Chrome 147 macOS, with `emulation-compression` enabled | Index challenge; `/private/tmp/pokington-main-wreq147-compression/index.html`, SHA-256 `494e63b8eee981cd4e41a58af008d8948b717d9046ee7130d691a9cd181e7a6a`, captured 22:58:16Z |

Comparing the installed library's profile headers to the successful isolated client's noncredential headers found that `wreq-util` requires its separate `emulation-compression` feature to emit the browser-order `Accept-Encoding` header; enabling only wreq's decompression features is insufficient. That feature is now enabled, and gzip/deflate/Brotli/Zstandard decoding is enabled in wreq. The supported profile handles user-agent and all other default headers consistently; no additional manually mixed user-agent or client-hint overrides remain. Correcting compression did not make the tested profiles reliable. The current code retains the documented Chrome 147 macOS profile with that feature enabled, as an unfinished feasibility configuration.

No more profile adjustments were attempted after these bounded checks. All unsuccessful responses are rejected by parser validation. The main site's 1,025-species listing and actual Dragonite Mega metadata are established, but complete main-site form totals, all detail pages, collision-free main-site keys and representative main-site PNG validation remain unfinished. The task is not ready for acceptance review or commit.

Later session-semantics check: the previously successful temporary curl_cffi Chrome profile also began returning challenge HTML for both the index and Ivysaur. Each independent request began with zero cookies, followed zero redirects, and kept the same final URL. Each response supplied two anonymous Incapsula cookies; only their names/counts were inspected, never their values. A separate fresh same-session index → Ivysaur pair remained blocked even though Ivysaur inherited the two anonymous cookies received from the index. Earlier successful curl_cffi calls likewise used fresh sessions, so no preexisting browser state explains them. Installed wreq defaults disable redirects and cookie persistence, but the observed zero-redirect, cookie-continuity failures do not establish that those defaults cause the current block. No extra cookie behavior or transport dependency was added. Current evidence no longer isolates a Rust-client defect; changing source-side access conditions are possible but unproven.

## 2026-11-06 continuation: main-site artwork samples

Following the parent's unsuccessful quiet index retry, metadata fetching and profile tuning remained stopped. A separate bounded artwork check used only genuine detail HTML already captured locally: `main-dragonite.body` and `curl-header-venusaur.html`. The existing parser validated their identities, form-selector/image/type alignment and full PNG references against the already captured main index/API. No further detail pages or guessed artwork suffixes were fetched. The worktree remained on local `main` with the same uncommitted task files; no permission record, review or commit was changed.

The new `examples/inspect_artwork.rs` reads those local inputs, sequentially fetches each referenced form image through the shared wreq Chrome 147 macOS profile, and stops on the first HTTP, decoding or checksum failure. Each response is capped at 16 MiB; decoded output is capped at 64 MiB. It preserves the raw evidence response and saves the `.png` after a complete frame decode and `reader.finish()`. All five requests succeeded as `image/png`, with the final URL equal to the extracted URL. Each image is **475×475, RGBA, 8-bit**. Files and raw `.response` copies remain in `/private/tmp/pokington-main-artwork/`.

| Referenced form / main-site PNG | Bytes | Capture UTC | SHA-256 |
| --- | ---: | --- | --- |
| [Dragonite original](https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/149.png) | 132,064 | 23:05:27 | `4bd7549caa15eac43a8197ba53e843a95f2b56c71b1121829ea76764fbf26f9c` |
| [Mega Dragonite](https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/149_f2.png) | 194,867 | 23:05:28 | `5f66c50ae041c5747880d37f82de2176fbe0b1dc0ece63a2bb42e53ba39df2da` |
| [Venusaur original](https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/003.png) | 186,011 | 23:05:43 | `106d0040aa775f95871706c30ddd4abaef2c962215f5e8bd4141e0caef0ef8e4` |
| [Mega Venusaur](https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/003_f2.png) | 190,329 | 23:05:44 | `e067b4e2dadf5e661b659462ba8303d7c3fb195d4c400286aee8f9a8d4a5ad1b` |
| [Gigantamax Venusaur](https://www.pokemon.com/static-assets/content-assets/cms2/img/pokedex/full/003_f3.png) | 153,927 | 23:05:45 | `0979c862c632c20ee2d355136f990452b4740254b86061f00e7f721116bc0e6a` |

These two exact commands completed successfully, making five image requests in total:

```sh
target/debug/examples/inspect_artwork /private/tmp/pokington-source/tls-main.body /private/tmp/pokington-source/reqwest-api.json 149 /private/tmp/pokington-source/main-dragonite.body /private/tmp/pokington-main-artwork
target/debug/examples/inspect_artwork /private/tmp/pokington-source/tls-main.body /private/tmp/pokington-source/reqwest-api.json 3 /private/tmp/pokington-source/curl-header-venusaur.html /private/tmp/pokington-main-artwork
```

Checksum correction: the earlier regional PNG runs used png 0.18.1 defaults, which verify PNG CRC but skip zlib Adler32. The earlier description must not be interpreted as an explicit Adler32 check. Both inspection examples now call `decoder.ignore_checksums(false)` before reading, enabling both checks. The nine existing regional PNGs were revalidated offline under that stricter configuration and all passed; no regional network requests were repeated.

Offline corruption checks used separate copies of the captured original Dragonite PNG. Modifying only an IDAT CRC produced `CrcMismatch` and exit 1. Modifying the zlib checksum while recomputing the PNG chunk CRC produced `CorruptFlateStream { err: WrongChecksum }` and exit 1. Removing IEND produced `UnexpectedEof` and exit 1. Each check ran `target/debug/examples/inspect_source /private/tmp/pokington-source/index.html /private/tmp/pokington-main-artwork/<fixture>.png`; the fixtures are `invalid-crc.png`, `invalid-adler32.png`, and `missing-iend.png`. The original valid images remain unchanged.

Affected verification passed: `cargo build --locked --example inspect_artwork`, both live sample invocations, stricter offline validation of nine regional files, the three expected-failure corruption checks, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings`. The existing nine metadata tests were not rerun because this continuation changed only example image inspection and evidence; their prior result remains recorded. Native linker warnings remain as documented above. Final clock observation: 23:06:43Z.

This establishes main-site original/Mega/Gigantamax PNG feasibility through the approved Rust client independently of the blocked metadata endpoints. Main-site regional and distinct-Mega samples are still missing because no corresponding genuine detail captures were available. Complete main-site form discovery, reliable full metadata retrieval and source acceptance remain incomplete. No source-policy relaxation, review, commit or later task is included.

## 2026-11-06T23:08:37Z — Block hypotheses

- Observed: the saved challenge says JavaScript or cookies may be disabled, lists unusually fast browsing, and asks the visitor to enable cookies and JavaScript. These are generic listed causes, not proof of the particular rule triggered. The page contains Imperva/Incapsula markers. Five PNG requests succeeded while the latest HTML index requests were challenged.
- Leading hypothesis: missing JavaScript-generated verification/session state. HTTP browser profiles reproduce request/network properties but do not execute page JavaScript. Merely accepting anonymous response cookies did not restore access.
- Second hypothesis: changing behavioral or IP reputation scoring. Initially successful configurations later failed across clients and fresh sessions. That pattern fits shared source-side classification but does not establish an IP ban, request quota or the exact scoring signal.
- Third hypothesis: remaining browser fingerprint inconsistencies. Header/profile changes helped individual requests, but the previously successful client also later failed, so fingerprint mismatch alone does not explain the full timeline.
- Path-specific enforcement is plausible: HTML discovery is challenged while source-linked PNGs remain accessible. Asset caching or separate route policies could explain that split; no server configuration was inspected.
- Primary corroboration: [Imperva's explanation](https://community-cybersec.thalesgroup.com/blogs/brooks-cunningham1/2020/06/10/imperva-advanced-bot-mitigation-reports-and-polici) documents JavaScript/cookie validation and distinct behavior/IP reputation policies. It explains available mechanisms, not Pokémon's exact current rule. No CAPTCHA was solved, browser run, security setting changed or private session imported to investigate these hypotheses.
