use std::path::{Path, PathBuf};

use crate::churn;
use crate::cli;
use crate::corpus_gen::GenConfig;
use crate::report;

/// Every failure — a per-sample oracle gate mismatch, an end-state
/// disagreement, a refusal (unknown run name, RAM-backed scratch, a bad
/// schedule) — is `Err`, which `main` renders as exit 2, and the messages name
/// the failing lane or probe.
/// # Errors
pub fn cmd_churn(args: &cli::ChurnArgs) -> Result<i32, String> {
    crate::devhonesty::assert_disk_backed(&args.corpus.dir, "the timed churn lanes")
        .map_err(|refusal| refusal.to_string())?;
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-churn",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    let cfg = churn::ops::ChurnConfig {
        r#gen: GenConfig {
            seed: args.corpus.seed,
            scale: args.corpus.scale,
        },
        cycles: args.cycles,
        sample_every: args.sample_every,
        vacuum_every: args.vacuum_every,
        analyze_every: args.analyze_every,
    };

    // a refusal listing the known names — the list is the registry's,

    let registry = churn::lanes::all();
    let selected: Vec<&churn::lanes::RunSpec> = match &args.runs {
        None => registry.iter().collect(),
        Some(names) => names
            .iter()
            .map(|name| {
                registry
                    .iter()
                    .find(|spec| spec.name == name.as_str())
                    .ok_or_else(|| {
                        let known: Vec<&str> = registry.iter().map(|spec| spec.name).collect();
                        format!("unknown churn run `{name}` (runs: {})", known.join(", "))
                    })
            })
            .collect::<Result<_, _>>()?,
    };
    let scratch = out_dir.join("scratch");
    let mut runs = Vec::with_capacity(selected.len());
    for spec in selected {
        runs.push(churn::run::run_spec(spec, &cfg, &scratch)?);
    }
    let churn_report = churn::report::ChurnReport {
        provenance: report::provenance(Path::new(".")),
        config: churn::report::ConfigReport {
            scale: cfg.r#gen.scale.label(),
            seed: cfg.r#gen.seed,
            cycles: cfg.cycles,
            sample_every: cfg.sample_every,
            vacuum_every: cfg.vacuum_every,
            analyze_every: cfg.analyze_every,
        },
        runs,
    };
    churn::report::write_artifacts(&churn_report, &out_dir)
        .map_err(|e| format!("artifacts: {e}"))?;
    print!("{}", churn::report::to_markdown(&churn_report));
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}
