use super::*;
use crate::args::{Cli, Commands};
use rpassword::{ConfigBuilder, read_password_with_config};

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(args).unwrap()
}

#[test]
fn add_with_all_args() {
    let cli = parse(&[
        "sheikah",
        "add",
        "-s",
        "github",
        "-c",
        "misc",
        "-u",
        "v",
        "-n",
        "funny note",
        "-g",
    ]);
    match cli.command {
        Some(Commands::Add(a)) => {
            assert_eq!(a.service.as_deref(), Some("github"));
            assert_eq!(a.category.as_deref(), Some("misc"));
            assert_eq!(a.user.as_deref(), Some("v"));
            assert_eq!(a.notes.as_deref(), Some("funny note"));
            assert_eq!(a.generate, Some(24));
        }
        _ => std::panic!("expected Add command"),
    }
}

#[test]
fn actions_as_word_short_and_long() {
    assert!(Cli::try_parse_from(["sheikah", "--copy", "x"]).is_err());
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
        Some(Commands::List { boxed: true })
    ));
    match parse(&["sheikah", "-s", "github", "-up"]).command {
        Some(Commands::Show(a)) => {
            assert_eq!(a.reference.as_deref(), Some("github"));
            assert!(a.username && a.password && !a.email);
        }
        _ => std::panic!("expected Show"),
    }
    match parse(&["sheikah", "-fb", "net"]).command {
        Some(Commands::Find { query, boxed }) => {
            assert_eq!(query.as_deref(), Some("net"));
            assert!(boxed);
        }
        _ => std::panic!("expected Find"),
    }
}

#[test]
fn help_and_version_without_long_flags() {
    use clap::error::ErrorKind;
    let kind = |args: &[&str]| Cli::try_parse_from(args).err().map(|e| e.kind());
    assert_eq!(kind(&["sheikah", "-h"]), Some(ErrorKind::DisplayHelp));
    assert_eq!(
        kind(&["sheikah", "list", "-h"]),
        Some(ErrorKind::DisplayHelp)
    );
    assert_eq!(kind(&["sheikah", "-V"]), Some(ErrorKind::DisplayVersion));
    assert!(matches!(
        parse(&["sheikah", "version"]).command,
        Some(Commands::Version)
    ));
    for long in ["--help", "--version"] {
        assert!(
            Cli::try_parse_from(["sheikah", long])
                .is_err_and(|e| e.kind() == ErrorKind::UnknownArgument)
        );
    }
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
fn password_prompt() {
    let conf = ConfigBuilder::new()
        .input_data("cat")
        .output_discard()
        .build();
    assert_eq!(read_password_with_config(conf).unwrap(), "cat");
}
