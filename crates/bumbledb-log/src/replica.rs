//! Published read capability and recovery states.
//!
//! A public read exposes only a complete published local snapshot and its
//! stamps — never the writable core `Db`. This is the successor replacement
//! for the deleted braided replica: no `Replica::db` writable escape, no
//! vector floors, no manifest/sidecar chain. Refresh captures one finite tip;
//! `AtLeast` checks actual same-lineage ancestry rather than comparing
//! sequence integers. Missing history returns `WitnessUnavailable`, never an
//! empty database.
//!
//! The bytes served come from the core's read snapshot; catching a
//! materialization up to a captured tip is the backend's job (hosted machine
//! plus `apply`), not a second recovery protocol here.

use crate::history::authority::HeadAuthority;
use crate::history::{DatabaseIdentity, DecisionStamp, StateStamp};

/// The freshness provenance of a published snapshot. `Cached` is potentially
/// stale but never speculative; `Latest`/`AtLeast` name what was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Cached,
    Latest,
    AtLeast { requested: DecisionStamp },
}

/// The read-consistency request. `Latest` captures the authoritative target
/// once then catches up to it under budget; it does not chase a moving tip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadConsistency {
    Cached,
    AtLeast { at: DecisionStamp },
    Latest,
}

/// Why a requested read consistency could not be honored. None of these is an
/// empty snapshot: a missing witness is explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadRefusal {
    /// The requested decision is not an ancestor of this incarnation's tip
    /// (wrong lineage or an unrelated hash at a retained sequence).
    NotAncestor { requested: DecisionStamp },
    /// The requested decision is ahead of the captured authoritative tip.
    NotYetAvailable {
        requested: DecisionStamp,
        captured: DecisionStamp,
    },
    /// The exact ancestry hash could not be established from retained evidence.
    WitnessUnavailable { requested: DecisionStamp },
    /// The authority is a terminal tombstone; ordinary reads refuse.
    DatabaseDeleted,
}

/// The provenance a published snapshot carries. The snapshot's actual query
/// capability is the core `QueryReader` over a pinned read transaction; this
/// value is the log-added identity/stamp/freshness envelope, with no writable
/// escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotProvenance {
    pub identity: DatabaseIdentity,
    pub decision: DecisionStamp,
    pub state: StateStamp,
    pub freshness: Freshness,
}

impl SnapshotProvenance {
    /// Derive the provenance a snapshot at `authority` satisfies for the given
    /// requested consistency, or a typed refusal. The caller supplies the
    /// bounded ancestry check `is_ancestor` over retained evidence, so this
    /// never claims an exact witness it did not verify.
    ///
    /// # Errors
    /// Refuses wrong-lineage, not-yet-available and unwitnessed coordinates,
    /// and reads against a deleted authority.
    pub fn resolve(
        authority: &HeadAuthority,
        consistency: ReadConsistency,
        is_ancestor: impl FnOnce(DecisionStamp) -> WitnessCheck,
    ) -> Result<Self, ReadRefusal> {
        let position = authority.position().ok_or(ReadRefusal::DatabaseDeleted)?;
        let base = Self {
            identity: authority.identity,
            decision: position.decision,
            state: position.state,
            freshness: Freshness::Cached,
        };
        match consistency {
            ReadConsistency::Cached => Ok(base),
            ReadConsistency::Latest => Ok(Self {
                freshness: Freshness::Latest,
                ..base
            }),
            ReadConsistency::AtLeast { at } => {
                if at.seq > position.decision.seq {
                    return Err(ReadRefusal::NotYetAvailable {
                        requested: at,
                        captured: position.decision,
                    });
                }
                if at == position.decision {
                    return Ok(Self {
                        freshness: Freshness::AtLeast { requested: at },
                        ..base
                    });
                }
                match is_ancestor(at) {
                    WitnessCheck::Ancestor => Ok(Self {
                        freshness: Freshness::AtLeast { requested: at },
                        ..base
                    }),
                    WitnessCheck::NotAncestor => Err(ReadRefusal::NotAncestor { requested: at }),
                    WitnessCheck::Unavailable => {
                        Err(ReadRefusal::WitnessUnavailable { requested: at })
                    }
                }
            }
        }
    }
}

/// The bounded ancestry check's result. `AtLeast` needs actual ancestry, not
/// sequence comparison: an unretained witness is `Unavailable`, never assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessCheck {
    Ancestor,
    NotAncestor,
    Unavailable,
}

/// The recovery states a materializer passes through before serving. Only a
/// complete identified published snapshot becomes `Ready`; failed hydration is
/// never an empty database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryState {
    Closed,
    OwnedDirectory,
    IdentifiedOrigin { identity: DatabaseIdentity },
    BuildingOrCatchingUp { target: DecisionStamp },
    Verifying { target: DecisionStamp },
    Ready(SnapshotProvenance),
    Retryable { reason: RecoveryStall },
    Refused { reason: RecoveryStall },
    Corrupt { evidence: RecoveryStall },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStall {
    DatabaseMissing,
    ForeignCache,
    MissingDependency,
    WitnessUnavailable,
    Tombstoned,
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use crate::history::authority::{Activation, ActivationCause, HeadAuthority};
    use crate::history::{
        DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, IncarnationId, OperationId,
        SchemaId,
    };

    use super::*;

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    fn authority(seq: u64) -> HeadAuthority {
        let genesis = DecisionStamp {
            seq: 0,
            hash: DecisionDigest::from_bytes([9; 32]),
        };
        let mut head = HeadAuthority::genesis(
            identity(),
            genesis,
            Activation::Activated {
                operation: OperationId::from_core(Id128::from_bytes([4; 16])),
                target_genesis: genesis.hash,
                cause: ActivationCause::Create,
            },
        )
        .unwrap();
        for index in 0..seq {
            let byte = u8::try_from((index % 255) + 1).expect("bounded above");
            head = head
                .decided(DecisionDigest::from_bytes([byte; 32]), true)
                .unwrap();
        }
        head
    }

    #[test]
    fn cached_and_latest_carry_the_captured_tip() {
        let head = authority(3);
        let cached = SnapshotProvenance::resolve(&head, ReadConsistency::Cached, |_| {
            WitnessCheck::Unavailable
        })
        .unwrap();
        assert_eq!(cached.freshness, Freshness::Cached);
        assert_eq!(cached.decision.seq, 3);
        let latest =
            SnapshotProvenance::resolve(&head, ReadConsistency::Latest, |_| WitnessCheck::Ancestor)
                .unwrap();
        assert_eq!(latest.freshness, Freshness::Latest);
    }

    #[test]
    fn at_least_needs_actual_ancestry_not_sequence_comparison() {
        let head = authority(3);
        let tip = head.position().unwrap().decision;
        // Requesting the exact tip needs no witness lookup.
        let exact =
            SnapshotProvenance::resolve(&head, ReadConsistency::AtLeast { at: tip }, |_| {
                WitnessCheck::Unavailable
            })
            .unwrap();
        assert_eq!(exact.freshness, Freshness::AtLeast { requested: tip });
        // A past sequence with retained ancestry evidence resolves.
        let past = DecisionStamp {
            seq: 1,
            hash: DecisionDigest::from_bytes([1; 32]),
        };
        let ok = SnapshotProvenance::resolve(&head, ReadConsistency::AtLeast { at: past }, |_| {
            WitnessCheck::Ancestor
        })
        .unwrap();
        assert_eq!(ok.freshness, Freshness::AtLeast { requested: past });
        // Without retained evidence, it is WitnessUnavailable, not assumed.
        assert_eq!(
            SnapshotProvenance::resolve(&head, ReadConsistency::AtLeast { at: past }, |_| {
                WitnessCheck::Unavailable
            }),
            Err(ReadRefusal::WitnessUnavailable { requested: past })
        );
        // A future coordinate is NotYetAvailable, never silently downgraded.
        let future = DecisionStamp {
            seq: 9,
            hash: DecisionDigest::from_bytes([9; 32]),
        };
        assert_eq!(
            SnapshotProvenance::resolve(&head, ReadConsistency::AtLeast { at: future }, |_| {
                WitnessCheck::Ancestor
            }),
            Err(ReadRefusal::NotYetAvailable {
                requested: future,
                captured: tip,
            })
        );
    }

    #[test]
    fn deleted_authority_refuses_ordinary_reads() {
        let head = authority(2);
        let deleted = match head
            .delete(
                OperationId::from_core(Id128::from_bytes([7; 16])),
                crate::history::authority::DeletedReason::Erasure,
            )
            .unwrap()
        {
            crate::history::authority::DeleteOutcome::Deleted(head) => head,
            crate::history::authority::DeleteOutcome::AlreadyDeleted { .. } => unreachable!(),
        };
        assert_eq!(
            SnapshotProvenance::resolve(&deleted, ReadConsistency::Cached, |_| {
                WitnessCheck::Ancestor
            }),
            Err(ReadRefusal::DatabaseDeleted)
        );
    }
}
