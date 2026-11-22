//! plaintext export & import. json is a lossless sheikah dump; csv uses chrome's columns so
//! other managers read it. import also takes unencrypted bitwarden json & most managers' csv
use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::entry::{self, Entry};

pub(crate) fn to_json(entries: &[Entry]) -> Result<String> {
    Ok(serde_json::to_string_pretty(entries)?)
}

pub(crate) fn to_csv(entries: &[Entry]) -> String {
    let mut out = csv_line(&[
        "name", "url", "username", "password", "note", "email", "tags",
    ]);
    for e in entries {
        fn o(f: &Option<String>) -> &str {
            f.as_deref().unwrap_or("")
        }
        let tags = e.tags.join(", ");
        let secret = e.secret();
        out += &csv_line(&[
            &e.service,
            o(&e.url),
            o(&e.username),
            &secret,
            o(&e.notes),
            o(&e.email),
            &tags,
        ]);
    }
    out
}

fn csv_line(fields: &[&str]) -> String {
    let quoted: Vec<String> = fields
        .iter()
        .map(|f| {
            if f.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.to_string()
            }
        })
        .collect();
    quoted.join(",") + "\n"
}

/// entries found in `text`, and how many items were skipped
pub(crate) fn parse(text: &str) -> Result<(Vec<Entry>, usize)> {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    if text.is_empty() {
        bail!("import file is empty");
    }
    let rows = if text.starts_with(['[', '{']) {
        let v: Value = serde_json::from_str(text).context("invalid json")?;
        if v.is_array() {
            let entries = serde_json::from_value(v).context("not a sheikah export")?;
            return Ok((entries, 0));
        }
        from_bitwarden(&v)?
    } else {
        from_csv(text)?
    };
    let total = rows.len();
    let entries: Vec<Entry> = rows.into_iter().filter_map(Row::into_entry).collect();
    let skipped = total - entries.len();
    Ok((entries, skipped))
}

/// one foreign record, every field as found
#[derive(Default)]
struct Row {
    service: String,
    url: String,
    username: String,
    email: String,
    password: String,
    notes: String,
    totp: String,
    tags: String,
}

impl Row {
    /// `None` when neither name, url nor username can label the entry
    fn into_entry(self) -> Option<Entry> {
        let opt = |s: &str| {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_string())
        };
        let service = opt(&self.service)
            .or_else(|| host(&self.url))
            .or_else(|| opt(&self.username))?;
        let mut e = Entry::new(service, self.password);
        e.url = opt(&self.url);
        e.username = opt(&self.username);
        e.email = opt(&self.email);
        e.tags = entry::parse_tags(&self.tags);
        let totp = opt(&self.totp).map(|t| format!("totp: {t}"));
        e.notes = match (opt(&self.notes), totp) {
            (Some(n), Some(t)) => Some(format!("{n}\n{t}")),
            (n, t) => n.or(t),
        };
        Some(e)
    }
}

/// `https://www.github.com:443/login` gives `github.com`
fn host(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let h = rest.split(['/', '?', '#']).next()?;
    let h = h.rsplit_once('@').map_or(h, |(_, h)| h);
    let h = h.split(':').next()?.trim_start_matches("www.");
    (!h.is_empty()).then(|| h.to_lowercase())
}

/// logins & secure notes; cards & identities have no place in a slate
fn from_bitwarden(v: &Value) -> Result<Vec<Row>> {
    if v["encrypted"] == true {
        bail!("encrypted bitwarden export, export it again as unencrypted json");
    }
    let items = v["items"]
        .as_array()
        .context("unknown json, expected a sheikah or bitwarden export")?;
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    Ok(items
        .iter()
        .map(|i| {
            if !matches!(i["type"].as_u64(), Some(1 | 2)) {
                return Row::default();
            }
            let l = &i["login"];
            Row {
                service: s(&i["name"]),
                url: s(&l["uris"][0]["uri"]),
                username: s(&l["username"]),
                password: s(&l["password"]),
                notes: s(&i["notes"]),
                totp: s(&l["totp"]),
                ..Default::default()
            }
        })
        .collect())
}

/// header names per `Row` field, compared lowercase without spaces, `_` or `-`
const COLUMNS: [&[&str]; 8] = [
    &["name", "title", "service", "account"],
    &["url", "loginuri", "website", "uri", "urls"],
    &["username", "loginusername", "user", "login"],
    &["email", "emailaddress"],
    &["password", "loginpassword"],
    &["note", "notes", "extra", "comments"],
    &["totp", "logintotp", "otpauth", "otp"],
    &["tags", "folder", "grouping", "group", "category"],
];

fn from_csv(text: &str) -> Result<Vec<Row>> {
    let mut rows = parse_csv(text);
    rows.retain(|r| r.iter().any(|f| !f.trim().is_empty()));
    if rows.is_empty() {
        bail!("csv has no header");
    }
    let header: Vec<String> = rows
        .remove(0)
        .iter()
        .map(|h| h.to_lowercase().replace([' ', '_', '-'], ""))
        .collect();
    let [service, url, username, email, password, notes, totp, tags] = COLUMNS.map(|names| {
        names
            .iter()
            .find_map(|n| header.iter().position(|h| h == n))
    });
    if password.is_none() {
        bail!("unknown csv header, found no password column");
    }
    Ok(rows
        .iter()
        .map(|r| {
            let get = |c: Option<usize>| c.and_then(|c| r.get(c)).cloned().unwrap_or_default();
            Row {
                service: get(service),
                url: get(url),
                username: get(username),
                email: get(email),
                password: get(password),
                notes: get(notes),
                totp: get(totp),
                tags: get(tags),
            }
        })
        .collect())
}

/// rfc 4180: quoted fields may hold commas, `""` & newlines; `\r\n` or `\n` ends a row
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') if chars.peek() == Some(&'\n') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (false, c) => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EntryType;

    fn sample() -> Vec<Entry> {
        let mut a = Entry::new("github".into(), "pa,ss\"w".into());
        a.username = Some("v".into());
        a.email = Some("v@x.io".into());
        a.url = Some("https://github.com".into());
        a.notes = Some("line one\nline two".into());
        a.tags = vec!["dev".into(), "git".into()];
        let mut b = Entry::new("proton".into(), String::new());
        b.kind = EntryType::Recovery;
        b.recovery = vec!["aaaa".into(), "bbbb".into()];
        let mut c = Entry::new("rpi".into(), "pi".into());
        c.kind = EntryType::Ssh;
        c.url = Some("10.0.0.2".into());
        c.port = Some("2222".into());
        vec![a, b, c]
    }

    #[test]
    fn json_round_trip_is_lossless() {
        let e = sample();
        assert_eq!(parse(&to_json(&e).unwrap()).unwrap(), (e, 0));
    }

    #[test]
    fn csv_round_trip_keeps_login_fields() {
        let e = sample();
        let out = to_csv(&e);
        assert!(out.starts_with("name,url,username,password,note,email,tags\n"));
        let (back, skipped) = parse(&out).unwrap();
        assert_eq!(skipped, 0);
        let (a, b) = (&e[0], &back[0]);
        assert_eq!(
            (
                &a.service,
                &a.url,
                &a.username,
                &a.password,
                &a.notes,
                &a.email,
                &a.tags
            ),
            (
                &b.service,
                &b.url,
                &b.username,
                &b.password,
                &b.notes,
                &b.email,
                &b.tags
            )
        );
        assert_eq!(back[1].password, "aaaa\nbbbb");
    }

    #[test]
    fn foreign_csv_headers() {
        let chrome = "name,url,username,password,note\ngh,https://github.com,v,pw,\n";
        let (e, _) = parse(chrome).unwrap();
        assert_eq!(
            (e[0].service.as_str(), e[0].password.as_str()),
            ("gh", "pw")
        );

        let firefox = "\"url\",\"username\",\"password\",\"httpRealm\"\r\n\
                       \"https://www.netflix.com/login\",\"me\",\"pw\",\r\n";
        assert_eq!(parse(firefox).unwrap().0[0].service, "netflix.com");

        let bitwarden = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\n\
                         Work,,login,mail,hi,,0,https://mail.x,me,pw,JBSW\n";
        let (e, _) = parse(bitwarden).unwrap();
        assert_eq!(e[0].tags, ["work"]);
        assert_eq!(e[0].notes.as_deref(), Some("hi\ntotp: JBSW"));

        let keepass = "\"Group\",\"Title\",\"Username\",\"Password\",\"URL\",\"Notes\"\n\
                       \"Root\",\"steam\",\"u\",\"p\",\"\",\"\"\n";
        assert_eq!(parse(keepass).unwrap().0[0].service, "steam");

        assert!(parse("a,b\n1,2\n").is_err());
    }

    #[test]
    fn bitwarden_json() {
        let bw = r#"{"encrypted":false,"items":[
            {"type":1,"name":"github","notes":null,
             "login":{"username":"v","password":"pw","totp":null,"uris":[{"uri":"https://github.com"}]}},
            {"type":3,"name":"visa","card":{}}]}"#;
        let (e, skipped) = parse(bw).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(e[0].password, "pw");
        assert_eq!(e[0].url.as_deref(), Some("https://github.com"));
        assert!(parse(r#"{"encrypted":true,"items":[]}"#).is_err());
        assert!(parse(r#"{"foo":1}"#).is_err());
    }

    #[test]
    fn host_from_url() {
        assert_eq!(
            host("https://www.GitHub.com:443/login?x").as_deref(),
            Some("github.com")
        );
        assert_eq!(host("user@example.org").as_deref(), Some("example.org"));
        assert_eq!(host(""), None);
    }
}
