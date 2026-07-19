//! Per-fact decode: the hoisted per-column decode plan
//! and the scan loop that fills the structure-of-arrays slabs through it.

use crate::encoding::{TypeDesc, decode_bool};
use crate::error::{CorruptionError, Error, Result};
use bumbledb_theory::schema::RelationId;

use super::{Column, ColumnSpan, ColumnWidth};

/// One field's hoisted decode step: static offset,
/// validation arm resolved once — the row loop runs bare loads/stores.
pub(super) enum Decode {
    Word {
        offset: usize,
        start: usize,
    },
    /// A `bytes<N>` field: its `⌈N/8⌉` padded words go to consecutive
    /// word columns (`starts`, one slab start per column), with the
    /// trailing pad validated zero — the pad is encoding, not data
    /// (`pad_mask` covers the last word's pad bytes; 0 for N % 8 == 0).
    FixedBytes {
        offset: usize,
        starts: Vec<usize>,
        pad_mask: u64,
    },
    /// An interval field: the first 8 bytes go to the start column, the
    /// second 8 to the end column (two ordinary word columns —
    /// `docs/architecture/50-storage.md`).
    Interval {
        offset: usize,
        start_column: usize,
        end_column: usize,
    },
    /// A fixed-width (`interval<E, w>`) field: ONE stored word — the
    /// start — filling BOTH word columns, the end derived as
    /// `start + w` in the order-preserving word domain (the bias is
    /// additive, so this is exact for either element). Kernels see the
    /// same two-column shape a general interval fills; the width lives
    /// only here, in the type.
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
                // A bytes<N> field of any span shape: word loads plus the
                // pad check (a bytes<8> field has no pad and decodes as a
                // plain word).
                (ColumnWidth::Word | ColumnWidth::Words { .. }, TypeDesc::FixedBytes { len }) => {
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
                            // BE words put the pad in the last word's low
                            // bytes; a zero mask means no pad to check.
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
                (ColumnWidth::WordPair, TypeDesc::Interval { width: Some(w), .. }) => {
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
                (ColumnWidth::Byte, TypeDesc::Bool) => Decode::Bool {
                    offset,
                    start: bytes_start(columns[first]),
                },
                (ColumnWidth::Byte, _) => unreachable!("1-byte columns are Bool"),
            }
        })
        .collect()
}

/// The scan loop: one width check per fact, then unchecked loads and
/// slab stores through the plan, filling positions `from..` in scan
/// order. Returns one past the last position filled. Both fill paths
/// share it: a full build passes [`crate::storage::read::scan`] and
/// `from = 0`; the append path passes [`crate::storage::read::scan_from`]
/// and the base image's row count, so only the tail rows decode
/// (`docs/architecture/50-storage.md`
/// § the image cache). The row id is discarded at this boundary — row
/// ids never enter images; positions are dense scan ordinals.
#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
pub(super) fn fill_columns<'txn>(
    rel: RelationId,
    scan: impl Iterator<Item = Result<(u64, &'txn [u8])>>,
    plan: &[Decode],
    fact_width: usize,
    from: usize,
    row_count: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<usize> {
    let mut position = from;
    for entry in scan {
        let (_row_id, fact_bytes) = entry?;
        if position >= row_count {
            return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                relation: rel,
                stored: row_count as u64,
            }));
        }
        decode_fact(rel, plan, fact_width, fact_bytes, position, words, bytes)?;
        position += 1;
    }
    Ok(position)
}

/// Decodes one canonical fact through the plan into the slabs at
/// `position` — one width check up front makes every plan offset
/// in-bounds. Both fill paths run this: the LMDB scan ([`fill_columns`])
/// and closed-relation synthesis ([`super::build::synthesize_closed`]),
/// so a sealed extension decodes through exactly the machinery a stored
/// fact does.
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)] // 00-product policy: image decode kernels
pub(super) fn decode_fact(
    rel: RelationId,
    plan: &[Decode],
    fact_width: usize,
    fact_bytes: &[u8],
    position: usize,
    words: &mut [u64],
    bytes: &mut [u8],
) -> Result<()> {
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
                // SAFETY: offset + 8 <= fact_width (layout-derived) and
                // the width was checked above, so the byte-aligned
                // array read is in-bounds; position < row_count checked
                // above, slabs sized to row_count.
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
                // (layout-derived) and the width was checked above;
                // slab bounds as for Word.
                let field =
                    unsafe { fact_bytes.get_unchecked(*offset..*offset + 8 * starts.len()) };
                // `as_chunks` carries the walk's width in its type; the
                // remainder is empty by construction (the field spans
                // whole words).
                let (word_bytes, _) = field.as_chunks::<8>();
                let mut last = 0u64;
                for (start, &bytes) in starts.iter().zip(word_bytes) {
                    let word = u64::from_be_bytes(bytes);
                    unsafe {
                        *words.get_unchecked_mut(start + position) = word;
                    }
                    last = word;
                }
                // The pad is encoding, not data: a nonzero trailing
                // pad byte is corruption — hard error, never a skip.
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
                // width checked above, so the byte-aligned array read
                // is in-bounds; slab bounds as for Word.
                let halves: [u8; 16] =
                    unsafe { fact_bytes.as_ptr().add(*offset).cast::<[u8; 16]>().read() };
                let (start_half, end_half) = crate::encoding::split_halves(halves);
                let start_word = u64::from_be_bytes(start_half);
                let end_word = u64::from_be_bytes(end_half);
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
            Decode::FixedInterval {
                offset,
                width,
                start_column,
                end_column,
            } => {
                // SAFETY: offset + 8 <= fact_width (layout-derived, the
                // fixed encoding is one word), width checked above; slab
                // bounds as for Word.
                let start_bytes: [u8; 8] =
                    unsafe { fact_bytes.as_ptr().add(*offset).cast::<[u8; 8]>().read() };
                // The Q2 corruption check and the end derivation are one
                // shared decoder — hard error, never a skip.
                let (start_word, end_word) =
                    crate::encoding::decode_fixed_interval_start(start_bytes, *width)
                        .map_err(Error::Corruption)?;
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
