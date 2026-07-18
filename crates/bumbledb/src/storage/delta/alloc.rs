use crate::error::{Error, Result};
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use bumbledb_theory::schema::{FieldId, RelationId};

use super::{FreshMark, WriteDelta};

impl WriteDelta<'_> {
    /// Mints the next fresh value for a `Fresh`-generation field: reads
    /// `Q` once per `(relation, field)` per transaction, then increments in
    /// memory. A minted value that escapes to the host is burned even when
    /// the transaction aborts (the escaped high-water flushes on every
    /// abort path — `commit`'s reject/infra exits, and `Db::write`'s
    /// `EscapedIdBurn` drop guard for the closure region, which covers
    /// the `Err`-returning and the PANICKING closure alike): the
    /// generator never re-issues an id it handed out, the transaction's
    /// fate irrelevant. Only the abort's data and generation stay
    /// untouched, never the sequence.
    ///
    /// # Errors
    ///
    /// `FreshExhausted` when the sequence reaches `u64::MAX`; `FactShape`
    /// from the sequence init's generation check (the dyn boundary's
    /// foreign-witness refusal — see [`WriteDelta::fresh_mark`]); `Lmdb`
    /// on a failed `Q` read.
    pub fn alloc(&mut self, view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
        let mark = self.fresh_mark(view, rel, field)?;
        let next = mark.next;
        if next == u64::MAX {
            return Err(Error::FreshExhausted {
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
    ///
    /// The lazy init is the dyn boundary's foreign-witness refusal
    /// (`70-api.md` § ETL): the schema-bound [`crate::FreshField`] makes
    /// a cross-schema witness a compile error, but `Db<SchemaDescriptor>`
    /// handles share one typestate, so another descriptor's witness can
    /// reach `alloc` well-typed. The generation check here refuses it
    /// typed before any `Q` key is touched — priced once per
    /// `(relation, field)` per transaction beside the `Q` read it
    /// precedes, zero on the steady-state mint (an occupied mark exists
    /// only because this check, or the schema-derived insert advance,
    /// admitted the pair). The typed lane's `Fresh` constants and the
    /// insert advance's schema-derived pairs pass vacuously.
    pub(super) fn fresh_mark(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        field: FieldId,
    ) -> Result<&mut FreshMark> {
        match self.marks.entry((rel, field)) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.schema.check_fresh_field(rel, field)?;
                let base = read_fresh_next(view, rel, field)?;
                Ok(entry.insert(FreshMark { base, next: base }))
            }
        }
    }
}

/// Reads the committed `Q` next-value for `(relation, field)`; a missing
/// entry means the sequence has never issued a value.
fn read_fresh_next(view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
    let mut buf = [0u8; keys::FRESH_KEY_LEN];
    let len = keys::fresh_key(&mut buf, rel, field);
    debug_assert_eq!(len, buf.len());
    match view.env().data().get(view.raw(), &buf[..len])? {
        Some(bytes) => crate::storage::stored_u64(bytes, "Q fresh next"),
        None => Ok(0),
    }
}
