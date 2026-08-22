use crate::error::{Error, Result};
use crate::storage::env::ReadTxn;
use crate::storage::keys;
use bumbledb_theory::schema::{FieldId, RelationId};

use super::{FreshMark, WriteDelta};

impl WriteDelta<'_> {

    /// `EscapedIdBurn` drop guard for the closure region, which covers

    /// # Errors

    /// (the dyn boundary's foreign-witness refusal — see

    pub fn reserve(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        field: FieldId,
        count: std::num::NonZeroU64,
    ) -> Result<u64> {
        let count = count.get();
        let mark = self.fresh_mark(view, rel, field)?;
        let next = mark.next;
        let end = next.checked_add(count).ok_or(Error::FreshExhausted {
            relation: rel,
            field,
        })?;
        mark.next = end;
        Ok(next)
    }

    /// The lazy init is the dyn boundary's foreign-witness refusal

    /// typed before any `Q` key is touched — priced once per

    pub(super) fn fresh_mark(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        field: FieldId,
    ) -> Result<&mut FreshMark> {
        match self.marks.entry((rel, field)) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.schema.check_fresh_field(rel, field)?;
                let base = read_fresh_next(view, rel, field)?;
                Ok(entry.insert(FreshMark { base, next: base }))
            }
        }
    }
}

pub(crate) fn read_fresh_next(view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
    let buf = keys::fresh_key(rel, field);
    let disk = match view.env().data().get(view.raw(), &buf)? {
        Some(bytes) => crate::storage::stored_u64(bytes, "Q fresh next")?,
        None => 0,
    };
    Ok(disk.max(view.env().in_process_fresh_next(rel, field)))
}
