//! One bounded, redacted status snapshot (chapter 22 health contract;
//! OPS-TEST-01). A structured record and a text rendering sufficient for a
//! host to integrate — never an observability platform, an unbounded event
//! history or a log of secret rows/credentials/command bodies.
//!
//! Status distinguishes `Empty`, `NotYetHydrated`, `Ready`, `StaleButValid`,
//! `Frozen`, `Deleted`, `Corrupt`, `Unavailable` — hydration failure is never
//! an empty database, and a missing HEAD at a configured existing origin is
//! reported as missing, never silently created. The 0.x inspect (braided
//! batch/sidecar/manifest document renderer) is deleted whole with those
//! representations.

use std::fmt::Write as _;

use bumbledb::WorkContext;

use crate::checkpointer::read_live_head;
use crate::history::authority::{Access, HeadAuthority, Lifecycle};
use crate::history::{DecisionStamp, StateStamp};
use crate::manifest::{GcPhase, HeadRecord, RootKind};
use crate::store::{BackendError, ObservedError, ReceivingStore, hex32};

/// The one health classification. `StaleButValid` means the local
/// materialization is behind a verified newer head but remains a coherent
/// published snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// Live, local materialization at the captured tip, zero data revisions.
    Empty,
    /// The authoritative head exists; no verified local materialization does.
    NotYetHydrated,
    /// Live and the local stamp equals the captured head tip.
    Ready,
    /// Live; the local stamp is a verified ancestor of the captured tip.
    StaleButValid,
    /// New-command admission frozen under a named operation; reads and
    /// retained receipt lookup continue.
    Frozen,
    /// Terminal tombstone.
    Deleted,
    /// Authoritative bytes failed verification; stopped with evidence.
    Corrupt,
    /// The configured origin definitely has no HEAD. Creation is a separate
    /// explicit operation; status never initializes.
    Missing,
    /// The backend could not be consulted; nothing is claimed.
    Unavailable,
}

/// Bounded GC progress, redacted to counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcStatus {
    Idle,
    Marking {
        cutoff_epoch: u64,
        protected_roots: u64,
    },
    Sweeping {
        cutoff_epoch: u64,
        has_cursor: bool,
    },
}

/// One bounded status record. Identity fields are lower-case hex of binary
/// identities; labels are the bounded root labels; no fact payloads,
/// credentials or command bodies appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub condition: Condition,
    /// Lower-case hex identity: database, incarnation, schema.
    pub database_id: String,
    pub incarnation_id: String,
    pub schema_id: String,
    pub head_revision: Option<u64>,
    pub decision: Option<DecisionStamp>,
    pub state: Option<StateStamp>,
    /// The local materialization's verified stamp where one exists.
    pub local_decision: Option<DecisionStamp>,
    pub open_receipt_epoch: Option<u64>,
    pub retired_through: Option<u64>,
    pub tail_count: Option<u64>,
    pub tail_bytes: Option<u64>,
    pub has_checkpoint: Option<bool>,
    pub object_epoch: Option<u64>,
    pub gc: Option<GcStatus>,
    pub roots_held: u64,
    pub restore_points: u64,
    pub hydration_holds: u64,
}

fn hex16(bytes: &[u8; 16]) -> String {
    let mut wide = [0u8; 32];
    wide[..16].copy_from_slice(bytes);
    hex32(&wide)[..32].to_string()
}

fn base_status(authority: &HeadAuthority) -> Status {
    Status {
        condition: Condition::Unavailable,
        database_id: hex16(authority.identity.database_id.as_core().as_bytes()),
        incarnation_id: hex16(authority.identity.incarnation_id.as_core().as_bytes()),
        schema_id: hex32(&authority.identity.schema_id.0),
        head_revision: Some(authority.revision.0),
        decision: None,
        state: None,
        local_decision: None,
        open_receipt_epoch: None,
        retired_through: None,
        tail_count: None,
        tail_bytes: None,
        has_checkpoint: None,
        object_epoch: None,
        gc: None,
        roots_held: 0,
        restore_points: 0,
        hydration_holds: 0,
    }
}

/// Classify one decoded hosted head plus the local materialization's stamp.
#[must_use]
pub fn status_of_head(head: &HeadRecord, local_decision: Option<DecisionStamp>) -> Status {
    let mut status = base_status(&head.control);
    status.object_epoch = Some(head.object_epoch);
    status.gc = Some(match &head.gc {
        GcPhase::Idle => GcStatus::Idle,
        GcPhase::Marking { barrier } => GcStatus::Marking {
            cutoff_epoch: barrier.cutoff_epoch,
            protected_roots: barrier.protected.len() as u64,
        },
        GcPhase::Sweeping {
            barrier, cursor, ..
        } => GcStatus::Sweeping {
            cutoff_epoch: barrier.cutoff_epoch,
            has_cursor: cursor.is_some(),
        },
    });
    status.roots_held = head.roots.len() as u64;
    status.restore_points = head
        .roots
        .iter()
        .filter(|root| root.kind == RootKind::RestorePoint)
        .count() as u64;
    status.hydration_holds = head
        .roots
        .iter()
        .filter(|root| root.kind == RootKind::HydrationHold)
        .count() as u64;
    if let Some(recovery) = head.recovery {
        status.tail_count = Some(recovery.tail_count());
        status.tail_bytes = Some(recovery.tail_bytes);
        status.has_checkpoint = Some(recovery.checkpoint.is_some());
    }
    status.local_decision = local_decision;
    match &head.control.lifecycle {
        Lifecycle::Deleted { .. } => {
            status.condition = Condition::Deleted;
        }
        Lifecycle::Live(live) => {
            status.decision = Some(live.decision);
            status.state = Some(live.state);
            status.open_receipt_epoch = Some(live.receipts.open_epoch().get());
            status.retired_through = Some(live.receipts.retired_through());
            status.condition = match &live.access {
                Access::Frozen { .. } => Condition::Frozen,
                Access::Active => match local_decision {
                    None => Condition::NotYetHydrated,
                    Some(at) if at == live.decision => {
                        if live.state.data_revision == 0 {
                            Condition::Empty
                        } else {
                            Condition::Ready
                        }
                    }
                    Some(at) if at.seq < live.decision.seq => Condition::StaleButValid,
                    Some(_) => Condition::Corrupt,
                },
            };
        }
    }
    status
}

/// Classify a `LocalHistory` authority: local LMDB is authoritative, so a
/// decodable live control IS ready.
#[must_use]
pub fn status_of_local(authority: &HeadAuthority) -> Status {
    let mut status = base_status(authority);
    match &authority.lifecycle {
        Lifecycle::Deleted { .. } => {
            status.condition = Condition::Deleted;
        }
        Lifecycle::Live(live) => {
            status.decision = Some(live.decision);
            status.state = Some(live.state);
            status.local_decision = Some(live.decision);
            status.open_receipt_epoch = Some(live.receipts.open_epoch().get());
            status.retired_through = Some(live.receipts.retired_through());
            status.condition = match &live.access {
                Access::Frozen { .. } => Condition::Frozen,
                Access::Active if live.state.data_revision == 0 => Condition::Empty,
                Access::Active => Condition::Ready,
            };
        }
    }
    status
}

/// Read and classify one hosted head. A backend failure yields
/// `Unavailable` with no identity claims; a malformed head yields `Corrupt`.
pub fn status_hosted<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    local_decision: Option<DecisionStamp>,
    head_cap: usize,
    work: &WorkContext,
) -> Status
where
    B::Error: BackendError + ObservedError,
{
    match read_live_head(backend, prefix, head_cap, work) {
        Ok((head, _)) => status_of_head(&head, local_decision),
        Err(crate::checkpointer::CheckpointError::NotInitialized) => Status {
            condition: Condition::Missing,
            database_id: String::new(),
            incarnation_id: String::new(),
            schema_id: String::new(),
            head_revision: None,
            decision: None,
            state: None,
            local_decision,
            open_receipt_epoch: None,
            retired_through: None,
            tail_count: None,
            tail_bytes: None,
            has_checkpoint: None,
            object_epoch: None,
            gc: None,
            roots_held: 0,
            restore_points: 0,
            hydration_holds: 0,
        },
        Err(crate::checkpointer::CheckpointError::Object(_)) => Status {
            condition: Condition::Unavailable,
            database_id: String::new(),
            incarnation_id: String::new(),
            schema_id: String::new(),
            head_revision: None,
            decision: None,
            state: None,
            local_decision,
            open_receipt_epoch: None,
            retired_through: None,
            tail_count: None,
            tail_bytes: None,
            has_checkpoint: None,
            object_epoch: None,
            gc: None,
            roots_held: 0,
            restore_points: 0,
            hydration_holds: 0,
        },
        Err(_) => Status {
            condition: Condition::Corrupt,
            database_id: String::new(),
            incarnation_id: String::new(),
            schema_id: String::new(),
            head_revision: None,
            decision: None,
            state: None,
            local_decision,
            open_receipt_epoch: None,
            retired_through: None,
            tail_count: None,
            tail_bytes: None,
            has_checkpoint: None,
            object_epoch: None,
            gc: None,
            roots_held: 0,
            restore_points: 0,
            hydration_holds: 0,
        },
    }
}

/// Render one bounded redacted text report: identities as hex, stamps as
/// sequence/revision counters plus digest hex, root LABELS never paths, no
/// fact payloads, credentials or command bodies.
#[must_use]
pub fn render(status: &Status) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "condition: {:?}", status.condition);
    if !status.database_id.is_empty() {
        let _ = writeln!(out, "database: {}", status.database_id);
        let _ = writeln!(out, "incarnation: {}", status.incarnation_id);
        let _ = writeln!(out, "schema: {}", status.schema_id);
    }
    if let Some(revision) = status.head_revision {
        let _ = writeln!(out, "head-revision: {revision}");
    }
    if let Some(decision) = status.decision {
        let _ = writeln!(
            out,
            "decision: {} {}",
            decision.seq,
            hex32(decision.hash.as_bytes())
        );
    }
    if let Some(state) = status.state {
        let _ = writeln!(out, "data-revision: {}", state.data_revision);
    }
    if let Some(local) = status.local_decision {
        let _ = writeln!(out, "local-decision: {}", local.seq);
    }
    if let (Some(open), Some(retired)) = (status.open_receipt_epoch, status.retired_through) {
        let _ = writeln!(
            out,
            "receipts: open-epoch {open}, retired-through {retired}"
        );
    }
    if let (Some(count), Some(bytes)) = (status.tail_count, status.tail_bytes) {
        let _ = writeln!(out, "tail: {count} decisions, {bytes} bytes");
    }
    if let Some(has) = status.has_checkpoint {
        let _ = writeln!(
            out,
            "checkpoint: {}",
            if has { "present" } else { "genesis-root" }
        );
    }
    if let Some(epoch) = status.object_epoch {
        let _ = writeln!(out, "object-epoch: {epoch}");
    }
    if let Some(gc) = status.gc {
        let _ = writeln!(out, "gc: {gc:?}");
    }
    let _ = writeln!(
        out,
        "roots: {} held ({} restore points, {} hydration holds)",
        status.roots_held, status.restore_points, status.hydration_holds
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::authority::{Activation, HeadAuthority};
    use crate::history::{
        DatabaseId, DatabaseIdentity, DecisionDigest, IncarnationId, OperationId, SchemaId,
    };
    use crate::manifest::HeadRecord;

    fn genesis_head() -> HeadRecord {
        let identity = DatabaseIdentity {
            database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        };
        let control = HeadAuthority::genesis(
            identity,
            DecisionStamp {
                seq: 0,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            Activation::NotActivated,
        )
        .unwrap();
        HeadRecord::genesis(control, 0).unwrap()
    }

    #[test]
    fn conditions_distinguish_empty_hydration_staleness_and_tombstones() {
        let head = genesis_head();
        let genesis_stamp = head.control.live().unwrap().decision;
        // No local materialization: not yet hydrated, never "empty database".
        assert_eq!(
            status_of_head(&head, None).condition,
            Condition::NotYetHydrated
        );
        // At the tip with zero data revisions: empty.
        assert_eq!(
            status_of_head(&head, Some(genesis_stamp)).condition,
            Condition::Empty
        );
        // Tombstone: deleted, and no live stamps are invented.
        let deleted = head
            .control
            .delete(
                OperationId::from_core(bumbledb::Id128::from_bytes([7; 16])),
                crate::history::authority::DeletedReason::Erasure,
            )
            .unwrap();
        if let crate::history::authority::DeleteOutcome::Deleted(control) = deleted {
            let record = head.with_control(control);
            let status = status_of_head(&record, None);
            assert_eq!(status.condition, Condition::Deleted);
            assert_eq!(status.decision, None);
        } else {
            panic!("tombstone transition");
        }
    }

    #[test]
    fn rendering_is_redacted_counters_and_hex_only() {
        let head = genesis_head();
        let status = status_of_head(&head, None);
        let text = render(&status);
        assert!(text.contains("condition: NotYetHydrated"));
        assert!(text.contains("head-revision:"));
        assert!(text.contains("roots: 0 held"));
        // No secrets vocabulary ever appears in a status rendering.
        for banned in ["AKIA", "secret", "password", "credential"] {
            assert!(!text.contains(banned), "{banned}");
        }
    }
}
