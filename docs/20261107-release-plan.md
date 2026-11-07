# GitHub releases implementation plan

## Continuation prompt

```text
Use $jamtat-work to continue GitHub releases using docs/20261107-release-plan.md and docs/20261107-release-log.md. Read D08 and current Run 02 in docs/logs/20261107-s08-release-log.md; earlier S06/S07 permissions are historical. If scope is unspecified, recommend the next incomplete S08 task. Preserve btat-github-releases and existing work. Sxx.T00 rows are completion markers.

S08.T01 is explicitly delegated to one gpt-6.1-sol agent at xhigh, with a matching concrete goal. Keep the implementation exactly within D08: ordinary inline commands, stock artifact actions and one final softprops draft job; delete the custom programs and helper tests. Do not launch review agents or infer new commit/push/manual-CI/release permission. Complete authorised local work/checks before presenting the concrete commit and publication proposal. Record evidence in the separate S08 log. Distinguish old passing CI from new local checks and future hosted acceptance.
```

Date: 2026-11-07. Status: S06/S07 completed; S08 local implementation/checks complete, commit/push and whole-PR Diffle approved; new hosted acceptance pending.

## Scope and precedence

Current user instructions and [release decisions D01–D08](20261107-release-design.md) govern, with D08/S08 superseding the custom helper implementation described historically below. The [brief](brief.md) and existing bootstrap/data/terminal contracts remain applicable. The [log index](20261107-release-log.md) links canonical run authority. Historical grants in other plans do not authorise this work.

Deliver a custom GitHub Actions release workflow for version `0.0.1` and future matching version tags, four native build/test targets, mimalloc in the executable, static Linux binaries, downloadable archives and SHA-256 checksums, and draft releases requiring manual publication. Build and test the exact tagged commit. Catalogue/artwork stay outside the archives and continue through bootstrap.

## Initial S06/S07 starting point and reuse

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

## S06 — Quilava: integrate mimalloc (completed history)

<a id="s06-t00"></a>
Stage prerequisites: approved branch and run. Start with allocator wiring before release automation so build failures can be attributed cleanly.

<a id="s06-t01"></a>
### S06.T01 — Vigoroth: use mimalloc in the executable

- Work: add the current compatible `mimalloc` crate to `Cargo.toml`, update `Cargo.lock` and declare its Rust global allocator in `src/main.rs`. Keep allocator ownership in the executable and retain existing CLI behavior. No performance improvement is asserted without measurement.
- Verify: `cargo fmt --check`; `cargo check --all-targets --all-features --locked`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; `cargo test-ci`; build the release executable and run its offline help/version paths. Do not run pre-commit hooks separately; normal commit hooks still run.
- Reviews: one fresh `gpt-6.1-sol` correctness reviewer at `high`, then one separate fresh `gpt-6.1-sol` simplification reviewer at `medium`, under the specific Run 01 approval.
- Evidence: local check outputs, dependency/allocator review findings and dispositions, commit subject/hash and log updates. Four-target runtime acceptance belongs to S07.

Stage exit: local checks and both approved reviews pass, resulting changes are verified and committed, and S06 evidence is recorded. Report the boundary and continue automatically under Run 01.

## S07 — Xurkitree: build and prepare GitHub releases (completed history; helper implementation superseded)

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

## S08 — Hypno: simplify releases to the researched pattern

<a id="s08-t00"></a>
Prerequisites: D08 and the explicit user delegation; existing branch at `d314459`. S06/S07's native CI passed before this change.

<a id="s08-t01"></a>
### S08.T01 — Infernape: remove custom release automation

- Implementation: one `gpt-6.1-sol` agent at `xhigh`; parent independently checks scope and local evidence without launching a review agent. Both have concrete goals matching D08.
- Work: modify existing CI/build/release workflows and installation/release docs. Delete the three `.github/scripts` files and local Rust action. Keep four native full application-test targets, mimalloc and the terminal-test correction unchanged. Use standard compiler/cache/artifact actions and short inline archive/checksum/smoke/ELF commands, then one final softprops draft job.
- Boundaries: no new scripts, helper tests, xtask, local composite action, automation program or generated workflow framework. No custom API discovery/repair or relocated sysroot. Use per-archive checksums. Historical records stay factual and carry D08's explicit correction.
- Verify: `actionlint`, inline shell validation and `git diff --check`; exercise actual inline packaging/checksum/extraction against the available native ARM64 macOS release binary. No packaging-helper test suite. No Rust behavior change is planned; broaden Cargo checks only if changes/failures justify them.
- Local task acceptance: the resulting diff visibly matches all five recommendation steps and passes the stated local checks. New native hosted validation remains a stage-exit requirement after an authorised push; old green checks do not cover rewritten workflows.
- Review/publication: Run 02 Q01 approves the concrete simplification commit, one push to PR #4 and opening the whole PR in Diffle. No new independent review is authorised. A prospective review recommendation is one `gpt-6.1-sol` at `high` for workflow gates and artifact correctness; it may run only with specific user approval.

Stage exit: local implementation and scope checks complete; any user-approved review and resulting fixes pass; authorised commit/push and new native CI establish hosted acceptance. Keep missing external evidence explicit.

## Historical S06/S07 requirements and failure coverage

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

Historical S06/S07 implementation was by the primary agent with four specifically approved reviews. Current S08 implementation is delegated to one `gpt-6.1-sol` at `xhigh`. No S08 independent reviewer has been authorised; earlier review grants do not apply.

Done remains blank until checks and required reviews pass. S07 hosted acceptance is recorded after the PR's automatic CI; an untested workflow is not completed acceptance. All historical permission restrictions remain unless explicitly superseded by Run 01.

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S06.T00](#s06-t00) | ✅ | Quilava: allocator stage completion | — | Both approved T01 reviews passed | Small isolated prerequisite | [S06 evidence](logs/20261107-s06-release-log.md); task commit follows |
| [S06.T01](#s06-t01) | ✅ | Vigoroth: executable allocator | Primary agent | Sol high correctness and Sol medium simplification: no findings | Verify ownership and keep integration minimal | [108 local tests and checks passed](logs/20261107-s06-release-log.md); native release evidence remains S07 |
| [S07.T00](#s07-t00) | ✅ | Xurkitree: original release stage completion | — | Both T01 reviews complete | Historical implementation; D08 supersedes helper design | [Native CI passed](https://github.com/jamtat/pokington/actions/runs/42401861071) at d314459; final PR report closed Run 01 |
| [S07.T01](#s07-t01) | ✅ | Dratini: original release pipeline and CI setup | Primary agent | Sol high correctness: one P1 resolved/tested; Sol high simplification: no findings | Historical implementation | [S07 evidence](logs/20261107-s07-release-log.md); final native checks passed before S08 |
| [S08.T00](#s08-t00) |  | Hypno: simplified release stage completion | — | No new review authorised | Require actual simplified pipeline acceptance | [S08 log](logs/20261107-s08-release-log.md) |
| [S08.T01](#s08-t01) | ✅ | Infernape: remove custom release automation locally | 1 gpt-6.1-sol xhigh agent | No new independent review authorised or run | Exact implementation delegated by user | [Local implementation/checks complete](logs/20261107-s08-release-log.md); commit/hosted acceptance remains S08.T00 |
