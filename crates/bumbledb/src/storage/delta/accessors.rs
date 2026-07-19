use crate::encoding::fact_hash;
use crate::error::Result;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::{FieldId, RelationId};

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

    /// The relations this commit deletes from, deduplicated, ascending —
    /// the image cache's per-relation dirty classification (reader:
    /// `write_witnessed`'s commit epilogue, which hands it to
    /// `ImageCache::advance`). Net dispositions make the discriminator
    /// exact: a delete-then-reinsert of the same fact cancels to no entry,
    /// so "absent here" is precisely "this commit removes no fact from
    /// the relation" — a delete-free relation's image survives as an
    /// append base. One ordered pass over the `(relation, hash)`-keyed
    /// map (contiguous per relation, so the last-pushed dedup is total);
    /// allocation is at most one small `Vec`.
    pub(crate) fn dirty_relations(&self) -> Vec<RelationId> {
        let mut dirty: Vec<RelationId> = Vec::new();
        for ((rel, _), (_, disposition)) in &self.facts {
            if *disposition == Disposition::Delete && dirty.last() != Some(rel) {
                dirty.push(*rel);
            }
        }
        dirty
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
    pub(crate) fn pending_interns(&self) -> impl Iterator<Item = (&[u8], u64)> + '_ {
        self.pending_interns
            .iter()
            .map(|(raw, id)| (raw.as_ref(), *id))
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
