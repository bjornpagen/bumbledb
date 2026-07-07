//! Columnar relation images, the image cache, and filtered views (docs/architecture).
//!
//! A relation image is **all columns** of a relation, decoded once from one
//! sequential `F`-prefix scan into structure-of-arrays vectors — the bridge
//! to paper-faithful execution (`docs/architecture/30-execution.md` D1,
//! `40-storage.md`). Immutable once built; `Arc` is the sharing unit.

pub mod cache;
pub mod view;

use std::sync::Arc;

use crate::encoding::{decode_bool, decode_enum, TypeDesc};
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::read;

/// The 16 KiB granule two hardware structures key on (measured,
/// docs/silicon/11): the L1D's set congruence (256 sets × 64 B lines,
/// index bits 6–13 — a mild ≤1.55× on real lockstep scans) and the
/// stream-prefetch trackers' page-number bits (the severe one: 4–6× on
/// DRAM lockstep scans when pitches sit near a multiple). The layout
/// rule pads PITCHES off multiples of this ([`PitchPadder`]); the old
/// belief that congruent bases cost "10–20×" is retired — that figure
/// required a fully serialized dependent chain and never applied to
/// scans.
const SET_STRIDE: usize = 16_384;

/// Column base alignment: 128 B is the L2/SLC/DRAM transfer granule
/// (the L1D manages 64 B lines behind it — both numbers are real,
/// docs/silicon/11); alignment to the outer granule serves both.
const LINE: usize = 128;

/// One decoded column: a range into the image's backing store. Positions
/// are dense scan ordinals `0..row_count`; row ids exist only in LMDB keys
/// and never appear in images.
#[derive(Debug, Clone, Copy)]
enum Column {
    /// 8-byte field: the byte-order-normalized u64 word. For every 8-byte
    /// type the word is `u64::from_be_bytes(canonical bytes)` — for U64 the
    /// numeric value, for I64 the sign-flipped biased word (order-preserving
    /// under u64 compare), for String/Bytes the intern id.
    Words { start: usize },
    /// 1-byte field: the validated Bool/Enum byte.
    Bytes { start: usize },
}

/// A borrowed view of one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnView<'a> {
    Words(&'a [u64]),
    Bytes(&'a [u8]),
}

/// The immutable full-width columnar image of one relation at one
/// generation.
#[derive(Debug)]
pub struct RelationImage {
    row_count: usize,
    /// Per-column exact distinct-value counts, computed LAZILY on first
    /// planner demand (docs/silicon/13): the eager per-column pass was
    /// the cold path's dominant fixed cost (~1.8 ms per 150k rows,
    /// paid before the first query could run — even a guard probe that
    /// needs no estimates). The image is generation-keyed by the cache,
    /// so a `OnceLock` per column IS the per-(snapshot, relation,
    /// column) stats cache; the counts themselves are unchanged (same
    /// exact algorithm, same values — laziness moves when, never what).
    distincts: Box<[std::sync::OnceLock<u64>]>,
    columns: Box<[Column]>,
    /// Backing store for 8-byte columns; column bases are 128-byte aligned
    /// with pitches padded off 16 KiB multiples (see [`PitchPadder`]).
    words: Vec<u64>,
    /// Backing store for 1-byte columns, same alignment discipline.
    bytes: Vec<u8>,
}

impl RelationImage {
    /// The image's heap footprint: both slab capacities in bytes (a
    /// store-level observability number — the benchmark report and the
    /// `image_build` trace span's byte arg read it).
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>() + self.bytes.capacity()
    }

    /// Number of facts in the image (dense positions `0..row_count`).
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    /// The exact distinct-value count of one column (docs/architecture/30-execution.md):
    /// word columns counted through a scratch hash set, byte columns
    /// through a 256-slot table. Intern ids are injective, so a
    /// String/Bytes column's word distincts are its value distincts.
    /// Computed on first demand and memoized on the image
    /// (docs/silicon/13); a plan that never asks — every guard probe —
    /// never pays the walk.
    #[must_use]
    pub fn distinct(&self, field_idx: usize) -> u64 {
        *self.distincts[field_idx].get_or_init(|| match self.column(field_idx) {
            ColumnView::Words(words) => {
                DistinctCounter::new(self.row_count).count_words(words)
            }
            ColumnView::Bytes(bytes) => DistinctCounter::count_bytes(bytes),
        })
    }

    /// The column for field `field_idx`, in declaration order.
    #[must_use]
    pub fn column(&self, field_idx: usize) -> ColumnView<'_> {
        match self.columns[field_idx] {
            Column::Words { start } => {
                ColumnView::Words(&self.words[start..start + self.row_count])
            }
            Column::Bytes { start } => {
                ColumnView::Bytes(&self.bytes[start..start + self.row_count])
            }
        }
    }

    /// The words of an 8-byte column.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `field_idx` is a 1-byte column
    /// (callers dispatch on the schema's `TypeDesc` width).
    #[cfg(test)]
    #[must_use]
    pub fn column_words(&self, field_idx: usize) -> &[u64] {
        match self.column(field_idx) {
            ColumnView::Words(words) => words,
            ColumnView::Bytes(_) => panic!("column {field_idx} is a 1-byte column"),
        }
    }

    /// The bytes of a 1-byte column.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `field_idx` is an 8-byte column.
    #[cfg(test)]
    #[must_use]
    pub fn column_bytes(&self, field_idx: usize) -> &[u8] {
        match self.column(field_idx) {
            ColumnView::Bytes(bytes) => bytes,
            ColumnView::Words(_) => panic!("column {field_idx} is an 8-byte column"),
        }
    }
}

/// Column pitches padded away from prefetch-tracker aliasing
/// (docs/silicon/11, bumblebench exp 10). The measured law: the L1D's
/// 16 KiB set congruence costs AT MOST 1.55× on real lockstep scans —
/// but stream-prefetch trackers alias on low 16 KiB page-number bits,
/// so power-of-two-ish pitches with small (1–3 line) staggers cost
/// 4–6× on DRAM-tier lockstep scans (8.13 vs 1.78 ns/row measured).
/// The old rule here — odd 128 B residues mod 16 KiB, the "stagger" —
/// was built against the first (mild) hazard and CREATED the second.
/// The replacement: when a column-to-column pitch is large enough to be
/// scanned from DRAM (≥ [`PAD_MIN_PITCH`]) and lands a small NONZERO
/// offset (≤ [`PAD_TOLERANCE`]) from a 16 KiB multiple, round it UP to
/// the next exact multiple — exact multiples measured clean (the
/// stagger-16,384 discriminator ran fast); the poison is the small
/// offset. Below [`PAD_MIN_PITCH`], columns are cache-resident at scan
/// time and no tracker interference was measured — disk is not free.
struct PitchPadder {
    /// Previous column start per backing slab (element index), so the
    /// pitch under test is always between neighbors in the SAME slab —
    /// lockstep scans stride within a slab.
    prev_start_by_width: [Option<usize>; 2],
}

/// Pitches below this never pad (the columns are cache-resident when
/// scanned; the pathology is a DRAM-stream phenomenon).
const PAD_MIN_PITCH: usize = 64 * 1024;

/// How close (bytes) to a 16 KiB multiple a pitch must land to count as
/// tracker-aliasing-shaped: the measured discriminators put stagger 8,
/// 32, 64, and 128 in the pathological band and 16,384 out of it.
const PAD_TOLERANCE: usize = 384;

impl PitchPadder {
    fn new() -> Self {
        Self {
            prev_start_by_width: [None; 2],
        }
    }

    /// Advances `cursor` (an element index into a backing store whose base
    /// address is `base_addr`, elements of `elem_size` bytes) to the next
    /// 128-byte-aligned position, then applies the pitch rule against the
    /// previous column in the same slab.
    fn place(&mut self, base_addr: usize, elem_size: usize, cursor: usize) -> usize {
        let mut idx = cursor;
        // Align the absolute address to the line size.
        let misalign = (base_addr + idx * elem_size) % LINE;
        if misalign != 0 {
            idx += (LINE - misalign) / elem_size;
        }
        let slab = usize::from(elem_size != 8);
        if let Some(prev) = self.prev_start_by_width[slab] {
            let pitch = (idx - prev) * elem_size;
            let residue = pitch % SET_STRIDE;
            // The measured band (exp 10's discriminators): EXACT 16 KiB
            // multiples are the fast configuration (stagger 16,384 ran
            // clean); the poison is a small NONZERO offset from one
            // (stagger 8/32 mild, 64/128 severe). Cure by rounding the
            // pitch UP to the next exact multiple.
            let in_band = (residue > 0 && residue <= PAD_TOLERANCE)
                || residue >= SET_STRIDE - PAD_TOLERANCE;
            if pitch >= PAD_MIN_PITCH && in_band {
                // Aligned starts make the residue a multiple of LINE,
                // so the delta divides evenly by either element size.
                idx += (SET_STRIDE - residue) / elem_size;
            }
        }
        self.prev_start_by_width[slab] = Some(idx);
        idx
    }
}

/// Checked slab lengths (in words and bytes) for the stored row count.
/// The `S` value is data: overflow in any size computation is typed
/// Corruption before a single byte is allocated.
fn slab_lengths(row_count: usize, word_cols: usize, byte_cols: usize) -> Result<(usize, usize)> {
    let corrupt = || Error::Corruption(CorruptionError::MalformedValue("S row count"));
    let word_len = row_count
        .checked_add(SET_STRIDE / 8 + LINE / 8)
        .and_then(|per_col| per_col.checked_mul(word_cols))
        .and_then(|words| words.checked_mul(8))
        .ok_or_else(corrupt)?
        / 8;
    let byte_len = row_count
        .checked_add(SET_STRIDE + LINE)
        .and_then(|per_col| per_col.checked_mul(byte_cols))
        .ok_or_else(corrupt)?;
    Ok((word_len, byte_len))
}

/// One column's hoisted decode step (docs/perf/ PRD 12): static offset,
/// validation arm resolved once — the row loop runs bare loads/stores.
enum Decode {
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
fn decode_plan(
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
fn fill_columns(
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

/// Builds the full-width image of `rel` from one sequential scan.
///
/// # Errors
///
/// Any scan corruption (wrong fact width) aborts the build; a scan yielding
/// a different number of rows than the stored `S` count is corruption too.
/// Dangling intern ids are *not* checked here — ids are opaque words at
/// this layer.
///
/// # Panics
///
/// Only on programmer-invariant violations (backing-store capacity computed
/// from the same counters the fill loop trusts).
pub fn build(txn: &ReadTxn<'_>, schema: &Schema, rel: RelationId) -> Result<Arc<RelationImage>> {
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let row_count = usize::try_from(read::row_count(txn, rel)?).expect("64-bit usize");

    // One up-front allocation per backing store, sized from the row count
    // plus per-column alignment/stagger slack. The stored `S` count is
    // data: every slab-size computation is checked, and overflow is
    // typed Corruption *before* any allocation is attempted (the
    // both-direction scan cross-check below stays the exactness
    // guarantee).
    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let word_cols = field_types.iter().filter(|t| t.width() == 8).count();
    let byte_cols = field_types.len() - word_cols;
    let (word_len, byte_len) = slab_lengths(row_count, word_cols, byte_cols)?;
    let mut words = vec![0u64; word_len];
    let mut bytes = vec![0u8; byte_len];

    // Lay out column bases: 128-byte aligned, pitches padded off 16 KiB
    // multiples (docs/silicon/11 — the tracker-aliasing rule).
    let words_addr = words.as_ptr().addr();
    let bytes_addr = bytes.as_ptr().addr();
    let mut stagger = PitchPadder::new();
    let mut word_cursor = 0usize;
    let mut byte_cursor = 0usize;
    let columns: Vec<Column> = field_types
        .iter()
        .map(|t| {
            if t.width() == 8 {
                let start = stagger.place(words_addr, 8, word_cursor);
                word_cursor = start + row_count;
                Column::Words { start }
            } else {
                let start = stagger.place(bytes_addr, 1, byte_cursor);
                byte_cursor = start + row_count;
                Column::Bytes { start }
            }
        })
        .collect();

    // One sequential scan fills every column (positions = scan ordinals),
    // through the hoisted decode plan (docs/perf/ PRD 12).
    let plan = decode_plan(&field_types, &columns, layout);
    let position = fill_columns(
        txn,
        schema,
        rel,
        &plan,
        layout.fact_width(),
        row_count,
        &mut words,
        &mut bytes,
    )?;
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }

    // Distinct counts are NOT computed here (docs/silicon/13): the
    // eager pass was the cold path's fixed cost. Each column's count
    // materializes on first planner demand ([`RelationImage::distinct`]).
    let distincts: Vec<std::sync::OnceLock<u64>> = columns
        .iter()
        .map(|_| std::sync::OnceLock::new())
        .collect();

    Ok(Arc::new(RelationImage {
        row_count,
        distincts: distincts.into_boxed_slice(),
        columns: columns.into_boxed_slice(),
        words,
        bytes,
    }))
}

/// The build-time distinct counter: a power-of-two open-addressed word
/// set sized once for the row count and memset-cleared per column.
struct DistinctCounter {
    slots: Vec<u64>,
    occupied: Vec<bool>,
}

impl DistinctCounter {
    fn new(row_count: usize) -> Self {
        let capacity = (row_count.max(1) * 2).next_power_of_two();
        Self {
            slots: vec![0; capacity],
            occupied: vec![false; capacity],
        }
    }

    fn count_words(&mut self, column: &[u64]) -> u64 {
        for flag in &mut self.occupied {
            *flag = false;
        }
        let mask = self.slots.len() - 1;
        let mut distinct = 0u64;
        for &word in column {
            // The COLT's word hash — one avalanche, linear probe.
            let mut h = 0x517C_C1B7_2722_0A95_u64 ^ word;
            h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            h ^= h >> 29;
            let mut idx = usize::try_from(h).expect("64-bit usize") & mask;
            loop {
                if !self.occupied[idx] {
                    self.occupied[idx] = true;
                    self.slots[idx] = word;
                    distinct += 1;
                    break;
                }
                if self.slots[idx] == word {
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
        distinct
    }

    fn count_bytes(column: &[u8]) -> u64 {
        let mut seen = [false; 256];
        let mut distinct = 0u64;
        for &byte in column {
            if !seen[usize::from(byte)] {
                seen[usize::from(byte)] = true;
                distinct += 1;
            }
        }
        distinct
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, encode_i64, ValueRef};
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::storage::keys::{self, KeyBuf, MAX_KEY};
    use crate::testutil::TempDir;

    /// R(id u64 serial, flag bool, kind enum[3], amount i64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "flag".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::Enum {
                            variants: ["A", "B", "C"].iter().map(|v| Box::from(*v)).collect(),
                        },
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);

    fn fact(schema: &Schema, id: u64, flag: bool, kind: u8, amount: i64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(id),
                ValueRef::Bool(flag),
                ValueRef::Enum(kind),
                ValueRef::I64(amount),
            ],
            schema.relation(R).layout(),
            &mut b,
        );
        b
    }

    fn populated(dir: &TempDir, schema: &Schema) -> Environment {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for i in 0..10u64 {
            let amount = i64::try_from(i).expect("small") * 7 - 30;
            delta
                .insert(
                    &view,
                    R,
                    &fact(schema, i, i % 2 == 0, (i % 3) as u8, amount),
                )
                .expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        env
    }

    /// Build-time distinct counts are exact per column type
    /// (docs/architecture/30-execution.md): serial ids all-distinct, bools 2, enums 3, and a
    /// skewed i64 column counted through the word set.
    #[test]
    fn distinct_counts_are_exact() {
        let dir = TempDir::new("image-distincts");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        // populated(): ids 0..10, flag i % 2, kind i % 3, amount i*7-30.
        assert_eq!(image.distinct(0), 10, "serial ids all distinct");
        assert_eq!(image.distinct(1), 2, "bools");
        assert_eq!(image.distinct(2), 3, "enum ordinals");
        assert_eq!(image.distinct(3), 10, "amounts all distinct");

        // A skewed refresh: 100 more rows sharing 5 amounts.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for i in 10..110u64 {
            let amount = i64::try_from(i % 5).expect("small");
            delta
                .insert(&view, R, &fact(&schema, i, true, 0, amount))
                .expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        assert_eq!(image.row_count(), 110);
        assert_eq!(image.distinct(0), 110);
        // Old 10 distinct amounts + {0..5}, minus the overlaps: the old
        // amounts are 7i - 30 (…-30, -23, …, 33); {0..5} intersects at
        // nothing except… 7i-30 ∈ {0,1,2,3,4} ⇔ i has no integer
        // solution except none (7i = 30..34 has none). 10 + 5 = 15.
        assert_eq!(image.distinct(3), 15);
    }

    /// PRD 12's profile split (ignored: timing evidence, run by hand):
    /// the LMDB cursor walk alone vs the full build, on a Posting-shaped
    /// 150k-row relation.
    #[test]
    #[ignore = "timing evidence, run by hand on the reference host"]
    fn image_build_split_evidence() {
        let dir = TempDir::new("image-split");
        let schema = posting_like_schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let txn0 = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let mut bytes = Vec::new();
        for i in 0..150_000u64 {
            bytes.clear();
            encode_fact(
                &[
                    ValueRef::U64(i),
                    ValueRef::U64(i % 512),
                    ValueRef::I64((i % 1000).cast_signed() - 500),
                    ValueRef::I64((i * 7 % 100_000).cast_signed()),
                    ValueRef::Bool(i % 2 == 0),
                ],
                schema.relation(R).layout(),
                &mut bytes,
            );
            delta.insert(&txn0, R, &bytes).expect("insert");
        }
        drop(txn0);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");

        // Walk floor: drain the cursor, touch every fact byte cheaply.
        let mut sink = 0u64;
        let walk = std::time::Instant::now();
        for _ in 0..5 {
            for entry in crate::storage::read::scan(&txn, &schema, R).expect("scan") {
                let (_, fact) = entry.expect("entry");
                sink = sink
                    .wrapping_add(u64::from(fact[0]))
                    .wrapping_add(fact.len() as u64);
            }
        }
        let walk = walk.elapsed() / 5;

        let full = std::time::Instant::now();
        for _ in 0..5 {
            let image = build(&txn, &schema, R).expect("build");
            sink = sink.wrapping_add(image.row_count() as u64);
        }
        let full = full.elapsed() / 5;
        println!(
            "image_build split over 150k rows: walk {walk:?}, full {full:?}, decode+scatter {:?} (sink {sink})",
            full.saturating_sub(walk)
        );
    }

    fn posting_like_schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "P".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "at".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "flag".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    #[test]
    fn columns_equal_per_field_decode_of_the_scan() {
        let dir = TempDir::new("image-columns");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        assert_eq!(image.row_count(), 10);

        let layout = schema.relation(R).layout();
        for (position, entry) in read::scan(&txn, &schema, R).expect("scan").enumerate() {
            let (_, fact_bytes) = entry.expect("ok");
            // 8-byte columns hold the byte-order-normalized word.
            let id_word = u64::from_be_bytes(fact_bytes[..8].try_into().expect("8"));
            assert_eq!(image.column_words(0)[position], id_word);
            let amount_off = layout.field_offset(3);
            let amount_word = u64::from_be_bytes(
                fact_bytes[amount_off..amount_off + 8]
                    .try_into()
                    .expect("8"),
            );
            assert_eq!(image.column_words(3)[position], amount_word);
            // 1-byte columns hold the validated byte.
            assert_eq!(image.column_bytes(1)[position], fact_bytes[8]);
            assert_eq!(image.column_bytes(2)[position], fact_bytes[9]);
        }
    }

    #[test]
    fn positions_stay_dense_under_row_id_holes() {
        let dir = TempDir::new("image-holes");
        let schema = schema();
        let env = populated(&dir, &schema);
        // Delete three facts, punching row-id holes.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for i in [2u64, 5, 7] {
            let amount = i64::try_from(i).expect("small") * 7 - 30;
            delta
                .delete(
                    &view,
                    R,
                    &fact(&schema, i, i % 2 == 0, (i % 3) as u8, amount),
                )
                .expect("delete");
        }
        drop(view);
        commit(delta, &env).expect("commit");

        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        assert_eq!(image.row_count(), 7);
        // Every position 0..7 is filled, in scan order.
        let scanned: Vec<u64> = read::scan(&txn, &schema, R)
            .expect("scan")
            .map(|e| {
                let (_, bytes) = e.expect("ok");
                u64::from_be_bytes(bytes[..8].try_into().expect("8"))
            })
            .collect();
        assert_eq!(image.column_words(0), &scanned[..]);
    }

    #[test]
    fn twelve_column_bases_are_aligned_and_stagger_distinctly() {
        let dir = TempDir::new("image-stagger");
        // 12 columns, mixed widths.
        let fields: Vec<FieldDescriptor> = (0..12)
            .map(|i| FieldDescriptor {
                name: format!("f{i}").into(),
                value_type: if i % 3 == 0 {
                    ValueType::Bool
                } else if i % 3 == 1 {
                    ValueType::U64
                } else {
                    ValueType::I64
                },
                generation: Generation::None,
            })
            .collect();
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Wide".into(),
                fields,
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture");
        let env = Environment::create(dir.path(), &schema).expect("create");
        // A few rows so columns have nonzero extent.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for row in 0..100i64 {
            let mut values = Vec::new();
            for i in 0..12 {
                values.push(match i % 3 {
                    0 => ValueRef::Bool(row % 2 == 0),
                    1 => ValueRef::U64(row.cast_unsigned() * 12 + i),
                    _ => ValueRef::I64(row * 12 + i64::try_from(i).expect("small")),
                });
            }
            let mut bytes = Vec::new();
            encode_fact(&values, schema.relation(R).layout(), &mut bytes);
            delta.insert(&view, R, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");

        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        let addrs: Vec<usize> = (0..12)
            .map(|i| match image.column(i) {
                ColumnView::Words(w) => w.as_ptr().addr(),
                ColumnView::Bytes(b) => b.as_ptr().addr(),
            })
            .collect();
        for (i, addr) in addrs.iter().enumerate() {
            assert_eq!(addr % LINE, 0, "column {i} base must be 128-byte aligned");
        }
        // The pitch rule (docs/silicon/11): no big same-slab pitch lands
        // within the tracker-aliasing tolerance of a 16 KiB multiple.
        // (At 100 rows every pitch is far below PAD_MIN_PITCH — assert
        // the rule vacuously holds here and structurally in
        // `big_column_pitches_avoid_the_tracker_band`.)
        for window in addrs.windows(2) {
            let pitch = window[1].abs_diff(window[0]);
            if pitch >= PAD_MIN_PITCH {
                let residue = pitch % SET_STRIDE;
                assert!(
                    residue == 0 || (residue > PAD_TOLERANCE && residue < SET_STRIDE - PAD_TOLERANCE),
                    "pitch {pitch} sits in the tracker-aliasing band"
                );
            }
        }
    }

    /// The pitch rule under DRAM-scale spans (docs/silicon/11): a
    /// power-of-two row span — the exact shape the old stagger rule
    /// turned into a 4–6× DRAM-scan pathology — lays out with every
    /// same-slab pitch clear of the 16 KiB tracker band.
    #[test]
    fn big_column_pitches_avoid_the_tracker_band() {
        // 4 u64 columns × 16384 rows: span = 128 KiB exactly (pow-2,
        // 16 KiB-multiple) — unpadded pitches would land at residue 0.
        let fields: Vec<FieldDescriptor> = (0..4)
            .map(|i| FieldDescriptor {
                name: format!("c{i}").into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            })
            .collect();
        let schema = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Big".into(),
                fields,
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture");
        let dir = TempDir::new("image-pitch");
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for row in 0..16_384u64 {
            let values = [
                ValueRef::U64(row),
                ValueRef::U64(row ^ 1),
                ValueRef::U64(row ^ 2),
                ValueRef::U64(row ^ 3),
            ];
            let mut bytes = Vec::new();
            encode_fact(&values, schema.relation(R).layout(), &mut bytes);
            delta.insert(&view, R, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        let addrs: Vec<usize> = (0..4)
            .map(|i| match image.column(i) {
                ColumnView::Words(w) => w.as_ptr().addr(),
                ColumnView::Bytes(_) => unreachable!("all u64"),
            })
            .collect();
        for (i, window) in addrs.windows(2).enumerate() {
            let pitch = window[1] - window[0];
            assert!(pitch >= PAD_MIN_PITCH, "spans are DRAM-scale here");
            let residue = pitch % SET_STRIDE;
            assert!(
                residue == 0 || (residue > PAD_TOLERANCE && residue < SET_STRIDE - PAD_TOLERANCE),
                "pitch {i}→{} = {pitch} sits in the tracker band (residue {residue})",
                i + 1
            );
        }
    }

    #[test]
    fn i64_word_order_matches_logical_order() {
        let samples = [
            i64::MIN,
            i64::MIN + 1,
            -1_000_000,
            -1,
            0,
            1,
            42,
            1_000_000,
            i64::MAX - 1,
            i64::MAX,
        ];
        let words: Vec<u64> = samples
            .iter()
            .map(|v| u64::from_be_bytes(encode_i64(*v)))
            .collect();
        for pair in words.windows(2) {
            assert!(pair[0] < pair[1], "u64 word compare must match i64 order");
        }
    }

    #[test]
    fn zero_row_relation_builds_an_empty_image() {
        let dir = TempDir::new("image-empty");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        assert_eq!(image.row_count(), 0);
        assert!(image.column_words(0).is_empty());
        assert!(image.column_bytes(1).is_empty());
    }

    #[test]
    fn scan_corruption_aborts_the_build() {
        let dir = TempDir::new("image-corrupt");
        let schema = schema();
        let env = populated(&dir, &schema);
        {
            let victim = {
                let txn = env.read_txn().expect("txn");
                read::scan(&txn, &schema, R)
                    .expect("scan")
                    .map(|e| e.expect("ok").0)
                    .max()
                    .expect("nonempty")
            };
            let mut wtxn = env.write_txn().expect("txn");
            let mut key: KeyBuf = [0; MAX_KEY];
            let len = keys::fact_key(&mut key, R, victim);
            env.data()
                .put(wtxn.raw_mut(), &key[..len], &[0xFF])
                .expect("put");
            wtxn.commit().expect("commit");
        }
        let txn = env.read_txn().expect("txn");
        let err = build(&txn, &schema, R).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::WrongFactWidth { .. })
            ),
            "{err:?}"
        );
    }

    #[test]
    fn byte_size_covers_rows_and_slab_slack() {
        let dir = TempDir::new("image-byte-size");
        let schema = schema();
        let env = populated(&dir, &schema);
        let txn = env.read_txn().expect("txn");
        let image = build(&txn, &schema, R).expect("build");
        // The fixture: 10 rows over 2 word columns (id, amount) and 2 byte
        // columns (flag, kind). Lower bound: the raw payload; upper bound:
        // payload plus per-column alignment/stagger slack.
        let payload = 10 * (2 * 8 + 2);
        assert!(image.byte_size() >= payload, "{}", image.byte_size());
        let slack = 4 * (SET_STRIDE + LINE);
        assert!(
            image.byte_size() <= payload + slack,
            "{}",
            image.byte_size()
        );
    }

    /// PRD 06 (docs/hardening): a corrupt (astronomical) stored `S` row
    /// count is typed Corruption before any slab allocation is
    /// attempted — never an OOM abort.
    #[test]
    fn a_corrupt_row_count_errors_before_allocating() {
        let dir = TempDir::new("image-corrupt-row-count");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        {
            let mut wtxn = env.write_txn().expect("txn");
            let mut key: KeyBuf = [0; MAX_KEY];
            let len = keys::stat_key(&mut key, R, keys::StatKind::RowCount);
            env.data()
                .put(
                    wtxn.raw_mut(),
                    &key[..len],
                    u64::MAX.to_le_bytes().as_slice(),
                )
                .expect("plant");
            wtxn.commit().expect("commit");
        }
        let txn = env.read_txn().expect("txn");
        let err = build(&txn, &schema, R).map(|_| ()).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::MalformedValue("S row count"))
            ),
            "{err:?}"
        );
    }
}
