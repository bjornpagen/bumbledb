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

    /// The net insert set — exactly the facts commit will add (reader:
    /// the plan derivation, which collects and sorts into the
    /// deterministic `(relation, fact_hash)` commit order; the hash-table
    /// iteration order here is meaningless). The hash rides along
    /// borrowed from the map key: the delta already computed and keyed
    /// by it, so the plan carries it into each fact op for free — the
    /// applier never re-hashes.
    pub(crate) fn inserts(&self) -> impl Iterator<Item = (RelationId, &[u8; 32], &[u8])> {
        self.dispositions(Disposition::Insert)
    }

    /// The net delete set — exactly the facts commit will remove (reader:
    /// the plan derivation), each with its map-key hash and the same
    /// meaningless iteration order as [`Self::inserts`].
    pub(crate) fn deletes(&self) -> impl Iterator<Item = (RelationId, &[u8; 32], &[u8])> {
        self.dispositions(Disposition::Delete)
    }

    /// The relations this commit deletes from, deduplicated, ascending —
    /// the image cache's per-relation dirty classification (reader:
    /// `write_witnessed`'s commit epilogue, which hands it to
    /// `ImageCache::advance`). Net dispositions make the discriminator
    /// exact: a delete-then-reinsert of the same fact cancels to no entry,
    /// so "absent here" is precisely "this commit removes no fact from
    /// the relation" — a delete-free relation's image survives as an
    /// append base. One pass over the hash-table plus a sort of the tiny
    /// per-relation result; allocation is at most one small `Vec`.
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

    /// Per fresh-keyed relation this commit inserts into, the SMALLEST
    /// inserted row id — the first fresh field's value under the one id
    /// allocator (R16). Reader: `write_witnessed`'s commit epilogue,
    /// which hands it to `ImageCache::advance` so an insert below a
    /// retained append base's boundary (explicit fresh re-supply — the
    /// non-tail arm) evicts the base: the prefix property is enforced,
    /// never assumed from counter shape. Ascending by relation (the tiny
    /// per-relation result sorts once); fresh-less relations never
    /// appear — their mints are tail by construction.
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

    /// Fresh next-values to flush to `Q` (reader: the 50-storage doc phase 4).
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

    /// Net row-count changes to fold into `S` (reader: the 50-storage doc phase 4).
    pub(crate) fn row_count_deltas(&self) -> impl Iterator<Item = (RelationId, i64)> + '_ {
        self.row_count_delta.iter().map(|(rel, d)| (*rel, *d))
    }

    /// Pending intern entries to flush to `_dict` (reader: the 50-storage doc phase 4).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn pending_interns(
        &self,
    ) -> impl Iterator<Item = (&[u8], crate::encoding::InternId)> + '_ {
        self.interns.iter().flat_map(super::PendingInterns::entries)
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
