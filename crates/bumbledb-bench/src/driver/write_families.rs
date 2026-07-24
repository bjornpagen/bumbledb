use std::path::Path;

use bumbledb::Db;

use crate::corpus_gen::GenConfig;
use crate::duralane::DurabilityLane;
use crate::schema::Ledger;
use crate::{clockproxy, corpus, families, harness, report, sqlite_run, writebench};

/// The write/cold families, run against a scratch corpus loaded under
/// `scratch` — bench never mutates the verified digest-dir corpus, so
/// the stamp stays honest.
///
/// The seam is typed [`DurabilityLane`], never a bare store mode: the
/// lane constructs BOTH sides — engine store and oracle pragmas — so an
/// `--ephemeral` run can no longer time `MDB_NOSYNC` against a
/// fullfsync oracle (the cross-matched pair the lane sum makes
/// unrepresentable; finding 020, `docs/architecture/61-bench-lanes.md`
/// § the ephemeral lane).
///
/// `pub(crate)` (not `pub(super)`) so the device-honesty lock test can
/// point it at a live ram disk and assert the refusal.
pub(crate) fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    lane: DurabilityLane,
) -> Result<Vec<report::WriteFamilyReport>, String> {
    // The scratch-corpus write families, table-driven: one entry per
    // family, an engine runner beside its `SQLite` mirror.
    type EngineRunner = fn(&Db<Ledger>, GenConfig) -> Result<harness::Measurement, String>;
    type OracleRunner =
        fn(&rusqlite::Connection, GenConfig) -> Result<harness::Measurement, String>;
    const PAIRED: [(&str, EngineRunner, OracleRunner); 4] = [
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
        // The delete-bearing cold lane (PRD-I2): the same walk behind a
        // delete+reinsert touch — the append lane's discriminator twin.
        (
            "cold_containment_walk_delete",
            writebench::cold_containment_walk_delete,
            sqlite_run::cold_containment_walk_delete,
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
        let db = lane.store_mode().create(&scratch.join("db"), Ledger)?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;
        let (conn, _) =
            corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg).map_err(|e| format!("{e}"))?;
        lane.configure(&conn)?;
        lane.assert_parity(&conn)?;
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
                ghz: Some(ghz.into()),
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
                ghz: Some(ghz.into()),
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
        lane.store_mode(),
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
                writebench::bulk_bumbledb(cfg, scratch, lane.store_mode())?,
                sqlite_run::bulk(cfg, scratch, lane)?,
            ))
        })?;
        out.push(report::WriteFamilyReport {
            name: "bulk".to_owned(),
            facts_per_sec: Some(harness::facts_per_sec(&ours, proto.samples)),
            ours: ours.stats,
            theirs: Some(theirs.stats),
            ghz: Some(ghz.into()),
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
