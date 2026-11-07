# GitHub releases implementation plan

## Continuation prompt

```text
Use $jamtat-work to continue GitHub releases using docs/20261107-release-plan.md and docs/20261107-release-log.md. Read the design and canonical Run 01 in docs/logs/20261107-s06-release-log.md before acting. If scope is unspecified, recommend the next incomplete task. Preserve the approved btat-github-releases branch and existing work. Sxx.T00 rows are completion markers.

For the same open Run 01, honour its approval of S06.T01 and S07.T01, four specific fresh reviews, automatic coherent commits and stage continuation, the recorded branch push/PR grants, Diffle and the completion summon. Q03 records the additional explicit commit/push grant for the native CI test correction. Show commit subjects and file tables without asking again. Never infer additional review approval from this prompt: each approved review's scope/model/effort/count is fixed in the run. New runs or additional reviews require explicit approval under the user's AGENTS.md. Do not push a version tag, create/publish a real release, merge, start/rerun/cancel CI manually or run unapproved sudo commands. Record checks, deviations, review outcomes and permissions in separate stage logs. Keep native hosted acceptance distinct from local checks; do not mark missing evidence as passed. Stop after the requested PR, Diffle and summon, or at a material unresolved blocker.
```

Date: 2026-11-07. Status: S06 complete; S07 implementation/local validation/reviews complete, hosted acceptance pending publication.

## Scope and precedence

Current user instructions and [release decisions D01–D07](20261107-release-design.md) govern. The [brief](brief.md) and existing bootstrap/data/terminal contracts remain applicable. The [log index](20261107-release-log.md) links canonical run authority. Historical grants in other plans do not authorise this work.

Deliver a custom GitHub Actions release workflow for version `0.0.1` and future matching version tags, four native build/test targets, mimalloc in the executable, static Linux binaries, downloadable archives and SHA-256 checksums, and draft releases requiring manual publication. Build and test the exact tagged commit. Catalogue/artwork stay outside the archives and continue through bootstrap.

## Starting point and reuse

| Capability | Existing evidence | Work boundary | Owner |
| --- | --- | --- | --- |
| Application and version | `src/main.rs`, `Cargo.toml`, `Cargo.lock`; `7a5f6c6` sets `0.0.1` | Add allocator without changing command behavior or library ownership | S06.T01 |
| Rust lint/test CI | `.github/workflows/ci.yml`, `.cargo/config.toml`, `.config/nextest.toml` | Reuse pinned toolchain, nextest version/alias and test reporting; share platform jobs with release execution | S07.T01 |
| Test fixtures and CLI coverage | `tests/`, especially executable checks in `tests/command.rs` | Run existing suite under each release target; avoid allocator tests that mirror the declaration | Both |
| Packaging and release | No release workflow or packaging scripts | Add repository-specific shell helpers and workflows, not a generic release framework | S07.T01 |
| User installation | README currently documents source builds | Add archive/checksum installation and maintainer release instructions | S07.T01 |

## Target matrix and feasibility

| Target | Native standard runner proposal | Extra build requirement |
| --- | --- | --- |
| `aarch64-apple-darwin` | `macos-15` | Native C compiler for mimalloc |
| `x86_64-apple-darwin` | `macos-15-intel` | Native C compiler for mimalloc |
| `x86_64-unknown-linux-musl` | `ubuntu-24.04` | musl C compiler/sysroot and static linkage |
| `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` | musl C compiler/sysroot and static linkage |

Check concrete runner/tool availability during implementation. Do not claim support for older macOS versions merely because a deployment target compiles. The local machine is macOS ARM64; Docker is installed but its daemon was unavailable during preparation. Hosted CI is necessary for the other native environments. Native dependency compatibility, especially TLS/compression/allocator C code, is an early S07 feasibility gate.

## S06 — Quilava: integrate mimalloc

<a id="s06-t00"></a>
Stage prerequisites: approved branch and run. Start with allocator wiring before release automation so build failures can be attributed cleanly.

<a id="s06-t01"></a>
### S06.T01 — Vigoroth: use mimalloc in the executable

- Work: add the current compatible `mimalloc` crate to `Cargo.toml`, update `Cargo.lock` and declare its Rust global allocator in `src/main.rs`. Keep allocator ownership in the executable and retain existing CLI behavior. No performance improvement is asserted without measurement.
- Verify: `cargo fmt --check`; `cargo check --all-targets --all-features --locked`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; `cargo test-ci`; build the release executable and run its offline help/version paths. Do not run pre-commit hooks separately; normal commit hooks still run.
- Reviews: one fresh `gpt-6.1-sol` correctness reviewer at `high`, then one separate fresh `gpt-6.1-sol` simplification reviewer at `medium`, under the specific Run 01 approval.
- Evidence: local check outputs, dependency/allocator review findings and dispositions, commit subject/hash and log updates. Four-target runtime acceptance belongs to S07.

Stage exit: local checks and both approved reviews pass, resulting changes are verified and committed, and S06 evidence is recorded. Report the boundary and continue automatically under Run 01.

## S07 — Xurkitree: build and prepare GitHub releases

<a id="s07-t00"></a>
Stage prerequisites: S06 complete; release design decisions settled. Hosted acceptance requires the authorised branch push/PR; local checks do not replace it.

<a id="s07-t01"></a>
### S07.T01 — Dratini: implement and verify the release pipeline

- Build/test: add a shared four-target workflow used by normal CI and tag releases. Use native runners, the pinned repository toolchain and `cargo test-ci --release --target TARGET`, keeping `--all-targets --all-features --locked` through the alias. Build `pokington` with the release profile and exact target. Reuse existing fixture tests; no live catalogue bootstrap is required in CI.
- Package: produce `pokington-VERSION-TARGET.tar.gz`, containing a versioned directory with executable and README, plus SHA-256 checksums. Extract each archive to a fresh temporary directory and execute the extracted binary's help/version commands. Verify Linux ELF architecture, absence of dynamic dependencies/interpreter, and expected version. Do not package catalogue/artwork or build directories.
- Release: trigger on version tag push, validate tag syntax and exact package/lockfile version before expensive builds, and check out the event's commit. Generate release notes and prepare a draft only after lint, all target tests and package checks pass. PR/main runs build the same assets without GitHub release writes. Use read-only default permissions and `contents: write` only for the tag-only draft job.
- Recovery: serialize runs for a tag. Require an existing remote tag; never silently create/move it. Permit retries against an existing draft for the same tag and source commit, replace only expected generated assets, retain draft state and edited notes, and refuse any mutation of an already published release. Treat incomplete uploads as a failed run; verify final asset names/checksums before reporting the draft ready. Manual CI reruns still need separate user approval.
- Speed investigation: inspect runner preinstalled tools, cache policy and job timings. Avoid duplicate builds between CI/release workflows, scope Rust caches by target/toolchain/lockfile, reuse nextest binaries, and avoid repeated apt work on warm musl-toolchain cache hits. Keep cold setup small and invalidate tool caches when the runner image or setup changes. Record observed setup/build/test/cache times from the PR; do not claim an unmeasured speedup or add custom runner infrastructure without evidence.
- Documentation: update README installation/development sections and add a dated maintainer release guide covering version/tag consistency, checksums, manual installation/publication, supported/tested environments, failed/incomplete drafts and protected published releases.
- Verification: required Rust checks and release executable smoke checks; `actionlint`, shell syntax checks and ShellCheck for new shell code. Add meaningful local failure-path tests for tag/version mismatch, archive/checksum validation and mocked draft-release interactions, including retry and published-release refusal. Use synthetic fixtures for external API behavior, label them as such, and inspect the real PR's automatically triggered native matrix results after publication.
- Reviews: one fresh `gpt-6.1-sol` correctness reviewer at `high`, then one separate fresh `gpt-6.1-sol` simplification reviewer at `high`, covering the exact scopes in Run 01. Resolve correctness findings and validate changes before the final implementation commit.
- Evidence: workflow/script checks, local executable results, four reviewers' relevant outcomes, commits, automatic PR CI results and timing observations, PR URL and Diffle/summon outcomes. Native CI failures leave acceptance incomplete; diagnose without weakening checks. No actual version tag or release will be created to validate this PR.

Stage exit: implementation/checks/reviews complete and committed, branch pushed, PR opened, and real native CI outcome recorded. Run 01 Q03 grants the additional native CI correction commit/push; manual CI reruns, extra reviews and release publication retain separate approval. Diffle and completion summon follow the final PR content.

## Requirements and failure coverage

| Requirement | Source | Task | Acceptance |
| --- | --- | --- | --- |
| mimalloc on all four targets | D05 | S06.T01, S07.T01 | Binary wiring; local checks; native target compilation and full suite |
| Four native targets, static Linux | D01, D04, D07 | S07.T01 | Native test jobs; ELF inspection and extracted executable runs |
| Custom workflow, exact source/version | D02, D03 | S07.T01 | Shared build workflow; tag/version rejection tests; pinned commit checkout |
| Archives and SHA-256 only | D06 | S07.T01 | Exact asset inventory, checksum checks and installation docs |
| Draft only, safe recovery | D03 | S07.T01 | Gate dependencies; mocked new/retry/published/error cases; manual publish docs |
| Fast CI investigation | Run 01 user extension | S07.T01 | Cache/setup rationale and actual PR job timing evidence |
| Branch, commits, PR, Diffle, summon | Run 01 | Both | Commit hooks; authorised push; PR and viewer/report results |

Test malformed/mismatched versions, missing/wrong/duplicate assets, corrupt checksums, wrong executable version, dynamic Linux linkage, failed target jobs and API failures. Native network/TLS data flows remain covered by local HTTP fixtures; no live source availability is made a release gate. macOS signing/notarisation, installers, Homebrew, Windows and allocator benchmarking are excluded from this release scope.

## Assignments and tracking

Implementation: primary agent; no implementation delegation. Reviews: `gpt-6.1-sol` at the stated effort, one fresh reviewer for each of the four explicitly approved reviews. Correctness checks platform and release behavior; simplification checks unnecessary code and duplication. No extra stage reviewer is required. Additional or repeat review assignments need specific user approval.

Done remains blank until checks and required reviews pass. S07 hosted acceptance is recorded after the PR's automatic CI; an untested workflow is not completed acceptance. All historical permission restrictions remain unless explicitly superseded by Run 01.

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S06.T00](#s06-t00) | ✅ | Quilava: allocator stage completion | — | Both approved T01 reviews passed | Small isolated prerequisite | [S06 evidence](logs/20261107-s06-release-log.md); task commit follows |
| [S06.T01](#s06-t01) | ✅ | Vigoroth: executable allocator | Primary agent | Sol high correctness and Sol medium simplification: no findings | Verify ownership and keep integration minimal | [108 local tests and checks passed](logs/20261107-s06-release-log.md); native release evidence remains S07 |
| [S07.T00](#s07-t00) |  | Xurkitree: release stage completion | — | T01 reviews complete; hosted acceptance pending | One integrated pipeline and evidence | [S07 evidence](logs/20261107-s07-release-log.md); final hosted outcome recorded in PR checks/description |
| [S07.T01](#s07-t01) |  | Dratini: release pipeline and CI setup | Primary agent | Sol high correctness: one P1 resolved/tested; Sol high simplification: no findings | Matrix, external writes and recovery need detailed review | [111 local release tests, ten helper tests and static checks pass](logs/20261107-s07-release-log.md); other native targets await PR CI |
