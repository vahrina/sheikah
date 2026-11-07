<p align="center">
  a quiet cli password manager with one encrypted file, keeping your credentials safe & locally available - portable & accessible on the fly!
</p>

<p align="center">
  <a href="https://github.com/vahrina/sheikah/actions/workflows/ci.yml"><img src="https://github.com/vahrina/sheikah/actions/workflows/ci.yml/badge.svg" alt="ci"></a>
  <a href="https://github.com/vahrina/sheikah/releases/latest"><img src="https://img.shields.io/github/v/release/vahrina/sheikah" alt="release"></a>
  <img src="https://img.shields.io/badge/license-mit-blue" alt="mit">
</p>

> disclosure: as i've become more busy & lost motivation with this project, i decided to hand it over to the clanker - don't expect quality non-ai content since [`3425c13`](https://github.com/vahrina/sheikah/commit/3425c1352afd90fcb69f5ef7d000eaab1c5e1835)

## about - for the curious

what's "_sheikah_"? what's a "_slate_"? what is _this_ & _why_?

- this analogy comes from the [sheikah race](https://zelda.fandom.com/wiki/sheikah) in the [zelda franchise](https://zelda.fandom.com/wiki/The_Legend_of_Zelda_(series)), not specifically tied to a single game in the series. they are a mysterious, hidden clan that protect the [royal family](https://zelda.fandom.com/wiki/Royal_Family_of_Hyrule) - likewise protecting your credentials in secrecy
- a slate refers to the [sheikah slate](https://zelda.fandom.com/wiki/Sheikah_Slate), made for the [chosen one](https://zelda.fandom.com/wiki/Link), which contains all necessary data to complete the game start to finish - likewise a slate containing all the data you provide
- the reason i came up with this was for one, not liking the gui feel that every other password manager i've tried so far provides. the other reason being to not leave the shell anymore, as my previous approach was quite tedious with a usb

## installation

### installer

downloads the right release archive for your machine into `~/.local/bin` - rerun to update

```sh
# inspect the installer first
curl -fsSL https://raw.githubusercontent.com/vahrina/sheikah/main/install.sh -o install.sh \
  && less $_

# if everything looks fine to you, proceed
bash install.sh
```

> don't forget to press <kbd>q</kbd> to exit the inspection

### manual

grab `sheikah-vX.Y.Z-<arch>.tar.gz` from [releases](https://github.com/vahrina/sheikah/releases), then:

```sh
tar -xzf sheikah*.tar.gz
install -m 755 sheikah ~/.local/bin/
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
- [`OpenSSH 8.4+`](https://www.openssh.org/txt/release-8.4) (optional)
- a [clipboard](#clipboard) tool (optional but recommended)
- [`just`](https://github.com/casey/just#installation) (optional, recommended when compiling from source)

## quickstart

### initializing a new slate

create your very first, encrypted file at `~/.sheikah/eye`

```sh
sheikah init
```

> optionally, you can specify a new name with the `-S` flag; more information about your slate can be found in [multiple slates](#multiple-slates)

### basic operations

```console
$ sheikah add
  # follow an interactive prompt, or via arguments
$ sheikah show gentoo
  # display all available information about the gentoo entry
```

> see [reference](#reference) for more in-depth examples

also make sure all [requirements](#dependencies) are satisfied after initializing a new slate:

```sh
sheikah doctor
```

## reference

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
  ssh          run ssh with last arg being your desired entry
  list, -l     list all entries
  find, -f     fuzzy pick an entry, optionally seeded with QUERY
  config, -C   open the config with your preferred $EDITOR
  passwd       change the master password
  backup, -b   pick one of the last saves & restore it
  completions  print or install a zsh completion script
  help         print this message or the help of the given subcommand(s)

Arguments:
  [REF]  entry to show: service name or search text

Options:
  -V, --version
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
  -T, --type <TYPE>   login | recovery | ssh
  -t, --tags <STR>    comma-separated tags (git, dev)
  -d <STR>            domain/url (ssh host for type=ssh)
  -S, --slate <NAME>  slate name (or unique prefix) or path/to/slate [env: SHEIKAH_SLATE=]
  -e <STR>            email
  -u <STR>            username/login
  -n <STR>            optional notes
```

**types** (`-T` / `--type`; interactive `add` prompts when omitted, otherwise defaults to `login`):

| type | prompts |
|---|---|
| `login` | username, email, url, tags, notes, then a password |
| `recovery` | same fields as `login`, then recovery codes instead of a password (replaces `-r`) |
| `ssh` | host, username, port, password |

with `ui.verbose = true` (the default), interactive `add` / `edit` print a dimmed `login | recovery | ssh` line before `type:`. `verbose = false` hides that line. a bad type only says `unknown type`.

### example usages

every action works as a word or a single-dash flag

```console
$ sheikah add -s 'csfloat' -t 'trading, market'
  # adds a new service with tags
$ sheikah -lb
  # list all entries inside a box (`-b` here = boxed, not backup)
$ sheikah list --tag trading
  # list only entries that are tagged with `trading`
$ sheikah github
  # will look for an entry 'github', picker will pop up if there are multiple
$ sheikah -s github -p | docker login -u v --password-stdin
  # print only the password (-p) of 'github' & hand it to docker
$ sheikah ssh pi -- rpi
  # regular ssh workflow, entry after `--` supplies the passphrase
$ sheikah -c github
  # copy the password into your clipboard, clears after a period of time
```

**several notes to take care of**:

- `-s` corresponds to `show` at top level, but as sub-argument to `add` becomes the site label
- `-b` corresponds to `backup` at top level, but as sub-argument to `list` becomes `--boxed`
- `ssh` reads passphrases from the terminal, never from stdin, so `sheikah -s rpi -p | ssh pi` cannot work. `sheikah ssh` runs ssh with `SSH_ASKPASS` pointing back at sheikah (needs OpenSSH 8.4+). the first passphrase/password prompt gets the entry's password through a private socket in `$XDG_RUNTIME_DIR`; later prompts & host key questions go to your terminal. any other arguments supplied to `ssh` will work as per usual

### multiple slates

`-S` is a slate name under `~/.sheikah/` or a path, e.g. `sheikah -S work init`. `init` without `-S` creates `~/.sheikah/eye`

- with only one slate, `-S` is never needed
- a unique prefix is enough: `-S w` picks `work`
- with several slates & no `-S`, fzf asks which one
- `export SHEIKAH_SLATE=work` in your shell rc makes `work` the default, `-S` still overrides it

### configuration

sheikah ships a variety of personalized options via `sheikah -C`/`config`, unlocks the slate, dumps a commented toml into a short-lived `0600` file under `$XDG_RUNTIME_DIR` (or `/dev/shm`) & lets you edit it via your preferred `$VISUAL`/`$EDITOR`. each slate carries its own (encrypted) configuration - there is no shared `~/.sheikah/config.toml`. below follows a customized version of the default template:

```toml
[censor]
glyph = "???"
service = false
username = false
email = true
password = true
url = false
notes = false
tags = false

[table]
index = false
sort = "alphabetical"
boxed = true
style = "ascii"

[clipboard]
clear = 30

[session]
cache = 60

[theme]
name = "0x96f"
header = "#fffeee"

[ui]
verbose = false
inspect_delete = true
```

> an invalid configuration file keeps the changes untouched as you exit

**censor**:
  - `glyph` (str): shown instead of a hidden field
  - `<field>` (bool): when `true`, that field is replaced by `glyph` in `list` & `show` - available fields:
    - `service`
    - `username`
    - `email`
    - `password`
    - `url`
    - `notes`
    - `tags`

> explicit `show -p` / `-u` / .. always reveals the specified field

**table**:
  - `index` (bool): leading `#` column counting rows from 1 in `list`
  - `sort` (str): order of `list` & `find`
    - `alphabetical`: service, then username
    - `modified`: newest edit first
    - `used`: most recently shown / copied / found first. each hit re-saves the slate; previous file goes to `<slate>.used.bak` instead of the rotating backups
  - `boxed` (bool): always draw a box around `list` (same as `-b`)
  - `style` (str): box glyphs styling when `boxed` / `-b` is on:

  | value | style |
  |---|---|
  | `ascii` | +, -, \| |
  | `square` | ┌┬┐, ├┼┤, └┴┘, ─, │ |
  | `rounded` | ╭┬╮, ├┼┤, ╰┴╯, ─, │ |
  | `double` | ╔╦╗, ╠╬╣, ╚╩╝, ═, ║ |
  | `heavy` | ┏┳┓, ┣╋┫, ┗┻┛, ━, ┃ |
  | `dashed` | ┌┬┐, ├┼┤, └┴┘, ╌, ╎ |

**clipboard**:
  - `clear` (int): seconds until a copied secret is wiped after `-c` / `copy`. `0` disables this behaviour. the used clipboard tool is covered in [clipboard](#clipboard)

**session**:
  - `cache` (int): seconds to remember the master password after a successful unlock. stored as a `0600` file under `$XDG_RUNTIME_DIR` (or `/dev/shm`) & wiped on expiry, failed unlock, `passwd`, or `backup` restore. `0` disables caching

**theme**:
  - `name` (str): base16 palette. built-ins & custom schemes in: [themes](#themes)
  - `<role>` (str, optional): override one role & leave the rest alone - available roles:
    - `header`
    - `border`
    - `text`
    - `muted`
    - `key`
    - `secret`
    - `accent`
    - `error`
    - `selection`
  - **accepted values for given roles**:
    - **name convention**: black red green yellow blue magenta cyan white (plus `bright-*`)
    - **base convention**: `baseXX` (see [here](https://github.com/chriskempson/base16/blob/main/styling.md))
    - **rgb**: `#rrggbb`
    - **default**: terminal's own foreground

**ui**:
  - `verbose` (bool): when `false`, hides usage tips & extra detail such as `(optional)` in `add`, the `login | recovery | ssh` type line, `via <tool>, clears in Ns` after `copy`, & `'enter' to keep | '-' to clear` in `edit`
  - `inspect_delete` (bool): `delete` shows the entry before the `[y/N]` confirmation dialogue; `false` skips straight to the confirm

## shell completion

### set up completions for zsh

> [zsh](https://www.zsh.org/) only for now, will commence with [bash](https://www.gnu.org/software/bash/)/[fish](https://fishshell.com/) once this project gains momentum

install the script into the default completions dir:

```sh
sheikah completions update
```

this creates `${XDG_CONFIG_HOME:-~/.config}/zsh/completions/_sheikah`. use `-d`/`--dir <DIR>` when your file does **not** live in `~/.config/zsh/completions`. zsh only picks it up if that dir is in `fpath` before `compinit` runs, so add this to your `~/.zshrc`

```sh
fpath=(~/.config/zsh/completions $fpath)
autoload -Uz compinit && compinit
```

> under an alias symlink, also add `compdef _sheikah <alias>`

if your `~/.zshrc` already runs `compinit` (oh-my-zsh, zinit, ..), only add the `fpath=` line above, then finally reload your shell:

```sh
exec zsh
```

> note that a mere `source ~/.zshrc` is not sufficient

rerun `sheikah completions update` after upgrading the binary

### troubleshooting shell completion

**q**: my shell doesn't display completions after following the [steps above](#shell-completion)

**a**: check these in order:
1. `which sheikah` points at the binary you expect & version is up to date
2. `~/.config/zsh/completions/_sheikah` exists & starts with `#compdef sheikah`
3. `echo $fpath` includes `~/.config/zsh/completions` near the beginning. when using [omz](https://ohmyz.sh/)/[zinit](https://github.com/zdharma-continuum/zinit), put that `fpath=` line above the `compinit` call
4. `autoload -Uz sheikah && which -a _sheikah` finds the function; if not, wipe & restart the shell: `rm -f ~/.zcompdump* && exec zsh`

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
- `0x96f`
- `ascendancy`
- `black-metal`
- `custom` keeps the plain bold/dim look

any other scheme can be imported from the [tinted gallery](https://tinted-theming.github.io/tinted-gallery/) (find the yaml files here: [tinted-theming/schemes](https://github.com/tinted-theming/schemes/tree/spec-0.11/base16)) & saved as `~/.sheikah/themes/<name>.yaml`. don't forget to also set `name = "<name>"` in the [config](#configuration)! a file with a built-in name replaces it & `base16-` prefixes from the gallery are ignored

### customizing the appearance

every other key overrides one role & leaves the rest of the theme alone, e.g. setting your theme to `nord` with your own border color:

```toml
[theme]
name = "nord"
border = "red"
```

[`truecolor`](https://github.com/termstandard/colors/blob/master/README.md) is used when `COLORTERM` is `truecolor`/`24bit`, otherwise the nearest of 256 colors

## clipboard

`sheikah -c` uses the first tool that works, & a detached `sh` empties the clipboard after 45 seconds by default - see [configuration](#configuration) to change the duration

**available clipboards**:

| order | tool | when |
|---|---|---|
| 1 | `wl-copy` | wayland |
| 2 | `xclip -selection clipboard` | x11 |
| 3 | `xsel --clipboard` | x11 |
| 4 | `clip.exe` | wsl |
| 5 | `pbcopy` | macos |


## storage

- `~/.sheikah/` is created with minimal owner permissions (`0700`), slate(s) with stricter permissions (`0600`)
- `init` optionally runs `sudo chattr +a` on the dir ([ref](https://unix.stackexchange.com/questions/269020/what-is-the-effect-of-chattr-a-on-a-directory)), so files in it cannot be deleted or renamed. saves therefore overwrite in place, after copying the previous version to a backup slot. this behaviour only works on certain filesystems (ext4, xfs, btrfs, ...)
- backups rotate through `<slate>.1.bak` .. `<slate>.5.bak`, the oldest slot gets overwritten (never deleted). `sheikah backup` restores one; a backup keeps the master password it was saved with

## crypto, security & passwords

the crypto aspect of this project follows this pattern:

```
[salt (16B)][nonce (12B)][ciphertext + gcm tag (16B)]
```

- key: argon2id(master password, salt) -> 256 bit, `m = 64 MiB, t = 3, p = 1`
- cipher: aes-256-gcm over the json-serialized entries
- every save draws a fresh random salt & nonce
- changing kdf params breaks existing slates, so be careful!

passwords will **always** have an interactive prompt to ensure security via [`rpassword`](https://crates.io/crates/rpassword)
