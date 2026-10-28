use anyhow::{Context, Result, anyhow, bail};
use std::{fs, path::Path};

use crate::config;

pub(crate) const DEFAULT: &str = "0x96f";
pub(crate) const CUSTOM: &str = "custom";

/// `base00` .. `base0F` as rgb
pub(crate) type Palette = [[u8; 3]; 16];

/// copied from https://github.com/tinted-theming/schemes (base16)
const BUILTIN: [(&str, [&str; 16]); 3] = [
    (
        "0x96f",
        [
            "262427", "3b393c", "514f52", "676567", "7c7b7d", "fcfcfc", "eae9eb", "fcfcfc",
            "ff7272", "fc9d6f", "ffca58", "bcdf59", "aee8f4", "49cae4", "a093e2", "ff8787",
        ],
    ),
    (
        "ascendancy",
        [
            "282828", "212f3d", "504945", "928374", "bdae93", "d5c7a1", "ebdbb2", "fbf1c7",
            "d05000", "fe8019", "ffcc1b", "b8bb26", "8f3f71", "458588", "fabd2f", "b59b4d",
        ],
    ),
    (
        "black-metal",
        [
            "000000", "121212", "222222", "333333", "999999", "c1c1c1", "999999", "c1c1c1",
            "5f8787", "aaaaaa", "a06666", "dd9999", "aaaaaa", "888888", "999999", "444444",
        ],
    ),
];

const NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Color {
    /// 0-15 follow the terminal's own palette, 16-255 the xterm cube
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    /// the SGR foreground parameters, `38;5;n` instead of `38;2;r;g;b` without truecolor
    pub(crate) fn sgr(self, truecolor: bool) -> String {
        match self {
            Color::Indexed(n @ 0..8) => format!("3{n}"),
            Color::Indexed(n @ 8..16) => format!("9{}", n - 8),
            Color::Indexed(n) => format!("38;5;{n}"),
            Color::Rgb(r, g, b) if truecolor => format!("38;2;{r};{g};{b}"),
            Color::Rgb(r, g, b) => format!("38;5;{}", xterm256(r, g, b)),
        }
    }

    fn fzf(self) -> String {
        match self {
            Color::Indexed(n) => n.to_string(),
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

fn xterm256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        return match r {
            0..8 => 16,
            249.. => 231,
            v => 232 + ((v - 8) / 10).min(23),
        };
    }
    let q = |c: u8| match c {
        0..48 => 0,
        48..115 => 1,
        c => (c - 35) / 40,
    };
    16 + 36 * q(r) + 6 * q(g) + q(b)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Style {
    pub(crate) fg: Option<Color>,
    pub(crate) bold: bool,
    pub(crate) dim: bool,
}

/// every painted role; `selection` is only a background, for fzf's current line
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Styles {
    pub(crate) header: Style,
    pub(crate) border: Style,
    pub(crate) text: Style,
    pub(crate) muted: Style,
    pub(crate) key: Style,
    pub(crate) secret: Style,
    pub(crate) accent: Style,
    pub(crate) error: Style,
    pub(crate) selection: Option<Color>,
}

impl Styles {
    /// the original look: bold & dim only, in the terminal's own colors
    fn mono() -> Self {
        let bold = Style {
            bold: true,
            ..Style::default()
        };
        let dim = Style {
            dim: true,
            ..Style::default()
        };
        Styles {
            header: bold,
            border: dim,
            text: Style::default(),
            muted: dim,
            key: dim,
            secret: bold,
            accent: bold,
            error: bold,
            selection: Some(Color::Indexed(236)),
        }
    }

    fn base16(p: &Palette) -> Self {
        let c = |i: usize| {
            let [r, g, b] = p[i];
            Color::Rgb(r, g, b)
        };
        let plain = |i| Style {
            fg: Some(c(i)),
            ..Style::default()
        };
        let bold = |i| Style {
            fg: Some(c(i)),
            bold: true,
            dim: false,
        };
        Styles {
            header: bold(0x0D),
            border: plain(0x03),
            text: plain(0x05),
            muted: plain(0x04),
            key: plain(0x0E),
            secret: bold(0x08),
            accent: plain(0x0B),
            error: bold(0x08),
            selection: Some(c(0x02)),
        }
    }

    /// `--color=...` for fzf, falling back to the old monochrome scheme per slot
    pub(crate) fn fzf(&self) -> String {
        let f = |s: Style, fallback: &str| s.fg.map_or(fallback.to_string(), Color::fzf);
        let bg = self.selection.map_or("236".to_string(), Color::fzf);
        format!(
            "--color=fg:{},bg:-1,hl:{},fg+:{},bg+:{bg},hl+:{},info:{},prompt:{},pointer:{},marker:{},query:{}",
            f(self.text, "7"),
            f(self.accent, "15"),
            f(self.header, "15"),
            f(self.accent, "15"),
            f(self.muted, "8"),
            f(self.muted, "8"),
            f(self.secret, "15"),
            f(self.muted, "8"),
            f(self.text, "15"),
        )
    }
}

impl Default for Styles {
    fn default() -> Self {
        Styles::base16(&builtin(DEFAULT).expect("default theme is built in"))
    }
}

fn builtin(name: &str) -> Option<Palette> {
    let (_, hexes) = BUILTIN.iter().find(|(n, _)| *n == name)?;
    let mut p = [[0; 3]; 16];
    for (slot, h) in p.iter_mut().zip(hexes) {
        let Color::Rgb(r, g, b) = hex(h).ok()? else {
            return None;
        };
        *slot = [r, g, b];
    }
    Some(p)
}

/// the configured theme with every override applied on top
pub(crate) fn resolve(t: &config::Theme, themes_dir: Option<&Path>) -> Result<Styles> {
    let pal = palette(&t.name, themes_dir)?;
    let mut s = pal.as_ref().map_or_else(Styles::mono, Styles::base16);

    let roles = [
        ("header", &t.header, &mut s.header),
        ("border", &t.border, &mut s.border),
        ("text", &t.text, &mut s.text),
        ("muted", &t.muted, &mut s.muted),
        ("key", &t.key, &mut s.key),
        ("secret", &t.secret, &mut s.secret),
        ("accent", &t.accent, &mut s.accent),
        ("error", &t.error, &mut s.error),
    ];
    for (role, val, style) in roles {
        if let Some(v) = val {
            style.fg = parse(v, pal.as_ref()).with_context(|| format!("theme.{role}"))?;
            style.dim = false;
        }
    }
    if let Some(v) = &t.selection {
        s.selection = parse(v, pal.as_ref()).context("theme.selection")?;
    }
    Ok(s)
}

/// `custom` has none; a base16 yaml in `themes_dir` wins over a built-in of the same name
fn palette(name: &str, themes_dir: Option<&Path>) -> Result<Option<Palette>> {
    let name = name.trim();
    if name == CUSTOM {
        return Ok(None);
    }
    let name = name.strip_prefix("base16-").unwrap_or(name);
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        bail!("invalid theme name '{name}'");
    }
    if let Some(dir) = themes_dir {
        for ext in ["yaml", "yml"] {
            let p = dir.join(format!("{name}.{ext}"));
            if p.is_file() {
                let text = fs::read_to_string(&p)
                    .with_context(|| format!("cannot read {}", p.display()))?;
                return scheme(&text)
                    .with_context(|| format!("bad base16 scheme {}", p.display()))
                    .map(Some);
            }
        }
    }
    builtin(name).map(Some).ok_or_else(|| {
        let known: Vec<&str> = BUILTIN.iter().map(|b| b.0).collect();
        anyhow!(
            "unknown theme '{name}'\nbuilt in are {}, {CUSTOM}\nor drop {name}.yaml into ~/.sheikah/themes/",
            known.join(", ")
        )
    })
}

/// the `baseXX: "rrggbb"` lines of a tinted-theming scheme; no yaml parser needed for those
fn scheme(text: &str) -> Result<Palette> {
    let mut slots: [Option<[u8; 3]>; 16] = [None; 16];
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim().to_ascii_lowercase();
        let Some(i) = k
            .strip_prefix("base")
            .filter(|n| n.len() == 2)
            .and_then(|n| usize::from_str_radix(n, 16).ok())
            .filter(|&i| i < 16)
        else {
            continue;
        };
        let v = v.trim();
        let v = match v.strip_prefix(['"', '\'']) {
            Some(rest) => rest.split(['"', '\'']).next().unwrap_or(""),
            None => v.split_whitespace().next().unwrap_or(""),
        };
        let Color::Rgb(r, g, b) =
            hex(v.trim_start_matches('#')).with_context(|| format!("base{i:02X}: '{v}'"))?
        else {
            unreachable!()
        };
        slots[i] = Some([r, g, b]);
    }
    let mut p = [[0; 3]; 16];
    for (i, s) in slots.iter().enumerate() {
        p[i] = s.with_context(|| format!("missing base{i:02X}"))?;
    }
    Ok(p)
}

/// `red`, `bright-red`, `#ff0000`, `base08` (needs a base16 theme) or `default`
pub(crate) fn parse(s: &str, pal: Option<&Palette>) -> Result<Option<Color>> {
    let v = s.trim().to_ascii_lowercase();
    if v == "default" {
        return Ok(None);
    }
    if let Some(h) = v.strip_prefix('#') {
        return hex(h).map(Some);
    }
    if let Some(i) = v
        .strip_prefix("base")
        .filter(|n| n.len() == 2)
        .and_then(|n| usize::from_str_radix(n, 16).ok())
    {
        let p = pal.context("baseXX colors need a base16 theme, not `custom`")?;
        let [r, g, b] = *p.get(i).context("only base00 .. base0F exist")?;
        return Ok(Some(Color::Rgb(r, g, b)));
    }
    let (bright, base) = match v.strip_prefix("bright-") {
        Some(b) => (true, b),
        None => (false, v.as_str()),
    };
    match NAMES.iter().position(|n| *n == base) {
        Some(i) => Ok(Some(Color::Indexed(i as u8 + if bright { 8 } else { 0 }))),
        None => bail!(
            "unknown color '{s}': use a name ({}, bright-*), #rrggbb, baseXX or default",
            NAMES.join(", ")
        ),
    }
}

fn hex(h: &str) -> Result<Color> {
    let d = |i: usize| u8::from_str_radix(&h[i..i + 2], 16);
    match (h.len() == 6 && h.is_ascii()).then(|| (d(0), d(2), d(4))) {
        Some((Ok(r), Ok(g), Ok(b))) => Ok(Color::Rgb(r, g, b)),
        _ => bail!("bad hex color '#{h}', use #rrggbb"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(s: &str) -> Color {
        parse(s, None).unwrap().unwrap()
    }

    #[test]
    fn color_formats() {
        assert_eq!(rgb("red"), Color::Indexed(1));
        assert_eq!(rgb("Bright-Blue"), Color::Indexed(12));
        assert_eq!(rgb("#49CAE4"), Color::Rgb(73, 202, 228));
        assert_eq!(parse("default", None).unwrap(), None);

        for bad in ["redd", "", "#f80", "#49cae480", "#gggggg", "rgb(1,2,3)"] {
            assert!(parse(bad, None).is_err(), "{bad}");
        }
        assert!(parse("base0D", None).is_err());
        let p = builtin(DEFAULT).unwrap();
        assert_eq!(
            parse("base0D", Some(&p)).unwrap(),
            Some(Color::Rgb(73, 202, 228))
        );
    }

    #[test]
    fn sgr_codes() {
        assert_eq!(Color::Indexed(1).sgr(true), "31");
        assert_eq!(Color::Indexed(9).sgr(true), "91");
        assert_eq!(Color::Indexed(236).sgr(true), "38;5;236");
        assert_eq!(Color::Rgb(1, 2, 3).sgr(true), "38;2;1;2;3");
        assert_eq!(Color::Rgb(255, 0, 0).sgr(false), "38;5;196");
        assert_eq!(Color::Rgb(0, 0, 0).sgr(false), "38;5;16");
    }

    #[test]
    fn builtins_parse() {
        for (name, _) in BUILTIN {
            assert!(builtin(name).is_some(), "{name}");
        }
    }

    #[test]
    fn scheme_file_formats() {
        let mut text = String::from("system: \"base16\"\nname: \"x\"\npalette:\n");
        for i in 0..16 {
            text.push_str(&format!("  base{i:02X}: \"#{i:02x}{i:02x}{i:02x}\" # c\n"));
        }
        let p = scheme(&text).unwrap();
        assert_eq!(p[0x0D], [0x0d; 3]);

        let old: String = (0..16)
            .map(|i| format!("base{i:02X}: {i:02x}0000\n"))
            .collect();
        assert_eq!(scheme(&old).unwrap()[15], [0x0f, 0, 0]);

        assert!(scheme("base00: \"000000\"").is_err());
    }

    #[test]
    fn overrides_win_over_the_theme() {
        let mut t = config::Theme::default();
        let base = resolve(&t, None).unwrap();
        assert_eq!(base, Styles::default());

        t.border = Some("red".into());
        t.secret = Some("base0B".into());
        let s = resolve(&t, None).unwrap();
        assert_eq!(s.border.fg, Some(Color::Indexed(1)));
        assert_eq!(s.secret.fg, base.accent.fg);
        assert!(s.secret.bold);
        assert_eq!(s.header, base.header);

        t.name = CUSTOM.into();
        t.secret = None;
        let s = resolve(&t, None).unwrap();
        assert_eq!(s.border.fg, Some(Color::Indexed(1)));
        assert!(!s.border.dim);
        assert_eq!(s.muted, Styles::mono().muted);

        t.name = "nope".into();
        assert!(resolve(&t, None).is_err());
        t.name = "../x".into();
        assert!(resolve(&t, None).is_err());
    }

    #[test]
    fn user_themes_shadow_builtins() {
        let dir = std::env::temp_dir().join(format!("sheikah-themes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let text: String = (0..16)
            .map(|i| format!("base{i:02X}: \"ffffff\"\n"))
            .collect();
        fs::write(dir.join("0x96f.yaml"), &text).unwrap();
        fs::write(dir.join("nord.yml"), &text).unwrap();

        let mut t = config::Theme::default();
        let s = resolve(&t, Some(&dir)).unwrap();
        assert_eq!(s.header.fg, Some(Color::Rgb(255, 255, 255)));
        t.name = "base16-nord".into();
        assert!(resolve(&t, Some(&dir)).is_ok());

        fs::remove_dir_all(&dir).unwrap();
    }
}
