mod cli;
mod job;
mod lint;
mod report;
mod runner;

#[global_allocator]
static GLOBAL_ALLOCATOR: bumbledb_lmdb::diagnostics::TrackingAllocator<std::alloc::System> =
    bumbledb_lmdb::diagnostics::TrackingAllocator::system();

fn main() {
    match cli::Config::from_env().and_then(runner::run_cli) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
