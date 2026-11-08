# Releasing Pokington

Date: 2026-11-07.

## Prepare and tag

1. Update the package version in `Cargo.toml` and let Cargo update the matching entry in `Cargo.lock` through a reviewed PR. Release versions use stable `X.Y.Z` numbers.
2. Merge the approved changes and ensure current `main` CI succeeds. Fetch the repository and check out the exact approved release commit.
3. Read the package version with `cargo metadata --no-deps --format-version 1 --locked | jq -r '.packages[] | select(.name == "pokington") | .version'`. The release tag must be that version prefixed with `v`, for example `v0.0.1`.
4. With the required tag-push approval, create an annotated version tag and push that tag only:

```sh
git tag -a v0.0.1 -m 'Pokington v0.0.1'
git push origin v0.0.1
```

The tag push starts `.github/workflows/release.yml`. Cargo metadata and `jq` validate the tag before lint and native builds run. All checkouts use the event's source commit. Never move a release tag to a different commit to recover a failed build.

## Builds and draft contents

The release workflow calls the same lint and four-target build workflows used by normal CI. Each native job runs `cargo test-ci --release --target TARGET`, preserving the complete application suite, locked dependencies and JUnit report. It then builds the release executable with locked dependencies, packages it with `tar`, and checks the extracted executable's help, version and CPU architecture. Linux jobs also reject ELF interpreters and dynamic library dependencies. Every lint, test, build and package check must succeed before draft preparation.

| Target | Native runner |
| --- | --- |
| `aarch64-apple-darwin` | `macos-15`, Apple Silicon |
| `x86_64-apple-darwin` | `macos-15-intel`, Intel |
| `x86_64-unknown-linux-musl` | `ubuntu-24.04`, x86-64 |
| `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm`, ARM64 |

Release binaries retain mimalloc. Linux jobs install `musl-tools` with apt and select the native `musl-gcc` compiler and linker through target-specific environment variables. macOS C and Rust builds use deployment target 11.0; runtime checks cover macOS 15 and do not establish acceptance on every older macOS release. Binaries are not signed or notarised by this workflow.

The draft contains four `pokington-VERSION-TARGET.tar.gz` archives, each with a matching `.tar.gz.sha256` file. Each archive contains a versioned directory with `pokington` and `README.md`. Standard `shasum -a 256` or `sha256sum` commands create and verify the checksums. Catalogue and artwork download separately through bootstrap.

Standard upload/download artifact actions transfer the archives and checksums between jobs. The final job verifies the downloaded checksums and refuses to change an already published release. A pinned `softprops/action-gh-release` action creates or updates the draft with generated release notes, the event tag and the event source commit. Only this final job receives `contents: write`; other jobs have read-only repository permissions, and checkouts do not retain credentials. No installer, Homebrew formula or crates.io package is published.

## Review and publish

Open the draft after the entire release workflow succeeds. Check all four archives, their checksum files and the generated release notes. Add any release-specific installation or migration guidance, then publish manually through GitHub when satisfied.

Download an archive and its checksum file into the same directory. On macOS, run `shasum -a 256 --check ARCHIVE.tar.gz.sha256`; on Linux, run `sha256sum --check ARCHIVE.tar.gz.sha256`. Continue only if verification succeeds, then follow the README installation instructions.

The workflow keeps the release as a draft. Once published, corrections require a new version.

## Failed runs and retries

- A lint, test, build or package failure prevents draft preparation. Inspect the failing job before another release attempt.
- If the final release action fails, an incomplete draft may remain. After an authorised rerun of the same tag and source commit, the release action updates the existing draft and replaces assets with matching names. Review its final contents before publication.
- Release notes and retry handling use the release action's supported behavior. Reconcile any unexpected draft contents manually before publication.
- Manual workflow reruns require their own approval under the repository's working instructions. A rerun uses the original tagged workflow and source; fixing a broken tagged workflow requires a newly approved version, commit and tag.

## CI setup

The workflows use ordinary Rust toolchain setup and dependency cache actions directly. Keep their Rust version aligned with `rust-toolchain.toml`. Cache scopes separate lint from each release target. Nextest is reused when the cached version matches the pin; otherwise the install action downloads a prebuilt binary with source-build fallback disabled.

Linux jobs run `sudo apt-get update` and `sudo apt-get install --yes musl-tools` on the native runner.

The shared build workflow serves normal CI and version tags. Jobs run in parallel, precompressed archives are uploaded without another compression pass, and temporary workflow artifacts expire after seven days. GitHub Release assets are separate from those artifacts. Hosted runs must confirm all four native builds and tests; local workflow validation alone cannot establish that acceptance.

2026-11-07 timing clarification: an earlier warm Linux CI run (`42397474494`) completed in about 32 seconds, including 10–12 seconds of Rust setup and 9 seconds for the test command. That observation predates this final workflow and does not establish its performance. Compare hosted setup, compilation, test and total times before claiming a speedup.
