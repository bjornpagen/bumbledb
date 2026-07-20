use std::path::PathBuf;

use crate::report;

/// `lawful`: the law home-turf world — judged-law admission vs SQL
/// constraint enforcement, `SQLite`'s strong regime benched to lose
/// honestly. Runs the post-state-gated fold ([`crate::lawful::run`])
/// and writes the artifacts (`lawful.md` for humans, `lawful.json` for
/// tooling — charts pin from committed copies of the JSON).
/// Report-class: always exit 0 unless a gate (post-state divergence) or
/// setup fails.
///
/// # Errors
///
/// Everything [`crate::lawful::run`] refuses, plus artifact I/O, as
/// messages.
pub fn cmd_lawful(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    let (markdown, json) =
        crate::lawful::run(&args.dir, args.seed, args.samples, args.only.as_deref())?;
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-lawful",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("lawful.md"), &markdown).map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(out_dir.join("lawful.json"), &json).map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}
