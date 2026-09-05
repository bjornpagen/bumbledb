//! One operation's finite work allowance and live owned-byte reservations.
//!
//! Clones share counters, deadline, and cancellation. They do not start a new
//! allowance. Byte reservations travel with the allocation's owner, including
//! across a completed operation; dropping a context does not refund live bytes.
//! These are logical native charges, not an RSS or LMDB page-cache limit.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Explicit finite allowances. Zero means no allowance, never unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionPolicy {
    pub input_bytes: u64,
    pub working_bytes: u64,
    pub scratch_bytes: u64,
    pub result_bytes: u64,
    pub rows: u64,
    pub work_units: u64,
    pub timeout: Duration,
}

/// The dimension which refused growth. Counters never saturate into success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    InputBytes,
    WorkingBytes,
    ScratchBytes,
    ResultBytes,
    Rows,
    WorkUnits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkError {
    Cancelled,
    DeadlineExceeded,
    InvalidTimeout,
    Exhausted {
        resource: Resource,
        used: u64,
        requested: u64,
        limit: u64,
    },
}

impl std::fmt::Display for WorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::DeadlineExceeded => f.write_str("operation deadline exceeded"),
            Self::InvalidTimeout => {
                f.write_str("operation timeout exceeds the monotonic clock range")
            }
            Self::Exhausted {
                resource,
                used,
                requested,
                limit,
            } => write!(
                f,
                "{resource:?} exhausted: {used} used + {requested} requested, limit {limit}"
            ),
        }
    }
}

impl std::error::Error for WorkError {}

#[derive(Debug)]
struct Ledger {
    limits: [u64; 6],
    used: [AtomicU64; 6],
    deadline: Instant,
    cancelled: AtomicBool,
}

/// Sendable operation context shared by queue admission and actual execution.
/// Cancellation requests cooperative stopping, not rollback of a published log
/// decision. Cleanup uses its separately reserved runtime allowance.
#[derive(Debug, Clone)]
pub struct WorkContext(Arc<Ledger>);

impl ExecutionPolicy {
    /// The deadline begins at operation admission, before queueing.
    /// # Errors
    /// Returns [`WorkError::InvalidTimeout`] for an unrepresentable deadline.
    pub fn start(self) -> Result<WorkContext, WorkError> {
        let deadline = Instant::now()
            .checked_add(self.timeout)
            .ok_or(WorkError::InvalidTimeout)?;
        Ok(WorkContext(Arc::new(Ledger {
            limits: [
                self.input_bytes,
                self.working_bytes,
                self.scratch_bytes,
                self.result_bytes,
                self.rows,
                self.work_units,
            ],
            used: std::array::from_fn(|_| AtomicU64::new(0)),
            deadline,
            cancelled: AtomicBool::new(false),
        })))
    }
}

impl Resource {
    const fn index(self) -> usize {
        match self {
            Self::InputBytes => 0,
            Self::WorkingBytes => 1,
            Self::ScratchBytes => 2,
            Self::ResultBytes => 3,
            Self::Rows => 4,
            Self::WorkUnits => 5,
        }
    }
}

impl WorkContext {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }

    /// # Errors
    /// Refuses a cancelled or expired operation, even if it has spare bytes.
    pub fn checkpoint(&self) -> Result<(), WorkError> {
        if self.0.cancelled.load(Ordering::Acquire) {
            return Err(WorkError::Cancelled);
        }
        if Instant::now() >= self.0.deadline {
            return Err(WorkError::DeadlineExceeded);
        }
        Ok(())
    }

    #[must_use]
    pub fn used(&self, resource: Resource) -> u64 {
        self.0.used[resource.index()].load(Ordering::Acquire)
    }

    #[must_use]
    pub fn limit(&self, resource: Resource) -> u64 {
        self.0.limits[resource.index()]
    }

    /// Input is charged on every ingestion, including duplicate facts/reruns.
    /// # Errors
    /// Refuses input beyond the operation's byte allowance or stopped work.
    pub fn input(&self, bytes: u64) -> Result<(), WorkError> {
        self.charge(Resource::InputBytes, bytes)
    }

    /// # Errors
    /// Refuses input beyond the operation's row allowance or stopped work.
    pub fn rows(&self, rows: u64) -> Result<(), WorkError> {
        self.charge(Resource::Rows, rows)
    }

    /// # Errors
    /// Refuses work beyond the operation's cumulative allowance or stopped work.
    pub fn step(&self, units: u64) -> Result<(), WorkError> {
        self.charge(Resource::WorkUnits, units)
    }

    fn charge(&self, resource: Resource, amount: u64) -> Result<(), WorkError> {
        self.checkpoint()?;
        let limit = self.limit(resource);
        self.0.used[resource.index()]
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(amount).filter(|next| *next <= limit)
            })
            .map(|_| ())
            .map_err(|used| WorkError::Exhausted {
                resource,
                used,
                requested: amount,
                limit,
            })
    }

    /// Reserve before allocation/growth; retain the returned owner until the
    /// allocation is actually gone. This does not allocate the backing buffer.
    /// # Errors
    /// Refuses bytes beyond the operation allowance or stopped work.
    pub fn reserve(&self, kind: ByteKind, bytes: u64) -> Result<ByteReservation, WorkError> {
        let resource = kind.resource();
        self.charge(resource, bytes)?;
        Ok(ByteReservation {
            ledger: Arc::clone(&self.0),
            resource,
            bytes,
        })
    }
}

/// Releasable live-byte dimensions, distinct from cumulative work/input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteKind {
    Working,
    Scratch,
    Result,
}

impl ByteKind {
    const fn resource(self) -> Resource {
        match self {
            Self::Working => Resource::WorkingBytes,
            Self::Scratch => Resource::ScratchBytes,
            Self::Result => Resource::ResultBytes,
        }
    }
}

/// Linear reservation: not clonable; Drop refunds exactly its owned charge.
#[derive(Debug)]
pub struct ByteReservation {
    ledger: Arc<Ledger>,
    resource: Resource,
    bytes: u64,
}

impl ByteReservation {
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for ByteReservation {
    fn drop(&mut self) {
        self.ledger.used[self.resource.index()].fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests;
