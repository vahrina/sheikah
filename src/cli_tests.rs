use super::*;
use crate::args::{Cli, Commands};
use clap::CommandFactory;
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
    ]);
    match cli.command {
        Some(Commands::Add(a)) => {
            assert_eq!(a.service.as_deref(), Some("github"));
            assert_eq!(a.category.as_deref(), Some("misc"));
            assert_eq!(a.user.as_deref(), Some("v"));
            assert_eq!(a.notes.as_deref(), Some("funny note"));
            assert!(!a.recovery);
        }
        _ => std::panic!("expected Add command"),
    }
    for args in [["sheikah", "add", "-r"], ["sheikah", "-a", "--recovery"]] {
        match parse(&args).command {
            Some(Commands::Add(a)) => assert!(a.recovery),
            _ => std::panic!("expected Add command"),
        }
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
    match parse(&["sheikah", "-f", "net"]).command {
        Some(Commands::Find { query }) => assert_eq!(query.as_deref(), Some("net")),
        _ => std::panic!("expected Find"),
    }
    assert!(matches!(
        parse(&["sheikah", "-b"]).command,
        Some(Commands::Backup)
    ));
}

#[test]
fn ssh_passes_args_through() {
    match parse(&[
        "sheikah", "ssh", "-p", "2222", "pi", "-t", "htop", "--", "rpi",
    ])
    .command
    {
        Some(Commands::Ssh { args }) => {
            assert_eq!(args, ["-p", "2222", "pi", "-t", "htop", "--", "rpi"]);
        }
        _ => std::panic!("expected ssh"),
    }
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
fn zsh_completions() {
    assert!(matches!(
        parse(&["sheikah", "completions", "zsh"]).command,
        Some(Commands::Completions { .. })
    ));
    assert!(Cli::try_parse_from(["sheikah", "completions", "fish"]).is_err());

    let mut out = Vec::new();
    crate::commands::completions(clap_complete::Shell::Zsh, &mut out);
    let script = String::from_utf8(out).unwrap();
    assert!(script.starts_with("#compdef sheikah"));
    assert!(!script.contains("::reference -- entry to show"));
    assert!(script.contains("case $line[1] in"));
    assert!(script.contains("passwd") && script.contains("backup"));
    assert!(!script.contains("_default") && !script.contains("_files"));
}

#[test]
fn password_prompt() {
    let conf = ConfigBuilder::new()
        .input_data("cat")
        .output_discard()
        .build();
    assert_eq!(read_password_with_config(conf).unwrap(), "cat");
}
