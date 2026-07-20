//! The `churn` driver command: registry selection, the device-honesty
//! gate, the per-run driver loop, and the series artifacts. This module
//! itself never times anything — [`crate::churn::run`] owns the clock,
//! and every timed NUMBER arrives only via the owner's night session.

use std::path::{Path, PathBuf};

use crate::churn;
use crate::cli;
use crate::corpus_gen::GenConfig;
use crate::report;

/// `churn`: the long-lived degradation lanes — every selected registry
/// run driven end to end ([`churn::run::run_spec`]), the series report
/// written as `churn-report.json` + `churn-report.md`.
///
/// Report-class exit semantics: completing = `Ok(0)` — nothing here
/// gates a claim. Every failure — a per-sample oracle gate mismatch, an
/// end-state disagreement, a refusal (unknown run name, RAM-backed
/// scratch, a bad schedule) — is `Err`, which `main` renders as exit 2,
/// and the messages name the failing lane or probe.
///
/// # Errors
///
/// Refusals and gate disagreements, each naming the offending lane,
/// probe, or knob.
pub fn cmd_churn(args: &cli::ChurnArgs) -> Result<i32, String> {
    // The device-honesty rule is symmetric: churn times reads AND
    // writes against its scratch, so the scratch root refuses a
    // RAM-backed volume exactly like every timed lane.
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
    // Selection against the registry's own data: an unknown run name is
    // a refusal listing the known names — the list is the registry's,
    // never a literal.
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

#[cfg(test)]
mod tests {
    use crate::cli::{ChurnArgs, CorpusArgs};
    use crate::corpus_gen::Scale;

    use super::cmd_churn;

    /// The end-to-end smoke: the nosync run (ephemeral stores, S-scale
    /// load, 4 cycles — seconds, not measurement) completes with exit 0
    /// and both series artifacts land in the out dir, the JSON one
    /// parsing through the crate's own parser.
    #[test]
    fn churn_cmd_nosync_smoke_writes_artifacts() {
        let scratch = std::env::temp_dir().join("bumbledb-bench-churn-cmd-scratch");
        let out = std::env::temp_dir().join("bumbledb-bench-churn-cmd-out");
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&out);
        let args = ChurnArgs {
            corpus: CorpusArgs {
                scale: Scale::S,
                seed: 1,
                dir: scratch.clone(),
            },
            cycles: 4,
            sample_every: 2,
            vacuum_every: 2,
            analyze_every: 2,
            runs: Some(vec!["nosync".into()]),
            out: Some(out.clone()),
        };
        let code = cmd_churn(&args).expect("the smoke run completes");
        assert_eq!(code, 0, "report-class: completing is exit 0");
        let json_path = out.join("churn-report.json");
        assert!(json_path.exists(), "the JSON series artifact exists");
        let text = std::fs::read_to_string(&json_path).expect("readable");
        crate::json::parse(&text).expect("the written artifact parses");
        assert!(
            out.join("churn-report.md").exists(),
            "the markdown artifact exists"
        );
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = std::fs::remove_dir_all(&out);
    }
}
