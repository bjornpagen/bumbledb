//! Scalar F64 at the existing log boundary. These are exact payload-byte
//! tests and real engine replay tests, not successor-protocol qualification.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use bumbledb::{Db, F64, Value};
use bumbledb_log::apply::{Applied, ApplyRefusal, apply};
use bumbledb_log::codec::{BatchHeader, Codec, DecodeError, Op, OpKind, VERSION};
use bumbledb_log::inspect::{Kind, render};
use bumbledb_log::schema_file::{TheoryFile, parse};
use bumbledb_log::sidecar::Chain;

const RELATION: RelationId = RelationId(0);
const HEADER_BYTES: usize = 104;
const OP_BYTES: usize = 9;
const ROW_BYTES: usize = 18; // u64 tag + LE word, F64 tag + BE payload.
const FIRST_FLOAT: usize = HEADER_BYTES + OP_BYTES + 9;

const CANONICAL_BITS: [u64; 13] = [
    0x0000_0000_0000_0000, // one zero
    0x0000_0000_0000_0001, // minimum positive subnormal
    0x8000_0000_0000_0001, // minimum negative subnormal
    0x000f_ffff_ffff_ffff, // maximum subnormal
    0x0010_0000_0000_0000, // minimum positive normal
    0x8010_0000_0000_0000, // minimum negative normal
    0x3ff0_0000_0000_0000, // +1
    0xbff0_0000_0000_0000, // -1
    0x7fef_ffff_ffff_ffff, // maximum finite
    0xffef_ffff_ffff_ffff, // minimum finite
    0x7ff0_0000_0000_0000, // positive infinity
    0xfff0_0000_0000_0000, // negative infinity
    0x7ff8_0000_0000_0000, // one NaN
];

const NONCANONICAL_BITS: [u64; 7] = [
    0x8000_0000_0000_0000, // negative zero
    0x7ff0_0000_0000_0001, // signaling NaN
    0xfff0_0000_0000_0001, // negative signaling NaN
    0x7ff8_0000_0000_0001, // quiet NaN with payload
    0x7fff_ffff_ffff_ffff,
    0xfff8_0000_0000_0000, // negative quiet NaN
    0xffff_ffff_ffff_ffff,
];

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bdb-log-float-{}-{stamp}-{seq}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("exclusive fixture root");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "sample".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "value".into(),
                    value_type: ValueType::F64,
                    generation: Generation::None,
                },
            ],
            extension: None,
        }],
        statements: vec![],
    }
}

fn codec() -> Codec {
    let descriptor = schema();
    let admitted = descriptor.clone().validate().expect("F64 schema");
    Codec::new(&descriptor, fingerprint(&admitted).0)
}

fn header(codec: &Codec, slot: u64, prev: [u8; 32]) -> BatchHeader {
    BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid: codec.braids().braid_of(RELATION).unwrap(),
        braid_gen: slot,
        prev,
        writer: 42,
        timestamp: 99,
    }
}

fn op(kind: OpKind, bits: &[u64]) -> Op {
    Op {
        kind,
        relation: RELATION,
        rows: bits
            .iter()
            .enumerate()
            .map(|(id, bits)| {
                Box::from([
                    Value::U64(u64::try_from(id).unwrap()),
                    Value::F64(F64::from_bits(*bits)),
                ])
            })
            .collect(),
    }
}

#[test]
fn command_roundtrips_canonical_payloads_not_index_order_keys() {
    let codec = codec();
    let header = header(&codec, 1, [0; 32]);
    let ops = [op(OpKind::Insert, &CANONICAL_BITS)];
    let bytes = codec.encode(&header, &ops).unwrap();
    assert_eq!(VERSION, 3, "no protocol version reset in this packet");
    assert_eq!(&bytes[4..6], &[3, 0]);
    assert_eq!(
        bytes.len(),
        HEADER_BYTES + OP_BYTES + ROW_BYTES * CANONICAL_BITS.len()
    );
    for (row, bits) in CANONICAL_BITS.iter().enumerate() {
        let at = FIRST_FLOAT + ROW_BYTES * row;
        assert_eq!(
            bytes[at], 7,
            "previously unused log tag, not core type tag 8"
        );
        assert_eq!(&bytes[at + 1..at + 9], &bits.to_be_bytes());
        assert_ne!(
            &bytes[at + 1..at + 9],
            &F64::from_bits(*bits).to_order_bytes()
        );
    }
    let decoded = codec.decode(&bytes).unwrap();
    assert_eq!(decoded.header, header);
    assert_eq!(decoded.ops, ops);
    assert_eq!(codec.encode(&decoded.header, &decoded.ops).unwrap(), bytes);
}

#[test]
fn host_zero_and_nan_normalize_before_command_identity_is_sealed() {
    let codec = codec();
    let header = header(&codec, 1, [0; 32]);
    for host_bits in NONCANONICAL_BITS {
        let expected = if host_bits == 0x8000_0000_0000_0000 {
            0
        } else {
            0x7ff8_0000_0000_0000
        };
        let from_host = codec
            .encode(&header, &[op(OpKind::Insert, &[host_bits])])
            .unwrap();
        let canonical = codec
            .encode(&header, &[op(OpKind::Insert, &[expected])])
            .unwrap();
        assert_eq!(from_host, canonical);
        assert_eq!(blake3::hash(&from_host), blake3::hash(&canonical));
    }
}

#[test]
fn wire_negative_zero_and_alternate_nans_are_named_refusals() {
    let codec = codec();
    let original = codec
        .encode(
            &header(&codec, 1, [0; 32]),
            &[op(OpKind::Insert, &[0x3ff0_0000_0000_0000, 0])],
        )
        .unwrap();
    let at = FIRST_FLOAT + ROW_BYTES;
    for bits in NONCANONICAL_BITS {
        let mut bytes = original.clone();
        bytes[at + 1..at + 9].copy_from_slice(&bits.to_be_bytes());
        let error = codec.decode(&bytes).unwrap_err();
        assert_eq!(
            error,
            DecodeError::NonCanonicalF64 {
                relation: RELATION,
                row: 1,
                field: 1,
                bits,
            }
        );
        assert_eq!(error.identity(), "NonCanonicalF64");
    }
}

#[test]
fn every_truncated_float_payload_refuses_and_legacy_tags_keep_their_meaning() {
    let codec = codec();
    let bytes = codec
        .encode(
            &header(&codec, 1, [0; 32]),
            &[op(OpKind::Insert, &[0x3ff0_0000_0000_0000])],
        )
        .unwrap();
    assert_eq!(&bytes[FIRST_FLOAT..], &[7, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0]);
    for end in FIRST_FLOAT..bytes.len() {
        assert!(matches!(
            codec.decode(&bytes[..end]),
            Err(DecodeError::Truncated { .. })
        ));
    }
    for tag in [0, 1, 2, 3, 4, 5, 6, 8, 255] {
        let mut wrong = bytes.clone();
        wrong[FIRST_FLOAT] = tag;
        assert_eq!(
            codec.decode(&wrong).unwrap_err(),
            DecodeError::TagMismatch {
                relation: RELATION,
                row: 0,
                field: 1,
                expected: ValueType::F64,
                got: tag,
            }
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        codec.decode(&trailing),
        Err(DecodeError::TrailingBytes { .. })
    ));
}

#[test]
fn real_replay_reopen_and_delete_preserve_canonical_float_facts() {
    let root = TestDir::new();
    let path = root.0.join("db");
    let codec = codec();
    let header = header(&codec, 1, [0; 32]);
    let mut chain = Chain::genesis(codec.braids());
    let inserted = op(OpKind::Insert, &CANONICAL_BITS);
    let bytes = codec
        .encode(&header, std::slice::from_ref(&inserted))
        .unwrap();
    let db = Db::create(&path, schema()).unwrap().unwrap();
    assert_eq!(
        apply(&db, &mut chain, &codec, header.braid, 1, &bytes).unwrap(),
        Applied::Advanced { generation: 1 }
    );
    let mut rewound = Chain::genesis(codec.braids());
    assert_eq!(
        apply(&db, &mut rewound, &codec, header.braid, 1, &bytes).unwrap(),
        Applied::Absorbed { generation: 1 }
    );
    drop(db);
    let db = Db::open(&path, schema()).unwrap();
    for row in &inserted.rows {
        assert!(db.read(|snap| snap.contains_dyn(RELATION, row)).unwrap());
    }
    let delete_header = BatchHeader {
        braid_gen: 2,
        prev: *blake3::hash(&bytes).as_bytes(),
        ..header
    };
    let deletion = codec
        .encode(&delete_header, &[op(OpKind::Delete, &CANONICAL_BITS)])
        .unwrap();
    assert_eq!(
        apply(&db, &mut chain, &codec, header.braid, 2, &deletion).unwrap(),
        Applied::Advanced { generation: 2 }
    );
    for row in &inserted.rows {
        assert!(!db.read(|snap| snap.contains_dyn(RELATION, row)).unwrap());
    }
}

#[test]
fn malformed_later_float_does_not_apply_the_valid_command_prefix() {
    let root = TestDir::new();
    let db = Db::create(&root.0.join("db"), schema()).unwrap().unwrap();
    let codec = codec();
    let header = header(&codec, 1, [0; 32]);
    let mut chain = Chain::genesis(codec.braids());
    let inserted = op(OpKind::Insert, &[0x3ff0_0000_0000_0000, 0]);
    let mut bytes = codec
        .encode(&header, std::slice::from_ref(&inserted))
        .unwrap();
    let at = FIRST_FLOAT + ROW_BYTES;
    bytes[at + 1..at + 9].copy_from_slice(&0x8000_0000_0000_0000_u64.to_be_bytes());
    assert!(matches!(
        apply(&db, &mut chain, &codec, header.braid, 1, &bytes).unwrap(),
        Applied::Refused(ApplyRefusal::Decode(DecodeError::NonCanonicalF64 {
            row: 1,
            field: 1,
            ..
        }))
    ));
    assert_eq!(db.generation().unwrap().value(), 0);
    assert_eq!(chain.sum(), 0);
    for row in &inserted.rows {
        assert!(!db.read(|snap| snap.contains_dyn(RELATION, row)).unwrap());
    }
}

fn literal_schema(body: &str) -> String {
    format!(
        r#"{{"relations":[{{"name":"sample","fields":[{{"name":"value","type":"f64"}}],"extension":[{{"handle":"one","values":[{{"$f64":{body}}}]}}]}}],"statements":[]}}"#
    )
}

#[test]
fn theory_file_float_literals_are_exact_canonical_hex_not_json_numbers() {
    for bits in CANONICAL_BITS {
        let descriptor = parse(&literal_schema(&format!("\"{bits:016x}\""))).unwrap();
        assert_eq!(descriptor.relations[0].fields[0].value_type, ValueType::F64);
        assert_eq!(
            descriptor.relations[0].extension.as_ref().unwrap()[0].values[0],
            Value::F64(F64::from_bits(bits))
        );
    }
    for bits in NONCANONICAL_BITS {
        assert!(matches!(
            parse(&literal_schema(&format!("\"{bits:016x}\""))),
            Err(TheoryFile::Shape("noncanonical f64"))
        ));
    }
    for body in [
        "0",
        "null",
        "\"NaN\"",
        "\"Infinity\"",
        "\"-0\"",
        "\"3FF0000000000000\"",
        "\"000000000000000\"",
        "\"00000000000000000\"",
        "\"€0000000000000\"",
    ] {
        assert!(
            parse(&literal_schema(body)).is_err(),
            "body must refuse: {body}"
        );
    }
}

#[test]
fn inspect_preserves_float_identity_as_exact_bits() {
    let codec = codec();
    let bytes = codec
        .encode(
            &header(&codec, 1, [0; 32]),
            &[op(
                OpKind::Insert,
                &[0x7ff8_0000_0000_0000, 0x8000_0000_0000_0001],
            )],
        )
        .unwrap();
    let text = render(Kind::Batch, &bytes, &schema()).unwrap();
    assert!(text.contains("f64:7ff8000000000000"));
    assert!(text.contains("f64:8000000000000001"));
}
