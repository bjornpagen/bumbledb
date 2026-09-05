use super::{Answers, Cell, EitherSink, ResolveMemo, ValueType};

use crate::error::Result;
use crate::exec::sink::ProjectionSink;
use crate::ir::validate::SignatureColumn;
use crate::storage::catalog::CatalogRead;

/// Reverses if: a profiled finalize shows the String/FixedBytes match arms'
/// mere presence taxing an all-words fill ≥ the house bar — re-twin before
/// believing it.
pub(super) fn finalize<C: CatalogRead>(
    sink: &mut EitherSink,
    answer_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    catalog: &C,
    columns: &[SignatureColumn],
    out: &mut Answers,
) -> Result<()> {
    memo.clear();
    match sink {
        EitherSink::Projection(sink) => {
            let base = out.cells.len();
            let result = fill_resolved_answers(out, catalog, memo, columns, sink);
            if result.is_err() {
                // The columnar fill pre-sizes its rows: drop the

                out.cells.truncate(base);
            }
            result
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(sink.group_count() * columns.len());
            sink.finalize_into(answer_scratch, |answer| {
                push_resolved_answer(out, catalog, memo, columns, answer)
            })
        }
    }
}

fn fill_resolved_answers<C: CatalogRead>(
    out: &mut Answers,
    catalog: &C,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    sink: &ProjectionSink,
) -> Result<()> {
    let arity = columns.len();
    let base = out.cells.len();
    out.cells.resize(base + sink.len() * arity, Cell::U64(0));
    let mut word = 0;
    for (col, column) in columns.iter().enumerate() {
        word += match column.ty() {
            ValueType::String => {
                for (row, answer) in sink.answers().enumerate() {
                    let (start, len) = memo.resolve(catalog, answer[word], out)?;
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
            ty => fill_fixed_column(&mut out.cells[base..], arity, col, ty, word, sink)?,
        };
    }
    Ok(())
}

fn fill_fixed_column(
    cells: &mut [Cell],
    arity: usize,
    col: usize,
    ty: &ValueType,
    word: usize,
    sink: &ProjectionSink,
) -> Result<usize> {
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
        ValueType::F64 => {
            for (slots, answer) in rows {
                slots[col] = Cell::F64(crate::encoding::decode_f64(answer[word].to_be_bytes())?);
            }
        }
        ValueType::Interval { element, .. } | ValueType::FixedInterval { element, .. } => {
            for (slots, answer) in rows {
                slots[col] = Answers::interval_cell(*element, answer[word], answer[word + 1]);
            }
            return Ok(2);
        }
        ValueType::String => {
            unreachable!("string columns resolve through the memo (fill_resolved_answers)")
        }
        ValueType::FixedBytes { .. } => {
            unreachable!("bytes<N> columns fill through the byte heap (fill_resolved_answers)")
        }
    }
    Ok(1)
}

fn push_resolved_answer<C: CatalogRead>(
    out: &mut Answers,
    catalog: &C,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    answer: &[u64],
) -> Result<()> {
    let mut word = 0;
    for column in columns {
        let (cell, width) = match column.ty() {
            ValueType::Bool => (Cell::Bool(answer[word] != 0), 1),
            ValueType::U64 => (Cell::U64(answer[word]), 1),
            ValueType::I64 => (Cell::I64((answer[word] ^ (1 << 63)).cast_signed()), 1),
            ValueType::F64 => (
                Cell::F64(crate::encoding::decode_f64(answer[word].to_be_bytes())?),
                1,
            ),
            ValueType::Interval { element, .. } | ValueType::FixedInterval { element, .. } => (
                Answers::interval_cell(*element, answer[word], answer[word + 1]),
                2,
            ),
            ValueType::String => {
                let (start, len) = memo.resolve(catalog, answer[word], out)?;
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
