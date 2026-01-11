use embeddenator::cli;
use embeddenator_obs::logging;
use std::process;

fn main() {
    logging::init();
    if let Err(e) = cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
