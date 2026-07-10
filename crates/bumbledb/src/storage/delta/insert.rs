use crate::encoding::{decode_u64, fact_hash, field_bytes};
use crate::error::Result;
use crate::schema::{FieldId, Generation, RelationId};
use crate::storage::env::ReadTxn;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records an insert, netted against committed state
    /// (docs/architecture/50-storage.md): committed + no pending entry is a
    /// redundant insert and records nothing; a pending `Delete` proves the
    /// fact committed, so the insert *cancels* it (net no-op — the entry is
    /// removed, never overwritten); only a genuinely absent fact records
    /// `Insert`. Returns whether the final state changes.
    ///
    /// Any serial field values the fact carries advance the in-memory
    /// high-water mark past them — explicit values are legal on the normal
    /// write path (`10-data-model.md`), and mixed explicit/generated
    /// allocation tracks the running maximum.
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed membership probe.
    pub fn insert(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<bool> {
        // Advancing BEFORE the no-op determination is sound — a no-op
        // insert can never dirty a mark. Invariant: the committed `Q`
        // high-water covers every committed serial value, because every
        // path that commits a fact ran this same advance (or `alloc`) at
        // that fact's original commit — the normal write path, bulk-load
        // chunks (`Db::bulk_load` chunks route through here), and
        // explicit resupply after a delete (a genuine insert again) —
        // and marks never retreat (`mark.max(value + 1)`; deletes do not
        // touch `Q`). So a no-op insert's serial values are already
        // below their committed bases: the advance lands exactly on the
        // base, the mark stays clean, and a pure-no-op transaction never
        // triggers the counters-only commit (pinned by
        // `commit/tests/commit.rs`,
        // `a_pure_noop_transaction_touches_neither_tx_id_nor_q_marks`).
        self.advance_serial_marks(view, rel, fact_bytes)?;
        let hash = fact_hash(fact_bytes);
        match self.facts.get(&(rel, hash)).copied() {
            Some((_, Disposition::Insert)) => Ok(false),
            Some((slice, Disposition::Delete)) => {
                // The pending Delete proves the fact committed: cancel it.
                self.facts.remove(&(rel, hash));
                self.record_guards(rel, fact_bytes, Some(slice));
                *self.row_count_delta.entry(rel).or_insert(0) += 1;
                Ok(true)
            }
            None => {
                if crate::storage::read::fact_row_by_hash(view, rel, &hash)?.is_some() {
                    return Ok(false); // committed: a redundant insert.
                }
                let slice = self.arena.alloc(fact_bytes);
                self.facts.insert((rel, hash), (slice, Disposition::Insert));
                self.record_guards(rel, fact_bytes, Some(slice));
                *self.row_count_delta.entry(rel).or_insert(0) += 1;
                Ok(true)
            }
        }
    }

    /// Effective membership: the delta's disposition if present, else an `M`
    /// probe against the read view (committed state).
    pub(super) fn present(
        &self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        hash: &[u8; 32],
    ) -> Result<bool> {
        if let Some((_, disposition)) = self.facts.get(&(rel, *hash)) {
            return Ok(*disposition == Disposition::Insert);
        }
        Ok(crate::storage::read::fact_row_by_hash(view, rel, hash)?.is_some())
    }

    /// Advances serial marks past any serial-field values the fact carries
    /// (the layout knows the offsets; serial fields are always U64).
    fn advance_serial_marks(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let relation = self.schema.relation(rel);
        for (idx, field) in relation.fields().iter().enumerate() {
            if field.generation != Generation::Serial {
                continue;
            }
            let field_id =
                FieldId(u16::try_from(idx).expect("validated schema: field ids fit u16"));
            let raw = field_bytes(fact_bytes, relation.layout(), idx);
            let value = decode_u64(raw.try_into().expect("serial fields are 8 bytes"));
            let mark = self.serial_mark(view, rel, field_id)?;
            // `saturating_add`: an explicit u64::MAX is legal to insert; the
            // sequence is then exhausted for the generator (alloc errors).
            mark.next = mark.next.max(value.saturating_add(1));
        }
        Ok(())
    }
}
