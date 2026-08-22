use std::path::PathBuf;

use crate::report;

/// `--trace` lands each family's traced twin sample under
/// `<out>/trace/crud/<lane>/` (refused without the obs build: an artifact with
/// no spans would be a lie wearing a real name).
/// # Errors
pub fn cmd_crud(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    if args.trace && !cfg!(feature = "obs") {
        return Err(super::bench::obs_missing("--trace"));
    }

    // <out>/trace/crud/<lane>/, so the run needs the root before it

    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-crud",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    let trace_root = args.trace.then_some(out_dir.as_path());
    let (markdown, json) = crate::crud::run(
        &args.dir,
        args.seed,
        args.samples,
        args.only.as_deref(),
        trace_root,
    )?;
    std::fs::write(out_dir.join("crud.md"), &markdown).map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(out_dir.join("crud.json"), &json).map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    if args.trace {
        println!("traces: {}", out_dir.join("trace").join("crud").display());
    }
    Ok(0)
}
