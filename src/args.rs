use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, ValueHint};

#[derive(Parser)]
#[command(
    name = "sheikah",
    about = "a quiet cli password manager for keyboard warriors",
    version,
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    version: Option<bool>,

    /// entry to show: service name or search text
    #[arg(value_name = "REF", value_hint = ValueHint::Other)]
    pub(crate) reference: Option<String>,

    /// disable colors (respects NO_COLOR)
    #[arg(short = 'P', long = "plain", global = true)]
    pub(crate) plain: bool,

    /// slate name (or unique prefix) or path/to/slate
    #[arg(
        short = 'S',
        long = "slate",
        global = true,
        value_name = "NAME",
        value_hint = ValueHint::Other,
        env = "SHEIKAH_SLATE"
    )]
    pub(crate) slate: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

/// every action works as a word (`list`) or a short flag (`-l`)
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// create a new slate where your secrets live
    #[command(disable_help_flag = true)]
    Init,

    /// check tools, permissions & health
    #[command(disable_help_flag = true)]
    Doctor,

    /// add an entry interactively or via args
    #[command(short_flag = 'a', disable_help_flag = true)]
    Add(AddArgs),

    /// edit an entry
    #[command(short_flag = 'e', disable_help_flag = true)]
    Edit {
        #[arg(value_hint = ValueHint::Other)]
        reference: Option<String>,
    },

    /// delete an entry
    #[command(short_flag = 'd', disable_help_flag = true)]
    Delete {
        #[arg(value_hint = ValueHint::Other)]
        reference: Option<String>,
    },

    /// show an entry, or only the chosen fields
    #[command(short_flag = 's', disable_help_flag = true)]
    Show(ShowArgs),

    /// copy an entry's password to the clipboard (cleared after 45s by default)
    #[command(short_flag = 'c', disable_help_flag = true)]
    Copy {
        #[arg(value_hint = ValueHint::Other)]
        reference: Option<String>,
    },

    /// list all entries
    #[command(short_flag = 'l', disable_help_flag = true)]
    List {
        /// draw the table inside a box
        #[arg(short)]
        boxed: bool,
    },

    /// fuzzy pick an entry, optionally seeded with QUERY
    #[command(short_flag = 'f', disable_help_flag = true)]
    Find {
        /// text to match against service, username & category
        #[arg(value_hint = ValueHint::Other)]
        query: Option<String>,
    },

    /// open the config with your preferred $EDITOR
    #[command(short_flag = 'C', disable_help_flag = true)]
    Config {
        /// slate name or path/to/slate
        #[arg(value_hint = ValueHint::Other)]
        slate: Option<String>,
    },

    /// change the master password
    #[command(disable_help_flag = true)]
    Passwd,

    /// pick one of the last saves & restore it
    #[command(short_flag = 'b', disable_help_flag = true)]
    Backup,

    /// print a shell completion script to stdout
    #[command(disable_help_flag = true)]
    Completions { shell: CompletionShell },

    /// print this message or the help of the given subcommand(s)
    #[command(disable_help_flag = true)]
    Help {
        #[arg(allow_hyphen_values = true, value_hint = ValueHint::Other)]
        command: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CompletionShell {
    Zsh,
}

#[derive(Args, Default)]
pub(crate) struct ShowArgs {
    #[arg(value_hint = ValueHint::Other)]
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
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) service: Option<String>,

    /// category (series, mail, ..)
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) category: Option<String>,

    /// domain/url
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) domain: Option<String>,

    /// email
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) email: Option<String>,

    /// username/login
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) user: Option<String>,

    /// optional notes
    #[arg(short, value_name = "STR", value_hint = ValueHint::Other)]
    pub(crate) notes: Option<String>,

    /// store recovery codes instead of a password
    #[arg(short = 'r', long)]
    pub(crate) recovery: bool,
}
