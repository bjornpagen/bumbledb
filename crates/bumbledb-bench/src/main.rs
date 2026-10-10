//! Exit codes: 0 ok / gates won; 1 verify mismatch or gate loss; 2 usage or
//! refusal (each refusal names the remedy).
use bumbledb_bench::cli;
use bumbledb_bench::harness::{appperf, boost, driver, lanes, micro, report};
use bumbledb_bench::worlds::families;

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
        cli::Cmd::VerifyStore(corpus) => driver::cmd_verify_store(corpus),
        cli::Cmd::Bench(args) => driver::cmd_bench(args),
        cli::Cmd::Profile(args) => driver::cmd_profile(args).map(|()| 0),
        cli::Cmd::Scenarios(args) => driver::cmd_scenarios(args),
        cli::Cmd::Crud(args) => driver::cmd_crud(args),
        cli::Cmd::Lawful(args) => driver::cmd_lawful(args),
        cli::Cmd::Storage(args) => lanes::storage::run(args),
        cli::Cmd::Writes(args) => lanes::writes::run(args),
        cli::Cmd::Curves(args) => lanes::curves::run(args),
        cli::Cmd::Heap(args) => lanes::heap::run(args),
        cli::Cmd::Micro(args) => micro::run(args),
        cli::Cmd::MicroCompare { old, new } => micro::compare(old, new).map(|markdown| {
            print!("{markdown}");
            0
        }),
        cli::Cmd::AppPerf(args) => appperf::run(args),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (globals, cmd) = match cli::parse_invocation(&args) {
        Ok(invocation) => invocation,
        Err(message) => {
            eprintln!("error: {message}\n");
            eprint!("{}", cli::help());
            std::process::exit(2);
        }
    };
    if let Some(jobs) = globals.jobs {
        report::record_parallel_jobs(jobs);
    }
    if globals.boost
        && cmd.runs_measurements()
        && let Err(message) = boost::engage()
    {
        eprintln!("error: {message}");
        std::process::exit(2);
    }
    match dispatch(&cmd) {
        Ok(code) => std::process::exit(code),
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
        }
    }
}
