use std::path::Path;

use bumbledb::Db;

use crate::cli::CorpusArgs;
use crate::corpus;
use crate::gen::{self, GenConfig};
use crate::schema::Ledger;

use super::CorpusPaths;

pub(super) fn gen_config(corpus: &CorpusArgs) -> GenConfig {
    GenConfig {
        seed: corpus.seed,
        scale: corpus.scale,
    }
}

/// Resolves the digest-keyed directory for a corpus config (the digest
/// is the corpus identity).
#[must_use]
pub fn corpus_paths(dir: &Path, cfg: GenConfig) -> CorpusPaths {
    let digest = gen::digest_hex(&gen::corpus_digest(cfg));
    let root = dir.join(&digest[..16]);
    CorpusPaths {
        db: root.join("db"),
        oracle: root.join("oracle.sqlite"),
        stamp: root.join("verify.stamp"),
        root,
    }
}

const CORPUS_MARKER: &str = "corpus.ok";

/// [`ensure_corpus`] with an injectable loader — the reuse-logic test
/// seam (a counter hook proves the marker short-circuits regeneration).
///
/// # Errors
///
/// The loader's error; scratch I/O as a message.
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

/// Generates + loads both stores into the digest-keyed directory,
/// reusing an existing one carrying the `corpus.ok` marker (regeneration
/// is identity; the cache is convenience for L).
///
/// # Errors
///
/// Load errors as messages.
pub fn ensure_corpus(dir: &Path, cfg: GenConfig) -> Result<CorpusPaths, String> {
    ensure_corpus_with(dir, cfg, &mut |paths| {
        eprintln!(
            "gen: loading corpus (seed {}, scale {}) into {}",
            cfg.seed,
            cfg.scale.label(),
            paths.root.display()
        );
        // Load into a scratch sibling, then compact into place
        // (docs/architecture/50-storage.md): a bulk load is exactly the CoW-churn-heavy
        // case — ~40% of the loaded file is freelist — and the cached
        // corpus is write-once, so it ships live-sized.
        let load_dir = paths.root.join("db-load");
        let db = Db::create(&load_dir, Ledger).map_err(|e| format!("create db: {e:?}"))?;
        corpus::load_bumbledb(&db, cfg).map_err(|e| format!("load bumbledb: {e:?}"))?;
        db.compact(&paths.db)
            .map_err(|e| format!("compact: {e:?}"))?;
        drop(db);
        std::fs::remove_dir_all(&load_dir).map_err(|e| format!("remove db-load: {e}"))?;
        corpus::load_sqlite(&paths.oracle, cfg).map_err(|e| format!("load sqlite: {e}"))?;
        Ok(())
    })
}
