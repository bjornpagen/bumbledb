use std::path::PathBuf;

use crate::harness::Protocol;
use crate::report;

/// # Errors
pub fn cmd_scenarios(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {

    if args.trace && !cfg!(feature = "obs") {
        return Err(super::bench::obs_missing("--trace"));
    }
    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(64),
    };

    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-scenarios",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    let modes = crate::scenarios::QueryModes {
        trace_root: args.trace.then(|| out_dir.clone()),
        alloc: args.alloc,
    };
    let (markdown, reports) =
        crate::scenarios::run(&args.dir, args.seed, proto, args.only.as_deref(), &modes)?;
    std::fs::write(out_dir.join("scenarios.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(
        out_dir.join("scenarios.json"),
        crate::scenarios::to_json(&reports, proto, args.seed),
    )
    .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    if args.trace {
        println!(
            "traces: {}",
            out_dir.join("trace").join("scenarios").display()
        );
    }
    Ok(0)
}
