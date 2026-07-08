use crate::error::Result;
use crate::storage::env::ReadTxn;

use super::WriteDelta;

impl WriteDelta<'_> {
    /// Interns a UTF-8 string for use in this transaction's facts: returns
    /// the committed id if present, else mints a provisional id flushed at
    /// commit. The `&str` boundary is the UTF-8 validation.
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary or counter read.
    pub fn intern_str(&mut self, view: &ReadTxn<'_>, value: &str) -> Result<u64> {
        self.intern(view, crate::storage::dict::TAG_STRING, value.as_bytes())
    }

    /// Interns a byte sequence; see [`Self::intern_str`].
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary or counter read.
    pub fn intern_bytes(&mut self, view: &ReadTxn<'_>, value: &[u8]) -> Result<u64> {
        self.intern(view, crate::storage::dict::TAG_BYTES, value)
    }

    /// Delete-side intern resolution for a UTF-8 string: never mints.
    /// `Ok(None)` proves the fact cannot exist — see [`Self::resolve`].
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary read.
    pub fn resolve_str(&self, view: &ReadTxn<'_>, value: &str) -> Result<Option<u64>> {
        self.resolve(view, crate::storage::dict::TAG_STRING, value.as_bytes())
    }

    /// Delete-side intern resolution for bytes; see [`Self::resolve_str`].
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary read.
    pub fn resolve_bytes(&self, view: &ReadTxn<'_>, value: &[u8]) -> Result<Option<u64>> {
        self.resolve(view, crate::storage::dict::TAG_BYTES, value)
    }

    /// Reverse lookup of a provisional intern id minted this transaction —
    /// the decode side of the point reads (a fact inserted this
    /// transaction carries pending ids the committed dictionary cannot
    /// resolve). A linear scan: the pending map is value-keyed for the
    /// hot forward probes, and a transaction's novel-value set is small.
    #[must_use]
    pub fn pending_raw(&self, tag: u8, id: u64) -> Option<&[u8]> {
        self.pending_interns[usize::from(tag)]
            .iter()
            .find_map(|(raw, &candidate)| (candidate == id).then_some(raw.as_ref()))
    }

    /// The non-minting sibling of [`Self::intern`], for the delete path:
    /// a pending-map hit returns the provisional id (insert-then-delete
    /// cancels byte-exactly), a committed-dict hit returns the committed
    /// id, and a double miss proves the fact absent from base *and*
    /// delta — its bytes would embed an id that was never minted — so
    /// the delete is a no-op and the dictionary stays untouched.
    fn resolve(&self, view: &ReadTxn<'_>, tag: u8, raw: &[u8]) -> Result<Option<u64>> {
        if let Some(id) = self.pending_interns[usize::from(tag)].get(raw) {
            return Ok(Some(*id));
        }
        crate::storage::dict::lookup_tagged(view, tag, raw)
    }

    fn intern(&mut self, view: &ReadTxn<'_>, tag: u8, raw: &[u8]) -> Result<u64> {
        // Pending first: a pending value was proven absent from the
        // committed dict at mint time, and the single-writer discipline
        // freezes the committed dict for the transaction's lifetime — so
        // a repeat intern costs one in-memory probe, not an LMDB get plus
        // a blake3.
        if let Some(id) = self.resolve(view, tag, raw)? {
            return Ok(id);
        }
        let next = match self.dict_next {
            Some(next) => next,
            // A corrupted stored counter (u64::MAX) is typed Corruption
            // inside this read; the assert below can therefore fire only
            // for genuine in-memory exhaustion — 2^64 mints in one
            // transaction — which is a documented panic, not data.
            None => view.dict_next_id()?,
        };
        assert!(
            next != crate::storage::dict::SENTINEL_ID,
            "dictionary id space exhausted (u64::MAX is the miss sentinel)"
        );
        self.pending_interns[usize::from(tag)].insert(Box::from(raw), next);
        self.dict_next = Some(next + 1);
        Ok(next)
    }
}
