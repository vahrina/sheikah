# changelog

all notable changes to this project are documented here.

the format is based on [keep a changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [semantic versioning](https://semver.org/).

## [unreleased]

### added

- `ui.inspect_delete`: `delete` offers to show the entry before confirming
- `ui.verbose`: `false` hides usage tips & extra detail, e.g. `(optional)` in `add`, `via xclip, clears in 45s` after `copy` & the `'enter' to keep` hint in `edit`
- `sheikah ssh [SSH_ARGS]... -- <REF>`: unlock an entry & run ssh with its password via `SSH_ASKPASS` ([OpenSSH 8.4](https://www.openssh.org/txt/release-8.4)+). piping `sheikah -s rpi -p | ssh pi` cannot work; ssh reads passphrases from `/dev/tty`, not stdin

### changes

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

[unreleased]: https://github.com/vahrina/sheikah/compare/v0.3.5...head
[0.3.5]: https://github.com/vahrina/sheikah/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/vahrina/sheikah/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/vahrina/sheikah/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/vahrina/sheikah/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/vahrina/sheikah/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/vahrina/sheikah/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/vahrina/sheikah/releases/tag/v0.2.0
