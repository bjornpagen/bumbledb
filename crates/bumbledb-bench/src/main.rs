//! `bumbledb-bench`: argument parsing plus dispatch — every capability
//! lives in the library.

use bumbledb_bench::cli;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Cmd::Help) => print!("{}", cli::help()),
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::help());
            std::process::exit(2);
        }
    }
}
