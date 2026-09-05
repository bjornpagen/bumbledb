//! The cache-scoped text interner: successor of the deleted persisted
//! dictionary (ENG-006). Stored rows own their text inline; the query
//! engine joins on fixed 64-bit words, so every distinct text observed
//! during one [`TextGeneration`] receives one dense token. Token equality
//! is text equality by construction — the map is keyed by full text bytes,
//! never a hash verdict (Q-COLLISION).
//!
//! Tokens are **generation-scoped and never persisted**: whole-generation
//! eviction invalidates every token together. Retention charges the
//! database-owned [`CacheLedger`], not the minting operation's working
//! allowance — cache pays once; operations pay decode work only.

use std::collections::HashMap;
use std::sync::Arc;

use crate::exec::scratch::ScratchCapability;
use crate::image::epoch::TextGeneration;
use crate::image::NonresidentTextStore;
use crate::work::{
    CacheError, CacheLedger, CacheReservation, GenerationHandle, WorkContext, WorkError,
};

/// Resident intern or image admission refused: the cache ledger cannot
/// retain more text or slabs. L05 execute/spill must open scratch text
/// on this generation — do not treat this as a generic allocation Error.
#[derive(Debug, Clone)]
pub struct ResidentTextExhausted {
    generation: GenerationHandle,
}

impl ResidentTextExhausted {
    #[must_use]
    pub fn new(generation: GenerationHandle) -> Self {
        Self { generation }
    }

    #[must_use]
    pub fn generation(&self) -> &GenerationHandle {
        &self.generation
    }

    /// Production constructor. L05 execute/spill calls this on
    /// [`ResidentAdmit::BeyondMemory`]; tests must not bind the store
    /// without going through this refusal.
    #[must_use]
    pub fn open_nonresident(&self, capability: &ScratchCapability) -> NonresidentTextStore {
        NonresidentTextStore::bind(capability, &self.generation)
    }
}

/// Outcome of a production resident intern or image admit.
#[derive(Debug)]
pub enum ResidentAdmit<T> {
    Ready(T),
    BeyondMemory(ResidentTextExhausted),
}

impl<T> ResidentAdmit<T> {
    pub fn expect_ready(self, msg: &str) -> T {
        match self {
            Self::Ready(value) => value,
            Self::BeyondMemory(_) => panic!("{msg}: resident path exhausted"),
        }
    }
}

/// The never-minted miss token (the old dictionary's sentinel, retained):
/// a text absent from every image matches nothing under `Eq` and
/// everything under `Ne`, exactly like any other unequal word.
pub(crate) const SENTINEL_WORD: u64 = u64::MAX;

/// High bit set on every scratch-minted token. Resident intern ids stay
/// in `0..TAG`. Dense ids occupy the low bits; `u64::MAX` is never
/// minted. Store identity is [`crate::image::TextStoreEpoch`] on the
/// store, not packed into the word. Equality is [`crate::image::TextEq`].
pub const SCRATCH_TOKEN_TAG: u64 = 1 << 63;

/// True for a token minted by [`NonresidentTextStore`], never intern.
#[must_use]
pub const fn is_scratch_token(token: u64) -> bool {
    token != SENTINEL_WORD && token & SCRATCH_TOKEN_TAG != 0
}

/// True for a token minted by the resident interner, never scratch.
#[must_use]
pub const fn is_resident_token(token: u64) -> bool {
    token != SENTINEL_WORD && token & SCRATCH_TOKEN_TAG == 0
}

/// One append-only exact text→token map for a single [`TextGeneration`].
#[derive(Debug)]
pub(crate) struct TextInterner {
    generation: TextGeneration,
    map: HashMap<Arc<str>, u64>,
    texts: Vec<Arc<str>>,
    bytes: usize,
    charges: Vec<CacheReservation>,
}

impl Default for TextInterner {
    fn default() -> Self {
        Self::new(TextGeneration::initial())
    }
}

impl TextInterner {
    #[must_use]
    pub(crate) fn new(generation: TextGeneration) -> Self {
        Self {
            generation,
            map: HashMap::new(),
            texts: Vec::new(),
            bytes: 0,
            charges: Vec::new(),
        }
    }

    #[must_use]
    pub(crate) const fn generation(&self) -> TextGeneration {
        self.generation
    }

    /// The token of `text`, minting one if absent. Never returns
    /// [`SENTINEL_WORD`]. A mint reserves retained bytes against the cache
    /// ledger; a repeated intern of a known text charges no retention.
    /// # Errors
    /// Stopped work, exhausted cache allowance, or allocation refusal.
    pub(crate) fn intern(
        &mut self,
        text: &str,
        work: &WorkContext,
        cache: &CacheLedger,
    ) -> Result<u64, InternError> {
        work.step(1 + text.len() as u64)?;
        if let Some(token) = self.map.get(text) {
            return Ok(*token);
        }
        let token = u64::try_from(self.texts.len()).expect("token count fits u64");
        debug_assert!(
            is_resident_token(token),
            "resident mint stays below the scratch tag"
        );
        let retained = text.len()
            + std::mem::size_of::<Arc<str>>()
            + std::mem::size_of::<HashMap<Arc<str>, u64>>().min(64);
        let charge = cache
            .reserve(retained as u64)
            .map_err(InternError::Cache)?;
        let owned: Arc<str> = Arc::from(text);
        self.map
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.texts
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.charges
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.bytes += retained;
        self.charges.push(charge);
        self.map.insert(Arc::clone(&owned), token);
        self.texts.push(owned);
        Ok(token)
    }

    /// The token of `text` if it was ever interned in this generation.
    #[must_use]
    pub(crate) fn lookup(&self, text: &str) -> Option<u64> {
        self.map.get(text).copied()
    }

    /// As [`Self::lookup`], returning the sentinel on a miss.
    #[must_use]
    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.lookup(text).unwrap_or(SENTINEL_WORD)
    }

    /// The text of a minted token. Scratch-tagged ids miss — they are
    /// not this resolver's words.
    #[must_use]
    pub(crate) fn text_of(&self, token: u64) -> Option<&str> {
        if !is_resident_token(token) {
            return None;
        }
        usize::try_from(token)
            .ok()
            .and_then(|idx| self.texts.get(idx))
            .map(AsRef::as_ref)
    }

    /// Shared text handle: one allocation, no duplicate full-string copy.
    #[must_use]
    pub(crate) fn owned_text(&self, token: u64) -> Option<Arc<str>> {
        if !is_resident_token(token) {
            return None;
        }
        usize::try_from(token)
            .ok()
            .and_then(|idx| self.texts.get(idx))
            .cloned()
    }

    #[must_use]
    pub(crate) fn retained_bytes(&self) -> usize {
        self.bytes
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.texts.len()
    }
}

/// A borrow-bundled generation resolver + work ledger for resolve/bind.
/// The handle's generation is the token owner: a token cannot outlive it.
pub(crate) struct InternerHandle<'a> {
    generation: &'a GenerationHandle,
    work: &'a WorkContext,
}

impl<'a> InternerHandle<'a> {
    pub(crate) fn new(generation: &'a GenerationHandle, work: &'a WorkContext) -> Self {
        Self { generation, work }
    }

    #[must_use]
    pub(crate) const fn generation(&self) -> &'a GenerationHandle {
        self.generation
    }

    /// Production intern: cache refusal is [`ResidentAdmit::BeyondMemory`],
    /// not a swallowed allocation Error. L05 execute/bind/spill must match
    /// and call [`ResidentTextExhausted::open_nonresident`].
    /// # Errors
    /// Stopped work only. Cache/allocation refusal is `BeyondMemory`.
    pub fn intern_text(&self, text: &str) -> crate::error::Result<ResidentAdmit<u64>> {
        self.intern_or_spill(text)
    }

    /// Same as [`Self::intern_text`]: the named production spill seam.
    pub fn intern_or_spill(&self, text: &str) -> crate::error::Result<ResidentAdmit<u64>> {
        match self
            .generation
            .lock_resolver()
            .intern(text, self.work, self.generation.ledger())
        {
            Ok(token) => Ok(ResidentAdmit::Ready(token)),
            Err(InternError::Cache(_)) | Err(InternError::Allocation) => {
                Ok(ResidentAdmit::BeyondMemory(ResidentTextExhausted::new(
                    self.generation.clone(),
                )))
            }
            Err(InternError::Work(work)) => Err(crate::error::Error::from(InternError::Work(work))),
        }
    }

    /// # Errors
    /// As [`Self::intern_text`].
    pub fn latch(&self, bytes: &[u8]) -> crate::error::Result<ResidentAdmit<u64>> {
        let text = std::str::from_utf8(bytes)
            .expect("IR string literals are UTF-8 by construction (Value::String)");
        self.intern_or_spill(text)
    }

    pub(crate) fn with_text<R>(&self, token: u64, read: impl FnOnce(&str) -> R) -> Option<R> {
        self.generation.resolver().with_text(token, read)
    }

    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.generation.resolver().lookup_word(text)
    }

    /// Exact generation-aware comparison: same resolver uses token
    /// identity; distinct generations compare canonical bytes.
    #[must_use]
    pub(crate) fn tokens_equal(&self, left: u64, other: &Self, right: u64) -> bool {
        self.generation.tokens_equal(left, other.generation, right)
    }

    /// The one production equality, including an optional live scratch store.
    /// Stamp memos with `eq.scratch_epoch()`; do not recover epoch from a token.
    #[must_use]
    pub fn text_eq<'b>(
        &'b self,
        scratch: Option<&'b crate::image::NonresidentTextStore>,
    ) -> crate::image::TextEq<'b> {
        crate::image::TextEq::bind(self.generation, scratch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternError {
    Work(WorkError),
    Cache(CacheError),
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
            InternError::Cache(_) => {
                crate::error::Error::from_store(crate::storage::store::StoreError::Allocation)
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
    use crate::work::CachePolicy;

    fn work() -> WorkContext {
        crate::api::prepared::source::unbounded_work().expect("unbounded ledger")
    }

    fn cache() -> CacheLedger {
        CacheLedger::new(CachePolicy {
            cache_bytes: 1 << 20,
        })
    }

    #[test]
    fn tokens_are_dense_stable_and_exact() {
        let work = work();
        let cache = cache();
        let mut interner = TextInterner::default();
        let a = interner.intern("alpha", &work, &cache).expect("intern");
        let b = interner.intern("beta", &work, &cache).expect("intern");
        assert_ne!(a, b, "distinct texts, distinct tokens");
        assert_eq!(interner.intern("alpha", &work, &cache).expect("intern"), a);
        assert_eq!(interner.lookup("alpha"), Some(a));
        assert_eq!(interner.lookup("gamma"), None);
        assert_eq!(interner.lookup_word("gamma"), SENTINEL_WORD);
        assert_eq!(interner.text_of(a), Some("alpha"));
        assert_eq!(interner.text_of(b), Some("beta"));
        assert_eq!(interner.text_of(SENTINEL_WORD), None);
        assert_eq!(interner.len(), 2);
        assert!(interner.retained_bytes() >= "alpha".len() + "beta".len());
    }

    #[test]
    fn equal_bytes_decide_identity_never_a_hash() {
        let work = work();
        let cache = cache();
        let mut interner = TextInterner::default();
        let long_a = "x".repeat(600);
        let long_b = format!("{}y", "x".repeat(599));
        let a = interner.intern(&long_a, &work, &cache).expect("intern");
        let b = interner.intern(&long_b, &work, &cache).expect("intern");
        assert_ne!(a, b);
        assert_eq!(interner.text_of(a).map(str::len), Some(600));
    }

    #[test]
    fn interning_charges_the_work_ledger() {
        let context = crate::work::ExecutionPolicy {
            work_units: 8,
            ..crate::api::prepared::source::UNBOUNDED_POLICY
        }
        .start()
        .expect("start");
        let cache = cache();
        let mut interner = TextInterner::default();
        let result = interner.intern("far-too-long-for-eight-units", &context, &cache);
        assert!(
            matches!(result, Err(InternError::Work(_))),
            "byte-proportional charge stops before growth"
        );
    }

    #[test]
    fn retained_tokens_are_charged_to_the_cache_ledger() {
        let work = work();
        let cache = CacheLedger::new(CachePolicy {
            cache_bytes: 4096,
        });
        let mut interner = TextInterner::default();
        interner.intern("stable", &work, &cache).expect("mint");
        let charged = cache.used();
        assert!(charged > 0, "the mint reserved its retained bytes");
        interner.intern("stable", &work, &cache).expect("re-intern");
        assert_eq!(cache.used(), charged, "re-intern charges no retention");
        let mut refused = false;
        for index in 0..4096u32 {
            match interner.intern(&format!("text-{index:04}"), &work, &cache) {
                Ok(_) => {}
                Err(InternError::Cache(_)) => {
                    refused = true;
                    break;
                }
                Err(other) => panic!("typed cache refusal, got {other:?}"),
            }
        }
        assert!(refused, "4 KiB cannot retain thousands of distinct texts");
        assert!(
            interner.retained_bytes() as u64 <= cache.limit(),
            "retained bytes stay within the reserved allowance"
        );
        assert_eq!(interner.lookup("stable"), Some(0));
        assert_eq!(work.used(crate::work::Resource::WorkingBytes), 0);
    }

    #[test]
    fn generation_aware_compare_does_not_alias_rotated_tokens() {
        use crate::work::{GenerationHandle, GenerationState};
        let work = work();
        let old = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            cache(),
        ));
        let new = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial().next(),
            cache(),
        ));
        let old_alpha = old
            .lock_resolver()
            .intern("alpha", &work, old.ledger())
            .expect("old");
        let new_beta = new
            .lock_resolver()
            .intern("beta", &work, new.ledger())
            .expect("new beta");
        let new_alpha = new
            .lock_resolver()
            .intern("alpha", &work, new.ledger())
            .expect("new alpha");
        assert_eq!(old_alpha, new_beta, "dense remint reuses the same id");
        assert!(
            !old.tokens_equal(old_alpha, &new, new_beta),
            "raw token identity is not meaning across generations"
        );
        assert!(
            old.tokens_equal(old_alpha, &new, new_alpha),
            "exact remapping compares canonical bytes"
        );
    }

    /// Production-path discriminator: intern_or_spill refusal — not a
    /// test-only `NonresidentTextStore::bind` — opens scratch and compares
    /// via `tokens_equal_resident` / `GenerationHandle::tokens_equal`.
    #[test]
    fn d02_production_intern_or_spill_reaches_scratch_resolver() {
        use crate::api::prepared::source::UNBOUNDED_POLICY;
        use crate::exec::scratch::capability::ScratchPolicy;
        use crate::work::{GenerationHandle, GenerationState};

        let work = work();
        let generation = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::new(CachePolicy { cache_bytes: 8 }),
        ));
        let handle = InternerHandle::new(&generation, &work);
        let admitted = handle
            .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
            .expect("unbounded work");
        let ResidentAdmit::BeyondMemory(exhausted) = admitted else {
            panic!("tiny cache must refuse resident intern through intern_or_spill");
        };
        assert!(
            exhausted.generation().ptr_eq(&generation),
            "refusal carries the same generation owner"
        );

        let cap = crate::exec::scratch::ScratchCapability::start(
            UNBOUNDED_POLICY,
            ScratchPolicy::unbounded(),
        )
        .expect("scratch");
        let mut store = exhausted.open_nonresident(&cap);
        let scratch = store
            .intern("shared-meaning", cap.work())
            .expect("scratch intern");

        let resident = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            cache(),
        ));
        let resident_tok = resident
            .lock_resolver()
            .intern("shared-meaning", &work, resident.ledger())
            .expect("resident intern");
        assert!(is_scratch_token(scratch));
        assert!(is_resident_token(resident_tok));
        assert_ne!(
            scratch, resident_tok,
            "intern and scratch tokens cannot alias at 0…"
        );
        assert!(
            crate::image::TextEq::bind(&resident, Some(&store))
                .tokens_equal(scratch, resident_tok)
                .expect("equal"),
            "TextEq unifies intern and scratch; raw words stay unequal"
        );
        assert!(
            !resident.tokens_equal(resident_tok, &resident, scratch),
            "GenerationHandle::tokens_equal does not treat a scratch id as intern"
        );

        let other = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial().next(),
            cache(),
        ));
        let other_tok = other
            .lock_resolver()
            .intern("shared-meaning", &work, other.ledger())
            .expect("other intern");
        assert!(
            resident.tokens_equal(resident_tok, &other, other_tok),
            "L05 compare across generations is byte-exact, never raw token identity"
        );
        assert!(
            !resident.tokens_equal(resident_tok, &other, 1),
            "unequal meanings stay unequal after remapping"
        );
    }

    /// Intern dense `0…` and scratch `TAG|0…` are disjoint: finalize
    /// must dispatch on [`is_scratch_token`], not try intern then store.
    #[test]
    fn d02_intern_and_scratch_tokens_do_not_alias() {
        use crate::api::prepared::source::UNBOUNDED_POLICY;
        use crate::exec::scratch::capability::ScratchPolicy;
        use crate::work::{GenerationHandle, GenerationState};

        let work = work();
        let generation = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            cache(),
        ));
        let intern_tok = generation
            .lock_resolver()
            .intern("shared", &work, generation.ledger())
            .expect("resident 0");
        assert_eq!(intern_tok, 0);
        assert!(is_resident_token(intern_tok));

        let exhausted = ResidentTextExhausted::new(generation.clone());
        let cap = crate::exec::scratch::ScratchCapability::start(
            UNBOUNDED_POLICY,
            ScratchPolicy::unbounded(),
        )
        .expect("scratch");
        let mut store = exhausted.open_nonresident(&cap);
        let scratch_tok = store.intern("shared", cap.work()).expect("scratch");
        assert!(is_scratch_token(scratch_tok));
        assert!(NonresidentTextStore::owns_token(scratch_tok));
        assert!(store.live(scratch_tok));
        assert_eq!(crate::image::scratch_token_epoch(scratch_tok), None);
        assert_ne!(intern_tok, scratch_tok);
        assert!(
            crate::image::TextEq::bind(&generation, Some(&store))
                .tokens_equal(scratch_tok, intern_tok)
                .expect("equal")
        );
        assert!(
            !generation.tokens_equal(intern_tok, &generation, scratch_tok),
            "raw identity across spaces is not meaning"
        );
        assert!(
            generation
                .resolver()
                .with_text(scratch_tok, |_| true)
                .is_none(),
            "intern resolve misses a scratch id"
        );
        let mut out = Vec::new();
        assert!(
            !store.resolve(intern_tok, &mut out).expect("miss"),
            "scratch resolve misses an intern id"
        );
    }
}
