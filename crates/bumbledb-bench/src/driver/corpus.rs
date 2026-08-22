use std::path::Path;

use bumbledb::Db;

use crate::cli::CorpusArgs;
use crate::corpus;
use crate::corpus_gen::{self, GenConfig};
use crate::schema::Ledger;

use super::CorpusPaths;

pub(super) fn gen_config(corpus: &CorpusArgs) -> GenConfig {
    GenConfig {
        seed: corpus.seed,
        scale: corpus.scale,
    }
}

#[must_use]
pub fn corpus_paths(dir: &Path, cfg: GenConfig) -> CorpusPaths {
    let digest = corpus_gen::digest_hex(&corpus_gen::corpus_digest(cfg));
    let root = dir.join(&digest[..16]);
    CorpusPaths {
        db: root.join("db"),
        oracle: root.join("oracle.sqlite"),
        cal_db: root.join("cal-db"),
        cal_oracle: root.join("cal-oracle.sqlite"),
        stamp: root.join("verify.stamp"),
        root,
    }
}

const CORPUS_MARKER: &str = "corpus.ok";

/// # Errors
pub fn ensure_corpus_with(
    dir: &Path,
    cfg: GenConfig,
    load: &mut dyn FnMut(&CorpusPaths) -> Result<(), String>,
) -> Result<CorpusPaths, String> {
    let paths = corpus_paths(dir, cfg);
    if paths.root.join(CORPUS_MARKER).exists() {
        return Ok(paths);
    }
    let _ = std::fs::remove_dir_all(&paths.root);
    std::fs::create_dir_all(&paths.root)
        .map_err(|e| format!("create {}: {e}", paths.root.display()))?;
    load(&paths)?;
    std::fs::write(paths.root.join(CORPUS_MARKER), "ok").map_err(|e| format!("marker: {e}"))?;
    Ok(paths)
}

/// # Errors
pub fn ensure_corpus(dir: &Path, cfg: GenConfig) -> Result<CorpusPaths, String> {
    ensure_corpus_with(dir, cfg, &mut |paths| {
        eprintln!(
            "gen: loading corpus (seed {}, scale {}) into {}",
            cfg.seed,
            cfg.scale.label(),
            paths.root.display()
        );

        let load_dir = paths.root.join("db-load");
        let db = Db::create(&load_dir, Ledger)
            .map_err(|e| format!("create db: {e:?}"))?
            .expect("accepted");
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load bumbledb: {e:?}"))?;
        db.compact(&paths.db)
            .map_err(|e| format!("compact: {e:?}"))?;
        drop(db);
        std::fs::remove_dir_all(&load_dir).map_err(|e| format!("remove db-load: {e}"))?;
        corpus::load_sqlite(&paths.oracle, cfg).map_err(|e| format!("load sqlite: {e}"))?;

        let cal_load_dir = paths.root.join("cal-db-load");
        let cal = Db::create(&cal_load_dir, crate::calendar::Scheduling)
            .map_err(|e| format!("create cal db: {e:?}"))?
            .expect("accepted");
        crate::calendar::corpus::load_bumbledb(&cal, cfg)
            .map_err(|e| format!("load calendar: {e:?}"))?;
        cal.compact(&paths.cal_db)
            .map_err(|e| format!("compact calendar: {e:?}"))?;
        drop(cal);
        std::fs::remove_dir_all(&cal_load_dir).map_err(|e| format!("remove cal-db-load: {e}"))?;
        crate::calendar::corpus::load_sqlite(&paths.cal_oracle, cfg)
            .map_err(|e| format!("load calendar sqlite: {e}"))?;
        Ok(())
    })
}
