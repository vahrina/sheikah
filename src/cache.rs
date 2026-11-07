//! short-lived master-password cache under `$XDG_RUNTIME_DIR` (else `/dev/shm`)
use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use std::{
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| PathBuf::from("/dev/shm"))
}

fn cache_file(slate: &Path) -> PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    slate.hash(&mut h);
    runtime_dir().join(format!("sheikah-pass-{:x}", h.finish()))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// load cached password if present and not expired; expired/corrupt files are removed
pub(crate) fn load(slate: &Path) -> Option<SecretString> {
    let path = cache_file(slate);
    let raw = Zeroizing::new(fs::read_to_string(&path).ok()?);
    let (exp, pass) = raw.split_once('\n')?;
    let exp: u64 = exp.trim().parse().ok()?;
    if now_secs() >= exp {
        clear(slate);
        return None;
    }
    let pass = pass.trim_end_matches('\n');
    if pass.is_empty() {
        clear(slate);
        return None;
    }
    Some(SecretString::from(pass.to_owned()))
}

/// remember `pass` for `secs` seconds; `0` clears the cache
pub(crate) fn store(slate: &Path, pass: &SecretString, secs: u32) -> Result<()> {
    if secs == 0 {
        clear(slate);
        return Ok(());
    }
    let path = cache_file(slate);
    let exp = now_secs().saturating_add(u64::from(secs));
    let body = Zeroizing::new(format!("{exp}\n{}", pass.expose_secret()));
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("cannot write {}", path.display()))?;
    f.write_all(body.as_bytes())?;
    f.sync_all()?;
    Ok(())
}

pub(crate) fn clear(slate: &Path) {
    let _ = fs::remove_file(cache_file(slate));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_load_clear_and_expire() {
        let slate = std::env::temp_dir().join(format!("sheikah-cache-test-{}", std::process::id()));
        clear(&slate);
        assert!(load(&slate).is_none());

        let secret = SecretString::from("hunter2".to_owned());
        store(&slate, &secret, 60).unwrap();
        assert_eq!(load(&slate).unwrap().expose_secret(), "hunter2");

        // force-expire by rewriting with past timestamp
        let path = cache_file(&slate);
        fs::write(&path, "1\nhunter2\n").unwrap();
        assert!(load(&slate).is_none());
        assert!(!path.exists());

        store(&slate, &secret, 0).unwrap();
        assert!(load(&slate).is_none());

        // store then clear
        store(&slate, &secret, 30).unwrap();
        clear(&slate);
        assert!(load(&slate).is_none());
    }
}
