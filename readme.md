a quiet cli password manager with one encrypted slate, no daemon/tui (yet)

> disclosure: as i've become more busy & lost motivation with this project, i decided to hand it over to the clanker - don't expect quality non-ai content

## installation

### installer

downloads the right release archive for your machine into `~/.local/bin` - rerun to update

```sh
curl -fsSL https://raw.githubusercontent.com/vahrina/sheikah/main/install.sh -o install.sh \
  && less $_
bash install.sh
```

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
just install # or: cargo install --path . --locked
```

if you'd like to give it a new name, symlink it afterwards

```sh
ln -sf "${CARGO_HOME:-$HOME/.cargo}/bin/sheikah" \
  "${CARGO_HOME:-$HOME/.cargo}/bin/<your desired alias>"
```

to ship a new version yourself: bump `Cargo.toml`, commit, then `just tag`

## dependencies

- [`fzf`](https://github.com/junegunn/fzf) (required)
- a clipboard tool, (optional, see [clipboard](#clipboard))
- [`just`](https://github.com/casey/just#installation) (optional, from-source only)

## get started

### initializing a new slate

a slate is the zelda-derived equivalent of a file where your secrets will be stored in - likewise the name "sheikah"

```sh
$ sheikah init
```

> optionally, you can specify a new name with the `-S` flag

### basic operations

adding, editing, deleting & viewing follow a similar pattern

```console
$ sheikah add # or via arguments, see `sheikah help add`
$ sheikah edit cloudflare
$ sheikah show gentoo
# yadeyadeyade
```

see [usage](#usage) for more in-depth examples

### passwords, security & misc

passwords will **always** have an interactive prompt to ensure security via [`rpassword`](https://crates.io/crates/rpassword) - read more about the security aspect under [crypto](#crypto)

also make sure all [requirements](#dependencies) are satisfied after initializing a new slate:

```md
sheikah doctor
```

## usage

```console
$ sheikah
a quiet cli password manager for keyboard warriors

Usage: sheikah [OPTIONS] [REF] [COMMAND]

Commands:
  init         create a new slate where your secrets live
  doctor       check tools, permissions & health
  add, -a      add an entry interactively or via args
  edit, -e     edit an entry
  delete, -d   delete an entry
  show, -s     show an entry, or only the chosen fields
  copy, -c     copy an entry's password to the clipboard
  list, -l     list all entries
  find, -f     fuzzy pick an entry, optionally seeded with QUERY
  config, -C   open the config with your preferred $EDITOR
  passwd       change the master password
  backup, -b   pick one of the last saves & restore it
  completions  print a shell completion script to stdout
  help         print this message or the help of the given subcommand(s)

Arguments:
  [REF]  entry to show: service name or search text

Options:
  -V, --version
  -P, --plain         disable colors (respects NO_COLOR)
  -S, --slate <NAME>  slate name (or unique prefix) or path/to/slate [env: SHEIKAH_SLATE=]
```

`REF` resolves as: exact service name, else a unique substring match, else fuzzy finder (typos like `nflx` still find `netflix`)

note that many commands include subcommands, make sure to skim through each one you're interested in:

```console
$ sheikah help add
add an entry interactively or via args

Usage: sheikah {add|-a} [OPTIONS]

Options:
  -s <STR>            label for site/app (netflix, proton, ..)
  -c <STR>            category (series, mail, ..)
  -P, --plain         disable colors (respects NO_COLOR)
  -d <STR>            domain/url
  -S, --slate <NAME>  slate name (or unique prefix) or path/to/slate [env: SHEIKAH_SLATE=]
  -e <STR>            email
  -u <STR>            username/login
  -n <STR>            optional notes
  -r, --recovery      store recovery codes instead of a password
```

### example usages

every action works as a word or a single-dash flag

```console
$ sheikah add -s 'csfloat' -c 'trading'
  # adds a new service with a categroy
$ sheikah github
  # will look for an entry 'github', picker will pop up if there are multiple
$ sheikah -s github -p | ssh raspberrypi
  # print only the password of 'github', useful for pipes
$ sheikah -c github
  # copy the password into your clipboard, clears after a period of time
$ sheikah -lb
  # list all entries inside a box
```

`list` & the fuzzy finder sort by `sort` under `[table]` in `sheikah -C`: `alphabetical` (default), `modified` or `used` (both newest first). `used` records every show/copy/find, which re-saves the slate; the previous file goes to `<slate>.used.bak` instead of the rotating backups

### multiple slates

`-S` is a slate name under `~/.sheikah/` or a path, e.g. `sheikah -S work init`. `init` without `-S` creates `~/.sheikah/eye`

- with only one slate, `-S` is never needed
- a unique prefix is enough: `-S w` picks `work`
- with several slates & no `-S`, fzf asks which one
- `export SHEIKAH_SLATE=work` in your shell rc makes `work` the default, `-S` still overrides it

## shell completion

> [zsh](https://www.zsh.org/) only for now, will commence with [bash](https://www.gnu.org/software/bash/)/[fish](https://fishshell.com/) once this project gains momentum

write the script into a completions dir:

```sh
mkdir -p ~/.config/zsh/completions
sheikah completions zsh > ~/.config/zsh/completions/_sheikah
```

zsh only picks it up if that dir is in `fpath` before `compinit` runs, so add this to your `~/.zshrc`

```sh
fpath=(~/.config/zsh/completions $fpath)
autoload -Uz compinit && compinit
```

> if your `~/.zshrc` already runs `compinit` (oh-my-zsh, zinit, ..), only add the `fpath=` line above

finally, reload your shell:

```sh
exec zsh
```

> note that a mere `source ~/.zshrc` is not sufficient enough

rerun `sheikah completions zsh > ...` **after** updating sheikah & under an alias symlink, also add `compdef _sheikah <alias>` to `~/.zshrc`

### troubleshooting [shell completion](#shell-completion)

**q**: my shell doesn't display completions after following the [steps above](#shell-completion)

**a**: check these in order:
1. `which sheikah` points at the binary you expect, not an old copy under `~/.local/bin`
2. `~/.config/zsh/completions/_sheikah` exists & starts with `#compdef sheikah`
3. `echo $fpath` includes `~/.config/zsh/completions` near the beginning. when using [omz](https://ohmyz.sh/)/[zinit](https://github.com/zdharma-continuum/zinit), put that `fpath=` line above the `compinit` call
4. `autoload -Uz sheikah && which -a _sheikah` finds the function; if not, `exec zsh` (or `rm -f ~/.zcompdump* && exec zsh`)

**q**: i updated but completions are outdated

**a**: regenerating with an old binary rewrites the old script - install/update first, then proceed:

```sh
# 1. regenerate completions
sheikah completions zsh > ~/.config/zsh/completions/_sheikah

# 2. make sure the script is current (must print 0)
grep -c _default ~/.config/zsh/completions/_sheikah

# 3. wipe the dump cache & force a full reload
rm -f ~/.zcompdump* && exec zsh
```

a `0` means the current script; anything else means you still ran an old binary. if the count is already `0` & artifacts persist, e.g. files show alongside commands, something in your zsh config forces file completion (`zstyle -L ':completion:*'`), not an issue sheikah produces

if none of the above worked out for you, feel free to [open an issue](https://github.com/vahrina/sheikah/issues)! :)

## themes

colors come from [base16](https://tinted-theming.github.io/tinted-gallery/) schemes, set under `[theme]` in `sheikah -C`

built in are:
- `0x96f`,
- `ascendancy`
- `black-metal`
- `custom` keeps the plain bold/dim look

any other scheme can be imported from the [tinted gallery](https://tinted-theming.github.io/tinted-gallery/), find the yaml files here: [tinted-theming/schemes](https://github.com/tinted-theming/schemes/tree/spec-0.11/base16)) & saved as `~/.sheikah/themes/<name>.yaml`. don't forget to also set `name = "<name>"`! a file with a built-in name replaces it & `base16-` prefixes from the gallery are ignored

### customizing the appearance

every other key overrides one role & leaves the rest of the theme alone, e.g. setting your theme to `nord` with your own border color:

```toml
[theme]
name = "nord"
border = "red"
```

### accepted roles & values

**roles**:
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

**values**:
- **name convetion**: black red green yellow blue magenta cyan white (plus `bright-*`)
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

change the delay with `clear` under `[clipboard]` in `sheikah -C`, whereas `0` disables this behaviour

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
