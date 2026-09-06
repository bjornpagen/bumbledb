use std::mem::MaybeUninit;

use super::result::ResultCharge;
use super::{Answers, Cell, EitherSink, ResolveMemo, ValueType};

use crate::error::Result;
use crate::exec::sink::{ProjectionSink, ResidentRows};
use crate::image::NonresidentTextStore;
use crate::image::intern::InternerHandle;
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
#[expect(
    clippy::too_many_arguments,
    reason = "Separate borrowed arenas and execution limits remain explicit on this internal path"
)]
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
                drain_spilled_answers(
                    out,
                    interner,
                    store.as_deref_mut(),
                    memo,
                    columns,
                    sink,
                    charge,
                )
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
    store: Option<&mut NonresidentTextStore>,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    sink: &ProjectionSink,
) -> Result<()> {
    // Dense results must remain a linear walk. Select the representation
    // once, not on every `next()` of every output column. Hash-backed rows
    // keep their insertion order and exact-key deduplication unchanged.
    match sink.answers() {
        ResidentRows::Dense(answers) => {
            fill_resident_rows(out, interner, store, memo, columns, sink.len(), &answers)
        }
        ResidentRows::Hashed(answers) => {
            fill_resident_rows(out, interner, store, memo, columns, sink.len(), &answers)
        }
    }
}

#[expect(
    unsafe_code,
    reason = "Publish the reserved cells only after every column initialized every row"
)]
fn fill_resident_rows<'a>(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    mut store: Option<&mut NonresidentTextStore>,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    rows: usize,
    answers: &(impl Iterator<Item = &'a [u64]> + Clone),
) -> Result<()> {
    let arity = columns.len();
    let base = out.cells.len();
    let additional = rows.checked_mul(arity).expect("answer cell count");
    out.cells.reserve(additional);
    let mut word = 0;
    for (col, column) in columns.iter().enumerate() {
        word += match column.ty() {
            ValueType::String => {
                let mut answers = answers.clone();
                for row in 0..rows {
                    let answer = answers.next().expect("resident sink length");
                    let (start, len) =
                        memo.resolve(interner, store.as_deref_mut(), answer[word], out)?;
                    out.cells.spare_capacity_mut()[row * arity + col]
                        .write(Cell::String { start, len });
                }
                1
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                let mut answers = answers.clone();
                for row in 0..rows {
                    let answer = answers.next().expect("resident sink length");
                    let cell = out.fixed_bytes_cell(*len, &answer[word..word + width]);
                    out.cells.spare_capacity_mut()[row * arity + col].write(cell);
                }
                width
            }
            ty => fill_fixed_column(
                &mut out.cells.spare_capacity_mut()[..additional],
                arity,
                col,
                ty,
                word,
                answers.clone(),
            )?,
        };
    }
    // SAFETY: reserve admitted `additional` slots beyond the unchanged
    // length. Each column writes its slot for every row, requiring an
    // answer rather than silently truncating a zip. Text/blob resolution
    // only grows those separate heaps, never the cells vector. On error
    // or panic the old length remains valid; Cell has no drop resources.
    unsafe { out.cells.set_len(base + additional) };
    Ok(())
}

fn fill_fixed_column<'a>(
    cells: &mut [MaybeUninit<Cell>],
    arity: usize,
    col: usize,
    ty: &ValueType,
    word: usize,
    mut answers: impl Iterator<Item = &'a [u64]>,
) -> Result<usize> {
    let rows = cells
        .chunks_exact_mut(arity)
        .map(|slots| (slots, answers.next().expect("resident sink length")));
    match ty {
        ValueType::Bool => {
            for (slots, answer) in rows {
                slots[col].write(Cell::Bool(answer[word] != 0));
            }
        }
        ValueType::U64 => {
            for (slots, answer) in rows {
                slots[col].write(Cell::U64(answer[word]));
            }
        }
        ValueType::I64 => {
            for (slots, answer) in rows {
                slots[col].write(Cell::I64((answer[word] ^ (1 << 63)).cast_signed()));
            }
        }
        ValueType::F64 => {
            for (slots, answer) in rows {
                slots[col].write(Cell::F64(crate::encoding::decode_f64(
                    answer[word].to_be_bytes(),
                )?));
            }
        }
        ValueType::Uuid => {
            for (slots, answer) in rows {
                slots[col].write(Answers::uuid_cell(answer[word], answer[word + 1]));
            }
            return Ok(2);
        }
        ValueType::Interval { element, .. } => {
            for (slots, answer) in rows {
                slots[col].write(Answers::interval_cell(
                    *element,
                    answer[word],
                    answer[word + 1],
                ));
            }
            return Ok(2);
        }
        ValueType::FixedInterval { element, .. } => {
            for (slots, answer) in rows {
                slots[col].write(Answers::interval_cell(
                    element.element(),
                    answer[word],
                    answer[word + 1],
                ));
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
            ValueType::Uuid => (Answers::uuid_cell(answer[word], answer[word + 1]), 2),
            ValueType::Interval { element, .. } => (
                Answers::interval_cell(*element, answer[word], answer[word + 1]),
                2,
            ),
            ValueType::FixedInterval { element, .. } => (
                Answers::interval_cell(element.element(), answer[word], answer[word + 1]),
                2,
            ),
            ValueType::String => {
                let (start, len) =
                    memo.resolve(interner, store.as_deref_mut(), answer[word], out)?;
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
