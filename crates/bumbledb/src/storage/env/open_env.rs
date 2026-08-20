//! The one raw-LMDB-open chokepoint. Unsafe policy (the 00-product
//! allowlist, boundary category): this module holds the sanctioned `unsafe` of the storage
//! layer — `heed 0.22` marks environment opening unsafe (double-opening
//! one path in a process is LMDB UB) and marks env-flag setting unsafe
//! (the flags can break durability or aliasing guarantees). Both are
//! confined here; the flags are DERIVED from the open lane
//! ([`OpenLane`]) — no caller can pass a flag, so the durable paths
//! structurally cannot reach `NO_SYNC` except through the bench-only
//! [`OpenLane::Nosync`] arm.

use std::path::Path;

use heed::{EnvFlags, EnvOpenOptions, WithoutTls};

use crate::error::Result;

use super::{MAP_SIZE, MAX_READERS};

/// Which surface is opening the environment — the flags are DERIVED
/// from this one value, so no caller can pass a flag
/// (`docs/architecture/50-storage.md`; the lock law is a writer law).
#[derive(Clone, Copy)]
pub(super) enum OpenLane {
    /// The writing constructors (`Db` handles): plain LMDB flags.
    Write,
    /// Bench-only NOSYNC. Not a store kind. Hidden `Db` constructors
    /// are the only callers.
    Nosync,
}

/// Opens the raw LMDB environment at `path`, with the environment flags
/// the open lane dictates and nothing else.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
pub(super) fn open_env(path: &Path, lane: OpenLane) -> Result<heed::Env<WithoutTls>> {
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
    // right here for the durable kind only and ran the full oracle
    // green, so semantics were untouched. The interleaved
    // same-session A/B (scripts/measure.sh, twin binaries alternated,
    // 3 reps per arm, fresh scratch per rep, min-of-3, scale S) read
    // NEUTRAL everywhere. Mechanism: durable commits are
    // fsync-barrier-dominated and bulk is hash+tree-build-dominated,
    // so LMDB's write-buffer memset is noise at every regime measured;
    // the flag buys nothing and the shipped durable flag set stays
    // exactly as derived above.
    match lane {
        OpenLane::Nosync => {
            // SAFETY: NO_SYNC trades machine-crash durability away.
            // Process-kill atomicity is preserved — commits still
            // pwrite through LMDB's ordinary path, they only skip the fsync
            // boundary, so no writable mapping and no aliasing hazard exists.
            // The only callers are the bench-hidden constructors.
            unsafe { options.flags(EnvFlags::NO_SYNC) };
        }
        OpenLane::Write => {}
    }
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    //
    // An OS-level failure here is the `Io` refusal, never an `Lmdb`
    // diagnosis: the writing lanes meet the path at the lock file,
    // which maps its failures the same way.
    let env = unsafe { options.open(path) }.map_err(|err| match err {
        heed::Error::Io(io) => crate::error::Error::from(io),
        other => crate::error::Error::from(other),
    })?;
    Ok(env)
}
