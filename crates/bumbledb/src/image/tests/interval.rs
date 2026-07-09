//! PRD 14 criteria: an interval field decodes into two parallel 8-byte
//! columns (start, end) — no 16-byte column kind — and inverted halves are
//! corruption, aborting the build.

use crate::encoding::{encode_fact, encode_i64, ValueRef};
use crate::error::{CorruptionError, Error};
use crate::image::{build, ColumnSpan, ColumnWidth};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::storage::read;
use crate::testutil::TempDir;

/// T(id u64, during interval<i64>, kind enum[3]).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "T".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "kind".into(),
                    value_type: ValueType::Enum {
                        variants: ["A", "B", "C"].iter().map(|v| Box::from(*v)).collect(),
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const T: RelationId = RelationId(0);

/// The rows, in insert (= scan) order: a fully negative interval, one
/// crossing zero, one fully positive — starts ascending, so the golden
/// word-order assertion pins the sign-flip.
const ROWS: [(u64, i64, i64, u8); 3] = [(0, -100, -7, 0), (1, -5, 9, 1), (2, 3, 7, 2)];

fn fact(schema: &Schema, id: u64, start: i64, end: i64, kind: u8) -> Vec<u8> {
    let mut b = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(id),
            ValueRef::IntervalI64(start, end),
            ValueRef::Enum(kind),
        ],
        schema.relation(T).layout(),
        &mut b,
    );
    b
}

/// The biased I64 column word.
fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn populated(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, start, end, kind) in ROWS {
        delta
            .insert(&view, T, &fact(schema, id, start, end, kind))
            .expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

#[test]
fn interval_field_decodes_into_two_word_columns_with_golden_words() {
    let dir = TempDir::new("image-interval-golden");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, T).expect("build");
    assert_eq!(image.row_count(), 3);

    // The field→column map: three fields, four columns — the interval
    // spans columns 1 and 2 (start, end), the enum lands at 3.
    assert_eq!(
        image.span(FieldId(0)),
        ColumnSpan {
            first_column: 0,
            width: ColumnWidth::Word,
        }
    );
    assert_eq!(
        image.span(FieldId(1)),
        ColumnSpan {
            first_column: 1,
            width: ColumnWidth::WordPair,
        }
    );
    assert_eq!(
        image.span(FieldId(2)),
        ColumnSpan {
            first_column: 3,
            width: ColumnWidth::Byte,
        }
    );

    // Golden words per row (positions are scan ordinals — row ids are
    // content hashes, so match rows by their id word): each half is the
    // byte-order-normalized word of its scalar encoding, including the
    // negative starts.
    let ids = image.column_words(0);
    let starts = image.column_words(1);
    let ends = image.column_words(2);
    let kinds = image.column_bytes(3);
    let mut seen = [false; ROWS.len()];
    for position in 0..image.row_count() {
        let row = usize::try_from(ids[position]).expect("fixture ids are 0..3");
        let (id, start, end, kind) = ROWS[row];
        assert_eq!(ids[position], id);
        assert_eq!(starts[position], w(start), "start word of row {id}");
        assert_eq!(ends[position], w(end), "end word of row {id}");
        assert_eq!(kinds[position], kind, "enum byte of row {id}");
        assert!(
            starts[position] < ends[position],
            "row {id}: start < end as bare u64 words"
        );
        seen[row] = true;
    }
    assert_eq!(seen, [true; ROWS.len()], "every fixture row decoded");

    // The sign-flip lands inside each half's encoding (never re-derived
    // in image code): sorting the columns as u64 words yields exactly
    // the i64 element order — negative bounds below positive.
    let mut start_words = starts.to_vec();
    start_words.sort_unstable();
    assert_eq!(start_words, [w(-100), w(-5), w(3)]);
    let mut end_words = ends.to_vec();
    end_words.sort_unstable();
    assert_eq!(end_words, [w(-7), w(7), w(9)]);
}

#[test]
fn inverted_interval_halves_abort_the_build() {
    let dir = TempDir::new("image-interval-inverted");
    let schema = schema();
    let env = populated(&dir, &schema);

    // Hand-corrupt the last row's F value: same width, halves swapped —
    // an interval whose encoded start ≥ end.
    let layout = schema.relation(T).layout();
    let offset = layout.field_offset(1);
    let mut corrupt = fact(&schema, 2, 3, 7, 2);
    for i in 0..8 {
        corrupt.swap(offset + i, offset + 8 + i);
    }
    let victim = {
        let txn = env.read_txn().expect("txn");
        read::scan(&txn, &schema, T)
            .expect("scan")
            .map(|e| e.expect("ok").0)
            .max()
            .expect("nonempty")
    };
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::fact_key(&mut key, T, victim);
        env.data()
            .put(wtxn.raw_mut(), &key[..len], &corrupt)
            .expect("put");
        wtxn.commit().expect("commit");
    }

    let txn = env.read_txn().expect("txn");
    let err = build(&txn, &schema, T).unwrap_err();
    let mut halves = [0u8; 16];
    halves.copy_from_slice(&corrupt[offset..offset + 16]);
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::InvalidInterval(bytes)) if bytes == halves
        ),
        "{err:?}"
    );
}
