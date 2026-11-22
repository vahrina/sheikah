use super::{parse, rejects};
use crate::args::Commands;

#[test]
fn completions_args() {
    assert!(matches!(
        parse(&["sheikah", "completions", "zsh"]).command,
        Some(Commands::Completions { .. })
    ));
    assert!(matches!(
        parse(&["sheikah", "completions", "update", "-d", "/tmp/zsh"]).command,
        Some(Commands::Completions { dir: Some(_), .. })
    ));
    assert!(rejects(&["sheikah", "completions", "powershell"]));
}

#[test]
fn bash_and_fish_scripts() {
    for (shell, needle) in [
        (clap_complete::Shell::Bash, "complete -F _sheikah"),
        (clap_complete::Shell::Fish, "complete -c sheikah"),
    ] {
        let mut out = Vec::new();
        crate::commands::completions(shell, &mut out);
        let script = String::from_utf8(out).unwrap();
        assert!(script.contains(needle) && script.contains("passwd"));
    }
}

#[test]
fn zsh_script() {
    let mut out = Vec::new();
    crate::commands::completions(clap_complete::Shell::Zsh, &mut out);
    let script = String::from_utf8(out).unwrap();
    assert!(script.starts_with("#compdef sheikah"));
    assert!(!script.contains("::reference -- entry to show"));
    assert!(script.contains("case $line[1] in"));
    assert!(script.contains("passwd") && script.contains("backup"));
    assert!(!script.contains("_default"));
    // only `export FILE` & `import FILE` complete paths
    let files = script.lines().filter(|l| l.contains("_files")).count();
    assert_eq!(files, 2);
}
