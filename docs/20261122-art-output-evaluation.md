# Artwork export evaluation

Date 2026-11-22. First release design accepted. [Implementation plan](20261122-art-output-plan.md).

## Current behavior

`pokington art` selects one locally installed PNG by species and form, with optional exclusions. `auto`, `img`, `blocks`, and `ascii` display artwork. Redirected `auto` emits ASCII. The collection contains Serebii illustrations, including alternate forms with matching art. Game sprites would require a separate source and catalogue decision. [Current brief](brief.md#art) · [Artwork selection and resizing](../src/art.rs) · [Command output](../src/command.rs).

## Embedded Markdown

A paste-ready image uses `![Mew](data:image/png;base64,…)`. The URL contains the PNG media type and standard padded Base64. The payload stays on one line, and the selected form's display name supplies escaped alt text. Raw Base64 is useful when a script supplies the media type and surrounding markup. [CommonMark images](https://spec.commonmark.org/0.31.2/#images) · [Data URLs](https://www.rfc-editor.org/rfc/rfc2397) · [Base64](https://www.rfc-editor.org/rfc/rfc4648.html#section-4).

| Destination | Evidence |
| --- | --- |
| Pandoc 3.14 HTML output | Local checks preserved the data URL byte for byte in image tags for `commonmark` and `gfm` input, at full and reduced widths. |
| VS Code Markdown preview | The source accepts `data:image/` links and the strict preview policy permits `data:` for images. This was verified in source. [Link validation](https://github.com/microsoft/vscode/blob/main/extensions/markdown-language-features/src/markdownEngine.ts#L271-L278) · [Preview policy](https://github.com/microsoft/vscode/blob/main/extensions/markdown-language-features/src/preview/documentRenderer.ts#L233-L251). |
| GitHub.com | GitHub applies sanitization after GFM parsing. A public discussion shows a pasted data image being stripped, and `remark-embed-images` documents the same limitation for Markdown image syntax. For repository Markdown, use a PNG at a relative path. [GFM spec](https://github.github.com/gfm/#what-is-github-flavored-markdown-) · [Public example](https://github.com/orgs/rehypejs/discussions/59) · [Remark documentation](https://github.com/remarkjs/remark-embed-images/blob/main/readme.md#when-should-i-use-this) · [GitHub image syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#images). |

Base64 adds about one third to the PNG's size. Across 1,427 installed PNGs, the median source file was 127,391 bytes, equivalent to 169,856 Base64 characters. The full-size examples below use the installed PNG bytes. The resized measurements use ImageMagick 7.1.2 to trim, resize, and encode RGBA PNGs. They estimate the size of the proposed output before testing Pokington's encoder.

| Illustration | Installed PNG | Full-size data URL | Data URL at 64 px wide | Data URL at 128 px wide |
| --- | ---: | ---: | ---: | ---: |
| Bulbasaur | 203,385 B | 271,202 B | 12,910 B | 34,946 B |
| Charizard | 144,568 B | 192,782 B | 12,130 B | 31,354 B |
| Mew | 133,175 B | 177,590 B | 11,394 B | 27,926 B |

## Design update, 2026-11-22

Markdown is the selected format for embedded images. The proposal to give Markdown a 64 px default and PNG a cropped source-size default was rejected. Switching between export formats must introduce no implicit size change.

The first release includes `--mode png` and `--mode markdown`. `--mode png` writes binary PNG bytes to stdout, so shell redirection produces an ordinary PNG file. `--mode markdown` uses Base64 to place the prepared PNG in a Markdown image reference. The user answered `y` when asked whether raw PNG export should ship alongside Markdown.

Both exports use the cropped source dimensions when width is omitted. The user selected this behavior in response to the proposed choice between a required width and a shared original-size default. This matches `--width original` in the terminal modes, which trims transparent margins and preserves the remaining pixel dimensions. Omitted width in the current terminal modes still fits the terminal or uses 80 columns for redirected ASCII.

The user also requested removal of the terminal `--title` flag because it did not work for them. Commit `1038cdb` removed the flag separately.

### Pixel-width syntax

| Option | Syntax | Trade-off |
| --- | --- | --- |
| A | `--width 64px` for either export. | The unit is explicit. Terminal widths continue to use bare numbers for columns. |
| B | `--width 64` for either export. | Reuses the existing parser. Numeric widths are pixels for exports and columns for terminal output. |

The initial recommendation was A. The user selected B and said “B pixels are the implicit unit for images i do not want to introduce parsing complexity”. Use `--width N` with the existing positive-integer parser. For PNG and Markdown, an explicit number is the image width in pixels and resizing preserves aspect ratio. `--width original` has the same effect as omitting the option for exports.

## Initial first release proposal, 2026-11-22

| Option | Output | Trade-off |
| --- | --- | --- |
| A | `--mode png` and `--mode markdown` | Supports a portable image file for GitHub and a self-contained snippet for compatible previews. Both use the same PNG preparation. |
| B | `--mode png` | Covers relative links, uploads, and shell pipelines. Users can produce a self-contained HTML document with Pandoc's `--embed-resources`. [Pandoc manual](https://pandoc.org/MANUAL.html#option--embed-resources). |
| C | `--mode markdown` | Gives the requested paste-ready snippet with one command. Users who need a separate PNG still need to extract it or locate the cached file. |

Recommendation is A. `--mode png` supplies the format most tools already consume. `--mode markdown` adds the data URL and alt text needed to paste directly into a document. Keep raw `--mode base64` and standalone `--mode data-url` for a demonstrated scripting need.

Proposed sizing uses pixels for these two explicit modes. Markdown defaults to 64 px wide, PNG defaults to the cropped source dimensions, and both accept `--width N` or `--width original`. Preserve aspect ratio and transparency. The command can reuse local selection, PNG decoding, visible-pixel cropping, and alpha-aware resizing before encoding. PNG stdout contains the image bytes. Markdown stdout contains one image reference and a trailing newline. `--name` and `--title` conflict with both modes because they append bytes to stdout. `--protocol` already requires `img`, and `--color` only affects terminal text modes. [Command output](../src/command.rs) · [Visible-pixel crop](../src/inline_image.rs) · [Current dependencies](../Cargo.toml).

One implementation stage could cover the shared PNG preparation, the selected CLI modes, and documentation. Acceptance should verify a PNG decode and alpha preservation, Markdown alt text and Base64 round-trip, pixel sizing independent of terminal geometry, and no stdout on selection, decode, or incompatible-argument errors. The first release scope is open.

## Other output modes

| Candidate | Assessment |
| --- | --- |
| Unicode Braille | A 2×4 dot grid gives narrow terminal output and fenced code blocks finer silhouette detail than half-blocks. In a local 16×8 character comparison, Braille retained more of Pikachu's and Mew's tails, while dark parts of Eevee still fragmented. Check typical fonts before adopting it. [Unicode Braille patterns](https://www.unicode.org/charts/PDF/U2800.pdf) · [Chafa symbol sets](https://github.com/hpjansson/chafa/blob/master/docs/chafa.xml) · [CommonMark fenced code blocks](https://spec.commonmark.org/0.31.2/#fenced-code-blocks). |
| Lossless WebP | Potential option for large batches of transparent exports. In an 11-image, 64 px ImageMagick sample, lossless WebP was about 40% smaller in aggregate than stripped RGBA PNG. The Rust encoder was outside that measurement. [WebP compression](https://developers.google.com/speed/webp/docs/compression). |
| Clipboard | Shell pipes can send PNG or Markdown to an existing clipboard tool. Native clipboard support would introduce platform-specific code. |
| Real sprites or animation | Requires a sprite source, coverage and form-matching work, and a decision on how the existing static illustration collection should coexist with it. [Current brief](brief.md#art) · [PNG validation](../src/store.rs). |
