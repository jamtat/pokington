# GitHub release design

Date: 2026-11-07. Status: D08 simplification implemented and checked locally on `btat-github-releases`; commit/publication and new hosted validation pending.

## Outcome and sources

The following paragraph records the initial planning snapshot; D08 governs the current simplification.

Distribute Pokington through GitHub Releases. Current user decisions govern this design. The [brief](brief.md) limits initial platform support to macOS and Linux. [Cargo.toml](../Cargo.toml) and [Cargo.lock](../Cargo.lock) contain version `0.0.1`, committed in `7a5f6c6`. The existing [CI workflow](../.github/workflows/ci.yml) checks Linux pull requests and pushes to `main`; no release workflow exists.

## D01: release build targets — 2026-11-07

- Initial proposal: macOS Apple Silicon, macOS Intel and Linux x86-64, with Linux ARM64 deferred.
- User instruction: “add linux arm64 to the targets to build”.
- Confirmation: after the four-target list was restated, the user replied “yes”.
- Accepted decision: build release binaries for macOS ARM64, macOS x86-64, Linux x86-64 and Linux ARM64.
- Rejected alternative: deferring Linux ARM64; the user explicitly included it.
- Consequence: the release workflow must produce an archive for each target. Linux libc compatibility and build/test environments remain to be settled.

## D02: release automation — 2026-11-07

- Options: A, maintain a small GitHub Actions workflow with explicit build, package and upload steps; B, use `cargo-dist` to generate the workflow and installers.
- Recommendation: A keeps the initial workflow limited to distributing one executable across four targets.
- User response: “A”, following an explanation of `cargo-dist`.
- Accepted decision: maintain a custom GitHub Actions release workflow.
- Rejected alternative: `cargo-dist`; the user selected the custom workflow.
- Consequence: the repository owns release validation, platform builds, packaging and GitHub upload steps. Installer scope remains open.

## D03: release trigger and publication — 2026-11-07

- Options: A, push a version tag to build a draft release for manual publication; B, push a version tag to build and publish automatically after checks; C, manually run the workflow for an existing tag to prepare a draft.
- Recommendation: A identifies the exact source with a tag and allows checking assets and notes before publication.
- User response: “a”.
- Accepted decision: pushing a version tag such as `v0.0.1` starts the release workflow; successful builds attach the four target archives to a draft GitHub Release. Publication is manual.
- Rejected alternatives: automatic publication and manually starting the normal release build; the user selected A.
- Consequence: complete required checks and all builds before treating the draft as ready. Define recovery for failed builds and safe reruns during implementation planning. This decision authorises workflow design, not an actual tag push or release publication.

## D04: Linux binary compatibility — 2026-11-07

- Options: A, static musl binaries for both Linux architectures; B, dynamic glibc binaries with a documented minimum glibc version.
- Recommendation: A avoids an installed glibc dependency for command-line downloads, subject to successful dependency builds and verification of static linkage.
- User response: “A using mimalloc for the allocator the default one is shit”.
- Accepted decision: build `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` release binaries with static linkage.
- Rejected alternative: dynamic glibc release binaries; the user selected A.
- Consequence: prove dependency build compatibility, inspect final ELF linkage and execute the packaged binaries on both Linux architectures. Compilation alone does not establish runtime compatibility.

## D05: application allocator — 2026-11-07

- User requirement: use mimalloc, supplied with the D04 response.
- Scope confirmation: after discussing macOS support and clarifying that applying mimalloc to all four targets is a policy choice rather than a musl requirement, the user replied “yes”. Retain mimalloc across all four targets.
- Accepted decision: use mimalloc as the Pokington executable's Rust global allocator. Apply the application allocator consistently across the four release targets; this is not a requirement to replace allocations internal to native dependencies.
- Implementation direction: add the `mimalloc` dependency and a `#[global_allocator]` static in `src/main.rs`, following the [crate documentation](https://docs.rs/mimalloc/latest/mimalloc/). The library currently declares no global allocator. Update `Cargo.lock` and ensure each target has the C compiler needed by the [Rust wrapper](https://github.com/purpleprotocol/mimalloc_rust).
- Rejected alternative: retaining the default Rust allocator for the released executable.
- Consequence: allocator wiring becomes part of the implementation and target build checks. This records a user requirement, not a measured performance improvement; no allocator benchmark has run.

## D06: installation scope — 2026-11-07

- Options: A, downloadable archives and checksums; B, also provide a platform-detecting shell installer; C, also provide a shell installer and Homebrew tap.
- Recommendation: A keeps implementation and maintenance small for the first release.
- User response: “A”.
- Accepted decision: distribute archives and SHA-256 checksums. Users extract the matching binary and put it on `PATH`.
- Rejected alternatives: shell installer and Homebrew tap in the initial release.
- Consequence: document selecting the correct archive, verifying its checksum and installing the executable. Installer automation and package-manager publication are outside the initial release scope. Catalogue and artwork remain first-run downloads under the existing bootstrap contract.

## D07: release validation — 2026-11-07

- Options: A, full tests on every target plus packaged executable checks; B, full tests on Linux x86-64 with packaged executable checks on the other targets.
- Recommendation: A covers musl, allocator and platform differences.
- User response: “A”, after asking about runner costs.
- Accepted decision: run the full suite on all four native architectures, extract and execute each packaged binary, check its version and verify static Linux linkage before preparing a draft.
- Rejected alternative: limiting full tests to Linux x86-64.
- Environment evidence: GitHub documents native standard runners for all four targets in its [runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). `gh repo view jamtat/pokington --json visibility,owner` reported `PUBLIC`; standard public-repository compute is free under the current [billing policy](https://docs.github.com/en/billing/concepts/product-billing/github-actions). Artifact/cache retention remains separate from runner compute.

## Implementation stages and authority

Current implementation follows D08 and S08 below. S06/S07 and their grants are historical records of the first implementation.

- S06: integrate mimalloc and verify the local application.
- S07: implement shared four-target build/test/packaging, tag validation, draft release preparation, safe recovery and installation/release documentation; investigate and reduce repeated CI setup.
- User instructions: “do it on a branch”, “then PR it it”, and “open diffle when you're done with the PR content. use summon when finished. i want you to auto proceed through all commits required and then push without my approval please”.
- The subsequent permission proposal explicitly listed automatic stage continuation, four specific reviews and two exact CI sudo commands. User response: “y, we should try and investigate how to make the ci super fast, installing that every time is a waste”. The [canonical run](logs/20261107-s06-release-log.md#run-01--s06t01-and-s07t01) records their scope. No actual version-tag push or GitHub release creation/publication is authorised in this run.
- Current work is governed by this design and the [implementation plan](20261107-release-plan.md), with evidence in the [separate log index](20261107-release-log.md). No product decisions remain open; material implementation contradictions require discussion.

## Implementation record — 2026-11-07

- S06: mimalloc wired into the executable; locked host checks, 108 tests and release help/version passed. Both approved allocator reviews reported no findings. Native release acceptance remains S07. See [S06 evidence](logs/20261107-s06-release-log.md).
- Dated correction: at the user's request, the branch was rebased onto `origin/main` at `8418404`. The version commit is now `e5b0e24` and the allocator commit is now `ded2ba5`; earlier hashes remain historical. Upstream species-exclusion behavior and documentation were preserved. See [S07 continuation](logs/20261107-s07-release-log.md#dated-continuation--requested-rebase).
- S07: custom shared native workflows, cached musl setup, archive/checksum verification, draft preparation/recovery and installation/release documentation implemented. Local ARM64 macOS release validation passed 111 tests; ten helper tests and static workflow/shell checks pass. Correctness review's draft-discovery finding was fixed and tested; simplification review had no findings. Real hosted matrix acceptance remains pending the PR's automatic CI. See [S07 evidence](logs/20261107-s07-release-log.md).
- Completion correction: the final S07 native CI run [42401861071](https://github.com/jamtat/pokington/actions/runs/42401861071) passed all six jobs at `d314459`; the earlier pending wording was the pre-publication snapshot. PR #4 and its final description recorded hosted acceptance. That evidence does not validate the later S08 workflow changes.

## D08: use ordinary workflow commands and a maintained release action — 2026-11-07

- User request after research: “delegate a sol xhigh agent to implement the suggestion. set a goal for yourself and the agent. it MUST look like your recommendation”. This explicitly authorises immediate implementation of the recommendation, with one `gpt-6.1-sol` implementer at `xhigh` and matching parent/agent goals.
- Evidence: three agents inspected 15 distinct Rust repositories. Recurring patterns were Cargo/cross, ordinary shell archive/checksum commands, standard artifact actions and softprops or gh. Close examples are [zoxide](https://github.com/ajeetdsouza/zoxide/blob/c21f547371ebd635eba0474c793069b37b0570ab/.github/workflows/release.yml), [delta](https://github.com/dandavison/delta/blob/3c2269c6b845913965575f11b0196d70ef5352f3/.github/workflows/cd.yml) and [bottom](https://github.com/ClementTsang/bottom/blob/cc6facab16600fd79d822e6e37deb42abefe7b0f/.github/workflows/deployment.yml). This sample did not establish the previously proposed taiki packaging/toolchain stack as standard.
- Accepted direction: retain the four native full-test jobs, mimalloc and application behavior. Use short inline Cargo metadata/jq, tar, checksum, extracted-executable and ELF checks. Transfer files with standard artifact actions. One final `softprops/action-gh-release` job uses `draft: true` after all checks pass; publication stays manual.
- Delete `release.mjs`, its Node tests, `setup-musl.sh` and the local Rust composite action. Use ordinary native Linux musl-tools setup and standard Rust dependency caching. Produce per-archive `.sha256` files. Update installation and maintainer instructions.
- Supersedes: S07's custom parser/API recovery program, combined checksum manifest, source-note markers, custom sysroot relocation/cache and helper test suite. Do not recreate them in Rust, shell helper files, large inline programs or another local action. Minimal published-release protection may use the existing CLI.
- Scope/permission limits: local implementation and checks are authorised. This instruction does not explicitly grant a new independent review, commit, push, manual CI action or release mutation. S08 records the delegated implementation and resulting evidence; earlier grants do not authorise new review agents.
- Local result: the Sol xhigh implementer deleted the three helper files and local Rust action, and replaced them with ordinary inline workflow commands and one final softprops draft job. Its nine implementation/doc files changed by +134/-601 lines. Workflow/shell validation, real ARM64 macOS archive checks, tag matching and ELF rejection checks passed; the parent confirmed recommendation compliance and unchanged application/Cargo/test files. New hosted CI remains pending an authorised push.

## D09: manually start release preparation; publish after user review — 2026-11-08

- User request: “i want a github action that can do the relase instead please and deal with all that.” Proposed flow: manually start the release action, select a version, update Cargo files, commit/tag, run the existing four-target checks and prepare release assets and notes.
- Publication history: the user initially selected “A” for automatic publication, then revised that choice: “actually i would like it to publish a draft then have the additional manual step of me approving is that ok”.
- Accepted publication decision: the action creates a draft after checks pass. The user reviews the draft and publishes it manually using GitHub's Publish release button. The revised answer supersedes automatic publication; D03/D08's draft behavior is retained while a manual workflow trigger is added.
- Version discussion: proposed stable `X.Y.Z` input and Ubuntu `sort -V` comparison against the highest existing stable `vX.Y.Z` tag, rejecting equal/older versions before writes and allowing the first release when no matching tags exist. Version selection remains to be settled.
- Scope limits: no workflow changes implemented yet. This design records future workflow behavior; it does not execute a commit, push, tag, workflow dispatch or release operation, or authorise an independent review.

## D10: allow replacing an unpublished draft — 2026-11-08

- User requirement: “we should be able to re-publish over draft versions right”.
- Accepted direction: allow preparing the same version again while its release remains a draft, replacing generated archives and checksums and retaining draft status. Reject changes to a published version.
- Correction to D09's proposed comparison: new versions must be newer than the highest published stable release version, rather than every existing tag. An existing draft may reuse its version, provided it remains newer than the highest published version. Existing tags without a release require explicit recovery handling; do not treat every existing tag as an editable draft.
- Implementation evidence: [softprops/action-gh-release documentation](https://github.com/softprops/action-gh-release#customizing) supports reusing drafts with `draft: true` and replacing matching assets with `overwrite_files: true`. Keep the existing published-release protection.
- Unresolved source decision: whether preparing an existing draft uses its original tagged commit or incorporates newer main changes. Incorporating newer changes would require revising the historical fixed-tag rule; do not move any tag before that decision and implementation approval.
- No workflow edits, actual release writes, tag changes, commits or pushes were performed to record this decision.

## D11: rebuild drafts from latest main — 2026-11-08

- Options: A, rebuild an existing draft from latest main so fixes can be included before publication; B, rebuild only its original tagged commit for retries.
- Recommendation: A permits correcting an unreleased version without a version bump. User response (exact): “a”.
- Accepted decision: preparing an existing draft uses latest main and replaces that draft's generated assets. Its version tag must identify the exact new release commit used for all builds; keep the release as a draft until manual publication.
- Supersedes: the historical rule forbidding tag movement is relaxed only for a confirmed existing draft newer than every published stable version. Published release tags and assets remain protected; an unrelated existing tag is not implicitly editable.
- Implementation boundary: serialize release preparation, verify draft status before writes, and reuse the existing checks and native packaging. Preserve manually edited draft notes when refreshing assets.
- Version input remains unresolved: exact stable version entry versus calculated patch/minor/major increment.
- This decision approves the intended workflow behavior, not running a release or performing a tag update during development. No workflow edits or release operations have been performed.

## D12: enter an exact stable version — 2026-11-08

- Options: A, enter an exact version such as `0.0.2`, reusing an existing draft's version to rebuild it; B, select patch/minor/major or rebuild the current draft. Recommendation: A is simpler and explicit. User response (exact): “a”.
- Accepted decision: the manual release action takes one required stable `X.Y.Z` version, without a `v` prefix. Validate numeric components without leading zeros; reject a version at or below the highest published stable version. Allow the first release and D10/D11's draft replacement.
- User asked whether the version box can be prefilled, adding “if not no issue”. GitHub supports literal input defaults but has no built-in repository-file lookup for the dispatch form. Keep the input manual instead of introducing a stale fixed default or extra automation. Sources: [manual inputs](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow#defining-inputs-for-manually-triggered-workflows), [expression context availability](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability).
- Design is settled. Proposed implementation is S09 in the existing release plan; implementation-stage approval remains pending. No workflow changes or release operations have been performed.

## 2026-11-08 — Current-version display requirement, decision pending

- User request: “it would be really helpful if somewhere in the input box or around the inputs for the workflow it showed the current version so i can decide what to bump it to”. This reopens D12's manual-form presentation; exact version entry remains accepted.
- Feasible direct display: put the current Cargo package version in the input description, for example `Release version (current: 0.0.1)`, and synchronize that literal in the version-update commit. GitHub does not run repository queries while rendering the dispatch form.
- Credential consequence: automatic synchronization modifies `.github/workflows/release.yml`; it needs authentication permitted to write workflow files, beyond the ordinary contents-only bot token assumed in S09. See [GitHub workflow-file permissions](https://docs.github.com/en/rest/repos/contents#create-or-update-file-contents) and [token limitations](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token). Do not silently add credentials, broaden permissions or represent the label as dynamically evaluated.
- Alternative: keep ordinary token permissions and provide the Cargo.toml location in the input description for manual lookup. Direct synchronized display versus manual lookup remains a user decision. No workflow changes, credential setup or implementation approval has occurred.

## D13: synchronize the current version in the input description — 2026-11-08

- Options: A, add a workflow-write token and keep the current version displayed in the input description; B, show the Cargo.toml location for manual lookup. Recommendation: A provides the current version where the user selects the release version. User response (exact): “A”.
- Accepted decision: use `Release version (current: X.Y.Z)` in the required version input's description, initialized from the package version. Synchronize the description, Cargo.toml and Cargo.lock in the same version-update commit. The label means the package version on main, which may be an unpublished draft version; it is not a claim about the latest published release. Rebuilding the same version must not create an empty version commit.
- Credential setup: document one fine-grained token restricted to this repository with Contents and Workflows write permission, stored as the Actions secret `RELEASE_TOKEN`. Use it only where required for release preparation; retain ordinary scoped GITHUB_TOKEN usage for other jobs. Check for the secret before mutations; never print it, request its value in chat or expose it in process arguments. The token does not bypass branch protection.
- Supersedes: D12's decision to omit extra form synchronization and the pending display choice above. Exact manual version entry, draft-only output, version ordering and published protection remain accepted.
- Design is settled; S09 implementation-stage and specific review approvals remain pending. Selecting the token-backed design does not create a token, configure a secret, commit/push, dispatch a workflow or run a release.

## S09 implementation record — 2026-11-08

- User approved local implementation on existing main and one Sol high correctness review followed by one separate Sol high simplification review; all changes must remain uncommitted. [Run 03 Q01](logs/20261108-s09-release-log.md#q01--implementation-stage-and-exact-reviews) supersedes the earlier pending implementation/review statements.
- Manual exact-version release preparation, synchronized version label, version/published-state checks, exact prepared SHA through all build jobs, post-check draft-tag replacement and manual publication documentation are implemented locally. No new release framework or helper-test suite was added.
- Actionlint/ShellCheck, inline shell syntax, 38 local actual-script fixtures and documentation/diff checks pass. Both approved reviewers found no changes required. This is local/synthetic evidence; no token setup, hosted acceptance or real release execution was performed. [Run 03 closure](logs/20261108-s09-release-log.md#local-stage-completion--2026-11-08t010429z).

## D14: drop synchronized version display; use the built-in token — 2026-11-08

- Context: after discussing PAT ownership for shared project administration and the GitHub App alternative, the user asked whether dropping the current-version label removes the PAT requirement. The answer explained that the workflow could use the built-in GITHUB_TOKEN for Cargo version commits, tags and draft releases without modifying its own YAML.
- User instruction (exact): “ok drop it”.
- Accepted decision: remove the synchronized current-version display and custom-token prerequisite. The input description points to Cargo.toml on main for manual version lookup. The version commit updates only Cargo.toml and the matching Cargo.lock package entry. Preparation and draft jobs request `contents: write`; other jobs remain read-only. All CLI authentication uses the built-in token. No PAT, GitHub App or extra repository secret is required.
- Supersedes: D13's synchronized input label and RELEASE_TOKEN setup. D09–D12's exact-version input, published-version ordering, draft replacement from latest main and manual publication remain unchanged. Branch/tag rules are still enforced.
- Implementation correction: removed the custom-secret validation and references, label edits and workflow file from the version commit. Removed `[skip ci]` from the future version commit message because built-in-token pushes do not trigger another push workflow; checks are invoked explicitly as before.
- Validation: all 38 updated local checks pass, including no custom-secret requirement, only the two Cargo files in the version commit, unchanged workflow content, built-in-token references and write permissions limited to preparation/finalization. Actionlint/ShellCheck and diff checks pass. The two Run 03 reviews cover the earlier variant; no fresh independent review was authorised or launched for this narrow revision.
- No repository commits, pushes, secret configuration, real workflow runs or release operations were performed. See [Run 04](logs/20261108-s09-release-log.md#run-04--remove-the-pat-and-synchronized-label).

2026-11-08 review correction: the user subsequently approved one fresh Sol high review of D14's final built-in-token variant. [Run 05](logs/20261108-s09-release-log.md#run-05--final-built-in-token-review) completed with no actionable findings and independently passed actionlint/ShellCheck, diff checks and all 38 updated local fixtures. Earlier no-review statements describe the pre-Run05 boundary. No hosted release validation or commit/publication was performed.
