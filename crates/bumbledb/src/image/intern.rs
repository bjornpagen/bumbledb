//! The cache-scoped text interner: successor of the deleted persisted
//! dictionary. Stored rows own their text inline; the query
//! engine joins on fixed 64-bit words, so every distinct text observed
//! during one [`GenerationHandle`] receives a monotonically minted token. Token equality
//! is text equality by construction — the map is keyed by full text bytes,
//! never a hash verdict (Q-COLLISION).
//!
//! Tokens are generation-scoped and never persisted. Consumers pin canonical
//! shared text; token numbers are monotone and never reused after reclamation.

use crate::work::{GenerationHandle, WorkContext, WorkError};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Reserved miss value, never minted as a text token.
pub(crate) const SENTINEL_WORD: u64 = u64::MAX;

/// Exact text→token and token→text indexes for one resolver namespace.
/// Token numbers never repeat, even after an entry is reclaimed.
#[derive(Debug, Default)]
pub(crate) struct TextInterner {
    map: HashMap<Arc<str>, u64>,
    texts: HashMap<u64, Arc<str>>,
    next_token: u64,
    bytes: usize,
    /// Round-robin token IDs, not extra text owners. Maintenance advances
    /// only on misses; a warm hit does no sweeping or allocation.
    reclaim_queue: VecDeque<u64>,
    misses_since_reclaim: u8,
    #[cfg(test)]
    reclaim_visits: usize,
}

/// A resident token together with its canonical text owner. The generation
/// stamp belongs to the surrounding image or resolved query state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternedText {
    pub(crate) word: u64,
    pub(crate) text: Arc<str>,
}

impl TextInterner {
    /// Intern exact bytes under the caller's resolver lock.
    pub(crate) fn intern(&mut self, text: &str, work: &WorkContext) -> Result<u64, InternError> {
        work.checkpoint()?;
        if let Some(token) = self.map.get(text) {
            return Ok(*token);
        }
        let token = self.next_token;
        let next = token.checked_add(1).ok_or(InternError::Allocation)?;
        self.bytes
            .checked_add(text.len())
            .ok_or(InternError::Allocation)?;
        self.misses_since_reclaim += 1;
        if self.misses_since_reclaim == 16 {
            self.reclaim(64);
            self.misses_since_reclaim = 0;
        }
        self.map
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.texts
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.reclaim_queue
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        let owned: Arc<str> = Arc::from(text);
        self.map.insert(Arc::clone(&owned), token);
        self.texts.insert(token, owned);
        self.reclaim_queue.push_back(token);
        self.bytes += text.len(); // checked before maintenance, which only subtracts
        self.next_token = next;
        Ok(token)
    }

    /// Test-only complete sweep; production incrementally visits at most
    /// 64 entries per 16 new texts. Work does not scale with the whole
    /// dictionary on any ordinary warm query, and IDs are never recycled.
    #[cfg(test)]
    pub(crate) fn reclaim_unowned(&mut self) {
        self.reclaim(self.reclaim_queue.len());
    }

    fn reclaim(&mut self, visits: usize) {
        for _ in 0..visits.min(self.reclaim_queue.len()) {
            let token = self
                .reclaim_queue
                .pop_front()
                .expect("bounded by queue length");
            let entry = self.texts.get(&token).expect("one queue entry per token");
            #[cfg(test)]
            {
                self.reclaim_visits += 1;
            }
            // The dictionary has exactly two strong references: its text
            // key and its token entry. A live consumer holds another Arc.
            if Arc::strong_count(entry) != 2 {
                self.reclaim_queue.push_back(token);
                continue;
            }
            self.map.remove(entry.as_ref());
            self.bytes -= entry.len();
            self.texts.remove(&token);
        }
    }

    /// The token of `text` if it is retained in this generation.
    #[must_use]
    pub(crate) fn lookup(&self, text: &str) -> Option<u64> {
        self.map.get(text).copied()
    }

    /// As [`Self::lookup`], returning the sentinel on a miss.
    #[must_use]
    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.lookup(text).unwrap_or(SENTINEL_WORD)
    }

    /// The text of a minted token. Reclaimed or unknown IDs miss.
    #[must_use]
    pub(crate) fn text_of(&self, token: u64) -> Option<&str> {
        if token == SENTINEL_WORD {
            return None;
        }
        self.texts.get(&token).map(AsRef::as_ref)
    }

    /// Shared text handle: one allocation, no duplicate full-string copy.
    #[must_use]
    pub(crate) fn owned_text(&self, token: u64) -> Option<Arc<str>> {
        if token == SENTINEL_WORD {
            return None;
        }
        self.texts.get(&token).map(Arc::clone)
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn retained_bytes(&self) -> usize {
        self.bytes
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.texts.len()
    }
}

/// A borrowed generation resolver and cancellation context for resolve/bind.
/// The generation establishes token identity. Retained values and images
/// separately own the canonical text behind the tokens they use.
pub(crate) struct InternerHandle<'a> {
    generation: Option<&'a GenerationHandle>,
    work: &'a WorkContext,
}

impl<'a> InternerHandle<'a> {
    pub(crate) fn new(generation: &'a GenerationHandle, work: &'a WorkContext) -> Self {
        Self {
            generation: Some(generation),
            work,
        }
    }

    /// Only the sealed text-free direct-probe capability may use this.
    /// This is absence of a resolver, never a substitute token namespace.
    pub(crate) fn without_text(work: &'a WorkContext) -> Self {
        Self {
            generation: None,
            work,
        }
    }

    pub(crate) fn text_eq(&self) -> crate::image::TextEq<'_> {
        crate::image::TextEq::from_optional_generation(self.generation)
    }

    #[must_use]
    pub(crate) fn generation(&self) -> &'a GenerationHandle {
        self.generation
            .expect("sealed text-free probe cannot resolve text")
    }

    /// Mint a token and pin its text atomically under the resolver lock.
    pub fn intern(&self, text: &str) -> crate::error::Result<InternedText> {
        let generation = self.generation.ok_or(crate::error::Error::Corruption(
            crate::error::CorruptionError::MalformedValue("text outside a sealed text-free probe"),
        ))?;
        let mut resolver = generation.lock_resolver();
        let word = resolver.intern(text, self.work)?;
        Ok(InternedText {
            word,
            text: resolver
                .owned_text(word)
                .expect("just interned under the same lock"),
        })
    }

    pub fn latch(&self, bytes: &[u8]) -> crate::error::Result<InternedText> {
        let text = std::str::from_utf8(bytes)
            .expect("IR string literals are UTF-8 by construction (Value::String)");
        self.intern(text)
    }

    pub(crate) fn with_text<R>(&self, token: u64, read: impl FnOnce(&str) -> R) -> Option<R> {
        self.generation().resolver().with_text(token, read)
    }

    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.generation().resolver().lookup_word(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternError {
    Work(WorkError),
    Allocation,
}

impl From<WorkError> for InternError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<InternError> for crate::error::Error {
    fn from(error: InternError) -> Self {
        match error {
            InternError::Work(work) => {
                crate::error::Error::from_store(crate::storage::store::StoreError::Work(work))
            }
            InternError::Allocation => {
                crate::error::Error::from_store(crate::storage::store::StoreError::Allocation)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::GenerationState;

    fn generation() -> GenerationHandle {
        GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
        ))
    }

    #[test]
    fn reclamation_preserves_live_owners_and_never_reuses_tokens() {
        let work = WorkContext::new();
        let mut interner = TextInterner::default();
        let old = interner.intern("obsolete", &work).unwrap();
        let live = interner.intern("live", &work).unwrap();
        let owner = interner.owned_text(live).unwrap();
        let before = interner.retained_bytes();
        interner.reclaim_unowned();
        assert_eq!(interner.lookup("obsolete"), None);
        assert_eq!(interner.text_of(old), None);
        assert_eq!(interner.lookup("live"), Some(live));
        assert_eq!(&*owner, "live");
        assert!(interner.retained_bytes() < before);
        let reminted = interner.intern("obsolete", &work).unwrap();
        assert!(reminted > live);
        assert_ne!(reminted, old);
        assert_eq!(interner.text_of(old), None, "reclaimed IDs never alias");
        drop(owner);
        interner.reclaim_unowned();
        assert_eq!(interner.len(), 0);
        assert_eq!(interner.retained_bytes(), 0);
    }

    #[test]
    fn token_exhaustion_never_mints_the_sentinel_or_wraps_after_reclamation() {
        let work = WorkContext::new();
        let mut interner = TextInterner {
            next_token: SENTINEL_WORD - 1,
            ..TextInterner::default()
        };
        let last = interner.intern("last", &work).unwrap();
        assert_eq!(last, SENTINEL_WORD - 1);
        let before = interner.retained_bytes();
        assert_eq!(
            interner.intern("overflow", &work),
            Err(InternError::Allocation)
        );
        assert_eq!(interner.lookup("overflow"), None);
        assert_eq!(interner.retained_bytes(), before);
        assert_eq!(interner.intern("last", &work).unwrap(), last);
        interner.reclaim_unowned();
        assert_eq!(
            interner.intern("after reclaim", &work),
            Err(InternError::Allocation)
        );
    }

    #[test]
    fn exact_bytes_define_identity_including_empty_unicode_and_long_prefixes() {
        let work = WorkContext::new();
        let mut interner = TextInterner::default();
        let texts = [
            String::new(),
            "alpha".into(),
            "β🐝".into(),
            "x".repeat(600),
            format!("{}y", "x".repeat(599)),
        ];
        for (index, text) in texts.iter().enumerate() {
            let token = interner.intern(text, &work).unwrap();
            assert_eq!(token, index as u64);
            assert_eq!(interner.intern(text, &work).unwrap(), token);
            assert_eq!(interner.lookup(text), Some(token));
            assert_eq!(interner.text_of(token), Some(text.as_str()));
        }
        assert_eq!(interner.lookup_word("absent"), SENTINEL_WORD);
        assert_eq!(interner.text_of(SENTINEL_WORD), None);
        assert_eq!(interner.len(), texts.len());
    }

    #[test]
    fn cancellation_is_checked_on_both_hits_and_misses_before_mutation() {
        let work = WorkContext::new();
        let mut interner = TextInterner::default();
        let token = interner.intern("live", &work).unwrap();
        let before = interner.retained_bytes();
        work.cancel();
        for text in ["live", "new"] {
            assert_eq!(
                interner.intern(text, &work),
                Err(InternError::Work(WorkError::Cancelled))
            );
        }
        assert_eq!(interner.retained_bytes(), before);
        assert_eq!(interner.lookup("new"), None);
        assert_eq!(interner.text_of(token), Some("live"));
    }

    #[test]
    fn handles_pin_the_same_allocation_and_warm_interning_allocates_nothing() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<InternerHandle<'static>>();
        send_sync::<crate::image::TextEq<'static>>();
        assert_eq!(
            std::mem::size_of::<InternerHandle<'_>>(),
            std::mem::size_of::<(&GenerationHandle, &WorkContext)>()
        );
        assert_eq!(
            std::mem::size_of::<crate::image::TextEq<'_>>(),
            std::mem::size_of::<&GenerationHandle>()
        );
        let generation = generation();
        let work = WorkContext::new();
        let handle = InternerHandle::new(&generation, &work);
        let live = handle.intern("stable shared payload").unwrap();
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        for _ in 0..128 {
            let next = handle.intern("stable shared payload").unwrap();
            assert_eq!(live.word, next.word);
            assert!(Arc::ptr_eq(&live.text, &next.text));
            assert!(handle.text_eq().tokens_equal(live.word, next.word).unwrap());
        }
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().window, before);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(handle.with_text(live.word, str::len), Some(live.text.len()));
        drop(live);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(generation.lock_resolver().len(), 0);
    }

    #[test]
    fn resident_text_has_no_fixed_cache_allowance() {
        let generation = generation();
        let work = WorkContext::new();
        let handle = InternerHandle::new(&generation, &work);
        let owners: Vec<_> = (0..8192)
            .map(|i| handle.intern(&format!("text-{i:08}")).unwrap())
            .collect();
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(generation.lock_resolver().len(), owners.len());
        for owner in &owners {
            assert_eq!(
                handle.with_text(owner.word, str::to_owned).as_deref(),
                Some(owner.text.as_ref())
            );
        }
        drop(owners);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(generation.lock_resolver().len(), 0);
    }

    #[test]
    fn automatic_reclamation_is_incremental_and_warm_hits_do_no_maintenance() {
        let generation = generation();
        let work = WorkContext::new();
        let handle = InternerHandle::new(&generation, &work);
        let owners: Vec<_> = (0..1024)
            .map(|i| handle.intern(&format!("live-{i}")).unwrap())
            .collect();
        for turn in 0..16_384 {
            let before = generation.lock_resolver().reclaim_visits;
            drop(handle.intern(&format!("discard-{turn}")).unwrap());
            let resolver = generation.lock_resolver();
            assert!(resolver.reclaim_visits - before <= 64);
            assert!(
                resolver.len() < 2048,
                "history must not accumulate behind live owners"
            );
            assert_eq!(resolver.lookup("live-0"), Some(owners[0].word));
        }
        let visits = generation.lock_resolver().reclaim_visits;
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        for _ in 0..128 {
            let same = handle.intern("live-0").unwrap();
            assert!(Arc::ptr_eq(&same.text, &owners[0].text));
        }
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().window, before);
        assert_eq!(generation.lock_resolver().reclaim_visits, visits);
        drop(owners);
        for turn in 0..4096 {
            drop(handle.intern(&format!("after-drop-{turn}")).unwrap());
        }
        let resolver = generation.lock_resolver();
        assert_eq!(resolver.lookup("live-0"), None);
        assert!(resolver.len() <= 32);
        assert_eq!(resolver.len(), resolver.reclaim_queue.len());
    }

    #[test]
    fn generation_aware_compare_never_aliases_rotated_tokens() {
        let work = WorkContext::new();
        let old = generation();
        let new = generation();
        let alpha = InternerHandle::new(&old, &work).intern("alpha").unwrap();
        let beta = InternerHandle::new(&new, &work).intern("beta").unwrap();
        let new_alpha = InternerHandle::new(&new, &work).intern("alpha").unwrap();
        assert_eq!(
            alpha.word, beta.word,
            "separate namespaces may mint the same number"
        );
        assert!(!old.tokens_equal(alpha.word, &new, beta.word));
        assert!(old.tokens_equal(alpha.word, &new, new_alpha.word));
        assert!(
            !old.text_eq()
                .tokens_equal(alpha.word, SENTINEL_WORD)
                .unwrap()
        );
        assert!(
            !old.text_eq()
                .tokens_equal(SENTINEL_WORD, SENTINEL_WORD)
                .unwrap()
        );
    }

    #[test]
    fn text_free_probe_cannot_silently_resolve_or_compare_text() {
        let work = WorkContext::new();
        let handle = InternerHandle::without_text(&work);
        assert!(matches!(
            handle.intern("no namespace"),
            Err(crate::error::Error::Corruption(_))
        ));
        assert!(matches!(
            handle.text_eq().tokens_equal(0, 0),
            Err(crate::error::Error::Corruption(_))
        ));
    }
}
