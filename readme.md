a quiet cli password manager with one encrypted slate, no daemon/tui (yet)

> disclosure: as i've become more busy & lost motivation with this project, i decided to hand it over to the clanker. don't expect quality non-ai content

## installation

because this isn't published on [crates.io](https://crates.io/), & probably never will be, feel free to compile it from source:

```sh
git clone https://github.com/vahrina/sheikah && cd sheikah
just install
```

if you'd like to give it a new name, symlink it afterwards:

```sh
ln -sf "${CARGO_HOME:-$HOME/.cargo}/bin/sheikah" "${CARGO_HOME:-$HOME/.cargo}/bin/<your desired alias>"
```

## dependencies

- [`fzf`](https://github.com/junegunn/fzf)
- a clipboard tool, [see below](#clipboard)

## usage

```sh
$ sheikah
a quiet cli password manager for keyboard warriors

Usage: sheikah [OPTIONS] [REF] [COMMAND]

Commands:
  show, -s    show an entry, or only the chosen fields
  copy, -c    copy an entry's password to the clipboard (cleared after 45s)
  list, -l    list all entries
  find, -f    fuzzy pick with fzf, or substring search for QUERY
  add, -a     add a new entry, interactively or via args
  edit, -e
  delete, -d
  config, -C  edit the config in $EDITOR
  init        create a new slate where your secrets live
  passwd      change the master password
  doctor      check tools and slate permissions
  version     print version
  help        Print this message or the help of the given subcommand(s)

Arguments:
  [REF]  entry to show: service name or search text

Options:
  -V, --version       print version
  -P, --plain         disable colors (respects NO_COLOR)
  -S, --slate <NAME>  slate name or path/to/slate [env: SHEIKAH_SLATE=]
```

note that many command includes subcommands, make sure to skim through each one you're interested in:

```sh
$ sheikah help passwd
change the master password

Usage: sheikah passwd [OPTIONS]

Options:
  -P          disable colors (also respects NO_COLOR)
  -S <NAME>   slate name inside ~/.sheikah/ or a path (default: the only slate, `eye` for init) [env: SHEIKAH_SLATE=]
  -h, --help  Print help
```

### example usages

```sh
$ sheikah add [args]           # add an entry interactively or with arguments
$ sheikah github               # show an entry
$ sheikah -s github -p         # print only the password (for pipes)
$ sheikah -c github            # copy its password (cleared after 45s)
$ sheikah -l                   # list entries (no secrets)
$ sheikah -lb                  # ... inside a box
$ sheikah find [query]         # pick an entry interactively or specify the query directly
$ sheikah doctor               # health check
$ sheikah -C                   # adjust your preferred settings
```

every action works as a word or a single-dash flag

`REF` resolves as: exact service name, else a unique substring match, else fuzzy finder (seeded with `REF`, so typos like `nflx` still find `netflix`)

`-S` (or `$SHEIKAH_SLATE`) is a slate name under `~/.sheikah/` or a path, e.g. `sheikah -S work init`. omit the slate to resolve to the default path & name `~/.sheikah/eye`

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
- `init` optionally runs `sudo chattr +a` on the dir ([ref](https://unix.stackexchange.com/questions/269020/what-is-the-effect-of-chattr-a-on-a-directory)), so files in it cannot be deleted or renamed. saves therefore overwrite in place, after copying the previous version to `<slate>.bak`

## crypto

```
[salt (16B)][nonce (12B)][ciphertext + gcm tag (16B)]
```

- key: argon2id(master password, salt) -> 256 bit, `m = 64 MiB, t = 3, p = 1`
- cipher: aes-256-gcm over the json-serialized entries
- every save draws a fresh random salt & nonce
- the kdf params are not stored in the file, changing them breaks it (duh..?)
