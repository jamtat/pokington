# GitHub releases implementation plan

## Continuation prompt

```text
Use $jamtat-work to continue GitHub releases using docs/20261107-release-plan.md and docs/20261107-release-log.md. Read D09–D14 in docs/20261107-release-design.md and the latest Run 06 in docs/logs/20261108-s09-release-log.md. S09.T01 is locally implemented, checked and independently reviewed. D14 drops the synchronized current-version label and uses only GITHUB_TOKEN; no PAT, GitHub App or custom secret is required. Do not reimplement completed work, resume historical S08 delegation or reuse closed-run review permissions. Sxx.T00 rows are completion markers. Preserve the current worktree and unrelated Dustox documentation edits.

Run 06 authorises one scoped commit titled release: S09.T01 automate manual draft releases and one push to origin/main including automatic CI. Honour that exact grant while unfinished; do not ask again. Run 05's final-token review passed; Run 03's initial two reviews are historical. Retain D08's ordinary inline commands, stock artifact actions and one final softprops draft job. No PR, manual CI, workflow dispatch, actual tag mutation or release operation is authorised. Distinguish local fixture checks, hosted build evidence and actual release acceptance.
```

Date: 2026-11-07. Status: S06/S07 completed; S08 local implementation/checks complete, commit/push and whole-PR Diffle approved; new hosted acceptance pending.

2026-11-08 update: release changes are merged on main as `e5b2d3b`; the historical S08 log has not been reconciled with final hosted evidence. D09–D12 settle the new manual-release design. S09 below is proposed; no implementation-stage approval or new review/publication grant has been recorded.

2026-11-08 display decision: D13 additionally accepts a synchronized current-version input description and repository-limited workflow-write token. S09 includes this work and its credential prerequisite; design is settled, implementation-stage and specific review approvals remain pending.

2026-11-08 execution update: user answered “y” to the S09 local implementation and two Sol high review proposal. [Run 03 Q01](logs/20261108-s09-release-log.md#q01--implementation-stage-and-exact-reviews) is current authority, superseding the pending approval statements above. Implementation and initial 38 local fixture checks pass; the correctness review is in progress. Changes must remain uncommitted; hosted and actual release execution are not authorised.

2026-11-08 completion update: S09 local implementation/checks and both approved reviews are complete with no findings requiring changes. All 38 local fixture checks and workflow/shell/documentation checks pass. Changes remain uncommitted under the user's explicit scope. Hosted/native release execution and token setup remain unverified. [Run 03 closure](logs/20261108-s09-release-log.md#local-stage-completion--2026-11-08t010429z).

2026-11-08 D14 correction: the user instructed “ok drop it” for the synchronized version label and custom-token requirement. The local workflow now uses GITHUB_TOKEN with write permission in preparation/finalization and commits only the two Cargo files. All 38 updated local checks and static validation pass. Earlier independent reviews cover the pre-D14 variant; no new review was authorised or performed. Changes remain uncommitted; real hosted acceptance remains unverified. [Run 04](logs/20261108-s09-release-log.md#run-04--remove-the-pat-and-synchronized-label).

2026-11-08 final review update: the user approved one fresh Sol high review of the final GITHUB_TOKEN change. That exact review is in progress under [Run 05](logs/20261108-s09-release-log.md#run-05--final-built-in-token-review); no new commit, push or release grant exists.

2026-11-08 final review result: Run 05 completed with no actionable findings; its fresh Sol high reviewer independently passed workflow/ShellCheck/diff checks and all 38 local fixtures for the final built-in-token variant. Current local implementation is ready for a separately approved commit/publication; real hosted execution remains unverified.

2026-11-08 publication grant: user answered “y” to the concrete S09.T01 commit proposal and one origin/main push, including automatic CI. [Run 06](logs/20261108-s09-release-log.md#run-06--approved-release-commit-and-originmain-publication) supersedes earlier uncommitted/no-publication limits for the nine selected release files. Normal hooks and the push remain to be executed; actual release dispatch/publication is not authorised. Dustox documentation edits are excluded.

## Scope and precedence

Current user instructions and [release decisions D01–D14](20261107-release-design.md) govern. D09–D12 and D14 define S09 manual preparation and draft replacement using the built-in token, superseding D13's synchronized display/custom token. D08's simple implementation boundary remains. Earlier S06–S08 descriptions record historical work and permissions. The [brief](brief.md) and existing bootstrap/data/terminal contracts remain applicable. The [log index](20261107-release-log.md) links canonical run authority. Historical grants in other plans do not authorise this work.

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

## S09 — Kakuna: prepare releases from the Actions UI

<a id="s09-t00"></a>
Prerequisites: D09–D12 and D14 settled; current local implementation is complete under Run 03 and the user's Run 04 revision. Use the current main checkout and preserve unrelated edits. Hosted execution uses GITHUB_TOKEN; existing branch/tag rules must permit the Actions bot's writes. No custom secret is required. S08 hosted-evidence reconciliation is separate from this stage.

<a id="s09-t01"></a>
### S09.T01 — Cyndaquil: automate versioning, builds and draft replacement

- Outcome: Actions → Release → Run workflow accepts one required exact stable `X.Y.Z` version. It uses latest main, prepares Cargo version files and a version commit when needed, runs all existing release checks and builds, then creates or replaces a draft. Manual Publish release remains the final step. No static input default, automatic version increment or protected-environment setup.
- Version lookup and authentication: the input description directs the user to Cargo.toml on main; no synchronized display or self-editing workflow remains. Use the built-in GITHUB_TOKEN for GitHub CLI operations, with contents write permission only in preparation/finalization. The version commit changes only Cargo.toml and Cargo.lock. Rebuilds at the same version do not create empty commits.
- Version validation: use ordinary shell, the GitHub CLI and Ubuntu `sort -V`. Reject malformed versions, leading zeros, and any version at or below the highest published stable release. Fail on API errors before writes; no published releases is a valid first-release state. A matching draft may be replaced; preserve edited notes. Existing version tags without a draft may be reused only when already pointing to the exact candidate commit; otherwise fail for manual recovery.
- Reuse/build boundary: extend `.github/workflows/release.yml`; thread an explicit source SHA through `ci.yml` and `build.yml` while retaining their ordinary CI defaults. Reuse native tests, mimalloc, archive/checksum/smoke/static-link checks, standard artifacts and the pinned softprops action. Update README and the existing dated maintainer guide. No new release framework, helper program, test suite that mirrors the workflow or composite action.
- Source and ordering: serialize manual release preparation globally and require dispatch from main. Read latest main at execution, update only the package version and matching lockfile metadata, and commit only those two files when changed. Push the version commit without force and build/test that exact SHA. Invoke reusable checks explicitly because built-in-token pushes do not trigger another push workflow. The release trigger is workflow_dispatch. Only after checks pass create the version tag or update the confirmed draft's tag to that SHA, then replace draft assets. Recheck published status before tag/release writes; use a lease for an existing draft tag so a concurrent ref change fails. Never force-update main or alter a published release. Keep permission writes confined to preparation/finalization jobs.
- Failure boundaries: a failed build leaves the version commit on main but does not replace draft assets or move its tag. Retry uses latest main and the requested version. A failure after tag creation can reuse that exact tag/commit; an unexplained conflicting tag fails. Missing/insufficient token permissions or branch rules may reject preparation; report the failure without bypassing protection or falling back to unrelated credentials. Manual publication during an in-flight draft rebuild must be detected before final writes; document the remaining API race rather than claiming atomicity across Git refs and release assets.
- Verification: run `actionlint` (including ShellCheck), inline shell syntax checks and `git diff --check`. Exercise the actual version-validation commands with synthetic release inventories and local temporary Git repositories: no releases, larger patch/minor/major, equal/older published version, malformed input, draft refresh from changed main, unrelated tag rejection, exact-commit recovery, stale tag lease and failed API/read/push paths. Confirm every checkout builds the selected SHA and normal push/PR CI retains its original default. No Rust behavior changes are planned; broaden Cargo checks only if relevant changes or failures justify them.
- Credential/scope verification: require no custom secret, confirm every GH_TOKEN reference uses github.token, verify preparation/finalization contents-write permissions and read-only build/check jobs, assert only Cargo.toml/Cargo.lock change during a version bump and workflow content remains unchanged, and retain unchanged-version rebuild behavior. No extra push-triggered release workflow is expected.
- Reviews proposed, not authorised: one fresh `gpt-6.1-sol` at `high` for version ordering, exact source propagation, published protection, tag/commit handling and failure paths; then a separate fresh `gpt-6.1-sol` at `high` for unnecessary logic, shell clarity and D08 compliance. Primary implements directly; reviewers make findings only. Offer further review at the stage boundary with its exact scope/model/effort/count; do not launch it without approval.
- Evidence/authority: create the S09 stage log when implementation starts; record local checks, synthetic evidence and both approved review outcomes separately. Present the final commit title and changes-by-file table before asking for commit permission. Current design choices do not grant commits, pushes, PRs, manual CI or a real release run.

Stage exit: local implementation and documentation pass checks and the specifically approved reviews. Commit/publication remain separately gated. Hosted native checks require an approved push; a real manual release remains unverified until separately authorised execution. Stop after S09; do not perform a release to demonstrate completion.

### S09 requirement mapping

| Requirement | Task | Acceptance |
| --- | --- | --- |
| Exact version; simple ordering | S09.T01 | Stable versions only; reject versions at or below the highest published stable version before writes. |
| Built-in token without custom setup | S09.T01 | No custom-secret references; preparation/finalization contents-write permission; only Cargo files change during a version bump. |
| Rebuild an existing draft from main | S09.T01 | Updated candidate SHA reaches all builds; tag/assets change only after passing checks; draft stays unpublished. |
| Protect published releases and unrelated tags | S09.T01 | Published-version refusal, conflicting-tag failure and stale-lease failure observed in local fixtures. |
| Existing four platforms and packaging | S09.T01 | Shared jobs preserved and statically validated; actual new hosted results recorded after an approved push. |
| Manual publication and simple implementation | S09.T01 | Draft link and review instructions documented; ordinary inline commands and stock actions retained. |

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

2026-11-08 correction: the preceding assignment describes historical S08. S09 uses direct primary implementation and the two separate `gpt-6.1-sol` `high` reviews approved in Run 03 Q01. No additional review, commit or publication grant exists.

Done remains blank until checks and required reviews pass. S07 hosted acceptance is recorded after the PR's automatic CI; an untested workflow is not completed acceptance. All historical permission restrictions remain unless explicitly superseded by Run 01.

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S06.T00](#s06-t00) | ✅ | Quilava: allocator stage completion | — | Both approved T01 reviews passed | Small isolated prerequisite | [S06 evidence](logs/20261107-s06-release-log.md); task commit follows |
| [S06.T01](#s06-t01) | ✅ | Vigoroth: executable allocator | Primary agent | Sol high correctness and Sol medium simplification: no findings | Verify ownership and keep integration minimal | [108 local tests and checks passed](logs/20261107-s06-release-log.md); native release evidence remains S07 |
| [S07.T00](#s07-t00) | ✅ | Xurkitree: original release stage completion | — | Both T01 reviews complete | Historical implementation; D08 supersedes helper design | [Native CI passed](https://github.com/jamtat/pokington/actions/runs/42401861071) at d314459; final PR report closed Run 01 |
| [S07.T01](#s07-t01) | ✅ | Dratini: original release pipeline and CI setup | Primary agent | Sol high correctness: one P1 resolved/tested; Sol high simplification: no findings | Historical implementation | [S07 evidence](logs/20261107-s07-release-log.md); final native checks passed before S08 |
| [S08.T00](#s08-t00) |  | Hypno: simplified release stage completion | — | No new review authorised | Require actual simplified pipeline acceptance | [S08 log](logs/20261107-s08-release-log.md) |
| [S08.T01](#s08-t01) | ✅ | Infernape: remove custom release automation locally | 1 gpt-6.1-sol xhigh agent | No new independent review authorised or run | Exact implementation delegated by user | [Local implementation/checks complete](logs/20261107-s08-release-log.md); commit/hosted acceptance remains S08.T00 |
| [S09.T00](#s09-t00) | ✅ | Kakuna: manual release preparation stage completion (local scope) | — | Initial two reviews passed; fresh final Sol high token review passed in Run 05 | Keep source, tag and assets consistent | [Run 05](logs/20261108-s09-release-log.md#run-05--final-built-in-token-review): final variant locally checked/reviewed; uncommitted; hosted execution unverified. |
| [S09.T01](#s09-t01) | ✅ | Cyndaquil: version input, exact-source builds and draft replacement (local scope) | Primary agent directly | Sol high correctness/simplification for initial variant; fresh Sol high final token review: no findings | Check release mutations and preserve simple workflow structure | [Run 05](logs/20261108-s09-release-log.md#run-05--final-built-in-token-review): 38 updated checks and static validation independently passed; built-in token only; uncommitted. |
