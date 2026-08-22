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
    /// cancels byte-exactly), a committed hit returns the committed id —
    /// answered by the transaction's memo when the string was already
    /// witnessed, else by one dictionary probe whose hit is memoized —
    /// and a double miss proves the fact absent from base *and* delta —
    /// its bytes would embed an id that was never minted — so the delete
    /// is a no-op and the dictionary stays untouched. (Additional
    /// reader: the commit path's selection-literal encoding,
    /// `storage::commit::judgment`.)
    pub(crate) fn resolve(&self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<Option<InternId>> {
        if let Some(id) = self.interns.as_ref().and_then(|interns| interns.get(raw)) {
            return Ok(Some(id));
        }
        // Pending missed, so the answer is committed-side — frozen for
        // the transaction under the single-writer mutex, so askable
        // through the memo. This blake3 is the occurrence's ONLY hash:
        // the memo key and the dictionary's forward key are the same 32
        // bytes, and a memo miss hands it straight to the LMDB get
        // (`dict::lookup_by_hash`) instead of re-hashing in
        // `forward_key`.
        let hash = *blake3::hash(raw).as_bytes();
        if let Some(id) = self.committed_memo.get(&hash) {
            return Ok(Some(id));
        }
        let found = crate::storage::dict::lookup_by_hash(view, &hash)?;
        // Hits only. Memoizing a MISS would be equally sound — committed
        // state is frozen, and a later mint of these bytes lands in the
        // pending map, which is probed FIRST — but the intern lane mints
        // on its first miss, so the pending map already memoizes it;
        // what remains is the cold delete/judgment probing of
        // never-interned strings, not worth a second entry kind.
        if let Some(id) = found {
            self.committed_memo.record(hash, id);
        }
        Ok(found)
    }

    fn intern(&mut self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<InternId> {
        // Pending first, then the committed memo, then one dict probe
        // (memoized on hit): a pending value was proven absent from the
        // committed dict at mint time, and the single-writer discipline
        // freezes the committed dict for the transaction's lifetime — so
        // a repeat intern of a pending string costs one in-memory probe
        // and of a committed string one blake3 plus one in-memory probe,
        // never a second LMDB get.
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
