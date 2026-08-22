use crate::encoding::fact_hash;
use crate::error::Result;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::{FieldId, RelationId};

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// # Errors
    pub fn contains(&self, view: &ReadTxn<'_>, rel: RelationId, fact_bytes: &[u8]) -> Result<bool> {
        self.present(view, rel, &fact_hash(fact_bytes))
    }

    pub(crate) fn inserts(&self) -> impl Iterator<Item = (RelationId, &[u8; 32], &[u8])> {
        self.dispositions(Disposition::Insert)
    }

    pub(crate) fn deletes(&self) -> impl Iterator<Item = (RelationId, &[u8; 32], &[u8])> {
        self.dispositions(Disposition::Delete)
    }

    pub(crate) fn dirty_relations(&self) -> Vec<RelationId> {
        let mut dirty: Vec<RelationId> = Vec::new();
        for ((rel, _), (_, disposition)) in &self.facts {
            if *disposition == Disposition::Delete && !dirty.contains(rel) {
                dirty.push(*rel);
            }
        }
        dirty.sort_unstable();
        dirty
    }

    pub(crate) fn inserted_floors(&self) -> Vec<(RelationId, u64)> {
        let mut floors: Vec<(RelationId, u64)> = Vec::new();
        for ((rel, _), (slice, disposition)) in &self.facts {
            if *disposition != Disposition::Insert {
                continue;
            }
            let relation = self.schema.relation(*rel);
            let Some(field) = self.schema.fresh_mint_field(*rel) else {
                continue;
            };
            let row_id = u64::from_be_bytes(crate::encoding::field_word_bytes(
                relation.layout().encoded(self.arena.get(*slice)),
                usize::from(field.0),
            ));
            match floors.iter_mut().find(|(seen, _)| seen == rel) {
                Some((_, min)) => *min = (*min).min(row_id),
                None => floors.push((*rel, row_id)),
            }
        }
        floors.sort_unstable_by_key(|&(rel, _)| rel);
        floors
    }

    fn dispositions(
        &self,
        wanted: Disposition,
    ) -> impl Iterator<Item = (RelationId, &[u8; 32], &[u8])> {
        self.facts
            .iter()
            .filter(move |(_, (_, disposition))| *disposition == wanted)
            .map(|((rel, hash), (slice, _))| (*rel, hash, self.arena.get(*slice)))
    }

    pub(crate) fn fresh_marks(&self) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.marks
            .iter()
            .map(|((rel, field), mark)| (*rel, *field, mark.next))
    }

    pub(crate) fn dirty_fresh_marks(
        &self,
    ) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.marks.iter().filter_map(|((rel, field), mark)| {
            (mark.next > mark.base).then_some((*rel, *field, mark.next))
        })
    }

    pub(crate) fn row_count_deltas(&self) -> impl Iterator<Item = (RelationId, i64)> + '_ {
        self.row_count_delta.iter().map(|(rel, d)| (*rel, *d))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn pending_interns(
        &self,
    ) -> impl Iterator<Item = (&[u8], crate::encoding::InternId)> + '_ {
        self.interns.iter().flat_map(super::PendingInterns::entries)
    }

    #[cfg(test)]
    #[must_use]
    pub fn disposition(&self, rel: RelationId, fact_bytes: &[u8]) -> Option<Disposition> {
        self.facts
            .get(&(rel, fact_hash(fact_bytes)))
            .map(|(_, disposition)| *disposition)
    }
}
