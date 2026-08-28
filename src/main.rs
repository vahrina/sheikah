use clap::Parser;
mod args;
mod prompt;
mod codec;
use crate::args::Cli;

#[cfg(test)]
mod cli_tests; // derive from `cli_tests.rs`

fn main() {
    let args = Cli::parse();
    args::run(args);
}
