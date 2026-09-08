use std::mem::MaybeUninit;

use super::{Answers, Cell, EitherSink, ResolveMemo, ValueType};

use super::source::work_error;
use crate::error::Result;
use crate::exec::sink::{ProjectionSink, ResidentRows};
use crate::image::intern::InternerHandle;
use crate::ir::validate::SignatureColumn;
use crate::work::WorkContext;

/// Finalize through the existing representation-specific kernels. Fixed
/// columns fill in bounded polling batches; no per-row quota accounting
/// forces an otherwise columnar result onto a row-major path.
pub(super) fn finalize(
    sink: &mut EitherSink,
    answer_scratch: &mut Vec<u64>,
    memo: &mut ResolveMemo,
    interner: &InternerHandle<'_>,
    columns: &[SignatureColumn],
    out: &mut Answers,
    work: &WorkContext,
) -> Result<()> {
    work.checkpoint().map_err(work_error)?;
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
                columns,
                out,
                work,
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
                drain_spilled_answers(out, interner, memo, columns, sink, work)
            } else {
                fill_resolved_answers(out, interner, memo, columns, sink, work)
            };
            if result.is_err() {
                // The fills pre-size/append rows: drop the partial carrier
                // content so no failed work looks like a result.
                out.cells.truncate(base);
            }
            result
        }
        EitherSink::Aggregate(sink) => {
            out.cells.reserve(
                sink.group_count()
                    .checked_mul(columns.len())
                    .ok_or(crate::error::Error::ResultBytesOverflow)?,
            );
            sink.finalize_into(answer_scratch, |answer| {
                work.checkpoint().map_err(work_error)?;
                push_resolved_answer(out, interner, memo, columns, answer)?;
                Ok(())
            })
        }
    }
}

fn drain_spilled_answers(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    sink: &mut ProjectionSink,
    work: &WorkContext,
) -> Result<()> {
    sink.for_each_answer(&mut |answer| {
        work.checkpoint().map_err(work_error)?;
        push_resolved_answer(out, interner, memo, columns, answer)
    })
}

fn fill_resolved_answers(
    out: &mut Answers,
    interner: &InternerHandle<'_>,
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    sink: &ProjectionSink,
    work: &WorkContext,
) -> Result<()> {
    // Dense results must remain a linear walk. Select the representation
    // once, not on every `next()` of every output column. Hash-backed rows
    // keep their insertion order and exact-key deduplication unchanged.
    match sink.answers() {
        ResidentRows::Dense(answers) => {
            fill_resident_rows(out, interner, memo, columns, sink.len(), &answers, work)
        }
        ResidentRows::Hashed(answers) => {
            fill_resident_rows(out, interner, memo, columns, sink.len(), &answers, work)
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
    memo: &mut ResolveMemo,
    columns: &[SignatureColumn],
    rows: usize,
    answers: &(impl Iterator<Item = &'a [u64]> + Clone),
    work: &WorkContext,
) -> Result<()> {
    let arity = columns.len();
    let base = out.cells.len();
    let additional = rows
        .checked_mul(arity)
        .ok_or(crate::error::Error::ResultBytesOverflow)?;
    out.cells.reserve(additional);
    let mut offset = 0;
    for (col, column) in columns.iter().enumerate() {
        work.checkpoint().map_err(work_error)?;
        offset += match column.ty() {
            ValueType::String => {
                let mut answers = answers.clone();
                for row in 0..rows {
                    if row % crate::exec::sink::STEP_QUANTUM as usize == 0 {
                        work.checkpoint().map_err(work_error)?;
                    }
                    let answer = answers.next().expect("resident sink length");
                    let (start, len) = memo.resolve(interner, answer[offset], out)?;
                    out.cells.spare_capacity_mut()[row * arity + col]
                        .write(Cell::String { start, len });
                }
                1
            }
            ValueType::FixedBytes { len } => {
                let width = crate::encoding::fixed_bytes_words(*len);
                let mut answers = answers.clone();
                for row in 0..rows {
                    if row % crate::exec::sink::STEP_QUANTUM as usize == 0 {
                        work.checkpoint().map_err(work_error)?;
                    }
                    let answer = answers.next().expect("resident sink length");
                    let cell = out.fixed_bytes_cell(*len, &answer[offset..offset + width]);
                    out.cells.spare_capacity_mut()[row * arity + col].write(cell);
                }
                width
            }
            ty => fill_fixed_column(
                &mut out.cells.spare_capacity_mut()[..additional],
                arity,
                col,
                ty,
                offset,
                answers.clone(),
                work,
            )?,
        };
    }
    // SAFETY: reserve admitted `additional` slots beyond the unchanged
    // length. Each column writes its slot for every row, requiring an
    // answer rather than silently truncating a zip. Text/blob resolution
    // only grows those separate heaps, never the cells vector. On error
    // or panic the old length remains valid; Cell has no drop resources.
    work.checkpoint().map_err(work_error)?;
    unsafe { out.cells.set_len(base + additional) };
    Ok(())
}

fn fill_fixed_column<'a>(
    cells: &mut [MaybeUninit<Cell>],
    arity: usize,
    col: usize,
    ty: &ValueType,
    offset: usize,
    mut answers: impl Iterator<Item = &'a [u64]>,
    work: &WorkContext,
) -> Result<usize> {
    let width = match ty {
        ValueType::Uuid | ValueType::Interval { .. } | ValueType::FixedInterval { .. } => 2,
        _ => 1,
    };
    let batch_cells = arity
        .checked_mul(crate::exec::sink::STEP_QUANTUM as usize)
        .ok_or(crate::error::Error::ResultBytesOverflow)?;
    for batch in cells.chunks_mut(batch_cells) {
        work.checkpoint().map_err(work_error)?;
        let filled_width = fill_fixed_chunk(batch, arity, col, ty, offset, &mut answers)?;
        debug_assert_eq!(filled_width, width);
    }
    Ok(width)
}

fn fill_fixed_chunk<'a>(
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
                let (start, len) = memo.resolve(interner, answer[word], out)?;
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
