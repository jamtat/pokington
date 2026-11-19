# Releasing Pokington

Date: 2026-11-07.

2026-11-08 correction: the manual Actions flow below supersedes the original version-PR and tag-push procedure. The accepted history remains in release decisions D03 and D09–D13. The workflow prepares a draft; publication stays manual.

2026-11-08 credential correction: the user dropped the synchronized current-version label to avoid a PAT or GitHub App. Release preparation now changes only Cargo version files and uses the built-in `GITHUB_TOKEN`; the former `RELEASE_TOKEN` setup is superseded. See D14 in the release design.

2026-11-19 branch protection correction. Merge version bumps through a PR that passes the required CI checks. The Release workflow now validates and builds the version already on `main`. This supersedes automatic version commits described in the earlier corrections. See D15 in the release design.

## Authentication

GitHub supplies `GITHUB_TOKEN` automatically. No PAT, GitHub App or custom repository secret is required. The final draft job requests `contents: write` for release tags and draft assets. Version validation, checks and builds use read-only permissions.

## Prepare a release

1. Merge the changes you want to release into `main`. Update `Cargo.toml` and the matching Pokington entry in `Cargo.lock` through a PR and let its required CI checks pass before merging.
2. Open **Actions → Release → Run workflow**, selecting the `main` branch.
3. Enter the version in [Cargo.toml on main](https://github.com/jamtat/pokington/blob/main/Cargo.toml), such as `0.0.2`, without a `v` prefix. Versions must be stable `X.Y.Z` numbers without leading zeros and newer than every published stable version. To replace an unpublished draft, enter the same version while it is still current on `main`.
4. Click **Run workflow**. The workflow validates the version on the latest `main` commit at checkout, then tests and builds that exact commit on all four platforms.
5. After the checks pass, the workflow creates or updates the annotated tag and draft release. Open the draft link in the workflow summary and review it before publishing.

Run the workflow manually after the version PR merges. Concurrent release preparation runs are serialized.

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

Standard upload/download artifact actions transfer the archives and checksums between jobs. The final job verifies the downloaded checksums and rechecks the release state before writing the tag or draft. A pinned `softprops/action-gh-release` action creates or updates the draft for the prepared commit with matching generated assets. New drafts get generated release notes; rebuilding a draft retains its existing notes. The built-in workflow token has read-only permissions except in preparation and the final draft job. Checkouts do not retain credentials. No installer, Homebrew formula or crates.io package is published.

## Review and publish

Open the draft after the entire release workflow succeeds. Check all four archives, their checksum files and the generated release notes. Add any release-specific installation or migration guidance, then click **Publish release** in GitHub when satisfied. This is the manual approval step; the action never publishes automatically.

Download an archive and its checksum file into the same directory. On macOS, run `shasum -a 256 --check ARCHIVE.tar.gz.sha256`; on Linux, run `sha256sum --check ARCHIVE.tar.gz.sha256`. Continue only if verification succeeds, then follow the README installation instructions.

The workflow keeps the release as a draft. Once published, corrections require a new version. Wait for an in-progress rebuild to finish before publishing: Git tag updates and release asset uploads are separate operations, and the final release-state check cannot make them atomic with a manual publication.

## Failed runs and retries

- A lint, test, build or package failure stops the workflow before tag or draft updates. Inspect the failure, merge fixes and start a new **Release** run using the same version on `main`.
- An existing draft rebuild uses latest main when its version matches the requested version, moves that draft's version tag to the successfully tested commit and replaces matching archives/checksums. Edited release notes remain.
- If draft creation fails after a new tag was pushed, starting again can reuse that tag only if it still points to the exact candidate commit. An existing tag pointing elsewhere without a matching draft requires manual recovery; the workflow refuses to overwrite it.
- A changed draft identity or a concurrent change to the tag fails the run. A tag update uses an explicit lease against the initially observed tag object. Published versions and versions at or below the highest published stable release are refused both during preparation and before final writes.
- If asset upload fails, an incomplete draft may remain. Start another run for that draft version, then review all eight expected assets and notes before publication. Reconcile unexpected extra draft contents manually.
- A new **Run workflow** invocation uses the workflow on main and latest main for its source. GitHub's **Re-run jobs** uses the old run's workflow definition; prefer a new run after changing workflow code. When an agent performs a manual dispatch or rerun, it still needs explicit approval under the repository's working instructions.

## CI setup

The workflows use ordinary Rust toolchain setup and dependency cache actions directly. Keep their Rust version aligned with `rust-toolchain.toml`. Cache scopes separate lint from each release target. Nextest is reused when the cached version matches the pin; otherwise the install action downloads a prebuilt binary with source-build fallback disabled.

Linux jobs run `sudo apt-get update` and `sudo apt-get install --yes musl-tools` on the native runner.

The shared build workflow serves normal CI and manual release preparation. Jobs run in parallel, precompressed archives are uploaded without another compression pass, and temporary workflow artifacts expire after seven days. GitHub Release assets are separate from those artifacts. Hosted runs must confirm all four native builds and tests; local workflow validation alone cannot establish that acceptance.

2026-11-07 timing clarification: an earlier warm Linux CI run (`42397474494`) completed in about 32 seconds, including 10–12 seconds of Rust setup and 9 seconds for the test command. That observation predates this final workflow and does not establish its performance. Compare hosted setup, compilation, test and total times before claiming a speedup.
