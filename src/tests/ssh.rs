use super::parse;
use crate::args::Commands;

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
