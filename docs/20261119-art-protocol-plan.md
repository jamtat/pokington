# Native artwork protocol implementation plan

## Continuation prompt

```text
Use $jamtat-work to continue the native artwork protocol change using docs/20261119-art-protocol-plan.md and docs/20261119-art-protocol-log.md.

Read the plan, design, current brief and code, log index and canonical run record. If scope is unspecified, recommend the next incomplete task. Obtain missing approval for scope, branch, model assignments, independent reviews, commits and publication including triggered CI. Record the exact answers and their limits before acting. Use existing grants only within the recorded run and scope.

Complete the approved task, including its checks, separate correctness and simplification reviews and stage exit review. Show the proposed commit subject and file/change table, include tracking and log updates, and make the approved task commit. Report verification, reviews, commit and publication status. Publish only with a recorded grant. Manual CI and infrastructure mutations require separate approval.
```

Date 2026-11-19. One stage is proposed. See the [design decision](20261119-art-protocol-design.md), [shipped contract](brief.md#art), and [implementation log](20261119-art-protocol-log.md). User instructions and recorded choices govern the run. The design defines the requested change. The brief defines existing behavior outside that change.

## Scope and reuse

| Capability | Existing location | Implementation boundary |
| --- | --- | --- |
| Artwork CLI and command validation | [command.rs](../src/command.rs) and [art.rs](../src/art.rs) | Add a typed protocol option with three values. Reject its use outside `--mode img` with argument-error status before creating the runtime. |
| Protocol detection, sizing and rendering | [inline_image.rs](../src/inline_image.rs) and [command.rs](../src/command.rs) | Reuse the picker probe, terminal checks, encoder and frame writer. Apply the requested `Picker::set_protocol_type` before checking for Halfblocks. Reject zero Picker cell dimensions before sizing. |
| CLI fixtures and terminal simulation | [command.rs tests](../tests/command.rs) and [inline_image.rs tests](../tests/inline_image.rs) | Extend CLI and injected-Picker or PTY coverage using local artwork fixtures. |

The pinned `ratatui-image` version already supports all three encoders and exposes the setter. No dependency change is needed. Preserve the Picker's cell size and tmux handling. Terminal probing errors and renderer constraints can still fail an explicit image invocation. Artwork, asset selection, protocol encoders and terminal transport are outside the change.

## Stage sequence

<a id="s01-t00"></a>
### S01 — Bergmite, selectable native artwork output

Prerequisites are the recorded run approval and a working Rust toolchain. The stage contains one implementation task.

<a id="s01-t01"></a>
#### S01.T01 — Mareep, add the `art --protocol` flag

- Add `--protocol kitty|sixel|iterm2`, validate the mode constraint, and route explicit selection through the existing native renderer. Update [README](../README.md), [artwork usage](20261107-artwork-usage.md), and the [brief](brief.md#art).
- Run focused command and inline-image regression cases, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --all-targets --all-features --locked --no-fail-fast`, and `git diff --check`. The normal commit hook and PR CI run `cargo test-ci` with Nextest. Record local commands, results and any environmental gaps.
- Complete independent correctness and Rust simplification reviews. Stage exit requires full verification, disposition of findings, and an independent review of the acceptance evidence, CLI help and user documentation.

## Requirement-to-acceptance mapping

| Requirement | Task | Evidence |
| --- | --- | --- |
| Force Kitty, Sixel or iTerm2 in native mode | S01.T01 | Test that each option produces the matching output sequence. Include a Picker that selected a different protocol and one that selected Halfblocks. |
| Limit the option to explicit image mode | S01.T01 | Help exposes the three values. Unknown values and use with omitted mode or `auto`, `blocks`, or `ascii` fail with status 2 before runtime work. |
| Keep existing stream checks and behavior without an override | S01.T01 | Forced output with redirected stdout fails without emitting a graphics payload. Existing native detection, terminal restoration, `img` errors and `auto` fallback tests pass. |
| Report invalid Picker cell dimensions | S01.T01 | Native rendering reports an unsupported geometry error for zero-width or zero-height cells, including with `--width original`. |

Automated checks can verify the emitted bytes and terminal-probe interaction. Visual testing in each emulator is outside this stage's acceptance scope.

## Stage and task tracking

Each review assignment is one fresh findings-only reviewer. Done requires the documented checks and reviews. Stage completion also requires its exit review.

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S01.T00](#s01-t00) | ✅ | Bergmite, stage completion | — | 1 × `gpt-6.2-sol` `high`, stage exit | Compare CLI help and documentation with the contract and test evidence. | [Stage log](logs/20261119-s01-art-protocol-log.md) |
| [S01.T01](#s01-t01) | ✅ | Mareep, native artwork protocol flag | `gpt-6.2-sol` `high` | 1 × `gpt-6.2-sol` `high`, correctness. 1 × `gpt-6.2-sol` `high`, simplification. | Trace CLI validation and protocol selection. Review Rust idioms and fixture reuse independently. | [Stage log](logs/20261119-s01-art-protocol-log.md) |
