# changelog

all notable changes to this project are documented here.

the format is based on [keep a changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [semantic versioning](https://semver.org/).

## [unreleased]

## 0.3.2 - 2026-10-28

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

[unreleased]: https://github.com/vahrina/sheikah/compare/v0.3.1...head
[0.3.1]: https://github.com/vahrina/sheikah/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/vahrina/sheikah/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/vahrina/sheikah/releases/tag/v0.2.0
