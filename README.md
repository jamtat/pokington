# Pokington

Pick Pokémon names and bring their illustrations to your terminal.

Pokington chooses random names, avoids names you have already used, and displays original or alternate-form artwork inline. Download the catalogue and artwork once, then use `choose` and `art` offline on macOS and Linux.

## Table of contents

- [Features](#features)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Choose names](#choose-names)
- [Display artwork](#display-artwork)
  - [Pokémon and forms](#pokémon-and-forms)
  - [Display modes](#display-modes)
  - [Size and colour](#size-and-colour)
  - [Names and terminal titles](#names-and-terminal-titles)
- [Download and refresh data](#download-and-refresh-data)
- [Local data](#local-data)
- [Help and scripting](#help-and-scripting)
- [Development](#development)

## Features

- Choose distinct Pokémon names uniformly at random.
- Exclude specific names or names found in your existing files.
- Display illustrations by name or National Pokédex number, or let Pokington pick one.
- Select illustrated Mega Evolutions, regional forms, Gigantamax forms, and other alternate forms.
- Automatically use native terminal images, Unicode blocks, or ASCII art.
- Control artwork size, text colour, name labels, and the terminal title.
- Work offline after the initial download, with explicit updates whenever you want them.

## Installation

With [Rust and Cargo](https://www.rust-lang.org/tools/install) installed, run this from the repository directory:

```sh
cargo install --locked --path .
```

Make sure Cargo's binary directory, usually `~/.cargo/bin`, is on your `PATH`.

## Quick start

```sh
# Pick three distinct names.
pokington choose --count 3

# Display Pikachu.
pokington art Pikachu

# Display a random illustration and its name.
pokington art --name
```

The first `choose` or `art` command downloads the full catalogue and all supported artwork before producing its result. This requires an internet connection. Later runs use the local data without checking for updates.

To download everything ahead of time:

```sh
pokington bootstrap
```

## Choose names

```text
pokington choose --count N [--exclude NAME]... [FILES]...
```

Names are printed one per line, without duplicates. Selection uses original species names; alternate forms do not add extra entries or change a species' chance of being chosen.

```sh
# Short options work too.
pokington choose -c 5

# Exclude exact names, ignoring case.
pokington choose -c 3 -e Pikachu -e Eevee

# Avoid names already used in one or more files.
pokington choose -c 3 tasks.md archive.md

# Combine both kinds of exclusion.
pokington choose -c 3 -e Ditto tasks.md
```

File exclusions match case-insensitive substrings anywhere in UTF-8 text. A file containing `MEWTWO` excludes both **Mew** and **Mewtwo**. An unknown name passed to `--exclude` has no effect.

If a file is unreadable or is not valid UTF-8, or too few eligible names remain, the command fails without printing any names. `--count 0` succeeds without printing names, provided the supplied files and local data are valid.

## Display artwork

```text
pokington art [NAME_OR_ID] [--form FORM] [--mode auto|img|blocks|ascii]
              [--width N|original] [--color auto|always|never]
              [--exclude NAME]... [--exclude-file FILE]...
              [--name] [--no-newline] [--title]
```

Artwork appears inline and remains visible after the command exits.

### Pokémon and forms

```sh
pokington art Eevee
pokington art 133
pokington art "Mr. Mime"
pokington art Charizard --form "mega x"
pokington art Raichu --form alolan

# Pick any illustration, including alternate forms.
pokington art

# Pick a random illustration matching a form name.
pokington art --form gigantamax

# Exclude entire species, including all their forms.
pokington art -e Pikachu --exclude Eevee

# Avoid species named in files, combined with exact exclusions.
pokington art --exclude-file tasks.md --exclude-file archive.md -e Ditto
```

Pokémon and form names are case-insensitive. A named Pokémon defaults to its original form. Form names are descriptive, such as `mega x` or `alolan`; an invalid form reports the available form names for that Pokémon.

Random artwork gives every species/form illustration an equal chance, so species with more illustrated forms appear more often overall. With `--form` and no Pokémon, selection is limited to entries matching that form.

`-e` / `--exclude` and `--exclude-file` use the same matching rules as `choose`: exact names ignore case, and file exclusions match species names as case-insensitive substrings anywhere in UTF-8 text. Unknown exclusion names have no effect. Excluding a species removes all its forms; form names alone do not exclude artwork. Unreadable or invalid UTF-8 files, an explicitly requested species that is excluded, or no remaining matching artwork cause an error before any artwork is printed.

The catalogue uses illustration PNGs from [Serebii](https://www.serebii.net/pokemon/nationalpokedex.shtml). Alternate forms are included when a matching illustration and supported type data are available.

### Display modes

| Mode | Output |
| --- | --- |
| `auto` | Detects native image support, then falls back to Unicode blocks or ASCII. The default. |
| `img` | Uses a native terminal image protocol. Reports an error if unsupported. |
| `blocks` | Renders with Unicode half-block characters. |
| `ascii` | Renders with ASCII characters. |

```sh
pokington art Bulbasaur --mode blocks
pokington art Gengar --mode ascii
```

When piped or redirected, `auto` produces plain, uncoloured ASCII by default:

```sh
pokington art Snorlax > snorlax.txt
```

### Size and colour

Transparent margins are trimmed before display. By default, artwork fits the available terminal width and height; redirected ASCII defaults to 80 columns. Set a positive width in character columns to choose your own size:

```sh
pokington art Dragonite --width 60
pokington art Mew --mode ascii --width 40 --color never
pokington art Lapras --mode blocks --color always > lapras.ansi
```

`--color` controls block and ASCII output: `auto` follows terminal colour support, `always` includes ANSI colour even when redirected, and `never` produces monochrome text. Native images retain their PNG colours.

Use `--width original` to skip resizing after trimming. Native images use one source pixel per screen pixel; blocks pack two source pixel rows into each character row; ASCII uses one source pixel per character. Original-size output can exceed the terminal dimensions.

### Names and terminal titles

```sh
# Print the selected Pokémon's display name below its artwork.
pokington art --name

# Set the terminal title to the selected display name too.
pokington art Charizard --form "mega y" --name --title

# Leave the cursor immediately after the printed name.
pokington art Eevee --name --no-newline
```

`--title` also works without `--name`. It writes a terminal title escape sequence, so use it for terminal output. `--no-newline` requires `--name` and only removes the newline after the label.

## Download and refresh data

```sh
# Refresh the catalogue and download new or missing artwork.
pokington bootstrap

# Refresh the catalogue and redownload all artwork.
pokington bootstrap --force

# Choose the number of concurrent download workers; the default is 4.
POKINGTON_CONCURRENCY=8 pokington bootstrap
```

`POKINGTON_CONCURRENCY` must be a positive integer. It also applies to automatic first-run downloads.

Download logs and the completion summary go to stderr. Interrupted downloads retain completed artwork so another `bootstrap` can reuse it. During an update, the previous complete dataset remains usable until the replacement is ready, including with `--force`.

If a command reports missing or corrupt local data, run `pokington bootstrap` to repair it. Use `--force` when you want to replace all downloaded artwork.

## Local data

Pokington stores its catalogue and PNG artwork in your user data directory:

| Platform | Location |
| --- | --- |
| macOS | `~/Library/Application Support/pokington` |
| Linux | `$XDG_DATA_HOME/pokington` when `XDG_DATA_HOME` is an absolute path; otherwise `~/.local/share/pokington` |

Ordinary `choose` and `art` commands use this local dataset. Run `bootstrap` whenever you want to discover newly available Pokémon or forms.

## Help and scripting

```sh
pokington --help
pokington choose --help
pokington art --help
pokington bootstrap --help
pokington --version
```

Help and version commands work without downloading data. Requested names and artwork go to stdout; errors and download progress go to stderr.

| Exit code | Meaning |
| --- | --- |
| `0` | Success, including help and version output. |
| `1` | An operation failed, such as a download, file read, lookup, or name selection. |
| `2` | Invalid command-line arguments. |

## Development

Install the pinned nextest version with Cargo from the repository directory:

```sh
cargo install-nextest
```

This installs `cargo-nextest` 0.9.151 into Cargo's binary directory, usually `~/.cargo/bin`. The first installation compiles nextest and can take a few minutes.

Run the same test command used by GitHub CI and the prek test hook:

```sh
cargo test-ci
```

The alias runs `cargo nextest run --profile ci --all-targets --all-features --locked`. It completes the full test suite after failures and writes the JUnit report to `target/nextest/ci/junit.xml`.

CI installs the same nextest version from a prebuilt binary to keep setup fast. Keep the version in `.cargo/config.toml` and `.github/workflows/ci.yml` aligned when upgrading nextest.
