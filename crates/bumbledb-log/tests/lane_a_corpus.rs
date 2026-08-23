//! The checked-in codec corpus: binary batches beside JSON expectation
//! sidecars, mirrored byte-exactly by the TypeScript implementation.
//! Every case is reproduced from this table and compared against the
//! files on disk; `BUMBLEDB_LOG_BLESS=1` rewrites the corpus after a
//! deliberate format change. Ok cases additionally pin the
//! decode-encode fixpoint.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::BTreeMap;

use bumbledb::Interval;
use bumbledb::Value;
use bumbledb::schema::SchemaDescriptor;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use serde_json::Value as Json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Ok,
    Refusal(&'static str),
}

struct Case {
    name: &'static str,
    schema: &'static str,
    bytes: Vec<u8>,
    intent: Intent,
}

fn codec_for(schemas: &BTreeMap<String, SchemaDescriptor>, name: &str) -> Codec {
    Codec::new(&schemas[name], support::corpus_fingerprint(name))
}

fn header(codec: &Codec, braid_raw: u32, braid_gen: u64, prev: [u8; 32]) -> BatchHeader {
    BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid: codec.braids().parse(braid_raw).expect("fixture braid"),
        braid_gen,
        prev,
        writer: 7,
        timestamp: 1_755_801_600_000,
    }
}

fn u(value: u64) -> Value {
    Value::U64(value)
}

fn i(value: i64) -> Value {
    Value::I64(value)
}

fn s(text: &str) -> Value {
    Value::String(text.into())
}

fn iv(start: u64, end: u64) -> Value {
    Value::IntervalU64(Interval::new(start, end).expect("interval"))
}

fn ivi(start: i64, end: i64) -> Value {
    Value::IntervalI64(Interval::new(start, end).expect("interval"))
}

fn fixed_u(start: u64) -> Value {
    Value::IntervalU64(Interval::fixed(start, 5).expect("fixed interval"))
}

fn fixed_i(start: i64) -> Value {
    Value::IntervalI64(Interval::fixed(start, 5).expect("fixed interval"))
}

fn bytes3(raw: [u8; 3]) -> Value {
    Value::FixedBytes(Box::from(raw))
}

fn kitchen_row_simple() -> Box<[Value]> {
    Box::from([
        Value::Bool(false),
        u(1),
        i(-1),
        s("a"),
        bytes3([9, 9, 9]),
        iv(1, 2),
        ivi(-1, 1),
        fixed_u(3),
        fixed_i(-3),
    ])
}

fn kitchen_minimal(codec: &Codec) -> Vec<u8> {
    let ops = [Op {
        kind: OpKind::Insert,
        relation: bumbledb::RelationId(0),
        rows: vec![kitchen_row_simple()],
    }];
    codec
        .encode(&header(codec, 0, 1, [0u8; 32]), &ops)
        .expect("minimal kitchen batch")
}

/// The ok half of the case table: one entry per wire feature.
#[expect(clippy::too_many_lines, reason = "the case table is the corpus")]
fn ok_cases(schemas: &BTreeMap<String, SchemaDescriptor>) -> Vec<Case> {
    let kitchen = codec_for(schemas, "kitchen");
    let booking = codec_for(schemas, "booking");
    let multi = codec_for(schemas, "multi");
    let rel = bumbledb::RelationId;

    let mut cases = Vec::new();

    let all_tags_ops = [
        Op {
            kind: OpKind::Insert,
            relation: rel(0),
            rows: vec![
                Box::from([
                    Value::Bool(true),
                    u(u64::MAX),
                    i(i64::MIN),
                    s("h\u{e9}llo\u{26f3}"),
                    bytes3([0xde, 0xad, 0xbe]),
                    iv(0, u64::MAX),
                    ivi(i64::MIN, i64::MAX),
                    fixed_u(10),
                    fixed_i(-2),
                ]),
                Box::from([
                    Value::Bool(false),
                    u(0),
                    i(i64::MAX),
                    s(""),
                    bytes3([0, 0, 0]),
                    iv(5, 6),
                    ivi(-1, 0),
                    fixed_u(0),
                    fixed_i(i64::MIN),
                ]),
            ],
        },
        Op {
            kind: OpKind::Delete,
            relation: rel(0),
            rows: vec![Box::from([
                Value::Bool(true),
                u(42),
                i(0),
                s("x"),
                bytes3([1, 2, 3]),
                iv(1, 2),
                ivi(0, 1),
                fixed_u(7),
                fixed_i(0),
            ])],
        },
    ];
    cases.push(Case {
        name: "ok_all_tags",
        schema: "kitchen",
        bytes: kitchen
            .encode(&header(&kitchen, 0, 1, [0u8; 32]), &all_tags_ops)
            .expect("encode"),
        intent: Intent::Ok,
    });

    cases.push(Case {
        name: "ok_empty_ops",
        schema: "kitchen",
        bytes: kitchen
            .encode(&header(&kitchen, 0, 9, [0x11; 32]), &[])
            .expect("encode"),
        intent: Intent::Ok,
    });

    let classes_ops = [
        Op {
            kind: OpKind::Insert,
            relation: rel(0),
            rows: vec![Box::from([u(7), u(1)])],
        },
        Op {
            kind: OpKind::Insert,
            relation: rel(1),
            rows: vec![
                Box::from([u(5), u(7), u(2), u(3), iv(10, 20)]),
                Box::from([u(6), u(7), u(2), u(2), iv(10, 20)]),
            ],
        },
        Op {
            kind: OpKind::Insert,
            relation: rel(2),
            rows: vec![Box::from([u(2), u(10)])],
        },
        Op {
            kind: OpKind::Delete,
            relation: rel(2),
            rows: vec![Box::from([u(9), u(5)])],
        },
        Op {
            kind: OpKind::Delete,
            relation: rel(0),
            rows: vec![Box::from([u(8), u(0)])],
        },
    ];
    cases.push(Case {
        name: "ok_conflict_classes",
        schema: "booking",
        bytes: booking
            .encode(&header(&booking, 0, 42, [0x22; 32]), &classes_ops)
            .expect("encode"),
        intent: Intent::Ok,
    });

    let closed_ops = [
        Op {
            kind: OpKind::Insert,
            relation: rel(1),
            rows: vec![Box::from([u(1)])],
        },
        Op {
            kind: OpKind::Insert,
            relation: rel(0),
            rows: vec![Box::from([u(3), u(9)])],
        },
    ];
    cases.push(Case {
        name: "ok_closed_empty",
        schema: "multi",
        bytes: multi
            .encode(&header(&multi, 0, 2, [0x44; 32]), &closed_ops)
            .expect("encode"),
        intent: Intent::Ok,
    });

    let serial_ops = [
        Op {
            kind: OpKind::Insert,
            relation: rel(6),
            rows: vec![Box::from([u(1)]), Box::from([u(2)])],
        },
        Op {
            kind: OpKind::Insert,
            relation: rel(5),
            rows: vec![Box::from([u(9)])],
        },
    ];
    cases.push(Case {
        name: "ok_serial_global",
        schema: "multi",
        bytes: multi
            .encode(&header(&multi, 5, 1, [0u8; 32]), &serial_ops)
            .expect("encode"),
        intent: Intent::Ok,
    });

    cases
}

fn patched(base: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut bytes = base.to_vec();
    bytes[at..at + with.len()].copy_from_slice(with);
    bytes
}

/// Raw wire scribbles for the refusal bins the encoder refuses to
/// produce: header layout mirrored by hand so the corpus construction
/// is independent of the encoder under test.
mod raw {
    pub fn header(fingerprint: &[u8; 32], braid: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"BDBL");
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(fingerprint);
        out.extend_from_slice(&braid.to_le_bytes());
        out.extend_from_slice(&1u64.to_le_bytes());
        out.extend_from_slice(&[0u8; 32]);
        out.extend_from_slice(&1u64.to_le_bytes());
        out.extend_from_slice(&1_000u64.to_le_bytes());
        out
    }

    pub fn one_row_op(header: &mut Vec<u8>, relation: u32, row: &[u8]) {
        header.extend_from_slice(&1u32.to_le_bytes());
        header.push(1);
        header.extend_from_slice(&relation.to_le_bytes());
        header.extend_from_slice(&1u32.to_le_bytes());
        header.extend_from_slice(row);
    }

    pub fn bool_field(byte: u8) -> Vec<u8> {
        vec![0, byte]
    }

    pub fn u64_field(value: u64) -> Vec<u8> {
        let mut out = vec![1u8];
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    pub fn i64_field(value: i64) -> Vec<u8> {
        let mut out = vec![2u8];
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    pub fn string_field(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![3u8];
        out.extend_from_slice(&u32::try_from(bytes.len()).expect("len").to_le_bytes());
        out.extend_from_slice(bytes);
        out
    }

    pub fn fixed_bytes_field(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![4u8];
        out.extend_from_slice(bytes);
        out
    }

    pub fn interval_u64_field(start: u64, end: u64) -> Vec<u8> {
        let mut out = vec![5u8];
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&end.to_le_bytes());
        out
    }

    pub fn fixed_interval_u64_field(start: u64) -> Vec<u8> {
        let mut out = vec![6u8];
        out.extend_from_slice(&start.to_le_bytes());
        out
    }
}

fn kitchen_row_prefix(upto: usize) -> Vec<u8> {
    let fields: Vec<Vec<u8>> = vec![
        raw::bool_field(0),
        raw::u64_field(1),
        raw::i64_field(-1),
        raw::string_field(b"a"),
        raw::fixed_bytes_field(&[9, 9, 9]),
        raw::interval_u64_field(1, 2),
    ];
    fields[..upto].concat()
}

fn row_refusal(fingerprint: &[u8; 32], poison: &[u8], prefix_fields: usize) -> Vec<u8> {
    let mut bytes = raw::header(fingerprint, 0);
    let mut row = kitchen_row_prefix(prefix_fields);
    row.extend_from_slice(poison);
    raw::one_row_op(&mut bytes, 0, &row);
    bytes
}

/// The refusal half of the case table: one entry per typed refusal.
#[expect(clippy::too_many_lines, reason = "the case table is the corpus")]
fn refusal_cases(schemas: &BTreeMap<String, SchemaDescriptor>) -> Vec<Case> {
    let kitchen = codec_for(schemas, "kitchen");
    let booking = codec_for(schemas, "booking");
    let multi = codec_for(schemas, "multi");
    let kitchen_fp = support::corpus_fingerprint("kitchen");

    let base = kitchen_minimal(&kitchen);
    let mut cases = Vec::new();
    let mut push = |name, schema, bytes, refusal| {
        cases.push(Case {
            name,
            schema,
            bytes,
            intent: Intent::Refusal(refusal),
        });
    };

    push(
        "r_bad_magic",
        "kitchen",
        patched(&base, 0, b"XDBL"),
        "BadMagic",
    );
    push(
        "r_version_1",
        "kitchen",
        patched(&base, 4, &1u16.to_le_bytes()),
        "Version",
    );
    push(
        "r_flags_nonzero",
        "kitchen",
        patched(&base, 6, &1u16.to_le_bytes()),
        "Flags",
    );
    push(
        "r_fingerprint_mismatch",
        "kitchen",
        patched(&base, 8, &[base[8] ^ 0xff]),
        "FingerprintMismatch",
    );
    push(
        "r_op_kind_3",
        "kitchen",
        patched(&base, 104, &[3]),
        "UnknownOpKind",
    );
    push(
        "r_op_kind_unknown",
        "kitchen",
        patched(&base, 104, &[9]),
        "UnknownOpKind",
    );
    push(
        "r_unknown_relation",
        "kitchen",
        patched(&base, 105, &99u32.to_le_bytes()),
        "UnknownRelation",
    );
    push(
        "r_trailing_byte",
        "kitchen",
        [base.as_slice(), &[0u8]].concat(),
        "TrailingBytes",
    );
    push(
        "r_truncated_row",
        "kitchen",
        base[..base.len() - 5].to_vec(),
        "Truncated",
    );

    let booking_ops = [Op {
        kind: OpKind::Insert,
        relation: bumbledb::RelationId(0),
        rows: vec![Box::from([Value::U64(7), Value::U64(1)]) as Box<[Value]>],
    }];
    let booking_base = booking
        .encode(&header(&booking, 0, 1, [0u8; 32]), &booking_ops)
        .expect("booking batch");
    push(
        "r_unknown_braid",
        "booking",
        patched(&booking_base, 40, &1u32.to_le_bytes()),
        "UnknownBraid",
    );
    push(
        "r_relation_outside_braid",
        "booking",
        patched(&booking_base, 105, &3u32.to_le_bytes()),
        "OpRelationOutsideBraid",
    );

    let multi_ops = [Op {
        kind: OpKind::Insert,
        relation: bumbledb::RelationId(1),
        rows: vec![Box::from([Value::U64(1)]) as Box<[Value]>],
    }];
    let multi_base = multi
        .encode(&header(&multi, 0, 1, [0u8; 32]), &multi_ops)
        .expect("multi batch");
    push(
        "r_closed_relation",
        "multi",
        patched(&multi_base, 105, &4u32.to_le_bytes()),
        "ClosedRelation",
    );

    push(
        "r_bool_byte_2",
        "kitchen",
        row_refusal(&kitchen_fp, &raw::bool_field(2), 0),
        "BoolByte",
    );
    push(
        "r_tag_mismatch",
        "kitchen",
        row_refusal(&kitchen_fp, &raw::u64_field(1), 0),
        "TagMismatch",
    );
    push(
        "r_string_bad_utf8",
        "kitchen",
        row_refusal(&kitchen_fp, &raw::string_field(&[0xff, 0xfe]), 3),
        "InvalidUtf8",
    );
    push(
        "r_interval_empty",
        "kitchen",
        row_refusal(&kitchen_fp, &raw::interval_u64_field(7, 7), 5),
        "EmptyInterval",
    );
    push(
        "r_interval_inverted",
        "kitchen",
        row_refusal(&kitchen_fp, &raw::interval_u64_field(9, 3), 5),
        "EmptyInterval",
    );
    {
        let mut row = kitchen_row_prefix(6);
        row.extend_from_slice(&{
            let mut lane = vec![5u8];
            lane.extend_from_slice(&(-1i64).to_le_bytes());
            lane.extend_from_slice(&1i64.to_le_bytes());
            lane
        });
        row.extend_from_slice(&raw::fixed_interval_u64_field(u64::MAX - 2));
        let mut bytes = raw::header(&kitchen_fp, 0);
        raw::one_row_op(&mut bytes, 0, &row);
        push(
            "r_fixed_interval_overflow",
            "kitchen",
            bytes,
            "IntervalOverflow",
        );
    }

    cases
}

fn sidecar_for(case: &Case, codec: &Codec) -> Json {
    let fingerprint = support::hex(codec.fingerprint());
    match codec.decode(&case.bytes) {
        Ok(batch) => {
            assert_eq!(case.intent, Intent::Ok, "case {} decoded", case.name);
            let reencoded = codec
                .encode(&batch.header, &batch.ops)
                .expect("re-encode decoded batch");
            assert_eq!(
                reencoded, case.bytes,
                "case {}: decode-encode fixpoint",
                case.name
            );
            let mut sidecar = support::render_batch(&batch);
            let object = sidecar.as_object_mut().expect("object");
            object.insert("schema".into(), Json::String(case.schema.into()));
            object.insert("fingerprint".into(), Json::String(fingerprint));
            object.insert("expect".into(), Json::String("ok".into()));
            sidecar
        }
        Err(refusal) => {
            let Intent::Refusal(expected) = case.intent else {
                panic!("case {} refused: {refusal:?}", case.name);
            };
            assert_eq!(
                refusal.identity(),
                expected,
                "case {}: refusal identity",
                case.name
            );
            serde_json::json!({
                "schema": case.schema,
                "fingerprint": fingerprint,
                "expect": "refusal",
                "refusal": refusal.identity(),
            })
        }
    }
}

#[test]
fn corpus_matches_the_case_table() {
    let schemas = support::load_schemas();
    let dir = support::corpus_dir().join("batch");
    if support::bless() {
        std::fs::create_dir_all(&dir).expect("corpus dir");
    }

    let mut all = ok_cases(&schemas);
    all.extend(refusal_cases(&schemas));
    let mut seen = Vec::new();

    for case in &all {
        let codec = codec_for(&schemas, case.schema);
        let sidecar = sidecar_for(case, &codec);
        let bin_path = dir.join(format!("{}.bin", case.name));
        let json_path = dir.join(format!("{}.json", case.name));
        if support::bless() {
            std::fs::write(&bin_path, &case.bytes).expect("write bin");
            let mut text = serde_json::to_string_pretty(&sidecar).expect("render sidecar");
            text.push('\n');
            std::fs::write(&json_path, text).expect("write sidecar");
        } else {
            let disk_bytes = std::fs::read(&bin_path).expect("corpus bin present");
            assert_eq!(
                disk_bytes, case.bytes,
                "case {}: bin bytes pinned",
                case.name
            );
            let disk_json: Json = serde_json::from_str(
                &std::fs::read_to_string(&json_path).expect("corpus sidecar present"),
            )
            .expect("sidecar parses");
            assert_eq!(disk_json, sidecar, "case {}: sidecar pinned", case.name);
        }
        seen.push(format!("{}.bin", case.name));
        seen.push(format!("{}.json", case.name));
    }

    if !support::bless() {
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
            .expect("corpus dir present")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .into_string()
                    .expect("name")
            })
            .collect();
        on_disk.sort();
        seen.sort();
        assert_eq!(on_disk, seen, "corpus holds exactly the table's cases");
    }
}
