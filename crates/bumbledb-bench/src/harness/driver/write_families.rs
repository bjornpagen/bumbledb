use std::path::Path;

use bumbledb::Db;

use crate::harness;
use crate::harness::{report, sqlite_run};
use crate::worlds::corpus_gen::GenConfig;
use crate::worlds::ledger::Ledger;
use crate::worlds::{corpus, families, writebench};

pub(super) fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
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

    let mut out = Vec::new();
    if PAIRED.iter().any(|(name, ..)| selected(name)) || selected("commit_witnessed") {
        eprintln!("bench: loading the scratch write corpus");
        let db = crate::harness::create_db(&scratch.join("db.bdb"), Ledger)?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;
        let (conn, _) =
            corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg).map_err(|e| format!("{e}"))?;
        crate::harness::sqlite_run::configure_durable(&conn)?;
        crate::harness::sqlite_run::assert_durable_parity(&conn)?;
        for (name, engine, oracle) in PAIRED {
            if !selected(name) {
                continue;
            }
            eprintln!("bench: {name}");
            let ours = engine(&db, cfg)?;
            let theirs = oracle(&conn, cfg)?;
            out.push(report::WriteFamilyReport {
                name: name.to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
            });
        }

        if selected("commit_witnessed") {
            eprintln!("bench: commit_witnessed");
            let ours = writebench::commit_witnessed_bumbledb(&db, cfg)?;
            out.push(report::WriteFamilyReport {
                name: "commit_witnessed".to_owned(),
                ours: ours.stats,
                theirs: None,
                facts_per_sec: None,
            });
        }
    }

    // scratch worlds, engine-only rows — after the ledger commit rows
    // (same fsync-bound class), before insert_stream (which stays last).
    out.extend(crate::worlds::windowed::write_families(
        cfg,
        &scratch.join("windowed"),
        selected,
    )?);

    out.extend(crate::worlds::capacity::write_families(
        cfg,
        &scratch.join("capacity"),
        selected,
    )?);

    // The insertion stream runs last: no family follows its fsync-heavy window.
    if selected("insert_stream") {
        eprintln!("bench: insert_stream");
        let proto = families::write_families()
            .iter()
            .find(|f| f.name == "insert_stream")
            .expect("registered")
            .protocol;
        let ours = writebench::insert_stream_bumbledb(cfg, scratch)?;
        let theirs = sqlite_run::insert_stream(cfg, scratch)?;
        out.push(report::WriteFamilyReport {
            name: "insert_stream".to_owned(),
            facts_per_sec: Some(harness::facts_per_sec(&ours, proto.samples)),
            ours: ours.stats,
            theirs: Some(theirs.stats),
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
