# GitHub releases implementation log index

Date: 2026-11-07.

Plan: [Implementation plan](20261107-release-plan.md). Current contract: [Release decisions](20261107-release-design.md).

2026-11-08 planning update: D09–D12 settle manual release preparation with an exact version, replacement of unpublished drafts from latest main and manual publication. S09 (Kakuna; S09.T01 Cyndaquil) is proposed in the existing plan. Implementation-stage, specific review and publication approvals remain pending. Create its separate stage log when work begins; historical S06–S08 grants do not apply. Unrelated Dustox documentation edits remain separate.

2026-11-08 display update: the user selected “A” for a synchronized current-version input description and workflow-write token. D13 and S09 now include the repository-limited `RELEASE_TOKEN` prerequisite, synchronized three-file version commit and display/credential checks. Design is settled. No token, secret, workflow implementation, review or release operation has been created or run; implementation-stage and specific review approvals remain pending.

2026-11-08 execution update: user answered “y” to local implementation on main plus one Sol high correctness review and one separate Sol high simplification review. [Run 03 Q01](logs/20261108-s09-release-log.md#q01--implementation-stage-and-exact-reviews) supersedes the earlier pending implementation/review wording. Workflow/docs implementation and initial 38 local checks pass; correctness review is in progress. No commits, publication, credentials or actual release operations are authorised.

2026-11-08 completion update: [Run 03](logs/20261108-s09-release-log.md#local-stage-completion--2026-11-08t010429z) completed the authorised local S09 scope at 2026-11-08T01:04:29Z. Implementation/docs, 38 local fixture checks, actionlint/ShellCheck, shell/link/diff checks and both independent reviews passed. Reviewers reported no actionable correctness findings or useful simplifications. Changes remain uncommitted; no actual token, hosted release or CI mutation was performed. Publication, token setup and real hosted acceptance remain separate prerequisites before use.

2026-11-08 D14 correction: user instructed “ok drop it” for synchronized version display and the PAT requirement. [Run 04](logs/20261108-s09-release-log.md#run-04--remove-the-pat-and-synchronized-label) removed self-editing workflow/version-label logic and custom secrets, uses the built-in GITHUB_TOKEN with contents write in preparation/finalization, and commits only Cargo version files. All 38 updated local fixture checks, actionlint/ShellCheck, shell syntax and diff checks pass. No new review was authorised or performed; prior reviews cover the earlier variant. No custom-token setup remains. Changes remain uncommitted and hosted acceptance unverified.

2026-11-08 final review grant: user answered “y” to the recommendation for one fresh Sol high review of the final GITHUB_TOKEN change. [Run 05](logs/20261108-s09-release-log.md#run-05--final-built-in-token-review) authorises exactly that findings-only review; it is in progress. No commits, pushes, additional reviewers, manual CI or release operations are authorised.

2026-11-08 final review result: Run 05 closed at 2026-11-08T02:34:39Z. The fresh Sol high reviewer found no actionable issues and independently passed actionlint/ShellCheck, diff checks and all 38 updated local fixtures. Current built-in-token variant is independently reviewed. Hosted execution remains unverified; all changes remain uncommitted.

2026-11-08 publication grant: user approved `release: S09.T01 automate manual draft releases` and one push to origin/main including automatic CI, excluding unrelated Dustox documentation changes. [Run 06](logs/20261108-s09-release-log.md#run-06--approved-release-commit-and-originmain-publication) is current authority. Closure takes effect after the authorised commit/push succeed; the final report supplies actual Git and CI receipts without another bookkeeping commit. Manual CI, tags and actual releases remain unauthorised.

## Stage logs

| Stage | Log and canonical run |
| --- | --- |
| S06 — Quilava | [Allocator integration and canonical Run 01](logs/20261107-s06-release-log.md) |
| S07 — Xurkitree | [Release pipeline and CI evidence](logs/20261107-s07-release-log.md); authority remains Run 01 |
| S08 — Hypno | [Simplification and canonical Run 02](logs/20261107-s08-release-log.md); D08 supersedes the helper design |
| S09 — Kakuna | [Runs 03–05](logs/20261108-s09-release-log.md); final built-in-token variant locally checked and independently reviewed; uncommitted, no publication |

Cross-stage entries link to canonical Run 01 rather than copying grants.

## Recording rules

- Preserve exact user instructions, scoped approvals and later changes. Record observation times as observation times; do not invent message timestamps.
- Record actual UTC stage/task starts, checks, review identities/outcomes, accepted/rejected simplifications, deviations and commits. Finish stays blank until required checks and reviews pass.
- Keep local, mocked and hosted evidence distinct. Missing native CI evidence does not pass because local checks pass.
- Run 01 closed with PR #4's final native acceptance report and completion summon. Run 02 governs the explicitly requested delegated simplification. Do not carry closed-run permissions into later work or create recursive bookkeeping-only commits.
- Existing grants cover only the same run and specified reviews, branch, recorded pushes to the same PR and summons; Run 01 Q03 records the additional correction commit/push. Follow current personal instructions for additional reviews, manual CI actions, sudo, merges, tags and release publication.
