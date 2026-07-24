use crate::arena::ArenaSlice;
use crate::schema::KeyId;
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

use super::{DeterminantOverlay, Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records one changed fact into the point-read overlay under every
    /// key statement of its relation — one `(slice, disposition)` entry
    /// pushed per tuple (`TupleOwners`; the revert target held as data,
    /// finding 097). No same-tuple special case exists anymore: a delete
    /// carries its own slice, so the `delete(old); insert(new)`-in-
    /// either-order idiom resolves at read time by the insert-wins rule,
    /// never by erasing another pending fact's record.
    ///
    /// Determinant bytes come from the one shared slicer ([`keys::determinant_image`])
    /// — the same derivation commit applies, so a point read and the
    /// judgment phase can never disagree on a tuple's identity.
    pub(super) fn record_determinants(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        slice: ArenaSlice,
        disposition: Disposition,
    ) {
        let relation = self.schema.relation(rel);
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::determinant_image(
                relation.layout(),
                &statement.projection,
                fact_bytes,
                &mut self.determinant_scratch,
            );
            let per_key = self.determinants.entry(key_id).or_default();
            // Probe before inserting: the scratch is cloned only the
            // first time a tuple is recorded — the scratch field's
            // no-per-key-statement allocation contract.
            if let Some(owners) = per_key.get_mut(self.determinant_scratch.as_bytes()) {
                owners.push((slice, disposition));
            } else {
                #[cfg(test)]
                {
                    self.determinant_scratch_clones += 1;
                }
                per_key.insert(self.determinant_scratch.clone(), vec![(slice, disposition)]);
            }
        }
    }

    /// Removes one CANCELLED op's own overlay entries (delete-cancels-
    /// insert and insert-cancels-delete alike, `delete.rs`/`insert.rs`):
    /// each of the cancelled fact's tuples drops exactly the cancelled
    /// slice and reverts to what remains — the owners still pending, or
    /// no overlay at all (the committed state answers unshadowed),
    /// exactly as if the cancelled pair never happened. Recording
    /// `Absent` instead would shadow a committed owner of the same
    /// tuple, breaking the point-read contract
    /// (`docs/architecture/70-api.md` § `WriteTx` point reads: before
    /// commit a point read answers exactly what a post-commit read
    /// transaction would). O(log |delta|) — the revert target is data,
    /// never a rescan of the pending set (finding 097).
    pub(super) fn cancel_determinants(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        slice: ArenaSlice,
    ) {
        let relation = self.schema.relation(rel);
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::determinant_image(
                relation.layout(),
                &statement.projection,
                fact_bytes,
                &mut self.determinant_scratch,
            );
            let Some(per_key) = self.determinants.get_mut(&key_id) else {
                continue;
            };
            let Some(owners) = per_key.get_mut(self.determinant_scratch.as_bytes()) else {
                continue;
            };
            owners.retain(|(owner, _)| *owner != slice);
            if owners.is_empty() {
                per_key.remove(self.determinant_scratch.as_bytes());
            }
        }
    }

    /// The delta's net overlay for one key statement's determinant tuple, if any
    /// — the delta-first leg of a point read (`docs/architecture/50-storage.md`
    /// § `WriteTx` point reads). `None` = the tuple is untouched by this
    /// transaction and the committed state answers. A hit resolves by the
    /// insert-wins rule: the LAST-recorded pending `Insert` owns the tuple
    /// in the final state (two pending inserts of one tuple are
    /// commit-doomed but representable — the later one answers, exactly
    /// the fact map's last-disposition order); owners that are all
    /// deletes record its absence.
    ///
    /// The probe borrows: determinant bytes look up as `&[u8]` through the
    /// nested map, so a typed point read touches no allocator (the
    /// borrowed-struct gate pins this).
    #[must_use]
    pub fn determinant_overlay(
        &self,
        key: KeyId,
        determinant: &[u8],
    ) -> Option<DeterminantOverlay<'_>> {
        self.determinants.get(&key)?.get(determinant).map(|owners| {
            owners
                .iter()
                .rev()
                .find_map(|(slice, disposition)| {
                    (*disposition == Disposition::Insert)
                        .then(|| DeterminantOverlay::Present(self.arena.get(*slice)))
                })
                .unwrap_or(DeterminantOverlay::Absent)
        })
    }
}
