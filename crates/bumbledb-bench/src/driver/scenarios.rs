use std::path::PathBuf;

use crate::harness::Protocol;
use crate::report;

/// `scenarios`: the non-ledger worlds — load, oracle-gate, time, and
/// write the artifacts (`scenarios.md` for humans, `scenarios.json` for
/// tooling — charts pin from committed copies of the JSON). Report-class:
/// always exit 0 unless a gate (engine disagreement) or setup fails.
///
/// # Errors
///
/// Setup failures and oracle disagreements, as messages.
pub fn cmd_scenarios(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(64),
    };
    let (markdown, reports) =
        crate::scenarios::run(&args.dir, args.seed, proto, args.only.as_deref())?;
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-scenarios",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("scenarios.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(
        out_dir.join("scenarios.json"),
        crate::scenarios::to_json(&reports, proto, args.seed),
    )
    .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}
