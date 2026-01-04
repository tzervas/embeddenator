use embeddenator_cli;
use clap::Parser;
use std::process;

fn main() {
    let cli = embeddenator_cli::Cli::parse();
    if let Err(e) = embeddenator_cli::run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
