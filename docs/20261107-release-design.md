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
