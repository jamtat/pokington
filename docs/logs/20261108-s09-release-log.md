# GitHub releases implementation log — S09

Plan: [Implementation plan](../20261107-release-plan.md#s09-t01). Index: [Implementation log](../20261107-release-log.md). Contract: [D09–D13](../20261107-release-design.md#d09-manually-start-release-preparation-publish-after-user-review--2026-11-08).

## Run 03 — S09.T01 Cyndaquil

- Preparation recorded at: 2026-11-08T00:54:16Z (clock observation; exact user-message time unavailable).
- Run status: complete for the approved local implementation/check/review scope.
- Closed at: 2026-11-08T01:04:29Z (clock observation).
- Originating request: “i want a github action that can do the relase instead please and deal with all that.” Design answers are recorded in D09–D13.
- Branch: existing `main` in `/Users/ben/code/pokington`; preserve unrelated uncommitted Dustox documentation changes.
- Execution: primary agent implements directly; no implementation delegation. Use jamtat-work with the narrower explicit no-commit grant below.
- Goal: S09.T01 local implementation/checks and both approved reviews, leaving changes uncommitted; created on thread `01a1b884-b59e-7301-a66b-a898d35350f8` without a token budget.
- Publication: no commit, push, PR, actual tag mutation, workflow dispatch/rerun, secret configuration or release operation authorised. Local temporary-repository fixtures are permitted checks.
- External prerequisites: hosted execution will need the repository-limited `RELEASE_TOKEN` secret; local work must not read or configure a real token. Four-platform hosted acceptance and a real release are separate from local verification.

### Q01 — Implementation stage and exact reviews

- Proposal (exact):

> Recorded. The input will show **“Release version (current: 0.0.1)”**, kept synchronized during version bumps. Setup will require a repository-scoped `RELEASE_TOKEN`.
>
> Approve one implementation stage on `main`, covering the workflow, documentation, local checks and two separate **Sol high** reviewers:
>
> - Correctness: version checks, draft replacement, exact-commit builds and credentials.
> - Simplification: keep the workflow small and direct.
>
> Changes will remain uncommitted. Reply `y` or `n`.

- The proposal also cited the user's stage-approval rule and jamtat-plan's requirement to present implementation stages for approval.
- User response (exact): “y”.
- Interpretation: approve S09.T01 implementation on current main, stated local checks, one fresh `gpt-6.1-sol` `high` findings-only correctness reviewer covering release correctness/credentials and integrated S09 behavior, then one separate fresh `gpt-6.1-sol` `high` findings-only simplification reviewer. No other reviews or agents. Leave all changes uncommitted.

## S09.T00 — Kakuna: stage timing and exit

- Start: 2026-11-08T00:54:16Z.
- Finish: 2026-11-08T01:04:29Z.
- Exit: local implementation, documentation, checks and both approved reviews; stop before commits or remote operations. Hosted native and actual release behavior remain separately unverified.

## S09.T01 — Cyndaquil: manual release preparation

- Start: 2026-11-08T00:54:16Z.
- Finish: 2026-11-08T01:04:29Z.
- Changes/checks: local implementation and documentation complete; actionlint/ShellCheck, shell syntax, all 38 local fixture checks, documentation links and diff checks pass.
- Correctness review: `/root/s09_correctness`, one fresh `gpt-6.1-sol` at `high`, complete with no actionable findings; independently reran actionlint, diff checks and all 38 local fixtures. Pinned release-action source confirms draft-note retention. No reviewer edits.
- Simplification review: `/root/s09_simplification`, one separate fresh `gpt-6.1-sol` at `high`, complete with no useful simplifications or blockers. Repeated release checks cover distinct mutation boundaries; extracting helpers would add structure. No reviewer edits.
- Commit/publication: explicitly excluded.

### Initial implementation and local checks

- Replaced the tag-push trigger with required-version workflow_dispatch, synchronized current-version description, a globally serialized preparation/checks/draft flow and explicit source SHA propagation through both reusable workflows.
- Preparation checks main dispatch, stable version syntax and RELEASE_TOKEN presence, reads paginated release inventory, rejects versions at/below the highest published stable version, updates only Cargo.toml/Cargo.lock/release.yml and pushes a version commit when needed. The future version commit uses `[skip ci]`; all checks run explicitly in this release workflow.
- Finalization verifies downloaded checksums, rechecks published/draft state, updates the tag with an explicit lease only after checks, and uses the existing softprops action to create/replace a draft. Existing notes are preserved; the summary links the draft for manual publication.
- Updated README and the current maintainer guide with token setup, version selection, draft replacement and failure recovery. Existing history is retained through a dated correction and the decision record.
- `actionlint` including ShellCheck and `git diff --check`: pass. All actual inline run blocks pass `bash -n`.
- Temporary fixture harness `/private/tmp/pokington-s09-checks.rb` extracts and executes the actual inline validation/preparation/tag scripts with real Cargo metadata, a fake gh release inventory and temporary local bare Git remotes. All 38 local checks pass; output: `/private/tmp/pokington-s09-checks.log`. Cases include input rejection, first release/no empty commit, synchronized three-file bump, patch/minor/major ordering, current draft refresh, exact-commit orphan recovery/conflicting-tag refusal, API failures, publication/draft identity change during builds, stale lease and rejected push. Static assertions cover exact SHA propagation, ordinary CI defaults, draft-only replacement and notes retention.
- These are local/synthetic checks on macOS, not Linux runner execution or live GitHub API/upload evidence. No app behavior, Cargo manifest/lock version or Rust source was changed in the real repository. No actual token was used, no secrets configured and no repository commit/push/release was performed.
- Launch the one approved fresh Sol high correctness reviewer now, findings only, covering S09 version ordering, exact source, draft/published/tag handling, credentials and stage integration. No additional review scope.

### Correctness result and simplification launch

- `/root/s09_correctness` completed the one approved Sol high correctness/stage review with no actionable findings. It independently reran `actionlint`, `git diff --check` and all 38 temporary local fixtures, and checked notes retention against the pinned action source.
- Hosted native builds, actual custom-token permissions and live release uploads remain unverified. No reviewer edits or remote operations.
- Proceed with Q01's one separate fresh `gpt-6.1-sol` `high` findings-only simplification review of the release workflows and maintainer documentation, preserving D08–D13 and the accepted release behavior. No extra review, delegation or commits.

### Local stage completion — 2026-11-08T01:04:29Z

- Both approved reviews completed with no changes required. The separate simplification reviewer found no useful reductions; the repeated release-state checks protect distinct mutation boundaries and do not justify a helper abstraction.
- Final local evidence: actionlint including ShellCheck, all inline shell syntax, 38 actual-script local fixtures, YAML parsing, plan/log relative links, unchanged real Cargo manifest/lock and Rust source/tests, and git diff --check pass. No production code changed after fixture verification; final bookkeeping updates are documentation only. No new helper program or helper-test suite was added to the repository.
- S09.T01 and S09.T00 are complete for the expressly approved local scope. All changes remain uncommitted. No repository commits/pushes, credential configuration, workflow dispatch/rerun or actual release mutation occurred. The unrelated Dustox documentation changes remain preserved.
- External limits: the required token secret is not configured or tested by this run; hosted native builds and actual release upload/publication remain unverified. Before use, an authorised publication must put the workflow on main and the user must configure `RELEASE_TOKEN` as documented.
- Historical S08 hosted-evidence reconciliation remains separate; this run supplies no missing S08 CI evidence. Any additional review requires a fresh specific approval; optional recommendation at the boundary is one `gpt-6.1-sol` `high` reviewer of the complete workflow diff, with reviewer count at the user's choice.

## Run 04 — Remove the PAT and synchronized label

- Start recorded at: 2026-11-08T02:03:11Z (clock observation; exact user-message time unavailable).
- User instruction (exact): “ok drop it”, following the explanation that dropping synchronized current-version display allows the built-in GITHUB_TOKEN to handle Cargo commits, tags and draft releases.
- Interpretation: authorise this narrow local revision directly on current main and its checks, preserving all other release behavior. No new independent review, commit, push, secret configuration, manual CI or actual release operation is authorised. Run 03's completed review grants are not reused.
- Design correction: D14 supersedes D13. Removed current-version synchronization, RELEASE_TOKEN references and custom-secret validation. Preparation now requests contents-write permission and both CLI write steps use github.token. Version commits include only Cargo.toml and Cargo.lock; removed the unnecessary skip-ci message suffix because built-in-token pushes do not trigger push workflows.
- Updated README, the maintainer guide, plan and log index. Preserved historical decisions, Run 03 evidence and unrelated Dustox edits.
- Verification: actionlint including ShellCheck, actual inline shell syntax, git diff --check and all 38 updated local fixtures pass. The fixture harness now asserts no custom-secret requirement, built-in-token use, explicit write permissions, the exact two-file version commit and unchanged workflow content; all previous version/draft/tag/source/failure cases still pass. Harness: `/private/tmp/pokington-s09-checks.rb`; output: `/private/tmp/pokington-s09-checks-built-in.log`. Prior harness preserved at `/private/tmp/pokington-s09-checks-with-label.rb`; its original Run 03 output remains unchanged.
- Evidence limit: fixtures use fake gh inventory and temporary local Git remotes with real Cargo metadata. No real token, hosted native workflow or live release upload was tested. The previous two independent reviews apply to the earlier variant; no fresh review was authorised or performed for D14.
- Finish/closure recorded at: 2026-11-08T02:05:43Z. Narrow local revision complete; all changes remain uncommitted. No PAT/App setup is required by the resulting workflow. Existing repository branch/tag rules still apply.

## Run 05 — Final built-in-token review

- Preparation recorded at: 2026-11-08T02:32:44Z (clock observation; exact user-message time unavailable).
- Run status: complete for the final review scope.
- Proposal: “I recommend one fresh **Sol high** review of the final `GITHUB_TOKEN` change before committing; the previous reviews covered the PAT-based version. You can choose a different reviewer count.”
- User response (exact): “y”.
- Interpretation: authorise one fresh `gpt-6.1-sol` at `high`, findings-only review of D14's final built-in-token change, its permissions, release behavior and documentation. Address in-scope findings locally and rerun affected checks. No second reviewer, fresh simplification pass, commits, pushes, secrets, manual CI or release operations are authorised.
- Branch: existing main; preserve unrelated changes. All changes remain uncommitted.
- Reviewer: `/root/s09_builtin_review`, one fresh `gpt-6.1-sol` at `high`, findings only.
- Outcome: no actionable findings. Independently passed actionlint/ShellCheck, git diff --check and all 38 updated local fixtures. Verified built-in-token authentication/CLI credential setup and explicit shared-check invocation against GitHub/CLI guidance. No reviewer edits or remote operations.
- Evidence limits: actual hosted token permissions, repository rules, native builds and draft updates remain unverified; local fixtures use mocked API responses and temporary local Git remotes.
- Closed at: 2026-11-08T02:34:39Z (clock observation). The final review scope is complete; all changes remain uncommitted. No commit/push permission was inferred from this review approval.

## Run 06 — Approved release commit and origin/main publication

- Preparation recorded at: 2026-11-08T02:40:15Z (clock observation; exact user-message time unavailable).
- Run status: open until the authorised commit and push succeed; report the actual Git receipt and automatic CI outcome without recursive bookkeeping commits.
- Proposal: “Proposed commit: `release: S09.T01 automate manual draft releases`.” The file table covered release.yml, ci.yml/build.yml, README/release guide, release design/plan/index and the S09 stage log. It explicitly excluded the separate Dustox documentation changes. Question (exact): “Commit and push these release changes to `origin/main`, including automatically triggered CI? Reply `y` or `n`.”
- User response (exact): “y”.
- Interpretation: authorise one scoped release commit with normal hooks and one origin/main push, including automatically triggered CI. Exclude docs/20261106-implementation-plan.md, docs/20261106-implementation-log.md and docs/logs/20261107-s04-implementation-log.md. No manual CI, extra review, tag/release operation, secret setup, PR or merge is authorised.
- Selected nine files: .github/workflows/release.yml, .github/workflows/ci.yml, .github/workflows/build.yml, README.md, docs/20261107-releasing.md, docs/20261107-release-design.md, docs/20261107-release-plan.md, docs/20261107-release-log.md and this S09 log.
- Prerequisite evidence: initial correctness/simplification and fresh final-token review passed; all 38 updated local fixtures and workflow/shell/diff checks pass. Normal commit hooks will check Rust formatting, compilation, strict Clippy and the application suite.
- Publication receipt: pending execution. Hosted CI on this commit remains unverified; actual release dispatch/publication remains outside this grant.
