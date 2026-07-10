use std::path::Path;

use bumbledb::Db;

use crate::gen::GenConfig;
use crate::schema::Ledger;
use crate::{clockproxy, corpus, families, harness, report, sqlite_run, writebench};

#[allow(clippy::cast_precision_loss)]
fn facts_per_sec(m: &harness::Measurement, samples: u32) -> f64 {
    let total_secs = (m.stats.mean_ns * u64::from(samples)) as f64 / 1e9;
    m.work as f64 / total_secs.max(f64::EPSILON)
}

/// The write/cold families, run against a scratch corpus loaded under
/// `scratch` — bench never mutates the verified digest-dir corpus, so
/// the stamp stays honest.
pub(super) fn write_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
) -> Result<Vec<report::WriteFamilyReport>, String> {
    let mut out = Vec::new();
    let commit_selected = selected("commit_single") || selected("commit_batch");
    let cold_selected = selected("cold_containment_walk");

    if commit_selected || cold_selected {
        eprintln!("bench: loading the scratch write corpus");
        let db = Db::create(&scratch.join("db"), Ledger).map_err(|e| format!("{e:?}"))?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("{e:?}"))?;
        let (conn, _) =
            corpus::load_sqlite(&scratch.join("oracle.sqlite"), cfg).map_err(|e| format!("{e}"))?;
        if selected("commit_single") {
            eprintln!("bench: commit_single");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::commit_single_bumbledb(&db, cfg)?,
                    sqlite_run::commit_single(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "commit_single".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
        if selected("commit_batch") {
            eprintln!("bench: commit_batch");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::commit_batch_bumbledb(&db, cfg)?,
                    sqlite_run::commit_batch(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "commit_batch".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
        if cold_selected {
            eprintln!("bench: cold_containment_walk");
            let ((ours, theirs), ghz) = clockproxy::stamped(|| {
                Ok((
                    writebench::cold_containment_walk(&db, cfg)?,
                    sqlite_run::cold_containment_walk(&conn, cfg)?,
                ))
            })?;
            out.push(report::WriteFamilyReport {
                name: "cold_containment_walk".to_owned(),
                ours: ours.stats,
                theirs: Some(theirs.stats),
                facts_per_sec: None,
                ghz: Some(write_ghz(ghz)),
            });
        }
    }
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
            ghz: Some(write_ghz(ghz)),
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

/// A write family's block is guarded as one bracket over both engines.
fn write_ghz(stamp: clockproxy::GhzStamp) -> report::GhzReport {
    report::GhzReport {
        pre: stamp.pre,
        post: stamp.post,
        retried: stamp.retried,
        contaminated: stamp.contaminated(),
    }
}
