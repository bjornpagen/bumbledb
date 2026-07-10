use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::{FieldId, RelationId};
use crate::storage::env::ReadTxn;

use super::{Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Effective membership of one fact in the final state: the delta's
    /// own disposition if present, else an `M` probe against the committed
    /// view — the read-only sibling of `insert`/`delete`'s changed report
    /// (`docs/architecture/50-storage.md` § `WriteTx` point reads).
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed membership probe.
    pub fn contains(&self, view: &ReadTxn<'_>, rel: RelationId, fact_bytes: &[u8]) -> Result<bool> {
        self.present(view, rel, &fact_hash(fact_bytes))
    }

    /// The net insert set in deterministic `(relation, fact_hash)` order —
    /// exactly the facts commit will add (readers: the apply phase and the
    /// source-side judgment, which iterates it directly).
    pub(crate) fn inserts(&self) -> impl Iterator<Item = (RelationId, &[u8])> {
        self.dispositions(Disposition::Insert)
    }

    /// The net delete set in deterministic `(relation, fact_hash)` order —
    /// exactly the facts commit will remove (reader: the apply phase).
    pub(crate) fn deletes(&self) -> impl Iterator<Item = (RelationId, &[u8])> {
        self.dispositions(Disposition::Delete)
    }

    fn dispositions(&self, wanted: Disposition) -> impl Iterator<Item = (RelationId, &[u8])> {
        self.facts
            .iter()
            .filter(move |(_, (_, disposition))| *disposition == wanted)
            .map(|((rel, _), (slice, _))| (*rel, self.arena.get(*slice)))
    }

    /// Fresh next-values to flush to `Q` (reader: the 40-storage doc phase 4).
    pub(crate) fn fresh_marks(&self) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.marks
            .iter()
            .map(|((rel, field), mark)| (*rel, *field, mark.next))
    }

    /// The fresh marks that advanced past their committed base — the
    /// allocations this transaction actually issued. These persist even
    /// when a commit nets to no fact change (reader: the commit's
    /// counters-only path).
    pub(crate) fn dirty_fresh_marks(
        &self,
    ) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.marks.iter().filter_map(|((rel, field), mark)| {
            (mark.next > mark.base).then_some((*rel, *field, mark.next))
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
