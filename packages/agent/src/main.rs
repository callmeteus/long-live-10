//! `ll10-agent` command line entry point.

fn main() {
    if let Err(error) = agent::cli::run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
