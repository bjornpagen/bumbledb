//! `bytes<N>` image columns: ⌈N/8⌉ parallel word columns (the interval
//! two-column precedent, generalized), padded-word round-trip at a pad
//! boundary, and the pad-corruption arm — a nonzero trailing pad byte is
//! typed corruption at build, never a skip.

use crate::encoding::{encode_fact, ValueRef};
use crate::error::{CorruptionError, Error};
use crate::image::{build, ColumnWidth};
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::storage::read;
use crate::testutil::TempDir;

/// D(id u64, head bytes<9>, hash bytes<32>).
fn schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "D".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("head", ValueType::FixedBytes { len: 9 }),
                field("hash", ValueType::FixedBytes { len: 32 }),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const D: RelationId = RelationId(0);

/// Adversarial digests: shared zero prefix, the row id at the tail —
/// row 0's hash is the all-zeros digest.
fn fact(schema: &Schema, id: u64) -> Vec<u8> {
    let mut head = [0u8; 9];
    head[8] = u8::try_from(id % 251).expect("byte");
    let mut hash = [0u8; 32];
    hash[24..].copy_from_slice(&id.to_be_bytes());
    let mut bytes = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(id),
            ValueRef::fixed_bytes(&head),
            ValueRef::fixed_bytes(&hash),
        ],
        schema.relation(D).layout(),
        &mut bytes,
    );
    bytes
}

fn populated(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for i in 0..10u64 {
        delta.insert(&view, D, &fact(schema, i)).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

#[test]
fn fixed_bytes_fields_decode_into_padded_word_columns() {
    let dir = TempDir::new("image-fixed-bytes");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, D).expect("build");

    // Spans: bytes<9> = 2 word columns, bytes<32> = 4 — column indices
    // shift accordingly (the field→column map, never raw field indices).
    let head = image.span(crate::schema::FieldId(1));
    assert_eq!(head.width, ColumnWidth::Words { count: 2 });
    let hash = image.span(crate::schema::FieldId(2));
    assert_eq!(hash.first_column, 3);
    assert_eq!(hash.width, ColumnWidth::Words { count: 4 });

    // Round-trip at the pad boundary: the head's second word carries the
    // ninth byte then zero pad; the hash's tail word is the source id and
    // its lead words are the shared zero prefix. Scan order is row-id
    // order, not insert order, so rows correlate through the id column.
    let ids = image.column_words(0);
    let head_tail = image.column_words(usize::from(head.first_column) + 1);
    let hash_lead = image.column_words(usize::from(hash.first_column));
    let hash_tail = image.column_words(usize::from(hash.first_column) + 3);
    let mut seen: Vec<u64> = ids.to_vec();
    seen.sort_unstable();
    assert_eq!(seen, (0..10).collect::<Vec<u64>>());
    for row in 0..10usize {
        let id = ids[row];
        assert_eq!(head_tail[row], (id % 251) << 56);
        assert_eq!(hash_lead[row], 0, "the adversarial shared prefix");
        assert_eq!(hash_tail[row], id);
    }
}

#[test]
fn a_nonzero_pad_byte_aborts_the_build_typed() {
    let dir = TempDir::new("image-fixed-bytes-pad");
    let schema = schema();
    let env = populated(&dir, &schema);
    let victim = {
        let txn = env.read_txn().expect("txn");
        read::scan(&txn, &schema, D)
            .expect("scan")
            .map(|e| e.expect("ok").0)
            .max()
            .expect("nonempty")
    };
    {
        // Corrupt the bytes<9> field's trailing pad (field offset 8,
        // value bytes 8..17, pad 17..24): the pad is encoding, not data.
        let mut corrupt = fact(&schema, 9);
        corrupt[20] = 0x5A;
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::fact_key(&mut key, D, victim);
        env.data()
            .put(wtxn.raw_mut(), &key[..len], &corrupt)
            .expect("plant");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let err = build(&txn, &schema, D).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::NonzeroFixedBytesPad(_))
        ),
        "{err:?}"
    );
}
