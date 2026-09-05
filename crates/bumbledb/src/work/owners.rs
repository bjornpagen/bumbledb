//! Charged owners: allocation capacity and byte charge move together.
//!
//! Values are read through borrows; retaining decoded bytes or buffers past
//! a function return requires moving the whole owner. There is no payload
//! extraction that sheds the reservation while bytes remain live.

use super::{ByteKind, ByteReservation, Resource, WorkContext, WorkError};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts backing-store growth attempts. D01 refuses before this increments.
#[cfg(test)]
pub(crate) static GROWTH_ALLOCS: AtomicU64 = AtomicU64::new(0);

fn exhausted(work: &WorkContext, kind: ByteKind, used: u64, requested: u64) -> WorkError {
    WorkError::Exhausted {
        resource: kind.resource(),
        used,
        requested,
        limit: work.limit(kind.resource()),
    }
}

fn conservative_capacity(needed: usize) -> usize {
    needed
        .max(8)
        .checked_next_power_of_two()
        .unwrap_or(needed)
}

fn try_reserve_vec(buf: &mut Vec<u8>, additional: usize) -> Result<(), ()> {
    #[cfg(test)]
    GROWTH_ALLOCS.fetch_add(1, Ordering::Relaxed);
    buf.try_reserve(additional).map_err(|_| ())
}

fn try_reserve_exact_vec(buf: &mut Vec<u8>, additional: usize) -> Result<(), ()> {
    #[cfg(test)]
    GROWTH_ALLOCS.fetch_add(1, Ordering::Relaxed);
    buf.try_reserve_exact(additional).map_err(|_| ())
}

/// An owned byte buffer whose reservation travels with it.
#[derive(Debug)]
pub struct ChargedBytes {
    bytes: Box<[u8]>,
    charge: ByteReservation,
}

impl ChargedBytes {
    /// Reserve for `bytes.len()`, then take ownership.
    /// # Errors
    /// Refuses growth beyond the operation allowance.
    pub fn adopt(
        work: &WorkContext,
        kind: ByteKind,
        bytes: Box<[u8]>,
    ) -> Result<Self, WorkError> {
        let charge = work.reserve(kind, bytes.len() as u64)?;
        Ok(Self { bytes, charge })
    }

    /// Reserve `capacity` bytes, then allocate an empty buffer of that size.
    /// Reservation is rolled back if the allocation fails.
    /// # Errors
    /// Refuses growth beyond the operation allowance, or allocator refusal.
    pub fn with_capacity(
        work: &WorkContext,
        kind: ByteKind,
        capacity: usize,
    ) -> Result<Self, WorkError> {
        let charge = work.reserve(kind, capacity as u64)?;
        let mut buf = Vec::new();
        if try_reserve_exact_vec(&mut buf, capacity).is_err() {
            drop(charge);
            return Err(exhausted(
                work,
                kind,
                work.used(kind.resource()),
                capacity as u64,
            ));
        }
        buf.resize(capacity, 0);
        Ok(Self {
            bytes: buf.into_boxed_slice(),
            charge,
        })
    }

    /// Reserve destination capacity, then copy. The source owner stays charged
    /// for the duration of the copy (C2 overlap).
    /// # Errors
    /// Refuses growth beyond the destination ledger, or allocator refusal.
    pub fn admit_copy(
        &self,
        work: &WorkContext,
        kind: ByteKind,
    ) -> Result<Self, WorkError> {
        let charge = work.reserve(kind, self.bytes.len() as u64)?;
        let mut buf = Vec::new();
        if try_reserve_exact_vec(&mut buf, self.bytes.len()).is_err() {
            drop(charge);
            return Err(exhausted(
                work,
                kind,
                work.used(kind.resource()),
                self.bytes.len() as u64,
            ));
        }
        buf.extend_from_slice(&self.bytes);
        Ok(Self {
            bytes: buf.into_boxed_slice(),
            charge,
        })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[must_use]
    pub fn charged_bytes(&self) -> u64 {
        self.charge.bytes()
    }

    /// Transfer the whole owner; charge and payload stay together (C2).
    #[must_use]
    pub fn into_owner(self) -> Self {
        self
    }
}

impl AsRef<[u8]> for ChargedBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// An owned growable buffer: the reservation covers the vector's capacity
/// envelope and grows only through fallible reservation before reallocation.
#[derive(Debug)]
pub struct ChargedBuffer {
    inner: Vec<u8>,
    charge: ByteReservation,
    charged_capacity: usize,
    work: WorkContext,
    kind: ByteKind,
}

impl ChargedBuffer {
    /// Reserve the conservative `capacity` envelope, then allocate it.
    /// # Errors
    /// Refuses growth beyond the operation allowance, or allocator refusal.
    pub fn with_capacity(
        work: &WorkContext,
        kind: ByteKind,
        capacity: usize,
    ) -> Result<Self, WorkError> {
        let reserved = conservative_capacity(capacity);
        let charge = work.reserve(kind, reserved as u64)?;
        let mut inner = Vec::new();
        if try_reserve_vec(&mut inner, reserved).is_err() {
            drop(charge);
            return Err(exhausted(
                work,
                kind,
                work.used(kind.resource()),
                reserved as u64,
            ));
        }
        let mut owner = Self {
            inner,
            charge,
            charged_capacity: reserved,
            work: work.clone(),
            kind,
        };
        owner.charge_actual_capacity()?;
        Ok(owner)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Reserve any additional capacity envelope, then append.
    /// # Errors
    /// Refuses growth beyond the operation allowance, or allocator refusal.
    pub fn try_extend_from_slice(&mut self, data: &[u8]) -> Result<(), WorkError> {
        let needed = self.inner.len().checked_add(data.len()).ok_or_else(|| {
            exhausted(
                &self.work,
                self.kind,
                self.charged_capacity as u64,
                u64::MAX,
            )
        })?;
        if needed > self.inner.capacity() {
            let next = conservative_capacity(
                needed.max(self.inner.capacity().saturating_mul(2).max(8)),
            );
            self.reserve_capacity(next)?;
        }
        self.inner.extend_from_slice(data);
        Ok(())
    }

    /// Clear length without releasing the charged capacity envelope.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[must_use]
    pub fn charged_capacity(&self) -> usize {
        self.charged_capacity
    }

    #[must_use]
    pub fn charged_bytes(&self) -> u64 {
        self.charge.bytes()
    }

    /// Reserve destination capacity, then copy. The source stays charged.
    /// # Errors
    /// Refuses growth beyond the destination ledger, or allocator refusal.
    pub fn admit_copy(&self, work: &WorkContext, kind: ByteKind) -> Result<Self, WorkError> {
        let reserved = self.charged_capacity.max(self.inner.capacity());
        let charge = work.reserve(kind, reserved as u64)?;
        let mut inner = Vec::new();
        if try_reserve_vec(&mut inner, reserved).is_err() {
            drop(charge);
            return Err(exhausted(
                work,
                kind,
                work.used(kind.resource()),
                reserved as u64,
            ));
        }
        inner.extend_from_slice(&self.inner);
        let mut copy = Self {
            inner,
            charge,
            charged_capacity: reserved,
            work: work.clone(),
            kind,
        };
        copy.charge_actual_capacity()?;
        Ok(copy)
    }

    /// Transfer the whole owner; charge and payload stay together (C2).
    #[must_use]
    pub fn into_owner(self) -> Self {
        self
    }

    /// Freeze length into a byte owner. Retained capacity charge travels.
    #[must_use]
    pub fn into_bytes(self) -> ChargedBytes {
        ChargedBytes {
            bytes: self.inner.into_boxed_slice(),
            charge: self.charge,
        }
    }

    fn reserve_capacity(&mut self, capacity: usize) -> Result<(), WorkError> {
        if capacity <= self.charged_capacity {
            let additional = capacity.saturating_sub(self.inner.len());
            if additional > 0 && try_reserve_vec(&mut self.inner, additional).is_err() {
                return Err(exhausted(
                    &self.work,
                    self.kind,
                    self.charged_capacity as u64,
                    additional as u64,
                ));
            }
            return self.charge_actual_capacity();
        }
        let delta = capacity - self.charged_capacity;
        let extra = self.work.reserve(self.kind, delta as u64)?;
        let additional = capacity.saturating_sub(self.inner.len());
        if try_reserve_vec(&mut self.inner, additional).is_err() {
            drop(extra);
            return Err(exhausted(
                &self.work,
                self.kind,
                self.charged_capacity as u64,
                delta as u64,
            ));
        }
        self.charge.join(extra);
        self.charged_capacity = capacity;
        self.charge_actual_capacity()
    }

    fn charge_actual_capacity(&mut self) -> Result<(), WorkError> {
        let actual = self.inner.capacity();
        if actual <= self.charged_capacity {
            return Ok(());
        }
        let delta = actual - self.charged_capacity;
        match self.work.reserve(self.kind, delta as u64) {
            Ok(extra) => {
                self.charge.join(extra);
                self.charged_capacity = actual;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

/// A charged cache-owned relation image slab: bytes and cache reservation
/// are inseparable until the whole owner is dropped or transferred.
#[derive(Debug)]
pub struct ChargedImage {
    bytes: usize,
    charge: super::CacheReservation,
}

impl ChargedImage {
    /// Reserve retained bytes against the database cache ledger.
    /// # Errors
    /// Refuses bytes beyond the cache allowance.
    pub fn admit(
        cache: &super::CacheLedger,
        bytes: usize,
    ) -> Result<Self, super::cache::CacheError> {
        let charge = cache.reserve(bytes as u64)?;
        Ok(Self { bytes, charge })
    }

    /// Reserve a second cache charge before a consumer copies the slab.
    /// The source owner stays charged (C2 overlap).
    /// # Errors
    /// Refuses bytes beyond the cache allowance.
    pub fn admit_copy(
        &self,
        cache: &super::CacheLedger,
    ) -> Result<Self, super::cache::CacheError> {
        let charge = cache.reserve(self.bytes as u64)?;
        Ok(Self {
            bytes: self.bytes,
            charge,
        })
    }

    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    #[must_use]
    pub fn charged_bytes(&self) -> u64 {
        self.charge.bytes()
    }

    /// Transfer the whole owner; charge and payload stay together (C2).
    #[must_use]
    pub fn into_owner(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::ExecutionPolicy;
    use std::time::Duration;

    fn work(working: u64) -> WorkContext {
        ExecutionPolicy {
            input_bytes: 0,
            working_bytes: working,
            scratch_bytes: 1024,
            result_bytes: 1024,
            rows: 0,
            work_units: 1024,
            timeout: Duration::from_secs(60),
        }
        .start()
        .expect("start")
    }

    #[test]
    fn charged_bytes_drop_refunds_exactly_once() {
        let ctx = work(1000);
        let baseline = ctx.used(Resource::WorkingBytes);
        let owner =
            ChargedBytes::adopt(&ctx, ByteKind::Working, Box::from(*b"payload")).expect("adopt");
        assert_eq!(
            ctx.used(Resource::WorkingBytes),
            baseline + owner.charged_bytes()
        );
        drop(owner);
        assert_eq!(ctx.used(Resource::WorkingBytes), baseline);
    }

    #[test]
    fn d01_zero_capacity_growth_refuses_before_allocation() {
        let before = GROWTH_ALLOCS.load(Ordering::Relaxed);
        let ctx = work(0);
        assert!(ChargedBytes::with_capacity(&ctx, ByteKind::Working, 1).is_err());
        assert!(ChargedBuffer::with_capacity(&ctx, ByteKind::Working, 64).is_err());
        assert_eq!(ctx.used(Resource::WorkingBytes), 0);
        assert_eq!(
            GROWTH_ALLOCS.load(Ordering::Relaxed),
            before,
            "D01: refusal must precede the instrumented allocation"
        );
    }

    #[test]
    fn charged_buffer_reserves_before_first_push() {
        let ctx = work(4096);
        let mut buffer = ChargedBuffer::with_capacity(&ctx, ByteKind::Working, 256).expect("create");
        assert!(buffer.capacity() >= 256);
        buffer.try_extend_from_slice(&[0u8; 128]).expect("extend");
        assert_eq!(buffer.len(), 128);
        let used = ctx.used(Resource::WorkingBytes);
        let before = GROWTH_ALLOCS.load(Ordering::Relaxed);
        buffer
            .try_extend_from_slice(&[0u8; 8000])
            .expect_err("refused");
        assert_eq!(ctx.used(Resource::WorkingBytes), used);
        assert_eq!(
            GROWTH_ALLOCS.load(Ordering::Relaxed),
            before,
            "D01: refused growth does not allocate"
        );
    }

    #[test]
    fn d01_charged_buffer_clear_keeps_capacity_charge() {
        let ctx = work(4096);
        let mut buffer = ChargedBuffer::with_capacity(&ctx, ByteKind::Working, 512).expect("create");
        buffer.try_extend_from_slice(&[1u8; 200]).expect("extend");
        let charged = ctx.used(Resource::WorkingBytes);
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(ctx.used(Resource::WorkingBytes), charged);
        assert!(buffer.capacity() >= 512);
    }

    #[test]
    fn d01_admit_copy_charges_overlap_until_source_drops() {
        let ctx = work(4096);
        let source =
            ChargedBytes::adopt(&ctx, ByteKind::Working, Box::from(*b"overlap-payload")).expect("src");
        let once = ctx.used(Resource::WorkingBytes);
        let dest = source.admit_copy(&ctx, ByteKind::Result).expect("copy");
        assert_eq!(
            ctx.used(Resource::WorkingBytes),
            once,
            "working source stays charged"
        );
        assert_eq!(ctx.used(Resource::ResultBytes), dest.charged_bytes());
        assert_eq!(dest.as_bytes(), source.as_bytes());
        let dest = dest.into_owner();
        drop(source);
        assert_eq!(ctx.used(Resource::WorkingBytes), 0);
        assert_eq!(ctx.used(Resource::ResultBytes), dest.charged_bytes());
        drop(dest);
        assert_eq!(ctx.used(Resource::ResultBytes), 0);
    }

    #[test]
    fn d01_failed_reservation_leaves_prior_owner() {
        let ctx = work(256);
        let mut buffer = ChargedBuffer::with_capacity(&ctx, ByteKind::Working, 32).expect("create");
        buffer.try_extend_from_slice(&[7u8; 8]).expect("seed");
        let used = ctx.used(Resource::WorkingBytes);
        let len = buffer.len();
        buffer
            .try_extend_from_slice(&[0u8; 4096])
            .expect_err("beyond allowance");
        assert_eq!(buffer.len(), len);
        assert_eq!(ctx.used(Resource::WorkingBytes), used);
    }

    #[test]
    fn d01_perturbation_charges_once_not_twice() {
        let ctx = work(4096);
        let owner =
            ChargedBytes::adopt(&ctx, ByteKind::Working, Box::from([0u8; 64])).expect("adopt");
        let once = owner.charged_bytes();
        assert_eq!(ctx.used(Resource::WorkingBytes), once);
        let moved = owner.into_owner();
        assert_eq!(
            ctx.used(Resource::WorkingBytes),
            once,
            "sensitivity: transferring the owner must not charge a second time"
        );
        drop(moved);
        assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    }

    #[test]
    fn d01_charged_bytes_as_ref_matches_as_bytes() {
        let ctx = work(4096);
        let owner =
            ChargedBytes::adopt(&ctx, ByteKind::Working, Box::from(*b"charged-as-ref")).expect("adopt");
        assert_eq!(owner.as_ref(), owner.as_bytes());
        assert_eq!(owner.as_ref(), b"charged-as-ref");
        let kept = owner.into_owner();
        assert_eq!(<&[u8]>::from(kept.as_ref()), kept.as_bytes());
        drop(kept);
        assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    }
}
