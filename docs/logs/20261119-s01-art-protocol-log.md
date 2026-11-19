# Native artwork protocol implementation log, S01

[Implementation plan](../20261119-art-protocol-plan.md) · [Log index](../20261119-art-protocol-log.md)

## Run 01 — S01.T01

- Preparation recorded at 2026-11-19T14:16:54Z.
- Status open.
- Originating invocation (exact): “ok, can you please plan the change and work on it using the jamtat-work skill”
- Proposed scope is S01.T01 and the S01 stage exit. The starting branch is `main` at `934c8c4`. The proposed branch is `btat-art-protocol-s01-t01`.
- The proposed assignments, checks and stage exit criteria are in the plan. Scope, exact branch, reviews and commit are pending explicit approval.
- Publication instruction (exact): “when done branch off and open PR and link me to it”
- Publication covers pushing the completed feature branch, opening its PR against `main`, and the CI triggered by these actions. The proposed head is `origin/btat-art-protocol-s01-t01` on `jamtat/pokington`.

### Model and reasoning effort

- Recorded at 2026-11-19T14:20:01Z.
- User instruction (exact): “sol-6.2 high for all parts, no astra today”
- Use `gpt-6.2-sol` at `high` for implementation, task correctness review, task simplification review, and stage exit review. This supersedes the model and effort settings in the initial run proposal. Each review retains its proposed scope and one fresh reviewer. The run approval is pending.

### Run approval

- Recorded at 2026-11-19T14:27:15Z.
- Question (exact): “Reply `y` to approve the revised `jamtat-work` run, including the branch, reviews, and task commit. Then I’ll implement the flag, open the PR against `main`, and send you the link.”
- User response (exact): “y”
- Approved S01.T01 implementation by one `gpt-6.2-sol` `high` agent on new branch `btat-art-protocol-s01-t01` from `main`, the plan checks, and one fresh `gpt-6.2-sol` `high` reviewer each for task correctness, Rust simplification, and stage exit evidence and documentation. Approved one task commit `art: S01.T01 add native protocol override` following those checks and reviews. The prior push and PR grant remains in effect for `origin` and `main` and includes automatically triggered CI. The run scope ends with the PR.

## S01.T00 stage execution

- Start 2026-11-19T14:27:42Z (first clock read after branch creation and agent launch).
- Finish 2026-11-19T14:37:13Z (first clock read after the stage review).
- Exit evidence requires the task checks, review findings and dispositions, and the stage exit review.
- The stage exit reviewer (`gpt-6.2-sol`, `high`, one fresh reviewer) confirmed the 114-test evidence, CLI help and status-2 error, and found no code or acceptance blockers. Applied both documentation recommendations by adding a dated implementation update to the design and clarifying the unsupported-graphics behavior in the dated brief correction. The mode conflict error retains Clap's top-level usage with the explicit `--mode img` requirement.

## S01.T01 native artwork protocol flag

- Start 2026-11-19T14:27:42Z (first clock read after agent launch).
- Finish 2026-11-19T14:37:13Z (first clock read after the stage review).
- Implementation assigned to `gpt-6.2-sol` at `high`.
- The branch was created from `934c8c4`. GitHub repository access and the `main` PR base were verified outside the restricted network sandbox.
- `src/command.rs` adds the protocol enum and validates its mode before runtime creation. It applies the explicit protocol to the acquired Picker. The tests cover help, invalid combinations, each encoder, and redirected output. The README, artwork guide and brief describe usage and terminal requirements.
- Focused checks passed via `cargo test --locked --test command --test inline_image` (26 tests). `cargo fmt --check`, strict all-targets Clippy and `git diff --check` passed. The full `cargo test --all-targets --all-features --locked --no-fail-fast` run passed 113 tests with loopback access for the mock HTTP servers. The initial baseline run in the restricted sandbox could not bind those servers.
- `target/debug/pokington art --help` exposed the three protocol values and the mode requirement. A binary invocation with `--mode auto --protocol kitty` returned status 2. Verification uses encoded output and simulated terminals. No visual emulator check was required by the plan.
- Task correctness review (`gpt-6.2-sol`, `high`, one reviewer) reproduced P2 in a PTY with 80×24 cells and a reported 79×480 pixel area. The Picker fell back to Halfblocks with a zero cell width. Overriding to Kitty reached division by zero with `--width original`. The implementation is adding an Unsupported error before sizing and a regression test. Follow-up is pending within the same review.
- Correctness follow-up resolved P2. `NativeFrame::render` rejects zero cell dimensions before sizing. The reviewer verified both zero axes with all three width modes in a PTY. Positive and unknown pixel measurements still encoded Kitty and terminal settings were restored. No remaining correctness findings.
- After the fix, the focused command and inline-image suites passed 27 tests. The full suite passed 114 tests with loopback access. Formatting, strict Clippy, and diff checks passed again.
- Separate simplification review (`gpt-6.2-sol`, `high`, one fresh reviewer) found two optional reductions and no blockers. Accepted the in-place mutation of the acquired Picker result, removing a closure and shadowed binding. Kept the existing native-mode test fixture and the new protocol test setup separate so each terminal configuration remains visible at its call site. Repeated the full checks after the production edit.
- Final formatting, strict Clippy, diff checks and all 114 tests passed after simplification. The macOS test linker reported a non-failing deployment target warning for an AWS LC dependency object.
- Approved task commit is `art: S01.T01 add native protocol override`. Publication follows the commit under the recorded grant.
- The first commit attempt passed formatting, compilation, and Clippy hooks. The test hook stopped because Nextest was missing. Downloaded the pinned Nextest 0.9.151 macOS binary from its official GitHub release into a temporary directory, verified its SHA-256 checksum, and added that directory to the commit process's PATH for the retry with normal hooks.
