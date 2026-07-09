//! Per-fact decode (docs/perf/ PRD 12): the hoisted per-column decode plan
//! and the scan loop that fills the structure-of-arrays slabs through it.

use crate::encoding::{decode_bool, decode_enum, TypeDesc};
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;

use super::{Column, ColumnSpan, ColumnWidth};

/// One field's hoisted decode step (docs/perf/ PRD 12): static offset,
/// validation arm resolved once — the row loop runs bare loads/stores.
pub(super) enum Decode {
    Word {
        offset: usize,
        start: usize,
    },
    /// An interval field: the first 8 bytes go to the start column, the
    /// second 8 to the end column (two ordinary word columns —
    /// `docs/architecture/50-storage.md`).
    Interval {
        offset: usize,
        start_column: usize,
        end_column: usize,
    },
    Bool {
        offset: usize,
        start: usize,
    },
    Enum {
        offset: usize,
        start: usize,
        variants: u16,
    },
}

/// The word-slab start of a column that must be 8-byte.
///
/// # Panics
///
/// On a programmer-invariant violation: the field→column map put a word
/// span over a byte column.
fn words_start(column: Column) -> usize {
    match column {
        Column::Words { start } => start,
        Column::Bytes { .. } => unreachable!("word spans cover word columns"),
    }
}

/// The byte-slab start of a column that must be 1-byte.
///
/// # Panics
///
/// On a programmer-invariant violation: the field→column map put a byte
/// span over a word column.
fn bytes_start(column: Column) -> usize {
    match column {
        Column::Bytes { start } => start,
        Column::Words { .. } => unreachable!("byte spans cover byte columns"),
    }
}

/// Builds the per-field decode plan from the field→column map.
pub(super) fn decode_plan(
    field_types: &[TypeDesc],
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
                (ColumnWidth::Word, _) => Decode::Word {
                    offset,
                    start: words_start(columns[first]),
                },
                (ColumnWidth::WordPair, _) => Decode::Interval {
                    offset,
                    start_column: words_start(columns[first]),
                    end_column: words_start(columns[first + 1]),
                },
                (ColumnWidth::Byte, TypeDesc::Bool) => Decode::Bool {
                    offset,
                    start: bytes_start(columns[first]),
                },
                (ColumnWidth::Byte, TypeDesc::Enum { variant_count }) => Decode::Enum {
                    offset,
                    start: bytes_start(columns[first]),
                    variants: *variant_count,
                },
                _ => unreachable!("1-byte columns are Bool or Enum"),
            }
        })
        .collect()
}

/// The scan loop: one width check per fact, then unchecked loads and
/// slab stores through the plan. Returns the rows filled.
#[allow(unsafe_code)] // 00-product policy: image decode kernels
#[allow(clippy::too_many_arguments)]
pub(super) fn fill_columns(
    txn: &ReadTxn<'_>,
    schema: &Schema,
    rel: RelationId,
    plan: &[Decode],
    fact_width: usize,
    row_count: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<usize> {
    let mut position = 0usize;
    for entry in read::scan(txn, schema, rel)? {
        let (_row_id, fact_bytes) = entry?;
        if position >= row_count {
            return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                relation: rel,
                stored: row_count as u64,
            }));
        }
        // One width check per fact makes every plan offset in-bounds.
        if fact_bytes.len() != fact_width {
            return Err(Error::Corruption(CorruptionError::WrongFactWidth {
                relation: rel,
                row_id: position as u64,
                expected: fact_width,
                actual: fact_bytes.len(),
            }));
        }
        for step in plan {
            match step {
                Decode::Word { offset, start } => {
                    // SAFETY: offset + 8 <= fact_width (layout-derived)
                    // and the width was checked above; position <
                    // row_count checked above, slabs sized to row_count.
                    let word = u64::from_be_bytes(unsafe {
                        fact_bytes
                            .get_unchecked(*offset..*offset + 8)
                            .try_into()
                            .expect("8-byte field")
                    });
                    unsafe {
                        *words.get_unchecked_mut(start + position) = word;
                    }
                }
                Decode::Interval {
                    offset,
                    start_column,
                    end_column,
                } => {
                    // SAFETY: offset + 16 <= fact_width (layout-derived),
                    // width checked above; slab bounds as for Word.
                    let halves: [u8; 16] = unsafe {
                        fact_bytes
                            .get_unchecked(*offset..*offset + 16)
                            .try_into()
                            .expect("16-byte field")
                    };
                    let start_word =
                        u64::from_be_bytes(halves[..8].try_into().expect("8-byte half"));
                    let end_word = u64::from_be_bytes(halves[8..].try_into().expect("8-byte half"));
                    // The stored halves are order-preserving words (the
                    // I64 sign-flip lives inside the encoding), so the
                    // strict `start < end` invariant IS this u64 compare.
                    // A violation is corruption: hard error, never a skip
                    // (`docs/architecture/50-storage.md`).
                    if start_word >= end_word {
                        return Err(Error::Corruption(CorruptionError::InvalidInterval(halves)));
                    }
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
                Decode::Enum {
                    offset,
                    start,
                    variants,
                } => {
                    // SAFETY: as above.
                    let byte = unsafe { *fact_bytes.get_unchecked(*offset) };
                    decode_enum(byte, *variants)?;
                    unsafe {
                        *bytes.get_unchecked_mut(start + position) = byte;
                    }
                }
            }
        }
        position += 1;
    }
    Ok(position)
}
