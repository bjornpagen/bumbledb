use bumbledb::{Db, ResultBuffer};

use crate::cli::CorpusArgs;
use crate::harness::{self, Rotation};
use crate::schema::Ledger;
use crate::{corpus, families, trace_out};

use super::corpus::gen_config;
use super::ensure_corpus;

/// `trace`: one traced warm+cold pair for one read family — artifacts
/// only, the quick-look tool.
///
/// # Errors
///
/// Unknown family; setup errors.
pub fn cmd_trace(corpus: &CorpusArgs, family_name: &str) -> Result<(), String> {
    let cfg = gen_config(corpus);
    let family = families::all()
        .iter()
        .find(|f| f.name == family_name)
        .ok_or_else(|| {
            format!(
                "unknown family `{family_name}` (families: {})",
                families::all()
                    .iter()
                    .map(|f| f.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    let paths = ensure_corpus(&corpus.dir, cfg)?;

    // The cold half touches (commits), so trace runs on a scratch copy —
    // never the verified corpus.
    let scratch = paths.root.join("trace-scratch");
    let _ = std::fs::remove_dir_all(&scratch);
    let db = Db::create(&scratch.join("db"), Ledger).map_err(|e| format!("{e:?}"))?;
    corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;

    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = ResultBuffer::new();
    let mut run = || {
        let args = crate::families::param_args(rotation.next_set());
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    };
    for _ in 0..4 {
        run()?;
    }
    let trace_dir = paths.root.join("trace");
    let (_, events) = harness::traced_sample(&mut run)?;
    let (engine, harness_events) = trace_out::split_harness(events);
    let warm = trace_out::write_trace_file(
        &trace_dir,
        &format!("{family_name}.warm"),
        &engine,
        &harness_events,
    )
    .map_err(|e| format!("trace: {e}"))?;
    print!("{}", trace_out::FlameSummary::compute(&engine).render());
    if let Some(phases) = trace_out::render_phase_table(&engine) {
        print!("{phases}");
    }

    let (_, events) = harness::traced_cold_sample(&mut harness::org_touch(&db), &mut run)?;
    let (engine, harness_events) = trace_out::split_harness(events);
    let cold = trace_out::write_trace_file(
        &trace_dir,
        &format!("{family_name}.cold"),
        &engine,
        &harness_events,
    )
    .map_err(|e| format!("trace: {e}"))?;
    println!("traces: {} / {}", warm.display(), cold.display());
    drop(db);
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(())
}
