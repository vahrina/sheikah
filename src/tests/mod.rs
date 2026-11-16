use crate::args::Cli;
use clap::Parser;

mod actions;
mod add;
mod completions;
mod ssh;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(args).unwrap()
}

fn rejects(args: &[&str]) -> bool {
    Cli::try_parse_from(args).is_err()
}
