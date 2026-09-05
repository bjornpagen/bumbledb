//! The execution-scoped text interner: the successor of the deleted
//! persisted dictionary (ENG-006). Stored rows own their text inline; the
//! query engine still joins, dedups and groups on fixed 64-bit words, so
//! every distinct text VALUE observed by this cache's image builds, param
//! binds and literal latches receives one dense token. Token equality is
//! text equality by construction — the map is keyed by the full text
//! bytes, never a hash verdict — so forced fingerprint collisions cannot
//! alias two texts (Q-COLLISION).
//!
//! Tokens are **process/cache-scoped and append-only**: a token never
//! changes or retires while its [`super::cache::ImageCache`] lives, which
//! is what makes memoized filter resolutions, param-word memos and parked
//! COLT views sound across executions. Nothing here persists; dropping the
//! cache drops every token (a deleted row leaves no immortal dictionary
//! entry anywhere).
//!
//! Cost honesty: interning charges the operation's work ledger per byte
//! (`WorkContext::step`), and the retained bytes are reported through
//! [`TextInterner::retained_bytes`] so cache owners can trim under
//! pressure (dropping the whole cache is the trim unit — token stability
//! is the invariant, partial eviction is not offered).

use std::collections::HashMap;

use crate::work::{WorkContext, WorkError};

/// The never-minted miss token (the old dictionary's sentinel, retained):
/// a text absent from every image matches nothing under `Eq` and
/// everything under `Ne`, exactly like any other unequal word.
pub(crate) const SENTINEL_WORD: u64 = u64::MAX;

/// One append-only exact text→token map plus its reverse table.
#[derive(Debug, Default)]
pub(crate) struct TextInterner {
    map: HashMap<Box<str>, u64>,
    texts: Vec<Box<str>>,
    bytes: usize,
}

impl TextInterner {
    /// The token of `text`, minting one if absent. Never returns
    /// [`SENTINEL_WORD`].
    /// # Errors
    /// Stopped work or allocation refusal.
    pub(crate) fn intern(&mut self, text: &str, work: &WorkContext) -> Result<u64, InternError> {
        work.step(1 + text.len() as u64)?;
        if let Some(token) = self.map.get(text) {
            return Ok(*token);
        }
        let token = u64::try_from(self.texts.len()).expect("token count fits u64");
        debug_assert_ne!(token, SENTINEL_WORD, "the sentinel is never minted");
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| InternError::Allocation)?;
        owned.push_str(text);
        let owned: Box<str> = owned.into_boxed_str();
        self.map
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        self.texts
            .try_reserve(1)
            .map_err(|_| InternError::Allocation)?;
        // Two owners of the bytes (map key + reverse table) is the retained
        // cost; count both so budget reports do not undercount by half.
        self.bytes += 2 * text.len() + std::mem::size_of::<Box<str>>() * 2;
        self.map.insert(owned.clone(), token);
        self.texts.push(owned);
        Ok(token)
    }

    /// The token of `text` if it was ever interned — used by scan-side
    /// comparisons where a miss means "equal to no interned word", never
    /// an allocation.
    #[must_use]
    pub(crate) fn lookup(&self, text: &str) -> Option<u64> {
        self.map.get(text).copied()
    }

    /// As [`Self::lookup`], returning the sentinel on a miss (the word
    /// probes and `Ne` comparisons want a word, not an Option).
    #[must_use]
    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.lookup(text).unwrap_or(SENTINEL_WORD)
    }

    /// The text of a minted token. `None` for the sentinel or a token this
    /// interner never minted (a corruption-grade condition at the caller).
    #[must_use]
    pub(crate) fn text_of(&self, token: u64) -> Option<&str> {
        usize::try_from(token)
            .ok()
            .and_then(|idx| self.texts.get(idx))
            .map(AsRef::as_ref)
    }

    /// Retained text bytes (both owners) plus table headroom — the cache
    /// owner's budgeting figure, not an allocator measurement.
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

/// A borrow-bundled interner + work ledger: the shape resolve/bind call
/// sites thread through the execution without carrying two parameters.
pub(crate) struct InternerHandle<'a> {
    interner: &'a std::sync::Mutex<TextInterner>,
    work: &'a WorkContext,
}

impl<'a> InternerHandle<'a> {
    pub(crate) fn new(interner: &'a std::sync::Mutex<TextInterner>, work: &'a WorkContext) -> Self {
        Self { interner, work }
    }

    /// Latch one text literal/param to its token (minting if new — the
    /// interner is append-only, so a latched token is final).
    /// # Errors
    /// Stopped work or allocation refusal.
    pub(crate) fn intern_text(&self, text: &str) -> crate::error::Result<u64> {
        self.interner
            .lock()
            .expect("interner mutex")
            .intern(text, self.work)
            .map_err(crate::error::Error::from)
    }

    /// Latch a template literal carried as raw bytes (IR `Value::String`
    /// payloads — UTF-8 by construction of the owned value).
    /// # Errors
    /// As [`Self::intern_text`].
    pub(crate) fn latch(&self, bytes: &[u8]) -> crate::error::Result<u64> {
        let text = std::str::from_utf8(bytes)
            .expect("IR string literals are UTF-8 by construction (Value::String)");
        self.intern_text(text)
    }

    /// Read one minted token's text under the lock.
    pub(crate) fn with_text<R>(&self, token: u64, read: impl FnOnce(&str) -> R) -> Option<R> {
        let interner = self.interner.lock().expect("interner mutex");
        interner.text_of(token).map(read)
    }

    /// The token of `text` or the sentinel, under the lock — the scan-side
    /// comparison word (a miss equals no interned word).
    pub(crate) fn lookup_word(&self, text: &str) -> u64 {
        self.interner
            .lock()
            .expect("interner mutex")
            .lookup_word(text)
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

    fn work() -> WorkContext {
        crate::api::prepared::source::unbounded_work().expect("unbounded ledger")
    }

    #[test]
    fn tokens_are_dense_stable_and_exact() {
        let work = work();
        let mut interner = TextInterner::default();
        let a = interner.intern("alpha", &work).expect("intern");
        let b = interner.intern("beta", &work).expect("intern");
        assert_ne!(a, b, "distinct texts, distinct tokens");
        assert_eq!(interner.intern("alpha", &work).expect("intern"), a);
        assert_eq!(interner.lookup("alpha"), Some(a));
        assert_eq!(interner.lookup("gamma"), None);
        assert_eq!(interner.lookup_word("gamma"), SENTINEL_WORD);
        assert_eq!(interner.text_of(a), Some("alpha"));
        assert_eq!(interner.text_of(b), Some("beta"));
        assert_eq!(interner.text_of(SENTINEL_WORD), None);
        assert_eq!(interner.len(), 2);
        assert!(interner.retained_bytes() >= 2 * ("alpha".len() + "beta".len()));
    }

    #[test]
    fn equal_bytes_decide_identity_never_a_hash() {
        // Two texts engineered to collide under any fixed 64-bit fold would
        // still take distinct tokens: the map is keyed by the bytes.
        let work = work();
        let mut interner = TextInterner::default();
        let long_a = "x".repeat(600); // beyond any physical key bound
        let long_b = format!("{}y", "x".repeat(599));
        let a = interner.intern(&long_a, &work).expect("intern");
        let b = interner.intern(&long_b, &work).expect("intern");
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
        let mut interner = TextInterner::default();
        let result = interner.intern("far-too-long-for-eight-units", &context);
        assert!(
            matches!(result, Err(InternError::Work(_))),
            "byte-proportional charge stops before growth"
        );
    }
}
