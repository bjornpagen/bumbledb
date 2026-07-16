use crate::arena::ArenaSlice;
use crate::schema::{KeyId, RelationId};
use crate::storage::keys;

use super::{DeterminantDisposition, DeterminantOverlay, WriteDelta};

impl WriteDelta<'_> {
    /// Records the determinant disposition of one changed fact under every key
    /// statement of its relation — `Some(slice)` for an insert (the fact
    /// establishes each of its key tuples), `None` for a delete. Called by
    /// `insert`/`delete` exactly when the final-state view changes —
    /// including a cancellation, whose op re-records the fact's committed
    /// disposition — so point reads compose delta-over-committed
    /// correctly; last disposition wins.
    ///
    /// Determinant bytes come from the one shared slicer ([`keys::determinant_image`])
    /// — the same derivation commit applies, so a point read and the
    /// judgment phase can never disagree on a tuple's identity.
    pub(super) fn record_determinants(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        establishes: Option<ArenaSlice>,
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
            let disposition = if let Some(slice) = establishes {
                DeterminantDisposition::Present(slice)
            } else {
                // A delete disestablishes its *own* tuple only: it must
                // not erase a record established by a different pending
                // fact under the same key bytes — `delete(old);
                // insert(new)` is blessed in either order
                // (`docs/architecture/70-api.md`), and the final state
                // keeps `new` whichever ran last.
                if let Some(DeterminantDisposition::Present(existing)) =
                    per_key.get(self.determinant_scratch.as_bytes())
                    && self.arena.get(*existing) != fact_bytes
                {
                    continue;
                }
                DeterminantDisposition::Absent
            };
            // Probe before inserting: an overwrite (the tuple was already
            // recorded this transaction) updates the resident entry in
            // place; the scratch is cloned only the first time a tuple is
            // recorded — the scratch field's no-per-key-statement
            // allocation contract.
            if let Some(recorded) = per_key.get_mut(self.determinant_scratch.as_bytes()) {
                *recorded = disposition;
            } else {
                #[cfg(test)]
                {
                    self.determinant_scratch_clones += 1;
                }
                per_key.insert(self.determinant_scratch.clone(), disposition);
            }
        }
    }

    /// The delta's net overlay for one key statement's determinant tuple, if any
    /// — the delta-first leg of a point read (`docs/architecture/50-storage.md`
    /// § `WriteTx` point reads). `None` = the tuple is untouched by this
    /// transaction and the committed state answers.
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
        self.determinants
            .get(&key)?
            .get(determinant)
            .map(|disposition| match disposition {
                DeterminantDisposition::Present(slice) => {
                    DeterminantOverlay::Present(self.arena.get(*slice))
                }
                DeterminantDisposition::Absent => DeterminantOverlay::Absent,
            })
    }
}
