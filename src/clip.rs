use anyhow::{Result, bail};
use std::{
    env::{split_paths, var_os},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(crate) const CLEAR_SECS: u32 = 45;

/// (binary, copy args, shell snippet that empties the clipboard)
type Backend = (&'static str, &'static [&'static str], &'static str);

const WAYLAND: Backend = ("wl-copy", &[], "wl-copy --clear");
const XCLIP: Backend = (
    "xclip",
    &["-selection", "clipboard"],
    "printf '' | xclip -selection clipboard",
);
const XSEL: Backend = (
    "xsel",
    &["--clipboard", "--input"],
    "xsel --clipboard --clear",
);
const WINDOWS: Backend = ("clip.exe", &[], "printf '' | clip.exe");
const MAC: Backend = ("pbcopy", &[], "printf '' | pbcopy");

pub(crate) fn is_wsl() -> bool {
    var_os("WSL_DISTRO_NAME").is_some() || Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists()
}

/// under wsl the windows clipboard is the one that matters, so clip.exe goes first
pub(crate) fn backends() -> Vec<Backend> {
    if is_wsl() {
        vec![WINDOWS, WAYLAND, XCLIP, XSEL, MAC]
    } else {
        vec![WAYLAND, XCLIP, XSEL, WINDOWS, MAC]
    }
}

pub(crate) fn which(bin: &str) -> Option<PathBuf> {
    split_paths(&var_os("PATH")?)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}

pub(crate) fn on_path(bin: &str) -> bool {
    which(bin).is_some()
}

/// copies with the first backend that works and schedules a clear; returns its name
pub(crate) fn copy(text: &str) -> Result<&'static str> {
    let mut tried = Vec::new();
    for (bin, args, clear) in backends() {
        let child = Command::new(bin)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(_) => {
                tried.push(bin);
                continue;
            }
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().is_ok_and(|s| s.success()) {
            schedule_clear(clear);
            return Ok(bin);
        }
        tried.push(bin);
    }

    if tried.is_empty() {
        bail!(
            "no clipboard tool found\n\
             install one: `wl-clipboard` (wayland), `xclip` or `xsel` (x11); under wsl `clip.exe` is used\n\
             or print instead with `sheikah -s REF -p`"
        );
    }
    bail!(
        "copying failed with: {} (no display? try `sheikah -s REF -p`)",
        tried.join(", ")
    )
}

/// best-effort: a detached shell empties the clipboard after `CLEAR_SECS`
fn schedule_clear(clear: &str) {
    let _ = Command::new("sh")
        .args(["-c", &format!("sleep {CLEAR_SECS}; {clear}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
