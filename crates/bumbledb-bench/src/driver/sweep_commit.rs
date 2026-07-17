//! `sweep-commit`: the CLI wrapper around the T8 commit-size sweep
//! ([`crate::sweep`]) — fresh ephemeral windowed twins per cell, the
//! judgment spans per commit size, the delta-order/key-sorted probe
//! A/B. A timing command: run it under `scripts/measure.sh` like every
//! other measurement.

use crate::cli::SweepArgs;
use crate::sweep;

/// `sweep-commit`. Prints the per-commit-size table to stdout.
///
/// # Errors
///
/// Refusals — a non-obs build, out-of-range knobs, the hash-model
/// drift — and engine errors, each naming the remedy.
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
