use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sheikah",
    about = "a quiet cli password manager for keyboard warriors",
    version,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub(crate) struct Cli {
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: Option<bool>,

    /// entry to show: service name or search text
    #[arg(value_name = "REF")]
    pub(crate) reference: Option<String>,

    /// disable colors (respects NO_COLOR)
    #[arg(short = 'P', long = "plain", global = true)]
    pub(crate) plain: bool,

    /// slate name or path/to/slate
    #[arg(short = 'S', long = "slate", global = true, value_name = "NAME", env = "SHEIKAH_SLATE")]
    pub(crate) slate: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

/// every action works as a word (`list`) or a short flag (`-l`)
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// show an entry, or only the chosen fields
    #[command(short_flag = 's', disable_help_flag = true)]
    Show(ShowArgs),

    /// copy an entry's password to the clipboard (cleared after 45s)
    #[command(short_flag = 'c', disable_help_flag = true)]
    Copy { reference: Option<String> },

    /// list all entries
    #[command(short_flag = 'l', disable_help_flag = true)]
    List {
        /// draw the table inside a box
        #[arg(short)]
        boxed: bool,
    },

    /// fuzzy pick with fzf, or substring search for QUERY
    #[command(short_flag = 'f', disable_help_flag = true)]
    Find {
        /// text to match against service, username, email, url, notes & category
        query: Option<String>,

        /// draw the table inside a box
        #[arg(short)]
        boxed: bool,
    },

    #[command(short_flag = 'a', disable_help_flag = true)]
    Add(AddArgs),

    #[command(short_flag = 'e', disable_help_flag = true)]
    Edit { reference: Option<String> },

    #[command(short_flag = 'd', disable_help_flag = true)]
    Delete { reference: Option<String> },

    /// edit the config with $EDITOR
    #[command(short_flag = 'C', disable_help_flag = true)]
    Config {
        /// slate name or path/to/slate
        slate: Option<String>,
    },

    /// create a new slate where your secrets live
    #[command(disable_help_flag = true)]
    Init,

    /// change the master password
    #[command(disable_help_flag = true)]
    Passwd,

    /// check tools & permissions
    #[command(disable_help_flag = true)]
    Doctor,

    /// print version
    #[command(disable_help_flag = true)]
    Version,
}

#[derive(Args, Default)]
pub(crate) struct ShowArgs {
    pub(crate) reference: Option<String>,

    /// only the service
    #[arg(short)]
    pub(crate) service: bool,
    /// only the username
    #[arg(short)]
    pub(crate) username: bool,
    /// only the email
    #[arg(short)]
    pub(crate) email: bool,
    /// only the password
    #[arg(short)]
    pub(crate) password: bool,
    /// only the url
    #[arg(short = 'w')]
    pub(crate) url: bool,
    /// only the notes
    #[arg(short)]
    pub(crate) notes: bool,
    /// only the category
    #[arg(short)]
    pub(crate) category: bool,
}

#[derive(Args, Default)]
pub(crate) struct AddArgs {
    /// label for site/app (netflix, proton, ..)
    #[arg(short, value_name = "STR")]
    pub(crate) service: Option<String>,

    /// category (series, mail, ..)
    #[arg(short, value_name = "STR")]
    pub(crate) category: Option<String>,

    /// domain/url
    #[arg(short, value_name = "STR")]
    pub(crate) domain: Option<String>,

    /// email
    #[arg(short, value_name = "STR")]
    pub(crate) email: Option<String>,

    /// username/login
    #[arg(short, value_name = "STR")]
    pub(crate) user: Option<String>,

    /// optional notes
    #[arg(short, value_name = "STR")]
    pub(crate) notes: Option<String>,

    /// generate a password of LEN chars instead of typing one
    #[arg(short, value_name = "LEN", num_args = 0..=1, default_missing_value = "24")]
    pub(crate) generate: Option<usize>,
}
