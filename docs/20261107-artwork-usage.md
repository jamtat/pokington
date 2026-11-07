# Artwork commands

Run `pokington art` to sample uniformly from installed species/form artwork. The first data command downloads the complete catalogue and artwork; subsequent commands use local data offline. Use `pokington bootstrap` to refresh or repair data, or `pokington bootstrap --force` to redownload it.

```sh
pokington art Charizard
pokington art 6 --form 'mega x'
pokington art Meowth --form galarian --mode blocks
pokington art --form gigantamax
pokington art Charizard --mode ascii --width 64 --color never
pokington art Charizard --mode img --width original
```

Names and descriptive form keys ignore case. Named species default to the original form. An invalid form lists that species's available keys; a random form filter with no matches reports an error.

| Option | Behavior |
| --- | --- |
| `--mode auto` | Native graphics when verified; otherwise Unicode blocks in a suitable terminal, then ASCII. Redirected output uses ASCII. |
| `--mode img` | Native images only; unsupported contexts report an error suggesting text modes. |
| `--mode blocks` | Two sample rows per character using Unicode half-blocks; monochrome output retains upper/lower positions. |
| `--mode ascii` | Brightness mapped to ASCII characters. |
| Omitted width | Fit terminal width/height with aspect correction; redirected text defaults to 80 columns. |
| `--width N` | Positive character-column width; may exceed the terminal. |
| `--width original` | Crop only. Native uses source pixels; blocks pack two pixel rows per character row; ASCII uses one source pixel per character. Output may overflow. |
| `--color auto` | Terminal-appropriate colour depth; redirected text is plain. |
| `--color always` | Emit ANSI text colour even when redirected. |
| `--color never` | Monochrome text. Native images retain their colours. |

Transparent margins are trimmed before sizing. Text uses alpha-weighted brightness/colour over black; fully transparent cells leave blank terminal cells. Native PNG output retains alpha. Downscaling and enlargement resample premultiplied RGBA; original width preserves source pixels.

Native detection and encoding use ratatui-image Picker and its Kitty, Sixel and iTerm2 protocols. SSH does not disable images. For auto/img on matching terminal stdin/stdout, Picker sends its standard capability and cell-size queries; terminal settings are restored afterward, including query timeout. Explicit text modes and pipelines skip probing. Output remains inline without an alternate screen. If the library cannot establish an image protocol, auto uses text and explicit img reports an error.

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
