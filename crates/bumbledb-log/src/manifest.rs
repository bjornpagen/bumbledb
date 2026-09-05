//! The hosted HEAD body (C08): P05-owned retention fields composed around
//! P04's authority control projection.
//!
//! ```text
//! HeadRecord {
//!   control: HeadAuthority   (P04's exact projection bytes, embedded)
//!   object_epoch,            (advanced only by the GC barrier CAS)
//!   recovery: RecoveryRoot?, (checkpoint at S plus exactly the decisions (S,T])
//!   roots: bounded list<NamedRoot>,
//!   gc: Idle | Marking { barrier } | Sweeping { barrier, marks, cursor }
//! }
//! ```
//!
//! Every composition here is a **value**; publication is the backend's
//! conditional replacement. The head is bounded metadata, never the receipt
//! database; large snapshot manifests are streamed objects ([`crate::codec`]).
//! A full root list returns `RootCapacityExceeded` and never discards another
//! root. `Deleted` heads carry no active recovery root — only explicitly
//! retained named roots and a running barrier keep protecting objects.
//! Physical bytes remain provisional until the F3 format freeze (C12).

use crate::history::authority::{HeadAuthority, Lifecycle, decode_control, encode_control};
use crate::history::{
    DecisionDigest, DecisionStamp, FrameError, IncarnationId, OperationId, StateStamp,
};
use crate::store::{ObjectKind, ObjectRef};

pub const FAMILY: &[u8] = b"bumbledb.head.v1\0";
pub const LAYOUT: u16 = 1;
const HEAD: u8 = 1;

/// Metadata/resource policy bounds for the named-root list — not a
/// database-size limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootPolicy {
    pub max_roots: usize,
    pub max_label_bytes: usize,
}

impl RootPolicy {
    pub const DEFAULT: Self = Self {
        max_roots: 64,
        max_label_bytes: 128,
    };
}

/// One configurable finite tail envelope in count AND bytes. The values are
/// deployment-qualified policy, not correctness constants; `UNBOUNDED` exists
/// for compositions whose caller enforces the envelope elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TailPolicy {
    pub max_count: u64,
    pub max_bytes: u64,
}

impl TailPolicy {
    pub const UNBOUNDED: Self = Self {
        max_count: u64::MAX,
        max_bytes: u64::MAX,
    };
}

/// "This checkpoint at `base` plus exactly the decisions `(base, tip]`" —
/// the recovery walker and the retention walker share this stopping
/// boundary. `checkpoint: None` is a genesis root: recovery replays the
/// whole chain from the sequence-zero sentinel. `epoch_floor` is the oldest
/// object epoch a tail decision of this root can live under, bounding the
/// fetch probe window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryRoot {
    pub checkpoint: Option<ObjectRef>,
    pub base: DecisionStamp,
    pub tip: DecisionStamp,
    pub tail_bytes: u64,
    pub epoch_floor: u64,
}

impl RecoveryRoot {
    #[must_use]
    pub const fn tail_count(&self) -> u64 {
        self.tip.seq.saturating_sub(self.base.seq)
    }

    /// Genesis recovery: no checkpoint object yet; the whole chain is the tail.
    #[must_use]
    pub const fn genesis(stamp: DecisionStamp, object_epoch: u64) -> Self {
        Self {
            checkpoint: None,
            base: stamp,
            tip: stamp,
            tail_bytes: 0,
            epoch_floor: object_epoch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    RestorePoint,
    HydrationHold,
}

/// One named root: an explicit retention pin with its exact captured
/// recovery closure and stamps. Captured control is provenance, not
/// permission to re-activate a restored old authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRoot {
    /// Unique root identity; never reused.
    pub id: OperationId,
    pub kind: RootKind,
    pub recovery: RecoveryRoot,
    pub state: StateStamp,
    /// Bounded display label; never a path component.
    pub label: Box<str>,
    /// The administrative operation that created this root.
    pub operation: OperationId,
}

/// The immutable protected closure of one collection: captured at the
/// epoch-closing CAS, never widened or swapped afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barrier {
    pub id: OperationId,
    pub cutoff_epoch: u64,
    pub protected: Box<[RecoveryRoot]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcPhase {
    Idle,
    Marking {
        barrier: Barrier,
    },
    Sweeping {
        barrier: Barrier,
        /// The immutable checked mark manifest (current-epoch object).
        marks: ObjectRef,
        /// Resume-after listing key; deletion progress never advances past a
        /// failed required deletion.
        cursor: Option<Box<[u8]>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadRecord {
    pub control: HeadAuthority,
    pub object_epoch: u64,
    pub recovery: Option<RecoveryRoot>,
    pub roots: Vec<NamedRoot>,
    pub gc: GcPhase,
}

/// Composition refusals distinct from frame grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadError {
    Frame(FrameError),
    /// The authority is a terminal tombstone; the requested composition
    /// needs a live head.
    Deleted,
    /// A live head is missing its recovery root — corruption-class.
    NoRecovery,
    /// Admission would exceed the configured tail envelope; checkpoint first.
    MaintenanceRequired {
        count: u64,
        bytes: u64,
    },
    /// The bounded named-root list is full; no other root is discarded.
    RootCapacityExceeded,
    /// The named root ID was not found (stale release refuses; it cannot
    /// remove a different root).
    UnknownRoot,
    /// A root with this ID already exists; root IDs are never reused.
    DuplicateRoot,
    Authority(crate::history::authority::AuthorityError),
}

impl From<FrameError> for HeadError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<crate::history::authority::AuthorityError> for HeadError {
    fn from(error: crate::history::authority::AuthorityError) -> Self {
        Self::Authority(error)
    }
}

impl HeadRecord {
    /// The genesis head for a new incarnation: genesis recovery root, empty
    /// named roots, Idle GC, the configured initial object epoch.
    ///
    /// # Errors
    /// Refuses a tombstone control.
    pub fn genesis(control: HeadAuthority, object_epoch: u64) -> Result<Self, HeadError> {
        let live = control.live().map_err(|_| HeadError::Deleted)?;
        let recovery = RecoveryRoot::genesis(live.decision, object_epoch);
        Ok(Self {
            control,
            object_epoch,
            recovery: Some(recovery),
            roots: Vec::new(),
            gc: GcPhase::Idle,
        })
    }

    /// A pre-genesis cancellation tombstone head: fences delayed genesis,
    /// carries no recovery root, roots or GC progress.
    #[must_use]
    pub fn cancelled_before_genesis(control: HeadAuthority, object_epoch: u64) -> Self {
        Self {
            control,
            object_epoch,
            recovery: None,
            roots: Vec::new(),
            gc: GcPhase::Idle,
        }
    }

    /// Compose the successor head for one published decision: the new
    /// control (already `decided`), tip advanced, tail grown by the decision
    /// object's length, all retention fields preserved. Refuses when the
    /// grown tail exceeds the envelope — `MaintenanceRequired` backpressure,
    /// including no-op/rejection decisions.
    ///
    /// # Errors
    /// Tombstones, missing recovery and the exceeded envelope refuse.
    pub fn decided(
        &self,
        new_control: HeadAuthority,
        decision_bytes: u64,
        policy: &TailPolicy,
    ) -> Result<Self, HeadError> {
        let live = new_control.live().map_err(|_| HeadError::Deleted)?;
        let recovery = self.recovery.ok_or(HeadError::NoRecovery)?;
        let grown = RecoveryRoot {
            tip: live.decision,
            tail_bytes: recovery.tail_bytes.saturating_add(decision_bytes),
            ..recovery
        };
        if grown.tail_count() > policy.max_count || grown.tail_bytes > policy.max_bytes {
            return Err(HeadError::MaintenanceRequired {
                count: grown.tail_count(),
                bytes: grown.tail_bytes,
            });
        }
        Ok(Self {
            control: new_control,
            recovery: Some(grown),
            roots: self.roots.clone(),
            gc: self.gc.clone(),
            object_epoch: self.object_epoch,
        })
    }

    /// Replace the recovery root (checkpoint publication): built from this
    /// exact head, preserving roots, receipt policy, mode and GC state. Only
    /// the recovery representation changes; control revision advances as
    /// maintenance.
    ///
    /// # Errors
    /// Tombstones refuse; the authority transition can refuse.
    pub fn with_recovery(&self, recovery: RecoveryRoot) -> Result<Self, HeadError> {
        self.control.live().map_err(|_| HeadError::Deleted)?;
        let control = self.control.maintained()?;
        Ok(Self {
            control,
            recovery: Some(recovery),
            roots: self.roots.clone(),
            gc: self.gc.clone(),
            object_epoch: self.object_epoch,
        })
    }

    /// Swap in an already-transitioned control (freeze/thaw/activate/rotate/
    /// retire — the transition itself already advanced the revision),
    /// preserving all retention fields. Tombstoning drops the active
    /// recovery root but preserves roots, epoch and barrier progress.
    #[must_use]
    pub fn with_control(&self, control: HeadAuthority) -> Self {
        let recovery = match control.lifecycle {
            Lifecycle::Live(_) => self.recovery,
            Lifecycle::Deleted { .. } => None,
        };
        Self {
            control,
            recovery,
            roots: self.roots.clone(),
            gc: self.gc.clone(),
            object_epoch: self.object_epoch,
        }
    }

    /// Add a named root captured against this exact head. Full lists refuse
    /// without discarding another root; duplicate IDs refuse (never reused).
    ///
    /// # Errors
    /// Capacity, duplicate ID and authority refusals.
    pub fn add_root(&self, root: NamedRoot, policy: &RootPolicy) -> Result<Self, HeadError> {
        if self.roots.len() >= policy.max_roots {
            return Err(HeadError::RootCapacityExceeded);
        }
        if root.label.len() > policy.max_label_bytes {
            return Err(HeadError::Frame(FrameError::LimitExceeded));
        }
        if self.roots.iter().any(|held| held.id == root.id) {
            return Err(HeadError::DuplicateRoot);
        }
        let control = self.control.maintained()?;
        let mut roots = self.roots.clone();
        roots.push(root);
        Ok(Self {
            control,
            recovery: self.recovery,
            roots,
            gc: self.gc.clone(),
            object_epoch: self.object_epoch,
        })
    }

    /// Release exactly one named root by ID. A stale release with a foreign
    /// ID refuses rather than removing a different root.
    ///
    /// # Errors
    /// Unknown IDs and authority refusals.
    pub fn release_root(&self, id: OperationId) -> Result<Self, HeadError> {
        let Some(index) = self.roots.iter().position(|held| held.id == id) else {
            return Err(HeadError::UnknownRoot);
        };
        let control = self.control.maintained()?;
        let mut roots = self.roots.clone();
        roots.remove(index);
        Ok(Self {
            control,
            recovery: self.recovery,
            roots,
            gc: self.gc.clone(),
            object_epoch: self.object_epoch,
        })
    }

    /// The protected closure a barrier captures from this exact head: the
    /// live recovery root (a tombstone contributes none) plus every named
    /// root's recovery. An empty set is valid.
    #[must_use]
    pub fn protected_closure(&self) -> Vec<RecoveryRoot> {
        let mut protected = Vec::new();
        if let Some(recovery) = self.recovery {
            protected.push(recovery);
        }
        protected.extend(self.roots.iter().map(|root| root.recovery));
        protected
    }
}

// ---------------------------------------------------------------------------
// Wire helpers. The history machine's framing module is private to P04's
// tree; these are this packet's own equivalents over the shared FrameError.
// ---------------------------------------------------------------------------

pub(crate) mod wire {
    use super::FrameError;

    pub(crate) fn check_limit(length: usize, cap: usize) -> Result<(), FrameError> {
        if length > cap {
            Err(FrameError::LimitExceeded)
        } else {
            Ok(())
        }
    }

    pub(crate) fn put_u64(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn put_u32(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn put_span(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), FrameError> {
        put_u64(
            out,
            u64::try_from(bytes.len()).map_err(|_| FrameError::LengthOverflow)?,
        );
        out.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) struct Reader<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Reader<'a> {
        pub(crate) fn begin(
            bytes: &'a [u8],
            family: &[u8],
            layout: u16,
            kind: u8,
            cap: usize,
        ) -> Result<Self, FrameError> {
            check_limit(bytes.len(), cap)?;
            let mut input = Self { bytes, at: 0 };
            if input.take(family.len())? != family {
                return Err(FrameError::Family);
            }
            let version = u16::from_be_bytes(input.array()?);
            if version != layout {
                return Err(FrameError::Layout { got: version });
            }
            let (_, got) = input.tag()?;
            if got != kind {
                return Err(FrameError::Kind { got });
            }
            Ok(input)
        }

        pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], FrameError> {
            let end = self.at.checked_add(len).ok_or(FrameError::LengthOverflow)?;
            let bytes = self
                .bytes
                .get(self.at..end)
                .ok_or(FrameError::Truncated { at: self.at })?;
            self.at = end;
            Ok(bytes)
        }

        pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
            let mut array = [0; N];
            array.copy_from_slice(self.take(N)?);
            Ok(array)
        }

        pub(crate) fn u64(&mut self) -> Result<u64, FrameError> {
            Ok(u64::from_be_bytes(self.array()?))
        }

        pub(crate) fn u32(&mut self) -> Result<u32, FrameError> {
            Ok(u32::from_be_bytes(self.array()?))
        }

        pub(crate) fn tag(&mut self) -> Result<(usize, u8), FrameError> {
            let at = self.at;
            Ok((at, self.array::<1>()?[0]))
        }

        pub(crate) fn span(&mut self, cap: usize) -> Result<&'a [u8], FrameError> {
            let len = usize::try_from(self.u64()?).map_err(|_| FrameError::LengthOverflow)?;
            check_limit(len, cap)?;
            self.take(len)
        }

        pub(crate) fn end(self) -> Result<(), FrameError> {
            if self.at == self.bytes.len() {
                Ok(())
            } else {
                Err(FrameError::TrailingBytes { at: self.at })
            }
        }
    }

    pub(crate) fn frame_header(family: &[u8], layout: u16, kind: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(family.len() + 3);
        out.extend_from_slice(family);
        out.extend_from_slice(&layout.to_be_bytes());
        out.push(kind);
        out
    }
}

use wire::{Reader, put_span, put_u32, put_u64};

fn put_stamp(out: &mut Vec<u8>, stamp: DecisionStamp) {
    put_u64(out, stamp.seq);
    out.extend_from_slice(stamp.hash.as_bytes());
}

fn read_stamp(input: &mut Reader<'_>) -> Result<DecisionStamp, FrameError> {
    Ok(DecisionStamp {
        seq: input.u64()?,
        hash: DecisionDigest::from_bytes(input.array()?),
    })
}

fn put_state(out: &mut Vec<u8>, state: StateStamp) {
    out.extend_from_slice(state.incarnation.as_core().as_bytes());
    put_u64(out, state.data_revision);
}

fn read_state(input: &mut Reader<'_>) -> Result<StateStamp, FrameError> {
    Ok(StateStamp {
        incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
        data_revision: input.u64()?,
    })
}

fn put_operation(out: &mut Vec<u8>, operation: OperationId) {
    out.extend_from_slice(operation.as_core().as_bytes());
}

fn read_operation(input: &mut Reader<'_>) -> Result<OperationId, FrameError> {
    Ok(OperationId::from_core(bumbledb::Id128::from_bytes(
        input.array()?,
    )))
}

pub(crate) fn put_object_ref(out: &mut Vec<u8>, reference: &ObjectRef) {
    put_u64(out, reference.epoch);
    out.push(match reference.kind {
        ObjectKind::Decision => 0,
        ObjectKind::Chunk => 1,
        ObjectKind::Checkpoint => 2,
        ObjectKind::Mark => 3,
    });
    out.extend_from_slice(&reference.digest);
    put_u64(out, reference.length);
}

pub(crate) fn read_object_ref(input: &mut Reader<'_>) -> Result<ObjectRef, FrameError> {
    let epoch = input.u64()?;
    let kind = match input.tag()? {
        (_, 0) => ObjectKind::Decision,
        (_, 1) => ObjectKind::Chunk,
        (_, 2) => ObjectKind::Checkpoint,
        (_, 3) => ObjectKind::Mark,
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    Ok(ObjectRef {
        epoch,
        kind,
        digest: input.array()?,
        length: input.u64()?,
    })
}

fn put_recovery(out: &mut Vec<u8>, recovery: &RecoveryRoot) {
    match &recovery.checkpoint {
        None => out.push(0),
        Some(reference) => {
            out.push(1);
            put_object_ref(out, reference);
        }
    }
    put_stamp(out, recovery.base);
    put_stamp(out, recovery.tip);
    put_u64(out, recovery.tail_bytes);
    put_u64(out, recovery.epoch_floor);
}

fn read_recovery(input: &mut Reader<'_>) -> Result<RecoveryRoot, FrameError> {
    let checkpoint = match input.tag()? {
        (_, 0) => None,
        (_, 1) => Some(read_object_ref(input)?),
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    let base = read_stamp(input)?;
    let tip = read_stamp(input)?;
    if tip.seq < base.seq {
        return Err(FrameError::InvalidSequence);
    }
    Ok(RecoveryRoot {
        checkpoint,
        base,
        tip,
        tail_bytes: input.u64()?,
        epoch_floor: input.u64()?,
    })
}

fn put_barrier(out: &mut Vec<u8>, barrier: &Barrier) -> Result<(), FrameError> {
    put_operation(out, barrier.id);
    put_u64(out, barrier.cutoff_epoch);
    put_u32(
        out,
        u32::try_from(barrier.protected.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    for root in &barrier.protected {
        put_recovery(out, root);
    }
    Ok(())
}

fn read_barrier(input: &mut Reader<'_>) -> Result<Barrier, FrameError> {
    let id = read_operation(input)?;
    let cutoff_epoch = input.u64()?;
    let count = input.u32()? as usize;
    if count > 4 * RootPolicy::DEFAULT.max_roots {
        return Err(FrameError::InvalidCount);
    }
    let mut protected = Vec::with_capacity(count);
    for _ in 0..count {
        protected.push(read_recovery(input)?);
    }
    Ok(Barrier {
        id,
        cutoff_epoch,
        protected: protected.into_boxed_slice(),
    })
}

/// Encode the whole head body. `cap` bounds the encoded bytes.
///
/// # Errors
/// Oversized frames refuse; nothing partial is returned.
pub fn encode_head(record: &HeadRecord, cap: usize) -> Result<Vec<u8>, FrameError> {
    let control = encode_control(&record.control, cap)?;
    let mut out = wire::frame_header(FAMILY, LAYOUT, HEAD);
    put_span(&mut out, &control)?;
    put_u64(&mut out, record.object_epoch);
    match &record.recovery {
        None => out.push(0),
        Some(recovery) => {
            out.push(1);
            put_recovery(&mut out, recovery);
        }
    }
    put_u32(
        &mut out,
        u32::try_from(record.roots.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    for root in &record.roots {
        put_operation(&mut out, root.id);
        out.push(match root.kind {
            RootKind::RestorePoint => 0,
            RootKind::HydrationHold => 1,
        });
        put_recovery(&mut out, &root.recovery);
        put_state(&mut out, root.state);
        put_span(&mut out, root.label.as_bytes())?;
        put_operation(&mut out, root.operation);
    }
    match &record.gc {
        GcPhase::Idle => out.push(0),
        GcPhase::Marking { barrier } => {
            out.push(1);
            put_barrier(&mut out, barrier)?;
        }
        GcPhase::Sweeping {
            barrier,
            marks,
            cursor,
        } => {
            out.push(2);
            put_barrier(&mut out, barrier)?;
            put_object_ref(&mut out, marks);
            match cursor {
                None => out.push(0),
                Some(cursor) => {
                    out.push(1);
                    put_span(&mut out, cursor)?;
                }
            }
        }
    }
    wire::check_limit(out.len(), cap)?;
    Ok(out)
}

/// Decode and validate a head body. Grammar and internal invariants only:
/// bytes from storage still need their outer conditional-read/authority
/// verification before they are trusted.
///
/// # Errors
/// Malformed frames and violated invariants refuse.
pub fn decode_head(bytes: &[u8], cap: usize) -> Result<HeadRecord, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, HEAD, cap)?;
    let control_bytes = input.span(cap)?;
    let control = decode_control(control_bytes, cap)?;
    let object_epoch = input.u64()?;
    let recovery = match input.tag()? {
        (_, 0) => None,
        (_, 1) => Some(read_recovery(&mut input)?),
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    // A live head names its recovery root; a tombstone has none.
    match (&control.lifecycle, recovery.is_some()) {
        (Lifecycle::Live(_), false) | (Lifecycle::Deleted { .. }, true) => {
            return Err(FrameError::InvalidSequence);
        }
        _ => {}
    }
    if let (Lifecycle::Live(live), Some(root)) = (&control.lifecycle, &recovery)
        && root.tip != live.decision
    {
        return Err(FrameError::InvalidTerminalStamp);
    }
    let count = input.u32()? as usize;
    if count > 4 * RootPolicy::DEFAULT.max_roots {
        return Err(FrameError::InvalidCount);
    }
    let mut roots = Vec::with_capacity(count);
    for _ in 0..count {
        let id = read_operation(&mut input)?;
        let kind = match input.tag()? {
            (_, 0) => RootKind::RestorePoint,
            (_, 1) => RootKind::HydrationHold,
            (at, got) => return Err(FrameError::Tag { at, got }),
        };
        let root_recovery = read_recovery(&mut input)?;
        let state = read_state(&mut input)?;
        let label_bytes = input.span(1_024)?;
        let label = std::str::from_utf8(label_bytes)
            .map_err(|_| FrameError::Truncated { at: 0 })?
            .into();
        let operation = read_operation(&mut input)?;
        if roots.iter().any(|held: &NamedRoot| held.id == id) {
            return Err(FrameError::InvalidCount);
        }
        roots.push(NamedRoot {
            id,
            kind,
            recovery: root_recovery,
            state,
            label,
            operation,
        });
    }
    let gc = match input.tag()? {
        (_, 0) => GcPhase::Idle,
        (_, 1) => GcPhase::Marking {
            barrier: read_barrier(&mut input)?,
        },
        (_, 2) => {
            let barrier = read_barrier(&mut input)?;
            let marks = read_object_ref(&mut input)?;
            let cursor = match input.tag()? {
                (_, 0) => None,
                (_, 1) => Some(Box::from(input.span(4_096)?)),
                (at, got) => return Err(FrameError::Tag { at, got }),
            };
            GcPhase::Sweeping {
                barrier,
                marks,
                cursor,
            }
        }
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    input.end()?;
    // GC epoch invariants: a barrier's cutoff is strictly below the open epoch.
    match &gc {
        GcPhase::Idle => {}
        GcPhase::Marking { barrier } | GcPhase::Sweeping { barrier, .. } => {
            if barrier.cutoff_epoch >= object_epoch {
                return Err(FrameError::InvalidEpoch);
            }
        }
    }
    Ok(HeadRecord {
        control,
        object_epoch,
        recovery,
        roots,
        gc,
    })
}

/// Decode only the authority projection out of a composed head body — the
/// accessor the publication machine uses (recorded P04 patch request:
/// `writer::hosted` reads/writes composed head bodies through this module).
///
/// # Errors
/// Malformed frames refuse.
pub fn head_authority(bytes: &[u8], cap: usize) -> Result<HeadAuthority, FrameError> {
    Ok(decode_head(bytes, cap)?.control)
}

/// Compose the successor head body for one published decision from the exact
/// parent head bytes: embed the already-transitioned control, advance the
/// tail accounting, preserve every retention field, enforce the envelope.
///
/// # Errors
/// Frame grammar, tombstones, missing recovery and `MaintenanceRequired`.
pub fn decided_head_body(
    parent_body: &[u8],
    new_control: &HeadAuthority,
    decision_bytes: u64,
    policy: &TailPolicy,
    cap: usize,
) -> Result<Vec<u8>, HeadError> {
    let parent = decode_head(parent_body, cap)?;
    let next = parent.decided(*new_control, decision_bytes, policy)?;
    Ok(encode_head(&next, cap)?)
}

/// The genesis head body for a new incarnation.
///
/// # Errors
/// Frame grammar and tombstone controls refuse.
pub fn genesis_head_body(
    control: &HeadAuthority,
    object_epoch: u64,
    cap: usize,
) -> Result<Vec<u8>, HeadError> {
    let record = HeadRecord::genesis(*control, object_epoch)?;
    Ok(encode_head(&record, cap)?)
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::*;
    use crate::history::authority::{Activation, ActivationCause};
    use crate::history::{DatabaseId, DatabaseIdentity, SchemaId};

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    fn op(byte: u8) -> OperationId {
        OperationId::from_core(Id128::from_bytes([byte; 16]))
    }

    fn genesis_control() -> HeadAuthority {
        HeadAuthority::genesis(
            identity(),
            DecisionStamp {
                seq: 0,
                hash: DecisionDigest::from_bytes([9; 32]),
            },
            Activation::Activated {
                operation: op(4),
                target_genesis: DecisionDigest::from_bytes([9; 32]),
                cause: ActivationCause::Create,
            },
        )
        .unwrap()
    }

    fn sample_root(id: u8) -> NamedRoot {
        NamedRoot {
            id: op(id),
            kind: RootKind::RestorePoint,
            recovery: RecoveryRoot {
                checkpoint: Some(ObjectRef {
                    epoch: 1,
                    kind: ObjectKind::Checkpoint,
                    digest: [id; 32],
                    length: 42,
                }),
                base: DecisionStamp {
                    seq: 1,
                    hash: DecisionDigest::from_bytes([7; 32]),
                },
                tip: DecisionStamp {
                    seq: 3,
                    hash: DecisionDigest::from_bytes([8; 32]),
                },
                tail_bytes: 100,
                epoch_floor: 1,
            },
            state: StateStamp {
                incarnation: identity().incarnation_id,
                data_revision: 2,
            },
            label: "before-migration".into(),
            operation: op(id),
        }
    }

    #[test]
    fn head_bodies_roundtrip_across_every_gc_phase_and_refuse_prefixes() {
        let genesis = HeadRecord::genesis(genesis_control(), 1).unwrap();
        let with_root = genesis
            .add_root(sample_root(10), &RootPolicy::DEFAULT)
            .unwrap();
        let barrier = Barrier {
            id: op(20),
            cutoff_epoch: 1,
            protected: with_root.protected_closure().into_boxed_slice(),
        };
        let marking = HeadRecord {
            object_epoch: 2,
            gc: GcPhase::Marking {
                barrier: barrier.clone(),
            },
            ..with_root.clone()
        };
        let sweeping = HeadRecord {
            object_epoch: 2,
            gc: GcPhase::Sweeping {
                barrier,
                marks: ObjectRef {
                    epoch: 2,
                    kind: ObjectKind::Mark,
                    digest: [5; 32],
                    length: 77,
                },
                cursor: Some(Box::from(&b"p/objects/1/chunk/aa"[..])),
            },
            ..with_root.clone()
        };
        for record in [genesis, with_root, marking, sweeping] {
            let bytes = encode_head(&record, 65_536).unwrap();
            assert_eq!(decode_head(&bytes, 65_536).unwrap(), record);
            assert_eq!(head_authority(&bytes, 65_536).unwrap(), record.control);
            for end in 0..bytes.len() {
                assert!(decode_head(&bytes[..end], 65_536).is_err(), "prefix {end}");
            }
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(decode_head(&trailing, 65_536).is_err());
        }
    }

    #[test]
    fn decided_composition_preserves_retention_and_enforces_the_envelope() {
        let head = HeadRecord::genesis(genesis_control(), 1)
            .unwrap()
            .add_root(sample_root(10), &RootPolicy::DEFAULT)
            .unwrap();
        let control = head
            .control
            .decided(DecisionDigest::from_bytes([7; 32]), true)
            .unwrap();
        let next = head
            .decided(
                control,
                100,
                &TailPolicy {
                    max_count: 10,
                    max_bytes: 10_000,
                },
            )
            .unwrap();
        assert_eq!(next.roots, head.roots, "unrelated roots are retained");
        assert_eq!(next.object_epoch, head.object_epoch);
        let recovery = next.recovery.unwrap();
        assert_eq!(recovery.tail_count(), 1);
        assert_eq!(recovery.tail_bytes, 100);
        // The envelope refuses further growth, including no-op decisions.
        let tight = TailPolicy {
            max_count: 1,
            max_bytes: 10_000,
        };
        let control2 = control
            .decided(DecisionDigest::from_bytes([8; 32]), false)
            .unwrap();
        assert!(matches!(
            next.decided(control2, 10, &tight),
            Err(HeadError::MaintenanceRequired { .. })
        ));
    }

    #[test]
    fn root_capacity_duplicates_and_stale_release_refuse_exactly() {
        let policy = RootPolicy {
            max_roots: 2,
            max_label_bytes: 64,
        };
        let head = HeadRecord::genesis(genesis_control(), 1).unwrap();
        let one = head.add_root(sample_root(1), &policy).unwrap();
        assert!(matches!(
            one.add_root(sample_root(1), &policy),
            Err(HeadError::DuplicateRoot)
        ));
        let two = one.add_root(sample_root(2), &policy).unwrap();
        assert!(matches!(
            two.add_root(sample_root(3), &policy),
            Err(HeadError::RootCapacityExceeded)
        ));
        assert_eq!(two.roots.len(), 2, "capacity refusal discarded nothing");
        assert!(matches!(
            two.release_root(op(9)),
            Err(HeadError::UnknownRoot)
        ));
        let released = two.release_root(op(1)).unwrap();
        assert_eq!(released.roots.len(), 1);
        assert_eq!(released.roots[0].id, op(2));
    }

    #[test]
    fn tombstoned_heads_drop_the_recovery_root_but_keep_roots_and_barrier() {
        let head = HeadRecord::genesis(genesis_control(), 1)
            .unwrap()
            .add_root(sample_root(1), &RootPolicy::DEFAULT)
            .unwrap();
        let deleted = match head
            .control
            .delete(op(40), crate::history::authority::DeletedReason::Erasure)
            .unwrap()
        {
            crate::history::authority::DeleteOutcome::Deleted(control) => control,
            crate::history::authority::DeleteOutcome::AlreadyDeleted { .. } => unreachable!(),
        };
        let tombstone = head.with_control(deleted);
        assert!(tombstone.recovery.is_none(), "no active recovery root");
        assert_eq!(tombstone.roots.len(), 1, "explicit roots survive");
        let bytes = encode_head(&tombstone, 65_536).unwrap();
        assert_eq!(decode_head(&bytes, 65_536).unwrap(), tombstone);
        assert_eq!(
            tombstone.protected_closure().len(),
            1,
            "a deleted head contributes no live root"
        );
    }
}
