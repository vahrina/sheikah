//! `sheikah ssh SSH_ARGS.. -- REF`: ssh reads passphrases from the tty, never stdin, so we
//! run it with `SSH_ASKPASS` pointing back at ourselves and hand the secret over a private socket

use anyhow::{Context, Result};
use std::{
    env::{args_os, current_exe, var_os},
    fs::{self, DirBuilder, File},
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, symlink},
        net::{UnixListener, UnixStream},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    process::{Command, exit},
    thread,
};
use zeroize::Zeroizing;

use crate::{commands::runtime_dir, signal};

const DIR_ENV: &str = "SHEIKAH_ASKPASS";
const LINK: &str = "askpass";
const SOCK: &str = "sock";

/// removes the private dir (symlink + socket) however ssh ends
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// runs ssh with `args`; the first passphrase/password prompt gets `secret`, the rest go to
/// the tty. returns ssh's exit code
pub(crate) fn ssh(secret: Zeroizing<String>, args: &[String]) -> Result<i32> {
    let dir = runtime_dir().join(format!("sheikah-askpass-{}", std::process::id()));
    DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .with_context(|| format!("cannot create {}", dir.display()))?;
    let _cleanup = Cleanup(dir.clone());

    let link = dir.join(LINK);
    symlink(current_exe()?, &link).context("cannot link askpass helper")?;
    let sock = dir.join(SOCK);
    let listener = UnixListener::bind(&sock).context("cannot open askpass socket")?;

    // served once: a wrong secret would only be rejected again
    thread::spawn(move || {
        if let Ok((mut s, _)) = listener.accept() {
            let _ = fs::remove_file(&sock);
            let _ = s.write_all(secret.as_bytes());
        }
    });

    let _child = signal::Child::enter();
    let status = Command::new("ssh")
        .args(args)
        .env("SSH_ASKPASS", &link)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env(DIR_ENV, &dir)
        .status()
        .context("cannot run ssh")?;
    Ok(status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)))
}

/// our private dir, when ssh runs us through the askpass symlink
pub(crate) fn invoked() -> Option<PathBuf> {
    let dir = PathBuf::from(var_os(DIR_ENV)?);
    let argv0 = args_os().next()?;
    (Path::new(&argv0) == dir.join(LINK)).then_some(dir)
}

/// answers one ssh prompt on stdout; exit 0 means yes/ok to ssh
pub(crate) fn respond(dir: &Path) -> ! {
    let prompt = args_os()
        .nth(1)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    match answer(dir, &prompt) {
        Ok(ok) => exit(if ok { 0 } else { 1 }),
        Err(e) => {
            eprintln!("sheikah askpass: {e:#}");
            exit(1)
        }
    }
}

fn answer(dir: &Path, prompt: &str) -> Result<bool> {
    let confirm = var_os("SSH_ASKPASS_PROMPT").is_some_and(|v| v == "confirm");
    let p = prompt.to_lowercase();
    if !confirm && (p.contains("passphrase") || p.contains("password")) {
        let secret = match UnixStream::connect(dir.join(SOCK)) {
            Ok(mut s) => {
                let mut buf = Zeroizing::new(String::new());
                s.read_to_string(&mut buf)?;
                buf
            }
            Err(_) => Zeroizing::new(rpassword::prompt_password(prompt)?),
        };
        println!("{}", *secret);
        return Ok(true);
    }

    // host key checks and other questions stay with the human
    let mut tty = File::options().read(true).write(true).open("/dev/tty")?;
    write!(tty, "{prompt}")?;
    tty.flush()?;
    let mut line = String::new();
    BufReader::new(tty).read_line(&mut line)?;
    let line = line.trim_end_matches(['\r', '\n']);
    if confirm {
        return Ok(matches!(line, "y" | "Y" | "yes"));
    }
    println!("{line}");
    Ok(true)
}
