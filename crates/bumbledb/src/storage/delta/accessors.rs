#[cfg(test)]
use crate::encoding::fact_hash;
use crate::schema::{FieldId, RelationId};

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Iterates every recorded disposition in deterministic
    /// `(relation, fact_hash)` order with its fact bytes (reader: the 40-storage doc's
    /// commit apply).
    pub(crate) fn entries(&self) -> impl Iterator<Item = (RelationId, &[u8], Disposition)> {
        self.facts
            .iter()
            .map(|((rel, _), (slice, disposition))| (*rel, self.arena.get(*slice), *disposition))
    }

    /// Serial next-values to flush to `Q` (reader: the 40-storage doc phase 4).
    pub(crate) fn serial_marks(&self) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.serial_next
            .iter()
            .map(|((rel, field), next)| (*rel, *field, *next))
    }

    /// The serial marks that advanced past their committed base — the
    /// allocations this transaction actually issued. These persist even
    /// when a commit nets to no fact change (reader: the commit's
    /// counters-only path).
    pub(crate) fn dirty_serial_marks(
        &self,
    ) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.serial_next.iter().filter_map(|((rel, field), next)| {
            let base = self
                .serial_base
                .get(&(*rel, *field))
                .expect("every serial_next entry began with a base read");
            (next > base).then_some((*rel, *field, *next))
        })
    }

    /// Net row-count changes to fold into `S` (reader: the 40-storage doc phase 4).
    pub(crate) fn row_count_deltas(&self) -> impl Iterator<Item = (RelationId, i64)> + '_ {
        self.row_count_delta.iter().map(|(rel, d)| (*rel, *d))
    }

    /// Pending intern entries to flush to `_dict` (reader: the 40-storage doc phase 4).
    pub(crate) fn pending_interns(&self) -> impl Iterator<Item = (u8, &[u8], u64)> + '_ {
        self.pending_interns
            .iter()
            .enumerate()
            .flat_map(|(tag, map)| {
                map.iter()
                    .map(move |(raw, id)| (u8::try_from(tag).expect("two tags"), raw.as_ref(), *id))
            })
    }

    /// The recorded disposition for a fact, if any (last one wins).
    #[cfg(test)]
    #[must_use]
    pub fn disposition(&self, rel: RelationId, fact_bytes: &[u8]) -> Option<Disposition> {
        self.facts
            .get(&(rel, fact_hash(fact_bytes)))
            .map(|(_, disposition)| *disposition)
    }
}
