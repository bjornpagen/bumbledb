use super::{Answers, Cell, EitherSink, ResolveMemo, ValueType};

use crate::error::Result;
use crate::exec::sink::ProjectionSink;
use crate::ir::validate::PredicateColumn;
use crate::storage::env::ReadTxn;

/// Drains the sink into the result buffer, decoding words by result type
/// (each distinct intern resolved once, docs/architecture/40-execution.md).
/// The aggregate sink finalizes mutably (`Pack`'s claim lists sort in
/// place); the answer reservation is a hint — Pack emits one answer per
/// (group, maximal segment), so groups is a floor there, not the count.
///
/// Sink answers are **word tuples** (the `SlotWidth` layout): each find
/// contributes its width — an interval find spans two words that
/// materialize as ONE interval cell — so every fill walks a word cursor
/// per find, never a bare column index.
///
/// The projection drain fills **column-major**: the type dispatch runs
/// once per column per finalize, not per cell (the `PredicateColumn`
/// roster is sealed at validation — the per-column writer is the match
/// arm, and each arm's row loop is monomorphic). Cells stay row-major
/// (`answer * arity + column`) — only the fill order is columnar, so
/// each column writes strided cell slots. The aggregate drain streams
/// rows through `finalize_into` (Pack sorts in place; groups are few)
/// and keeps the row-major path with ONE dispatch per cell.
///
/// GRAVESTONE (cleanup-0.5.0 ruling 7, the Measure phase, 2026-07-19,
/// `bench-out/measure-twins/`): an all-words fast path — a second
/// `AnswerHeap::Words` route that skipped the memo/byte-heap plumbing
/// when no column was string/bytes — was measure-or-merge twinned and
/// REFUTED: resolved/words 0.996–1.005 on both sinks (projection
/// 20,000-answer fill, aggregate 997-group drain; warm DRAM,
/// interleaved min-of-7) against the 1.09 pre-stated bar. The word
/// columns' arms below ARE the word path — the dispatch is per column
/// (projection) or per cell (aggregate) either way, so the duplicate
/// route bought nothing. The `AnswerHeap` seal died with it. Reverses
/// if: a profiled finalize shows the String/FixedBytes match arms'
/// mere presence taxing an all-words fill ≥ the house bar — re-twin
/// before believing it.
pub(super) fn finalize(
    sink: &mut EitherSink,
    answer_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    txn: &ReadTxn<'_>,
    columns: &[PredicateColumn],
    out: &mut Answers,
) -> Result<()> {
    memo.clear();
    match sink {
        EitherSink::Projection(sink) => {
            let base = out.cells.len();
            let result = fill_resolved_answers(out, txn, memo, columns, sink);
            if result.is_err() {
                // The columnar fill pre-sizes its rows: drop the
                // placeholder cells so no half-written row survives an
                // error (the byte heap keeps orphan bytes, harmlessly —
                // nothing references past a written cell's range).
                out.cells.truncate(base);
            }
            result
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * columns.len());
            sink.finalize_into(answer_scratch, |answer| {
                push_resolved_answer(out, txn, memo, columns, answer)
            })
        }
    }
}

/// The resolving columnar fill: String goes through the intern memo
/// per cell (that probe IS the resolution semantics — and the columnar
/// order maximizes the run memo's coherence), a `bytes<N>` column
/// re-assembles its padded slot words per answer (inline — no
/// dictionary); everything else fills fixed-width. Byte-heap columns
/// index their strided slot instead of holding the cells borrow — the
/// heap append and the memo both need `out` whole.
fn fill_resolved_answers(
    out: &mut Answers,
    txn: &ReadTxn<'_>,
    memo: &mut ResolveMemo,
    columns: &[PredicateColumn],
    sink: &ProjectionSink,
) -> Result<()> {
    let arity = columns.len();
    let base = out.cells.len();
    out.cells.resize(base + sink.len() * arity, Cell::U64(0));
    let mut word = 0;
    for (col, column) in columns.iter().enumerate() {
        word += match &column.ty {
            ValueType::String => {
                for (row, answer) in sink.answers().enumerate() {
                    let (start, len) = memo.resolve(txn, answer[word], out)?;
                    out.cells[base + row * arity + col] = Cell::String { start, len };
                }
                1
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                for (row, answer) in sink.answers().enumerate() {
                    let cell = out.fixed_bytes_cell(*len, &answer[word..word + width]);
                    out.cells[base + row * arity + col] = cell;
                }
                width
            }
            ty => fill_fixed_column(&mut out.cells[base..], arity, col, ty, word, sink),
        };
    }
    Ok(())
}

/// One word-backed column's strided fill — the hoisted dispatch: ONE
/// match, then a monomorphic row loop per arm. Returns the column's
/// word width.
fn fill_fixed_column(
    cells: &mut [Cell],
    arity: usize,
    col: usize,
    ty: &ValueType,
    word: usize,
    sink: &ProjectionSink,
) -> usize {
    let rows = cells.chunks_exact_mut(arity).zip(sink.answers());
    match ty {
        ValueType::Bool => {
            for (slots, answer) in rows {
                slots[col] = Cell::Bool(answer[word] != 0);
            }
        }
        ValueType::U64 => {
            for (slots, answer) in rows {
                slots[col] = Cell::U64(answer[word]);
            }
        }
        ValueType::I64 => {
            for (slots, answer) in rows {
                slots[col] = Cell::I64((answer[word] ^ (1 << 63)).cast_signed());
            }
        }
        ValueType::Interval { element, .. } => {
            for (slots, answer) in rows {
                slots[col] = Answers::interval_cell(*element, answer[word], answer[word + 1]);
            }
            return 2;
        }
        ValueType::String => {
            unreachable!("string columns resolve through the memo (fill_resolved_answers)")
        }
        ValueType::FixedBytes { .. } => {
            unreachable!("bytes<N> columns fill through the byte heap (fill_resolved_answers)")
        }
    }
    1
}

/// One word answer's cells: String goes through the
/// intern memo, a `bytes<N>` find re-assembles its padded slot words
/// (inline — no dictionary); everything else decodes inline. ONE
/// dispatch per cell.
fn push_resolved_answer(
    out: &mut Answers,
    txn: &ReadTxn<'_>,
    memo: &mut ResolveMemo,
    columns: &[PredicateColumn],
    answer: &[u64],
) -> Result<()> {
    let mut word = 0;
    for column in columns {
        let (cell, width) = match &column.ty {
            ValueType::Bool => (Cell::Bool(answer[word] != 0), 1),
            ValueType::U64 => (Cell::U64(answer[word]), 1),
            ValueType::I64 => (Cell::I64((answer[word] ^ (1 << 63)).cast_signed()), 1),
            ValueType::Interval { element, .. } => (
                Answers::interval_cell(*element, answer[word], answer[word + 1]),
                2,
            ),
            ValueType::String => {
                let (start, len) = memo.resolve(txn, answer[word], out)?;
                (Cell::String { start, len }, 1)
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                (
                    out.fixed_bytes_cell(*len, &answer[word..word + width]),
                    width,
                )
            }
        };
        out.cells.push(cell);
        word += width;
    }
    Ok(())
}
