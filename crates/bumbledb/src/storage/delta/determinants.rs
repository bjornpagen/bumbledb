use crate::arena::ArenaSlice;
use crate::schema::{KeyId, RelationId};
use crate::storage::keys;

use super::{DeterminantDisposition, DeterminantOverlay, Disposition, WriteDelta};

impl WriteDelta<'_> {
    /// Records the determinant disposition of one changed fact under every key
    /// statement of its relation — `Some(slice)` for an insert (the fact
    /// establishes each of its key tuples), `None` for a delete. Called by
    /// `insert`/`delete` exactly when the final-state view changes —
    /// including an insert-cancels-delete, whose fact is committed-present
    /// and re-establishes its tuples (a delete-cancels-insert instead
    /// routes through [`Self::restore_determinants`]) — so point reads
    /// compose delta-over-committed correctly; last disposition wins.
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
            per_key.insert(self.determinant_scratch.clone(), disposition);
        }
    }

    /// Restores the point-read overlay after a delete *cancels* a pending
    /// insert (`delete.rs`): the cancelled fact's net effect is nothing, so
    /// each of its key tuples reverts to whatever still owns it in the
    /// final state — a remaining pending `Insert` re-establishes it, a
    /// remaining pending `Delete` records its absence, and a tuple no
    /// pending fact touches loses its overlay entirely, exactly as if the
    /// cancelled pair never happened. Recording `Absent` here would shadow
    /// a committed owner of the same tuple, breaking the point-read
    /// contract (`docs/architecture/70-api.md` § `WriteTx` point reads:
    /// before commit a point read answers exactly what a post-commit read
    /// transaction would).
    pub(super) fn restore_determinants(&mut self, rel: RelationId, fact_bytes: &[u8]) {
        let relation = self.schema.relation(rel);
        let mut candidate = keys::DeterminantImage::scratch();
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::determinant_image(
                relation.layout(),
                &statement.projection,
                fact_bytes,
                &mut self.determinant_scratch,
            );
            // Resolve the tuple among the relation's remaining pending
            // facts: an `Insert` owns it in the final state (committed
            // state satisfies the key, so any same-tuple `Delete` names
            // the committed owner leaving — the insert wins); no match
            // means the committed state answers unshadowed.
            let mut resolved = None;
            for (slice, disposition) in self
                .facts
                .range((rel, [0u8; 32])..=(rel, [0xFF; 32]))
                .map(|(_, entry)| entry)
            {
                keys::determinant_image(
                    relation.layout(),
                    &statement.projection,
                    self.arena.get(*slice),
                    &mut candidate,
                );
                if candidate.as_bytes() != self.determinant_scratch.as_bytes() {
                    continue;
                }
                match disposition {
                    Disposition::Insert => {
                        resolved = Some(DeterminantDisposition::Present(*slice));
                        break;
                    }
                    Disposition::Delete => resolved = Some(DeterminantDisposition::Absent),
                }
            }
            let Some(per_key) = self.determinants.get_mut(&key_id) else {
                continue;
            };
            match resolved {
                Some(disposition) => {
                    per_key.insert(self.determinant_scratch.clone(), disposition);
                }
                None => {
                    per_key.remove(self.determinant_scratch.as_bytes());
                }
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
