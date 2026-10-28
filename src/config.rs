use serde::{Deserialize, Serialize};

/// lives encrypted inside the slate; edited as toml via `sheikah config`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) censor: Censor,
    pub(crate) table: Table,
    pub(crate) clipboard: Clipboard,
    pub(crate) theme: Theme,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct Clipboard {
    /// seconds until a copied secret is wiped, 0 keeps it
    pub(crate) clear: u32,
}

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard { clear: 45 }
    }
}

/// `name` picks the base16 palette; every other key overrides that one role of it
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct Theme {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) muted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) selection: Option<String>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: crate::theme::DEFAULT.into(),
            header: None,
            border: None,
            text: None,
            muted: None,
            key: None,
            secret: None,
            accent: None,
            error: None,
            selection: None,
        }
    }
}

/// fields hidden as `glyph` in `list` and `show` (explicit `show -p` etc. always reveals)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct Censor {
    pub(crate) glyph: String,
    pub(crate) service: bool,
    pub(crate) username: bool,
    pub(crate) email: bool,
    pub(crate) password: bool,
    pub(crate) url: bool,
    pub(crate) notes: bool,
    pub(crate) category: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub(crate) struct Table {
    /// always draw a box, not only with `-b`
    pub(crate) boxed: bool,
    /// a leading `#` column counting rows from 1
    pub(crate) index: bool,
    pub(crate) style: BoxStyle,
    /// order of `list` and the fzf pickers
    pub(crate) sort: Sort,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Sort {
    /// service, then username
    #[default]
    Alphabetical,
    /// newest edit first
    Modified,
    /// most recently shown/copied first
    Used,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BoxStyle {
    #[default]
    Ascii,
    Square,
    Rounded,
    Double,
    Heavy,
    Dashed,
}

/// corners and joints, as `[[top], [middle], [bottom]]` of `[left, join, right]`, plus lines
pub(crate) struct Glyphs {
    pub(crate) rows: [[char; 3]; 3],
    pub(crate) h: char,
    pub(crate) v: char,
}

impl BoxStyle {
    pub(crate) fn glyphs(self) -> Glyphs {
        let g = |rows, h, v| Glyphs { rows, h, v };
        match self {
            BoxStyle::Ascii => g([['+'; 3]; 3], '-', '|'),
            BoxStyle::Square => g(
                [['┌', '┬', '┐'], ['├', '┼', '┤'], ['└', '┴', '┘']],
                '─',
                '│',
            ),
            BoxStyle::Rounded => g(
                [['╭', '┬', '╮'], ['├', '┼', '┤'], ['╰', '┴', '╯']],
                '─',
                '│',
            ),
            BoxStyle::Double => g(
                [['╔', '╦', '╗'], ['╠', '╬', '╣'], ['╚', '╩', '╝']],
                '═',
                '║',
            ),
            BoxStyle::Heavy => g(
                [['┏', '┳', '┓'], ['┣', '╋', '┫'], ['┗', '┻', '┛']],
                '━',
                '┃',
            ),
            BoxStyle::Dashed => g(
                [['┌', '┬', '┐'], ['├', '┼', '┤'], ['└', '┴', '┘']],
                '╌',
                '╎',
            ),
        }
    }
}

impl Default for Censor {
    fn default() -> Self {
        Censor {
            glyph: "***".into(),
            service: false,
            username: false,
            email: true,
            password: true,
            url: false,
            notes: false,
            category: false,
        }
    }
}

pub(crate) const HELP: &str = "\
# censor
#   glyph: shown instead of a hidden field
#   <field> = true hides it in list/show
# table
#   style: ascii | square | rounded | double | heavy | dashed
#   index: number rows in list
#   sort: alphabetical | modified | used (newest first)
# clipboard
#   clear: seconds until a copied secret is wiped, 0 to disable
# theme
#   name: 0x96f | ascendancy | black-metal | custom
#   <role> overrides one role of the theme:
#     header, border, text, muted, key, secret, accent, error, selection
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_partial_toml() {
        let c = Config::default();
        assert!(c.censor.password && c.censor.email && !c.censor.username);
        assert_eq!(c.censor.glyph, "***");
        assert_eq!(c.table.sort, Sort::Alphabetical);
        assert_eq!(c.clipboard.clear, 45);

        let c: Config = toml::from_str("[table]\nsort = \"used\"\n[clipboard]\nclear = 0").unwrap();
        assert_eq!(c.table.sort, Sort::Used);
        assert_eq!(c.clipboard.clear, 0);

        let c: Config = toml::from_str("[table]\nstyle = \"rounded\"").unwrap();
        assert_eq!(c.table.style, BoxStyle::Rounded);
        assert!(c.censor.password);
        assert!(!c.table.index);

        let c: Config = toml::from_str("[table]\nindex = true").unwrap();
        assert!(c.table.index);

        let back: Config = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);

        let old: Config = toml::from_str("[find]\nfuzzy = true").unwrap();
        assert_eq!(old, Config::default());
        assert_eq!(old.theme.name, "0x96f");

        let c: Config = toml::from_str("[theme]\nborder = \"red\"").unwrap();
        assert_eq!(c.theme.name, "0x96f");
        assert_eq!(c.theme.border.as_deref(), Some("red"));
        let out = toml::to_string(&Config::default()).unwrap();
        assert!(out.contains("name = \"0x96f\"") && !out.contains("border"));
    }
}
