//! Canonical-row text decoding and borrowed resolver access.
use crate::error::{Error, Result};
use crate::image::canon::{RowWords, TextWords};
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, FilterPredicate, Operands};
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

pub(super) fn words_equal(interner: &InternerHandle<'_>, left: u64, right: u64) -> Result<bool> {
    interner.text_eq().tokens_equal(left, right)
}

pub(super) fn holds_with_text<O: Operands>(
    predicate: &FilterPredicate,
    ops: &O,
    params: &[Const],
    interner: &InternerHandle<'_>,
) -> Result<Option<bool>>
where
    Error: From<O::Error>,
{
    crate::image::view::holds(predicate, ops, params, interner.text_eq())
}
