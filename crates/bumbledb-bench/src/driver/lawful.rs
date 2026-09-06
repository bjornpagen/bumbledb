use std::path::PathBuf;

use crate::report;

/// # Errors
pub fn cmd_lawful(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-lawful",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;

    let (markdown, json) =
        crate::lawful::run(&args.dir, args.seed, args.samples, args.only.as_deref())?;
    std::fs::write(out_dir.join("lawful.md"), &markdown).map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(out_dir.join("lawful.json"), &json).map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}
