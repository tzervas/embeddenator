use std::process;

fn main() {
    if let Err(e) = embeddenator_cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
