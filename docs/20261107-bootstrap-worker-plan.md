# Bootstrap worker queue implementation plan

## Continuation prompt

```text
Use $jamtat-work to continue the bootstrap worker queue change using docs/20261107-bootstrap-worker-plan.md and docs/20261107-bootstrap-worker-log.md. Read the linked decisions and current instructions. If scope is unspecified, recommend the single incomplete task S05.T01. S05.T00 is a completion marker. Preserve unrelated changes and use existing local main; do not create a branch or worktree. Ask any unresolved questions one at a time. Obtain specific approval for the two recommended reviews before launching them, and explicit commit approval after presenting the subject and changes by file. This prompt does not grant reviews, commits or publication; the user's narrower consent requirements govern. Implement the selected scope after approval, record checks and actual permissions in a separate stage log, and stop at the stage boundary. Do not push, open a PR or run manual CI without a scoped grant.
```

Date: 2026-11-07. Status: implementation including environment concurrency complete; all 95 working-tree tests and checks pass. The approved Sol `high` correctness review reports no findings; the user declined the separate simplification review and authorised the final commit and `origin/main` push. Completion takes effect with the final task commit; publication follows its successful push. [Run 01 Q04](logs/20261107-s05-bootstrap-worker-log.md) records the exact grant and evidence.

## Scope and precedence

Current user instructions and [worker decisions](20261107-bootstrap-worker-design.md) govern this change. [Brief](brief.md) and [existing design](20261106-design.md) retain the source, reuse, recovery and publication contract. The [original implementation plan](20261106-implementation-plan.md) remains separate; this change does not authorise its remaining rendering work. This plan grants no operations itself.

Use one queue of wrapped `BootstrapTask` values, fallible execution returning data and follow-up tasks, and stop-and-drain failure handling. Worker count defaults to four; the user's later D07 extension adds `POKINGTON_CONCURRENCY=n` with positive-integer validation before store writes. Remove progress bars; keep download logs and the completion summary. Keep task payload separate from execution metadata. Simplicity is a requirement.

## Reuse and boundaries

| Capability | Existing location | Change |
| --- | --- | --- |
| HTTP validation and browser profile | `src/bootstrap.rs`, `src/source.rs` | Reuse; move retry-attempt bookkeeping to the task wrapper without changing retry eligibility or delays. |
| Source discovery and reconciliation | `src/serebii.rs` | Reuse parsers; represent required page fetches as enum tasks. |
| Checkpoints, reuse, immutable artwork and publication | `src/store.rs` | Reuse existing writer API in the coordinator; retain complete discovery before checkpoint preparation. |
| Scheduling and reporting | `src/bootstrap.rs` | Replace both batch loops with one queue and configured worker loops (default four); serialize storage and stderr output. |
| Local HTTP integration fixtures | `tests/bootstrap.rs` | Extend to prove continued scheduling beyond four tasks and failure recovery. |

Preserve pre-existing changes in `src/command.rs` and `tests/command.rs`. Limit dependency changes to enabling existing Tokio features if required; add no executor library. No worker-count option, queue persistence, source-policy change or storage-schema redesign.

## Dependencies and implementation details

Seed the queue with the national index task. Its success returns species tasks. Species and game-page tasks return records or required follow-up page tasks. Once all species records exist, the coordinator prepares the existing checkpoint, reuses valid artwork and queues missing-artwork tasks. Publish only after all required tasks and saves succeed.

Idle workers wait while executing tasks may produce more work. Finish only when the queue and executing count are empty and every result has been applied. Release the queue lock before network I/O and retry backoff. On terminal execution or coordinator failure, close task admission/dequeue, drain running work, retain successful saves and return the first error. Discard follow-up work after failure. Dropping bootstrap cancels workers; the coordinator owns the writer lock for the whole run. Use the smallest concrete result type and synchronization needed; do not introduce a reusable scheduler framework.

## Implementation stage

<a id="s05-t00"></a>
### S05 — Bastiodon: replace bootstrap batches with queued work

Prerequisites: implementation-stage approval; existing local `main` verified; unrelated working-tree changes preserved. No implementation delegation is required.

<a id="s05-t01"></a>
#### S05.T01 — Muk: implement and verify the task queue

- Work: update `src/bootstrap.rs`, focused `tests/bootstrap.rs` coverage, and reporting assertions affected by removal of bars; adjust Tokio features only if necessary. Reuse all existing storage behavior.
- Verification: prove a fifth task starts while a deliberately held first task is still running, and that global concurrency never exceeds four, including enrichment and retries. Use fixture synchronization for ordering assertions rather than speed thresholds.
- Failure checks: exhausted retries stop dequeue; already running successful images are saved; pending work is not admitted; retry resumes saved images; failed force preserves active data; cancellation releases writer ownership and prevents later background writes. Retain current MIME/status/PNG and source-reconciliation tests.
- Completion checks: empty queue with running work is not mistaken for completion; returned tasks run exactly once; all metadata precedes checkpoint preparation; complete catalogue publication remains atomic; download logs and summary remain on stderr with no bars.
- D07 extension checks: isolated child processes verify the unset default and `POKINGTON_CONCURRENCY=1`, `2` and `8` against actual simultaneous held requests; invalid/zero/overflowing/non-Unicode values exit 1 before creating store files. Keep result-channel capacity independent of worker count.
- Commands: `cargo fmt --check`; `cargo check --all-targets --all-features --locked`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; `cargo test --all-targets --all-features --locked`. Do not run pre-commit hooks separately.
- Required evidence: check results and fixture ordering/concurrency observations in the stage log, approved review findings and their disposition, and any approved final commit result. No checks have run during planning.

Stage exit: implementation and checks pass; correctness includes queue termination, failure/cancellation, recovery and stage integration. The user approved one Sol `high` correctness review, which has no findings, then explicitly declined the offered separate simplification review and authorised commit/push in Run 01 Q04. This supersedes the earlier two-review proposal for this stage. Show the concrete file table, commit with normal hooks, push existing `main` to `origin/main`, and stop. Subject: `bootstrap: S05.T01 replace batches with queued tasks`.

## Requirement mapping

| Requirement | Task | Acceptance |
| --- | --- | --- |
| Dynamic enum tasks; four workers | S05.T01 | Index-generated tasks and fifth-task ordering; global maximum of four. |
| Fallible result and separate metadata | S05.T01 | Enum carries work only; wrapper owns retry state; terminal failures reach coordinator. |
| Stop and drain | S05.T01 | No dequeue after failure is recorded; running successful saves retained. |
| Recovery and safe publication | S05.T01 | Existing initial/force/reuse/cancellation integration checks remain passing. |
| Simple implementation and reporting | S05.T01 | Existing parsers/storage reused; no generic executor; stderr logs and summary. |
| Environment concurrency, D07 | S05.T01 | Default/configured counts observed in isolated processes; invalid configuration creates no store files. |

## External prerequisites and limits

Local fixture verification requires Rust dependencies/toolchain and loopback sockets. No live Serebii download is required to prove this scheduling change; fixture evidence must not be reported as live-source acceptance. D07 adds environment-configured concurrency to scope; a concurrency CLI flag and artwork downloads overlapping incomplete metadata discovery remain outside this change.

## Tracking

Implementation: primary agent; no implementation delegation. One `gpt-6.1-sol` correctness reviewer at `high` completed with no findings. The user waived the separate simplification review in Q04. Completion rows below take effect when their final task commit succeeds; publication must then complete before run closure. Logs: [index](20261107-bootstrap-worker-log.md).

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S05.T00](#s05-t00) | ✅ | Bastiodon: completed queue change | — | Included in T01 correctness scope | One integrated stage. | [Run 01](logs/20261107-s05-bootstrap-worker-log.md); completed by final task commit under Q04. |
| [S05.T01](#s05-t01) | ✅ | Muk: implement and verify queue | Primary agent | 1 Sol high correctness complete; separate simplification waived by user | Queue behavior and environment configuration verified. | [Run 01](logs/20261107-s05-bootstrap-worker-log.md); 95 working-tree tests/checks pass, no findings; final task commit and push authorised. |
