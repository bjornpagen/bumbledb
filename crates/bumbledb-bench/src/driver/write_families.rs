use std::path::Path;

use bumbledb::Db;

use crate::corpus_gen::GenConfig;
use crate::duralane::DurabilityLane;
use crate::schema::Ledger;
use crate::{clockproxy, corpus, families, harness, report, sqlite_run, writebench};

/// `pub(crate)` (not `pub(super)`) so the device-honesty lock test can point it
/// at a live ram disk and assert the refusal.
#[expect(
    clippy::too_many_lines,
    reason = "one lane list, ordered by the fsync-shadow rule — splitting would hide the order"
)]
pub(crate) fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    lane: DurabilityLane,
) -> Result<Vec<report::WriteFamilyReport>, String> {
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
        (
            "cold_containment_walk_delete",
            writebench::cold_containment_walk_delete,
            sqlite_run::cold_containment_walk_delete,
        ),
    ];

    // Refuse RAM-backed durable timings before loading a scratch corpus.
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
            let ((ours, theirs, boundary), ghz) = clockproxy::stamped(|| {
                let ours = engine(&db, cfg)?;
                // One untimed observation separates the engine blocks.
                // Keep the original outer stamp, including its flag.
                let boundary = clockproxy::effective_ghz();
                let theirs = oracle(&conn, cfg)?;
                Ok((ours, theirs, boundary))
            })?;
            let (ghz_ours, ghz_theirs) = ghz.split_at(boundary);
            out.push(report::WriteFamilyReport {
                name: name.to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(ghz.into()),
                ghz_ours: Some(ghz_ours.into()),
                ghz_theirs: Some(ghz_theirs.into()),
            });
        }

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
                ghz_ours: Some(ghz.into()),
                ghz_theirs: None,
            });
        }
    }

    // scratch worlds, engine-only rows — after the ledger commit rows
    // (same fsync-bound class), before insert_stream (which stays last).
    out.extend(crate::windowed::write_families(
        cfg,
        &scratch.join("windowed"),
        selected,
        lane.store_mode(),
    )?);

    out.extend(crate::capacity::write_families(
        cfg,
        &scratch.join("capacity"),
        selected,
        lane.store_mode(),
    )?);

    // The insertion stream runs last: no family follows its fsync-heavy window.
    if selected("insert_stream") {
        eprintln!("bench: insert_stream");
        let proto = families::write_families()
            .iter()
            .find(|f| f.name == "insert_stream")
            .expect("registered")
            .protocol;
        let ((ours, theirs, boundary), ghz) = clockproxy::stamped(|| {
            let ours = writebench::insert_stream_bumbledb(cfg, scratch, lane.store_mode())?;
            // Runners include their own untimed preseed/teardown. This is
            // block attribution, not an in-sample frequency measurement.
            let boundary = clockproxy::effective_ghz();
            let theirs = sqlite_run::insert_stream(cfg, scratch, lane)?;
            Ok((ours, theirs, boundary))
        })?;
        let (ghz_ours, ghz_theirs) = ghz.split_at(boundary);
        out.push(report::WriteFamilyReport {
            name: "insert_stream".to_owned(),
            facts_per_sec: Some(harness::facts_per_sec(&ours, proto.samples)),
            ours: ours.stats,
            theirs: Some(theirs.stats),
            ghz: Some(ghz.into()),
            ghz_ours: Some(ghz_ours.into()),
            ghz_theirs: Some(ghz_theirs.into()),
        });
    }

    // Preserve that ordering if the family roster changes.
    debug_assert!(
        out.iter()
            .position(|w| w.name == "insert_stream")
            .is_none_or(|i| i == out.len() - 1),
        "insert_stream must be the last write family"
    );
    Ok(out)
}
