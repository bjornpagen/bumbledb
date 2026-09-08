//! Shared text resolver generations. Live images and resolved query state
//! pin canonical text owners independently of cache membership.
use crate::image::CacheGeneration;
use crate::image::intern::TextInterner;
use std::sync::{Arc, Mutex, Weak};

/// Shared generation owner: resolver storage and generation identity (C3).
/// Every token-bearing image holds this owner. Individual text payloads
/// are pinned separately, so keeping a generation does not retain its history.
#[derive(Debug)]
pub struct GenerationState {
    identity: CacheGeneration,
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
    pub fn new(identity: CacheGeneration) -> Self {
        Self {
            identity,
            resolver: Mutex::new(TextInterner::default()),
        }
    }

    #[must_use]
    pub const fn identity(&self) -> CacheGeneration {
        self.identity
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

    /// Compare pinned tokens in this generation without copying text.
    #[must_use]
    pub fn text_eq(&self) -> crate::image::TextEq<'_> {
        crate::image::TextEq::bind(self)
    }

    /// Compare live tokens, using exact bytes across different generations.
    #[must_use]
    pub fn tokens_equal(&self, left: u64, other: &Self, right: u64) -> bool {
        if left == crate::image::intern::SENTINEL_WORD
            || right == crate::image::intern::SENTINEL_WORD
        {
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

    /// Share the canonical allocation without copying text bytes.
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
    pub fn new() -> Self {
        Self {
            current: Mutex::new(GenerationHandle::new(GenerationState::new(
                CacheGeneration::initial(),
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
    pub fn rotate(&self) -> (GenerationHandle, GenerationHandle) {
        let mut current = self.lock();
        let previous = current.clone();
        let next = GenerationHandle::new(GenerationState::new(previous.identity().next()));
        *current = next.clone();
        (previous, next)
    }
}

impl Default for GenerationProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_rotate_do_not_alias_resolvers() {
        let protocol = GenerationProtocol::new();
        let first = protocol.acquire();
        assert_eq!(first.identity(), CacheGeneration::initial());
        let (previous, next) = protocol.rotate();
        assert!(first.ptr_eq(&previous));
        assert!(!first.ptr_eq(&next));
        assert_eq!(next.identity().as_u64(), 1);
        assert_eq!(protocol.acquire().identity().as_u64(), 1);
    }

    #[test]
    fn weak_idle_handle_fails_after_last_strong_drop() {
        let handle = GenerationHandle::new(GenerationState::new(CacheGeneration::initial()));
        let weak = handle.downgrade();
        assert_eq!(weak.identity(), CacheGeneration::initial());
        assert!(weak.upgrade().is_some());
        drop(handle);
        assert!(weak.upgrade().is_none());
    }
}
