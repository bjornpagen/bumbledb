use crate::encoding::{decode_u64, fact_hash, field_bytes};
use crate::error::Result;
use crate::schema::{FieldId, Generation, RelationId};
use crate::storage::env::ReadTxn;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records an insert. Returns whether the final state changes (an
    /// idempotent no-op if the fact is already present).
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
        self.advance_serial_marks(view, rel, fact_bytes)?;
        let hash = fact_hash(fact_bytes);
        let changed = !self.present(view, rel, &hash)?;
        if changed {
            let slice = self.arena.alloc(fact_bytes);
            self.facts.insert((rel, hash), (slice, Disposition::Insert));
            self.record_guards(rel, fact_bytes, Some(slice));
            *self.row_count_delta.entry(rel).or_insert(0) += 1;
        }
        Ok(changed)
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
            let mark = match self.serial_next.get(&(rel, field_id)).copied() {
                Some(mark) => mark,
                None => self.serial_base_of(view, rel, field_id)?,
            };
            // `saturating_add`: an explicit u64::MAX is legal to insert; the
            // sequence is then exhausted for the generator (alloc errors).
            self.serial_next
                .insert((rel, field_id), mark.max(value.saturating_add(1)));
        }
        Ok(())
    }
}
