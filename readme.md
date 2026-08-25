**general layout** (for now)

```
src/
  main.rs // clap parsing + dispatch
  envelope.rs // init/load/save/rekey
  commands.rs // cmd_* functions, find/filter/sort logic
  ui.rs // print helpers, prompt(), pick(), table printer
  entry.rs // entry struct (service, username, email, password, url, notes, category, created, modified)
```

- [ ] add wl-copy/wl-paste with fallback in another way soon
- [ ] create the file in `$HOME/.sheikah/<secret>` with its own name if the user wants one, otherwise fall back to `heirloom` with:
  - 700: `.sheikah/`
  - 600: `heirloom`
  - `sudo chattr -R +i .sheikah/`
