//! Per-fact decode: the hoisted per-column decode plan
//! and the scan loop that fills the structure-of-arrays slabs through it.

use crate::encoding::{ValueType, decode_bool};
use crate::error::{CorruptionError, Error, Mismatch, Result};
use bumbledb_theory::schema::RelationId;

use super::{Column, ColumnSpan, ColumnWidth};

pub(super) enum Decode {
    Word {
        offset: usize,
        start: usize,
    },

    FixedBytes {
        offset: usize,
        starts: Vec<usize>,
        pad_mask: u64,
    },

    Interval {
        offset: usize,
        start_column: usize,
        end_column: usize,
    },

    FixedInterval {
        offset: usize,
        width: u64,
        start_column: usize,
        end_column: usize,
    },
    Bool {
        offset: usize,
        start: usize,
    },
}

/// # Panics
/// On a programmer-invariant violation: the field→column map put a word span
/// over a byte column.
fn words_start(column: Column) -> usize {
    match column {
        Column::Words { start } => start,
        Column::Bytes { .. } => unreachable!("word spans cover word columns"),
    }
}

/// # Panics
/// On a programmer-invariant violation: the field→column map put a byte span
/// over a word column.
fn bytes_start(column: Column) -> usize {
    match column {
        Column::Bytes { start } => start,
        Column::Words { .. } => unreachable!("byte spans cover byte columns"),
    }
}

pub(super) fn decode_plan(
    field_types: &[ValueType],
    spans: &[ColumnSpan],
    columns: &[Column],
    layout: &crate::encoding::FactLayout,
) -> Vec<Decode> {
    field_types
        .iter()
        .zip(spans)
        .enumerate()
        .map(|(field_idx, (desc, span))| {
            let offset = layout.field_offset(field_idx);
            let first = usize::from(span.first_column);
            match (span.width, desc) {

                (ColumnWidth::Word | ColumnWidth::Words { .. }, ValueType::FixedBytes { len }) => {
                    let words = crate::encoding::fixed_bytes_words(*len);
                    let pad_bytes = words * 8 - usize::from(*len);
                    if pad_bytes == 0 && words == 1 {
                        Decode::Word {
                            offset,
                            start: words_start(columns[first]),
                        }
                    } else {
                        Decode::FixedBytes {
                            offset,
                            starts: (0..words)
                                .map(|i| words_start(columns[first + i]))
                                .collect(),

                            pad_mask: if pad_bytes == 0 {
                                0
                            } else {
                                (1u64 << (8 * pad_bytes)) - 1
                            },
                        }
                    }
                }
                (ColumnWidth::Word, _) => Decode::Word {
                    offset,
                    start: words_start(columns[first]),
                },
                (ColumnWidth::WordPair, ValueType::FixedInterval { width: w, .. }) => {
                    Decode::FixedInterval {
                        offset,
                        width: *w,
                        start_column: words_start(columns[first]),
                        end_column: words_start(columns[first + 1]),
                    }
                }
                (ColumnWidth::WordPair, _) => Decode::Interval {
                    offset,
                    start_column: words_start(columns[first]),
                    end_column: words_start(columns[first + 1]),
                },
                (ColumnWidth::Words { .. }, _) => {
                    unreachable!("Words spans cover bytes<N> fields")
                }
                (ColumnWidth::Byte, ValueType::Bool) => Decode::Bool {
                    offset,
                    start: bytes_start(columns[first]),
                },
                (ColumnWidth::Byte, _) => unreachable!("1-byte columns are Bool"),
            }
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
pub(super) fn fill_one(
    rel: RelationId,
    plan: &[Decode],
    fact_width: usize,
    fact_bytes: &[u8],
    position: usize,
    row_count: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<usize> {
    if position >= row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }
    decode_fact(rel, plan, fact_width, fact_bytes, position, words, bytes)?;
    Ok(position + 1)
}

#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] 
pub(super) fn decode_fact(
    rel: RelationId,
    plan: &[Decode],
    fact_width: usize,
    fact_bytes: &[u8],
    position: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<()> {

    if fact_bytes.len() != fact_width {
        return Err(Error::Corruption(CorruptionError::WrongFactWidth {
            relation: rel,
            row_id: position as u64,
            mismatch: Mismatch {
                witnessed: fact_bytes.len(),
                required: fact_width,
            },
        }));
    }
    for step in plan {
        match step {
            Decode::Word { offset, start } => {
                // SAFETY: offset + 8 <= fact_width (layout-derived) and

                let word = u64::from_be_bytes(unsafe {
                    fact_bytes.as_ptr().add(*offset).cast::<[u8; 8]>().read()
                });
                unsafe {
                    *words.get_unchecked_mut(start + position) = word;
                }
            }
            Decode::FixedBytes {
                offset,
                starts,
                pad_mask,
            } => {
                // SAFETY: offset + 8 * starts.len() <= fact_width

                let field =
                    unsafe { fact_bytes.get_unchecked(*offset..*offset + 8 * starts.len()) };

                let (word_bytes, _) = field.as_chunks::<8>();
                let mut last = 0u64;
                for (start, &bytes) in starts.iter().zip(word_bytes) {
                    let word = u64::from_be_bytes(bytes);
                    unsafe {
                        *words.get_unchecked_mut(start + position) = word;
                    }
                    last = word;
                }

                if last & pad_mask != 0 {
                    return Err(Error::Corruption(CorruptionError::NonzeroFixedBytesPad(
                        last.to_be_bytes(),
                    )));
                }
            }
            Decode::Interval {
                offset,
                start_column,
                end_column,
            } => {
                // SAFETY: offset + 16 <= fact_width (layout-derived),

                let halves: [u8; 16] =
                    unsafe { fact_bytes.as_ptr().add(*offset).cast::<[u8; 16]>().read() };
                let (start_half, end_half) = crate::encoding::split_halves(halves);
                let start_word = u64::from_be_bytes(start_half);
                let end_word = u64::from_be_bytes(end_half);

                // strict `start < end` invariant IS this u64 compare.

                if start_word >= end_word {
                    return Err(Error::Corruption(CorruptionError::InvalidInterval(halves)));
                }
                unsafe {
                    *words.get_unchecked_mut(start_column + position) = start_word;
                    *words.get_unchecked_mut(end_column + position) = end_word;
                }
            }
            Decode::FixedInterval {
                offset,
                width,
                start_column,
                end_column,
            } => {
                // SAFETY: offset + 8 <= fact_width (layout-derived, the

                let start_bytes: [u8; 8] =
                    unsafe { fact_bytes.as_ptr().add(*offset).cast::<[u8; 8]>().read() };

                let (start_word, end_word) =
                    crate::encoding::decode_fixed_interval_start(start_bytes, *width)
                        .map_err(|e| Error::Corruption(e.into()))?;
                unsafe {
                    *words.get_unchecked_mut(start_column + position) = start_word;
                    *words.get_unchecked_mut(end_column + position) = end_word;
                }
            }
            Decode::Bool { offset, start } => {
                // SAFETY: as above.
                let byte = unsafe { *fact_bytes.get_unchecked(*offset) };
                decode_bool(byte)?;
                unsafe {
                    *bytes.get_unchecked_mut(start + position) = byte;
                }
            }
        }
    }
    Ok(())
}
