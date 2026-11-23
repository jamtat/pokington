# Artwork export implementation plan

## Continuation prompt

```text
Use $jamtat-work to implement artwork export using docs/20261122-art-output-plan.md and docs/20261122-art-output-log.md. Create the stage log at the first run preparation entry.

Read applicable instructions, the accepted design, the current brief, and the repository state. If I have not supplied scope, propose S01.T01. S01.T00 records stage completion. Before acting, present the proposed scope, exact branch action, implementation settings, and publication choice including automatically triggered CI. Include the proposed reviewer model, effort, count, and scope for each review. Obtain and record my explicit approval for each review and the task commit. Show the proposed commit title and a short table of changes by file. An earlier commit or review grant does not cover this work.

For the approved task, run the checks, record actual results and any deviations in the stage log, complete the approved reviews, and commit with the task ID after approval. Report the completed IDs and verification, review, commit, and publication results. Push, open or update a PR, or manually run CI only under the applicable recorded permission.
```

Date 2026-11-22. See the [S01 implementation log](20261122-art-output-log.md) for current status and approvals.

## Scope and sources

The [accepted design](20261122-art-output-evaluation.md#design-update-2026-11-22) governs the export changes. The [current brief](brief.md#art) governs existing behavior. User instructions and permissions govern execution. The initial proposal remains part of the design history.

- Add `--mode png` for binary PNG stdout and `--mode markdown` for one Markdown image reference with Base64 PNG data and a trailing newline. Use the selected form's display name as escaped alt text.
- Both modes use original dimensions after trimming fully transparent margins when `--width` is omitted or `original`. `--width N` specifies a positive pixel width with aspect-ratio preservation. Terminal modes keep their column units and fit behavior.
- Use the existing species and form selection, exclusions, offline store, and error reporting. Exporting should work with redirected stdout and should avoid terminal protocol probing. Preserve image colour and alpha.
- Reject `--name` and its dependent `--no-newline` in export modes to keep stdout valid. The existing `--protocol` validation restricts it to `img`. Document that `--color` applies to terminal text output. The previous removal of `--title` is in commit `1038cdb`.

## Repository starting point and reuse

| Capability | Location | Implementation boundary |
| --- | --- | --- |
| CLI, validation, and output | [command.rs](../src/command.rs) · [art.rs](../src/art.rs) | Add two mode values and route exports after selection and PNG decoding. Reuse the width parser and writer error handling. |
| Image preparation | [art.rs](../src/art.rs) · [inline_image.rs](../src/inline_image.rs) | Reuse visible-pixel cropping and alpha-aware resizing. Compute raster dimensions directly from the cropped image, using the existing output size bound. |
| Encoding | [Cargo.toml](../Cargo.toml) | Reuse the enabled PNG encoder. Make the existing Base64 crate a runtime dependency and use standard padded Base64 with no line wrapping. Prepare the image before writing the export. |
| Verification and usage | [command tests](../tests/command.rs) · [art tests](../tests/art.rs) · [README](../README.md) · [artwork usage](20261107-artwork-usage.md) | Extend the local fixture tests and document the new modes, width units, redirection, and renderer compatibility. Update the brief with the implemented behavior. |

## Stage sequence

<a id="s01-t00"></a>
### S01 — Farigiraf, export artwork for files and Markdown

Prerequisites are recorded scope and branch approval and a working Rust toolchain. For full-suite verification, the HTTP fixtures need loopback access. Existing cached or fixture artwork suffices for export acceptance.

<a id="s01-t01"></a>
#### S01.T01 — Teddiursa, add PNG and Markdown modes

- Implement shared raster preparation and PNG encoding, then use the encoded image in the two stdout formats. Preserve selection, argument, store, decode, and writer error handling.
- Cover default, `original`, and numeric widths in both modes. Exercise original and alternate-form selection and the output of a label requiring Markdown escaping. Check corrupt or transparent PNGs, invalid arguments, and incompatible options before output. Use the existing command harness and injected terminal geometry to check that presentation settings do not affect export dimensions.
- Document the two workflows and their size behavior. Include a relative PNG link example for GitHub and an embedded Markdown example for compatible renderers. Run `cargo test --locked --test command --test art --test inline_image` and `git diff --check`. The normal [commit hooks](../prek.toml) run formatting, compilation, Clippy, and `cargo test-ci`. Record the focused checks and hook results in the stage log.

Stage exit requires the acceptance checks below, completed reviews with findings resolved, the scoped task commit, and a stage log linking the evidence. Review the final CLI help and usage examples against the accepted design. Real terminal display and hosted renderer changes are outside this stage's acceptance scope.

## Requirement-to-acceptance mapping

| Requirement | Task | Acceptance evidence |
| --- | --- | --- |
| Export a PNG file | S01.T01 | Stdout decodes as one PNG with the expected dimensions and alpha. No terminal control bytes or label are appended. |
| Export a self-contained Markdown image | S01.T01 | Stdout contains one image reference and newline. Its escaped alt text describes the selected form, and decoding the data URL yields the same PNG as the binary export for the same inputs. |
| Use one sizing policy for both export modes | S01.T01 | For a fixture with transparent margins, omitted width and `original` preserve the cropped pixels. Numeric width sets pixel width and preserves aspect ratio after dimension rounding. Changing terminal geometry leaves output unchanged. |
| Preserve CLI and terminal behavior | S01.T01 | Help includes both modes. Argument conflicts fail with status 2 before runtime creation. Missing, invalid, or fully transparent artwork fails with empty stdout. Existing native, `auto`, blocks, and ASCII tests pass. |

## Deferred work

Raw Base64 and standalone data URLs remain available as future options if a scripting caller needs them. WebP, Braille, clipboard integration, and new sprite sources are separate proposals. [Format evaluation](20261122-art-output-evaluation.md#other-output-modes).

## Stage and task tracking

Model and reasoning settings are proposals. Each review requires explicit user approval. A review assignment specifies one independent reviewer per listed scope. Leave Done blank until the required checks and reviews pass.

| ID | Done | Stage or task | Implementation | Review | Rationale | Comments |
| --- | --- | --- | --- | --- | --- | --- |
| [S01.T00](#s01-t00) | ✅ | Farigiraf, stage completion | — | 1 × `gpt-6.2-sol` `medium`, stage exit | Compare CLI help, documentation, and test evidence against the design. | Exit review and all commit hooks passed. [Log](20261122-art-output-log.md) |
| [S01.T01](#s01-t01) | ✅ | Teddiursa, add PNG and Markdown modes | `gpt-6.2-sol` `high` | 1 × `gpt-6.2-sol` `high`, correctness. 1 × `gpt-6.2-sol` `medium`, simplification. | Check binary and Markdown output, width behavior, and terminal interactions. Review the Rust structure and reuse separately. | Committed as `01df76e`. [Log](20261122-art-output-log.md) |
