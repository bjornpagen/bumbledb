//! `bumbledb-bench`: argument parsing plus dispatch — every capability
//! lives in the library. Exit codes: 0 ok / gates won; 1 verify mismatch
//! or gate loss; 2 usage or refusal (each refusal names the remedy).

use bumbledb_bench::{cli, driver, families};

fn dispatch(cmd: &cli::Cmd) -> Result<i32, String> {
    match cmd {
        cli::Cmd::Help => {
            print!("{}", cli::help());
            Ok(0)
        }
        cli::Cmd::Queries => {
            print!("{}", families::render_queries_md());
            Ok(0)
        }
        cli::Cmd::Gen(corpus) => driver::cmd_gen(corpus).map(|()| 0),
        cli::Cmd::Verify { corpus, cases } => driver::cmd_verify(corpus, *cases),
        cli::Cmd::Bench(args) => driver::cmd_bench(args),
        cli::Cmd::Trace { corpus, family } => driver::cmd_trace(corpus, family).map(|()| 0),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match cli::parse(&args) {
        Ok(cmd) => cmd,
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::help());
            std::process::exit(2);
        }
    };
    match dispatch(&cmd) {
        Ok(code) => std::process::exit(code),
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    }
}
