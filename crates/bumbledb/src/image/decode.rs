//! Per-fact decode (docs/perf/ PRD 12): the hoisted per-column decode plan
//! and the scan loop that fills the structure-of-arrays slabs through it.

use crate::encoding::{decode_bool, decode_enum, TypeDesc};
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;

use super::Column;

/// One column's hoisted decode step (docs/perf/ PRD 12): static offset,
/// validation arm resolved once — the row loop runs bare loads/stores.
pub(super) enum Decode {
    Word {
        offset: usize,
        start: usize,
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

/// Builds the per-column decode plan from the layout.
pub(super) fn decode_plan(
    field_types: &[TypeDesc],
    columns: &[Column],
    layout: &crate::encoding::FactLayout,
) -> Vec<Decode> {
    field_types
        .iter()
        .zip(columns)
        .enumerate()
        .map(|(field_idx, (desc, column))| {
            let offset = layout.field_offset(field_idx);
            match (column, desc) {
                (Column::Words { start }, _) => Decode::Word {
                    offset,
                    start: *start,
                },
                (Column::Bytes { start }, TypeDesc::Bool) => Decode::Bool {
                    offset,
                    start: *start,
                },
                (Column::Bytes { start }, TypeDesc::Enum { variant_count }) => Decode::Enum {
                    offset,
                    start: *start,
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
