use anyhow::{Context, Result, bail};
use rpassword::prompt_password;
use std::{
    env::var_os,
    io::{self, ErrorKind, IsTerminal, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};
use zeroize::Zeroizing;

use crate::{config::Config, entry::Entry};

static OUT: AtomicBool = AtomicBool::new(false);
static ERR: AtomicBool = AtomicBool::new(false);

/// monochrome only: dim, bold, default. off for pipes, `NO_COLOR`, `TERM=dumb` and `-P`
pub(crate) fn init_color(plain: bool) {
    let ok = !plain
        && var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && var_os("TERM").is_none_or(|t| t != "dumb");
    OUT.store(ok && io::stdout().is_terminal(), Relaxed);
    ERR.store(ok && io::stderr().is_terminal(), Relaxed);
}

fn paint(on: &AtomicBool, code: &str, s: &str) -> String {
    if on.load(Relaxed) {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub(crate) fn dim(s: &str) -> String {
    paint(&OUT, "2", s)
}
pub(crate) fn bold(s: &str) -> String {
    paint(&OUT, "1", s)
}
pub(crate) fn edim(s: &str) -> String {
    paint(&ERR, "2", s)
}
pub(crate) fn ebold(s: &str) -> String {
    paint(&ERR, "1", s)
}

/// a status line on stdout, e.g. `~ added 'github'`
pub(crate) fn say(msg: &str) {
    println!("{} {msg}", dim("~"));
}

/// a hint on stderr
pub(crate) fn note(msg: &str) {
    eprintln!("{}", edim(msg));
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
        "--color=fg:7,bg:-1,hl:15,fg+:15,bg+:236,hl+:15,info:8,prompt:8,pointer:15,marker:8,query:15"
    } else {
        "--color=bw"
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
            color,
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

pub(crate) const MASK: &str = "•••";

fn shown(v: Option<&str>, hide: bool) -> String {
    match v {
        None => "-".into(),
        Some(_) if hide => MASK.into(),
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
    let head = ["service", "username", "email", "category"].map(String::from);
    let rows: Vec<[String; 4]> = idx
        .iter()
        .map(|&i| {
            let e = &entries[i];
            [
                shown(Some(&e.service), c.service),
                shown(e.username.as_deref(), c.username),
                shown(e.email.as_deref(), c.email),
                shown(e.category.as_deref(), c.category),
            ]
        })
        .collect();
    let mut w = widths(&rows);
    for (c, h) in head.iter().enumerate() {
        w[c] = w[c].max(h.len());
    }

    let boxed = boxed || cfg.table.boxed;
    let g = cfg.table.style.glyphs();
    let cell = |s: &str, c: usize, header: bool| {
        let p = pad(s, w[c]);
        match (header, s) {
            (true, _) => bold(&p),
            (false, "-" | MASK) => dim(&p),
            _ => p,
        }
    };
    let line = |r: &[String; 4], header: bool| -> String {
        let cells: Vec<String> = (0..4).map(|c| cell(&r[c], c, header)).collect();
        if boxed {
            let bar = dim(&g.v.to_string());
            format!("{bar} {} {bar}", cells.join(&format!(" {bar} ")))
        } else {
            cells.join("  ").trim_end().to_string()
        }
    };
    let rule = |row: usize| {
        let [l, j, r] = g.rows[row];
        let segs: Vec<String> = w.iter().map(|&n| g.h.to_string().repeat(n + 2)).collect();
        dim(&format!("{l}{}{r}", segs.join(&j.to_string())))
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
    let fields = [
        ("service", Some(e.service.as_str()), c.service),
        ("username", e.username.as_deref(), c.username),
        ("email", e.email.as_deref(), c.email),
        ("password", Some(e.password.as_str()), c.password),
        ("url", e.url.as_deref(), c.url),
        ("category", e.category.as_deref(), c.category),
        ("notes", e.notes.as_deref(), c.notes),
        ("created", Some(e.created.as_str()), false),
        ("modified", Some(e.modified.as_str()), false),
    ];

    if let [one] = only {
        if let Some((_, v, _)) = fields.iter().find(|f| f.0 == *one) {
            println!("{}", v.unwrap_or(""));
        }
        return;
    }
    for (k, v, hide) in fields {
        let Some(v) = v else { continue };
        if !only.is_empty() && !only.contains(&k) {
            continue;
        }
        let v = match (hide && only.is_empty(), k) {
            (true, _) => dim(MASK),
            (false, "password") => bold(v),
            _ => v.to_string(),
        };
        println!("{} {v}", dim(&pad(k, 9)));
    }
}
