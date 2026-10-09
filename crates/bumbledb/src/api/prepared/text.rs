//! Canonical-row text decoding and borrowed resolver access.
use crate::error::Result;
use crate::image::canon::{RowWords, TextWords};
use crate::image::intern::InternerHandle;
use crate::work::WorkContext;
use bumbledb_theory::schema::FieldDescriptor;

/// Decode once, keeping row-local text owners until this row is reused.
pub(crate) fn decode_row(
    row: &mut RowWords,
    fields: &[FieldDescriptor],
    bytes: &[u8],
    interner: &InternerHandle<'_>,
    work: &WorkContext,
    intern: bool,
) -> Result<()> {
    work.checkpoint().map_err(super::source::work_error)?;
    let mut text = if intern {
        TextWords::HandleIntern(interner)
    } else {
        TextWords::HandleLookup(interner)
    };
    row.decode(fields, bytes, &mut text)
}

pub(super) fn resolve_text(
    interner: &InternerHandle<'_>,
    token: u64,
    write: impl FnOnce(&str),
) -> Option<usize> {
    interner.with_text(token, |text| {
        write(text);
        text.len()
    })
}

pub(crate) fn owned_text(interner: &InternerHandle<'_>, token: u64) -> Option<Box<str>> {
    interner.with_text(token, |text| Box::<str>::from(text))
}
