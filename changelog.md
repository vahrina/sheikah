# changelog

all notable changes to this project are documented here.

the format is based on [keep a changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [semantic versioning](https://semver.org/).

## [unreleased]

## 0.3.7 - 2026-11-22

### added

- user defined backup count through config ([`afc7448`](https://github.com/vahrina/sheikah/commit/afc7448))
- backup displays what it'll actually restore, discard & revert ([`7de48fc`](https://github.com/vahrina/sheikah/7de48fcf934822e729b41740ae80a363277c221e))
- bash & fish completions ([`70ee672`](https://github.com/vahrina/sheikah/commit/70ee672))
- `lock` forgets the cached master password on demand ([`be01b63`](https://github.com/vahrina/sheikah/commit/be01b63))
- `destroy` deletes a slate & its backups after listing what is lost & asking for its name ([`be01b63`](https://github.com/vahrina/sheikah/commit/be01b63))
- `export` (json/csv) & `import` ([`fa964e5`](https://github.com/vahrina/sheikah/commit/fa964e5))
  - tested: sheikah & bitwarden
  - untested: chrome, firefox, keepassxc, 1password, lastpass, proton pass

### changed

- drop `pbcopy` (macos) clipboard backend, linux only ([`2346ef9`](https://github.com/vahrina/sheikah/commit/2346ef9))
- `ui.confirm_overwrite` (default `false`) asks before `edit` saves or `passwd` rekeys ([`f7b3610`](https://github.com/vahrina/sheikah/commit/f7b3610))
- edit tips: navigation (in `add`/`edit`) & invalid field messages (`service` & `type`) display always regardless of verbosity ([`371e8e6`](https://github.com/vahrina/sheikah/commit/371e8e6))
- `sheikah completions` installs completions for the shell it runs from; `zsh` & `update` are gone, `-d` stays ([`a6c102a`](https://github.com/vahrina/sheikah/commit/a6c102a))
- `just install` includes the stable toolchain via `+stable` ([`064c80d`](https://github.com/vahrina/sheikah/commit/064c80d))

### docs

- backup count ([`b255b89`](https://github.com/vahrina/sheikah/commit/b255b89))
- export & import ([`fa964e5`](https://github.com/vahrina/sheikah/commit/fa964e5))
- `lock`, `destroy` & `ui.confirm_overwrite`
- shell completion: bash/fish paths, auto-detect via `sheikah completions`
- clipboard: drop `pbcopy` (macos)

### chores

- deps: cargo update ([`b7cd570`](https://github.com/vahrina/sheikah/commit/b7cd570))

## [0.3.6] - 2026-11-17

### added

- cycle through `add`/`edit` via <kbd>ctrl</kbd>-<kbd>j</kbd>/<kbd>k</kbd> or <kbd>up</kbd>/<kbd>down</kbd> ([`df0c552`](https://github.com/vahrina/sheikah/commit/df0c5528937a12a135ba57a11573b3e28f090609))
- uploaded to [`crates.io`](https://crates.io/crates/sheikah) & added installation instructions for cargo
- `sheikah completions update [--dir <DIR>]` installs or refreshes zsh completions automatically ([`7deadf8`](https://github.com/vahrina/sheikah/commit/7deadf83763f8e3c48f10b3222e361f7d13e8e08))
- master password cache ([#3](https://github.com/vahrina/sheikah/pull/3), thanks @ttkm)
- add entry types, remove -r ([#2](https://github.com/vahrina/sheikah/pull/2), thanks @ttkm)
- allow empty passwords ([`086e43e`](https://github.com/vahrina/sheikah/commit/086e43e3cccfa614183a8d21ad88726964f12859))

### changed

- split `cli_tests.rs` into its own dedicated dir @ `src/tests` ([`82387b9`](https://github.com/vahrina/sheikah/commit/82387b9f0a30d74c4c66a35052b0c7cfa9541b87))
- readme: dedupe, grammatical mistakes & some reorganizing ([`71183cb`](https://github.com/vahrina/sheikah/commit/71183cbccc839b23c395fda1e1797eb7ec8d4936), [`49f1b5f`](https://github.com/vahrina/sheikah/commit/49f1b5fde904a6ffbcd2df73401c697cea1f9086), [`dacb3c5`](https://github.com/vahrina/sheikah/commit/dacb3c54275d06d93fde3cf15ef6593d981f0974), [`7eb83b1`](https://github.com/vahrina/sheikah/commit/7eb83b11ff7e999bd97e80b3467f1d0260aea510))

### removed

- `-r`/`--recovery` flag, replaced by entry types ([#2](https://github.com/vahrina/sheikah/pull/2)) (**breaking**)
- `-P`/`--plain` flag ([`4efe797`](https://github.com/vahrina/sheikah/commit/4efe79703a84578b091355b4dd88cf43bf46c6eb)) (**breaking**)

### chores

- deps: cargo update ([`9e8fbf1`](https://github.com/vahrina/sheikah/commit/9e8fbf1dbee7293261e14724036e93c3aad1ea33))
- deps: cargo update ([`6607036`](https://github.com/vahrina/sheikah/commit/66070369d7677244501022f34db3c33dc66958b3))

## [0.3.5] - 2026-10-30

### added

- `doctor` checks for `ssh` & warns below OpenSSH 8.4, which `sheikah ssh` needs
- `ui.inspect_delete`: `delete` shows the entry before confirming
- `ui.verbose`: `false` hides usage tips & extra detail, e.g. `(optional)` in `add`, `via xclip, clears in 45s` after `copy` & the `'enter' to keep` hint in `edit`
- `sheikah ssh [SSH_ARGS]... -- <REF>`: unlock an entry & run ssh with its password via `SSH_ASKPASS` ([OpenSSH 8.4](https://www.openssh.org/txt/release-8.4)+). piping `sheikah -s rpi -p | ssh pi` cannot work; ssh reads passphrases from `/dev/tty`, not stdin

### fixed

- zsh completion: `sheikah add -<tab>` (& every other subcommand) lists its flags right away; the optional top-level `REF` used to swallow the subcommand name, so only `sheikah add add -<tab>` worked before

### changed

- readme: documented `configuration` & drop the duplicate explanation for sorting & whatever else i found
- swap configuration `[table]` keys
  - before: boxed, index, style, sort
  - after: index, sort, boxed, style

## [0.3.4] - 2026-10-28

### added

- `completions zsh` prints a zsh completion script
- add a description for args: `add`, `edit`, `delete` (quite obvious what they do, but it messes with newlines in `completions`) & fix one missing slash to print the `config` description as well

## [0.3.3] - 2026-10-28

### added

- `censor.glyph` sets the placeholder for hidden fields
- `table.sort` applies different sorting filters: alphabetical, last modified & last used
- `clipboard.clear` allows to change the amount of time (in seconds) it takes for a copied secret to be wiped. default is 45; 0 disables this behaviour

### changed

- hidden fields show `***` instead of `•••` by default

## [0.3.2] - 2026-10-28

### added

- base16 themes: `0x96f` (default), `ascendancy`, `black-metal`, `custom`, or any scheme in `~/.sheikah/themes/`
- per-role color overrides in `[theme]`: names, `#rrggbb`, `baseXX` & `default`
- fzf picker follows the theme

## [0.3.1] - 2026-10-22

### added

- `index` boolean option in the config

## [0.3.0] - 2026-10-22

### added

- recovery code argument, subcommand in `sheikah add`
- installation methods documented in the readme

### changed

- shorter release archive names

## [0.2.0] - 2026-10-22

first tagged release & retroactively documented from git history

### added

- `init` initializes the data directory with `chattr` & asks whether to set the attribute
- `add` command (non-interactive arguments)
- `passwd` command to change the password of specified files
- rotating backups
- slate (file) picker
- `justfile`, ci workflow, `deny.toml` & licensing

### changed

- [`fzf`](https://github.com/junegunn/fzf) is now required
- data file renamed from `heirloom.json` -> `eye.enc`
- minor ui adjustments

### fixed

- wrong directory permissions (octal instead of hex)

[unreleased]: https://github.com/vahrina/sheikah/compare/v0.3.7...head
[0.3.7]: https://github.com/vahrina/sheikah/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/vahrina/sheikah/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/vahrina/sheikah/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/vahrina/sheikah/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/vahrina/sheikah/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/vahrina/sheikah/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/vahrina/sheikah/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/vahrina/sheikah/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/vahrina/sheikah/releases/tag/v0.2.0
