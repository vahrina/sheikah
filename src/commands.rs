use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use secrecy::ExposeSecret;
use std::{
    cmp::Reverse,
    env::var_os,
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};
use zeroize::Zeroizing;

use crate::{
    args::{AddArgs, Cli, Commands, ShowArgs},
    askpass, cache, clip,
    config::{self, Config, Sort},
    entry::{self, Entry},
    slate::{self, Slate},
    theme, transfer, ui,
};

pub(crate) fn run(cli: Cli) -> Result<()> {
    // `$SHEIKAH_VAULT` predates the slate rename
    let env_vault = var_os("SHEIKAH_VAULT").map(|v| v.to_string_lossy().into_owned());
    let s = cli.slate.as_deref().or(env_vault.as_deref());

    let needs_fzf = !matches!(
        (&cli.command, &cli.reference),
        (
            Some(
                Commands::Doctor
                    | Commands::Lock
                    | Commands::Help { .. }
                    | Commands::Completions { .. }
            ),
            _
        ) | (None, None)
    );
    if needs_fzf && !clip::on_path("fzf") {
        bail!(ui::FZF_MISSING);
    }

    match cli.command {
        Some(Commands::Add(a)) => cmd_add(s, a),
        Some(Commands::Edit {
            reference,
            kind,
            tags,
        }) => cmd_edit(s, reference.as_deref().unwrap_or(""), kind, tags.as_deref()),
        Some(Commands::Delete { reference }) => cmd_delete(s, reference.as_deref().unwrap_or("")),
        Some(Commands::Config { slate }) => cmd_config(slate.as_deref().or(s)),
        Some(Commands::Init) => cmd_init(s),
        Some(Commands::Passwd) => cmd_passwd(s),
        Some(Commands::Lock) => cmd_lock(s),
        Some(Commands::Destroy) => cmd_destroy(s),
        Some(Commands::Doctor) => cmd_doctor(s),
        Some(Commands::Backup) => cmd_backup(s),
        Some(Commands::Export { file }) => cmd_export(s, Path::new(&file)),
        Some(Commands::Import { file }) => cmd_import(s, Path::new(&file)),
        Some(Commands::Show(a)) => cmd_show(s, a),
        Some(Commands::Copy { reference }) => cmd_copy(s, reference.as_deref().unwrap_or("")),
        Some(Commands::Ssh { args }) => {
            let Some(i) = args.iter().position(|a| a == "--") else {
                bail!("usage: sheikah ssh [SSH_ARGS]... -- <REF>");
            };
            let (ssh_args, rest) = args.split_at(i);
            match &rest[1..] {
                [reference] if !ssh_args.is_empty() => cmd_ssh(s, reference, ssh_args),
                _ => bail!("usage: sheikah ssh [SSH_ARGS]... -- <REF>"),
            }
        }
        Some(Commands::List { boxed, tag }) => cmd_list(s, boxed, &tag),
        Some(Commands::Find { query, tag }) => cmd_find(s, query.as_deref().unwrap_or(""), &tag),
        Some(Commands::Help { command }) => cmd_help(command.as_deref()),
        Some(Commands::Completions { dir }) => cmd_completions(dir.as_deref()),
        None => match cli.reference {
            Some(r) => cmd_show(
                s,
                ShowArgs {
                    reference: Some(r),
                    ..Default::default()
                },
            ),
            None => cmd_help(None),
        },
    }
}

/// `help show` or `help -s`
fn cmd_help(sub: Option<&str>) -> Result<()> {
    let mut cmd = Cli::command();
    cmd.build();
    let Some(sub) = sub else {
        cmd.print_help()?;
        return Ok(());
    };
    let short = sub.strip_prefix('-').and_then(|f| f.parse::<char>().ok());
    match cmd
        .get_subcommands_mut()
        .find(|c| c.get_name() == sub || short.is_some_and(|f| c.get_short_flag() == Some(f)))
    {
        Some(c) => c.print_help()?,
        None => bail!("unknown command: {sub}"),
    }
    Ok(())
}

/// the shell that ran us (`/proc/<ppid>/comm`), else the login shell from `$SHELL`
fn current_shell() -> String {
    let parent = fs::read_to_string(format!(
        "/proc/{}/comm",
        std::os::unix::process::parent_id()
    ));
    let name = |s: &str| s.trim().trim_start_matches('-').to_string();
    match parent.map(|p| name(&p)) {
        Ok(p) if matches!(p.as_str(), "bash" | "fish" | "zsh") => p,
        _ => {
            let login = var_os("SHELL").unwrap_or_default();
            name(
                &Path::new(&login)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
        }
    }
}

fn cmd_completions(dir: Option<&str>) -> Result<()> {
    use clap_complete::Shell;
    let xdg = |var: &str, fallback: &str| {
        var_os(var)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join(fallback)
            })
    };
    let config = xdg("XDG_CONFIG_HOME", ".config");
    let shell = current_shell();
    let (sh, default, file) = match shell.as_str() {
        "zsh" => (Shell::Zsh, config.join("zsh/completions"), "_sheikah"),
        "fish" => (Shell::Fish, config.join("fish/completions"), "sheikah.fish"),
        "bash" => (
            Shell::Bash,
            xdg("XDG_DATA_HOME", ".local/share").join("bash-completion/completions"),
            "sheikah",
        ),
        _ => bail!("unsupported shell {shell:?}, run this from bash, fish or zsh"),
    };
    let dir = dir.map_or(default, PathBuf::from);
    fs::create_dir_all(&dir)?;
    let path = dir.join(file);
    completions(sh, &mut fs::File::create(&path)?);
    ui::say(&format!("updated {shell} completions: {}", tilde(&path)));
    ui::say(&format!("run `exec {shell}` to load them"));
    Ok(())
}

/// the optional top-level REF would claim the first word, so `sheikah add -<tab>` completed
/// as if `add` were an entry; completion only offers subcommands there
pub(crate) fn completions(shell: clap_complete::Shell, out: &mut dyn Write) {
    let full = Cli::command();
    let name = full.get_name().to_string();
    let mut cmd = clap::Command::new(name.clone())
        .version(full.get_version().unwrap_or_default().to_string())
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .args(
            full.get_arguments()
                .filter(|a| a.get_id() != "reference")
                .cloned(),
        )
        .subcommands(full.get_subcommands().cloned());
    clap_complete::generate(shell, &mut cmd, name, out);
}

/// `~/.sheikah/eye` instead of `/home/you/.sheikah/eye`
fn tilde(p: &Path) -> String {
    match var_os("HOME").map(PathBuf::from) {
        Some(h) if p.starts_with(&h) => {
            format!("~/{}", p.strip_prefix(&h).unwrap_or(p).display())
        }
        _ => p.display().to_string(),
    }
}

/// a tmpfs keeps decrypted data off disk; never ~/.sheikah, which may be `chattr +a`
pub(crate) fn runtime_dir() -> PathBuf {
    var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("/dev/shm"))
}

fn remember(path: &Path, pass: &secrecy::SecretString, secs: u32) {
    if let Err(e) = cache::store(path, pass, secs) {
        ui::note(&format!("could not cache master password: {e:#}"));
    }
}

fn unlock(name: Option<&str>) -> Result<Slate> {
    let path = slate::resolve(name)?;
    let cached = cache::load(&path);
    let used_cache = cached.is_some();
    let pass = match cached {
        Some(p) => Zeroizing::new(p.expose_secret().to_owned()),
        None => ui::secret("master password")?,
    };
    let secret = secrecy::SecretString::from(pass.as_str().to_owned());
    let s = match Slate::load(path.clone(), pass) {
        Ok(s) => s,
        Err(_) if used_cache => {
            cache::clear(&path);
            let pass = ui::secret("master password")?;
            let secret = secrecy::SecretString::from(pass.as_str().to_owned());
            let s = Slate::load(path.clone(), pass)?;
            ui::set_theme(&s.config.theme);
            ui::set_verbose(s.config.ui.verbose);
            remember(&path, &secret, s.config.session.cache);
            return Ok(s);
        }
        Err(e) => return Err(e),
    };
    ui::set_theme(&s.config.theme);
    ui::set_verbose(s.config.ui.verbose);
    remember(&path, &secret, s.config.session.cache);
    Ok(s)
}

fn cmd_init(name: Option<&str>) -> Result<()> {
    let path = slate::init_path(name)?;
    if path.exists() {
        bail!("refusing to overwrite existing file: {}", tilde(&path));
    }
    let pass = ui::new_secret("master password")?;
    slate::init(&path, &pass)?;
    ui::say(&format!("slate ready: {}", tilde(&path)));

    if let Some(dir) = path.parent()
        && ui::confirm(&format!(
            "protect {} from deletion via `sudo chattr +a`?",
            tilde(dir)
        ))?
    {
        match Command::new("sudo")
            .args(["chattr", "+a"])
            .arg(dir)
            .output()
        {
            Ok(o) if o.status.success() => ui::say(&format!("chattr +a applied to {}", tilde(dir))),
            Ok(o) => ui::note(&format!(
                "chattr +a failed, deletion protection is NOT applied: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => ui::note(&format!(
                "could not run sudo chattr, protection NOT applied: {e}"
            )),
        }
    }
    Ok(())
}

/// form keys shown per type, after `service` & `type`
fn type_keys(kind: entry::EntryType) -> &'static [&'static str] {
    match kind {
        entry::EntryType::Login | entry::EntryType::Recovery => {
            &["username", "email", "url", "tags", "notes"]
        }
        entry::EntryType::Ssh => &["host", "username", "port"],
    }
}

/// empty means `fallback`
fn form_type(raw: &str, fallback: entry::EntryType) -> Result<entry::EntryType, String> {
    match raw.trim() {
        "" | "-" => Ok(fallback),
        x => x.parse(),
    }
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn take_or_prompt(val: Option<String>, label: &str, ask: bool) -> Result<Option<String>> {
    match val {
        Some(x) => Ok((!x.is_empty()).then_some(x)),
        None if ask => ui::prompt_opt(label),
        None => Ok(None),
    }
}

/// `add -s ...`: fields come from args; ssh still asks for missing host, username & port
fn add_from_args(a: AddArgs) -> Result<Entry> {
    let service = a.service.unwrap_or_default();
    if service.is_empty() {
        bail!("service must not be empty");
    }
    let mut e = Entry::new(service, String::new());
    e.kind = a.kind.unwrap_or_default();
    let opt = |v: Option<String>| v.filter(|x| !x.is_empty());
    e.tags = a.tags.as_deref().map(entry::parse_tags).unwrap_or_default();
    e.notes = opt(a.notes);
    match e.kind {
        entry::EntryType::Login | entry::EntryType::Recovery => {
            e.username = opt(a.user);
            e.email = opt(a.email);
            e.url = opt(a.domain);
        }
        entry::EntryType::Ssh => {
            e.url = take_or_prompt(a.domain, "host", true)?;
            e.username = take_or_prompt(a.user, "username", true)?;
            e.port = take_or_prompt(None, "port", true)?;
        }
    }
    Ok(e)
}

/// plain `add`: one form, args prefill their fields
fn add_from_form(a: &AddArgs) -> Result<Entry> {
    let opt = |l: &str| format!("{l}{}", ui::hint(" (optional)"));
    let arg = |v: &Option<String>| v.clone().unwrap_or_default();
    let type_label = format!(
        "type{}",
        ui::hint(&format!(" ({})", entry::EntryType::CHOICES))
    );
    let mut f = vec![
        ui::Field::new("service", "service", ""),
        ui::Field::new("type", type_label, ""),
        ui::Field::new("username", opt("username"), arg(&a.user)),
        ui::Field::new("email", opt("email"), arg(&a.email)),
        ui::Field::new("url", opt("url"), arg(&a.domain)),
        ui::Field::new("host", "host", arg(&a.domain)),
        ui::Field::new("port", "port", ""),
        ui::Field::new("tags", opt("tags"), arg(&a.tags)),
        ui::Field::new("notes", opt("notes"), arg(&a.notes)),
    ];
    let fixed = a.kind;
    let kind_of = |f: &[ui::Field]| {
        fixed.map_or_else(|| form_type(ui::value(f, "type"), Default::default()), Ok)
    };
    ui::form(
        &mut f,
        |f| {
            let mut keys = vec!["service"];
            if fixed.is_none() {
                keys.push("type");
            }
            keys.extend(type_keys(kind_of(f).unwrap_or_default()));
            keys
        },
        |f| {
            if non_empty(ui::value(f, "service")).is_none() {
                Some(("service", "service must not be empty".into()))
            } else {
                kind_of(f).err().map(|m| ("type", m))
            }
        },
    )?;
    let v = |k| non_empty(ui::value(&f, k));
    let kind = kind_of(&f).unwrap_or_default();
    let mut e = Entry::new(v("service").unwrap_or_default(), String::new());
    e.kind = kind;
    match kind {
        entry::EntryType::Login | entry::EntryType::Recovery => {
            e.username = v("username");
            e.email = v("email");
            e.url = v("url");
            e.tags = entry::parse_tags(ui::value(&f, "tags"));
            e.notes = v("notes");
        }
        entry::EntryType::Ssh => {
            e.url = v("host");
            e.username = v("username");
            e.port = v("port");
            e.tags = a.tags.as_deref().map(entry::parse_tags).unwrap_or_default();
            e.notes = a.notes.clone().filter(|n| !n.is_empty());
        }
    }
    Ok(e)
}

fn cmd_add(name: Option<&str>, a: AddArgs) -> Result<()> {
    let mut s = unlock(name)?;
    let mut e = if a.service.is_some() {
        add_from_args(a)?
    } else {
        add_from_form(&a)?
    };
    if e.kind.uses_recovery() {
        e.recovery = ui::prompt_lines("recovery codes")?;
    } else {
        e.password = ui::new_secret("password")?.to_string();
    }

    let svc = e.service.clone();
    s.entries.push(e);
    s.save()?;
    ui::say(&format!("added {svc}"));
    Ok(())
}

fn normalize_tags(tag: &[String]) -> Vec<String> {
    tag.iter()
        .flat_map(|t| entry::parse_tags(t))
        .fold(Vec::new(), |mut acc, t| {
            if !acc.contains(&t) {
                acc.push(t);
            }
            acc
        })
}

fn with_tags(entries: &[Entry], tag: &[String]) -> Vec<usize> {
    let required = normalize_tags(tag);
    (0..entries.len())
        .filter(|&i| entries[i].has_tags(&required))
        .collect()
}

fn cmd_list(name: Option<&str>, boxed: bool, tag: &[String]) -> Result<()> {
    let s = unlock(name)?;
    let allowed = with_tags(&s.entries, tag);
    let idx: Vec<usize> = sorted(&s.entries, s.config.table.sort)
        .into_iter()
        .filter(|i| allowed.contains(i))
        .collect();
    ui::table(&s.entries, &idx, &s.config, boxed);
    Ok(())
}

/// stamps `used` only while sorting by it, so reads stay read-only otherwise. best-effort
fn mark_used(s: &mut Slate, i: usize) {
    if s.config.table.sort != Sort::Used {
        return;
    }
    s.entries[i].used = Some(entry::now());
    if let Err(e) = s.save_used() {
        ui::note(&format!("could not record last use: {e:#}"));
    }
}

fn cmd_find(name: Option<&str>, q: &str, tag: &[String]) -> Result<()> {
    let mut s = unlock(name)?;
    let allowed = with_tags(&s.entries, tag);
    if allowed.is_empty() {
        if tag.is_empty() {
            bail!("slate is empty (add one with `sheikah add`)");
        }
        bail!("no entries match the given tag(s)");
    }
    let order: Vec<usize> = sorted(&s.entries, s.config.table.sort)
        .into_iter()
        .filter(|i| allowed.contains(i))
        .collect();
    let i = ui::pick(&s.entries, &order, q)?;
    ui::detail(&s.entries[i], &s.config, &[]);
    mark_used(&mut s, i);
    Ok(())
}

fn cmd_show(name: Option<&str>, a: ShowArgs) -> Result<()> {
    let mut s = unlock(name)?;
    let allowed = with_tags(&s.entries, &a.tag);
    if !a.tag.is_empty() && allowed.is_empty() {
        bail!("no entries match the given tag(s)");
    }
    let i = find_ref_among(
        &s.entries,
        &allowed,
        a.reference.as_deref().unwrap_or(""),
        s.config.table.sort,
    )?;
    let only: Vec<&str> = [
        (a.service, "service"),
        (a.username, "username"),
        (a.email, "email"),
        (a.password, "password"),
        (a.url, "url"),
        (a.tags, "tags"),
        (a.notes, "notes"),
    ]
    .into_iter()
    .filter_map(|(on, k)| on.then_some(k))
    .collect();
    ui::detail(&s.entries[i], &s.config, &only);
    mark_used(&mut s, i);
    Ok(())
}

fn cmd_copy(name: Option<&str>, r: &str) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r, s.config.table.sort)?;
    let e = &s.entries[i];
    let secs = s.config.clipboard.clear;
    let via = clip::copy(&e.secret(), secs)?;
    let when = match secs {
        0 => format!(" via {via}, never cleared"),
        n => format!(" via {via}, clears in {n}s"),
    };
    ui::say(&format!("copied {}{}", e.service, ui::out_hint(&when)));
    mark_used(&mut s, i);
    Ok(())
}

fn cmd_ssh(name: Option<&str>, r: &str, args: &[String]) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r, s.config.table.sort)?;
    if s.entries[i].is_recovery() {
        bail!(
            "{} holds recovery codes, not a password",
            s.entries[i].service
        );
    }
    let secret = Zeroizing::new(s.entries[i].password.clone());
    mark_used(&mut s, i);
    drop(s);
    match askpass::ssh(secret, args)? {
        0 => Ok(()),
        code => std::process::exit(code),
    }
}

fn cmd_edit(
    name: Option<&str>,
    r: &str,
    kind: Option<entry::EntryType>,
    tags: Option<&str>,
) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r, s.config.table.sort)?;
    let e = &mut s.entries[i];
    ui::tip("'enter' to keep | '-' to clear");

    let cur = |v: Option<&str>| ui::edim(&format!("[{}]", v.unwrap_or("-")));
    let lbl = |k: &str, v: Option<&str>| format!("{k} {}", cur(v));
    let was = e.entry_type();
    let shown_tags = e.tags_display();
    let mut f = vec![
        ui::Field::new("service", lbl("service", Some(&e.service)), ""),
        ui::Field::new("type", lbl("type", Some(was.as_str())), ""),
        ui::Field::new("username", lbl("username", e.username.as_deref()), ""),
        ui::Field::new("email", lbl("email", e.email.as_deref()), ""),
        ui::Field::new("url", lbl("url", e.url.as_deref()), ""),
        ui::Field::new("host", lbl("host", e.url.as_deref()), ""),
        ui::Field::new("port", lbl("port", e.port.as_deref()), ""),
        ui::Field::new("notes", lbl("notes", e.notes.as_deref()), ""),
        ui::Field::new("tags", lbl("tags", shown_tags.as_deref()), ""),
    ];
    let kind_of = |f: &[ui::Field]| kind.map_or_else(|| form_type(ui::value(f, "type"), was), Ok);
    ui::form(
        &mut f,
        |f| {
            let mut keys = vec!["service"];
            if kind.is_none() {
                keys.push("type");
            }
            let k = kind_of(f).unwrap_or(was);
            keys.extend(type_keys(k).iter().filter(|&&x| x != "tags"));
            if tags.is_none() {
                keys.push("tags");
            }
            keys
        },
        |f| kind_of(f).err().map(|m| ("type", m)),
    )?;
    let new_kind = kind_of(&f).unwrap_or(was);

    let apply = |raw: &str, cur: &mut Option<String>| match raw.trim() {
        "" => {}
        "-" => *cur = None,
        x => *cur = Some(x.to_string()),
    };
    let v = |k| ui::value(&f, k);
    if let Some(x) = non_empty(v("service"))
        && x != "-"
    {
        e.service = x;
    }
    e.kind = new_kind;
    match new_kind {
        entry::EntryType::Login | entry::EntryType::Recovery => {
            apply(v("username"), &mut e.username);
            apply(v("email"), &mut e.email);
            apply(v("url"), &mut e.url);
            apply(v("notes"), &mut e.notes);
        }
        entry::EntryType::Ssh => {
            apply(v("host"), &mut e.url);
            apply(v("username"), &mut e.username);
            apply(v("port"), &mut e.port);
        }
    }
    match tags {
        Some(t) => e.tags = entry::parse_tags(t),
        None => match v("tags").trim() {
            "" => {}
            "-" => e.tags.clear(),
            x => e.tags = entry::parse_tags(x),
        },
    }

    if e.entry_type().uses_recovery() {
        if ui::confirm("replace recovery codes?")? {
            e.recovery = ui::prompt_lines("recovery codes")?;
            e.password.clear();
        }
    } else if ui::confirm("change password?")? {
        e.password = ui::new_secret("password")?.to_string();
        e.recovery.clear();
    }
    e.touch();

    let svc = e.service.clone();
    if s.config.ui.confirm_overwrite && !ui::confirm("save changes?")? {
        ui::say(&format!("kept {svc} unchanged"));
        return Ok(());
    }
    s.save()?;
    ui::say(&format!("updated {svc}"));
    Ok(())
}

fn cmd_delete(name: Option<&str>, r: &str) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r, s.config.table.sort)?;
    if s.config.ui.inspect_delete {
        ui::detail(&s.entries[i], &s.config, &[]);
    }
    if !ui::confirm(&format!("delete {}?", s.entries[i].service))? {
        ui::say(&format!("kept {}", s.entries[i].service));
        return Ok(());
    }
    let e = s.entries.remove(i);
    s.save()?;
    ui::say(&format!("deleted {}", e.service));
    Ok(())
}

fn cmd_backup(name: Option<&str>) -> Result<()> {
    let path = slate::resolve(name)?;
    let baks = slate::backups(&path);
    if baks.is_empty() {
        bail!(
            "no backups of {} yet, one is made on every save",
            tilde(&path)
        );
    }
    // the current slate may be the broken one, so it never has to open
    let mut pass = match cache::load(&path) {
        Some(p) => Zeroizing::new(p.expose_secret().to_owned()),
        None => ui::secret("master password")?,
    };
    let current = slate::peek(&path, &pass).ok();
    let peeked: Vec<Option<(Config, Vec<Entry>)>> = baks
        .iter()
        .map(|(p, _)| slate::peek(p, &pass).ok())
        .collect();

    let lines: Vec<(usize, String)> = baks
        .iter()
        .zip(&peeked)
        .enumerate()
        .map(|(i, ((p, t), e))| {
            let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let summary = e
                .as_ref()
                .map_or("other master password".into(), |(_, e)| backup_summary(e));
            (
                i,
                format!("{}  {}  {summary}", entry::rfc3339(secs), tilde(p)),
            )
        })
        .collect();
    let pick = ui::choose(&lines, "")?;
    let bak = &baks[pick].0;

    let (bak_config, entries) = match &peeked[pick] {
        Some(e) => e.clone(),
        None => {
            ui::note("this backup was saved with another master password");
            pass = ui::secret("backup master password")?;
            slate::peek(bak, &pass)?
        }
    };
    match &current {
        Some((_, cur)) => backup_diff(cur, &entries),
        None => ui::note("current slate does not open, cannot compare"),
    }
    let keep = current
        .as_ref()
        .map_or(bak_config.backup.keep, |(c, _)| c.backup.keep);

    if !ui::confirm(&format!(
        "restore {}? the current slate becomes a backup",
        tilde(bak)
    ))? {
        ui::say("nothing restored");
        return Ok(());
    }
    slate::restore(&path, bak, &pass, keep)?;
    cache::clear(&path);
    ui::say(&format!("restored {}", tilde(bak)));
    Ok(())
}

/// `12 entries, last github @ 2026-10-21T01:48:00Z`
fn backup_summary(entries: &[Entry]) -> String {
    let n = entries.len();
    match entries.iter().max_by(|a, b| a.created.cmp(&b.created)) {
        Some(e) => format!("{n} entries, last {} @ {}", e.service, e.created),
        None => "empty".into(),
    }
}

/// services the restore brings back (+), loses (-) or changes (~)
fn backup_diff(current: &[Entry], backup: &[Entry]) {
    let find = |es: &[Entry], s: &str| {
        es.iter()
            .find(|e| e.service == s)
            .map(|e| e.modified.clone())
    };
    let (back, lost, reverted) = (ui::eaccent("+"), ui::eerror("-"), ui::ekey("~"));
    let mut lines = Vec::new();
    for e in backup {
        match find(current, &e.service) {
            None => lines.push(format!("{back} {}", e.service)),
            Some(m) if m != e.modified => lines.push(format!("{reverted} {}", e.service)),
            Some(_) => {}
        }
    }
    for e in current {
        if find(backup, &e.service).is_none() {
            lines.push(format!("{lost} {}", e.service));
        }
    }
    if lines.is_empty() {
        ui::note("same entries as the current slate");
        return;
    }
    let d = ui::edim;
    eprintln!(
        "{}{back}{}{lost}{}{reverted}{}",
        d("restoring changes: "),
        d(" back, "),
        d(" lost, "),
        d(" reverted")
    );
    for l in lines {
        eprintln!("  {l}");
    }
}

fn is_csv(file: &Path) -> bool {
    file.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("csv"))
}

/// `.csv` for other managers, anything else is lossless json
fn cmd_export(name: Option<&str>, file: &Path) -> Result<()> {
    let s = unlock(name)?;
    let text = Zeroizing::new(if is_csv(file) {
        transfer::to_csv(&s.entries)
    } else {
        transfer::to_json(&s.entries)?
    });
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(file)
        .with_context(|| format!("cannot create {} (never overwritten)", file.display()))?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    ui::say(&format!(
        "exported {} entries to {}",
        s.entries.len(),
        tilde(file)
    ));
    ui::note("the export is unencrypted, delete it once you are done");
    Ok(())
}

/// same service (case-insensitive), username & secret
fn is_duplicate(have: &[Entry], e: &Entry) -> bool {
    have.iter().any(|h| {
        h.service.eq_ignore_ascii_case(&e.service)
            && h.username == e.username
            && h.secret() == e.secret()
    })
}

fn cmd_import(name: Option<&str>, file: &Path) -> Result<()> {
    let text = Zeroizing::new(
        fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?,
    );
    let (found, unreadable) = transfer::parse(&text)?;
    let mut s = unlock(name)?;

    let total = found.len();
    let mut fresh: Vec<Entry> = Vec::new();
    for e in found {
        if !is_duplicate(&s.entries, &e) && !is_duplicate(&fresh, &e) {
            fresh.push(e);
        }
    }
    let skipped = unreadable + total - fresh.len();
    if skipped > 0 {
        ui::note(&format!("{skipped} skipped (duplicates or unsupported)"));
    }
    let n = fresh.len();
    if n == 0 || !ui::confirm(&format!("import {n} entries?"))? {
        ui::say("nothing imported");
        return Ok(());
    }
    s.entries.extend(fresh);
    s.save()?;
    ui::say(&format!("imported {n} entries"));
    ui::note("delete the import file, it is unencrypted");
    Ok(())
}

fn cmd_lock(name: Option<&str>) -> Result<()> {
    match name {
        Some(n) => {
            let path = slate::resolve(Some(n))?;
            cache::clear(&path);
            ui::say(&format!("locked {}", tilde(&path)));
        }
        None => {
            let n = cache::clear_all();
            ui::say(&format!("locked {n} slate(s)"));
        }
    }
    Ok(())
}

/// `12 entries (9 login, 2 recovery, 1 ssh): github, proton, rpi, .. +9 more`
fn destroy_summary(entries: &[Entry]) -> String {
    use entry::EntryType::{Login, Recovery, Ssh};
    let kinds: Vec<String> = [Login, Recovery, Ssh]
        .into_iter()
        .filter_map(|k| {
            let n = entries.iter().filter(|e| e.kind == k).count();
            (n > 0).then(|| format!("{n} {k}"))
        })
        .collect();
    let names: Vec<&str> = sorted(entries, Sort::Alphabetical)
        .into_iter()
        .take(5)
        .map(|i| entries[i].service.as_str())
        .collect();
    let more = match entries.len() - names.len() {
        0 => String::new(),
        n => format!(", .. +{n} more"),
    };
    match entries.len() {
        0 => "no entries".into(),
        n => format!(
            "{n} entries ({}): {}{more}",
            kinds.join(", "),
            names.join(", ")
        ),
    }
}

fn cmd_destroy(name: Option<&str>) -> Result<()> {
    let path = slate::resolve(name)?;
    let pass = match cache::load(&path) {
        Some(p) => Zeroizing::new(p.expose_secret().to_owned()),
        None => ui::secret("master password")?,
    };
    match slate::peek(&path, &pass) {
        Ok((_, entries)) => ui::note(&format!("lost: {}", destroy_summary(&entries))),
        Err(_) => ui::note("lost: unknown, the master password does not open this slate"),
    }
    let files = slate::files(&path);
    ui::note(&format!("deleted with it ({} files):", files.len()));
    for f in &files {
        eprintln!("  {}", ui::edim(&tilde(f)));
    }
    let slate_name = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    ui::tip(&format!(
        "keep a copy first: `sheikah -S {} export {slate_name}.json`",
        name.unwrap_or(&slate_name)
    ));
    let typed = ui::prompt(&format!(
        "type '{slate_name}' to delete it for good, anything else keeps it"
    ))?;
    if typed != slate_name {
        ui::say(&format!("kept {}", tilde(&path)));
        return Ok(());
    }
    slate::remove(&files)?;
    cache::clear(&path);
    ui::say(&format!("destroyed {}", tilde(&path)));
    Ok(())
}

fn cmd_passwd(name: Option<&str>) -> Result<()> {
    let mut s = unlock(name)?;
    let path = s.path.clone();
    if s.config.ui.confirm_overwrite
        && !ui::confirm(&format!(
            "overwrite the master password for {}?",
            tilde(&path)
        ))?
    {
        ui::say("master password unchanged");
        return Ok(());
    }
    let new = ui::new_secret("new master password")?;
    let secret = secrecy::SecretString::from(new.as_str().to_owned());
    let ttl = s.config.session.cache;
    s.rekey(new)?;
    remember(&path, &secret, ttl);
    ui::say(&format!("master password changed for {}", tilde(&path)));
    Ok(())
}

fn cmd_config(name: Option<&str>) -> Result<()> {
    let mut s = unlock(name)?;
    let text = format!("{}\n{}", config::HELP, toml::to_string_pretty(&s.config)?);

    let tmp = runtime_dir().join(format!("sheikah-config-{}.toml", std::process::id()));
    {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&tmp)
            .with_context(|| format!("cannot create {}", tmp.display()))?;
        f.write_all(text.as_bytes())?;
    }

    let editor = var_os("VISUAL")
        .or_else(|| var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    let status = Command::new(&editor).arg(&tmp).status();
    let edited = fs::read_to_string(&tmp);
    let _ = fs::remove_file(&tmp);

    if !status.is_ok_and(|st| st.success()) {
        bail!(
            "editor {} failed, config unchanged",
            editor.to_string_lossy()
        );
    }
    let new: Config = toml::from_str(&edited?).context("invalid config, nothing saved")?;
    theme::resolve(&new.theme, ui::themes_dir().as_deref())
        .context("invalid theme, nothing saved")?;
    if new == s.config {
        ui::say("config unchanged");
        return Ok(());
    }
    s.config = new;
    s.save()?;
    ui::say("config saved");
    Ok(())
}

/// `sheikah ssh` relies on `SSH_ASKPASS_REQUIRE`, added in OpenSSH 8.4
fn ssh_row() -> [String; 3] {
    let Some(p) = clip::which("ssh") else {
        return ["ssh".into(), "-".into(), "missing".into()];
    };
    // `ssh -V` prints e.g. `OpenSSH_8.9p1 Ubuntu-3ubuntu0.18, OpenSSL ..` to stderr
    let v = Command::new(&p)
        .arg("-V")
        .output()
        .ok()
        .and_then(|o| openssh_version(&String::from_utf8_lossy(&o.stderr)));
    let status = match v {
        Some((maj, min)) if (maj, min) >= (8, 4) => "ok".into(),
        Some(_) => "needs 8.4+ for `ssh`".into(),
        None => "unknown version".into(),
    };
    let info = match v {
        Some((maj, min)) => format!("{maj}.{min} {}", p.display()),
        None => p.display().to_string(),
    };
    ["ssh".into(), info, status]
}

fn openssh_version(s: &str) -> Option<(u32, u32)> {
    let rest = s.split("OpenSSH_").nth(1)?;
    let (maj, rest) = rest.split_once('.')?;
    let min: String = rest.chars().take_while(char::is_ascii_digit).collect();
    Some((maj.parse().ok()?, min.parse().ok()?))
}

fn cmd_doctor(name: Option<&str>) -> Result<()> {
    let mut rows: Vec<[String; 3]> = Vec::new();

    let clips: Vec<&str> = clip::backends().into_iter().map(|b| b.0).collect();
    let found: Vec<&str> = clips.iter().copied().filter(|b| clip::on_path(b)).collect();
    let mut tools = vec!["fzf"];
    tools.extend(if found.is_empty() { clips } else { found });
    for t in tools {
        let row = match clip::which(t) {
            Some(p) => {
                let v = (t == "fzf")
                    .then(|| Command::new(t).arg("--version").output().ok())
                    .flatten()
                    .and_then(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .split_whitespace()
                            .next()
                            .map(String::from)
                    });
                let info = match v {
                    Some(v) => format!("{v} {}", p.display()),
                    None => p.display().to_string(),
                };
                [t.to_string(), info, "ok".into()]
            }
            None => [t.to_string(), "-".into(), "missing".into()],
        };
        rows.push(row);
    }
    rows.push(ssh_row());

    let mode = |p: &Path| fs::metadata(p).map(|m| m.permissions().mode() & 0o777).ok();
    let dir = slate::dir()?;
    rows.push(match mode(&dir) {
        Some(m) => [
            "~/.sheikah".into(),
            format!("{m:o}"),
            if m == 0o700 { "ok" } else { "should be 700" }.into(),
        ],
        None => ["~/.sheikah".into(), "-".into(), "missing".into()],
    });
    let slates = match name {
        Some(n) => slate::resolve(Some(n)).into_iter().collect(),
        None => slate::all().unwrap_or_default(),
    };
    for p in slates {
        let m = mode(&p).unwrap_or(0);
        rows.push([
            tilde(&p),
            format!("{m:o}"),
            if m == 0o600 { "ok" } else { "should be 600" }.into(),
        ]);
    }

    let w0 = rows.iter().map(|r| r[0].chars().count()).max().unwrap_or(0);
    let w1 = rows.iter().map(|r| r[1].chars().count()).max().unwrap_or(0);
    for [a, b, c] in rows {
        let status = if c == "ok" {
            ui::accent(&c)
        } else {
            ui::dim(&c)
        };
        println!("{a:<w0$}  {}  {status}", ui::dim(&format!("{b:<w1$}")));
    }
    Ok(())
}

/// indices of all entries in `by` order; ties (and never used entries) fall back to
/// service then username (case-insensitive)
pub(crate) fn sorted(entries: &[Entry], by: Sort) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..entries.len()).collect();
    idx.sort_by_key(|&i| {
        let e = &entries[i];
        let stamp = match by {
            Sort::Alphabetical => None,
            Sort::Modified => Some(e.modified.as_str()),
            Sort::Used => e.used.as_deref(),
        };
        (
            Reverse(stamp),
            e.service.to_lowercase(),
            e.username.as_deref().unwrap_or("").to_lowercase(),
        )
    });
    idx
}

/// case-insensitive substring match over every non-secret field
pub(crate) fn filter(entries: &[Entry], query: &str, by: Sort) -> Vec<usize> {
    let q = query.to_lowercase();
    sorted(entries, by)
        .into_iter()
        .filter(|&i| {
            entries[i]
                .haystack()
                .iter()
                .any(|f| f.to_lowercase().contains(&q))
        })
        .collect()
}

/// exact service name, then a unique substring match, otherwise fzf decides
pub(crate) fn find_ref(entries: &[Entry], r: &str, by: Sort) -> Result<usize> {
    find_ref_among(entries, &(0..entries.len()).collect::<Vec<_>>(), r, by)
}

/// like `find_ref`, but only among `allowed` indices (e.g. after a `--tag` filter)
pub(crate) fn find_ref_among(
    entries: &[Entry],
    allowed: &[usize],
    r: &str,
    by: Sort,
) -> Result<usize> {
    if entries.is_empty() || allowed.is_empty() {
        bail!("slate is empty (add one with `sheikah add`)");
    }
    let order: Vec<usize> = sorted(entries, by)
        .into_iter()
        .filter(|i| allowed.contains(i))
        .collect();
    if r.is_empty() {
        return ui::pick(entries, &order, "");
    }

    let exact: Vec<usize> = order
        .iter()
        .copied()
        .filter(|&i| entries[i].service.eq_ignore_ascii_case(r))
        .collect();
    let candidates = if exact.is_empty() {
        filter(entries, r, by)
            .into_iter()
            .filter(|i| allowed.contains(i))
            .collect()
    } else {
        exact
    };

    match candidates.as_slice() {
        [one] => Ok(*one),
        [] => ui::pick(entries, &order, r),
        many => ui::pick(entries, many, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(service: &str, user: Option<&str>, tags: &[&str]) -> Entry {
        let mut e = Entry::new(service.into(), "pw".into());
        e.username = user.map(Into::into);
        e.tags = tags.iter().map(|t| (*t).into()).collect();
        e
    }

    fn sample() -> Vec<Entry> {
        vec![
            entry("Proton", Some("me"), &["mail"]),
            entry("github", Some("v"), &["dev"]),
            entry("netflix", None, &["series"]),
            entry("github", Some("alt"), &["dev"]),
        ]
    }

    const ABC: Sort = Sort::Alphabetical;

    #[test]
    fn sorts_by_service_then_user() {
        assert_eq!(sorted(&sample(), ABC), vec![3, 1, 2, 0]);
    }

    #[test]
    fn sorts_newest_first_then_alphabetical() {
        let mut e = sample();
        for (x, ts) in e
            .iter_mut()
            .zip(["2026-01-01", "2026-03-01", "2026-02-01", "2026-03-01"])
        {
            x.modified = ts.into();
        }
        assert_eq!(sorted(&e, Sort::Modified), vec![3, 1, 2, 0]);

        e[2].used = Some("2026-05-01".into());
        e[0].used = Some("2026-04-01".into());
        assert_eq!(sorted(&e, Sort::Used), vec![2, 0, 3, 1]);
    }

    #[test]
    fn filter_matches_any_field_case_insensitive() {
        let e = sample();
        assert_eq!(filter(&e, "MAIL", ABC), vec![0]);
        assert_eq!(filter(&e, "dev", ABC), vec![3, 1]);
        assert_eq!(filter(&e, "pw", ABC), Vec::<usize>::new());
        assert_eq!(filter(&e, "", ABC).len(), 4);
    }

    #[test]
    fn parses_openssh_version() {
        let v = "OpenSSH_8.9p1 Ubuntu-3ubuntu0.18, OpenSSL 3.0.2 15 Mar 2022";
        assert_eq!(openssh_version(v), Some((8, 9)));
        assert_eq!(openssh_version("OpenSSH_10.0p2, LibreSSL"), Some((10, 0)));
        assert_eq!(openssh_version("OpenSSH_for_Windows_9.5p1"), None);
        assert_eq!(openssh_version("nope"), None);
    }

    #[test]
    fn export_format_from_extension() {
        assert!(is_csv(Path::new("out.CSV")));
        assert!(!is_csv(Path::new("out.json")));
        assert!(!is_csv(Path::new("out")));
    }

    #[test]
    fn destroy_summary_counts_and_names() {
        let mut e = sample();
        e[2].kind = entry::EntryType::Recovery;
        assert_eq!(
            destroy_summary(&e),
            "4 entries (3 login, 1 recovery): github, github, netflix, Proton"
        );
        e.extend((0..3).map(|i| entry(&format!("z{i}"), None, &[])));
        assert!(destroy_summary(&e).ends_with("Proton, z0, .. +2 more"));
        assert_eq!(destroy_summary(&[]), "no entries");
    }

    #[test]
    fn import_skips_duplicates() {
        let have = sample();
        assert!(is_duplicate(&have, &entry("GITHUB", Some("v"), &[])));
        assert!(!is_duplicate(&have, &entry("github", Some("new"), &[])));
        let mut changed = entry("github", Some("v"), &[]);
        changed.password = "other".into();
        assert!(!is_duplicate(&have, &changed));
    }

    #[test]
    fn find_ref_prefers_unique_matches() {
        let e = sample();
        assert_eq!(find_ref(&e, "proton", ABC).unwrap(), 0);
        assert_eq!(find_ref(&e, "flix", ABC).unwrap(), 2);
        assert!(find_ref(&[], "x", ABC).is_err());
    }
}
