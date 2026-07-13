use super::{EitherSink, ResolveMemo, ResultBuffer, ValueType};

use crate::error::Result;
use crate::ir::validate::PredicateColumn;
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
    columns: &[PredicateColumn],
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
            out.cells.reserve(sink.len() * columns.len());
            if all_words {
                for row in sink.rows() {
                    push_word_row(out, columns, row);
                }
                return Ok(());
            }
            for row in sink.rows() {
                push_resolved_row(out, txn, memo, columns, row)?;
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * columns.len());
            if all_words {
                return sink.finalize_into(row_scratch, |row| {
                    push_word_row(out, columns, row);
                    Ok(())
                });
            }
            sink.finalize_into(row_scratch, |row| {
                push_resolved_row(out, txn, memo, columns, row)
            })
        }
    }
}

/// One word row's cells, all-words regime: infallible, no dictionary.
fn push_word_row(out: &mut ResultBuffer, columns: &[PredicateColumn], row: &[u64]) {
    let mut word = 0;
    for column in columns {
        if let ValueType::Interval { element } = &column.ty {
            out.cells.push(ResultBuffer::interval_cell(
                *element,
                row[word],
                row[word + 1],
            ));
            word += 2;
        } else {
            out.cells
                .push(ResultBuffer::word_cell(&column.ty, row[word]));
            word += 1;
        }
    }
}

/// One word row's cells, resolving regime: String goes through the
/// intern memo, a `bytes<N>` find re-assembles its padded slot words
/// (inline — no dictionary); everything else decodes inline.
fn push_resolved_row(
    out: &mut ResultBuffer,
    txn: &ReadTxn<'_>,
    memo: &mut ResolveMemo,
    columns: &[PredicateColumn],
    row: &[u64],
) -> Result<()> {
    let mut word = 0;
    for column in columns {
        match &column.ty {
            ValueType::Interval { element } => {
                out.cells.push(ResultBuffer::interval_cell(
                    *element,
                    row[word],
                    row[word + 1],
                ));
                word += 2;
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                out.push_fixed_bytes(*len, &row[word..word + width]);
                word += width;
            }
            ty => {
                out.push_word(txn, ty, row[word], memo)?;
                word += 1;
            }
        }
    }
    Ok(())
}
