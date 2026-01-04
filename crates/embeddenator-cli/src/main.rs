//! Main entry point for embeddenator-cli binary

use embeddenator_cli::run;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
