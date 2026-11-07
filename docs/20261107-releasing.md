# Releasing Pokington

Date: 2026-11-07.

## Prepare and tag

1. Update the literal package version in `Cargo.toml` and the matching package entry in `Cargo.lock` through a reviewed PR. This workflow accepts stable `X.Y.Z` versions; prerelease tags are not supported yet.
2. Merge the approved changes and ensure current `main` CI succeeds. Fetch the current repository and check out the exact approved release commit.
3. Check the proposed tag locally: `node .github/scripts/release.mjs version v0.0.1`, replacing the example with the intended version.
4. With the required tag-push approval, create an annotated version tag and push that tag only:

```sh
git tag -a v0.0.1 -m 'Pokington v0.0.1'
git push origin v0.0.1
```

The tag push starts `.github/workflows/release.yml`. Invalid or mismatched tags fail before the expensive jobs. CI builds the tagged commit, rather than the later branch head. Never move a release tag to a different commit to recover a failed build.

## Builds and draft contents

The release workflow calls the same lint, helper-test and four-target build workflows used by normal CI. Each native job runs the complete nextest suite under the release profile, builds the executable, extracts its archive and checks help/version and CPU architecture. Linux jobs reject ELF interpreters and dynamic library dependencies. All jobs must succeed before the draft job runs.

| Target | Verification environment |
| --- | --- |
| `aarch64-apple-darwin` | macOS 15, ARM64 |
| `x86_64-apple-darwin` | macOS 15, Intel |
| `x86_64-unknown-linux-musl` | Ubuntu 24.04, x86-64, static musl |
| `aarch64-unknown-linux-musl` | Ubuntu 24.04, ARM64, static musl |

macOS C and Rust builds use the same deployment target, 11.0. CI runtime checks cover macOS 15; this does not establish runtime acceptance on every older macOS release. Binaries are not signed or notarised by this workflow.

The draft contains four `pokington-VERSION-TARGET.tar.gz` archives and `SHA256SUMS`. Each archive contains a versioned directory with `pokington` and `README.md`. Catalogue/artwork are downloaded separately by the existing bootstrap mechanism. Generated release notes include manual installation guidance and a source-commit marker. No installer, Homebrew formula or crates.io package is published.

The draft job requires the remote tag to resolve to the checked-out commit. It checks the uploaded asset inventory, downloads the assets again and verifies their checksums before reporting the draft ready. Only that job receives `contents: write`; other jobs use read-only repository permissions and checkout does not retain credentials.

## Review and publish

Open the draft after the entire release workflow succeeds. Check the four archives, checksums and release notes. Keep the source marker in the notes so retries can identify the draft. Publish manually through GitHub when satisfied. Do not publish while the workflow is still uploading or verifying files.

The workflow never publishes automatically. Once a release is published, the helper refuses to update it. Prepare a new version for corrections to published binaries.

## Failed runs and retries

- A lint, test, build or package failure prevents draft preparation. Inspect the failing job and correct the issue before the next release attempt.
- If draft creation or upload fails partway, an incomplete draft may remain. An approved rerun of the same tag/source commit can replace the expected generated files and complete its checks without overwriting hand-edited notes.
- An existing published release, unexpected draft assets, missing source marker or different recorded source commit causes failure before replacement. Reconcile the draft manually with explicit approval; do not bypass the check or delete a published release.
- Draft discovery uses authenticated, paginated release listings and exact tag matching. GitHub/API errors or duplicate matching releases fail the helper; they are not treated as permission to create another release.
- Manual workflow reruns require their own approval under the repository's working instructions. A rerun uses the original tagged workflow and source; changes to a broken tagged workflow need a newly approved version/commit/tag rather than moving the old tag.

## CI setup and speed

The checked-in Rust toolchain is selected directly with rustup. Rust dependency caches are separated by release target and lint mode, and include toolchain/environment and Cargo manifest/lockfile inputs through the pinned Rust cache action. Native C builds use per-target compiler variables. Nextest is restored from the Rust tool cache when the pinned version matches; otherwise a prebuilt binary is downloaded with source-build fallback disabled.

Linux setup caches a small user-owned musl sysroot using runner image version, architecture and setup-script hash. A cache miss updates apt metadata and downloads/extracts `musl` and `musl-dev`; it does not install them system-wide. Restores need no sudo. A cache hit regenerates local compiler paths without calling apt. Runner image changes deliberately invalidate both the sysroot and Linux dependency build cache so native objects refresh with their C environment.

The same build workflow serves CI and release tags, preventing separate implementations and allowing default-branch caches to warm release jobs. Jobs run in parallel, precompressed archives are uploaded without another compression pass, and intermediate workflow artifacts expire after seven days. GitHub Release assets are separate from those temporary artifacts.

The sysroot cache is saved immediately after successful setup, so a later build/test failure does not force another apt download. Native job retries replace their temporary workflow artifact; replacement of actual release assets remains restricted to the matching draft.

Before this change, one observed warm Linux CI run (`42397474494`) completed in about 32 seconds, including 10–12 seconds for Rust setup and 9 seconds for the test command. The new matrix provides more coverage; compare setup, compilation, tests, cache restore/save and total elapsed time separately. First-run native compilation and runner queues will dominate a cold run. No numerical speedup is claimed until comparable hosted evidence exists. Warm-cache setup is covered by a local test proving apt/sudo are skipped; real native cache/build outcomes belong in the implementation log.
