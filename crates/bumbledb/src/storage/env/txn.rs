use heed::{RoTxn, WithoutTls};

use crate::error::Result;

use super::{Environment, ReadTxn, WriteTxn};

impl Environment {
    /// Begins a read snapshot. The underlying LMDB transaction is the
    /// `'static` form (the heed env is `Arc`-backed and rides inside it)
    /// so [`Db`](crate::Db)'s reader cache can hold one across calls —
    /// the per-read `mdb_txn_begin` was the point path's last fixed
    /// cost.
    ///
    /// # Errors
    ///
    /// `Lmdb` on transaction failure (e.g. reader-slot exhaustion).
    pub fn read_txn(&self) -> Result<ReadTxn<'_>> {
        Ok(self.resume_read_txn(self.env.clone().static_read_txn()?))
    }

    /// Wraps an already-open raw read transaction (the reader cache's
    /// resume path): a fresh generation cell, same snapshot.
    pub(crate) fn resume_read_txn(&self, txn: RoTxn<'static, WithoutTls>) -> ReadTxn<'_> {
        ReadTxn {
            env: self,
            txn,
            generation: std::cell::OnceCell::new(),
        }
    }

    /// Begins the write transaction (LMDB serializes writers).
    ///
    /// # Errors
    ///
    /// `Lmdb` on transaction failure.
    pub fn write_txn(&self) -> Result<WriteTxn<'_>> {
        Ok(WriteTxn {
            env: self,
            txn: self.env.write_txn()?,
        })
    }
}
