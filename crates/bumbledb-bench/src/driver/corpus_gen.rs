use bumbledb::Db;
use rusqlite::Connection;

use crate::cli::CorpusArgs;
use crate::schema::Ledger;
use crate::{corpus, verify};

use super::corpus::gen_config;
use super::{CASES_FILE, ensure_corpus};

/// `gen`.
///
/// # Errors
///
/// As [`ensure_corpus`].
pub fn cmd_gen(corpus: &CorpusArgs) -> Result<(), String> {
    let paths = ensure_corpus(&corpus.dir, gen_config(corpus))?;
    println!("corpus ready: {}", paths.root.display());
    Ok(())
}

/// `verify`: the oracle against the digest directory, stamp inside it.
/// Returns the process exit code (1 on mismatch).
///
/// # Errors
///
/// Setup errors as messages (mismatches are an exit code, not an error —
/// the bundles are the artifact).
pub fn cmd_verify(corpus: &CorpusArgs, cases: u32) -> Result<i32, String> {
    let cfg = gen_config(corpus);
    let paths = ensure_corpus(&corpus.dir, cfg)?;
    let db = Db::open(&paths.db, Ledger).map_err(|e| format!("open db: {e:?}"))?;
    let conn = Connection::open(&paths.oracle).map_err(|e| format!("open oracle: {e}"))?;
    corpus::configure_sqlite(&conn).map_err(|e| format!("configure oracle: {e}"))?;
    let cal_db = Db::open(&paths.cal_db, crate::calendar::Scheduling)
        .map_err(|e| format!("open calendar db: {e:?}"))?;
    let cal_conn =
        Connection::open(&paths.cal_oracle).map_err(|e| format!("open calendar oracle: {e}"))?;
    corpus::configure_sqlite(&cal_conn).map_err(|e| format!("configure calendar oracle: {e}"))?;
    let vcfg = verify::VerifyConfig {
        corpus_gen: cfg,
        random_cases: cases,
        out_dir: paths.root.clone(),
    };
    match verify::run_prepared(&vcfg, &db, &conn, &cal_db, &cal_conn, |_| None) {
        Ok(report) => {
            std::fs::write(paths.root.join(CASES_FILE), cases.to_string())
                .map_err(|e| format!("cases sidecar: {e}"))?;
            println!("verify OK: {} cases, stamp {}", report.cases, report.stamp);
            Ok(0)
        }
        Err(failure) => {
            eprint!("{failure}");
            Ok(1)
        }
    }
}
