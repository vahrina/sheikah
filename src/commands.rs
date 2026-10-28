use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use std::{
    env::var_os,
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::UNIX_EPOCH,
};

use crate::{
    args::{AddArgs, Cli, Commands, ShowArgs},
    clip,
    config::{self, Config},
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
        (Some(Commands::Doctor | Commands::Help { .. }), _) | (None, None)
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
        Some(Commands::List { boxed }) => cmd_list(s, boxed),
        Some(Commands::Find { query }) => cmd_find(s, query.as_deref().unwrap_or("")),
        Some(Commands::Help { command }) => cmd_help(command.as_deref()),
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

/// `~/.sheikah/eye` instead of `/home/you/.sheikah/eye`
fn tilde(p: &Path) -> String {
    match var_os("HOME").map(PathBuf::from) {
        Some(h) if p.starts_with(&h) => {
            format!("~/{}", p.strip_prefix(&h).unwrap_or(p).display())
        }
        _ => p.display().to_string(),
    }
}

fn unlock(name: Option<&str>) -> Result<Slate> {
    let path = slate::resolve(name)?;
    let pass = ui::secret("master password")?;
    let s = Slate::load(path, pass)?;
    ui::set_theme(&s.config.theme);
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
            None if interactive => ui::prompt_opt(&format!("{label} {}", ui::edim("(optional)"))),
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
    ui::table(&s.entries, &sorted(&s.entries), &s.config, boxed);
    Ok(())
}

fn cmd_find(name: Option<&str>, q: &str) -> Result<()> {
    let s = unlock(name)?;
    if s.entries.is_empty() {
        bail!("slate is empty (add one with `sheikah add`)");
    }
    let i = ui::pick(&s.entries, &sorted(&s.entries), q)?;
    ui::detail(&s.entries[i], &s.config, &[]);
    Ok(())
}

fn cmd_show(name: Option<&str>, a: ShowArgs) -> Result<()> {
    let s = unlock(name)?;
    let i = find_ref(&s.entries, a.reference.as_deref().unwrap_or(""))?;
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
    Ok(())
}

fn cmd_copy(name: Option<&str>, r: &str) -> Result<()> {
    let s = unlock(name)?;
    let e = &s.entries[find_ref(&s.entries, r)?];
    let via = clip::copy(&e.secret())?;
    ui::say(&format!(
        "copied {} {}",
        e.service,
        ui::dim(&format!("via {via}, clears in {}s", clip::CLEAR_SECS))
    ));
    Ok(())
}

fn cmd_edit(name: Option<&str>, r: &str) -> Result<()> {
    let mut s = unlock(name)?;
    let i = find_ref(&s.entries, r)?;
    let e = &mut s.entries[i];
    ui::note("'enter' to keep | '-' to clear");

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
    let i = find_ref(&s.entries, r)?;
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

    // a tmpfs keeps the decrypted config off disk; never ~/.sheikah, which may be `chattr +a`
    let dir = var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("/dev/shm"));
    let tmp = dir.join(format!("sheikah-config-{}.toml", std::process::id()));
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

/// indices of all entries, ordered by service then username (case-insensitive)
pub(crate) fn sorted(entries: &[Entry]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..entries.len()).collect();
    idx.sort_by_key(|&i| {
        let e = &entries[i];
        (
            e.service.to_lowercase(),
            e.username.as_deref().unwrap_or("").to_lowercase(),
        )
    });
    idx
}

/// case-insensitive substring match over every non-secret field
pub(crate) fn filter(entries: &[Entry], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    sorted(entries)
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
pub(crate) fn find_ref(entries: &[Entry], r: &str) -> Result<usize> {
    if entries.is_empty() {
        bail!("slate is empty (add one with `sheikah add`)");
    }
    if r.is_empty() {
        return ui::pick(entries, &sorted(entries), "");
    }

    let exact: Vec<usize> = sorted(entries)
        .into_iter()
        .filter(|&i| entries[i].service.eq_ignore_ascii_case(r))
        .collect();
    let candidates = if exact.is_empty() {
        filter(entries, r)
    } else {
        exact
    };

    match candidates.as_slice() {
        [one] => Ok(*one),
        [] => ui::pick(entries, &sorted(entries), r),
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

    #[test]
    fn sorts_by_service_then_user() {
        assert_eq!(sorted(&sample()), vec![3, 1, 2, 0]);
    }

    #[test]
    fn filter_matches_any_field_case_insensitive() {
        let e = sample();
        assert_eq!(filter(&e, "MAIL"), vec![0]);
        assert_eq!(filter(&e, "dev"), vec![3, 1]);
        assert_eq!(filter(&e, "pw"), Vec::<usize>::new());
        assert_eq!(filter(&e, "").len(), 4);
    }

    #[test]
    fn find_ref_prefers_unique_matches() {
        let e = sample();
        assert_eq!(find_ref(&e, "proton").unwrap(), 0);
        assert_eq!(find_ref(&e, "flix").unwrap(), 2);
        assert!(find_ref(&[], "x").is_err());
    }
}
