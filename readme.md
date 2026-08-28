**general layout** (for now)

```
src/
  args.rs // clap parsing
  ?.rs // init/load/save/rekey
  commands.rs // cmd_* functions, find/filter/sort logic
  ui.rs // print helpers, prompt(), pick(), table printer
  entry.rs // entry struct (service, username, email, password, url, notes, category, created, modified)
```

- [ ] add wl-copy/wl-paste with fallback in another way soon
- [ ] create the file in `$HOME/.sheikah/<secret>` with its own name if the user wants one, otherwise fall back to `eye` with:
  - [x] 700: `.sheikah/`
  - [x] 600: `eye.enc`
  - [x] `sudo chattr +a` on dir ([ref](https://unix.stackexchange.com/questions/269020/what-is-the-effect-of-chattr-a-on-a-directory))
- [ ] encrypt file via `argon2` + `aes-gcm` with `rand` (salt/nonce) over previously used `pbkdf2` -> derive 256b key from mpass
  ```c
  [salt (16B)[nonce (12B)][c + gcm tag]] // something like that
  ```
