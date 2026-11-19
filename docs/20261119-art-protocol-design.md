# Native artwork protocol design

Date 2026-11-19. The CLI choice is accepted. The implementation scope is awaiting approval.

## CLI choice

- Accepted interface: `pokington art Mew --mode img --protocol kitty`. The selectable values are `kitty`, `sixel`, and `iterm2`. Omitting `--protocol` keeps automatic protocol detection. Supplying it with any other mode or with the default mode is an argument error.
- Alternative considered: values such as `--mode img:kitty` make the invocation shorter. A separate flag keeps the existing `--mode` values stable and lets Clap describe and validate the protocol values independently.
- User response to the separate-flag recommendation: “ok, can you please plan the change and work on it using the jamtat-work skill”

## Terminal behavior proposed for implementation

- Use the existing `ratatui-image` Picker and terminal probe. Apply the explicit protocol after obtaining the Picker so the requested encoder wins even when the probe detects a different protocol or yields Halfblocks. The probe still supplies cell dimensions and handles tmux. Preserve the existing matching terminal stdin/stdout requirement and propagate probe IO failures.
- An explicit protocol is the user's assertion of terminal support. If rendering or writing fails, report the error. An emulator can ignore a graphics sequence without reporting an error to Pokington.
- This narrows [D16](20261106-design.md#d16--unsupported-explicit-image-mode--2026-11-06) and [D33](20261106-design.md#d33--use-ratatui-image-detection-and-protocols--2026-11-07) only for explicit protocol selection. The [current brief](brief.md) continues to describe shipped behavior until implementation. Update it in the same change as the flag.

## Proposed stage

S01 adds the flag, selects the encoder, extends CLI and terminal regression coverage, and documents the behavior. See the [implementation plan](20261119-art-protocol-plan.md) and [log index](20261119-art-protocol-log.md).

## Implementation update, 2026-11-19

The approved implementation adds the flag and terminal behavior above, including an error for zero terminal cell dimensions. The [brief](brief.md#art) records the change and the [stage log](logs/20261119-s01-art-protocol-log.md) records verification and review.
