# HTTP dependency cleanup

Date: 2026-11-07. Decision: [D34](20261106-design.md#d34--remove-browser-impersonation-and-retain-compression--2026-11-07).

- Replace `wreq` and `wreq-util` with `reqwest` 0.13.5, default features disabled, using Rustls TLS. Retain gzip, Brotli, deflate and Zstd as explicitly requested to reduce source bandwidth.
- Remove direct `http-body-util` use. The shared `source::read_limited` reads chunks and rejects a body exceeding its limit before appending the offending chunk. Limits apply to decompressed bytes regardless of Content-Length or transfer framing. Empty-body rejection remains the bootstrap caller's responsibility.
- Preserve the production HTTPS restriction, no-proxy policy, connect/request timeouts, redirect limit, status/MIME/origin validation, concurrency, retries and store recovery/publication behavior.
- Update historical examples to compile against the same client; their previous provider acceptance is historical evidence, not a claim that those providers still accept unprofiled requests.
- Disable unused `scraper` defaults, including its bundled command-line program and parser error collection. Retain all other direct dependencies.

| Dependency measure | Before | After |
| --- | ---: | ---: |
| Direct runtime dependencies | 14 | 12 |
| Direct development dependencies | 2 | 2 |
| Unique selected macOS build packages, including normal/build/dev dependencies and excluding Pokington | 257 | 241 |
| Cargo.lock packages across all platforms, including Pokington | 290 | 303 |

The selected macOS build loses 38 packages and gains 22, for a net reduction of 16. Removed packages include browser profiles, btls/BoringSSL bindings, bindgen/Clang tooling, the duplicate Brotli/Zstd versions and getopts. Rustls uses AWS-LC, which still has a native build; this change does not eliminate native compilation. Reqwest's Rustls feature uses platform certificate verification, replacing wreq's bundled WebPKI roots. The all-platform lockfile grows because the replacement resolves platform-specific TLS dependencies and additional optional packages; lockfile size is not the selected build's package count. Earlier metadata-graph measurements included packages not selected for the build; the table uses `cargo tree --target aarch64-apple-darwin -e normal,build,dev --prefix none --format '{p}'`, deduplicated by package/version, for both the original and changed manifests/lockfiles.

## Verification

- `cargo check --offline --all-targets`: passed after dependency resolution.
- `cargo test --locked --offline --all-targets --all-features`: 106 tests passed, including the existing bootstrap concurrency, retry, cancellation, recovery and publication checks.
- New HTTP tests cover fixed-length, chunked and close-delimited bodies at/exceeding the byte limit; truncated responses; empty bodies with a zero limit; and advertisement, decompression and decompressed-size enforcement for all four compression formats. The focused tests passed again after test-helper Clippy fixes.
- `cargo clippy --locked --offline --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `git diff --check`: passed. Fixture tests required execution outside the default sandbox because it denies loopback socket binding; no tests were skipped.
- Before implementation, ordinary curl requests without browser headers returned genuine Serebii index, species and generation HTML plus original/Mega artwork. Both PNGs passed full checksum/frame/stream validation. These five requests were only preliminary transport evidence.
- `target/debug/examples/bootstrap_data /private/tmp/pokington-reqwest-validation.HgBqXN`: passed with the unchanged default concurrency of four, yielding 1,025 species, 1,427 forms and 11 excluded illustrations. All 1,427 PNGs were newly downloaded (181,677,334 stored bytes); the log records 2,541 total download attempts including metadata requests. Log: `/private/tmp/pokington-reqwest-bootstrap.log`. Temporary artifacts are local evidence, not durable repository fixtures. The user's installed data was untouched.
- `target/debug/examples/bootstrap_data /private/tmp/pokington-reqwest-validation.HgBqXN --offline`: passed, reloading the published catalogue and fully validating every stored artwork PNG without network access. Output: `Offline data verified: 1025 species, 1427 forms`.

No commit, push or independent review agent was requested or performed. Verification in this change is on macOS; no new Linux runtime evidence is claimed.

2026-11-07 commit authorization: after implementation and verification, the user requested “commit please”. This supersedes the preceding no-commit status and authorizes one local commit, `http: replace browser impersonation with reqwest`, with normal commit hooks. No push or independent review agent is authorized.
