use std::path::PathBuf;

use crate::report;

/// `crud`: the OLTP home-turf world — round-trips under matched
/// durability pairs, `SQLite`'s strong regime benched to lose honestly.
/// Runs the gated fold ([`crate::crud::run`]) and writes the artifacts
/// (`crud.md` for humans, `crud.json` for tooling — charts pin from
/// committed copies of the JSON). Report-class: always exit 0 unless a
/// gate (engine disagreement, post-state divergence) or setup fails.
///
/// # Errors
///
/// Everything [`crate::crud::run`] refuses, plus artifact I/O, as
/// messages.
pub fn cmd_crud(args: &crate::cli::ScenarioArgs) -> Result<i32, String> {
    let (markdown, json) =
        crate::crud::run(&args.dir, args.seed, args.samples, args.only.as_deref())?;
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-crud",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    std::fs::write(out_dir.join("crud.md"), &markdown).map_err(|e| format!("artifact: {e}"))?;
    std::fs::write(out_dir.join("crud.json"), &json).map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    Ok(0)
}
