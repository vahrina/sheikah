use super::*;

#[test]
fn add_with_all_args() {
    let cli = Cli::try_parse_from([
        "sheikah", "add",
        "-s", "github",
        "-c", "misc",
        "-u", "v",
        "-n", "funny note"
    ]).unwrap();

    match cli.command {
        Some(Commands::Add(a)) => {
            assert_eq!(a.service.as_deref(), Some("github"));
            assert_eq!(a.category.as_deref(), Some("misc"));
            assert_eq!(a.user.as_deref(), Some("v"));
            assert_eq!(a.notes.as_deref(), Some("funny note"));
        }
        _ => std::panic!("expected Add command"),
    }
}

#[test]
fn init_with_path() {
    let cli = Cli::try_parse_from([
        "sheikah", "init", "--path", "/tmp/foo"
    ]).unwrap();

    match cli.command {
        Some(Commands::Init { path }) => {
            assert_eq!(path, Some(PathBuf::from("/tmp/foo")));
        }
        _ => std::panic!("expected Init command"),
    }
}

#[test]
fn conflicting_or_bad_args_err() {
    let cli = Cli::try_parse_from([
        "sheikah", "--deadbeef"
    ]);

    assert!(cli.is_err());
}
