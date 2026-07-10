use crate::error::{Error, Result};
use crate::schema::{FieldId, Generation, RelationId};
use crate::storage::env::ReadTxn;
use crate::storage::keys;

use super::{SerialMark, WriteDelta};

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
        let mark = self.serial_mark(view, rel, field)?;
        let next = mark.next;
        if next == u64::MAX {
            return Err(Error::SerialExhausted {
                relation: rel,
                field,
            });
        }
        mark.next = next + 1;
        Ok(next)
    }

    /// The sequence's transaction-local mark, lazily initialized whole
    /// from the committed `Q` value (read once per transaction; the base
    /// is the dirtiness baseline).
    pub(super) fn serial_mark(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        field: FieldId,
    ) -> Result<&mut SerialMark> {
        match self.serials.entry((rel, field)) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let base = read_serial_next(view, rel, field)?;
                Ok(entry.insert(SerialMark { base, next: base }))
            }
        }
    }
}

/// Reads the committed `Q` next-value for `(relation, field)`; a missing
/// entry means the sequence has never issued a value.
fn read_serial_next(view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
    let mut buf = [0u8; keys::SERIAL_KEY_LEN];
    let len = keys::serial_key(&mut buf, rel, field);
    debug_assert_eq!(len, buf.len());
    match view.env().data().get(view.raw(), &buf[..len])? {
        Some(bytes) => crate::storage::stored_u64(bytes, "Q serial next"),
        None => Ok(0),
    }
}
