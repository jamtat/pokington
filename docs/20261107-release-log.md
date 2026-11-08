# GitHub releases implementation log index

Date: 2026-11-07.

Plan: [Implementation plan](20261107-release-plan.md). Current contract: [Release decisions](20261107-release-design.md).

## Stage logs

| Stage | Log and canonical run |
| --- | --- |
| S06 — Quilava | [Allocator integration and canonical Run 01](logs/20261107-s06-release-log.md) |
| S07 — Xurkitree | [Release pipeline and CI evidence](logs/20261107-s07-release-log.md); authority remains Run 01 |
| S08 — Hypno | [Simplification and canonical Run 02](logs/20261107-s08-release-log.md); D08 supersedes the helper design |

Cross-stage entries link to canonical Run 01 rather than copying grants.

## Recording rules

- Preserve exact user instructions, scoped approvals and later changes. Record observation times as observation times; do not invent message timestamps.
- Record actual UTC stage/task starts, checks, review identities/outcomes, accepted/rejected simplifications, deviations and commits. Finish stays blank until required checks and reviews pass.
- Keep local, mocked and hosted evidence distinct. Missing native CI evidence does not pass because local checks pass.
- Run 01 closed with PR #4's final native acceptance report and completion summon. Run 02 governs the explicitly requested delegated simplification. Do not carry closed-run permissions into later work or create recursive bookkeeping-only commits.
- Existing grants cover only the same run and specified reviews, branch, recorded pushes to the same PR and summons; Run 01 Q03 records the additional correction commit/push. Follow current personal instructions for additional reviews, manual CI actions, sudo, merges, tags and release publication.
