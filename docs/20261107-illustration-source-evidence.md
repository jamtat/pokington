# Illustration sources — 2026-11-07

## Requirement correction

The user inspected the HOME renders and said, “I don't really like the way they look, i prefer the illustration style”. HOME renders are therefore not an accepted artwork target. The [earlier investigation](20261106-pokemondb-artwork-evidence.md) remains valid as technical evidence, but its complete HOME tier does not satisfy this visual preference.

**2026-11-07 continuation:** The initial bounded findings below were followed by a complete byte-level check of the exact Serebii candidate set. **All 1,025 original-species illustrations plus all 45 newer Mega candidates are valid native transparent PNGs with visible artwork.** See the final section for the completed results; its findings supersede the earlier link-only status and proposed validation step.

## Pokémon Database illustrations

An independent parse of the already captured species pages found **source-linked JPEG illustrations for all 1,025 original species**. The first artwork image on each detail page supplies the original candidate. Of these, 1,023 also expose a smaller AVIF display version; Pumpkaboo and Gourgeist expose standalone JPEG images without that AVIF alternative.

All **45 newer Mega forms missing from the HOME tier** have explicit illustration JPEG/AVIF links on their species detail pages. This also establishes a discovery distinction: the additional-artwork galleries can omit assets already present on the species pages. For example, [Raichu's species page](https://pokemondb.net/pokedex/raichu) links both newer Mega illustrations even though its additional-artwork gallery did not include them. No image filenames were guessed.

These are complete **source-link inventories**, not complete binary-download claims. Five representative Pokémon Database files were fetched through the existing browser-impersonating HTTP client and decoded with the installed ImageMagick; AVIF properties were also checked with `ffprobe` for Pecharunt.

| Explicit source image | HTTP type | Decoded dimensions | Alpha |
| --- | --- | --- | --- |
| [Large Charizard JPEG](https://img.pokemondb.net/artwork/large/charizard.jpg) | `image/jpeg` | 900×708 | None; opaque white background |
| [Large Pecharunt JPEG](https://img.pokemondb.net/artwork/large/pecharunt.jpg) | `image/jpeg` | 778×590 | None; opaque white background |
| [Pecharunt AVIF](https://img.pokemondb.net/artwork/avif/pecharunt.avif) | `image/avif` | 360×273 | None; opaque |
| [Large Mega Raichu X JPEG](https://img.pokemondb.net/artwork/large/raichu-mega-x.jpg) | `image/jpeg` | 453×544 | None; opaque white background |
| [Mega Raichu X AVIF](https://img.pokemondb.net/artwork/avif/raichu-mega-x.avif) | `image/avif` | 333×400 | None; opaque |

All five responses were HTTP 200. Charizard, Pecharunt and Mega Raichu X JPEGs were visually inspected: they have the requested full illustration style. The two tested AVIFs contain three colour channels, no alpha channel, and fewer pixels than their corresponding large JPEG. These samples disprove a universally transparent AVIF alternative; they do not establish alpha properties for every AVIF.

The earlier complete native-PNG illustration inventory still applies: Global Link illustrations cover **807 species**, with **218 missing from Meltan through Pecharunt**. Pokémon Database's examined source-linked assets therefore do not establish native PNG illustration coverage for every species.

## Bounded codec test

One Pecharunt JPEG was decoded and saved as a real PNG with the installed ImageMagick. This was a local codec test, not adoption of a conversion policy or a dataset conversion.

- Source JPEG: 75,550 bytes, 778×590.
- Result: `/tmp/pokemondb-investigation/pecharunt-codec-test.png`, 285,121 bytes, 778×590 RGB PNG.
- `magick compare -metric AE` reported **zero different decoded pixels** between the JPEG and PNG. Conversion preserved the decoded illustration exactly; it did not recover detail already lost to JPEG compression.
- The resulting PNG passed signature, CRC, Adler32, frame and stream-end decoding through `inspect_png`.
- It contains **zero transparent pixels**. Re-encoding does not remove the white background or make transparency-based trimming work on it.
- No background removal, image redraw, whole-dataset conversion or production codec dependency was introduced.

## Native illustration PNG alternative from cached Serebii pages

Without using earlier provider conclusions, the cached [Serebii national index](https://www.serebii.net/pokemon/nationalpokedex.shtml) and its cached species pages were parsed directly. The index independently enumerates **1,025 species**. Every corresponding hub page exposes an original illustration PNG through its `og:image` source reference. Cached pages also contain explicitly labelled artwork PNGs for **all 45 newer Mega names** absent from Pokémon Database's HOME tier.

This is a promising source-linked native illustration candidate, not yet complete verified image coverage. No new Serebii network fetches were made during this follow-up. Four already cached PNGs were independently decoded with checksums enabled:

| Source image | Decoded dimensions | Verified transparency |
| --- | --- | --- |
| [Pecharunt](https://www.serebii.net/pokemon/art/1025.png) | 475×475 RGBA | 146,381 fully transparent pixels |
| [Dragonite](https://www.serebii.net/pokemon/art/149.png) | 475×475 RGBA | 158,274 fully transparent pixels |
| [Mega Dragonite](https://www.serebii.net/pokemon/art/149-m.png) | 570×570 RGBA | 175,711 fully transparent pixels |
| [Mega Charizard X](https://www.serebii.net/pokemon/art/006-mx.png) | 475×475 RGBA | 140,520 fully transparent pixels |

All four passed full PNG CRC, Adler32, frame and stream-end checks. Pecharunt and Mega Dragonite were visually inspected and show full illustrations rather than HOME renders. Direct pixel checks confirm transparent corners for both. A momentary impression of a white border in the displayed Mega Dragonite preview was not supported by those file measurements and is not a reported image defect.

## Smallest next option

**Prefer validating Serebii's source-linked illustration PNGs for the 1,025 originals and 45 newer Mega forms before choosing a provider.** The cached evidence directly addresses the desired illustration style, native PNG format and real transparency without conversion. Complete HTTP/image validation remains necessary before claiming all those links are usable images. This investigation does not repeat an internal game-form/type audit or claim every alternate form is covered.

If Pokémon Database must remain the source, the smaller policy change is to download its source-linked JPEG illustrations and deterministically re-encode them as PNG. That preserves the illustration style and available resolution, but the user would need to accept **opaque white backgrounds**. The tested AVIF display versions do not solve this limitation. No such policy has been adopted.

## Artifacts and limits

- [Source inventory](logs/20261107-illustration-source-inventory.json): every Pokémon Database original JPEG/AVIF mapping, its 45 newer Mega mappings, and independently parsed cached Serebii original/newer-Mega PNG mappings.
- [Sample file hashes](logs/20261107-illustration-sample-hashes.json): exact local files, byte counts and SHA-256 hashes for the five fresh samples, one codec result and four cached native PNG samples.
- Fresh samples remain in `/tmp/pokemondb-investigation`; Serebii captures remain in `/tmp/pokington-serebii`.
- No production code was changed. No source adoption, whole-dataset conversion, background removal, review, commit, push or PR occurred.
- Final measurements and documentation were recorded on **2026-11-07 UTC**. All fresh data fetches used the existing browser-profile HTTP client; no browser control, credentials or challenge handling was used.

## Completed native illustration validation — 2026-11-07T00:15:56.802407Z

After the initial bounded report, the exact **1,070 candidate URLs** were downloaded through the shared Chrome/macOS HTTP profile with a three-request concurrency limit, a short delay before uncached requests and a separate local cache. No additional metadata crawl or provider expansion was performed. Network work stopped when that set finished.

Every candidate passed **HTTP 200, `image/png`, PNG signature, CRC, Adler32, full frame decoding and stream-end checks**. Every image has both visible pixels and fully transparent pixels. All are RGBA; none is opaque, fully transparent or uniformly blank. The smallest visible area is 24,452 pixels, and the smallest fully transparent area is 80,878 pixels. There are no duplicate image hashes in this set.

| Property | Complete measured result |
| --- | --- |
| Original species | 1,025 checked, zero missing or failed |
| Specifically requested newer Mega candidates | 45 checked, zero missing, ambiguous or failed |
| Valid native illustration PNGs | 1,070 |
| Opaque / fully transparent / uniform blank images | 0 / 0 / 0 |
| Dimensions | 1,006 at 475×475; 44 at 570×570; 17 at 450×450; 3 at 500×500 |
| Total image bytes | 140,023,048 |
| Finished | 2026-11-07T00:15:56.802407Z |

Original candidates come from each source species hub's original `og:image` portrait. Every newer Mega candidate has exactly one distinct source URL and an exact matching Mega label on the corresponding species hub; matching only an unrelated evolution page was insufficient. IDs and labels are preserved in the candidate manifest. This establishes original-source portraits and those 45 Mega forms, not a universal default-selection policy across providers or complete coverage of all other alternate forms.

Fresh Bulbasaur and Mega Raichu X images were visually inspected and show the requested illustration style. Fresh Pecharunt, Dragonite and Mega Dragonite files have identical SHA-256 hashes to the previously inspected sample files, tying those visual checks to the completed download set.

The image validator now also rejects fully transparent and uniform-solid PNGs. Two synthetic offline fixtures confirmed both rejection paths; no HTTP requests were made for those fixtures. The complete real-image run had no such failures. `cargo build --quiet --example probe_pokemondb` and `cargo clippy --quiet --example probe_pokemondb -- -D warnings` passed.

Files ready for inspection:

- [Native illustration folder](/private/tmp/pokington-illustration-validation/images/pokemon/art)
- [Bulbasaur](/private/tmp/pokington-illustration-validation/images/pokemon/art/001.png)
- [Pecharunt](/private/tmp/pokington-illustration-validation/images/pokemon/art/1025.png)
- [Mega Raichu X](/private/tmp/pokington-illustration-validation/images/pokemon/art/026-mx.png)
- [Mega Dragonite](/private/tmp/pokington-illustration-validation/images/pokemon/art/149-m.png)
- [Exact candidate identities and source links](logs/20261107-native-illustration-candidates.json)
- [Every HTTP/decode/alpha/dimension/hash result](logs/20261107-native-illustration-validation.json)
- [Completed summary](logs/20261107-native-illustration-summary.json)

The reused investigation example accepts `CACHE_DIRECTORY png-list CANDIDATE_JSON` for this bounded source-linked image list. It was extended only for the explicit Serebii host, file-list input and blank-image checks; production bootstrap remains unchanged.

**Recommendation after validation:** use these native illustration PNGs if the user accepts the Serebii source and inspected style. This avoids JPEG conversion and preserves real transparency. The Pokémon Database JPEG-to-PNG option remains technically feasible but retains white backgrounds. Provider adoption and coverage of any remaining alternate forms are separate decisions; no production policy, commit, push, PR or review was performed.
