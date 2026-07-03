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

/// L1D set stride on the target machine: 8-way associative with a 16 KiB
/// stride, so columns scanned in lockstep must not sit at bases congruent
/// mod 16 KiB — the pathological aliasing case is a 10-20x slowdown
/// (`docs/reference/apple-silicon-performance.md`, Category 5).
const SET_STRIDE: usize = 16_384;

/// Column base alignment: safe under either reading of the flagged
/// 64B-vs-128B L1D line-size contradiction (128 implies 64).
const LINE: usize = 128;

/// Distinct 128-byte-aligned residues within one set stride. Relations with
/// more columns than this cannot have all-distinct residues; BCNF relations
/// are narrow, so the excess (if ever) simply reuses residues.
const RESIDUE_SLOTS: usize = SET_STRIDE / LINE;

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
    columns: Box<[Column]>,
    /// Backing store for 8-byte columns; column bases are 128-byte aligned
    /// with staggered set-stride residues (see [`SET_STRIDE`]).
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

/// Tracks which set-stride residues are taken while laying out columns.
struct ResidueStagger {
    used: [bool; RESIDUE_SLOTS],
}

impl ResidueStagger {
    fn new() -> Self {
        Self {
            used: [false; RESIDUE_SLOTS],
        }
    }

    /// Advances `cursor` (an element index into a backing store whose base
    /// address is `base_addr`, elements of `elem_size` bytes) to the next
    /// 128-byte-aligned position whose absolute address occupies an unused
    /// set-stride residue. Falls back to the first aligned position when
    /// every residue is taken (>128 columns).
    fn place(&mut self, base_addr: usize, elem_size: usize, cursor: usize) -> usize {
        let step = LINE / elem_size;
        let mut idx = cursor;
        // Align the absolute address to the line size.
        let misalign = (base_addr + idx * elem_size) % LINE;
        if misalign != 0 {
            idx += (LINE - misalign) / elem_size;
        }
        let aligned = idx;
        for _ in 0..RESIDUE_SLOTS {
            let slot = ((base_addr + idx * elem_size) % SET_STRIDE) / LINE;
            if !self.used[slot] {
                self.used[slot] = true;
                return idx;
            }
            idx += step;
        }
        aligned
    }
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
    // plus per-column alignment/stagger slack.
    let field_types: Vec<TypeDesc> = relation
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let word_cols = field_types.iter().filter(|t| t.width() == 8).count();
    let byte_cols = field_types.len() - word_cols;
    let mut words = vec![0u64; word_cols * (row_count + SET_STRIDE / 8 + LINE / 8)];
    let mut bytes = vec![0u8; byte_cols * (row_count + SET_STRIDE + LINE)];

    // Lay out column bases: 128-byte aligned, no two congruent mod 16 KiB.
    let words_addr = words.as_ptr().addr();
    let bytes_addr = bytes.as_ptr().addr();
    let mut stagger = ResidueStagger::new();
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

    // One sequential scan fills every column (positions = scan ordinals).
    let mut position = 0usize;
    for entry in read::scan(txn, schema, rel)? {
        let (_row_id, fact_bytes) = entry?;
        if position >= row_count {
            return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                relation: rel,
                stored: row_count as u64,
            }));
        }
        for (field_idx, (desc, column)) in field_types.iter().zip(&columns).enumerate() {
            let offset = layout.field_offset(field_idx);
            match column {
                Column::Words { start } => {
                    let word = u64::from_be_bytes(
                        fact_bytes[offset..offset + 8]
                            .try_into()
                            .expect("8-byte field"),
                    );
                    words[start + position] = word;
                }
                Column::Bytes { start } => {
                    let byte = fact_bytes[offset];
                    // Validated decode: corrupt Bool/Enum bytes abort the
                    // build — never a skip.
                    match desc {
                        TypeDesc::Bool => {
                            decode_bool(byte)?;
                        }
                        TypeDesc::Enum { variant_count } => {
                            decode_enum(byte, *variant_count)?;
                        }
                        _ => unreachable!("1-byte columns are Bool or Enum"),
                    }
                    bytes[start + position] = byte;
                }
            }
        }
        position += 1;
    }
    if position != row_count {
        return Err(Error::Corruption(CorruptionError::RowCountMismatch {
            relation: rel,
            stored: row_count as u64,
        }));
    }

    Ok(Arc::new(RelationImage {
        row_count,
        columns: columns.into_boxed_slice(),
        words,
        bytes,
    }))
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
        for i in 0..12 {
            for j in (i + 1)..12 {
                assert_ne!(
                    addrs[i] % SET_STRIDE,
                    addrs[j] % SET_STRIDE,
                    "columns {i} and {j} alias the same L1D set stride"
                );
            }
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
}
