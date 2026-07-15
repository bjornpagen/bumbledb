//! The one raw-LMDB-open chokepoint. Unsafe policy (the 40-storage doc
//! amendment): this module holds the sanctioned `unsafe` of the storage
//! layer — `heed 0.22` marks environment opening unsafe (double-opening
//! one path in a process is LMDB UB) and marks env-flag setting unsafe
//! (the flags can break durability or aliasing guarantees). Both are
//! confined here; the flags are DERIVED from the store kind — no caller
//! can pass a flag, so the durable paths structurally cannot reach
//! `WRITE_MAP`/`NO_SYNC`.

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
    // readers across commits are a designed-for pattern, 40-storage).
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(MAP_SIZE)
        .max_dbs(3)
        .max_readers(MAX_READERS);
    if kind == StoreKind::Ephemeral {
        // SAFETY: WRITE_MAP|NO_SYNC trade machine-crash durability away,
        // which is the ephemeral store kind's on-disk claim
        // (docs/architecture/50-storage.md § the ephemeral store kind);
        // process-kill atomicity is preserved (the crashpoint sweep runs
        // against ephemeral stores, fuzz/tests/crash.rs). WRITE_MAP's
        // writable mapping is confined to the single-writer engine: no
        // engine surface hands out `&mut` into the map, and readers see
        // LMDB CoW pages exactly as on a durable store.
        unsafe { options.flags(EnvFlags::WRITE_MAP | EnvFlags::NO_SYNC) };
    }
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    let env = unsafe { options.open(path)? };
    Ok(env)
}
