use serde::{Deserialize, Serialize};

/// lives encrypted inside the slate; edited as toml via `sheikah config`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) censor: Censor,
    pub(crate) table: Table,
}

/// fields hidden as `•••` in `list` and `show` (explicit `show -p` etc. always reveals)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct Censor {
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
# censor: hide a field as ••• in list/show
# table.style: ascii | square | rounded | double | heavy | dashed
# table.index: number rows in list
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_partial_toml() {
        let c = Config::default();
        assert!(c.censor.password && c.censor.email && !c.censor.username);

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
    }
}
