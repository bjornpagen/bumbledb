use crate::error::{Error, Result};
use crate::schema::{FieldId, Generation, RelationId};
use crate::storage::env::ReadTxn;
use crate::storage::keys;

use super::WriteDelta;

impl WriteDelta<'_> {
    /// Mints the next serial value for a `Serial`-generation field: reads
    /// `Q` once per `(relation, field)` per transaction, then increments in
    /// memory. Aborted transactions never touch the committed sequence.
    ///
    /// # Errors
    ///
    /// `SerialExhausted` when the sequence reaches `u64::MAX`; `Lmdb` on a
    /// failed `Q` read.
    pub fn alloc(&mut self, view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
        // Both callers are proof-carrying — the macro-generated `Serial`
        // newtypes on the typed path, the `SerialField` witness on the
        // dynamic path — so the assert documents the invariant; no
        // boundary re-checks it.
        debug_assert_eq!(
            self.schema.relation(rel).field(field).generation,
            Generation::Serial,
            "alloc on a non-serial field is a programmer error"
        );
        let next = match self.serial_next.get(&(rel, field)).copied() {
            Some(next) => next,
            None => self.serial_base_of(view, rel, field)?,
        };
        if next == u64::MAX {
            return Err(Error::SerialExhausted {
                relation: rel,
                field,
            });
        }
        self.serial_next.insert((rel, field), next + 1);
        Ok(next)
    }

    /// The committed `Q` value for a sequence, read once per transaction
    /// and remembered as the dirtiness baseline.
    pub(super) fn serial_base_of(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        field: FieldId,
    ) -> Result<u64> {
        if let Some(base) = self.serial_base.get(&(rel, field)) {
            return Ok(*base);
        }
        let base = read_serial_next(view, rel, field)?;
        self.serial_base.insert((rel, field), base);
        Ok(base)
    }
}

/// Reads the committed `Q` next-value for `(relation, field)`; a missing
/// entry means the sequence has never issued a value.
fn read_serial_next(view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
    let mut buf = [0u8; keys::SERIAL_KEY_LEN];
    let len = keys::serial_key(&mut buf, rel, field);
    debug_assert_eq!(len, buf.len());
    match view.env().data().get(view.raw(), &buf[..len])? {
        Some(bytes) => Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "Q serial next",
            ))
        })?)),
        None => Ok(0),
    }
}
