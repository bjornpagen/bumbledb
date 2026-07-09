use crate::error::Result;
use crate::storage::env::ReadTxn;

/// The `_data` DBI's total entry count (`mdb_stat` — O(1), read off the
/// B-tree metadata, no scan). The DBI spans every namespace (F/M/U/R/Q/S),
/// so for any one relation the count *over-approximates* its `F` rows —
/// which is exactly what the reopen-trust ceiling wants: a witness no
/// in-band counter can inflate, bounding a claimed `S` row count before
/// it sizes an allocation (`docs/architecture/50-storage.md`).
///
/// # Errors
///
/// `Lmdb` on storage failure.
pub fn data_entries(txn: &ReadTxn<'_>) -> Result<u64> {
    let stat = txn.env().data().stat(txn.raw())?;
    Ok(u64::try_from(stat.entries).expect("64-bit usize"))
}
