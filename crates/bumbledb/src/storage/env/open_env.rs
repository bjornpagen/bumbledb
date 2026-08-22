//! (the flags can break durability or aliasing guarantees). Both are
//! confined here; the flags are DERIVED from the open lane
//! ([`OpenLane`]) — no caller can pass a flag, so the durable paths
//! structurally cannot reach `NO_SYNC` except through the bench-only
//! [`OpenLane::Nosync`] arm.
//! The one raw-LMDB-open chokepoint. Unsafe policy (the 00-product
//! allowlist, boundary category): this module holds the sanctioned `unsafe` of the storage
//! layer — `heed 0.22` marks environment opening unsafe (double-opening
//! one path in a process is LMDB UB) and marks env-flag setting unsafe

use std::path::Path;

use heed::{EnvFlags, EnvOpenOptions, WithoutTls};

use crate::error::Result;

use super::{MAP_SIZE, MAX_READERS};

#[derive(Clone, Copy)]
pub(super) enum OpenLane {
    /// The writing constructors (`Db` handles): plain LMDB flags.
    Write,

    Nosync,
}

#[expect(
    unsafe_code,
    reason = "the localized unsafe operations have documented safety invariants"
)]
pub(super) fn open_env(path: &Path, lane: OpenLane) -> Result<heed::Env<WithoutTls>> {
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(MAP_SIZE)
        .max_dbs(3)
        .max_readers(MAX_READERS);

    // so LMDB's write-buffer memset is noise at every regime measured;

    match lane {
        OpenLane::Nosync => {
            // SAFETY: NO_SYNC trades machine-crash durability away.

            // pwrite through LMDB's ordinary path, they only skip the fsync

            unsafe { options.flags(EnvFlags::NO_SYNC) };
        }
        OpenLane::Write => {}
    }
    // SAFETY: bumbledb opens each environment through exactly this function,

    // is already open in this process, upholding LMDB's single-open rule.

    // An OS-level failure here is the `Io` refusal, never an `Lmdb`

    let env = unsafe { options.open(path) }.map_err(|err| match err {
        heed::Error::Io(io) => crate::error::Error::from(io),
        other => crate::error::Error::from(other),
    })?;
    Ok(env)
}
