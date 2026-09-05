use super::result::ResultCharge;
use super::{Answers, Cell, EitherSink, ResolveMemo, ValueType};

use crate::error::Result;
use crate::exec::sink::ProjectionSink;
use crate::image::intern::InternerHandle;
use crate::image::NonresidentTextStore;
use crate::ir::validate::SignatureColumn;

/// Reverses if: a profiled finalize shows the String/FixedBytes match arms'
/// mere presence taxing an all-words fill ≥ the house bar — re-twin before
/// believing it.
///
/// With a [`ResultCharge`] installed (the sealed-result path) every
/// appended row is noted: result bytes charge in bounded quanta as the set
/// grows and past-allowance rows continue in the scratch backing — the
/// column-major bulk fill is bypassed there, since it materializes the
/// whole set before any charge could refuse.
pub(super) fn finalize(
    sink: &mut EitherSink,
    answer_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    interner: &InternerHandle<'_>,
    mut store: Option<&mut NonresidentTextStore>,
    columns: &[SignatureColumn],
    out: &mut Answers,
    mut charge: Option<&mut ResultCharge<'_>>,
) -> Result<()> {
    memo.clear();
    match sink {
        EitherSink::Computed(sink) => {
            if let Some(error) = &sink.error {
                return Err(error.clone());
            }
            finalize(
                &mut sink.inner,
                answer_scratch,
                memo,
                interner,
                store,
                columns,
                out,
                charge,
            )
        }
        EitherSink::Projection(sink) => {
            let base = out.cells.len();
            let result = if let Some(error) = sink.take_error() {
                // A sticky emit failure spoiled the execution: refuse
                // before any answer publishes (Q-ATOMIC).
                Err(error)
            } else if sink.spilled() {
                // The spilled drain is row-major across both tiers.
                drain_spilled_answers(out, interner, store.as_deref_mut(), memo, columns, sink, charge)
            } else if let Some(charge) = charge {
                // Charged construction is row-major: note every row so the
                // budget can refuse (and the backing can spill) before the
                // whole set materializes.
                let mut result = Ok(());
                for answer in sink.answers() {
                    result = push_resolved_answer(
                        out,
                        interner,
                        store.as_deref_mut(),
                        memo,
                        columns,
                        answer,
                    )
                        .and_then(|()| charge.note_row(out, memo));
                    if result.is_err() {
                        break;
                    }
                }
                result
            } else {
                fill_resolved_answers(out, interner, store.as_deref_mut(), memo, columns, sink)
            };
            if result.is_err() {
                // The fills pre-size/append rows: drop the partial carrier
                // content so no failed work looks like a result.
                out.cells.truncate(base);
            }
            result
        }
        EitherSink::Aggregate(sink) => {
            if charge.is_none() {
                out.cells.reserve(sink.group_count() * columns.len());
            }
            sink.finalize_into(answer_scratch, |answer| {
                push_resolved_answer(out, interner, store.as_deref_mut(), memo, columns, answer)?;
                match charge.as_deref_mut() {
                    Some(charge) => charge.note_row(out, memo),
                    None => Ok(()),
                }
            })
        }
    }
}

fn drain_spilled_answers(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    mut store: Option<&mut NonresidentTextStore>,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    sink: &mut ProjectionSink,
    mut charge: Option<&mut ResultCharge<'_>>,
) -> Result<()> {
    sink.for_each_answer(&mut |answer| {
        push_resolved_answer(out, interner, store.as_deref_mut(), memo, columns, answer)?;
        match charge.as_deref_mut() {
            Some(charge) => charge.note_row(out, memo),
            None => Ok(()),
        }
    })
}

fn fill_resolved_answers(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    mut store: Option<&mut NonresidentTextStore>,
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
                    let (start, len) =
                        memo.resolve(interner, store.as_deref_mut(), answer[word], out)?;
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
        ValueType::Id128 => {
            for (slots, answer) in rows {
                slots[col] = Answers::id128_cell(answer[word], answer[word + 1]);
            }
            return Ok(2);
        }
        ValueType::Interval { element, .. } => {
            for (slots, answer) in rows {
                slots[col] = Answers::interval_cell(*element, answer[word], answer[word + 1]);
            }
            return Ok(2);
        }
        ValueType::FixedInterval { element, .. } => {
            for (slots, answer) in rows {
                slots[col] =
                    Answers::interval_cell(element.element(), answer[word], answer[word + 1]);
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

fn push_resolved_answer(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    mut store: Option<&mut NonresidentTextStore>,
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
            ValueType::Id128 => (Answers::id128_cell(answer[word], answer[word + 1]), 2),
            ValueType::Interval { element, .. } => (
                Answers::interval_cell(*element, answer[word], answer[word + 1]),
                2,
            ),
            ValueType::FixedInterval { element, .. } => (
                Answers::interval_cell(element.element(), answer[word], answer[word + 1]),
                2,
            ),
            ValueType::String => {
                let (start, len) = memo.resolve(interner, store.as_deref_mut(), answer[word], out)?;
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
