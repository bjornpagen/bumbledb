//! Lane 7's Rust half of the cross-language parity goldens: the corpus
//! under `conformance/corpus` is consumed from disk as the oracle — the
//! sidecar is parsed, the binary is decoded against it, the footprint
//! is recomputed as the pure function, and the batch is re-encoded byte
//! for byte. The TypeScript suite consumes the identical files, so any
//! drift between the two codecs, footprint functions, or braid
//! derivations lands here or there as a typed disagreement.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb::{Interval, Value};
use bumbledb_log::apply::{Applied, ApplyRefusal, ChainCause, apply};
use bumbledb_log::braids::braids;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::footprint::footprint;
use bumbledb_log::sidecar::{Chain, ChainEntry};
use serde_json::Value as Json;

fn corpus_files(section: &str) -> Vec<(String, Json)> {
    let dir = support::corpus_dir().join(section);
    let mut fixtures: Vec<(String, Json)> = std::fs::read_dir(&dir)
        .expect("corpus section present")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("name")
        })
        .filter_map(|name| {
            let stem = name.strip_suffix(".json")?;
            let text = std::fs::read_to_string(dir.join(&name)).expect("sidecar readable");
            let json: Json = serde_json::from_str(&text).expect("sidecar parses");
            Some((stem.to_string(), json))
        })
        .collect();
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

fn corpus_bytes(section: &str, stem: &str) -> Vec<u8> {
    std::fs::read(
        support::corpus_dir()
            .join(section)
            .join(format!("{stem}.bin")),
    )
    .expect("corpus bin present")
}

fn codec_of(fixture: &Json) -> (String, Codec) {
    let schema = fixture["schema"].as_str().expect("schema name").to_string();
    let descriptor = support::schema(&schema);
    let codec =
        Codec::new(&descriptor, support::corpus_fingerprint(&schema)).expect("fixture vocabulary");
    assert_eq!(
        fixture["fingerprint"].as_str().expect("fingerprint"),
        support::hex(codec.fingerprint()),
        "the carried fingerprint is the corpus's own synthetic hash"
    );
    (schema, codec)
}

/// Every batch golden, decoded from disk against its sidecar: ok
/// fixtures must yield the sidecar's header, ops, and footprint, agree
/// with the pure recomputation, and re-encode to the identical bytes;
/// refusal fixtures must carry the sidecar's typed identity.
#[test]
fn batch_corpus_decodes_recomputes_and_reencodes() {
    let fixtures = corpus_files("batch");
    assert!(!fixtures.is_empty(), "the batch corpus is populated");
    for (stem, fixture) in fixtures {
        let (_, codec) = codec_of(&fixture);
        let bytes = corpus_bytes("batch", &stem);
        match fixture["expect"].as_str().expect("expect") {
            "ok" => {
                let batch = codec.decode(&bytes).expect("ok fixture decodes");
                assert_eq!(
                    support::render_header(&batch.header),
                    fixture["header"],
                    "{stem}: header"
                );
                assert_eq!(
                    support::render_ops(&batch.ops),
                    fixture["ops"],
                    "{stem}: ops"
                );
                assert_eq!(
                    Json::Array(batch.footprint.iter().map(support::render_entry).collect()),
                    fixture["footprint"],
                    "{stem}: carried footprint section"
                );
                let recomputed =
                    footprint(codec.vocabulary(), &batch.ops).expect("footprint recomputes");
                assert_eq!(
                    recomputed, batch.footprint,
                    "{stem}: the pure function reproduces the carried section"
                );
                let reencoded = codec
                    .encode(&batch.header, &batch.ops)
                    .expect("ok fixture re-encodes");
                assert_eq!(reencoded, bytes, "{stem}: byte-exact re-encode");
            }
            "refusal" => {
                let refusal = codec
                    .decode(&bytes)
                    .expect_err("refusal fixture must refuse");
                assert_eq!(
                    refusal.identity(),
                    fixture["refusal"].as_str().expect("refusal identity"),
                    "{stem}: refusal identity"
                );
            }
            other => panic!("{stem}: unknown expectation {other}"),
        }
    }
}

/// The corpus coverage census: every decode refusal identity, both op
/// kinds, every sidecar value arm, and the numeric boundary values must
/// each appear in at least one golden — a shrunk corpus fails here
/// before it silently stops exercising a lane of the wire.
#[test]
fn batch_corpus_covers_the_wire() {
    let roster = [
        "Truncated",
        "BadMagic",
        "Version",
        "Flags",
        "FingerprintMismatch",
        "UnknownBraid",
        "UnknownOpKind",
        "UnknownRelation",
        "ClosedRelation",
        "OpRelationOutsideBraid",
        "TagMismatch",
        "BoolByte",
        "InvalidUtf8",
        "EmptyInterval",
        "IntervalOverflow",
        "UnknownFootprintClass",
        "UnknownFootprintMode",
        "UnsortedFootprint",
        "DuplicateFootprintEntry",
        "TrailingBytes",
    ];
    let mut refused = std::collections::BTreeSet::new();
    let mut kinds = std::collections::BTreeSet::new();
    let mut arms = std::collections::BTreeSet::new();
    let mut scalars = std::collections::BTreeSet::new();
    for (stem, fixture) in corpus_files("batch") {
        match fixture["expect"].as_str().expect("expect") {
            "refusal" => {
                refused.insert(fixture["refusal"].as_str().expect("identity").to_string());
            }
            "ok" => {
                for op in fixture["ops"].as_array().expect("ops") {
                    kinds.insert(op["kind"].as_str().expect("kind").to_string());
                    for row in op["rows"].as_array().expect("rows") {
                        for value in row.as_array().expect("row") {
                            let object = value.as_object().expect("value object");
                            let (arm, body) = object.iter().next().expect("one arm");
                            arms.insert(arm.clone());
                            if let Some(text) = body.as_str() {
                                scalars.insert(text.to_string());
                            }
                        }
                    }
                }
            }
            other => panic!("{stem}: unknown expectation {other}"),
        }
    }
    for identity in roster {
        assert!(refused.contains(identity), "corpus refuses {identity}");
    }
    for identity in &refused {
        assert!(
            roster.contains(&identity.as_str()),
            "sidecar identity {identity} is on the decoder's roster"
        );
    }
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        ["delete", "insert"],
        "both op kinds appear"
    );
    for arm in [
        "bool",
        "u64",
        "i64",
        "string",
        "fixedBytes",
        "intervalU64",
        "intervalI64",
    ] {
        assert!(
            arms.contains(arm),
            "value arm {arm} appears in an ok golden"
        );
    }
    for boundary in [
        "18446744073709551615",
        "-9223372036854775808",
        "9223372036854775807",
    ] {
        assert!(
            scalars.contains(boundary),
            "boundary value {boundary} appears in an ok golden"
        );
    }
}

/// Every braid golden, derived from the shared schema file and compared
/// against the checked-in component map and serial-at roster the
/// TypeScript derivation is held to.
#[test]
fn braids_corpus_matches_the_derivation() {
    let schemas = support::load_schemas();
    let fixtures = corpus_files("braids");
    assert_eq!(
        fixtures.len(),
        schemas.len(),
        "one braid golden per fixture schema"
    );
    for (stem, fixture) in fixtures {
        let schema = fixture["schema"].as_str().expect("schema name");
        assert_eq!(schema, stem, "the golden is named for its schema");
        let derived = braids(&schemas[schema]);
        let components: serde_json::Map<String, Json> = derived
            .components()
            .into_iter()
            .map(|(braid, relations)| {
                (
                    braid.to_string(),
                    Json::Array(
                        relations
                            .into_iter()
                            .map(|relation| Json::from(relation.0))
                            .collect(),
                    ),
                )
            })
            .collect();
        assert_eq!(
            Json::Object(components),
            fixture["braids"],
            "{stem}: braid map"
        );
        assert_eq!(
            Json::Array(
                derived
                    .serial_at()
                    .iter()
                    .map(|statement| Json::from(statement.0))
                    .collect()
            ),
            fixture["serialAt"],
            "{stem}: serial-at statements"
        );
    }
}

/// A chain golden: batch bytes beside the fetch context — the key the
/// object came from and the replica's chain position — and the verdict
/// the chain discipline must pronounce, shared with the TypeScript
/// suite through the sidecar's cause names.
struct ChainCase {
    name: &'static str,
    schema: &'static str,
    fetched: u32,
    slot: u64,
    position: ChainEntry,
    bytes: Vec<u8>,
    expect: ChainExpect,
}

enum ChainExpect {
    Advance,
    Cause(&'static str),
}

fn chain_header(
    codec: &Codec,
    braid_raw: u32,
    slot_gen: u64,
    prev: [u8; 32],
    ts: u64,
) -> BatchHeader {
    BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid: codec.braids().parse(braid_raw).expect("fixture braid"),
        braid_gen: slot_gen,
        prev,
        writer: 7,
        timestamp: ts,
    }
}

fn kitchen_insert() -> Op {
    Op {
        kind: OpKind::Insert,
        relation: bumbledb::RelationId(0),
        rows: vec![Box::from([
            Value::Bool(true),
            Value::U64(11),
            Value::I64(-4),
            Value::String("chained".into()),
            Value::FixedBytes(Box::from([7, 8, 9])),
            Value::IntervalU64(Interval::new(3, 9).expect("interval")),
            Value::IntervalI64(Interval::new(-2, 2).expect("interval")),
            Value::IntervalU64(Interval::fixed(20u64, 5).expect("fixed interval")),
            Value::IntervalI64(Interval::fixed(-20i64, 5).expect("fixed interval")),
        ])],
    }
}

fn audit_insert() -> Op {
    Op {
        kind: OpKind::Insert,
        relation: bumbledb::RelationId(3),
        rows: vec![Box::from([Value::U64(1), Value::String("ledger".into())])],
    }
}

const CHAIN_TS: u64 = 1_755_801_600_000;

/// The chain case table: one advance and the three proved mismatch
/// causes, the slot cause split into its generation and braid halves —
/// the braid half is the wrong-key object a hostile or confused store
/// could serve, refused before any apply on both implementations.
fn chain_cases(kitchen: &Codec, booking: &Codec) -> Vec<ChainCase> {
    vec![
        ChainCase {
            name: "ok_chain_advance",
            schema: "kitchen",
            fetched: 0,
            slot: 1,
            position: ChainEntry::GENESIS,
            bytes: kitchen
                .encode(
                    &chain_header(kitchen, 0, 1, [0u8; 32], CHAIN_TS),
                    &[kitchen_insert()],
                )
                .expect("encode"),
            expect: ChainExpect::Advance,
        },
        ChainCase {
            name: "r_chain_prev",
            schema: "kitchen",
            fetched: 0,
            slot: 1,
            position: ChainEntry::GENESIS,
            bytes: kitchen
                .encode(
                    &chain_header(kitchen, 0, 1, [0x55; 32], CHAIN_TS),
                    &[kitchen_insert()],
                )
                .expect("encode"),
            expect: ChainExpect::Cause("prev"),
        },
        ChainCase {
            name: "r_chain_slot_gen",
            schema: "kitchen",
            fetched: 0,
            slot: 1,
            position: ChainEntry::GENESIS,
            bytes: kitchen
                .encode(
                    &chain_header(kitchen, 0, 2, [0u8; 32], CHAIN_TS),
                    &[kitchen_insert()],
                )
                .expect("encode"),
            expect: ChainExpect::Cause("slot"),
        },
        ChainCase {
            name: "r_chain_slot_braid",
            schema: "booking",
            fetched: 0,
            slot: 1,
            position: ChainEntry::GENESIS,
            bytes: booking
                .encode(
                    &chain_header(booking, 3, 1, [0u8; 32], CHAIN_TS),
                    &[audit_insert()],
                )
                .expect("encode"),
            expect: ChainExpect::Cause("slot"),
        },
        ChainCase {
            name: "r_chain_timestamp",
            schema: "kitchen",
            fetched: 0,
            slot: 2,
            position: ChainEntry {
                g: 1,
                prev: [0x66; 32],
                ts: 200_000,
            },
            bytes: kitchen
                .encode(
                    &chain_header(kitchen, 0, 2, [0x66; 32], 100_000),
                    &[kitchen_insert()],
                )
                .expect("encode"),
            expect: ChainExpect::Cause("timestamp"),
        },
    ]
}

fn chain_sidecar(case: &ChainCase, codec: &Codec) -> Json {
    let braid = codec.braids().parse(case.fetched).expect("fetched braid");
    let mut sidecar = serde_json::json!({
        "schema": case.schema,
        "fingerprint": support::hex(codec.fingerprint()),
        "braid": braid.to_string(),
        "slot": case.slot.to_string(),
        "chain": {
            "g": case.position.g.to_string(),
            "prev": support::hex(&case.position.prev),
            "ts": case.position.ts.to_string(),
        },
    });
    let object = sidecar.as_object_mut().expect("object");
    match case.expect {
        ChainExpect::Advance => {
            object.insert("expect".into(), Json::String("ok".into()));
        }
        ChainExpect::Cause(label) => {
            object.insert("expect".into(), Json::String("chainMismatch".into()));
            object.insert("cause".into(), Json::String(label.into()));
            object.insert("writer".into(), Json::String("7".into()));
        }
    }
    sidecar
}

fn cause_name(cause: &ChainCause) -> &'static str {
    match cause {
        ChainCause::Slot { .. } => "slot",
        ChainCause::Prev { .. } => "prev",
        ChainCause::Timestamp { .. } => "timestamp",
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("f7_parity_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Writes the fixture pair under bless, or holds the disk files to the
/// case table byte for byte otherwise.
fn pin_chain_fixture(dir: &std::path::Path, case: &ChainCase, sidecar: &Json) {
    let bin_path = dir.join(format!("{}.bin", case.name));
    let json_path = dir.join(format!("{}.json", case.name));
    if support::bless() {
        std::fs::write(&bin_path, &case.bytes).expect("write bin");
        let mut text = serde_json::to_string_pretty(sidecar).expect("render sidecar");
        text.push('\n');
        std::fs::write(&json_path, text).expect("write sidecar");
        return;
    }
    assert_eq!(
        std::fs::read(&bin_path).expect("chain bin present"),
        case.bytes,
        "case {}: bin bytes pinned",
        case.name
    );
    let disk: Json =
        serde_json::from_str(&std::fs::read_to_string(&json_path).expect("chain sidecar present"))
            .expect("sidecar parses");
    assert_eq!(&disk, sidecar, "case {}: sidecar pinned", case.name);
}

/// The chain corpus, generated from the case table and pinned to disk
/// like the batch corpus, then driven whole through `apply` over a
/// fresh store: the advance fixture lands `Advanced` at generation one,
/// and every mismatch fixture surfaces `ChainMismatch` carrying the
/// sidecar's cause name, the fetched braid, the slot, and the writer —
/// the identity the TypeScript `verifyChain` is held to over the same
/// files.
#[test]
fn chain_corpus_pins_the_three_causes() {
    let schemas = support::load_schemas();
    let dir = support::corpus_dir().join("chain");
    if support::bless() {
        std::fs::create_dir_all(&dir).expect("chain dir");
    }
    let kitchen =
        Codec::new(&schemas["kitchen"], support::corpus_fingerprint("kitchen")).expect("codec");
    let booking =
        Codec::new(&schemas["booking"], support::corpus_fingerprint("booking")).expect("codec");

    let cases = chain_cases(&kitchen, &booking);
    let mut seen = Vec::new();
    let mut mismatch_names = std::collections::BTreeSet::new();
    for case in &cases {
        let codec = if case.schema == "kitchen" {
            &kitchen
        } else {
            &booking
        };
        let sidecar = chain_sidecar(case, codec);
        pin_chain_fixture(&dir, case, &sidecar);
        seen.push(format!("{}.bin", case.name));
        seen.push(format!("{}.json", case.name));

        let braid = codec.braids().parse(case.fetched).expect("fetched braid");
        // The chain discipline refuses before the store is touched, so
        // every mismatch case runs over a kitchen-theory scratch store;
        // only the advance case — itself a kitchen batch — writes to it.
        // (The booking fixture schema is codec vocabulary, not an
        // engine-admissible theory: its containment target carries no
        // backing key statement.)
        let db = bumbledb::Db::create(&temp_root(case.name).join("db"), schemas["kitchen"].clone())
            .expect("create")
            .expect("theory admits empty store");
        let mut chain = Chain::genesis(codec.braids());
        chain.entries.insert(braid, case.position);
        let applied = apply(&db, &mut chain, codec, braid, case.slot, &case.bytes, 0)
            .expect("apply infrastructure");
        match case.expect {
            ChainExpect::Advance => {
                assert_eq!(
                    applied,
                    Applied::Advanced { generation: 1 },
                    "case {}: the clean chain advances",
                    case.name
                );
            }
            ChainExpect::Cause(expected) => {
                let Applied::Refused(ApplyRefusal::ChainMismatch {
                    cause,
                    braid: refused_braid,
                    slot,
                    writer,
                }) = applied
                else {
                    panic!(
                        "case {}: expected a chain mismatch, got {applied:?}",
                        case.name
                    );
                };
                assert_eq!(cause_name(&cause), expected, "case {}: cause", case.name);
                assert_eq!(refused_braid, braid, "case {}: fetched braid", case.name);
                assert_eq!(slot, case.slot, "case {}: slot", case.name);
                assert_eq!(writer, 7, "case {}: writer", case.name);
                mismatch_names.insert(expected);
            }
        }
    }

    assert_eq!(
        mismatch_names.into_iter().collect::<Vec<_>>(),
        ["prev", "slot", "timestamp"],
        "all three proved causes are pinned"
    );
    if !support::bless() {
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
            .expect("chain dir present")
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
        assert_eq!(
            on_disk, seen,
            "chain corpus holds exactly the table's cases"
        );
    }
}
