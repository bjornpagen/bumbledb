//! Epoch-barrier garbage collection: one active barrier, immutable mark
//! evidence, resumable durable sweep progress (chapter 21; GC-01..13).
//!
//! The complete persistent GC state lives in the head. No collector owns
//! authority by a wall-clock lease; multiple collectors may duplicate safe
//! work and the head CAS selects durable progress. A stale collector
//! receives `CollectionMoved`/`AlreadyFinished`, never unconditional
//! installation of old progress. Rebase preserves intervening data changes
//! but cannot regress a cursor, revive a completed barrier, clear a newer
//! barrier, or swap in another job's mark evidence.
//!
//! Deleted 0.x mechanisms: the 90-day wall-clock retention window, restore
//! vectors, GET-only backlink walks, scratch-lease deletion authority and
//! historical token/slot scans. Listing enumerates actual extant names:
//! cost tracks retained/orphan objects, never historical slot count.

use std::collections::BTreeSet;

use bumbledb::{WorkContext, WorkError};

use crate::checkpointer::read_live_head;
use crate::codec::{StreamLimits, decode_manifest};
use crate::history::command::Limits;
use crate::history::authority::{AuthorityError, HeadAuthority};
use crate::history::locator::{self, ChainVisitor};
use crate::history::{FrameError, HeadRevision, OperationId};
use crate::manifest::wire::{self, Reader};
use crate::manifest::{Barrier, GcPhase, HeadError, HeadRecord, RecoveryRoot, encode_head};
use crate::store::{
    BackendError, ConditionalOutcome, ConditionalStore, ObjectError, ObjectKind, ObjectRef,
    ObservedError, ReceivingStore, TransportContext, backend as backend_error,
    get_verified, head_key, objects_prefix, parse_object_key, put_verified,
};

pub const MARK_FAMILY: &[u8] = b"bumbledb.mark.v1\0";
pub const MARK_LAYOUT: u16 = 1;
const MARK_KIND: u8 = 1;

#[derive(Debug, Clone, Copy)]
pub struct GcPolicy {
    pub head_cap: usize,
    pub stream: StreamLimits,
    /// Mark-set budget in encoded bytes, charged before growth. Exceeding it
    /// is a typed refusal without deletion, never silent truncation.
    pub mark_budget_bytes: u64,
    /// Bounded CAS attempts for each durable progress transition.
    pub cas_attempts: u32,
    /// Bounded decision-walk budget per protected root.
    pub walk_budget: u64,
}

impl GcPolicy {
    pub const DEFAULT: Self = Self {
        head_cap: 1024 * 1024,
        stream: StreamLimits::DEFAULT,
        mark_budget_bytes: 256 * 1024 * 1024,
        cas_attempts: 16,
        walk_budget: 1_048_576,
    };
}

#[derive(Debug)]
pub enum GcError {
    Object(ObjectError),
    Frame(FrameError),
    Head(HeadError),
    Work(WorkError),
    /// Another collector/maintenance moved the collection state; re-read and
    /// resume from the durable head, never install stale progress.
    CollectionMoved,
    /// The barrier this collector was driving is already complete.
    AlreadyFinished,
    /// A required protected dependency is malformed/missing: GC stops
    /// without deletion; the tenant needs repair evidence, not a sweep.
    Corruption(&'static str),
    /// The bounded mark budget was exhausted; no deletion certificate exists.
    MarkBudget,
    /// One eligible deletion failed. Durable progress was NOT advanced past
    /// it; resume retries the same key.
    DeleteFailed {
        key: String,
    },
    /// Bounded CAS attempts exhausted by contention.
    CasExhausted,
    Checkpoint(crate::checkpointer::CheckpointError),
}

impl From<ObjectError> for GcError {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}

impl From<FrameError> for GcError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<HeadError> for GcError {
    fn from(error: HeadError) -> Self {
        Self::Head(error)
    }
}

impl From<WorkError> for GcError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<crate::checkpointer::CheckpointError> for GcError {
    fn from(error: crate::checkpointer::CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

impl From<AuthorityError> for GcError {
    fn from(error: AuthorityError) -> Self {
        Self::Head(HeadError::from(error))
    }
}

/// One durable-progress CAS: read the exact current head, let `compose`
/// build the successor from it (preserving intervening data changes), and
/// conditionally replace. `compose` refuses with `CollectionMoved` when the
/// observed state is no longer the one this collector is driving.
fn cas_head<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    policy: &GcPolicy,
    work: &WorkContext,
    mut compose: impl FnMut(&HeadRecord) -> Result<HeadRecord, GcError>,
) -> Result<HeadRecord, GcError>
where
    B::Error: BackendError + ObservedError,
{
    for _ in 0..policy.cas_attempts {
        work.checkpoint()?;
        let (current, version) = read_live_head(backend, prefix, policy.head_cap)?;
        let proposed = compose(&current)?;
        let body = encode_head(&proposed, policy.head_cap)?;
        match backend
            .replace_head(&head_key(prefix), &version, &body)
            .map_err(backend_error)
            .map_err(GcError::Object)?
        {
            ConditionalOutcome::Published { .. } => return Ok(proposed),
            ConditionalOutcome::PreconditionFailed => {}
            ConditionalOutcome::Indeterminate => {
                let (observed, _) = read_live_head(backend, prefix, policy.head_cap)?;
                if observed.gc == proposed.gc && observed.object_epoch == proposed.object_epoch {
                    return Ok(observed);
                }
                // Not observably ours; retry against the durable state.
            }
        }
    }
    Err(GcError::CasExhausted)
}

/// Close the open object epoch: from the exact head with epoch E and
/// `gc = Idle`, capture the immutable protected closure (live recovery root
/// if any, plus every explicitly retained named root), then CAS `E+1` and
/// `Marking { barrier }`. A tombstoned head contributes no live root; an
/// empty closure is valid. Every old publication attempt is invalidated at
/// this CAS; commands continue under the new epoch.
///
/// # Errors
/// A running collection refuses with `CollectionMoved`.
pub fn close_epoch<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<Barrier, GcError>
where
    B::Error: BackendError + ObservedError,
{
    let mut barrier_out = None;
    cas_head(backend, prefix, policy, work, |current| {
        match &current.gc {
            GcPhase::Idle => {}
            GcPhase::Marking { barrier } | GcPhase::Sweeping { barrier, .. } => {
                if barrier.id == operation {
                    barrier_out = Some(barrier.clone());
                    return Err(GcError::AlreadyFinished);
                }
                return Err(GcError::CollectionMoved);
            }
        }
        let barrier = Barrier {
            id: operation,
            cutoff_epoch: current.object_epoch,
            protected: current.protected_closure().into_boxed_slice(),
        };
        barrier_out = Some(barrier.clone());
        // Tombstones still advance their epoch for collection: use the raw
        // record fields (control untouched except revision maintenance where
        // live; deleted controls keep their recorded revision).
        let control = match current.control.maintained() {
            Ok(control) => control,
            Err(AuthorityError::Deleted) => {
                let revision = current
                    .control
                    .revision
                    .0
                    .checked_add(1)
                    .ok_or(GcError::Corruption("head revision exhausted"))?;
                HeadAuthority {
                    revision: HeadRevision(revision),
                    ..current.control
                }
            }
            Err(error) => return Err(error.into()),
        };
        Ok(HeadRecord {
            control,
            recovery: current.recovery,
            roots: current.roots.clone(),
            object_epoch: current
                .object_epoch
                .checked_add(1)
                .ok_or(GcError::Corruption("object epoch exhausted"))?,
            gc: GcPhase::Marking { barrier },
        })
    })
    .map(|_| ())
    .or_else(|error| match (&error, &barrier_out) {
        (GcError::AlreadyFinished, Some(_)) => Ok(()),
        _ => Err(error),
    })?;
    barrier_out.ok_or(GcError::CollectionMoved)
}

/// Walk one protected recovery root's exact dependency closure into `marks`.
/// Validates every object hash/length and bounded grammar before following
/// nested references; a malformed/missing required dependency stops GC
/// without deletion. Chunk leaves are named by a verified manifest and are
/// marked without a body download (they carry no nested references).
#[expect(clippy::too_many_arguments, reason = "one bounded mark walk")]
fn mark_root<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    root: &RecoveryRoot,
    _cutoff: u64,
    limits: Limits,
    policy: &GcPolicy,
    budget: &mut u64,
    marks: &mut BTreeSet<String>,
    work: &WorkContext,
) -> Result<(), GcError>
where
    B::Error: BackendError + ObservedError,
{
    let charge = |key: &String, budget: &mut u64| -> Result<(), GcError> {
        let cost = key.len() as u64 + 16;
        if *budget < cost {
            return Err(GcError::MarkBudget);
        }
        *budget -= cost;
        Ok(())
    };
    if let Some(checkpoint) = &root.checkpoint {
        let charged = get_verified(
            backend,
            prefix,
            checkpoint,
            TransportContext::new(
                work,
                locator::receive_limits_for_object(checkpoint, policy.stream.manifest_bytes),
            ),
        )
        .map_err(|_| GcError::Corruption("protected checkpoint manifest unavailable"))?;
        let manifest = decode_manifest(charged.as_bytes(), policy.stream)?;
        let key = checkpoint.key(prefix);
        charge(&key, budget)?;
        marks.insert(key);
        for chunk in &manifest.chunks {
            work.step(1)?;
            let key = chunk.key(prefix);
            charge(&key, budget)?;
            marks.insert(key);
        }
        drop(charged.into_owner());
    }
    // Decisions of exactly the tail (base, tip]. The shared walker stops at
    // the captured base and never epoch-probes.
    struct MarkVisitor<'a> {
        prefix: &'a str,
        budget: &'a mut u64,
        marks: &'a mut BTreeSet<String>,
        work: &'a WorkContext,
        charge: &'a dyn Fn(&String, &mut u64) -> Result<(), GcError>,
    }
    impl ChainVisitor for MarkVisitor<'_> {
        type Error = GcError;
        fn visit(
            &mut self,
            _stamp: crate::history::DecisionStamp,
            _bytes: &[u8],
            reference: ObjectRef,
        ) -> Result<bool, GcError> {
            self.work.checkpoint()?;
            let key = reference.key(self.prefix);
            (self.charge)(&key, self.budget)?;
            self.marks.insert(key);
            Ok(true)
        }
    }
    let mut walk_budget = policy.walk_budget;
    let mut visitor = MarkVisitor {
        prefix,
        budget,
        marks,
        work,
        charge: &charge,
    };
    locator::walk_decision_chain(
        backend,
        prefix,
        root.tip,
        root.base,
        root.tip_object,
        limits,
        &mut walk_budget,
        work,
        &mut visitor,
    )
    .map_err(|error| match error {
        GcError::Object(ObjectError::Backend(inner))
            if inner.to_string().contains("decision walk budget exhausted") =>
        {
            GcError::MarkBudget
        }
        GcError::Object(ObjectError::Missing { .. }) => {
            GcError::Corruption("parent locator missing before recovery base")
        }
        GcError::Object(ObjectError::WrongDigest { .. }) => {
            GcError::Corruption("protected tail digest mismatch")
        }
        GcError::Object(_) => GcError::Corruption("protected tail decision unavailable"),
        other => other,
    })?;
    Ok(())
}

/// # Errors
/// Oversized mark sets refuse before encoding.
fn encode_marks(barrier: &Barrier, marks: &BTreeSet<String>) -> Result<Vec<u8>, GcError> {
    let mut out = wire::frame_header(MARK_FAMILY, MARK_LAYOUT, MARK_KIND);
    out.extend_from_slice(barrier.id.as_core().as_bytes());
    out.extend_from_slice(&barrier.cutoff_epoch.to_be_bytes());
    out.extend_from_slice(&(marks.len() as u64).to_be_bytes());
    for key in marks {
        let len =
            u16::try_from(key.len()).map_err(|_| GcError::Frame(FrameError::LengthOverflow))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(key.as_bytes());
    }
    Ok(out)
}

/// # Errors
/// Malformed mark manifests and foreign barriers refuse.
pub fn decode_marks(
    bytes: &[u8],
    expected_barrier: OperationId,
    cap: usize,
) -> Result<BTreeSet<String>, GcError> {
    let mut input = Reader::begin(bytes, MARK_FAMILY, MARK_LAYOUT, MARK_KIND, cap)?;
    let id = OperationId::from_core(bumbledb::Id128::from_bytes(
        input.array().map_err(GcError::Frame)?,
    ));
    if id != expected_barrier {
        // Mark evidence from another job can never certify this sweep.
        return Err(GcError::CollectionMoved);
    }
    let _cutoff = input.u64().map_err(GcError::Frame)?;
    let count = input.u64().map_err(GcError::Frame)?;
    let mut marks = BTreeSet::new();
    for _ in 0..count {
        let len = usize::from(u16::from_be_bytes(input.array().map_err(GcError::Frame)?));
        let key = input.take(len).map_err(GcError::Frame)?;
        marks.insert(
            std::str::from_utf8(key)
                .map_err(|_| GcError::Corruption("mark key not utf-8"))?
                .to_string(),
        );
    }
    input.end().map_err(GcError::Frame)?;
    Ok(marks)
}

/// Mark the exact dependency closure of the current barrier and publish the
/// immutable mark manifest, transitioning `Marking → Sweeping`. Only a
/// complete verified mark set can enable a sweep; partial work or a crashed
/// mark task is not a deletion certificate — re-running restarts marking
/// from the durable barrier.
///
/// # Errors
/// Corruption stops GC without deletion; stale collectors get
/// `CollectionMoved`/`AlreadyFinished`.
pub fn mark<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    limits: Limits,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<ObjectRef, GcError>
where
    B::Error: BackendError + ObservedError,
{
    let (head, _) = read_live_head(backend, prefix, policy.head_cap)?;
    let barrier = match &head.gc {
        GcPhase::Marking { barrier } => barrier.clone(),
        GcPhase::Sweeping { marks, .. } => return Ok(*marks),
        GcPhase::Idle => return Err(GcError::AlreadyFinished),
    };
    let mut marks = BTreeSet::new();
    let mut budget = policy.mark_budget_bytes;
    for root in &barrier.protected {
        mark_root(
            backend,
            prefix,
            root,
            barrier.cutoff_epoch,
            limits,
            policy,
            &mut budget,
            &mut marks,
            work,
        )?;
    }
    // New mark-work objects live in the current (open) epoch, outside the
    // collection cutoff, so a concurrent collector cannot collect them.
    let bytes = encode_marks(&barrier, &marks)?;
    let (current, _) = read_live_head(backend, prefix, policy.head_cap)?;
    let marks_ref = put_verified(
        backend,
        prefix,
        current.object_epoch,
        ObjectKind::Mark,
        &bytes,
    )?;
    let installed = cas_head(backend, prefix, policy, work, |current| match &current.gc {
        GcPhase::Marking { barrier: recorded } if recorded.id == barrier.id => {
            let control = current.control.maintained().map_err(GcError::from)?;
            Ok(HeadRecord {
                control,
                recovery: current.recovery,
                roots: current.roots.clone(),
                object_epoch: current.object_epoch,
                gc: GcPhase::Sweeping {
                    barrier: recorded.clone(),
                    marks: marks_ref,
                    cursor: None,
                },
            })
        }
        GcPhase::Sweeping {
            barrier: recorded,
            marks,
            ..
        } if recorded.id == barrier.id => {
            // Another collector completed marking first; its evidence wins.
            let _ = marks;
            Err(GcError::CollectionMoved)
        }
        _ => Err(GcError::CollectionMoved),
    });
    match installed {
        Ok(head) => match head.gc {
            GcPhase::Sweeping { marks, .. } => Ok(marks),
            _ => Err(GcError::Corruption("sweeping install lost its phase")),
        },
        Err(GcError::CollectionMoved) => {
            // Re-read: duplicated safe work is fine; durable evidence wins.
            let (observed, _) = read_live_head(backend, prefix, policy.head_cap)?;
            match observed.gc {
                GcPhase::Sweeping {
                    barrier: recorded,
                    marks,
                    ..
                } if recorded.id == barrier.id => Ok(marks),
                _ => Err(GcError::CollectionMoved),
            }
        }
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SweepReport {
    pub deleted: u64,
    pub retained_marked: u64,
    pub retained_newer: u64,
    pub retained_unparsed: u64,
    pub pages: u64,
    pub finished: bool,
}

/// Sweep only closed names: delete a key exactly when its parsed epoch is
/// `≤ cutoff` AND it is absent from the complete verified mark set. `HEAD`,
/// unknown/unparseable namespaces, newer epochs and marked objects are never
/// deleted. Progress persists in the head by exact CAS after each page; a
/// failed deletion never advances durable progress past itself.
///
/// # Errors
/// Stale collectors get `CollectionMoved`; failed deletions return
/// `DeleteFailed` with resumable durable progress retained.
pub fn sweep<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<SweepReport, GcError>
where
    B::Error: BackendError + ObservedError,
{
    let (head, _) = read_live_head(backend, prefix, policy.head_cap)?;
    let (barrier, marks_ref, mut cursor) = match &head.gc {
        GcPhase::Sweeping {
            barrier,
            marks,
            cursor,
        } => (barrier.clone(), *marks, cursor.clone()),
        GcPhase::Marking { .. } => return Err(GcError::CollectionMoved),
        GcPhase::Idle => return Err(GcError::AlreadyFinished),
    };
    let charged = get_verified(
        backend,
        prefix,
        &marks_ref,
        TransportContext::new(
            work,
            locator::receive_limits_for_object(&marks_ref, policy.stream.manifest_bytes),
        ),
    )
    .map_err(|_| GcError::Corruption("mark manifest unavailable"))?;
    let marks = decode_marks(charged.as_bytes(), barrier.id, policy.stream.manifest_bytes)?;
    drop(charged.into_owner());
    let mut report = SweepReport::default();
    let listing_prefix = objects_prefix(prefix);
    loop {
        work.checkpoint()?;
        let page = backend
            .list_objects(&listing_prefix, cursor.as_deref())
            .map_err(backend_error)
            .map_err(GcError::Object)?;
        report.pages += 1;
        let mut last_processed: Option<String> = None;
        for key in &page.keys {
            work.step(1)?;
            match parse_object_key(prefix, key) {
                None => {
                    // Never delete an unknown or unparseable namespace.
                    report.retained_unparsed += 1;
                }
                Some((epoch, _, _)) if epoch > barrier.cutoff_epoch => {
                    report.retained_newer += 1;
                }
                Some(_) if marks.contains(key) => {
                    report.retained_marked += 1;
                }
                Some(_) => {
                    if let Ok(()) = backend.delete_object(key) {
                        report.deleted += 1;
                    } else {
                        // Persist progress up to (not past) the failure,
                        // then surface the retryable evidence.
                        if let Some(done) = &last_processed {
                            persist_cursor(
                                backend,
                                prefix,
                                policy,
                                work,
                                &barrier,
                                marks_ref,
                                Some(done.as_bytes()),
                            )?;
                        }
                        return Err(GcError::DeleteFailed { key: key.clone() });
                    }
                }
            }
            last_processed = Some(key.clone());
        }
        // L11: ListPage.next is the last fully processed canonical key on a
        // complete page, not an opaque provider token.
        match page.next {
            Some(next) => {
                persist_cursor(
                    backend,
                    prefix,
                    policy,
                    work,
                    &barrier,
                    marks_ref,
                    Some(&next),
                )?;
                cursor = Some(next);
            }
            None => break,
        }
    }
    // Finish: CAS gc = Idle, retaining the monotone object epoch. The mark
    // manifest becomes an ordinary unreachable object for a later pass.
    cas_head(backend, prefix, policy, work, |current| match &current.gc {
        GcPhase::Sweeping {
            barrier: recorded, ..
        } if recorded.id == barrier.id => {
            let control = current.control.maintained().map_err(GcError::from)?;
            Ok(HeadRecord {
                control,
                recovery: current.recovery,
                roots: current.roots.clone(),
                object_epoch: current.object_epoch,
                gc: GcPhase::Idle,
            })
        }
        GcPhase::Idle => Err(GcError::AlreadyFinished),
        _ => Err(GcError::CollectionMoved),
    })
    .map(|_| ())
    .or_else(|error| match error {
        GcError::AlreadyFinished => Ok(()),
        other => Err(other),
    })?;
    report.finished = true;
    Ok(report)
}

fn persist_cursor<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    policy: &GcPolicy,
    work: &WorkContext,
    barrier: &Barrier,
    marks_ref: ObjectRef,
    cursor: Option<&[u8]>,
) -> Result<(), GcError>
where
    B::Error: BackendError + ObservedError,
{
    cas_head(backend, prefix, policy, work, |current| match &current.gc {
        GcPhase::Sweeping {
            barrier: recorded,
            marks,
            cursor: held_cursor,
        } if recorded.id == barrier.id && *marks == marks_ref => {
            // A last-completed canonical key never regresses: durable
            // progress wins over a stale collector's older page.
            if let (Some(held_cursor), Some(proposed)) = (held_cursor.as_deref(), cursor)
                && held_cursor > proposed
            {
                return Err(GcError::CollectionMoved);
            }
            let control = current.control.maintained().map_err(GcError::from)?;
            Ok(HeadRecord {
                control,
                recovery: current.recovery,
                roots: current.roots.clone(),
                object_epoch: current.object_epoch,
                gc: GcPhase::Sweeping {
                    barrier: recorded.clone(),
                    marks: *marks,
                    cursor: cursor.map(Box::from),
                },
            })
        }
        _ => Err(GcError::CollectionMoved),
    })
    .map(|_| ())
}

/// One complete collection pass: close the epoch (or adopt the running
/// barrier), mark, sweep. Resumable at every durable boundary.
///
/// # Errors
/// Every phase's typed refusal propagates with durable progress retained.
pub fn run_collection<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    limits: Limits,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<SweepReport, GcError>
where
    B::Error: BackendError + ObservedError,
{
    let (head, _) = read_live_head(backend, prefix, policy.head_cap)?;
    match &head.gc {
        GcPhase::Idle => {
            close_epoch(backend, prefix, operation, policy, work)?;
        }
        GcPhase::Marking { .. } | GcPhase::Sweeping { .. } => {
            // Resume the running collection; duplicated work is safe.
        }
    }
    let (head, _) = read_live_head(backend, prefix, policy.head_cap)?;
    if matches!(head.gc, GcPhase::Marking { .. }) {
        mark(backend, prefix, limits, policy, work)?;
    }
    sweep(backend, prefix, policy, work)
}
