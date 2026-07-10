use crate::arena::ArenaSlice;
use crate::schema::{RelationId, StatementId};
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
        for &statement in relation.keys() {
            keys::guard_bytes(
                relation.layout(),
                self.schema.key_projection(statement),
                fact_bytes,
                &mut self.guard_scratch,
            );
            let entry = (statement, Box::from(self.guard_scratch.as_slice()));
            let disposition = if let Some(slice) = establishes {
                GuardDisposition::Present(slice)
            } else {
                // A delete disestablishes its *own* tuple only: it must
                // not erase a record established by a different pending
                // fact under the same key bytes — `delete(old);
                // insert(new)` is blessed in either order
                // (`docs/architecture/70-api.md`), and the final state
                // keeps `new` whichever ran last.
                if let Some(GuardDisposition::Present(existing)) = self.guards.get(&entry) {
                    if self.arena.get(*existing) != fact_bytes {
                        continue;
                    }
                }
                GuardDisposition::Absent
            };
            self.guards.insert(entry, disposition);
        }
    }

    /// The delta's net overlay for one key statement's guard tuple, if any
    /// — the delta-first leg of a point read (`docs/architecture/50-storage.md`
    /// § `WriteTx` point reads). `None` = the tuple is untouched by this
    /// transaction and the committed state answers.
    ///
    /// The probe boxes a key copy: the map key is the PRD-specified
    /// `(StatementId, Box<[u8]>)` tuple, and point reads are write-path
    /// methods outside the zero-alloc contract.
    #[must_use]
    pub fn guard_overlay(&self, key: StatementId, guard: &[u8]) -> Option<GuardOverlay<'_>> {
        self.guards
            .get(&(key, guard.into()))
            .map(|disposition| match disposition {
                GuardDisposition::Present(slice) => GuardOverlay::Present(self.arena.get(*slice)),
                GuardDisposition::Absent => GuardOverlay::Absent,
            })
    }
}
