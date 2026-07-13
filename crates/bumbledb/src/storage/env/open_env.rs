use std::path::Path;

use heed::{EnvOpenOptions, WithoutTls};

use crate::error::Result;

use super::{MAP_SIZE, MAX_READERS};

/// Opens the raw LMDB environment at `path`.
///
/// This is the single sanctioned `unsafe` block outside `exec::kernel`
/// (the 40-storage doc amendment): `heed 0.22` marks environment opening unsafe because
/// opening one environment path twice in a process is LMDB UB.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
pub(super) fn open_env(path: &Path) -> Result<heed::Env<WithoutTls>> {
    // MDB_NOTLS: reader slots belong to transaction objects, not threads —
    // a thread may pin an old snapshot while opening new ones (long-lived
    // readers across commits are a designed-for pattern, 40-storage).
    let mut options = EnvOpenOptions::new().read_txn_without_tls();
    options
        .map_size(MAP_SIZE)
        .max_dbs(3)
        .max_readers(MAX_READERS);
    // SAFETY: bumbledb opens each environment through exactly this function,
    // and heed itself refuses (Error::EnvAlreadyOpened) to open a path that
    // is already open in this process, upholding LMDB's single-open rule.
    let env = unsafe { options.open(path)? };
    Ok(env)
}
