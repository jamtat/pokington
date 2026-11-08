# GitHub releases implementation log — S07

Plan: [Implementation plan](../20261107-release-plan.md). Index: [Implementation log](../20261107-release-log.md). Canonical authority: [Run 01](20261107-s06-release-log.md#run-01--s06t01-and-s07t01).

## S07.T00 — Xurkitree: stage timing and exit

- Start: 2026-11-07T18:06:30Z.
- Finish:
- Prerequisite: S06 committed as `db18507`; all hooks passed.
- Exit evidence: pending implementation, local checks, the two approved S07 reviews and automatic native PR CI.

## S07.T01 — Dratini: release pipeline and CI speed

- Start: 2026-11-07T18:06:30Z.
- Finish:
- Scope and permissions: canonical Run 01; no actual version tag or release will be created.
- Baseline: CI run `42397474494` succeeded in about 32 seconds end to end. Rust test job setup action took 10 seconds, nextest installation 1 second, tests including incremental compilation 9 seconds; lint setup took 12 seconds, cargo check 3 seconds and Clippy 3 seconds. These are one warm run's observations, not universal timings.
- Investigation: Ubuntu 24.04's published runner toolset does not include musl-tools. Existing Rust setup already enables dependency caching but also runs `brew install bash` on macOS. Prefer existing rustup with a direct pinned Rust cache action, prebuilt/cached nextest, and a small user-owned cached musl sysroot keyed by runner image, architecture and setup script. Warm musl setup must not call apt; avoid custom container/runner infrastructure until measurements justify it.
- Checks, review findings, commits and hosted acceptance: pending.

## Local implementation and review evidence — 2026-11-07T18:22:21Z

- Implemented shared native release builds/full tests, direct rustup and Rust caching, version/lock/tag validation, extracted archive checks, static ELF checks, checksums, draft creation/retry and user/maintainer documentation. Normal PR/main CI builds the same archives as tags, without release-write permissions.
- Musl setup refinement: download/extract only `musl` and `musl-dev` on cache misses, using the approved `sudo apt-get update`. The approved system-wide install command was unnecessary and is not used. Save the prepared user-owned sysroot immediately; warm setup skips apt and sudo. Cache keys include architecture, image version and setup hash; native Rust dependency caches change with the musl environment. This avoids custom container/runner infrastructure.
- Native local evidence on the rebased source: `MACOSX_DEPLOYMENT_TARGET=11.0 cargo test-ci --release --target aarch64-apple-darwin` passed 111/111 tests with no skips; clean per-target dependency compilation took 36.40 seconds and tests 4.33 seconds. `cargo build --release --locked --bin pokington --target aarch64-apple-darwin` reused that build in 0.08 seconds. Real archive extraction, help/version and Mach-O architecture checks passed. Setting the deployment environment consistently avoided the earlier cached-object warning.
- Helper evidence: ten dependency-free Node tests pass, covering version/tag/lock mismatch, missing/unexpected/symlink/corrupt assets, paginated draft discovery/retry, preserved notes, published/unrelated release refusal, duplicate matches, API/upload/download failures, cache-hit setup without apt, wrong binary version/architecture and dynamic ELF rejection. Mocked API/ELF behavior is not live GitHub/Linux acceptance.
- `actionlint` 1.7.12, `bash -n`, ShellCheck and `git diff --check` passed. Final commit hooks will repeat the ordinary Rust checks against the rebased source.
- Correctness review: `/root/s07_correctness`, one `gpt-6.1-sol` high reviewer, found one P1: the tag endpoint only promises published releases and can omit drafts. Replaced it with authenticated paginated list-releases plus exact-tag and duplicate checks. The mock now enforces that endpoint contract; retry, pagination, duplicate and API-error tests pass. No other actionable findings. Source: [GitHub REST release API](https://docs.github.com/en/rest/releases/releases).
- Separate simplification review: `/root/s07_simplification`, one fresh `gpt-6.1-sol` high reviewer, no findings. Shared workflows/actions remove meaningful duplication; asset checks protect distinct boundaries. After that review, executable inspection was moved unchanged into a named function for negative-path tests, and workflow artifact overwrite/cache-save settings were tightened for failed-run recovery; local checks were repeated.
- Native hosted acceptance remains pending the authorised final push and PR. No additional independent review or manual CI action has been launched. The final PR check results and PR description will record hosted evidence and run closure without a bookkeeping-only additional push.

## Native CI continuation — 2026-11-07T18:29:49Z

- PR: [#4](https://github.com/jamtat/pokington/pull/4), source `1563945`. Diffle opened at `http://127.0.0.1:4966/`; its optional language servers were unavailable for some languages, but the diff viewer started successfully.
- First native run [42401117347](https://github.com/jamtat/pokington/actions/runs/42401117347): lint, helper tests and both macOS native test/package jobs passed. Both Linux targets compiled and ran all 111 tests; each had 110 passes and one failure in the existing terminal-restoration test.
- Diagnosis: `nix::Termios` equality includes libc storage beyond the initialized Linux kernel terminal settings. The failing values had identical meaningful flags/speeds and differing unused trailing control storage. The test now obtains terminal settings through the existing rustix API and compares every defined mode, special control code, speed and Linux line discipline. PENDIN remains the existing documented exception. No test was skipped or assertion on a meaningful setting removed.
- Local fix validation: all 111 native ARM64 macOS release tests passed in 4.32 seconds after the fix. Commit hooks will validate the ordinary profile as well. No extra reviewer or manual CI rerun was launched.
- Cold setup evidence: Linux sysroot preparation succeeded in 5 seconds on x86-64 and 7 seconds on ARM64, and both caches were saved before compilation. The next ordinary push will also exercise cached setup under the same PR cache scope.
- The first correction commit attempt passed formatting/check/tests but Clippy rejected the expanded test's 123-line function. Extracted the same defined-setting comparisons into `assert_terminal_restored`; no lint was disabled. The user then explicitly instructed “commit and push now please”; canonical Q03 records that grant.

## Dated continuation — requested rebase

- User instruction: “rebase branch on current origin main please”.
- Fetched `origin/main` at `8418404` (`art: support species exclusions (#3)`) and rebased the two branch commits with tracked in-progress release edits preserved by autostash; untracked release files did not overlap upstream paths. Rebase and autostash restoration succeeded without conflicts.
- Historical commit correction: original version commit `7a5f6c6` is now `e5b0e24`; original allocator commit `db18507` is now `ded2ba5`. Earlier records describe their original hashes, not current branch identities. Full checks are being repeated on the new base.
