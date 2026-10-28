a quiet cli password manager with one encrypted slate, no daemon/tui (yet)

> disclosure: as i've become more busy & lost motivation with this project, i decided to hand it over to the clanker - don't expect quality non-ai content

## installation

### installer

downloads the right release archive for your machine into `~/.local/bin`

```sh
curl -fsSL https://raw.githubusercontent.com/vahrina/sheikah/main/install.sh -o install.sh \
  && less $_
bash install.sh
```

> run again to update

### manual

grab `sheikah-vX.Y.Z-<arch>.tar.gz` from [releases](https://github.com/vahrina/sheikah/releases), then:

```sh
tar -xzf sheikah-v0.2.0-x86_64.tar.gz
install -m 755 sheikah ~/.local/bin/sheikah
```

### source

needs a rust toolchain & preferably [`just`](https://github.com/casey/just#installation)

```sh
git clone https://github.com/vahrina/sheikah && cd sheikah
just install   # or: cargo install --path .
```

if you'd like to give it a new name, symlink it afterwards

```sh
ln -sf "${CARGO_HOME:-$HOME/.cargo}/bin/sheikah" "${CARGO_HOME:-$HOME/.cargo}/bin/<your desired alias>"
```

to ship a new version yourself: bump `Cargo.toml`, commit, then `just tag` (pushes `vX.Y.Z` & triggers the release workflow)

## dependencies

- [`fzf`](https://github.com/junegunn/fzf) (required)
- a clipboard tool, [see below](#clipboard) (optional)
- [`just`](https://github.com/casey/just#installation) (optional, from-source only)

## get started

### initializing a new slate

a slate is the zelda-derived equivalent of a file where your secrets will be stored in - likewise the name "sheikah"

```sh
sheikah init
```

> optionally, you can specify a new name with the `-S` flag

### basic operations

adding, editing, deleting & viewing follow a similar pattern

```sh
sheikah add # or via arguments, see `sheikah help add`
sheikah edit
# yadeyadeyade
```

see [usage](#usage) for more in-depth examples

### passwords, security & misc

passwords will **always** have an interactive prompt to ensure security via [`rpassword`](https://crates.io/crates/rpassword). read more about the security aspect under [crypto](#crypto)

also make sure all [requirements](#dependencies) are satisfied after initializing a new slate

```sh
sheikah doctor
```

## usage

```sh
$ sheikah
a quiet cli password manager for keyboard warriors

Usage: sheikah [OPTIONS] [REF] [COMMAND]

Commands:
  init        create a new slate where your secrets live
  doctor      check tools & permissions
  add, -a     basic operations
  edit, -e
  delete, -d
  show, -s    show an entry, or only the chosen fields
  copy, -c    copy an entry's password to the clipboard (cleared after 45s)
  list, -l
  find, -f    fuzzy pick an entry, optionally seeded with QUERY
  config, -C
  passwd      change the master password
  backup, -b  pick one of the last saves and restore it
  help        print this message or the help of the given subcommand(s)

Arguments:
  [REF]  entry to show: service name or search text

Options:
  -V, --version
  -P, --plain         disable colors (respects NO_COLOR)
  -S, --slate <NAME>  slate name (or unique prefix) or path/to/slate [env: SHEIKAH_SLATE=]
```

note that many command includes subcommands, make sure to skim through each one you're interested in (`help show` & `help -s` both work):

```sh
$ sheikah help passwd
change the master password

Usage: sheikah passwd [OPTIONS]

Options:
  -P, --plain         disable colors (respects NO_COLOR)
  -S, --slate <NAME>  slate name (or unique prefix) or path/to/slate [env: SHEIKAH_SLATE=]
```

### example usages

```sh
$ sheikah add [args]           # add an entry interactively or with arguments
$ sheikah add -r               # store recovery codes instead of a password
$ sheikah github               # show an entry
$ sheikah -s github -p         # print only the password (for pipes)
$ sheikah -c github            # copy its password (cleared after 45s)
$ sheikah -l                   # list entries (no secrets)
$ sheikah -lb                  # ... inside a box
$ sheikah find [query]         # fuzzy pick an entry, optionally seeded with a query
$ sheikah -b                   # restore one of the last 5 saves
$ sheikah doctor               # health check
$ sheikah -C                   # adjust your preferred settings
```

every action works as a word or a single-dash flag

`REF` resolves as: exact service name, else a unique substring match, else fuzzy finder (seeded with `REF`, so typos like `nflx` still find `netflix`)

### multiple slates

`-S` is a slate name under `~/.sheikah/` or a path, e.g. `sheikah -S work init`. `init` without `-S` creates `~/.sheikah/eye`

- with only one slate, `-S` is never needed
- a unique prefix is enough: `-S w` picks `work`
- with several slates & no `-S`, fzf asks which one
- `export SHEIKAH_SLATE=work` in your shell rc makes `work` the default, `-S` still overrides it

## themes

colors come from [base16](https://tinted-theming.github.io/tinted-gallery/) schemes, set under `[theme]` in `sheikah -C`

built in are `0x96f` (default), `ascendancy` & `black-metal`; `custom` keeps the plain bold/dim look. any other scheme works offline by saving any theme from [tinted-theming/schemes](https://github.com/tinted-theming/schemes/tree/spec-0.11/base16)/[tinted gallery](https://tinted-theming.github.io/tinted-gallery/)) as `~/.sheikah/themes/<name>.yaml` & set `name = "<name>"`. a file with a built-in name replaces it & `base16-` prefixes from the gallery are ignored

every other key overrides one role & leaves the rest of the theme alone:

```toml
[theme]
name = "nord"
border = "red"            # names, bright-red, ...
header = "#88c0d0"        # #rgb, #rrggbb
secret = "hsl(0, 70%, 60%)"
muted = "rgb(76, 86, 106)"
key = "base0E"            # a slot of the current theme
```

following roles exist:
- `header`
- `border`
- `text`
- `muted`
- `key`
- `secret`
- `accent`
- `error`
- `selection` (fzf's current line)

> truecolor is used when `COLORTERM` is `truecolor`/`24bit`, otherwise the nearest of 256 colors

corresponding to the roles, following values are accepted:
- **naming convention**: black red green yellow blue magenta cyan white, plus `bright-*`
- **base convention**: `baseXX` (see [here](https://github.com/chriskempson/base16/blob/main/styling.md))
- **rgb**: `#rrggbb`
- **default**: terminal's own foreground

## clipboard

`-c` uses the first tool that works, & a detached `sh` empties the clipboard 45s later

| order | tool | when |
|---|---|---|
| 1 | `wl-copy` | wayland |
| 2 | `xclip -selection clipboard` | x11 |
| 3 | `xsel --clipboard` | x11 |
| 4 | `clip.exe` | wsl |
| 5 | `pbcopy` | macos |

## storage

- `~/.sheikah/` is created with minimal owner permissions (`0700`), slate(s) with stricter permissions (`0600`)
- `init` optionally runs `sudo chattr +a` on the dir ([ref](https://unix.stackexchange.com/questions/269020/what-is-the-effect-of-chattr-a-on-a-directory)), so files in it cannot be deleted or renamed. saves therefore overwrite in place, after copying the previous version to a backup slot
- backups rotate through `<slate>.1.bak` .. `<slate>.5.bak`, the oldest slot gets overwritten (never deleted). `sheikah backup` restores one; a backup keeps the master password it was saved with

## crypto

```
[salt (16B)][nonce (12B)][ciphertext + gcm tag (16B)]
```

- key: argon2id(master password, salt) -> 256 bit, `m = 64 MiB, t = 3, p = 1`
- cipher: aes-256-gcm over the json-serialized entries
- every save draws a fresh random salt & nonce
- the kdf params are not stored in the file, changing them breaks it (duh..?)
