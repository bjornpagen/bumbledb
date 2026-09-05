//! Exact scratch-backed text resolution for nonresident execution.
//!
//! Forward/reverse maps are L03 charged [`ScratchRelation`]s. A small
//! working-charged warm alias cache sits in front of them so [`TextEq`]
//! does not lock the intern or walk bytes on a warm join. The cache is
//! bounded; eviction never drops the exact scratch entries.
//!
//! Store identity is [`TextStoreEpoch`] (full `u64`, never packed into
//! tokens). Memos stamp `store.epoch()` and invalidate on mismatch.
//! Scratch tokens are [`SCRATCH_TOKEN_TAG`] `| dense`; `u64::MAX` is
//! never minted.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{CorruptionError, Error, Result};
use crate::exec::scratch::{ScratchCapability, ScratchRelation};
use crate::image::epoch::TextGeneration;
use crate::image::intern::{SCRATCH_TOKEN_TAG, SENTINEL_WORD, is_resident_token, is_scratch_token};
use crate::work::{ByteKind, ByteReservation, GenerationHandle, WorkContext, WorkError};

static NEXT_STORE_EPOCH: AtomicU64 = AtomicU64::new(1);

const WARM_ENTRY_BYTES: u64 = 32;
const WARM_CACHE_LIMIT: u64 = 64 * 1024;

/// Owner identity of one [`NonresidentTextStore`]. Not packed into tokens.
/// Stamp memos with this; invalidate when it differs or the store is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextStoreEpoch(u64);

impl TextStoreEpoch {
    fn next() -> Self {
        let prior = NEXT_STORE_EPOCH.fetch_add(1, Ordering::Relaxed);
        assert!(
            prior != 0 && prior != u64::MAX,
            "text store epoch space exhausted"
        );
        Self(prior)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Tokens no longer pack an epoch. Stamp [`NonresidentTextStore::epoch`]
/// on memos instead of recovering identity from a word.
#[must_use]
pub const fn scratch_token_epoch(_token: u64) -> Option<TextStoreEpoch> {
    None
}

fn dense_of(token: u64) -> Option<u64> {
    is_scratch_token(token).then_some(token & !SCRATCH_TOKEN_TAG)
}

fn mint_token(dense: u64) -> Result<u64, WorkError> {
    if dense >= SCRATCH_TOKEN_TAG {
        return Err(WorkError::Exhausted {
            resource: crate::work::Resource::Rows,
            used: dense,
            requested: 1,
            limit: SCRATCH_TOKEN_TAG,
        });
    }
    let token = SCRATCH_TOKEN_TAG | dense;
    if token == SENTINEL_WORD {
        return Err(WorkError::Exhausted {
            resource: crate::work::Resource::Rows,
            used: dense,
            requested: 1,
            limit: SCRATCH_TOKEN_TAG,
        });
    }
    Ok(token)
}

/// Bounded, working-charged alias cache. Exact text stays in scratch.
#[derive(Debug)]
struct WarmAliases {
    to_canonical: HashMap<u64, u64>,
    charges: HashMap<u64, ByteReservation>,
    bytes: u64,
    limit: u64,
    work: WorkContext,
}

impl WarmAliases {
    fn new(work: WorkContext, limit: u64) -> Self {
        Self {
            to_canonical: HashMap::new(),
            charges: HashMap::new(),
            bytes: 0,
            limit,
            work,
        }
    }

    fn get(&self, token: u64) -> Option<u64> {
        self.to_canonical.get(&token).copied()
    }

    fn insert(&mut self, token: u64, canonical: u64) {
        if WARM_ENTRY_BYTES > self.limit {
            return;
        }
        if self.to_canonical.contains_key(&token) {
            self.to_canonical.insert(token, canonical);
            return;
        }
        while self.bytes + WARM_ENTRY_BYTES > self.limit && !self.to_canonical.is_empty() {
            self.evict_one();
        }
        let Ok(charge) = self.work.reserve(ByteKind::Working, WARM_ENTRY_BYTES) else {
            return;
        };
        self.bytes += WARM_ENTRY_BYTES;
        self.charges.insert(token, charge);
        self.to_canonical.insert(token, canonical);
    }

    fn evict_one(&mut self) {
        let Some((&token, _)) = self.to_canonical.iter().next() else {
            return;
        };
        self.to_canonical.remove(&token);
        self.charges.remove(&token);
        self.bytes = self.bytes.saturating_sub(WARM_ENTRY_BYTES);
    }

    fn bytes(&self) -> u64 {
        self.bytes
    }

    fn limit(&self) -> u64 {
        self.limit
    }
}

/// The one production text-token equality. Filters, unification, joins,
/// negation, grouping and dedup call [`Self::tokens_equal`] /
/// [`Self::canonical`] — not raw `u64 ==`.
#[derive(Clone, Copy)]
pub struct TextEq<'a> {
    generation: &'a GenerationHandle,
    scratch: Option<&'a NonresidentTextStore>,
    stamp: Option<TextStoreEpoch>,
}

impl<'a> TextEq<'a> {
    #[must_use]
    pub fn bind(
        generation: &'a GenerationHandle,
        scratch: Option<&'a NonresidentTextStore>,
    ) -> Self {
        Self {
            generation,
            scratch,
            stamp: scratch.map(NonresidentTextStore::epoch),
        }
    }

    /// Bind a memo stamp. Scratch tokens miss when `stamp` is not this
    /// store's epoch (replacement / drop).
    #[must_use]
    pub fn with_memo_stamp(self, stamp: TextStoreEpoch) -> Self {
        Self {
            stamp: Some(stamp),
            ..self
        }
    }

    #[must_use]
    pub fn scratch_epoch(self) -> Option<TextStoreEpoch> {
        self.scratch.map(NonresidentTextStore::epoch)
    }

    #[must_use]
    pub fn accepts_stamp(self, stamp: TextStoreEpoch) -> bool {
        self.scratch_epoch() == Some(stamp)
    }

    fn scratch_live(self) -> bool {
        match (self.scratch_epoch(), self.stamp) {
            (Some(live), Some(want)) => live == want,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }

    /// Canonical identity for equality, hashing, grouping and dedup.
    /// Warm scratch hits the charged alias cache (no intern lock).
    ///
    /// `Ok(None)` is a miss (stale stamp, not live, not text).
    /// `Err` is scratch I/O, work refusal, or corrupt UTF-8 — never unequal.
    pub fn canonical(self, token: u64) -> Result<Option<u64>> {
        if is_resident_token(token) {
            return Ok(Some(token));
        }
        if !is_scratch_token(token) || !self.scratch_live() {
            return Ok(None);
        }
        let Some(store) = self.scratch else {
            return Ok(None);
        };
        if !store.live(token) {
            return Ok(None);
        }
        if let Some(canonical) = store.warm.get(token) {
            if store.handle.ptr_eq(self.generation) {
                return Ok(Some(canonical));
            }
        }
        store
            .alias_from_scratch(token, self.generation)
            .map(Some)
    }

    /// Same function as [`Self::canonical`]: grouping/hash/dedup keys.
    pub fn identity(self, token: u64) -> Result<Option<u64>> {
        self.canonical(token)
    }

    /// Production compare. `Ok(true)` / `Ok(false)` is the verdict.
    /// `Err` fails execution; it is not inequality.
    pub fn tokens_equal(self, left: u64, right: u64) -> Result<bool> {
        match (self.canonical(left)?, self.canonical(right)?) {
            (Some(left), Some(right)) => Ok(
                left == right || self.generation.tokens_equal(left, self.generation, right),
            ),
            _ => Ok(false),
        }
    }
}

/// A bounded exact text resolver backed by charged scratch maps.
pub struct NonresidentTextStore {
    handle: GenerationHandle,
    generation: TextGeneration,
    epoch: TextStoreEpoch,
    forward: ScratchRelation,
    reverse: Mutex<ScratchRelation>,
    next_dense: u64,
    warm: WarmAliases,
    #[cfg(test)]
    alias_fault: std::cell::Cell<Option<TextAliasFault>>,
}

impl std::fmt::Debug for NonresidentTextStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonresidentTextStore")
            .field("epoch", &self.epoch)
            .field("next_dense", &self.next_dense)
            .field("warm_bytes", &self.warm.bytes())
            .finish_non_exhaustive()
    }
}

impl NonresidentTextStore {
    /// Open one forward/reverse pair under an explicit scratch capability.
    /// Production execute/spill must construct via
    /// [`crate::image::ResidentTextExhausted::open_nonresident`].
    #[must_use]
    pub(super) fn new(capability: &ScratchCapability, generation: &GenerationHandle) -> Self {
        let work = capability.work().clone();
        Self {
            handle: generation.clone(),
            generation: TextGeneration::of(generation.identity()),
            epoch: TextStoreEpoch::next(),
            forward: capability.relation(),
            reverse: Mutex::new(capability.relation_with_ram(0)),
            next_dense: 0,
            warm: WarmAliases::new(work, WARM_CACHE_LIMIT),
            #[cfg(test)]
            alias_fault: std::cell::Cell::new(None),
        }
    }

    pub const TOKEN_TAG: u64 = SCRATCH_TOKEN_TAG;

    #[must_use]
    pub const fn owns_token(token: u64) -> bool {
        is_scratch_token(token)
    }

    #[must_use]
    pub(super) fn bind(capability: &ScratchCapability, generation: &GenerationHandle) -> Self {
        Self::new(capability, generation)
    }

    #[must_use]
    pub const fn generation(&self) -> TextGeneration {
        self.generation
    }

    /// Owner identity. Stamp `param_word_memo` / `arena_ranges` / latched
    /// literals with this. Invalidate when it changes or the store is dropped.
    #[must_use]
    pub const fn epoch(&self) -> TextStoreEpoch {
        self.epoch
    }

    /// Dense id minted by **this** instance. Not a cross-store identity;
    /// pair with [`Self::epoch`] on memos.
    #[must_use]
    pub fn live(&self, token: u64) -> bool {
        dense_of(token).is_some_and(|dense| dense < self.next_dense)
    }

    #[must_use]
    pub fn text_eq(&self) -> TextEq<'_> {
        TextEq::bind(&self.handle, Some(self))
    }

    /// Charged warm-cache retained bytes (not scratch map traffic).
    #[must_use]
    pub fn resident_cache_bytes(&self) -> u64 {
        self.warm.bytes()
    }

    #[must_use]
    pub fn resident_cache_limit(&self) -> u64 {
        self.warm.limit()
    }

    fn alias_from_scratch(&self, token: u64, generation: &GenerationHandle) -> Result<u64> {
        #[cfg(test)]
        if let Some(fault) = self.alias_fault.take() {
            return Err(fault.into_error());
        }
        let mut out = Vec::new();
        let mut reverse = self.reverse.lock().expect("scratch reverse");
        let hit = reverse.get(&encode_token(token), &mut out)?;
        drop(reverse);
        if !hit {
            return Ok(token);
        }
        let text = std::str::from_utf8(&out).map_err(|_| {
            Error::Corruption(CorruptionError::MalformedValue("nonresident text"))
        })?;
        Ok(generation.resolver().lookup(text).unwrap_or(token))
    }

    fn remember(&mut self, token: u64, text: &str) {
        let canonical = self.handle.resolver().lookup(text).unwrap_or(token);
        self.warm.insert(token, canonical);
    }

    /// Mint or find the token. Both scratch directions are written before
    /// the token is published; a failed put does not increment dense and
    /// does not populate the warm cache.
    /// # Errors
    /// Stopped work, scratch refusal, or I/O failure.
    pub fn intern(&mut self, text: &str, work: &WorkContext) -> Result<u64> {
        work.step(1 + text.len() as u64)
            .map_err(|error| Error::from_store(crate::storage::store::StoreError::Work(error)))?;
        let mut found = Vec::new();
        if self.forward.get(text.as_bytes(), &mut found)? {
            let token = decode_token(&found)?;
            if self.live(token) {
                self.remember(token, text);
                return Ok(token);
            }
        }
        let token = mint_token(self.next_dense).map_err(|error| {
            Error::from_store(crate::storage::store::StoreError::Work(error))
        })?;
        let encoded = encode_token(token);
        // Reverse first: a failed forward put must not leave a get-hit
        // that publishes a token whose reverse/next_dense are unset.
        self.reverse
            .lock()
            .expect("scratch reverse")
            .put(&encoded, text.as_bytes())?;
        if let Err(error) = self.forward.put(text.as_bytes(), &encoded) {
            return Err(error);
        }
        self.next_dense += 1;
        self.remember(token, text);
        Ok(token)
    }

    /// Resolve a minted token to its exact text bytes.
    /// # Errors
    /// Stopped work, scratch refusal, or I/O failure.
    pub fn resolve(&mut self, token: u64, out: &mut Vec<u8>) -> Result<bool> {
        if !is_scratch_token(token) {
            return Ok(false);
        }
        self.reverse
            .lock()
            .expect("scratch reverse")
            .get(&encode_token(token), out)
    }

    /// Exact token↔canonical text compare.
    /// # Errors
    /// As [`Self::resolve`].
    pub fn token_eq_text(&mut self, token: u64, text: &str) -> Result<bool> {
        let mut out = Vec::new();
        if !self.resolve(token, &mut out)? {
            return Ok(false);
        }
        Ok(out == text.as_bytes())
    }

    /// Mixed compare via [`TextEq`].
    /// # Errors
    /// Scratch I/O, work refusal, or corrupt UTF-8 from the resolver.
    pub fn tokens_equal_resident(
        &mut self,
        token: u64,
        resident: &GenerationHandle,
        resident_token: u64,
    ) -> Result<bool> {
        TextEq::bind(resident, Some(self)).tokens_equal(token, resident_token)
    }

    /// Next cold-cache alias lookup fails with this typed error (tests).
    #[cfg(test)]
    pub(crate) fn inject_alias_fault(&self, fault: TextAliasFault) {
        self.alias_fault.set(Some(fault));
    }
}

/// Injected cold-cache alias failure. Production never constructs this.
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextAliasFault {
    ReverseGet,
    Utf8,
    Work,
}

#[cfg(test)]
impl TextAliasFault {
    fn into_error(self) -> Error {
        match self {
            Self::ReverseGet => {
                Error::from_store(crate::storage::store::StoreError::Allocation)
            }
            Self::Utf8 => Error::Corruption(CorruptionError::MalformedValue("nonresident text")),
            Self::Work => Error::from_store(crate::storage::store::StoreError::Work(
                WorkError::Cancelled,
            )),
        }
    }
}

fn encode_token(token: u64) -> [u8; 8] {
    token.to_be_bytes()
}

fn decode_token(bytes: &[u8]) -> Result<u64> {
    let array: [u8; 8] = bytes.try_into().map_err(|_| {
        Error::Corruption(crate::error::CorruptionError::MalformedValue("text token"))
    })?;
    Ok(u64::from_be_bytes(array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::prepared::source::UNBOUNDED_POLICY;
    use crate::exec::scratch::capability::ScratchPolicy;
    use crate::work::{CacheLedger, ExecutionPolicy, GenerationHandle, GenerationState};

    fn capability() -> ScratchCapability {
        ScratchCapability::start(UNBOUNDED_POLICY, ScratchPolicy::unbounded()).expect("start")
    }

    fn generation() -> GenerationHandle {
        GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::unbounded(),
        ))
    }

    #[test]
    fn nonresident_text_intern_and_resolve_are_exact() {
        let cap = capability();
        let generation = generation();
        let mut store = NonresidentTextStore::new(&cap, &generation);
        let work = cap.work().clone();
        let alpha = store.intern("alpha", &work).expect("intern");
        let beta = store.intern("beta", &work).expect("intern");
        assert_ne!(alpha, beta);
        assert!(is_scratch_token(alpha) && is_scratch_token(beta));
        assert!(!is_resident_token(alpha));
        assert_ne!(alpha, 0, "scratch ids never reuse intern space");
        assert_ne!(alpha, SENTINEL_WORD);
        assert!(store.live(alpha));
        assert_eq!(scratch_token_epoch(alpha), None);
        assert_eq!(store.intern("alpha", &work).expect("re-intern"), alpha);
        let mut out = Vec::new();
        assert!(store.resolve(alpha, &mut out).expect("resolve"));
        assert_eq!(out, b"alpha");
        assert!(
            !store.resolve(0, &mut out).expect("resident id misses"),
            "intern token 0 is not a scratch word"
        );
    }

    #[test]
    fn nonresident_text_continues_under_tiny_scratch_ram() {
        let policy = ExecutionPolicy {
            scratch_bytes: 1 << 20,
            ..UNBOUNDED_POLICY
        };
        let cap = ScratchCapability::start(policy, ScratchPolicy::unbounded()).expect("start");
        let generation = generation();
        let mut store = NonresidentTextStore::new(&cap, &generation);
        let work = cap.work().clone();
        for index in 0..512u32 {
            let text = format!("distinct-text-{index:04}");
            store.intern(&text, &work).expect("intern");
        }
        let first = store.intern("distinct-text-0000", &work).expect("first");
        assert!(is_scratch_token(first));
        assert_ne!(first, 0);
        let mut out = Vec::new();
        assert!(store.resolve(first, &mut out).expect("resolve"));
        assert_eq!(out, b"distinct-text-0000");
        assert!(!store.resolve(0, &mut out).expect("intern id misses"));
    }

    #[test]
    fn nonresident_compares_exact_bytes_to_a_resident_generation() {
        let cap = capability();
        let work = cap.work().clone();
        let resident = generation();
        let token = resident
            .lock_resolver()
            .intern("shared", &work, resident.ledger())
            .expect("resident intern");
        let mut store = NonresidentTextStore::bind(&cap, &resident);
        let scratch = store.intern("shared", &work).expect("scratch intern");
        assert_ne!(scratch, token, "scratch and intern ids are disjoint");
        assert!(is_scratch_token(scratch) && is_resident_token(token));
        assert!(
            store
                .tokens_equal_resident(scratch, &resident, token)
                .expect("compare")
        );
        assert!(TextEq::bind(&resident, Some(&store))
            .tokens_equal(scratch, token)
            .expect("equal"));
        assert_eq!(
            TextEq::bind(&resident, Some(&store))
                .canonical(scratch)
                .expect("canonical"),
            TextEq::bind(&resident, Some(&store))
                .identity(scratch)
                .expect("identity")
        );
        let other = store.intern("other", &work).expect("other");
        assert!(!store
            .tokens_equal_resident(other, &resident, token)
            .expect("unequal"));
    }

    #[test]
    fn d02_text_eq_unifies_intern_and_scratch_without_raw_word() {
        let cap = capability();
        let work = cap.work().clone();
        let generation = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            crate::work::CacheLedger::new(crate::work::CachePolicy { cache_bytes: 8 }),
        ));
        let admitted = crate::image::intern::InternerHandle::new(&generation, &work)
            .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
            .expect("work");
        let crate::image::ResidentAdmit::BeyondMemory(exhausted) = admitted else {
            panic!("tiny cache must spill");
        };
        let mut store = exhausted.open_nonresident(&cap);
        let scratch = store
            .intern("shared-meaning", cap.work())
            .expect("scratch");
        let fat = generation();
        let intern = fat
            .lock_resolver()
            .intern("shared-meaning", &work, fat.ledger())
            .expect("intern");
        let eq = TextEq::bind(&fat, Some(&store));
        assert_ne!(scratch, intern, "raw words stay disjoint");
        assert!(eq.tokens_equal(scratch, intern).expect("equal"));
        assert_eq!(eq.canonical(scratch).expect("canonical"), Some(intern));
        assert_eq!(
            eq.identity(scratch).expect("identity"),
            eq.canonical(scratch).expect("canonical")
        );
        assert!(!eq
            .tokens_equal(scratch, intern.wrapping_add(1))
            .expect("unequal"));
    }

    #[test]
    fn d02_dropped_store_epoch_does_not_alias_new_text() {
        let cap = capability();
        let generation = generation();
        let exhausted = crate::image::ResidentTextExhausted::new(generation.clone());
        let mut first = exhausted.open_nonresident(&cap);
        let old = first.intern("alpha", cap.work()).expect("old");
        let old_epoch = first.epoch();
        assert!(first.live(old));
        drop(first);

        let mut second = exhausted.open_nonresident(&cap);
        assert_ne!(second.epoch(), old_epoch);
        let fresh = second.intern("beta", cap.work()).expect("new text");
        let eq = TextEq::bind(&generation, Some(&second)).with_memo_stamp(old_epoch);
        assert!(
            !eq.accepts_stamp(old_epoch),
            "replaced store rejects the old memo stamp"
        );
        assert!(
            eq.canonical(old).expect("stale miss").is_none(),
            "stale stamp: scratch tokens miss"
        );
        assert!(!eq.tokens_equal(old, fresh).expect("stale unequal"));
        let live = TextEq::bind(&generation, Some(&second));
        let remint = second.intern("alpha", cap.work()).expect("remint alpha");
        assert!(live.accepts_stamp(second.epoch()));
        assert!(!eq.tokens_equal(old, remint).expect("stale remint"));
        assert!(live.tokens_equal(remint, remint).expect("live identity"));
    }

    /// Mixed resident/scratch stays exact when the warm cache is tight.
    #[test]
    fn d02_mixed_exact_under_constrained_warm_cache() {
        let cap = capability();
        let work = cap.work().clone();
        let resident = generation();
        let intern = resident
            .lock_resolver()
            .intern("shared", &work, resident.ledger())
            .expect("intern");
        let mut store = NonresidentTextStore::new(&cap, &resident);
        for index in 0..4000u32 {
            store
                .intern(&format!("fill-{index:04}"), &work)
                .expect("fill");
        }
        let scratch = store.intern("shared", &work).expect("shared");
        assert!(store.resident_cache_bytes() <= store.resident_cache_limit());
        let eq = TextEq::bind(&resident, Some(&store));
        assert!(eq.tokens_equal(scratch, intern).expect("equal"));
        assert_eq!(
            eq.canonical(scratch).expect("canonical"),
            eq.identity(scratch).expect("identity")
        );
    }

    /// Failed scratch insert does not publish a token; retry is exact.
    #[test]
    fn d02_failed_insert_does_not_publish_then_retry() {
        let policy = ExecutionPolicy {
            scratch_bytes: 48,
            working_bytes: 1 << 16,
            ..UNBOUNDED_POLICY
        };
        let cap = ScratchCapability::start(
            policy,
            ScratchPolicy {
                scratch_bytes: 48,
                ram_bytes_per_relation: 16,
            },
        )
        .expect("tiny scratch");
        let generation = generation();
        let mut store = NonresidentTextStore::new(&cap, &generation);
        let huge = "x".repeat(256);
        let first = store.intern(&huge, cap.work());
        assert!(first.is_err(), "tiny scratch refuses a huge insert");
        assert_eq!(store.resident_cache_bytes(), 0, "failed insert is not cached");
        let retry = store.intern(&huge, cap.work());
        assert!(retry.is_err(), "retry sees the same refusal, not a ghost hit");
        let cap = capability();
        let mut roomy = NonresidentTextStore::new(&cap, &generation);
        let ok = roomy.intern("ok", cap.work()).expect("roomy intern");
        assert!(is_scratch_token(ok));
        assert_ne!(ok, SENTINEL_WORD);
        assert!(roomy.live(ok));
    }

    /// Live dictionary memory stays within the charged warm-cache cap.
    #[test]
    fn d02_warm_dictionary_is_bounded_not_merely_counted() {
        let cap = capability();
        let generation = generation();
        let mut store = NonresidentTextStore::new(&cap, &generation);
        let work = cap.work().clone();
        let limit = store.resident_cache_limit();
        for index in 0..8000u32 {
            store
                .intern(&format!("bounded-dict-{index:05}"), &work)
                .expect("intern");
            assert!(
                store.resident_cache_bytes() <= limit,
                "warm cache must not grow without bound"
            );
        }
        let mut out = Vec::new();
        let first = store
            .intern("bounded-dict-00000", &work)
            .expect("still exact");
        assert!(store.resolve(first, &mut out).expect("scratch retains"));
        assert_eq!(out, b"bounded-dict-00000");
        assert!(store.resident_cache_bytes() <= limit);
    }

    /// Cold-cache reverse-get / UTF-8 / work refusal on a would-be-equal
    /// pair is a typed error, never `Ok(false)`.
    #[test]
    fn d02_alias_fault_is_typed_error_not_unequal() {
        let cap = capability();
        let work = cap.work().clone();
        let mint = generation();
        let mut store = NonresidentTextStore::new(&cap, &mint);
        let scratch = store.intern("shared-meaning", &work).expect("scratch");
        let fat = generation();
        let intern = fat
            .lock_resolver()
            .intern("shared-meaning", &work, fat.ledger())
            .expect("intern");
        assert_ne!(scratch, intern, "raw words stay disjoint");
        let eq = TextEq::bind(&fat, Some(&store));
        assert!(
            eq.tokens_equal(scratch, intern).expect("warm-or-cold equal"),
            "control: same text is equal before a fault"
        );
        for fault in [
            TextAliasFault::ReverseGet,
            TextAliasFault::Utf8,
            TextAliasFault::Work,
        ] {
            store.inject_alias_fault(fault);
            let verdict = eq.tokens_equal(scratch, intern);
            assert!(
                verdict.is_err(),
                "{fault:?} must fail execution, not report unequal"
            );
            assert_ne!(
                verdict,
                Ok(false),
                "{fault:?} must not become text inequality"
            );
        }
    }
}
