use crate::arena::ArenaSlice;
use crate::schema::{KeyId, RelationId};
use crate::storage::keys;

use super::{GuardDisposition, GuardOverlay, WriteDelta};

impl WriteDelta<'_> {
    /// Records the guard disposition of one changed fact under every key
    /// statement of its relation — `Some(slice)` for an insert (the fact
    /// establishes each of its key tuples), `None` for a delete. Called by
    /// `insert`/`delete` exactly when the final-state view changes —
    /// including a cancellation, whose op re-records the fact's committed
    /// disposition — so point reads compose delta-over-committed
    /// correctly; last disposition wins.
    ///
    /// Guard bytes come from the one shared slicer ([`keys::guard_bytes`])
    /// — the same derivation commit applies, so a point read and the
    /// judgment phase can never disagree on a tuple's identity.
    pub(super) fn record_guards(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        establishes: Option<ArenaSlice>,
    ) {
        let relation = self.schema.relation(rel);
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::guard_bytes(
                relation.layout(),
                &statement.projection,
                fact_bytes,
                &mut self.guard_scratch,
            );
            let per_key = self.guards.entry(key_id).or_default();
            let disposition = if let Some(slice) = establishes {
                GuardDisposition::Present(slice)
            } else {
                // A delete disestablishes its *own* tuple only: it must
                // not erase a record established by a different pending
                // fact under the same key bytes — `delete(old);
                // insert(new)` is blessed in either order
                // (`docs/architecture/70-api.md`), and the final state
                // keeps `new` whichever ran last.
                if let Some(GuardDisposition::Present(existing)) =
                    per_key.get(self.guard_scratch.as_slice())
                    && self.arena.get(*existing) != fact_bytes
                {
                    continue;
                }
                GuardDisposition::Absent
            };
            per_key.insert(Box::from(self.guard_scratch.as_slice()), disposition);
        }
    }

    /// The delta's net overlay for one key statement's guard tuple, if any
    /// — the delta-first leg of a point read (`docs/architecture/50-storage.md`
    /// § `WriteTx` point reads). `None` = the tuple is untouched by this
    /// transaction and the committed state answers.
    ///
    /// The probe borrows: guard bytes look up as `&[u8]` through the
    /// nested map, so a typed point read touches no allocator (the
    /// borrowed-struct gate pins this).
    #[must_use]
    pub fn guard_overlay(&self, key: KeyId, guard: &[u8]) -> Option<GuardOverlay<'_>> {
        self.guards
            .get(&key)?
            .get(guard)
            .map(|disposition| match disposition {
                GuardDisposition::Present(slice) => GuardOverlay::Present(self.arena.get(*slice)),
                GuardDisposition::Absent => GuardOverlay::Absent,
            })
    }
}
