use super::{parse, rejects};
use crate::args::{Cli, Commands};
use clap::CommandFactory;

#[test]
fn actions_as_word_short_and_long() {
    assert!(rejects(&["sheikah", "--copy", "x"]));
    for args in [["sheikah", "copy", "x"], ["sheikah", "-c", "x"]] {
        match parse(&args).command {
            Some(Commands::Copy { reference }) => assert_eq!(reference.as_deref(), Some("x")),
            _ => std::panic!("expected Copy"),
        }
    }
}

#[test]
fn combined_short_flags() {
    assert!(matches!(
        parse(&["sheikah", "-lb"]).command,
        Some(Commands::List {
            boxed: true,
            tag: _
        })
    ));
    match parse(&["sheikah", "-s", "github", "-up"]).command {
        Some(Commands::Show(a)) => {
            assert_eq!(a.reference.as_deref(), Some("github"));
            assert!(a.username && a.password && !a.email);
        }
        _ => std::panic!("expected Show"),
    }
    match parse(&["sheikah", "list", "--tag", "git", "--tag", "dev"]).command {
        Some(Commands::List { tag, .. }) => assert_eq!(tag, ["git", "dev"]),
        _ => std::panic!("expected List"),
    }
    match parse(&["sheikah", "-f", "net"]).command {
        Some(Commands::Find { query, tag }) => {
            assert_eq!(query.as_deref(), Some("net"));
            assert!(tag.is_empty());
        }
        _ => std::panic!("expected Find"),
    }
    assert!(matches!(
        parse(&["sheikah", "-b"]).command,
        Some(Commands::Backup)
    ));
}

#[test]
fn help_by_word_or_flag() {
    for args in [["sheikah", "help", "show"], ["sheikah", "help", "-s"]] {
        match parse(&args).command {
            Some(Commands::Help { command }) => assert!(command.is_some()),
            _ => std::panic!("expected Help"),
        }
    }
    let help = Cli::command().render_help().to_string();
    assert!(help.contains("print this message or the help of the given subcommand(s)"));
    assert!(!help.contains("Print this message"));
}

#[test]
fn positional_ref_and_config() {
    let cli = parse(&["sheikah", "github"]);
    assert_eq!(cli.reference.as_deref(), Some("github"));
    assert!(cli.command.is_none());

    assert!(parse(&["sheikah"]).command.is_none());
    for args in [["sheikah", "-C", "work"], ["sheikah", "config", "work"]] {
        match parse(&args).command {
            Some(Commands::Config { slate }) => assert_eq!(slate.as_deref(), Some("work")),
            _ => std::panic!("expected Config"),
        }
    }
}

#[test]
fn export_import_args() {
    assert!(matches!(
        parse(&["sheikah", "export", "out.csv"]).command,
        Some(Commands::Export { file }) if file == "out.csv"
    ));
    assert!(rejects(&["sheikah", "export"]));
    assert!(matches!(
        parse(&["sheikah", "import", "bw.json"]).command,
        Some(Commands::Import { file }) if file == "bw.json"
    ));
    assert!(rejects(&["sheikah", "import"]));
}

#[test]
fn lock_args() {
    assert!(matches!(
        parse(&["sheikah", "lock"]).command,
        Some(Commands::Lock)
    ));
    assert!(parse(&["sheikah", "-S", "work", "lock"]).slate.is_some());
    assert!(rejects(&["sheikah", "lock", "extra"]));
}
