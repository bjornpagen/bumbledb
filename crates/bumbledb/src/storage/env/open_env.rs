//! The one raw-LMDB-open chokepoint. Unsafe policy (the 00-product
//! allowlist, boundary category): this module holds the sanctioned `unsafe` of the storage
//! layer — `heed 0.22` marks environment opening unsafe (double-opening
//! one path in a process is LMDB UB) and marks env-flag setting unsafe
//! (the flags can break durability or aliasing guarantees). Both are
//! confined here; the flags are DERIVED from the store kind — no caller
//! can pass a flag, so the durable paths structurally cannot reach
//! `NO_SYNC`. (Cleanup-0.5.0 ruling 1 retired `WRITE_MAP` from the
//! ephemeral flag set — the recorded fallback,
//! `docs/architecture/50-storage.md` § the ephemeral store kind — and
//! with it the capacity contract's two preallocation `unsafe` sites
//! that lived below.)

use std::path::Path;

use heed::{EnvFlags, EnvOpenOptions, WithoutTls};

use crate::error::Result;

use super::{MAP_SIZE, MAX_READERS, StoreKind};

/// Opens the raw LMDB environment at `path`, with the environment flags
/// the store kind dictates and nothing else.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
pub(super) fn open_env(path: &Path, kind: StoreKind) -> Result<heed::Env<WithoutTls>> {
    // MDB_NOTLS: reader slots belong to transaction objects, not threads —
    // a thread may pin an old snapshot while opening new ones (long-lived
    // readers across commits are a designed-for pattern, 50-storage).
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(MAP_SIZE)
        .max_dbs(3)
        .max_readers(MAX_READERS);
    // PRD-C1 gravestone — `MDB_NOMEMINIT` on the durable flag set,
    // measured NEUTRAL, not taken (the retired C1 heed-flags packet,
    // git history). The twin armed `EnvFlags::NO_MEM_INIT`
    // right here for the durable kind only (the ephemeral kind then
    // ran `WRITE_MAP`, where LMDB ignores `NOMEMINIT` — writes landed
    // in the map, no malloc'd write buffer existed to zero; ruling 1
    // has since retired WRITE_MAP) and ran the full oracle green (2862
    // verify cases), so semantics were untouched. The interleaved
    // same-session A/B (scripts/measure.sh, twin binaries alternated,
    // 3 reps per arm, fresh scratch per rep, min-of-3, scale S) read
    // NEUTRAL everywhere, base → twin p50: commit_single 5.02 → 5.00
    // ms (−0.5%), commit_witnessed 5.13 → 5.06 ms (−1.2%),
    // commit_batch 24.04 → 24.26 ms (+0.9%), bulk 1.210 → 1.209 s
    // (−0.05%; −0.9% min) — all F_FULLFSYNC-bound — and the
    // durable-read spot-check point 395 → 398 ns (+0.8%), range 18.4 →
    // 18.4 µs (0.0%), warm-cache tier, proxy-clean. Every family
    // inside the ±2% band. Mechanism: durable commits are
    // fsync-barrier-dominated and bulk is hash+tree-build-dominated,
    // so LMDB's write-buffer memset is noise at every regime measured;
    // the flag buys nothing and the shipped durable flag set stays
    // exactly as derived above.
    if kind == StoreKind::Ephemeral {
        // SAFETY: NO_SYNC trades machine-crash durability away, which
        // is the ephemeral store kind's on-disk claim
        // (docs/architecture/50-storage.md § the ephemeral store kind);
        // process-kill atomicity is preserved (verified by the ephemeral
        // crashpoint sweep while it lived — the sweep died with the
        // fuzzing apparatus, docs/architecture/60-validation.md § the
        // deletion record) — commits still
        // pwrite through LMDB's ordinary path, they only skip the fsync
        // boundary, so no writable mapping and no aliasing hazard exists.
        unsafe { options.flags(EnvFlags::NO_SYNC) };
    }
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    let env = unsafe { options.open(path)? };
    Ok(env)
}
