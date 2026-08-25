use clap::{
    Args,
    Parser,
    Subcommand
};
use std::{
    path::PathBuf,
    os::unix::fs::OpenOptionsExt,
    env::var,
    fs::{
        OpenOptions,
        read_dir,
    },
};

#[derive(Parser)]
#[command(about = "a quick, easy & secure cli password manager")]
#[command(version, long_about = None)]
pub struct Cli {
    /// copy an entry's secret directly to clipboard
    #[arg(short, long, value_name = "REF")]
    get: Option<String>,

    /// displays a detailed list of the crated entry
    #[arg(short, long, value_name = "REF")]
    show: Option<String>,

    /// list all available entries
    #[arg(short, long)]
    list: bool,

    /// edit an existing entry
    #[arg(short, long, value_name = "REF")]
    edit: Option<String>,

    /// delete an existing entry
    #[arg(short, long, value_name = "REF")]
    delete: Option<String>,

    /// an overview of how to use various commands
    #[arg(short, long)]
    flags: bool,

    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// crate a new entry point where your secrets live
    Init {
        #[arg(long, value_name = "FILE")]
        path: Option<PathBuf>,
    },

    /// add a new entry with creds, interactively or via args
    Add(AddArgs),

    /// change master password (omit --path if you only have one secret file)
    Passwd {
        #[arg(long, value_name = "FILE")]
        path: Option<PathBuf>,
    }
}

#[derive(Args)]
pub(crate) struct AddArgs {
    /// label for site/app (netflix, proton, ..)
    #[arg(short, long, value_name = "STR")]
    pub(crate) service: Option<String>,

    /// category (series, mail, ..)
    #[arg(short, long, value_name = "STR")]
    pub(crate) category: Option<String>,

    /// domain/url
    #[arg(short, long, value_name = "STR")]
    pub(crate) domain: Option<String>,

    /// email
    #[arg(short, long, value_name = "STR")]
    pub(crate) email: Option<String>,

    /// username/login
    #[arg(short, long, value_name = "STR")]
    pub(crate) user: Option<String>,

    /// optional notes
    #[arg(short, long, value_name = "STR")]
    pub(crate) notes: Option<String>,

    /*
    * password will always be prompted after either interactive or args are done
    * avoid putting as arg at all cost! will land in shell history & `/proc/<pid>/cmdline`
    * remember to put `-p` arg into the `edit` option
    */
}

fn default_init_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| std::env::var("XDG_HOME").unwrap_or_default());
    PathBuf::from(home).join(".sheikah").join("heirloom.json")
}

#[allow(dead_code)]
fn check_files_in_dir() -> PathBuf {
    let home = var("HOME").unwrap_or_else(|_| var("XDG_HOME").unwrap_or_default());
    let dir = PathBuf::from(home).join(".sheikah");

    let entries: Vec<PathBuf> = read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();

    match entries.as_slice() {
        [only] => only.clone(),
        [] => panic!("no secret file present in {}", dir.display()),
        many => panic!(
            "more than 1 secret file present: {}\npick one with the --path flag",
            many.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        ),
    }
}

pub(crate) fn run(args: Cli) {
    match &args.command {
        Some(Commands::Init { path }) => {
            let res_path = path.clone().unwrap_or_else(default_init_path);

            if res_path.exists() {
                eprintln!("refusing to overwrite existing file: {}", res_path.display());
                std::process::exit(1);
            }

            if let Some(parent) = res_path.parent() {
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|e| panic!("cannot crate {}: {e}", parent.display()));
            }

            let _file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&res_path)
                .unwrap_or_else(|e| panic!("cannot crate {}: {e}", res_path.display()));

            println!("initialized new secret file at {}", res_path.display());
        }

        Some(Commands::Add(add_args)) => {
            match &add_args.service {
                Some(service) => {
                    todo!()
                    // add all args as serialized json into `~/sheikah/heirloom.json` & hash
                    // also account for an interactive mode, prompting piece by piece
                    // ^ additionally password prompt for both use cases
                }
                None => {
                    eprintln!("-s/--service is required");
                    std::process::exit(1);
                }
            }
        },

        Some(Commands::Passwd { path }) => {
            todo!();
        },
        None => {},
    }
}
