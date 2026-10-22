use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct Entry {
    pub(crate) service: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<String>,
    pub(crate) created: String,
    pub(crate) modified: String,
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

    pub(crate) fn is_recovery(&self) -> bool {
        !self.recovery.is_empty()
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
    pub(crate) fn haystack(&self) -> [&str; 6] {
        fn o(f: &Option<String>) -> &str {
            f.as_deref().unwrap_or("")
        }
        [
            &self.service,
            o(&self.username),
            o(&self.email),
            o(&self.url),
            o(&self.notes),
            o(&self.category),
        ]
    }
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
}
