use super::{EitherSink, FindSpec, ResolveMemo, ResultBuffer, ValueType};

use crate::error::Result;
use crate::storage::env::ReadTxn;

/// Drains the sink into the result buffer, decoding words by result type
/// (each distinct intern resolved once, docs/architecture/30-execution.md).
pub(super) fn finalize(
    sink: &EitherSink,
    row_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    txn: &ReadTxn<'_>,
    finds: &[(FindSpec, ValueType)],
    all_words: bool,
    out: &mut ResultBuffer,
) -> Result<()> {
    memo.clear();
    // The all-words fast path (docs/perf/ PRD 08): one reservation, then
    // infallible cell writes — no Result, no dictionary plumbing per
    // cell. Interned finds keep the resolving path (the per-cell memo
    // probe is the resolution semantics, softened by the run memo).
    match sink {
        EitherSink::Projection(sink) => {
            out.cells.reserve(sink.len() * finds.len());
            if all_words {
                for row in sink.rows() {
                    for (column, (_, ty)) in finds.iter().enumerate() {
                        out.cells.push(ResultBuffer::word_cell(ty, row[column]));
                    }
                }
                return Ok(());
            }
            for row in sink.rows() {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column], memo)?;
                }
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * finds.len());
            if all_words {
                return sink.finalize_into(row_scratch, |row| {
                    for (column, (_, ty)) in finds.iter().enumerate() {
                        out.cells.push(ResultBuffer::word_cell(ty, row[column]));
                    }
                    Ok(())
                });
            }
            sink.finalize_into(row_scratch, |row| {
                for (column, (_, ty)) in finds.iter().enumerate() {
                    out.push_word(txn, ty, row[column], memo)?;
                }
                Ok(())
            })
        }
    }
}
