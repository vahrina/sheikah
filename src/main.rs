use clap::Parser;

mod args;
mod askpass;
mod clip;
mod commands;
mod config;
mod entry;
mod signal;
mod slate;
mod theme;
mod ui;

#[cfg(test)]
mod cli_tests;

fn main() {
    signal::install();
    if let Some(dir) = askpass::invoked() {
        askpass::respond(&dir);
    }
    let cli = args::Cli::parse();
    ui::init_color(cli.plain);

    if let Err(e) = commands::run(cli) {
        eprintln!("{} {e:#}", ui::eerror("error:"));
        std::process::exit(1);
    }
}
