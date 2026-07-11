use super::{EitherSink, ResolveMemo, ResultBuffer, ValueType};

use crate::error::Result;
use crate::storage::env::ReadTxn;

/// Drains the sink into the result buffer, decoding words by result type
/// (each distinct intern resolved once, docs/architecture/40-execution.md).
/// The aggregate sink finalizes mutably (`Pack`'s claim lists sort in
/// place); the buffer reservation is a hint — Pack emits one row per
/// (group, maximal segment), so groups is a floor there, not the count.
///
/// Sink rows are **word rows** (the `SlotWidth` layout): each find
/// contributes its width — an interval find spans two words that
/// materialize as ONE interval cell — so both loops walk a word cursor
/// per find, never a bare column index.
pub(super) fn finalize(
    sink: &mut EitherSink,
    row_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    txn: &ReadTxn<'_>,
    types: &[ValueType],
    all_words: bool,
    out: &mut ResultBuffer,
) -> Result<()> {
    memo.clear();
    // The all-words fast path: one reservation, then
    // infallible cell writes — no Result, no dictionary plumbing per
    // cell (intervals are word-backed and stay on it). Interned finds
    // keep the resolving path (the per-cell memo probe is the resolution
    // semantics, softened by the run memo).
    match sink {
        EitherSink::Projection(sink) => {
            out.cells.reserve(sink.len() * types.len());
            if all_words {
                for row in sink.rows() {
                    push_word_row(out, types, row);
                }
                return Ok(());
            }
            for row in sink.rows() {
                push_resolved_row(out, txn, memo, types, row)?;
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * types.len());
            if all_words {
                return sink.finalize_into(row_scratch, |row| {
                    push_word_row(out, types, row);
                    Ok(())
                });
            }
            sink.finalize_into(row_scratch, |row| {
                push_resolved_row(out, txn, memo, types, row)
            })
        }
    }
}

/// One word row's cells, all-words regime: infallible, no dictionary.
fn push_word_row(out: &mut ResultBuffer, types: &[ValueType], row: &[u64]) {
    let mut word = 0;
    for ty in types {
        if let ValueType::Interval { element } = ty {
            out.cells.push(ResultBuffer::interval_cell(
                *element,
                row[word],
                row[word + 1],
            ));
            word += 2;
        } else {
            out.cells.push(ResultBuffer::word_cell(ty, row[word]));
            word += 1;
        }
    }
}

/// One word row's cells, resolving regime: String/Bytes go through the
/// intern memo; everything else decodes inline.
fn push_resolved_row(
    out: &mut ResultBuffer,
    txn: &ReadTxn<'_>,
    memo: &mut ResolveMemo,
    types: &[ValueType],
    row: &[u64],
) -> Result<()> {
    let mut word = 0;
    for ty in types {
        if let ValueType::Interval { element } = ty {
            out.cells.push(ResultBuffer::interval_cell(
                *element,
                row[word],
                row[word + 1],
            ));
            word += 2;
        } else {
            out.push_word(txn, ty, row[word], memo)?;
            word += 1;
        }
    }
    Ok(())
}
