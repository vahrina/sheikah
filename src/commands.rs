use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use std::{
    cmp::Reverse,
    env::var_os,
    fs,
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};
use zeroize::Zeroizing;

use crate::{
    args::{AddArgs, Cli, Commands, CompletionAction, ShowArgs},
    askpass, clip,
    config::{self, Config, Sort},
    entry::{self, Entry},
    slate::{self, Slate},
    theme, ui,
};

pub(crate) fn run(cli: Cli) -> Result<()> {
    // `$SHEIKAH_VAULT` predates the slate rename
    let env_vault = var_os("SHEIKAH_VAULT").map(|v| v.to_string_lossy().into_owned());
    let s = cli.slate.as_deref().or(env_vault.as_deref());

    let needs_fzf = !matches!(
        (&cli.command, &cli.reference),
        (
            Some(Commands::Doctor | Commands::Help { .. } | Commands::Completions { .. }),
            _
        ) | (None, None)
    );
    if needs_fzf && !clip::on_path("fzf") {
        bail!(ui::FZF_MISSING);
    }

    match cli.command {
        Some(Commands::Add(a)) => cmd_add(s, a),
        Some(Commands::Edit { reference }) => cmd_edit(s, reference.as_deref().unwrap_or("")),
        Some(Commands::Delete { reference }) => cmd_delete(s, reference.as_deref().unwrap_or("")),
        Some(Commands::Config { slate }) => cmd_config(slate.as_deref().or(s)),
        Some(Commands::Init) => cmd_init(s),
        Some(Commands::Passwd) => cmd_passwd(s),
        Some(Commands::Doctor) => cmd_doctor(s),
        Some(Commands::Backup) => cmd_backup(s),
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
        Some(Commands::List { boxed }) => cmd_list(s, boxed),
        Some(Commands::Find { query }) => cmd_find(s, query.as_deref().unwrap_or("")),
        Some(Commands::Help { command }) => cmd_help(command.as_deref()),
        Some(Commands::Completions { action, dir }) => cmd_completions(action, dir.as_deref()),
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

fn cmd_completions(action: CompletionAction, dir: Option<&str>) -> Result<()> {
    match action {
        CompletionAction::Zsh => completions(clap_complete::Shell::Zsh, &mut io::stdout()),
        CompletionAction::Update => {
            let dir = dir.map(PathBuf::from).unwrap_or_else(|| {
                var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                    .unwrap_or_else(|| PathBuf::from(".config"))
                    .join("zsh/completions")
            });
            fs::create_dir_all(&dir)?;
            let path = dir.join("_sheikah");
            let mut file = fs::File::create(&path)?;
            completions(clap_complete::Shell::Zsh, &mut file);
            ui::say(&format!("updated {}", tilde(&path)));
        }
    }
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

fn unlock(name: Option<&str>) -> Result<Slate> {
    let path = slate::resolve(name)?;
    let pass = ui::secret("master password")?;
    let s = Slate::load(path, pass)?;
    ui::set_theme(&s.config.theme);
    ui::set_verbose(s.config.ui.verbose);
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

fn cmd_add(name: Option<&str>, a: AddArgs) -> Result<()> {
    let mut s = unlock(name)?;
    let interactive = a.service.is_none();

    let service = match a.service {
        Some(x) => x,
        None => ui::prompt("service")?,
    };
    if service.is_empty() {
        bail!("service must not be empty");
    }
    let field = |val: Option<String>, label: &str| -> Result<Option<String>> {
        match val {
            Some(x) => Ok(Some(x)),
            None if interactive => ui::prompt_opt(&format!("{label}{}", ui::hint(" (optional)"))),
            None => Ok(None),
        }
    };
    let username = field(a.user, "username")?;
    let email = field(a.email, "email")?;
    let url = field(a.domain, "url")?;
    let category = field(a.category, "category")?;
    let notes = field(a.notes, "notes")?;

    let mut e = if a.recovery {
        let mut e = Entry::new(service, String::new());
        e.recovery = ui::prompt_lines("recovery codes")?;
        e
    } else {
        Entry::new(service, ui::new_secret("password")?.to_string())
    };
    e.username = username;
    e.email = email;
    e.url = url;
    e.category = category;
    e.notes = notes;

    let svc = e.service.clone();
    s.entries.push(e);
    s.save()?;
    ui::say(&format!("added {svc}"));
    Ok(())
}

fn cmd_list(name: Option<&str>, boxed: bool) -> Result<()> {
    let s = unlock(name)?;
    let idx = sorted(&s.entries, s.config.table.sort);
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

fn cmd_find(name: Option<&str>, q: &str) -> Result<()> {
    let mut s = unlock(name)?;
    if s.entries.is_empty() {
        bail!("slate is empty (add one with `sheikah add`)");
    }
    let i = ui::pick(&s.entries, &sorted(&s.entries, s.config.table.sort), q)?;
    ui::detail(&s.entries[i], &s.config, &[]);
    mark_used(&mut s, i);
    Ok(())
}

fn cmd_show(name: Option<&str>, a: ShowArgs) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(
        &s.entries,
        a.reference.as_deref().unwrap_or(""),
        s.config.table.sort,
    )?;
    let only: Vec<&str> = [
        (a.service, "service"),
        (a.username, "username"),
        (a.email, "email"),
        (a.password, "password"),
        (a.url, "url"),
        (a.category, "category"),
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

fn cmd_edit(name: Option<&str>, r: &str) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r, s.config.table.sort)?;
    let e = &mut s.entries[i];
    ui::tip("'enter' to keep | '-' to clear");

    if let Some(x) = ui::prompt_opt(&format!(
        "service {}",
        ui::edim(&format!("[{}]", e.service))
    ))? && x != "-"
    {
        e.service = x;
    }
    edit_field("username", &mut e.username)?;
    edit_field("email", &mut e.email)?;
    edit_field("url", &mut e.url)?;
    edit_field("category", &mut e.category)?;
    edit_field("notes", &mut e.notes)?;
    if e.is_recovery() {
        if ui::confirm("replace recovery codes?")? {
            e.recovery = ui::prompt_lines("recovery codes")?;
        }
    } else if ui::confirm("change password?")? {
        e.password = ui::new_secret("password")?.to_string();
    }
    e.touch();

    let svc = e.service.clone();
    s.save()?;
    ui::say(&format!("updated {svc}"));
    Ok(())
}

fn edit_field(label: &str, cur: &mut Option<String>) -> Result<()> {
    let shown = ui::edim(&format!("[{}]", cur.as_deref().unwrap_or("-")));
    match ui::prompt_opt(&format!("{label} {shown}"))?.as_deref() {
        None => {}
        Some("-") => *cur = None,
        Some(x) => *cur = Some(x.to_string()),
    }
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
    let lines: Vec<(usize, String)> = baks
        .iter()
        .enumerate()
        .map(|(i, (p, t))| {
            let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
            (i, format!("{}  {}", entry::rfc3339(secs), tilde(p)))
        })
        .collect();
    let bak = &baks[ui::choose(&lines, "")?].0;

    if !ui::confirm(&format!(
        "restore {}? the current slate becomes a backup",
        tilde(bak)
    ))? {
        ui::say("nothing restored");
        return Ok(());
    }
    ui::note("the backup keeps the master password it was saved with");
    let pass = ui::secret("master password")?;
    slate::restore(&path, bak, &pass)?;
    ui::say(&format!("restored {}", tilde(bak)));
    Ok(())
}

fn cmd_passwd(name: Option<&str>) -> Result<()> {
    let mut s = unlock(name)?;
    let new = ui::new_secret("new master password")?;
    s.rekey(new)?;
    ui::say(&format!("master password changed for {}", tilde(&s.path)));
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
    if entries.is_empty() {
        bail!("slate is empty (add one with `sheikah add`)");
    }
    if r.is_empty() {
        return ui::pick(entries, &sorted(entries, by), "");
    }

    let exact: Vec<usize> = sorted(entries, by)
        .into_iter()
        .filter(|&i| entries[i].service.eq_ignore_ascii_case(r))
        .collect();
    let candidates = if exact.is_empty() {
        filter(entries, r, by)
    } else {
        exact
    };

    match candidates.as_slice() {
        [one] => Ok(*one),
        [] => ui::pick(entries, &sorted(entries, by), r),
        many => ui::pick(entries, many, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(service: &str, user: Option<&str>, category: Option<&str>) -> Entry {
        let mut e = Entry::new(service.into(), "pw".into());
        e.username = user.map(Into::into);
        e.category = category.map(Into::into);
        e
    }

    fn sample() -> Vec<Entry> {
        vec![
            entry("Proton", Some("me"), Some("mail")),
            entry("github", Some("v"), Some("dev")),
            entry("netflix", None, Some("series")),
            entry("github", Some("alt"), Some("dev")),
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
    fn find_ref_prefers_unique_matches() {
        let e = sample();
        assert_eq!(find_ref(&e, "proton", ABC).unwrap(), 0);
        assert_eq!(find_ref(&e, "flix", ABC).unwrap(), 2);
        assert!(find_ref(&[], "x", ABC).is_err());
    }
}
