use super::{parse, rejects};
use crate::args::Commands;
use crate::entry::EntryType;

#[test]
fn add_with_all_args() {
    let cli = parse(&[
        "sheikah",
        "add",
        "-s",
        "github",
        "-T",
        "login",
        "-t",
        "git, Dev",
        "-u",
        "v",
        "-n",
        "funny note",
    ]);
    match cli.command {
        Some(Commands::Add(a)) => {
            assert_eq!(a.service.as_deref(), Some("github"));
            assert_eq!(a.kind, Some(EntryType::Login));
            assert_eq!(a.tags.as_deref(), Some("git, Dev"));
            assert_eq!(a.user.as_deref(), Some("v"));
            assert_eq!(a.notes.as_deref(), Some("funny note"));
        }
        _ => std::panic!("expected Add command"),
    }
}

#[test]
fn add_type_flag() {
    for (args, want) in [
        (["sheikah", "add", "-T", "recovery"], EntryType::Recovery),
        (["sheikah", "-a", "--type", "ssh"], EntryType::Ssh),
    ] {
        match parse(&args).command {
            Some(Commands::Add(a)) => assert_eq!(a.kind, Some(want)),
            _ => std::panic!("expected Add command"),
        }
    }
}

#[test]
fn add_rejects_retired_flags_and_types() {
    assert!(rejects(&["sheikah", "add", "-r"]));
    assert!(rejects(&["sheikah", "add", "--recovery"]));
    assert!(rejects(&["sheikah", "add", "-T", "api"]));
    assert!(rejects(&["sheikah", "add", "-T", "backup"]));
    assert!(rejects(&["sheikah", "add", "-T", "custom"]));
}
