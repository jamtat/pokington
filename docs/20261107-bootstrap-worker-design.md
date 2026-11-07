# Bootstrap worker queue design

Date: 2026-11-07. Status: design and environment extension implemented; all 95 working-tree tests/checks pass and the approved correctness review has no findings. The user waived the separate simplification review and authorised final commit/push; completion takes effect with that commit and publication.

## Scope and sources

Replace the fixed download batches in [bootstrap.rs](../src/bootstrap.rs) with queued work and four workers. Preserve the recovery and atomic publication requirements in [the current design](20261106-design.md) and [brief](brief.md). Existing unrelated changes in `src/command.rs` and `tests/command.rs` are outside this change.

## Accepted direction

- User request: “the concurrency is a bit rubbish, we should just have a queue of all the tasks to do and then <n> workers (hard code 4 for now).”
- Use queued work with a hardcoded worker count of four. Available workers take the next task without waiting for unrelated tasks to finish a batch.
- Replace batch barriers; do not add a worker-count CLI option in this change.
- Design records below govern this change; the current brief retains the existing data and recovery contract.

## D01: queue scope and task execution — 2026-11-07

- A: keep discovery and artwork as separate phases, each using a queue serviced by four workers. A species task includes any required linked game-page requests, fetched sequentially within that task. This keeps catalogue validation, artwork reuse and checkpoint preparation before artwork downloads.
- B: use one dynamic queue for discovery and artwork, adding tasks as pages reveal them. Artwork can start earlier, but checkpoint preparation and reuse need redesign because the complete catalogue is not yet known.
- Recommendation: A removes the observed batch delays while retaining the existing storage and recovery lifecycle.
- User response: “Have an enum of BootstrapTask. Keep these in a queue. Workers pop off the queue. The task is executed (execute method on the task). A task execution also should return more tasks to run (e.g. the national pokedex task will return all the tasks to scrape the individual pokemon pages)”
- Accepted direction: one queue of `BootstrapTask` enum values, four workers popping tasks, and an async `execute` method returning follow-up tasks alongside the task result. Seed the queue with the national Pokédex task, whose execution returns individual species-page tasks. This supersedes the recommendation to use separate queues; task dependencies still determine when work can be created.
- Follow-up tasks are enqueued when execution completes. An empty queue does not mean completion while workers are executing tasks that may return more work. Finish only when both the queue and executing-task count are empty.

## D02: fallible execution result — 2026-11-07

- User clarification: “execution may need to return a result, it could be fallible”
- Accepted direction: `BootstrapTask::execute` returns `Result<BootstrapTaskResult, BootstrapError>`. `BootstrapTaskResult` contains produced data needed by the coordinator and follow-up tasks; failures propagate as errors rather than ordinary task results. HTTP retries remain inside execution. Final variant and field names are bounded implementation details.
- Consequence: workers report execution success or failure to the coordinator; successful follow-up tasks enter the queue, while terminal failures follow D03 below.

## Proposed dependency and ownership details

- Task variants cover the national index, species pages, optional linked game pages, catalogue preparation and artwork. A game-page task carries accumulated species evidence and returns the next required game-page task only if reconciliation still needs it.
- Keep existing checkpoint preparation dependent on complete reconciled discovery. Once all species records are collected, enqueue catalogue preparation; it validates the complete catalogue, resumes/reuses artwork and returns missing-artwork tasks into the same queue. A single queue does not remove that data dependency. Overlapping artwork with incomplete discovery is not inferred from the user's enum/queue instruction.
- Keep storage mutation and progress output serialized; do not hold the queue lock across network I/O. Preserve HTTP validation, retries, exclusive bootstrap writer ownership and atomic final publication.
- Task completion must carry discovered records or downloaded bytes as well as follow-up tasks. Define the smallest concrete result type needed by the coordinator; avoid a general task framework.

## D03: failure handling — 2026-11-07

- A: after retries are exhausted, stop taking queued tasks, let executing tasks finish, save their successful artwork and return the first error. Retain pending state for the next bootstrap.
- B: continue independent queued work after an error, retaining successful work, then report failure after the queue is exhausted. This can complete more work but continues requests after bootstrap is already known to have failed.
- Recommendation: A preserves the current behavior of stopping new work after a failure while retaining successful transfers already underway.
- User selection: “A”
- Accepted decision: stop taking queued tasks after the first terminal failure; drain running tasks, retain successful artwork and return the first error. Do not enqueue their follow-up tasks after failure. Cancellation remains distinct: dropping bootstrap cancels workers and releases the writer.

## D04: task metadata belongs in a wrapper — 2026-11-07

- User instruction: “separate the metadata of tasks from the body, so if you need to track retries then do that on an object that wraps the task enum”
- Accepted decision: `BootstrapTask` contains only the work payload. `QueuedBootstrapTask` wraps it and holds required execution metadata. Keep retry attempts outside the enum; one call to `BootstrapTask::execute` performs one attempt. Preserve the current three attempts and 100 ms/200 ms delays for retryable request failures, without retrying parser or storage failures. Backoff occupies the existing worker rather than creating extra workers.
- Rejected alternative: adding retry fields to every enum variant mixes execution policy with work data.

## D05: simplify reporting — 2026-11-07

- User permission: “you can also drop the requirement for the progress bar if need me”
- Decision: remove progress bars; retain download logs and a final completion summary on stderr. No shared progress-bar state or dynamically recalculated totals are required.
- Consequence: this supersedes the brief's former requirement for both a bar and a download log, solely for bootstrap reporting. Update the brief with a dated clarification.

## D06: smallest implementation meeting the requirements — 2026-11-07

- User instruction: “this implementation should be as simple as possible”
- Accepted decision: direct enum execution, one queue, four worker loops, a small metadata wrapper and serialized result handling. Reuse existing HTTP validation, parsers, storage and publication. Add only state necessary for waiting, termination, retries, errors and cancellation; do not build a general executor framework.
- Dated refinement of the earlier proposed task variants: keep catalogue preparation and publication in the coordinator using existing `Writer` methods. They do not need separate task variants. Network tasks cover the index, species pages, required game pages and artwork. After complete discovery, the coordinator prepares the checkpoint and adds missing-artwork tasks to the same queue. This avoids sharing the writer across workers while preserving current recovery.

## D07: environment-configured concurrency — 2026-11-07

- User extension: “Add a bonus feature, support using POKINGTON_CONCURRENCY=n. default to 4”, clarified by “as an env var”.
- Accepted decision: read `POKINGTON_CONCURRENCY` when bootstrap starts; an unset variable means four workers. Validate a positive integer before writer acquisition or downloads. Reject zero, malformed, overflowing and non-Unicode values with a configuration error. This supersedes the hardcoded-four restriction in the preceding history; retain one queue and the same workers/tasks design.
- Keep the result channel's buffer capacity at four independently of worker count; configuration changes the number of workers, without inflating buffered results or hitting Tokio's channel capacity bound.
- Verify actual default and configured worker counts using isolated child processes and held HTTP fixture responses. Do not mutate the parent test process's environment.

## Implementation stage and records

One stage with one implementation task: replace batching, add focused concurrency and recovery checks, and run the repository checks. [Implementation plan](20261107-bootstrap-worker-plan.md); [separate log index](20261107-bootstrap-worker-log.md).

No material design question remains. The user approved implementation and checks with “yes, feel free to aggressively delete existing code as needed”. The later answer “at high” approves the offered single `gpt-6.1-sol` correctness reviewer at `high`, superseding its proposed `xhigh` setting for this review only. Commit, publication and a separate simplification review remain unapproved. Preserve unrelated working-tree changes.

## Next steps

Present the single implementation stage for approval. At completion, show changes and check results, offer the specifically described independent reviews, and obtain any missing commit approval with a concrete subject and file table. No implementation or review has run during planning.

2026-11-07 implementation update: the stage was approved and implemented. Formatting, check, strict Clippy and all 92 tests pass; [Run 01](logs/20261107-s05-bootstrap-worker-log.md) records fixture concurrency/recovery evidence and execution approval. The preceding next-step text records the planning boundary. Independent reviews and a final commit remain pending approval.

2026-11-07 extension/review update: D07 is implemented; all 95 tests and checks pass. The single specifically approved `gpt-6.1-sol` correctness review at `high` reports no findings. This supersedes the preceding 92-test and pending-correctness-review status; a separate simplification review and final commit remain pending approval.

2026-11-07 final-stage correction: “n, commit and push” declines the separate simplification review and authorises the final bootstrap commit and push of existing `main` to `origin/main`, including triggered Actions. This supersedes the preceding pending-review/commit status for S05. No PR or manual CI operation is approved. Preserve unrelated command changes outside the commit. [Run 01 Q04](logs/20261107-s05-bootstrap-worker-log.md) records exact scope and completion rules.
