use heed::{RoTxn, WithoutTls};

use crate::error::Result;

use super::{Environment, ReadTxn, WriteTxn};

impl Environment {
    /// Begins a read snapshot. The underlying LMDB transaction is the

    /// # Errors

    pub fn read_txn(&self) -> Result<ReadTxn<'_>> {
        Ok(self.resume_read_txn(self.env.clone().static_read_txn()?))
    }

    pub(crate) fn resume_read_txn(&self, txn: RoTxn<'static, WithoutTls>) -> ReadTxn<'_> {
        ReadTxn {
            env: self,
            txn,
            generation: std::cell::OnceCell::new(),
        }
    }

    /// Begins the write transaction (LMDB admits one writer at a time).

    /// # Errors

    pub fn write_txn(&self) -> Result<WriteTxn<'_>> {
        Ok(WriteTxn {
            env: self,
            txn: self.env.write_txn()?,
        })
    }
}
