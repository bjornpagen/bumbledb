//! The database-owned retained-cache ledger: cross-operation capacity
//! distinct from any single operation's [`super::ExecutionPolicy`].
//!
//! A [`GenerationHandle`] is the synchronized owner of one resolver and
//! generation identity. Cache map entries are eviction references, not
//! charge owners: image-slab charges live inside the shared image and
//! refund only when the last strong owner is dropped.

use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};

use crate::image::intern::TextInterner;
use crate::image::CacheGeneration;

/// Cross-operation resident-cache allowance. Zero means no retention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachePolicy {
    pub cache_bytes: u64,
}

impl CachePolicy {
    /// A host-policy default large enough for ordinary single-tenant use;
    /// explicit product configuration replaces this at open time.
    #[must_use]
    pub const fn platform_default() -> Self {
        Self {
            cache_bytes: 512 << 20,
        }
    }

    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            cache_bytes: u64::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    Exhausted {
        used: u64,
        requested: u64,
        limit: u64,
    },
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exhausted {
                used,
                requested,
                limit,
            } => write!(
                f,
                "cache bytes exhausted: {used} used + {requested} requested, limit {limit}"
            ),
        }
    }
}

impl std::error::Error for CacheError {}

#[derive(Debug)]
struct CacheLedgerInner {
    limit: u64,
    used: AtomicU64,
}

/// Shared retained-cache accounting for one database instance.
#[derive(Debug, Clone)]
pub struct CacheLedger(Arc<CacheLedgerInner>);

impl CacheLedger {
    #[must_use]
    pub fn new(policy: CachePolicy) -> Self {
        Self(Arc::new(CacheLedgerInner {
            limit: policy.cache_bytes,
            used: AtomicU64::new(0),
        }))
    }

    #[must_use]
    pub fn unbounded() -> Self {
        Self::new(CachePolicy::unbounded())
    }

    #[must_use]
    pub fn used(&self) -> u64 {
        self.0.used.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn limit(&self) -> u64 {
        self.0.limit
    }

    /// Reserve before cache growth; retain the owner until release.
    /// # Errors
    /// Refuses bytes beyond the cache allowance.
    pub fn reserve(&self, bytes: u64) -> Result<CacheReservation, CacheError> {
        let limit = self.0.limit;
        self.0
            .used
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|next| *next <= limit)
            })
            .map(|_| CacheReservation {
                ledger: Arc::clone(&self.0),
                bytes,
            })
            .map_err(|used| CacheError::Exhausted {
                used,
                requested: bytes,
                limit,
            })
    }
}

/// Linear cache reservation: refunds exactly once at drop.
#[derive(Debug)]
pub struct CacheReservation {
    ledger: Arc<CacheLedgerInner>,
    bytes: u64,
}

impl CacheReservation {
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for CacheReservation {
    fn drop(&mut self) {
        self.ledger.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

/// Shared generation owner: resolver storage and generation identity (C3).
/// Every token-bearing image holds this owner. Cache map entries are
/// eviction references, not charge owners.
#[derive(Debug)]
pub struct GenerationState {
    identity: CacheGeneration,
    ledger: CacheLedger,
    resolver: Mutex<TextInterner>,
}

/// Strong handle to a [`GenerationState`]. Borrowed resolver views do not
/// keep the generation alive; this handle does.
#[derive(Debug, Clone)]
pub struct GenerationHandle(Arc<GenerationState>);

/// Weak, versioned handle for idle prepared memos. Upgrade fails after
/// the last strong owner (image, execution, or current-cache pin) drops.
#[derive(Debug, Clone)]
pub struct WeakGenerationHandle {
    identity: CacheGeneration,
    inner: Weak<GenerationState>,
}

/// Borrowed view of a generation's resolver. Must not outlive the handle.
/// Resolution locks the generation mutex per call; it does not intern.
#[derive(Debug, Clone, Copy)]
pub struct ResolverView<'a> {
    generation: CacheGeneration,
    state: &'a GenerationState,
}

impl GenerationState {
    #[must_use]
    pub fn new(identity: CacheGeneration, ledger: CacheLedger) -> Self {
        Self {
            identity,
            ledger,
            resolver: Mutex::new(TextInterner::new(
                crate::image::TextGeneration::of(identity),
            )),
        }
    }

    #[must_use]
    pub const fn identity(&self) -> CacheGeneration {
        self.identity
    }

    #[must_use]
    pub fn ledger(&self) -> &CacheLedger {
        &self.ledger
    }

    pub(crate) fn lock_resolver(&self) -> std::sync::MutexGuard<'_, TextInterner> {
        self.resolver.lock().expect("generation resolver")
    }
}

impl GenerationHandle {
    #[must_use]
    pub fn new(state: GenerationState) -> Self {
        Self(Arc::new(state))
    }

    #[must_use]
    pub fn state(&self) -> &GenerationState {
        &self.0
    }

    #[must_use]
    pub fn identity(&self) -> CacheGeneration {
        self.0.identity
    }

    #[must_use]
    pub fn ledger(&self) -> &CacheLedger {
        &self.0.ledger
    }

    /// True when both handles own the same resolver allocation.
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    #[must_use]
    pub fn resolver(&self) -> ResolverView<'_> {
        ResolverView {
            generation: self.0.identity,
            state: &self.0,
        }
    }

    #[must_use]
    pub fn downgrade(&self) -> WeakGenerationHandle {
        WeakGenerationHandle {
            identity: self.0.identity,
            inner: Arc::downgrade(&self.0),
        }
    }

    /// Strong-count of this generation, including this handle.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    pub(crate) fn lock_resolver(&self) -> std::sync::MutexGuard<'_, TextInterner> {
        self.0.lock_resolver()
    }

    /// The one production text equality: intern, scratch, and mixed.
    /// [`crate::image::TextEq::tokens_equal`] is `Result<bool, _>` —
    /// resolver failure is `Err`, not inequality. Stamp retained tokens
    /// with `eq.scratch_epoch()` and rebind via
    /// [`crate::image::TextEq::with_memo_stamp`] after a store replace.
    #[must_use]
    pub fn text_eq<'a>(
        &'a self,
        scratch: Option<&'a crate::image::NonresidentTextStore>,
    ) -> crate::image::TextEq<'a> {
        crate::image::TextEq::bind(self, scratch)
    }

    /// Resident-only compare. Scratch-tagged ids are not this resolver's
    /// words — mixed intern/scratch uses [`Self::text_eq`].
    #[must_use]
    pub fn tokens_equal(&self, left: u64, other: &Self, right: u64) -> bool {
        if !crate::image::is_resident_token(left) || !crate::image::is_resident_token(right) {
            return false;
        }
        if self.ptr_eq(other) {
            return left == right;
        }
        let left_text = self.resolver().owned_text(left);
        let right_text = other.resolver().owned_text(right);
        match (left_text, right_text) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }
}

impl WeakGenerationHandle {
    #[must_use]
    pub const fn identity(&self) -> CacheGeneration {
        self.identity
    }

    #[must_use]
    pub fn upgrade(&self) -> Option<GenerationHandle> {
        self.inner.upgrade().map(GenerationHandle)
    }
}

impl ResolverView<'_> {
    #[must_use]
    pub const fn generation(&self) -> CacheGeneration {
        self.generation
    }

    #[must_use]
    pub fn lookup(&self, text: &str) -> Option<u64> {
        self.state.lock_resolver().lookup(text)
    }

    #[must_use]
    pub fn lookup_word(&self, text: &str) -> u64 {
        self.state.lock_resolver().lookup_word(text)
    }

    pub fn with_text<R>(&self, token: u64, read: impl FnOnce(&str) -> R) -> Option<R> {
        let intern = self.state.lock_resolver();
        intern.text_of(token).map(read)
    }

    /// Copies resolved text. Prefer [`Self::with_text`] on the hot path.
    #[must_use]
    pub fn owned_text(&self, token: u64) -> Option<std::sync::Arc<str>> {
        self.state.lock_resolver().owned_text(token)
    }
}

/// One synchronized acquire/rotate cursor for a database cache.
#[derive(Debug)]
pub struct GenerationProtocol {
    current: Mutex<GenerationHandle>,
}

impl GenerationProtocol {
    #[must_use]
    pub fn new(ledger: CacheLedger) -> Self {
        Self {
            current: Mutex::new(GenerationHandle::new(GenerationState::new(
                CacheGeneration::initial(),
                ledger,
            ))),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, GenerationHandle> {
        self.current.lock().expect("generation protocol")
    }

    /// Clone the current generation. This is the only acquisition path.
    #[must_use]
    pub fn acquire(&self) -> GenerationHandle {
        self.lock().clone()
    }

    #[must_use]
    pub fn identity(&self) -> CacheGeneration {
        self.lock().identity()
    }

    /// Install a fresh generation as current. The previous handle is
    /// returned so the caller can drop the cache's strong pin after
    /// detaching map entries. Live image owners keep the old resolver.
    pub fn rotate(&self, ledger: &CacheLedger) -> (GenerationHandle, GenerationHandle) {
        let mut current = self.lock();
        let previous = current.clone();
        let next = GenerationHandle::new(GenerationState::new(
            previous.identity().next(),
            ledger.clone(),
        ));
        *current = next.clone();
        (previous, next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_rotate_do_not_alias_resolvers() {
        let ledger = CacheLedger::unbounded();
        let protocol = GenerationProtocol::new(ledger.clone());
        let first = protocol.acquire();
        assert_eq!(first.identity(), CacheGeneration::initial());
        let (previous, next) = protocol.rotate(&ledger);
        assert!(first.ptr_eq(&previous));
        assert!(!first.ptr_eq(&next));
        assert_eq!(next.identity().as_u64(), 1);
        assert_eq!(protocol.acquire().identity().as_u64(), 1);
    }

    #[test]
    fn weak_idle_handle_fails_after_last_strong_drop() {
        let ledger = CacheLedger::unbounded();
        let handle = GenerationHandle::new(GenerationState::new(
            CacheGeneration::initial(),
            ledger,
        ));
        let weak = handle.downgrade();
        assert_eq!(weak.identity(), CacheGeneration::initial());
        assert!(weak.upgrade().is_some());
        drop(handle);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn cache_reservation_refunds_on_drop() {
        let ledger = CacheLedger::new(CachePolicy {
            cache_bytes: 1024,
        });
        let reservation = ledger.reserve(512).expect("reserve");
        assert_eq!(ledger.used(), 512);
        drop(reservation);
        assert_eq!(ledger.used(), 0);
    }
}
