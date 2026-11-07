# S01.T02 inline native-image prototype

Date: 2026-11-07. Automated evidence, one failed manual detection attempt and a successful corrected actual Ghostty validation. At 2026-11-07T02:45:24Z the user explicitly confirmed the coloured native image, inline persistence and normal prompt/typing after both runs. The viewport correction below supersedes the initial divisibility rule; native success is established by the actual reports/capture plus this user answer, not environment hints or tool PTYs.

## Implementation

- `src/inline_image.rs` supplies conservative selection, transparent cropping, exact original-size Kitty encoding through `ratatui-image`, and inline output cleanup. `examples/inline_image_probe.rs` is a runnable feasibility probe; it does not implement the product `art` command. Automatic fallback reports the selected block/ASCII renderer; full text artwork remains S04 work.
- Detection requires terminal stdin/stdout identifying the same device, paired Ghostty hints (`TERM=xterm-ghostty`, `TERM_PROGRAM=ghostty`, `TERM_PROGRAM_VERSION=1.4.0`), no detected tmux/screen/SSH context, and usable `TIOCGWINSZ` pixel/cell metrics. Bare `TERM` is insufficient. Missing, zero or ambiguous cell metrics fail conservatively; Ghostty 1.4.0 viewport remainders use the verified inverse described below. Other terminal/version measurement profiles are unverified and fail safely, including the Kitty application; Kitty graphics remains the output protocol. Hints select a candidate protocol; they do not prove that a terminal displays it.
- Detection performs a fixed number of local metadata calls. It sends zero escape queries, reads zero input bytes, opens no controlling terminal, starts no background reader and waits for no user response. There is therefore no response timeout or delayed-response/input-draining problem. Redirected stdout selects plain ASCII in auto mode and produces a clear error in explicit img mode. Missing stdin also fails safely. This deliberately avoids `Picker::from_query_stdio`, whose current implementation changes termios and reads stdin in a worker thread. [Upstream Picker source](https://raw.githubusercontent.com/ratatui/ratatui-image/v11.1.0/src/picker.rs).
- `Kitty::new` and the `Image` widget encode/render to an in-memory Ratatui buffer. The probe serializes its native symbols and foreground styles as inline rows. It never creates a full-screen Ratatui terminal, changes termios, hides the cursor or switches screens. Kitty transmissions use `q=2`, suppressing terminal replies. [Upstream Kitty encoder](https://raw.githubusercontent.com/ratatui/ratatui-image/v11.1.0/src/protocol/kitty.rs).
- Cropping includes every nonzero alpha pixel. Visible RGBA pixels are copied without interpolation to a transparent canvas padded only on the right/bottom to full cells. Payload pixel dimensions equal placement columns/rows multiplied by measured cell pixels. This matters because Ghostty's virtual-placement renderer scales into its rounded cell grid even when `c`/`r` are omitted. Equal canvas/grid dimensions make that scale exactly one. [Ghostty placement implementation](https://raw.githubusercontent.com/ghostty-org/ghostty/main/src/terminal/kitty/graphics_unicode.zig), [Kitty placement specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/#unicode-placeholders).
- Original output can overflow: the full padded payload remains unchanged; placeholders clip at the terminal's right edge, and rows print/scroll inline. The prototype rejects the upstream 297-cell-per-axis placeholder limit instead of silently dropping cells. It also bounds the padded payload to 16 megapixels and PNG decoding to 4096 pixels per axis/64 MiB. These are explicit prototype limits, not a change to accepted product sizing semantics.
- Success ends each image row with an SGR reset and CR/LF. A partial write error or unwinding makes a best-effort CAN/ST/SGR-reset/newline cleanup. A permanently broken writer is reported as unable to receive cleanup. Raw mode, cursor visibility and screen selection are preserved because the prototype never changes them. Existing terminal scrollback is not cleared.

## Dependencies and checks

Resolved versions: `ratatui-image 11.1.0`, `ratatui 0.30.2`, `image 0.25.10`, `rustix 0.38.44`; test/probe support `nix 0.31.3` and test payload decoder `base64 0.22.1`. Ratatui/image defaults are disabled; only PNG decoding is enabled. No Chafa/system graphics dependency or terminal event backend is required. Source transport/parser dependencies remain unchanged.

Checked on macOS with Rust 1.99.1. Required commands completed successfully:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Initial implementation checks passed all 41 integration tests: eight new native-image tests and all 33 existing source tests. The viewport correction below adds a ninth native-image test. Existing `btls-sys` linker warnings about missing `out/build/lib` and `out/build/crypto` search directories still appear during linking; they were not suppressed. A subsequent test link also warned that cached zstd/btls objects target macOS 27.2 while the executable targets 11.0. These checks do not establish Linux runtime behavior or source transport portability.

New tests decode the actual multi-chunk Kitty RGBA payload and compare every visible pixel and every padded transparent pixel against a PNG round trip at four cell sizes. They assert native commands, quiet mode, chunk bounds, no compression dependency, placement/payload equality, odd crop dimensions, partial alpha, overflow and explicit library/allocation limits. Own-PTY tests preserve queued user input and exact termios on success, failed detection and partial output failure, verify absence of query output, and complete detection within 500 ms without a response peer. PTYs are safety evidence, not image-display evidence.

Executable checks also passed:

| Invocation | Observed result |
| --- | --- |
| `target/debug/examples/inline_image_probe --mode auto --record /private/tmp/pokington-inline-redirected-auto.json > /private/tmp/pokington-inline-redirected-auto.txt` | Exit 0, plain `fallback=Ascii` decision, no escape codes; detection 1 µs |
| `target/debug/examples/inline_image_probe --mode img --record /private/tmp/pokington-inline-redirected-img.json > /private/tmp/pokington-inline-redirected-img.txt` | Exit 1, stdout empty, unsupported-image diagnostic; detection 1 µs |
| `target/debug/examples/inline_image_probe --mode img --record /private/tmp/pokington-inline-unavailable-input.json < /dev/null` in an owned tool PTY | Exit 1, unavailable-stdin diagnostic, no graphics/query output; detection 1 µs |

Those processes inherited Ghostty 1.4.0 environment hints, but their output was redirected or an owned tool PTY. They are explicitly not actual Ghostty rendering evidence.

## Ghostty 1.4.0 viewport correction — 2026-11-07

The user ran the first manual command in Ghostty 1.4.0. Both invocations rejected detection before emitting graphics: ioctl reported 105 columns, 59 rows and a 1801×2196 pixel viewport. Input/output were terminals, no multiplexer/remote context was detected, and termios remained equal. This is a real failed detection result, not native-display or mid-write-recovery evidence. Original reports were preserved as `/private/tmp/pokington-inline-ghostty-first-failure.json` (SHA-256 `d2bada02baafa8cd6f5763f421aa07b55013d8371f2513f73a319430cf90a7ae`) and `/private/tmp/pokington-inline-ghostty-first-error-failure.json` (`2d06a36237b9f3a3c9aa3b2777a4fcba83d6a579e74f13bc694612f22cbf0cbc`). Root separately archived the original reports in `docs/logs/20261107-inline-ghostty-attempt01*.json`.

The original divisibility requirement was too strict. Pinned source establishes the measurement relationship:

- Ghostty v1.4.0 [`Size.grid`/`Size.terminal`, lines 33–43](https://github.com/ghostty-org/ghostty/blob/v1.4.0/src/renderer/size.zig#L33), both subtract the same configured/balanced padding. [`GridSize.update`, lines 253–261](https://github.com/ghostty-org/ghostty/blob/v1.4.0/src/renderer/size.zig#L253) truncates the division by integer cell pixels and clamps grid counts to at least one. Therefore partial-cell viewport leftovers are valid; the application must not subtract padding again.
- [`Termio.resize`, lines 486–506](https://github.com/ghostty-org/ghostty/blob/v1.4.0/src/termio/Termio.zig#L486) sends that grid and padding-subtracted viewport to the PTY backend. [`Exec.Subprocess.resize`, lines 1042–1059](https://github.com/ghostty-org/ghostty/blob/v1.4.0/src/termio/Exec.zig#L1042) exports them as ioctl rows/columns/pixels, saturating each field at `u16::MAX`. Internal terminal image geometry separately receives the actual cell dimensions.

For viewport pixels `P`, grid count `n > 1` and integer cell pixels `c`, `n = floor(P/c)` implies the inclusive interval `[floor(P/(n+1))+1, floor(P/n)]`. The prototype accepts only when both bounds are equal. It rejects the one-cell clamp, saturated ioctl fields, impossible values and ambiguous intervals. This inverse applies only to the checked `TERM_PROGRAM_VERSION=1.4.0` Ghostty profile; fractional metrics from other versions remain unsupported. It does not simply round down viewport/grid and assume an exact cell size.

The observed width interval is `[floor(1801/106)+1, floor(1801/105)] = [17,17]`; height is `[floor(2196/60)+1, floor(2196/59)] = [37,37]`. The viewport remainders are `1801−105×17=16` and `2196−59×37=13`. Thus the 131×97 crop needs an 8×3-cell, 136×111-pixel transparent canvas. The JSON now records the method, both candidate intervals and both remainders alongside the actual ioctl metrics.

The added own-PTY regression uses the exact observed ioctl tuple, asserts those dimensions, decodes the Kitty payload and compares all 131×97 visible pixels and the transparent remainder without resampling. It also rejects ambiguous intervals even when viewport/grid division is exact, one-cell/clamped sizes, and the same fractional tuple under an unverified version. All 42 integration tests and the required formatting/locked Clippy/whitespace checks pass after this correction. No input queries, protocol adapter, production sizing-contract change or actual native-display pass was introduced.

## Actual attempt 02 result — 2026-11-07T02:45:24Z

The user ran the revised block in Ghostty 1.4.0 and reported `success=0 expected_failure=1 terminal_settings=unchanged`. The [success](logs/20261107-inline-ghostty-attempt02.json) and [fault](logs/20261107-inline-ghostty-attempt02-error.json) reports preserve actual 105×59/1801×2196 viewport metrics and unique 17×37 physical cells. The native frame completed; the error run injected after 30 bytes and flushed all nine cleanup bytes. Neither run read input or changed termios/screen/cursor modes.

Root independently decoded all 20 captured Kitty chunks and compared every RGBA byte against the actual calibration PNG crop and transparent padding. Visible pixels are 131×97; payload and placement are 136×111 pixels (8×3 cells), with scale one. Capture SHA-256 is `39a00f2515772fc1c0602a3ac20645c72bb20c91270bb4c94d6cf7bc36e31741`. The user then answered exact `y` to seeing the four-colour rectangle remain inline with normal typing afterward. [Combined pixel/visual validation](logs/20261107-inline-ghostty-attempt02-payload-validation.json).

This is actual macOS/Ghostty evidence for the S01 gate, not Linux runtime acceptance or a completed product `art` command. T02 correctness/stage-integration review, separate simplification and approved final commit remain pending.

## Correctness finding N1 resolution — 2026-11-07

The independent correctness review found that missing/unverified Ghostty versions fell through to generic exact viewport/grid division. A 105×2-cell, 1785×100-pixel viewport was accepted as 17×50 cells even though the verified Ghostty model permits heights throughout `[34,50]`. Divisibility alone cannot prove exact cell dimensions.

Removed that fallback. Native detection now requires the verified Ghostty 1.4.0 measurement profile; missing versions, 1.4.1, unknown versions and Kitty-application hints return an explicit unverified-profile error. Auto selects a safe text fallback, while img errors. No additional profile was inferred or researched. The product sizing/no-input contract is unchanged; this prototype's established native terminal is Ghostty 1.4.0 using Kitty graphics.

The owned-PTY regression covers the exact divisible ambiguous tuple for all rejected profiles, auto/img behavior, zero query output, preserved queued input and unchanged termios. The verified 1.4.0 profile also rejects that tuple, while the actual 105×59/1801×2196 tuple still passes with exact pixel geometry. All 43 tests (10 rendering, 33 source), formatting, locked all-target Clippy and whitespace checks pass.

The verified renderer/cleanup source is byte-identical before/after N1. A newly encoded frame for the actual metrics was compared with the user-observed attempt02 capture: all 60,384 RGBA bytes are identical, SHA-256 `7c5393c3f712ec81aaa65ae02cd71e77743fd977df7d4cc6b8ebcf58a399d4c6`. The complete terminal frame is also identical after normalizing only its random image identity: one ID header, three RGB foreground sequences and three high-byte combining marks. Normalized frame SHA-256: `17c46f9342187eb985aee684a825c381a8e4b4e48cad2b14baa001ae0de5ab70`. Comparison record: `/private/tmp/pokington-inline-n1-frame-comparison.json`. The temporary test capture statement was removed; permanent tests do not write files. Existing actual visual/recovery evidence remains applicable; no user rerun was requested or claimed.

## Final simplification and checks — 2026-11-07

The approved separate simplification review found no blockers. Its two accepted cleanups make successful measurement intervals non-optional arrays (JSON unchanged) and move calibration generation to shared probe/test support. All 43 tests, formatting, locked all-target Clippy and whitespace checks pass after these changes. [Comparison](logs/20261107-inline-simplification-comparison.json) confirms identical positive detection JSON, byte-identical calibration PNG (SHA-256 `988a56d30a3651de2994623217c83801c291e3ea755e5e3b66277ff39926d062`) and the same normalized native frame/RGBA data as the actual successful user run. No user rerun or further source/terminal queries were needed.

Correctness/stage-integration review is complete, its N1 profile issue is resolved, and both S01 gates have their required evidence. The final local S01.T02 commit is authorised by the user's Q19 autocommit instruction; S01/B01 completion is conditional on its successful Git operation. No product `art` command, Linux runtime acceptance, publication or S02 implementation is claimed.

## Manual Ghostty validation

Run the following block in a normal Ghostty shell, outside tmux/screen/SSH. It does not read input or wait for a response. Use the already built executable so build messages do not obscure the image. The first run generates and decodes a 151×117 PNG with a 131×97 visible crop, prints the native image and saves the exact output/metrics. The second run forwards and flushes a short prefix ending inside the first Kitty APC header, fails once, then permits the existing escape/style cleanup. The shell compares terminal settings across both runs.

2026-11-07 refinement: the earlier proposed manual error case stopped before output and could not demonstrate recovery from a partial escape sequence. It has been replaced by `--fail-mid-write`. The same probe-only fault writer is now exercised in the own-PTY tests, which verify exact forwarded prefix/cleanup bytes, unchanged termios and preserved queued input. The error JSON records injection, byte counts and whether all nine cleanup bytes were forwarded and flushed; the failed frame retains `native_emitted=false`.

```sh
(
  cd /Users/ben/code/pokington || exit
  pokington_tty_before=$(stty -g) || exit
  ./target/debug/examples/inline_image_probe --mode img --width original --record /private/tmp/pokington-inline-ghostty-attempt02.json --payload /private/tmp/pokington-inline-ghostty-attempt02.bin --fixture /private/tmp/pokington-inline-calibration-attempt02.png
  pokington_success=$?
  ./target/debug/examples/inline_image_probe --mode img --width original --fail-mid-write --record /private/tmp/pokington-inline-ghostty-attempt02-error.json
  pokington_failure=$?
  if [ "$pokington_tty_before" = "$(stty -g)" ]; then pokington_tty_result=unchanged; else pokington_tty_result=CHANGED; fi
  printf 'success=%s expected_failure=%s terminal_settings=%s\n' "$pokington_success" "$pokington_failure" "$pokington_tty_result"
)
```

Expected: `success=0 expected_failure=1 terminal_settings=unchanged`; the four-colour rectangle has a thin white border and central alternating black/white vertical pixel stripes, remains inline after exit, and the prompt/cursor/input echo behave normally. Confirm the image is a native image rather than replacement characters or text blocks. Record the actual visual appearance, persistence and input/cursor recovery; if it fails, preserve the diagnostic rather than forcing environment hints or supplying guessed cell dimensions. The JSON report contains actual terminal hints and ioctl metrics, crop/payload/grid sizes, and before/after termios equality. Its visual-confirmation field intentionally remains pending until a human result is recorded separately. Do not `cat` the `.bin` file: it contains terminal graphics escape sequences.

The optional `--image /private/tmp/pokington-illustration-validation/images/pokemon/art/001.png` can subsequently exercise the already validated source PNG. The synthetic calibration is the primary sizing evidence. No source fetch, computer/browser control, review, commit, publication or next-stage implementation occurred in this task implementation pass.
