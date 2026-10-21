use clap::Parser;

mod args;
mod clip;
mod commands;
mod config;
mod entry;
mod signal;
mod slate;
mod ui;

#[cfg(test)]
mod cli_tests;

fn main() {
    signal::install();
    let cli = args::Cli::parse();
    ui::init_color(cli.plain);

    if let Err(e) = commands::run(cli) {
        eprintln!("{} {e:#}", ui::ebold("error:"));
        std::process::exit(1);
    }
}
