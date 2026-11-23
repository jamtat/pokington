# Artwork commands

Run `pokington art` to sample uniformly from installed species/form artwork. The first data command downloads the complete catalogue and artwork; subsequent commands use local data offline. Use `pokington bootstrap` to refresh or repair data, or `pokington bootstrap --force` to redownload it.

```sh
pokington art Charizard
pokington art Sandshrew --form alolan --name
pokington art 6 --form 'mega x'
pokington art Meowth --form galarian --mode blocks
pokington art --form gigantamax
pokington art Charizard --mode ascii --width 64 --color never
pokington art Charizard --mode img --width original
pokington art Mew --mode img --protocol kitty
pokington art Mew --mode png > mew.png
pokington art Mew --mode markdown --width 64 > mew.md
```

Names and descriptive form keys ignore case. Named species default to the original form. An invalid form lists that species's available keys; a random form filter with no matches reports an error.

| Option | Behavior |
| --- | --- |
| `--name` | Print the selected form's name below terminal artwork. |
| `--no-newline` | Omit the newline after the printed name so the caller can append text. Requires `--name`. |
| `--mode auto` | Native graphics when verified; otherwise Unicode blocks in a suitable terminal, then ASCII. Redirected output uses ASCII. |
| `--mode img` | Native images only; unsupported contexts report an error suggesting text modes. |
| `--protocol kitty\|sixel\|iterm2` | Force an image protocol supported by your terminal. Requires explicit `--mode img`. |
| `--mode blocks` | Two sample rows per character using Unicode half-blocks; monochrome output retains upper/lower positions. |
| `--mode ascii` | Brightness mapped to ASCII characters. |
| `--mode png` | Binary PNG on stdout. |
| `--mode markdown` | One Markdown image reference with Base64 PNG data and the selected form's name as alt text. |
| Omitted width | Fit terminal width and height, or use 80 columns for redirected text. Export modes use the cropped original size. |
| `--width N` | Positive width in pixels for PNG and Markdown, or in character columns for terminal rendering. |
| `--width original` | Crop only. Native and export modes retain source pixels. Blocks pack two pixel rows per character row. ASCII uses one source pixel per character. |
| `--color auto` | Terminal-appropriate colour depth; redirected text is plain. |
| `--color always` | Emit ANSI text colour even when redirected. |
| `--color never` | Monochrome text. Native images retain their colours. |

For terminal labels and Markdown alt text, a form display name containing the species name (ignoring case) is used as stored, for example `Mega Dragonite`. Otherwise, the label combines the species name and the form display name in parentheses and removes the standalone word “form” (ignoring case), so `Sandshrew` with `Alola Form` becomes `Sandshrew (Alola)`. Markdown escapes punctuation in the label. Both export modes reject `--name` and `--no-newline`.

Transparent margins are trimmed before sizing. Text uses alpha-weighted brightness/colour over black; fully transparent cells leave blank terminal cells. Native PNG output and both export modes retain alpha. Downscaling and enlargement resample premultiplied RGBA; original width preserves source pixels.

Use `![Mew](mew.png)` to link a PNG from Markdown in the same directory, including in GitHub READMEs. A Markdown image with embedded PNG data works in viewers that allow data URLs. [Compatibility evidence](20261122-art-output-evaluation.md#embedded-markdown).

Native detection and encoding use ratatui-image Picker and its Kitty, Sixel and iTerm2 protocols. SSH does not disable images. For auto/img on matching terminal stdin/stdout, Picker sends its standard capability and cell-size queries; terminal settings are restored afterward, including query timeout. Explicit text modes and pipelines skip probing. Output remains inline without an alternate screen. If detection finds no native protocol, `auto` uses text and `img` without an explicit protocol reports an error.

With `--protocol`, Pokington uses the requested encoder and keeps the Picker's cell sizing and tmux handling. Native images still require stdin and stdout on the same terminal. The receiving terminal can ignore graphics sequences it does not support.

## Acceptance evidence

Automated PTY/protocol tests verify dimensions, source-pixel preservation, fallback, terminal-setting restoration and cleanup after failed writes. They do not establish actual visual/platform acceptance. Run the following in a real image-capable terminal, locally or over SSH to exercise the real command runner against the existing isolated complete store:

```sh
cargo run --locked --example art_acceptance -- /private/tmp/pokington-b02-live.mQwTZE/store /private/tmp/pokington-s04-macos-manual
```

The example displays fitted, numeric-width and original native images plus monochrome blocks/ASCII. It records output payloads, terminal identity/measurements, exit statuses and terminal-state equality. Visual confirmation remains a manual result: images stay inline, native sizes are sensible, and normal prompt/input operation resumes. Use an equivalent complete store path and a separate evidence directory on Linux. Keep missing environments recorded as unfinished acceptance.

For the prepared local Linux build, run this in the same real terminal. Source, binaries and the complete store are read-only; only the evidence directory is writable. Networking is disabled.

```sh
docker run --rm -it --network none --env TERM=xterm-256color --env LANG=C.UTF-8 --mount type=bind,source=/private/tmp/pokington-s04-linux-picker-work,target=/work,readonly --mount type=bind,source=/private/tmp/pokington-s04-linux-work/target,target=/work/target,readonly --mount type=bind,source=/private/tmp/pokington-b02-live.mQwTZE/store,target=/data,readonly --mount type=bind,source=/private/tmp/pokington-s04-linux-manual,target=/evidence --workdir /work rust:1-bookworm ./target/debug/examples/art_acceptance /data /evidence
```
