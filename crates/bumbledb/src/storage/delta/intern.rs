use crate::encoding::InternId;
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
    pub fn intern_str(&mut self, view: &ReadTxn<'_>, value: &str) -> Result<InternId> {
        self.intern(view, value.as_bytes())
    }

    /// Delete-side intern resolution for a UTF-8 string: never mints.
    /// `Ok(None)` proves the fact cannot exist — see [`Self::resolve`].
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary read.
    pub fn resolve_str(&self, view: &ReadTxn<'_>, value: &str) -> Result<Option<InternId>> {
        self.resolve(view, value.as_bytes())
    }

    /// Reverse lookup of a provisional intern id minted this transaction —
    /// the decode side of the point reads (a fact inserted this
    /// transaction carries pending ids the committed dictionary cannot
    /// resolve). A linear scan: the pending map is value-keyed for the
    /// hot forward probes, and a transaction's novel-value set is small.
    #[must_use]
    pub fn pending_raw(&self, id: InternId) -> Option<&[u8]> {
        self.interns
            .as_ref()
            .and_then(|interns| interns.pending_raw(id))
    }

    /// The non-minting sibling of [`Self::intern`], for the delete path:
    /// a pending-map hit returns the provisional id (insert-then-delete
    /// cancels byte-exactly), a committed-dict hit returns the committed
    /// id, and a double miss proves the fact absent from base *and*
    /// delta — its bytes would embed an id that was never minted — so
    /// the delete is a no-op and the dictionary stays untouched.
    /// (Additional reader: the commit path's selection-literal encoding,
    /// `storage::commit::judgment`.)
    pub(crate) fn resolve(&self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<Option<InternId>> {
        if let Some(id) = self.interns.as_ref().and_then(|interns| interns.get(raw)) {
            return Ok(Some(id));
        }
        crate::storage::dict::lookup(view, raw)
    }

    fn intern(&mut self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<InternId> {
        // Pending first: a pending value was proven absent from the
        // committed dict at mint time, and the single-writer discipline
        // freezes the committed dict for the transaction's lifetime — so
        // a repeat intern costs one in-memory probe, not an LMDB get plus
        // a blake3.
        if let Some(id) = self.resolve(view, raw)? {
            return Ok(id);
        }
        let next = match &self.interns {
            Some(interns) => interns.next_id,
            // A corrupted stored counter (sentinel) is typed Corruption
            // inside this read; the assert below can therefore fire only
            // for genuine in-memory exhaustion — 2^64 mints in one
            // transaction — which is a documented panic, not data.
            None => view.dict_next_id()?,
        };
        let id = InternId::from_raw(next);
        assert!(
            !id.is_sentinel(),
            "dictionary id space exhausted (u64::MAX is the miss sentinel)"
        );
        let next_id = next + 1;
        match &mut self.interns {
            Some(interns) => interns.insert(raw, id, next_id),
            None => self.interns = Some(super::PendingInterns::first(raw, id, next_id)),
        }
        Ok(id)
    }
}
