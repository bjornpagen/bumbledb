mod cli;
mod job;
mod lint;
mod report;
mod runner;

fn main() {
    match cli::Config::from_env().and_then(runner::run_cli) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
