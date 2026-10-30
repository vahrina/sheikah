use anyhow::{Context, Result, bail};
use rpassword::prompt_password;
use std::{
    env::var_os,
    io::{self, ErrorKind, IsTerminal, Write},
    process::{Command, Stdio},
    sync::{
        LazyLock, RwLock,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
};
use zeroize::Zeroizing;

use crate::{
    config::{self, Config},
    entry::Entry,
    slate,
    theme::{self, Style, Styles},
};

static OUT: AtomicBool = AtomicBool::new(false);
static ERR: AtomicBool = AtomicBool::new(false);
static TRUECOLOR: AtomicBool = AtomicBool::new(false);
static VERBOSE: AtomicBool = AtomicBool::new(true);
static STYLES: LazyLock<RwLock<Styles>> = LazyLock::new(|| RwLock::new(Styles::default()));

/// off for pipes, `NO_COLOR`, `TERM=dumb` and `-P`; 256 colors unless `COLORTERM` says truecolor
pub(crate) fn init_color(plain: bool) {
    let ok = !plain
        && var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && var_os("TERM").is_none_or(|t| t != "dumb");
    OUT.store(ok && io::stdout().is_terminal(), Relaxed);
    ERR.store(ok && io::stderr().is_terminal(), Relaxed);
    TRUECOLOR.store(
        var_os("COLORTERM").is_some_and(|c| c == "truecolor" || c == "24bit"),
        Relaxed,
    );
}

/// swaps in the slate's theme; a broken one only warns, the default stays
pub(crate) fn set_theme(t: &config::Theme) {
    match theme::resolve(t, themes_dir().as_deref()) {
        Ok(s) => *STYLES.write().unwrap_or_else(|e| e.into_inner()) = s,
        Err(e) => note(&format!("theme ignored: {e:#}")),
    }
}

pub(crate) fn themes_dir() -> Option<std::path::PathBuf> {
    slate::dir().ok().map(|d| d.join("themes"))
}

fn paint(on: &AtomicBool, role: fn(&Styles) -> Style, s: &str) -> String {
    if !on.load(Relaxed) {
        return s.to_string();
    }
    let st = role(&STYLES.read().unwrap_or_else(|e| e.into_inner()));
    let mut codes = Vec::new();
    if st.bold {
        codes.push("1".to_string());
    }
    if st.dim {
        codes.push("2".to_string());
    }
    if let Some(c) = st.fg {
        codes.push(c.sgr(TRUECOLOR.load(Relaxed)));
    }
    if codes.is_empty() {
        s.to_string()
    } else {
        format!("\x1b[{}m{s}\x1b[0m", codes.join(";"))
    }
}

pub(crate) fn dim(s: &str) -> String {
    paint(&OUT, |t| t.muted, s)
}
pub(crate) fn accent(s: &str) -> String {
    paint(&OUT, |t| t.accent, s)
}
fn header(s: &str) -> String {
    paint(&OUT, |t| t.header, s)
}
fn border(s: &str) -> String {
    paint(&OUT, |t| t.border, s)
}
fn text(s: &str) -> String {
    paint(&OUT, |t| t.text, s)
}
fn key(s: &str) -> String {
    paint(&OUT, |t| t.key, s)
}
fn secret_text(s: &str) -> String {
    paint(&OUT, |t| t.secret, s)
}
pub(crate) fn edim(s: &str) -> String {
    paint(&ERR, |t| t.muted, s)
}
pub(crate) fn eerror(s: &str) -> String {
    paint(&ERR, |t| t.error, s)
}

/// a status line on stdout, e.g. `~ added 'github'`
pub(crate) fn say(msg: &str) {
    println!("{} {msg}", accent("~"));
}

/// a hint on stderr
pub(crate) fn note(msg: &str) {
    eprintln!("{}", edim(msg));
}

/// `ui.verbose` from the slate config; usage tips & extra detail only show when on
pub(crate) fn set_verbose(on: bool) {
    VERBOSE.store(on, Relaxed);
}

/// a usage tip on stderr, dropped when not verbose
pub(crate) fn tip(msg: &str) {
    if VERBOSE.load(Relaxed) {
        note(msg);
    }
}

/// dimmed extra detail for a stderr prompt, empty when not verbose
pub(crate) fn hint(s: &str) -> String {
    if VERBOSE.load(Relaxed) {
        edim(s)
    } else {
        String::new()
    }
}

/// dimmed extra detail for a stdout line, empty when not verbose
pub(crate) fn out_hint(s: &str) -> String {
    if VERBOSE.load(Relaxed) {
        dim(s)
    } else {
        String::new()
    }
}

fn label(text: &str) -> String {
    format!("{text}{} ", edim(":"))
}

/// prompts go to stderr so stdout stays clean for piping (`sheikah -s x -p | tee`)
pub(crate) fn prompt(text: &str) -> Result<String> {
    eprint!("{}", label(text));
    io::stderr().flush()?;
    let mut inp = String::new();
    match io::stdin().read_line(&mut inp) {
        Ok(0) => bail!("unexpected end of input"),
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::Interrupted => crate::signal::interrupted(),
        Err(e) => return Err(e.into()),
    }
    Ok(inp.trim().to_string())
}

pub(crate) fn prompt_opt(text: &str) -> Result<Option<String>> {
    let v = prompt(text)?;
    Ok((!v.is_empty()).then_some(v))
}

/// one `> ` line per item until an empty line; rejects an empty list
pub(crate) fn prompt_lines(text: &str) -> Result<Vec<String>> {
    note(&format!("{text}{}", hint(", one per line, empty line to finish")));
    let mut items = Vec::new();
    loop {
        eprint!("{} ", edim(">"));
        io::stderr().flush()?;
        let mut inp = String::new();
        match io::stdin().read_line(&mut inp) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => crate::signal::interrupted(),
            Err(e) => return Err(e.into()),
        }
        match inp.trim() {
            "" => break,
            v => items.push(v.to_string()),
        }
    }
    if items.is_empty() {
        bail!("{text} must not be empty");
    }
    Ok(items)
}

pub(crate) fn confirm(text: &str) -> Result<bool> {
    let v = prompt(&format!("{text} {}", edim("[y/N]")))?;
    Ok(matches!(v.as_str(), "y" | "Y"))
}
pub(crate) fn secret(text: &str) -> Result<Zeroizing<String>> {
    match prompt_password(label(text)) {
        Ok(p) => Ok(Zeroizing::new(p)),
        Err(e) if e.kind() == ErrorKind::Interrupted => crate::signal::interrupted(),
        Err(e) => Err(e.into()),
    }
}

/// asks twice, rejects empty or mismatching input
pub(crate) fn new_secret(text: &str) -> Result<Zeroizing<String>> {
    let a = secret(text)?;
    if a.is_empty() {
        bail!("{text} must not be empty");
    }
    let b = secret(&format!("repeat {text}"))?;
    if *a != *b {
        bail!("{text}s do not match");
    }
    Ok(a)
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\n'], " ")
}

fn or_dash(f: &Option<String>) -> String {
    f.as_deref().map(clean).unwrap_or_else(|| "-".into())
}

pub(crate) const FZF_MISSING: &str = "fzf required: https://github.com/junegunn/fzf#installation";

/// fuzzy-pick one entry with fzf; returns its index into `entries`
pub(crate) fn pick(entries: &[Entry], idx: &[usize], query: &str) -> Result<usize> {
    let rows: Vec<[String; 3]> = idx
        .iter()
        .map(|&i| {
            let e = &entries[i];
            [
                clean(&e.service),
                or_dash(&e.username),
                or_dash(&e.category),
            ]
        })
        .collect();
    let w = widths(&rows);
    let lines: Vec<(usize, String)> = idx
        .iter()
        .zip(&rows)
        .map(|(&i, r)| {
            let line = format!("{:<a$}  {:<b$}  {}", r[0], r[1], r[2], a = w[0], b = w[1]);
            (i, line)
        })
        .collect();
    choose(&lines, query)
}

/// fuzzy-pick one of `(id, line)` with fzf; returns the chosen id
pub(crate) fn choose(lines: &[(usize, String)], query: &str) -> Result<usize> {
    let color = if ERR.load(Relaxed) {
        STYLES.read().unwrap_or_else(|e| e.into_inner()).fzf()
    } else {
        "--color=bw".to_string()
    };
    let mut child = Command::new("fzf")
        .args([
            "--delimiter=\t",
            "--with-nth=2..",
            "--tabstop=2",
            "--height=40%",
            "--layout=reverse",
            "--info=hidden",
            "--no-bold",
            "--prompt=> ",
            "--pointer= ",
            "--marker= ",
            "--select-1",
            "--exit-0",
            &color,
            &format!("--query={query}"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow::anyhow!(FZF_MISSING),
            _ => anyhow::Error::new(e).context("cannot run fzf"),
        })?;

    {
        let mut stdin = child.stdin.take().context("fzf stdin")?;
        for (i, line) in lines {
            let _ = writeln!(stdin, "{i}\t{line}");
        }
    }

    let out = child.wait_with_output()?;
    match out.status.code() {
        Some(0) => {}
        Some(1) => bail!("no match"),
        Some(130) => crate::signal::interrupted(),
        _ => bail!("fzf failed"),
    }
    String::from_utf8_lossy(&out.stdout)
        .split('\t')
        .next()
        .and_then(|i| i.trim().parse().ok())
        .context("unexpected fzf output")
}

fn widths<const N: usize>(rows: &[[String; N]]) -> [usize; N] {
    let mut w = [0; N];
    for r in rows {
        for (c, cell) in r.iter().enumerate() {
            w[c] = w[c].max(cell.chars().count());
        }
    }
    w
}

fn pad(s: &str, w: usize) -> String {
    format!("{s}{}", " ".repeat(w.saturating_sub(s.chars().count())))
}

fn shown(v: Option<&str>, hide: bool, glyph: &str) -> String {
    match v {
        None => "-".into(),
        Some(_) if hide => glyph.into(),
        Some(v) => clean(v),
    }
}

/// space-aligned columns, or a box in the configured style
pub(crate) fn table(entries: &[Entry], idx: &[usize], cfg: &Config, boxed: bool) {
    if idx.is_empty() {
        note("no entries");
        return;
    }
    let c = &cfg.censor;
    let glyph = c.glyph.as_str();
    let index = cfg.table.index;
    let mut head: Vec<String> = ["service", "username", "email", "category"]
        .map(String::from)
        .to_vec();
    if index {
        head.insert(0, "#".into());
    }
    let rows: Vec<Vec<String>> = idx
        .iter()
        .enumerate()
        .map(|(n, &i)| {
            let e = &entries[i];
            let mut r = vec![
                shown(Some(&e.service), c.service, glyph),
                shown(e.username.as_deref(), c.username, glyph),
                shown(e.email.as_deref(), c.email, glyph),
                shown(e.category.as_deref(), c.category, glyph),
            ];
            if index {
                r.insert(0, (n + 1).to_string());
            }
            r
        })
        .collect();
    let cols = head.len();
    let mut w: Vec<usize> = head.iter().map(|h| h.chars().count()).collect();
    for r in &rows {
        for (c, cell) in r.iter().enumerate() {
            w[c] = w[c].max(cell.chars().count());
        }
    }

    let boxed = boxed || cfg.table.boxed;
    let g = cfg.table.style.glyphs();
    let cell = |s: &str, c: usize, head: bool| {
        let p = pad(s, w[c]);
        match (head, s) {
            (true, _) => header(&p),
            (false, s) if s == "-" || s == glyph => dim(&p),
            _ => text(&p),
        }
    };
    let line = |r: &[String], head: bool| -> String {
        let cells: Vec<String> = (0..cols).map(|c| cell(&r[c], c, head)).collect();
        if boxed {
            let bar = border(&g.v.to_string());
            format!("{bar} {} {bar}", cells.join(&format!(" {bar} ")))
        } else {
            cells.join("  ").trim_end().to_string()
        }
    };
    let rule = |row: usize| {
        let [l, j, r] = g.rows[row];
        let segs: Vec<String> = w.iter().map(|&n| g.h.to_string().repeat(n + 2)).collect();
        border(&format!("{l}{}{r}", segs.join(&j.to_string())))
    };

    if boxed {
        println!("{}", rule(0));
    }
    println!("{}", line(&head, true));
    if boxed {
        println!("{}", rule(1));
    }
    for r in &rows {
        println!("{}", line(r, false));
    }
    if boxed {
        println!("{}", rule(2));
    }
}

/// every field, censored per config; `only` prints just those fields, uncensored
pub(crate) fn detail(e: &Entry, cfg: &Config, only: &[&str]) {
    let c = &cfg.censor;
    let secret = e.secret();
    let (secret_key, secret_val) = if e.is_recovery() {
        ("recovery", None)
    } else {
        ("password", Some(secret.as_str()))
    };
    let fields = [
        ("service", Some(e.service.as_str()), c.service),
        ("username", e.username.as_deref(), c.username),
        ("email", e.email.as_deref(), c.email),
        (secret_key, secret_val, c.password),
        ("url", e.url.as_deref(), c.url),
        ("category", e.category.as_deref(), c.category),
        ("notes", e.notes.as_deref(), c.notes),
        ("created", Some(e.created.as_str()), false),
        ("modified", Some(e.modified.as_str()), false),
    ];

    // `-p` asks for the secret, whichever form it has
    if let [one] = only {
        if *one == "password" {
            println!("{secret}");
        } else if let Some((_, v, _)) = fields.iter().find(|f| f.0 == *one) {
            println!("{}", v.unwrap_or(""));
        }
        return;
    }
    for (k, v, hide) in fields {
        let wanted =
            only.is_empty() || only.contains(&k) || (k == "recovery" && only.contains(&"password"));
        if !wanted {
            continue;
        }
        let hide = hide && only.is_empty();
        if k == "recovery" {
            if hide {
                println!("{} {}", key(&pad(k, 9)), dim(&c.glyph));
            } else {
                for (n, code) in e.recovery.iter().enumerate() {
                    let label = if n == 0 { k } else { "" };
                    println!("{} {} {}", key(&pad(label, 9)), dim("•"), secret_text(code));
                }
            }
            continue;
        }
        let Some(v) = v else { continue };
        let v = match (hide, k) {
            (true, _) => dim(&c.glyph),
            (false, "password") => secret_text(v),
            _ => text(v),
        };
        println!("{} {v}", key(&pad(k, 9)));
    }
}
