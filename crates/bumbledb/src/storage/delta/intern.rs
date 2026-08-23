use crate::encoding::InternId;
use crate::error::Result;
use crate::storage::env::ReadTxn;

use super::WriteDelta;

impl WriteDelta<'_> {
    /// # Errors
    pub fn intern_str(&mut self, view: &ReadTxn<'_>, value: &str) -> Result<InternId> {
        self.intern(view, value.as_bytes())
    }

    /// # Errors
    pub fn resolve_str(&self, view: &ReadTxn<'_>, value: &str) -> Result<Option<InternId>> {
        self.resolve(view, value.as_bytes())
    }

    #[must_use]
    pub fn pending_raw(&self, id: InternId) -> Option<&[u8]> {
        self.interns
            .as_ref()
            .and_then(|interns| interns.pending_raw(id))
    }

    pub(crate) fn resolve(&self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<Option<InternId>> {
        if let Some(id) = self.interns.as_ref().and_then(|interns| interns.get(raw)) {
            return Ok(Some(id));
        }

        // bytes, and a memo miss hands it straight to the LMDB get

        let hash = *blake3::hash(raw).as_bytes();
        if let Some(id) = self.committed_memo.get(&hash) {
            return Ok(Some(id));
        }
        let found = crate::storage::dict::lookup_by_hash(view, &hash)?;

        if let Some(id) = found {
            self.committed_memo.record(hash, id);
        }
        Ok(found)
    }

    /// The mint law: pending intern ids assign in first-use order — a
    /// novel string takes the running next-id at its first sight and
    /// every repeat returns that same id — so identical intern sequences
    /// against identical committed dictionaries mint identical ids on
    /// every store. Replication replays a batch's ops in their recorded
    /// order, which pins the first-use sequence and therefore the ids
    /// (pinned:
    /// `identical_batches_against_identical_stores_mint_identical_intern_ids`).
    fn intern(&mut self, view: &ReadTxn<'_>, raw: &[u8]) -> Result<InternId> {
        // never a second LMDB get.
        if let Some(id) = self.resolve(view, raw)? {
            return Ok(id);
        }
        let next = match &self.interns {
            Some(interns) => interns.next_id,

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
