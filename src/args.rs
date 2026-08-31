use clap::{Args, Parser, Subcommand};
use rpassword::prompt_password;
use std::{
    env::var,
    fs::{DirBuilder, OpenOptions, read_dir},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use crate::prompt::confirm;

#[derive(Parser)]
#[command(about = "a quick, easy & secure cli password manager")]
#[command(version, long_about = None)]
pub struct Cli {
    /// copy an entry's secret directly to clipboard
    #[arg(short, long, value_name = "REF")]
    get: Option<String>,

    /// displays a detailed list of the created entry
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
    /// create a new entry point where your secrets live
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
    },
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
    let home = std::env::var("HOME")
        .unwrap_or_else(|_| std::env::var("XDG_DATA_HOME").unwrap_or_default());
    PathBuf::from(home).join(".sheikah").join("eye.enc")
}

#[allow(dead_code)]
fn check_files_in_dir() -> PathBuf {
    let home = var("HOME").unwrap_or_else(|_| var("XDG_DATA_HOME").unwrap_or_default());
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
            many.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn passwd_prompt(_path: Option<PathBuf>) -> std::io::Result<String> {
    let prompt: &str = "master password\n> ";
    prompt_password(prompt)
}

fn change_master_passwd(path: &Path, old_pass: &str, new_pass: &str) {
    // load, verify `old_pass` against stored hash, re-encrypt, rewrite
}

pub(crate) fn run(args: Cli) {
    match &args.command {
        Some(Commands::Init { path }) => {
            let res_path = path.clone().unwrap_or_else(default_init_path);

            if res_path.exists() {
                eprintln!(
                    "refusing to overwrite existing file: {}",
                    res_path.display()
                );
                std::process::exit(1);
            }

            // create the ~/.sheikah dir
            if let Some(parent) = res_path.parent() {
                DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(parent)
                    .unwrap_or_else(|e| panic!("cannot create {}: {e}", parent.display()));

                let mode = std::fs::metadata(parent)
                    .unwrap_or_else(|e| panic!("cannot stat {}: {e}", parent.display()))
                    .permissions()
                    .mode()
                    & 0o777;

                // although chattr +a is enabled by default for now,
                // this behaviour is experimental & may be determined
                // via another flag (idk, e.g. `--enable-chattr-protection`?)
                let answer_chattr = confirm(&format!(
                    "protect '{}' from deletion via 'sudo chattr +a'?",
                    parent.display()
                ));

                if answer_chattr {
                    match std::process::Command::new("sudo")
                        .args(["chattr", "+a", &parent.to_string_lossy()])
                        .output()
                    {
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            eprintln!(
                                "created dir '{}', but chattr +a failed, thus deletion protection is NOT applied: {}",
                                parent.display(),
                                stderr.trim()
                            );
                        }

                        Err(e) => {
                            eprintln!(
                                "created dir '{}', but could not run chattr (is sudo installed?): {e}",
                                parent.display()
                            );
                        }
                    }
                } else {
                    println!("skipping chattr attribute...")
                }

                // additionally ~/.sheikah/eye.enc
                let _file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .mode(0o600)
                    .open(&res_path)
                    .unwrap_or_else(|e| panic!("cannot create {}: {e}", res_path.display()));

                println!(
                    "successfully initialized! your file lives here: {}",
                    res_path.display()
                );
            }
        }

        Some(Commands::Add(add_args)) => {
            match &add_args.service {
                Some(service) => {
                    todo!()
                    // add all args as serialized enc into `~/sheikah/eye.enc` & hash
                    // also account for an interactive mode, prompting piece by piece
                    // ^ additionally password prompt for both use cases
                }
                None => {
                    eprintln!("-s/--service is required");
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Passwd { path }) => {
            todo!();
        }
        None => {}
    }
}
