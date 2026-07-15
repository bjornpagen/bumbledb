use std::path::Path;

use bumbledb::Db;

use crate::corpus_gen::GenConfig;
use crate::schema::Ledger;
use crate::{clockproxy, corpus, families, harness, report, sqlite_run, writebench};

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn facts_per_sec(m: &harness::Measurement, samples: u32) -> f64 {
    let total_secs = (m.stats.mean_ns * u64::from(samples)) as f64 / 1e9;
    m.work as f64 / total_secs.max(f64::EPSILON)
}

/// The write/cold families, run against a scratch corpus loaded under
/// `scratch` — bench never mutates the verified digest-dir corpus, so
/// the stamp stays honest.
///
/// `pub(crate)` (not `pub(super)`) so the device-honesty lock test can
/// point it at a live ram disk and assert the refusal.
pub(crate) fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
) -> Result<Vec<report::WriteFamilyReport>, String> {
    // The scratch-corpus write families, table-driven: one entry per
    // family, an engine runner beside its `SQLite` mirror.
    type EngineRunner = fn(&Db<Ledger>, GenConfig) -> Result<harness::Measurement, String>;
    type OracleRunner =
        fn(&rusqlite::Connection, GenConfig) -> Result<harness::Measurement, String>;
    const PAIRED: [(&str, EngineRunner, OracleRunner); 3] = [
        (
            "commit_single",
            writebench::commit_single_bumbledb,
            sqlite_run::commit_single,
        ),
        (
            "commit_batch",
            writebench::commit_batch_bumbledb,
            sqlite_run::commit_batch,
        ),
        (
            "cold_containment_walk",
            writebench::cold_containment_walk,
            sqlite_run::cold_containment_walk,
        ),
    ];

    // The device-honesty rule (docs/architecture/60-validation.md): the
    // timed write families are fsync-bound, so a RAM-backed scratch
    // would report a number physics never signed. Checked before any
    // store exists; the verify/differential/fuzz lanes are exempt (they
    // check answers, not clocks).
    crate::devhonesty::assert_disk_backed(scratch, "the timed write families")
        .map_err(|refusal| refusal.to_string())?;

    let mut out = Vec::new();
    if PAIRED.iter().any(|(name, ..)| selected(name)) || selected("commit_witnessed") {
        eprintln!("bench: loading the scratch write corpus");
        let db = Db::create(&scratch.join("db"), Ledger).map_err(|e| format!("{e:?}"))?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;
        let (conn, _) =
            corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg).map_err(|e| format!("{e}"))?;
        for (name, engine, oracle) in PAIRED {
            if !selected(name) {
                continue;
            }
            eprintln!("bench: {name}");
            let ((ours, theirs), ghz) =
                clockproxy::stamped(|| Ok((engine(&db, cfg)?, oracle(&conn, cfg)?)))?;
            out.push(report::WriteFamilyReport {
                name: name.to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(super::ghz_report(ghz)),
            });
        }
        // The witnessed-write row (the PRD-18 spine debt): engine-only —
        // SQLite has no snapshot-witness surface, so the row is
        // unpaired by decision (an emulation would time the emulation).
        if selected("commit_witnessed") {
            eprintln!("bench: commit_witnessed");
            let (ours, ghz) =
                clockproxy::stamped(|| writebench::commit_witnessed_bumbledb(&db, cfg))?;
            out.push(report::WriteFamilyReport {
                name: "commit_witnessed".to_owned(),
                ours: ours.stats,
                theirs: None,
                facts_per_sec: None,
                ghz: Some(super::ghz_report(ghz)),
            });
        }
    }
    // The window-judgment lane (the roster extension): its own twin
    // scratch worlds, engine-only rows — after the ledger commit rows
    // (same fsync-bound class), before bulk (which stays last).
    out.extend(crate::windowed::write_families(
        cfg,
        &scratch.join("windowed"),
        selected,
    )?);

    // bulk stays LAST: seconds of fsync — nothing
    // may measure after it in this process.
    if selected("bulk") {
        eprintln!("bench: bulk");
        let proto = families::write_families()
            .iter()
            .find(|f| f.name == "bulk")
            .expect("registered")
            .protocol;
        let ((ours, theirs), ghz) = clockproxy::stamped(|| {
            Ok((
                writebench::bulk_bumbledb(cfg, scratch)?,
                sqlite_run::bulk(cfg, scratch)?,
            ))
        })?;
        out.push(report::WriteFamilyReport {
            name: "bulk".to_owned(),
            facts_per_sec: Some(facts_per_sec(&ours, proto.samples)),
            ours: ours.stats,
            theirs: Some(theirs.stats),
            ghz: Some(super::ghz_report(ghz)),
        });
    }
    // The write-order pin (measured): bulk's seconds of fsync
    // leave the deepest clock shadow — nothing measures after it.
    debug_assert!(
        out.iter()
            .position(|w| w.name == "bulk")
            .is_none_or(|i| i == out.len() - 1),
        "bulk must be the last write family"
    );
    Ok(out)
}
