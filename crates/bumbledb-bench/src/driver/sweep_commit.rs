use crate::cli::SweepArgs;
use crate::sweep;

/// # Errors
pub fn cmd_sweep_commit(args: &SweepArgs) -> Result<(), String> {
    let sizes = args
        .sizes
        .clone()
        .unwrap_or_else(|| sweep::DEFAULT_SIZES.to_vec());
    let samples = args.samples.unwrap_or(sweep::DEFAULT_SAMPLES);
    let scratch = args.dir.join("sweep-commit-scratch");
    let table = sweep::run(&scratch, &sizes, samples, args.seed)?;
    print!("{table}");
    Ok(())
}
