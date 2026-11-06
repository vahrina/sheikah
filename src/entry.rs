use serde::{Deserialize, Serialize};
use std::{
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

/// builtin entry types; drives which fields `add`/`edit` ask for
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EntryType {
    /// also accepts a stored `custom` from earlier slates
    #[default]
    #[serde(alias = "custom")]
    Login,
    /// also accepts a stored `backup` from earlier slates
    #[serde(alias = "backup")]
    Recovery,
    Ssh,
}

impl EntryType {
    pub(crate) const CHOICES: &str = "login | recovery | ssh";

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            EntryType::Login => "login",
            EntryType::Recovery => "recovery",
            EntryType::Ssh => "ssh",
        }
    }

    pub(crate) fn uses_recovery(self) -> bool {
        matches!(self, EntryType::Recovery)
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EntryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "login" => Ok(EntryType::Login),
            "recovery" => Ok(EntryType::Recovery),
            "ssh" => Ok(EntryType::Ssh),
            other => Err(format!("unknown type '{other}'")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct Entry {
    pub(crate) service: String,
    /// missing on older slates; `custom` and `backup` load as `login` and `recovery`
    #[serde(default, rename = "type")]
    pub(crate) kind: EntryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<String>,
    pub(crate) password: String,
    /// recovery codes, stored instead of a password
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) recovery: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    /// ssh port when `type = ssh`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tags: Vec<String>,
    pub(crate) created: String,
    pub(crate) modified: String,
    /// last show/copy/find, only stamped while `table.sort = "used"`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) used: Option<String>,
}

impl Entry {
    pub(crate) fn new(service: String, password: String) -> Self {
        let ts = now();
        Entry {
            service,
            password,
            created: ts.clone(),
            modified: ts,
            ..Default::default()
        }
    }

    pub(crate) fn touch(&mut self) {
        self.modified = now();
    }

    pub(crate) fn entry_type(&self) -> EntryType {
        self.kind
    }

    pub(crate) fn is_recovery(&self) -> bool {
        self.entry_type().uses_recovery() || !self.recovery.is_empty()
    }

    /// the password, or the recovery codes one per line
    pub(crate) fn secret(&self) -> String {
        if self.is_recovery() {
            self.recovery.join("\n")
        } else {
            self.password.clone()
        }
    }

    /// every searchable field except the password
    pub(crate) fn haystack(&self) -> Vec<&str> {
        fn o(f: &Option<String>) -> &str {
            f.as_deref().unwrap_or("")
        }
        let mut h = vec![
            self.service.as_str(),
            self.entry_type().as_str(),
            o(&self.username),
            o(&self.email),
            o(&self.url),
            o(&self.port),
            o(&self.notes),
        ];
        h.extend(self.tags.iter().map(String::as_str));
        h
    }

    pub(crate) fn tags_display(&self) -> Option<String> {
        (!self.tags.is_empty()).then(|| self.tags.join(", "))
    }

    pub(crate) fn has_tags(&self, required: &[String]) -> bool {
        required.iter().all(|t| self.tags.iter().any(|et| et == t))
    }
}

/// trim, lowercase, drop empties & duplicates from a comma-separated tags string
pub(crate) fn parse_tags(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let t = part.trim().to_lowercase();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// current UTC time as RFC3339 (`2026-10-21T01:48:00Z`)
pub(crate) fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    rfc3339(secs)
}

pub(crate) fn rfc3339(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);

    // civil-from-days, http://howardhinnant.github.io/date_algorithms.html
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(mo <= 2);

    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_codes_as_secret() {
        let mut e = Entry::new("github".into(), String::new());
        e.recovery = vec!["aaaa-bbbb".into(), "cccc-dddd".into()];
        assert!(e.is_recovery());
        assert_eq!(e.secret(), "aaaa-bbbb\ncccc-dddd");

        let back: Entry = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(back, e);

        let old: Entry =
            serde_json::from_str(r#"{"service":"x","password":"pw","created":"","modified":""}"#)
                .unwrap();
        assert!(!old.is_recovery());
        assert_eq!(old.secret(), "pw");
    }

    #[test]
    fn rfc3339_known_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_792_547_280), "2026-10-21T01:48:00Z");
    }

    #[test]
    fn parse_tags_trims_lowers_dedups() {
        assert_eq!(parse_tags("git, DEV, git, , ,Dev"), vec!["git", "dev"]);
        assert!(parse_tags("  , , ").is_empty());
    }
    #[test]
    fn missing_and_retired_types_load() {
        let old: Entry =
            serde_json::from_str(r#"{"service":"x","password":"pw","created":"","modified":""}"#)
                .unwrap();
        assert_eq!(old.kind, EntryType::Login);

        let custom: Entry = serde_json::from_str(
            r#"{"service":"x","password":"pw","type":"custom","created":"","modified":""}"#,
        )
        .unwrap();
        assert_eq!(custom.kind, EntryType::Login);

        let backup: Entry = serde_json::from_str(
            r#"{"service":"x","password":"pw","type":"backup","created":"","modified":""}"#,
        )
        .unwrap();
        assert_eq!(backup.kind, EntryType::Recovery);
    }

    #[test]
    fn entry_type_parses_builtins() {
        assert_eq!("login".parse::<EntryType>().unwrap(), EntryType::Login);
        assert_eq!("SSH".parse::<EntryType>().unwrap(), EntryType::Ssh);
        assert!("backup".parse::<EntryType>().is_err());
        assert!("custom".parse::<EntryType>().is_err());
        assert!("api".parse::<EntryType>().is_err());
    }
}
