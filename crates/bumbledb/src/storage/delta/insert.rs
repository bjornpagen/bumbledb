use crate::encoding::{decode_u64, fact_hash, field_word_bytes};
use crate::error::Result;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::{FieldId, Generation, RelationId};

use super::{DeltaEffect, Disposition, WriteDelta};

impl WriteDelta<'_> {
    #[cfg(test)]
    pub fn insert(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<DeltaEffect> {
        self.apply(view, rel, fact_bytes, Disposition::Insert)
    }

    /// Fresh marks advance on insert before the no-op determination —

    /// commit — the write path and explicit resupply after a delete —
    /// and marks never retreat (`mark.max(value + 1)`; deletes do not

    /// # Errors

    pub fn apply(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
        want: Disposition,
    ) -> Result<DeltaEffect> {
        if want == Disposition::Insert {
            self.advance_fresh_marks(view, rel, fact_bytes)?;
        }
        let hash = fact_hash(fact_bytes);
        let sign = match want {
            Disposition::Insert => 1,
            Disposition::Delete => -1,
        };
        match self.facts.get(&(rel, hash)).copied() {
            Some((_, have)) if have == want => Ok(DeltaEffect::NoOp),
            Some((slice, _)) => {
                self.facts.remove(&(rel, hash));
                self.cancel_determinants(rel, fact_bytes, slice);
                *self.row_count_delta.entry(rel).or_insert(0) += sign;
                Ok(DeltaEffect::Cancelled)
            }
            None => {
                let present = self.present(view, rel, &hash)?;
                let redundant = match want {
                    Disposition::Insert => present,
                    Disposition::Delete => !present,
                };
                if redundant {
                    return Ok(DeltaEffect::NoOp);
                }
                let slice = self.arena.alloc(fact_bytes);
                self.facts.insert((rel, hash), (slice, want));
                self.record_determinants(rel, fact_bytes, slice, want);
                *self.row_count_delta.entry(rel).or_insert(0) += sign;
                Ok(DeltaEffect::Recorded)
            }
        }
    }

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

    fn advance_fresh_marks(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let relation = self.schema.relation(rel);
        for (idx, field) in relation.fields().iter().enumerate() {
            if field.generation != Generation::Fresh {
                continue;
            }
            let field_id = FieldId(u16::try_from(idx).expect("field count fits u16"));
            let value = decode_u64(field_word_bytes(relation.layout().encoded(fact_bytes), idx));
            let mark = self.fresh_mark(view, rel, field_id)?;

            mark.next = mark.next.max(value.saturating_add(1));
        }
        Ok(())
    }
}
