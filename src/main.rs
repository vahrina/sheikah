use clap::Parser;
mod args;
use crate::args::Cli;

#[cfg(test)]
mod cli_tests; // derive from `cli_tests.rs`

fn main() {
    let args = Cli::parse();
    args::run(args);
}
