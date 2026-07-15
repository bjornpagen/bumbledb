use super::{AnswerHeap, Answers, EitherSink, ResolveMemo, ValueType};

use crate::error::Result;
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
/// materialize as ONE interval cell — so both loops walk a word cursor
/// per find, never a bare column index.
pub(super) fn finalize(
    sink: &mut EitherSink,
    answer_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    txn: &ReadTxn<'_>,
    columns: &[PredicateColumn],
    answer_heap: AnswerHeap,
    out: &mut Answers,
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
            if answer_heap == AnswerHeap::Words {
                for answer in sink.answers() {
                    push_word_answer(out, columns, answer);
                }
                return Ok(());
            }
            for answer in sink.answers() {
                push_resolved_answer(out, txn, memo, columns, answer)?;
            }
            Ok(())
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * columns.len());
            if answer_heap == AnswerHeap::Words {
                return sink.finalize_into(answer_scratch, |answer| {
                    push_word_answer(out, columns, answer);
                    Ok(())
                });
            }
            sink.finalize_into(answer_scratch, |answer| {
                push_resolved_answer(out, txn, memo, columns, answer)
            })
        }
    }
}

/// One word answer's cells, all-words regime: infallible, no dictionary.
fn push_word_answer(out: &mut Answers, columns: &[PredicateColumn], answer: &[u64]) {
    let mut word = 0;
    for column in columns {
        if let ValueType::Interval { element, .. } = &column.ty {
            out.cells.push(Answers::interval_cell(
                *element,
                answer[word],
                answer[word + 1],
            ));
            word += 2;
        } else {
            out.cells.push(Answers::word_cell(&column.ty, answer[word]));
            word += 1;
        }
    }
}

/// One word answer's cells, resolving regime: String goes through the
/// intern memo, a `bytes<N>` find re-assembles its padded slot words
/// (inline — no dictionary); everything else decodes inline.
fn push_resolved_answer(
    out: &mut Answers,
    txn: &ReadTxn<'_>,
    memo: &mut ResolveMemo,
    columns: &[PredicateColumn],
    answer: &[u64],
) -> Result<()> {
    let mut word = 0;
    for column in columns {
        match &column.ty {
            ValueType::Interval { element, .. } => {
                out.cells.push(Answers::interval_cell(
                    *element,
                    answer[word],
                    answer[word + 1],
                ));
                word += 2;
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                out.push_fixed_bytes(*len, &answer[word..word + width]);
                word += width;
            }
            ty => {
                out.push_word(txn, ty, answer[word], memo)?;
                word += 1;
            }
        }
    }
    Ok(())
}
