//! Erasure is a lifecycle across representations (chapter 22; ERASE-01..04).
//!
//! Two distinct scopes, never confused:
//!
//! - **Application fact removal** is an ordinary admitted command through the
//!   history machine. A later checked rebuild/checkpoint omits unreachable
//!   values from the NEW materialization; retained named roots intentionally
//!   keep their older evidence until explicit release.
//! - **Whole-tenant erasure** freezes, settles/reports outstanding
//!   uncertainty (the caller's duty over its retained refs), tombstones the
//!   active head through the Deleted authority, releases only the
//!   policy-allowed named roots, collects former live objects under the
//!   ordinary GC barrier, and reports RESIDUAL copies instead of claiming
//!   secure erasure.
//!
//! The tombstoned head/incarnation marker is deliberately retained: it
//! prevents recreation by stale writers and contains no application facts or
//! receipt payloads. Backups, exports, declared blobs and encryption keys
//! are separately governed; ordinary GC never owns the backup namespace, and
//! this module never derives a backup destination from the source. Deleting
//! LMDB files does not overwrite SSD blocks; nothing here claims it does.

use bumbledb::{Db, WorkContext};

use crate::admin::{self, AdminError};
use crate::checkpointer::read_live_head;
use crate::gc::{self, GcError, GcPolicy, SweepReport};
use crate::history::OperationId;
use crate::history::authority::{DeleteOutcome, DeletedReason};
use crate::history::command::Limits;
use crate::store::{BackendError, ConditionalStore, backend as backend_error, objects_prefix};

#[derive(Debug)]
pub enum EraseError {
    Admin(AdminError),
    Gc(GcError),
    /// A named root the plan wanted released does not exist; a stale release
    /// cannot remove a different root.
    UnknownRoot(OperationId),
}

impl From<AdminError> for EraseError {
    fn from(error: AdminError) -> Self {
        Self::Admin(error)
    }
}
impl From<GcError> for EraseError {
    fn from(error: GcError) -> Self {
        Self::Gc(error)
    }
}

/// What remains AFTER a completed whole-tenant erasure pass — the honest
/// residual inventory, never a "securely erased" claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualReport {
    /// The tombstoned head object itself: retained identity metadata with no
    /// application facts or receipt payloads.
    pub head_tombstone_retained: bool,
    /// Objects still extant under the tenant's `objects/` namespace —
    /// explicitly retained roots' closures plus not-yet-collected orphans.
    pub remaining_objects: u64,
    /// Explicitly retained named roots that were honored, with labels.
    pub retained_roots: Vec<(OperationId, Box<str>)>,
    /// Separately governed copies this operation deliberately did NOT touch.
    /// Always true in 1.0: backups/exports/blobs/keys are separate policy.
    pub backups_exports_blobs_keys_untouched: bool,
}

/// One completed whole-tenant erasure pass.
#[derive(Debug)]
pub struct EraseReport {
    pub tombstone: DeleteOutcome,
    pub released_roots: Vec<OperationId>,
    pub sweep: SweepReport,
    pub residual: ResidualReport,
}

/// Erase a hosted tenant under a retention policy: tombstone the active
/// head (dropping its active recovery root, preserving explicitly retained
/// roots and any running barrier's progress), release exactly the
/// policy-allowed roots, then run one collection pass so former live
/// objects become collectible. Resumable: every transition is
/// evidence-idempotent under the same operation ID.
///
/// The caller must FIRST settle or explicitly record outstanding unknown
/// commands (chapter 22): this function does not resolve them, and it
/// cannot — erasure with unresolved uncertainty is a documented operator
/// decision, not a library default.
///
/// # Errors
/// Authority conflicts (activation won, foreign tombstone), unknown roots
/// and backend/GC refusals; durable progress is retained across retries.
pub fn erase_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    release_roots: &[OperationId],
    limits: Limits,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<EraseReport, EraseError>
where
    B::Error: BackendError,
{
    // 1. Release exactly the policy-allowed roots — while the head is still
    //    live, so the release is an ordinary maintained transition. Releases
    //    are idempotent under retry (`already_released_ok`).
    let mut released = Vec::with_capacity(release_roots.len());
    for root in release_roots {
        admin::release_named_root_hosted(backend, prefix, *root, true, policy.head_cap, work)?;
        released.push(*root);
    }
    // 2. Terminal tombstone through the Deleted authority. Explicit retained
    //    roots and a running barrier's progress survive inside the head.
    let tombstone = admin::tombstone_hosted(
        backend,
        prefix,
        operation,
        DeletedReason::Erasure,
        policy.head_cap,
        work,
    )?;
    // 3. One collection pass: the tombstone contributes no live recovery
    //    root, so former live objects outside retained roots become
    //    collectible now (or in a later pass for barrier-protected ones).
    let sweep = gc::run_collection(backend, prefix, operation, limits, policy, work)?;
    // 4. The honest residual inventory.
    let residual = residual_report(backend, prefix, policy, work)?;
    Ok(EraseReport {
        tombstone,
        released_roots: released,
        sweep,
        residual,
    })
}

/// Enumerate what actually remains: the tombstone, extant objects, retained
/// roots. Read-only; usable independently of an erase pass (ERASE-03).
///
/// # Errors
/// Backend and head-decode refusals.
pub fn residual_report<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    policy: &GcPolicy,
    work: &WorkContext,
) -> Result<ResidualReport, EraseError>
where
    B::Error: BackendError,
{
    let (head, _) = read_live_head(backend, prefix, policy.head_cap).map_err(AdminError::from)?;
    let mut remaining = 0u64;
    let listing_prefix = objects_prefix(prefix);
    let mut cursor: Option<Box<[u8]>> = None;
    loop {
        work.checkpoint().map_err(AdminError::from)?;
        let page = backend
            .list_objects(&listing_prefix, cursor.as_deref())
            .map_err(backend_error)
            .map_err(AdminError::Object)?;
        remaining += page.keys.len() as u64;
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    Ok(ResidualReport {
        head_tombstone_retained: head.control.live().is_err(),
        remaining_objects: remaining,
        retained_roots: head
            .roots
            .iter()
            .map(|root| (root.id, root.label.clone()))
            .collect(),
        backups_exports_blobs_keys_untouched: true,
    })
}

/// Erase a `LocalHistory` tenant's authority: the terminal tombstone commits in
/// one LMDB transaction. The facts/receipt rows physically remain in the
/// tombstoned materialization until the owner closes and removes the cache
/// directory under its kernel lock — which this function deliberately does
/// not do behind the caller's back; report, then let the owner tear down.
///
/// # Errors
/// Authority conflicts and storage refusals.
pub fn erase_local<S>(
    db: &Db<S>,
    operation: OperationId,
    cap: usize,
    work: &WorkContext,
) -> Result<DeleteOutcome, EraseError> {
    Ok(admin::tombstone_local(
        db,
        operation,
        DeletedReason::Erasure,
        cap,
        work,
    )?)
}
