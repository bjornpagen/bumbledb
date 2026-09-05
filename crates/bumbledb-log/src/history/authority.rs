//! The one durable authority record and its legal transitions.
//!
//! `HeadAuthority` is the P04-owned authority projection of a tenant head:
//! identity, head revision, lifecycle (Live access/decision/state/receipts or
//! a terminal Deleted tombstone) and the one-time activation marker. Hosted
//! retention fields (recovery root, named roots, object epoch, GC state) are
//! P05-owned and ride outside this projection; every transition here returns a
//! **value** that the actual authority must commit atomically — an S3 head CAS
//! or one `LocalHistory` LMDB transaction. Constructing a transition is never
//! publication.
//!
//! Frozen and Deleted are authority states, not local flags. Deleted is
//! terminal: no transition returns an incarnation to Live. Activation is a
//! one-time bounded control record; a matching retry returns the recorded
//! evidence without mutating anything and cannot thaw a later freeze or revive
//! a deleted authority.

use super::frame::{
    FrameError, Reader, begin_frame, frame_len, put_identity, put_stamp, put_state, put_u64,
};
use super::{
    AccessMode, CounterExhausted, DatabaseIdentity, DecisionDigest, DecisionStamp, HeadRevision,
    HistoryPosition, IncarnationId, OperationId, PolicyError, ReceiptEpoch, ReceiptPolicy,
    StateStamp, admission::AdmissionView,
};

pub const FAMILY: &[u8] = b"bumbledb.authority.v1\0";
pub const LAYOUT: u16 = 1;
const CONTROL: u8 = 1;

/// Why an authority is frozen. The operation identity is fixed before
/// dispatch; a different plan/operation cannot take over a frozen authority
/// by reusing its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeIntent {
    Migration {
        plan_set_digest: [u8; 32],
        target: IncarnationId,
    },
    Erasure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Active,
    Frozen {
        operation: OperationId,
        intent: FreezeIntent,
    },
}

impl Access {
    #[must_use]
    pub const fn mode(self) -> AccessMode {
        match self {
            Self::Active => AccessMode::Active,
            Self::Frozen { .. } => AccessMode::Frozen,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletedReason {
    Erasure,
    MigrationAborted {
        source_database: super::DatabaseId,
        source_incarnation: IncarnationId,
        plan_set_digest: [u8; 32],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationCause {
    Create,
    Restore,
    Migration { plan_set_digest: [u8; 32] },
}

/// One-time activation evidence. Preserved by later commands, maintenance,
/// freeze, GC bookkeeping, receipt retirement and deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    NotActivated,
    Activated {
        operation: OperationId,
        target_genesis: DecisionDigest,
        cause: ActivationCause,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveAuthority {
    pub access: Access,
    pub decision: DecisionStamp,
    pub state: StateStamp,
    pub receipts: ReceiptPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Live(LiveAuthority),
    /// Terminal tombstone: no current checkpoint/tip, receipt table or
    /// migration-history dependency, and no transition back to Live.
    Deleted {
        operation: OperationId,
        reason: DeletedReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadAuthority {
    pub identity: DatabaseIdentity,
    pub revision: HeadRevision,
    pub lifecycle: Lifecycle,
    pub activation: Activation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityError {
    /// The authority is a terminal tombstone.
    Deleted,
    /// A live-only transition was asked of a frozen authority.
    Frozen {
        operation: OperationId,
    },
    /// A frozen-only transition was asked of an active authority.
    NotFrozen,
    /// The transition named a different operation than the one recorded.
    OperationMismatch {
        held: OperationId,
    },
    /// Activation evidence exists and does not match the request.
    ActivationEvidenceMismatch,
    /// Genesis construction requires a sequence-zero decision stamp.
    InvalidGenesis,
    Exhausted(CounterExhausted),
    Policy(PolicyError),
}

impl From<CounterExhausted> for AuthorityError {
    fn from(error: CounterExhausted) -> Self {
        Self::Exhausted(error)
    }
}

impl From<PolicyError> for AuthorityError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

/// A freeze that already happened under the same operation is evidence, not
/// a second mutation.
#[expect(
    clippy::large_enum_variant,
    reason = "HeadAuthority is a fixed-size Copy control frame; these Copy \
              outcome shapes are consumed by ts/crate log_wire/admin.rs and \
              boxing would break Copy"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeOutcome {
    Frozen(HeadAuthority),
    AlreadyFrozen {
        operation: OperationId,
        intent: FreezeIntent,
    },
}

#[expect(
    clippy::large_enum_variant,
    reason = "HeadAuthority is a fixed-size Copy control frame; these Copy \
              outcome shapes are consumed by ts/crate log_wire/admin.rs and \
              boxing would break Copy"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivateOutcome {
    Activated(HeadAuthority),
    /// Matching retry: recorded evidence plus current access, no mutation.
    /// This never thaws a later freeze or revives a deleted authority.
    AlreadyActivated {
        activation: Activation,
        access: AccessMode,
    },
}

#[expect(
    clippy::large_enum_variant,
    reason = "HeadAuthority is a fixed-size Copy control frame; these Copy \
              outcome shapes are consumed by ts/crate log_wire/admin.rs and \
              boxing would break Copy"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted(HeadAuthority),
    /// Matching retry against the recorded tombstone; no mutation.
    AlreadyDeleted {
        operation: OperationId,
        reason: DeletedReason,
    },
}

impl HeadAuthority {
    /// A new incarnation: sequence-zero genesis decision, state revision zero,
    /// open receipt epoch 1, retired-through 0, head revision 1, Live/Active.
    /// The genesis record itself is framed by [`super::decision`]; ordinary
    /// blank creation may publish its activation marker with genesis.
    /// # Errors
    /// Refuses a non-genesis stamp or a wrong-incarnation identity.
    pub fn genesis(
        identity: DatabaseIdentity,
        genesis: DecisionStamp,
        activation: Activation,
    ) -> Result<Self, AuthorityError> {
        if genesis.seq != 0 {
            return Err(AuthorityError::InvalidGenesis);
        }
        Ok(Self {
            identity,
            revision: HeadRevision(1),
            lifecycle: Lifecycle::Live(LiveAuthority {
                access: Access::Active,
                decision: genesis,
                state: StateStamp {
                    incarnation: identity.incarnation_id,
                    data_revision: 0,
                },
                receipts: ReceiptPolicy::INITIAL,
            }),
            activation,
        })
    }

    /// A conditional tombstone created **instead of** a migration target's
    /// genesis: cancelling an unpublished target durably fences its delayed
    /// genesis/activation. No fictitious checkpoint or decision stamp exists.
    #[must_use]
    pub const fn cancelled_before_genesis(
        planned_identity: DatabaseIdentity,
        operation: OperationId,
        reason: DeletedReason,
    ) -> Self {
        Self {
            identity: planned_identity,
            revision: HeadRevision(1),
            lifecycle: Lifecycle::Deleted { operation, reason },
            activation: Activation::NotActivated,
        }
    }

    /// # Errors
    /// A terminal tombstone has no live authority.
    pub const fn live(&self) -> Result<&LiveAuthority, AuthorityError> {
        match &self.lifecycle {
            Lifecycle::Live(live) => Ok(live),
            Lifecycle::Deleted { .. } => Err(AuthorityError::Deleted),
        }
    }

    /// The coherent admission projection, or `None` for a tombstone (which
    /// refuses ordinary resolve/submit before hydration).
    #[must_use]
    pub fn admission_view(&self) -> Option<AdmissionView> {
        let live = self.live().ok()?;
        Some(AdmissionView {
            identity: self.identity,
            access: live.access.mode(),
            receipts: live.receipts,
            decision: live.decision,
            state: live.state,
        })
    }

    #[must_use]
    pub fn position(&self) -> Option<HistoryPosition> {
        let live = self.live().ok()?;
        Some(HistoryPosition {
            head: self.revision,
            decision: live.decision,
            state: live.state,
        })
    }

    /// One published terminal decision: head revision and decision sequence
    /// advance; state revision advances iff application facts changed.
    /// Requires a live, active authority — a frozen authority publishes no
    /// new decisions and a historical replay installs positions instead.
    /// # Errors
    /// Refuses tombstones, frozen access and exhausted counters.
    pub fn decided(
        &self,
        hash: DecisionDigest,
        facts_changed: bool,
    ) -> Result<Self, AuthorityError> {
        let live = self.live()?;
        if let Access::Frozen { operation, .. } = live.access {
            return Err(AuthorityError::Frozen { operation });
        }
        let next = HistoryPosition {
            head: self.revision,
            decision: live.decision,
            state: live.state,
        }
        .decided(hash, facts_changed)?;
        Ok(Self {
            revision: next.head,
            lifecycle: Lifecycle::Live(LiveAuthority {
                decision: next.decision,
                state: next.state,
                ..*live
            }),
            ..*self
        })
    }

    /// Maintenance: only the head revision advances. Decision and state
    /// stamps never move, so read witnesses survive maintenance.
    /// # Errors
    /// Refuses tombstones and revision exhaustion.
    pub fn maintained(&self) -> Result<Self, AuthorityError> {
        self.live()?;
        let revision = self
            .revision
            .0
            .checked_add(1)
            .ok_or(CounterExhausted::HeadRevision)?;
        Ok(Self {
            revision: HeadRevision(revision),
            ..*self
        })
    }

    /// Rotation strictly advances the open receipt epoch; a maintenance CAS.
    /// # Errors
    /// Refuses tombstones, frozen access, backward rotation and exhaustion.
    pub fn rotate_receipts(&self, next_epoch: ReceiptEpoch) -> Result<Self, AuthorityError> {
        let live = *self.require_active()?;
        let receipts = live.receipts.rotate(next_epoch)?;
        let maintained = self.maintained()?;
        Ok(Self {
            lifecycle: Lifecycle::Live(LiveAuthority { receipts, ..live }),
            ..maintained
        })
    }

    /// Retirement advances the monotone frontier through closed epochs only.
    /// The authority commit that installs this value must atomically stop
    /// promising the retired receipt rows (checkpoint filter or one local
    /// LMDB transaction), never a later best-effort cleanup.
    /// # Errors
    /// Refuses tombstones, frozen access and non-monotone/open retirement.
    pub fn retire_receipts(&self, through: u64) -> Result<Self, AuthorityError> {
        let live = *self.require_active()?;
        let receipts = live.receipts.retire(through)?;
        let maintained = self.maintained()?;
        Ok(Self {
            lifecycle: Lifecycle::Live(LiveAuthority { receipts, ..live }),
            ..maintained
        })
    }

    /// Durable freeze under a named operation. Reads and retained receipt
    /// lookup continue; only new command admission stops. No timer thaws it.
    /// # Errors
    /// Refuses tombstones and a different operation's freeze.
    pub fn freeze(
        &self,
        operation: OperationId,
        intent: FreezeIntent,
    ) -> Result<FreezeOutcome, AuthorityError> {
        let live = *self.live()?;
        match live.access {
            Access::Frozen {
                operation: held,
                intent: held_intent,
            } => {
                if held == operation && held_intent == intent {
                    Ok(FreezeOutcome::AlreadyFrozen {
                        operation: held,
                        intent: held_intent,
                    })
                } else {
                    Err(AuthorityError::OperationMismatch { held })
                }
            }
            Access::Active => {
                let maintained = self.maintained()?;
                Ok(FreezeOutcome::Frozen(Self {
                    lifecycle: Lifecycle::Live(LiveAuthority {
                        access: Access::Frozen { operation, intent },
                        ..live
                    }),
                    ..maintained
                }))
            }
        }
    }

    /// Thaw the matching frozen operation. For a migration abort the caller
    /// must already hold the durable target fence (target tombstone or
    /// refused activation); an uncertain target cancellation never authorizes
    /// thaw. That ordering is the runner's obligation — this value transition
    /// only enforces operation identity.
    /// # Errors
    /// Refuses tombstones, active authorities and mismatched operations.
    pub fn thaw(&self, operation: OperationId) -> Result<Self, AuthorityError> {
        let live = *self.live()?;
        match live.access {
            Access::Active => Err(AuthorityError::NotFrozen),
            Access::Frozen {
                operation: held, ..
            } => {
                if held != operation {
                    return Err(AuthorityError::OperationMismatch { held });
                }
                let maintained = self.maintained()?;
                Ok(Self {
                    lifecycle: Lifecycle::Live(LiveAuthority {
                        access: Access::Active,
                        ..live
                    }),
                    ..maintained
                })
            }
        }
    }

    /// One-time activation: `NotActivated` under this operation's freeze
    /// becomes `Activated` atomically with access becoming Active. A matching
    /// retry returns the recorded evidence and the *current* access mode
    /// without mutating: it never thaws a later freeze or revives a deleted
    /// authority.
    /// # Errors
    /// Refuses tombstone activation of a never-activated target, foreign
    /// operations and conflicting activation evidence.
    pub fn activate(
        &self,
        operation: OperationId,
        target_genesis: DecisionDigest,
        cause: ActivationCause,
    ) -> Result<ActivateOutcome, AuthorityError> {
        if let Activation::Activated {
            operation: held,
            target_genesis: held_genesis,
            cause: held_cause,
        } = self.activation
        {
            if held == operation && held_genesis == target_genesis && held_cause == cause {
                let access = match &self.lifecycle {
                    Lifecycle::Live(live) => live.access.mode(),
                    Lifecycle::Deleted { .. } => AccessMode::Deleted,
                };
                return Ok(ActivateOutcome::AlreadyActivated {
                    activation: self.activation,
                    access,
                });
            }
            return Err(AuthorityError::ActivationEvidenceMismatch);
        }
        let live = *self.live()?;
        match live.access {
            Access::Active => Err(AuthorityError::NotFrozen),
            Access::Frozen {
                operation: held, ..
            } => {
                if held != operation {
                    return Err(AuthorityError::OperationMismatch { held });
                }
                let maintained = self.maintained()?;
                Ok(ActivateOutcome::Activated(Self {
                    lifecycle: Lifecycle::Live(LiveAuthority {
                        access: Access::Active,
                        ..live
                    }),
                    activation: Activation::Activated {
                        operation,
                        target_genesis,
                        cause,
                    },
                    ..maintained
                }))
            }
        }
    }

    /// Terminal deletion: the live state becomes a tombstone that preserves
    /// identity, revision continuity and prior activation evidence. A live
    /// unactivated migration target may be cancelled this way only under its
    /// matching operation; if activation already won, cancellation refuses.
    /// # Errors
    /// Refuses a conflicting recorded tombstone and post-activation
    /// cancellation of the same operation.
    pub fn delete(
        &self,
        operation: OperationId,
        reason: DeletedReason,
    ) -> Result<DeleteOutcome, AuthorityError> {
        if let Lifecycle::Deleted {
            operation: held,
            reason: held_reason,
        } = self.lifecycle
        {
            if held == operation && held_reason == reason {
                return Ok(DeleteOutcome::AlreadyDeleted {
                    operation: held,
                    reason: held_reason,
                });
            }
            return Err(AuthorityError::OperationMismatch { held });
        }
        if let (
            DeletedReason::MigrationAborted { .. },
            Activation::Activated {
                operation: activated,
                ..
            },
        ) = (reason, self.activation)
            && activated == operation
        {
            // Activation won this race: automatic abort/thaw must refuse.
            return Err(AuthorityError::ActivationEvidenceMismatch);
        }
        let maintained = self.maintained()?;
        Ok(DeleteOutcome::Deleted(Self {
            lifecycle: Lifecycle::Deleted { operation, reason },
            ..maintained
        }))
    }

    fn require_active(&self) -> Result<&LiveAuthority, AuthorityError> {
        let live = self.live()?;
        if let Access::Frozen { operation, .. } = live.access {
            return Err(AuthorityError::Frozen { operation });
        }
        Ok(live)
    }
}

/// Encode the authority projection. Hosted head bytes embed this projection
/// beside P05-owned retention fields; `LocalHistory` commits it as the LMDB
/// attachment component. Bytes remain provisional until the F3 format freeze.
/// # Errors
/// Refuses oversized frames and allocation failure.
#[expect(
    clippy::too_many_lines,
    reason = "one bounded encoder over the frozen control grammar"
)]
pub fn encode_control(authority: &HeadAuthority, cap: usize) -> Result<Vec<u8>, FrameError> {
    let lifecycle_len = match &authority.lifecycle {
        Lifecycle::Live(live) => {
            1 + match live.access {
                Access::Active => 1,
                Access::Frozen { intent, .. } => {
                    1 + 16
                        + 1
                        + match intent {
                            FreezeIntent::Migration { .. } => 48,
                            FreezeIntent::Erasure => 0,
                        }
                }
            } + 40
                + 24
                + 16
        }
        Lifecycle::Deleted { reason, .. } => {
            1 + 16
                + 1
                + match reason {
                    DeletedReason::Erasure => 0,
                    DeletedReason::MigrationAborted { .. } => 64,
                }
        }
    };
    let activation_len = match authority.activation {
        Activation::NotActivated => 1,
        Activation::Activated { cause, .. } => {
            1 + 16
                + 32
                + 1
                + match cause {
                    ActivationCause::Migration { .. } => 32,
                    _ => 0,
                }
        }
    };
    let len = frame_len(FAMILY.len(), &[64, 8, lifecycle_len, activation_len])?;
    let mut out = begin_frame(FAMILY, LAYOUT, CONTROL, len, cap)?;
    put_identity(&mut out, authority.identity);
    put_u64(&mut out, authority.revision.0);
    match &authority.lifecycle {
        Lifecycle::Live(live) => {
            out.push(0);
            match live.access {
                Access::Active => out.push(0),
                Access::Frozen { operation, intent } => {
                    out.push(1);
                    out.extend_from_slice(operation.as_core().as_bytes());
                    match intent {
                        FreezeIntent::Migration {
                            plan_set_digest,
                            target,
                        } => {
                            out.push(0);
                            out.extend_from_slice(&plan_set_digest);
                            out.extend_from_slice(target.as_core().as_bytes());
                        }
                        FreezeIntent::Erasure => out.push(1),
                    }
                }
            }
            put_stamp(&mut out, live.decision);
            put_state(&mut out, live.state);
            put_u64(&mut out, live.receipts.open_epoch().get());
            put_u64(&mut out, live.receipts.retired_through());
        }
        Lifecycle::Deleted { operation, reason } => {
            out.push(1);
            out.extend_from_slice(operation.as_core().as_bytes());
            match reason {
                DeletedReason::Erasure => out.push(0),
                DeletedReason::MigrationAborted {
                    source_database,
                    source_incarnation,
                    plan_set_digest,
                } => {
                    out.push(1);
                    out.extend_from_slice(source_database.as_core().as_bytes());
                    out.extend_from_slice(source_incarnation.as_core().as_bytes());
                    out.extend_from_slice(plan_set_digest);
                }
            }
        }
    }
    match authority.activation {
        Activation::NotActivated => out.push(0),
        Activation::Activated {
            operation,
            target_genesis,
            cause,
        } => {
            out.push(1);
            out.extend_from_slice(operation.as_core().as_bytes());
            out.extend_from_slice(target_genesis.as_bytes());
            match cause {
                ActivationCause::Create => out.push(0),
                ActivationCause::Restore => out.push(1),
                ActivationCause::Migration { plan_set_digest } => {
                    out.push(2);
                    out.extend_from_slice(&plan_set_digest);
                }
            }
        }
    }
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

/// Decode and validate an authority projection. Grammar and internal
/// invariants only; bytes from storage still need their outer object/digest
/// verification before they are trusted as an authority.
/// # Errors
/// Refuses malformed frames and violated authority invariants.
pub fn decode_control(bytes: &[u8], cap: usize) -> Result<HeadAuthority, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, CONTROL, cap)?;
    let identity = input.identity()?;
    let revision = HeadRevision(input.u64()?);
    if revision.0 == 0 {
        return Err(FrameError::InvalidSequence);
    }
    let lifecycle = match input.tag()? {
        (_, 0) => {
            let access = match input.tag()? {
                (_, 0) => Access::Active,
                (_, 1) => {
                    let operation =
                        OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
                    let intent = match input.tag()? {
                        (_, 0) => FreezeIntent::Migration {
                            plan_set_digest: input.array()?,
                            target: IncarnationId::from_core(bumbledb::Id128::from_bytes(
                                input.array()?,
                            )),
                        },
                        (_, 1) => FreezeIntent::Erasure,
                        (at, got) => return Err(FrameError::Tag { at, got }),
                    };
                    Access::Frozen { operation, intent }
                }
                (at, got) => return Err(FrameError::Tag { at, got }),
            };
            let decision = input.stamp()?;
            let state = input.state()?;
            let receipts = ReceiptPolicy::new(
                ReceiptEpoch::new(input.u64()?).ok_or(FrameError::InvalidEpoch)?,
                input.u64()?,
            )
            .map_err(|_| FrameError::InvalidPolicy)?;
            if state.incarnation != identity.incarnation_id {
                return Err(FrameError::StateIdentityMismatch);
            }
            if state.data_revision > decision.seq {
                return Err(FrameError::InvalidTerminalStamp);
            }
            Lifecycle::Live(LiveAuthority {
                access,
                decision,
                state,
                receipts,
            })
        }
        (_, 1) => {
            let operation = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
            let reason = match input.tag()? {
                (_, 0) => DeletedReason::Erasure,
                (_, 1) => DeletedReason::MigrationAborted {
                    source_database: super::DatabaseId::from_core(bumbledb::Id128::from_bytes(
                        input.array()?,
                    )),
                    source_incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(
                        input.array()?,
                    )),
                    plan_set_digest: input.array()?,
                },
                (at, got) => return Err(FrameError::Tag { at, got }),
            };
            Lifecycle::Deleted { operation, reason }
        }
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    let activation = match input.tag()? {
        (_, 0) => Activation::NotActivated,
        (_, 1) => {
            let operation = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
            let target_genesis = DecisionDigest::from_bytes(input.array()?);
            let cause = match input.tag()? {
                (_, 0) => ActivationCause::Create,
                (_, 1) => ActivationCause::Restore,
                (_, 2) => ActivationCause::Migration {
                    plan_set_digest: input.array()?,
                },
                (at, got) => return Err(FrameError::Tag { at, got }),
            };
            Activation::Activated {
                operation,
                target_genesis,
                cause,
            }
        }
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    input.end()?;
    Ok(HeadAuthority {
        identity,
        revision,
        lifecycle,
        activation,
    })
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::super::{DatabaseId, SchemaId};
    use super::*;

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    fn genesis() -> HeadAuthority {
        HeadAuthority::genesis(
            identity(),
            DecisionStamp {
                seq: 0,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            Activation::Activated {
                operation: OperationId::from_core(Id128::from_bytes([4; 16])),
                target_genesis: DecisionDigest::from_bytes([9; 32]),
                cause: ActivationCause::Create,
            },
        )
        .unwrap()
    }

    fn op(byte: u8) -> OperationId {
        OperationId::from_core(Id128::from_bytes([byte; 16]))
    }

    #[test]
    fn decisions_move_all_three_coordinates_distinctly() {
        let head = genesis();
        let decided = head
            .decided(DecisionDigest::from_bytes([7; 32]), true)
            .unwrap();
        assert_eq!(decided.revision.0, 2);
        let live = decided.live().unwrap();
        assert_eq!(live.decision.seq, 1);
        assert_eq!(live.state.data_revision, 1);
        let no_change = decided
            .decided(DecisionDigest::from_bytes([8; 32]), false)
            .unwrap();
        let live = no_change.live().unwrap();
        assert_eq!(live.decision.seq, 2);
        assert_eq!(live.state.data_revision, 1, "no-op keeps the witness");
        let maintained = no_change.maintained().unwrap();
        assert_eq!(maintained.revision.0, 4);
        assert_eq!(maintained.live().unwrap().decision.seq, 2);
        assert_eq!(maintained.live().unwrap().state.data_revision, 1);
    }

    #[test]
    fn freeze_blocks_decisions_and_matching_retries_are_evidence_not_mutation() {
        let head = genesis();
        let intent = FreezeIntent::Migration {
            plan_set_digest: [5; 32],
            target: IncarnationId::from_core(Id128::from_bytes([6; 16])),
        };
        let frozen = match head.freeze(op(10), intent).unwrap() {
            FreezeOutcome::Frozen(frozen) => frozen,
            FreezeOutcome::AlreadyFrozen { .. } => panic!("first freeze mutates"),
        };
        assert!(matches!(
            frozen.decided(DecisionDigest::from_bytes([7; 32]), true),
            Err(AuthorityError::Frozen { .. })
        ));
        assert!(matches!(
            frozen.freeze(op(10), intent).unwrap(),
            FreezeOutcome::AlreadyFrozen { .. }
        ));
        assert!(matches!(
            frozen.freeze(op(11), intent),
            Err(AuthorityError::OperationMismatch { .. })
        ));
        assert!(matches!(
            frozen.rotate_receipts(ReceiptEpoch::new(2).unwrap()),
            Err(AuthorityError::Frozen { .. })
        ));
        let thawed = frozen.thaw(op(10)).unwrap();
        assert!(matches!(thawed.live().unwrap().access, Access::Active));
        assert!(matches!(
            frozen.thaw(op(12)),
            Err(AuthorityError::OperationMismatch { .. })
        ));
        assert!(matches!(
            thawed.thaw(op(10)),
            Err(AuthorityError::NotFrozen)
        ));
    }

    #[test]
    fn activation_is_one_time_and_matching_retry_reports_current_access() {
        let target = HeadAuthority {
            activation: Activation::NotActivated,
            ..genesis()
        };
        let cause = ActivationCause::Migration {
            plan_set_digest: [5; 32],
        };
        let genesis_hash = DecisionDigest::from_bytes([9; 32]);
        // Activation requires the matching freeze (AwaitingCutover).
        assert!(matches!(
            target.activate(op(20), genesis_hash, cause),
            Err(AuthorityError::NotFrozen)
        ));
        let frozen = match target
            .freeze(
                op(20),
                FreezeIntent::Migration {
                    plan_set_digest: [5; 32],
                    target: identity().incarnation_id,
                },
            )
            .unwrap()
        {
            FreezeOutcome::Frozen(frozen) => frozen,
            FreezeOutcome::AlreadyFrozen { .. } => unreachable!(),
        };
        let activated = match frozen.activate(op(20), genesis_hash, cause).unwrap() {
            ActivateOutcome::Activated(head) => head,
            ActivateOutcome::AlreadyActivated { .. } => panic!("first activation mutates"),
        };
        assert!(matches!(activated.live().unwrap().access, Access::Active));
        // Matching retry after a LATER freeze reports Frozen and does not thaw.
        let later = match activated.freeze(op(30), FreezeIntent::Erasure).unwrap() {
            FreezeOutcome::Frozen(frozen) => frozen,
            FreezeOutcome::AlreadyFrozen { .. } => unreachable!(),
        };
        match later.activate(op(20), genesis_hash, cause).unwrap() {
            ActivateOutcome::AlreadyActivated { access, .. } => {
                assert_eq!(access, AccessMode::Frozen);
            }
            ActivateOutcome::Activated(_) => panic!("retry must not mutate"),
        }
        assert!(matches!(
            later.activate(op(21), genesis_hash, cause),
            Err(AuthorityError::ActivationEvidenceMismatch)
        ));
    }

    #[test]
    fn deletion_is_terminal_and_cancellation_races_activation_correctly() {
        let head = genesis();
        let reason = DeletedReason::Erasure;
        let deleted = match head.delete(op(40), reason).unwrap() {
            DeleteOutcome::Deleted(deleted) => deleted,
            DeleteOutcome::AlreadyDeleted { .. } => unreachable!(),
        };
        assert!(matches!(
            deleted.delete(op(40), reason).unwrap(),
            DeleteOutcome::AlreadyDeleted { .. }
        ));
        assert!(matches!(
            deleted.delete(op(41), reason),
            Err(AuthorityError::OperationMismatch { .. })
        ));
        for illegal in [
            deleted.decided(DecisionDigest::from_bytes([7; 32]), true),
            deleted.maintained(),
            deleted.thaw(op(40)),
        ] {
            assert!(matches!(
                illegal,
                Err(AuthorityError::Deleted | AuthorityError::NotFrozen)
            ));
        }
        assert!(
            deleted.admission_view().is_none(),
            "refuse before hydration"
        );
        // A target whose activation already won cannot be auto-aborted.
        let abort = DeletedReason::MigrationAborted {
            source_database: identity().database_id,
            source_incarnation: identity().incarnation_id,
            plan_set_digest: [5; 32],
        };
        let activated_target = HeadAuthority {
            activation: Activation::Activated {
                operation: op(50),
                target_genesis: DecisionDigest::from_bytes([9; 32]),
                cause: ActivationCause::Migration {
                    plan_set_digest: [5; 32],
                },
            },
            ..genesis()
        };
        assert!(matches!(
            activated_target.delete(op(50), abort),
            Err(AuthorityError::ActivationEvidenceMismatch)
        ));
        // A pre-genesis cancellation tombstone fences delayed genesis.
        let fenced = HeadAuthority::cancelled_before_genesis(identity(), op(50), abort);
        assert!(matches!(fenced.lifecycle, Lifecycle::Deleted { .. }));
        assert_eq!(fenced.activation, Activation::NotActivated);
    }

    #[test]
    fn control_codec_roundtrips_every_lifecycle_and_activation_arm() {
        let heads = [
            genesis(),
            HeadAuthority {
                activation: Activation::NotActivated,
                ..genesis()
            },
            match genesis()
                .freeze(
                    op(1),
                    FreezeIntent::Migration {
                        plan_set_digest: [5; 32],
                        target: IncarnationId::from_core(Id128::from_bytes([6; 16])),
                    },
                )
                .unwrap()
            {
                FreezeOutcome::Frozen(frozen) => frozen,
                FreezeOutcome::AlreadyFrozen { .. } => unreachable!(),
            },
            match genesis().freeze(op(2), FreezeIntent::Erasure).unwrap() {
                FreezeOutcome::Frozen(frozen) => frozen,
                FreezeOutcome::AlreadyFrozen { .. } => unreachable!(),
            },
            match genesis().delete(op(3), DeletedReason::Erasure).unwrap() {
                DeleteOutcome::Deleted(deleted) => deleted,
                DeleteOutcome::AlreadyDeleted { .. } => unreachable!(),
            },
            HeadAuthority::cancelled_before_genesis(
                identity(),
                op(4),
                DeletedReason::MigrationAborted {
                    source_database: identity().database_id,
                    source_incarnation: identity().incarnation_id,
                    plan_set_digest: [7; 32],
                },
            ),
        ];
        for head in heads {
            let bytes = encode_control(&head, 4096).unwrap();
            assert_eq!(decode_control(&bytes, 4096).unwrap(), head, "{head:?}");
            for end in 0..bytes.len() {
                assert!(decode_control(&bytes[..end], 4096).is_err(), "prefix {end}");
            }
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(decode_control(&trailing, 4096).is_err());
        }
    }

    #[test]
    fn decoded_controls_reject_violated_invariants() {
        let head = HeadAuthority {
            activation: Activation::NotActivated,
            ..genesis()
        };
        let bytes = encode_control(&head, 4096).unwrap();
        // Zero head revision.
        let mut zero = bytes.clone();
        let base = FAMILY.len() + 3 + 64;
        zero[base..base + 8].fill(0);
        assert_eq!(
            decode_control(&zero, 4096),
            Err(FrameError::InvalidSequence)
        );
        // Retirement at/after the open epoch. With `NotActivated` the frame
        // tail is [.. open_epoch(8) retired(8) activation_tag(1)].
        let mut policy = bytes;
        let len = policy.len();
        policy[len - 2] = 1; // retired_through = 1 == open epoch
        assert_eq!(
            decode_control(&policy, 4096),
            Err(FrameError::InvalidPolicy)
        );
    }
}
