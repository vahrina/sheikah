use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use std::{
    env::var_os,
    fs::{self, DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

use crate::{config::Config, entry::Entry};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
pub(crate) const DEFAULT_NAME: &str = "eye";

// m KiB, t, p. not stored in the file: `open` also tries LEGACY, and the next save upgrades
#[cfg(not(test))]
const ARGON: (u32, u32, u32) = (19 * 1024, 2, 1);
#[cfg(test)]
const ARGON: (u32, u32, u32) = (64, 1, 1);
const LEGACY: (u32, u32, u32) = (64 * 1024, 3, 1);

/// an unlocked slate: decrypted entries plus what is needed to re-encrypt them
pub(crate) struct Slate {
    pub(crate) path: PathBuf,
    pass: Zeroizing<String>,
    pub(crate) config: Config,
    pub(crate) entries: Vec<Entry>,
}

/// plaintext inside the slate; older slates hold only the entry list
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Stored {
    Full {
        #[serde(default)]
        config: Config,
        entries: Vec<Entry>,
    },
    Legacy(Vec<Entry>),
}

pub(crate) fn dir() -> Result<PathBuf> {
    let home = var_os("HOME")
        .or_else(|| var_os("XDG_DATA_HOME"))
        .context("neither $HOME nor $XDG_DATA_HOME is set")?;
    Ok(PathBuf::from(home).join(".sheikah"))
}

/// `name` is either a bare slate name inside `~/.sheikah/` or a path
fn named(name: &str) -> Result<PathBuf> {
    if name.contains('/') {
        Ok(PathBuf::from(name))
    } else {
        Ok(dir()?.join(name))
    }
}

/// target path for `init`: the given name/path, else `~/.sheikah/eye`
pub(crate) fn init_path(name: Option<&str>) -> Result<PathBuf> {
    named(name.unwrap_or(DEFAULT_NAME))
}

/// existing slate: the given name/path, else the only slate in `~/.sheikah/`
pub(crate) fn resolve(name: Option<&str>) -> Result<PathBuf> {
    if let Some(n) = name {
        let p = named(n)?;
        if !p.is_file() {
            bail!("no slate at {}", p.display());
        }
        return Ok(p);
    }

    let dir = dir()?;
    let mut slates: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("cannot read {} (run `sheikah init` first)", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_none_or(|e| e != "bak"))
        .collect();
    slates.sort();

    match slates.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!("no slate in {} (run `sheikah init`)", dir.display()),
        many => bail!(
            "more than one slate in {}: {}\npick one with -S <NAME>",
            dir.display(),
            many.iter()
                .filter_map(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn derive_key(pass: &str, salt: &[u8], (m, t, p): (u32, u32, u32)) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(m, t, p, Some(32)).map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pass.as_bytes(), salt, key.as_mut())
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(key)
}

/// `[salt 16B][nonce 12B][ciphertext + 16B gcm tag]`
pub(crate) fn seal(pass: &str, plain: &[u8]) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    rand::fill(&mut salt);
    rand::fill(&mut nonce);

    let key = derive_key(pass, &salt, ARGON)?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref()).expect("32 byte key");
    let ct = cipher
        .encrypt(&Nonce::from(nonce), plain)
        .map_err(|_| anyhow::anyhow!("encryption failed"))?;

    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ct.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// decrypted plaintext, and whether it was sealed with the slower LEGACY params
pub(crate) fn open(pass: &str, blob: &[u8]) -> Result<(Zeroizing<Vec<u8>>, bool)> {
    if blob.len() < SALT_LEN + NONCE_LEN + TAG_LEN {
        bail!("slate file is truncated or not a sheikah slate");
    }
    let (salt, rest) = blob.split_at(SALT_LEN);
    let (nonce, ct) = rest.split_at(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce.try_into().expect("12 byte nonce");

    for params in [ARGON, LEGACY] {
        let key = derive_key(pass, salt, params)?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref()).expect("32 byte key");
        if let Ok(plain) = cipher.decrypt(&Nonce::from(nonce), ct) {
            return Ok((Zeroizing::new(plain), params == LEGACY));
        }
    }
    bail!("wrong master password or corrupted slate")
}

/// create `~/.sheikah/` (0700) and an empty slate file (0600)
pub(crate) fn init(path: &Path, pass: &str) -> Result<()> {
    if path.exists() {
        bail!("refusing to overwrite existing file: {}", path.display());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }

    let blob = seal(pass, b"[]")?;
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    f.write_all(&blob)?;
    f.sync_all()?;
    Ok(())
}

impl Slate {
    pub(crate) fn load(path: PathBuf, pass: Zeroizing<String>) -> Result<Self> {
        let blob = fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
        let (plain, legacy) = open(&pass, &blob)?;
        let (config, entries) =
            match serde_json::from_slice(&plain).context("slate contents are malformed")? {
                Stored::Full { config, entries } => (config, entries),
                Stored::Legacy(entries) => (Config::default(), entries),
            };
        let slate = Slate {
            path,
            pass,
            config,
            entries,
        };
        if legacy {
            slate.save()?;
        }
        Ok(slate)
    }

    /// re-encrypts with a fresh salt + nonce.
    ///
    /// writes in place (after copying the old file to `<slate>.bak`) instead of
    /// tmp + rename, since rename cannot replace files in a `chattr +a` dir
    pub(crate) fn save(&self) -> Result<()> {
        let plain = Zeroizing::new(serde_json::to_vec(&serde_json::json!({
            "config": self.config,
            "entries": self.entries,
        }))?);
        let blob = seal(&self.pass, &plain)?;

        let mut bak = self.path.clone().into_os_string();
        bak.push(".bak");
        let _guard = crate::signal::Critical::enter();
        write_file(Path::new(&bak), &fs::read(&self.path)?)?;
        write_file(&self.path, &blob)
    }

    pub(crate) fn rekey(&mut self, new_pass: Zeroizing<String>) -> Result<()> {
        self.pass = new_pass;
        self.save()
    }
}

fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("cannot write {}", path.display()))?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("sheikah-test-{}-{name}", std::process::id()));
        let _ = fs::remove_file(&p);
        let _ = fs::remove_file(p.with_extension("bak"));
        p
    }

    #[test]
    fn seal_open_round_trip() {
        let blob = seal("hunter2", b"[]").unwrap();
        assert_eq!(blob.len(), SALT_LEN + NONCE_LEN + 2 + TAG_LEN);
        assert_eq!(open("hunter2", &blob).unwrap().0.as_slice(), b"[]");
        assert!(open("wrong", &blob).is_err());
    }

    #[test]
    fn fresh_salt_and_nonce_each_seal() {
        let a = seal("pw", b"x").unwrap();
        let b = seal("pw", b"x").unwrap();
        assert_ne!(a[..SALT_LEN + NONCE_LEN], b[..SALT_LEN + NONCE_LEN]);
    }

    #[test]
    fn init_save_load_rekey() {
        let path = tmp("slate");
        init(&path, "old").unwrap();
        assert!(init(&path, "old").is_err());

        let mut v = Slate::load(path.clone(), Zeroizing::new("old".into())).unwrap();
        assert!(v.entries.is_empty());
        v.entries.push(Entry::new("github".into(), "s3cret".into()));
        v.save().unwrap();

        let mut v = Slate::load(path.clone(), Zeroizing::new("old".into())).unwrap();
        assert_eq!(v.entries[0].service, "github");
        assert_eq!(v.entries[0].password, "s3cret");

        v.rekey(Zeroizing::new("new".into())).unwrap();
        assert!(Slate::load(path.clone(), Zeroizing::new("old".into())).is_err());
        let v = Slate::load(path.clone(), Zeroizing::new("new".into())).unwrap();
        assert_eq!(v.entries.len(), 1);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("bak"));
    }
}
