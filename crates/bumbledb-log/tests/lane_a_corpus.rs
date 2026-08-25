//! The v:3 codec case table. `inventory.json` is the roster: `batch_ok`
//! and `batch_refusal` name every batch fixture, and each sidecar
//! names the outcome. Ok rows reconstruct wire bytes from the sidecar
//! header and ops; refusal rows pin the sidecar's typed identity.
//! Goldens are the oracle — this suite does not rewrite them.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::BTreeMap;

use bumbledb::RelationId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind, MAGIC, VERSION};
use serde_json::Value as Json;

fn inventory() -> Json {
    serde_json::from_str(
        &std::fs::read_to_string(support::corpus_dir().join("inventory.json")).expect("inventory"),
    )
    .expect("inventory parses")
}

fn stems(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item.as_str().expect("stem").to_string())
        .collect()
}

fn sidecar(stem: &str) -> Json {
    serde_json::from_str(
        &std::fs::read_to_string(
            support::corpus_dir()
                .join("batch")
                .join(format!("{stem}.json")),
        )
        .expect("sidecar present"),
    )
    .expect("sidecar parses")
}

fn bin_path(stem: &str) -> std::path::PathBuf {
    support::corpus_dir()
        .join("batch")
        .join(format!("{stem}.bin"))
}

fn codec_for(schemas: &BTreeMap<String, bumbledb::SchemaDescriptor>, name: &str) -> Codec {
    Codec::new(&schemas[name], support::corpus_fingerprint(name))
}

fn decimal_u64(value: &Json, label: &str) -> u64 {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{label}: decimal string"))
        .parse()
        .unwrap_or_else(|_| panic!("{label}: u64"))
}

fn parse_braid(codec: &Codec, text: &str) -> bumbledb_log::braids::BraidId {
    let raw = text
        .strip_prefix('c')
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .expect("c + hex braid");
    codec.braids().parse(raw).expect("fixture braid")
}

fn parse_header(codec: &Codec, json: &Json) -> BatchHeader {
    let prev: [u8; 32] = support::unhex(json["prev"].as_str().expect("prev"))
        .try_into()
        .expect("32-byte prev");
    BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid: parse_braid(codec, json["braid"].as_str().expect("braid")),
        braid_gen: decimal_u64(&json["braidGen"], "braidGen"),
        prev,
        writer: decimal_u64(&json["writer"], "writer"),
        timestamp: decimal_u64(&json["timestamp"], "timestamp"),
    }
}

fn parse_ops(json: &Json) -> Vec<Op> {
    json.as_array()
        .expect("ops")
        .iter()
        .map(|op| Op {
            kind: match op["kind"].as_str().expect("kind") {
                "insert" => OpKind::Insert,
                "delete" => OpKind::Delete,
                other => panic!("unknown op kind {other}"),
            },
            relation: RelationId(
                u32::try_from(op["relation"].as_u64().expect("relation")).expect("relation fits"),
            ),
            rows: op["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(|row| {
                    row.as_array()
                        .expect("row")
                        .iter()
                        .map(support::parse_value)
                        .collect()
                })
                .collect(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Intent {
    Ok,
    Refusal(String),
    EncodeRefusal(String),
}

struct Case {
    name: String,
    schema: String,
    intent: Intent,
}

/// The case table is `inventory.json`'s `batch_ok` then `batch_refusal`.
/// Each sidecar supplies the named outcome (`ok`, `refusal`, or
/// `encode-refusal` plus the typed identity).
fn case_table() -> Vec<Case> {
    let roster = inventory();
    assert_eq!(roster["version"], 3, "inventory version is 3");
    let mut cases = Vec::new();
    for name in stems(&roster["batch_ok"]) {
        let sidecar = sidecar(&name);
        assert_eq!(sidecar["expect"], "ok", "{name}: inventory ok stem");
        cases.push(Case {
            schema: sidecar["schema"].as_str().expect("schema").to_string(),
            intent: Intent::Ok,
            name,
        });
    }
    for name in stems(&roster["batch_refusal"]) {
        let sidecar = sidecar(&name);
        let refusal = sidecar["refusal"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: named refusal"))
            .to_string();
        let intent = match sidecar["expect"].as_str().expect("expect") {
            "refusal" => Intent::Refusal(refusal),
            "encode-refusal" => Intent::EncodeRefusal(refusal),
            other => panic!("{name}: unknown expect {other}"),
        };
        cases.push(Case {
            schema: sidecar["schema"].as_str().expect("schema").to_string(),
            intent,
            name,
        });
    }
    cases
}

fn json_stems_in_batch() -> Vec<String> {
    let dir = support::corpus_dir().join("batch");
    let mut stems: Vec<String> = std::fs::read_dir(&dir)
        .expect("batch dir")
        .filter_map(|entry| {
            let path = entry.expect("entry").path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json")).then(|| {
                path.file_stem()
                    .expect("stem")
                    .to_string_lossy()
                    .into_owned()
            })
        })
        .collect();
    stems.sort();
    stems
}

fn assert_ok_case(case: &Case, codec: &Codec, sidecar: &Json) {
    assert_eq!(sidecar["expect"], "ok", "{}: ok", case.name);
    assert_eq!(
        sidecar["fingerprint"].as_str().expect("fp"),
        support::hex(codec.fingerprint()),
        "{}: fingerprint",
        case.name
    );
    let header = parse_header(codec, &sidecar["header"]);
    let ops = parse_ops(&sidecar["ops"]);
    let encoded = codec
        .encode(&header, &ops)
        .unwrap_or_else(|err| panic!("{}: sidecar encodes: {err:?}", case.name));
    assert_eq!(&encoded[..4], MAGIC, "{}: magic", case.name);
    assert_eq!(
        u16::from_le_bytes([encoded[4], encoded[5]]),
        VERSION,
        "{}: wire version 3",
        case.name
    );

    let disk = std::fs::read(bin_path(&case.name)).expect("ok bin present");
    assert_eq!(
        encoded, disk,
        "{}: sidecar encodes to the golden",
        case.name
    );

    let batch = codec
        .decode(&disk)
        .unwrap_or_else(|err| panic!("{}: ok golden decodes, got {}", case.name, err.identity()));
    assert_eq!(
        support::render_header(&batch.header),
        sidecar["header"],
        "{}: header",
        case.name
    );
    assert_eq!(
        support::render_ops(&batch.ops),
        sidecar["ops"],
        "{}: ops",
        case.name
    );
    let again = codec.encode(&batch.header, &batch.ops).expect("re-encode");
    assert_eq!(again, disk, "{}: decode-encode fixpoint", case.name);
}

fn assert_refusal_case(case: &Case, codec: &Codec, sidecar: &Json, want: &str) {
    assert_eq!(sidecar["expect"], "refusal", "{}: refusal", case.name);
    assert_eq!(
        sidecar["refusal"].as_str().expect("identity"),
        want,
        "{}: sidecar names the table's refusal",
        case.name
    );
    let bytes = std::fs::read(bin_path(&case.name)).expect("refusal bin present");
    let refusal = codec
        .decode(&bytes)
        .expect_err(&format!("{}: refusal golden must refuse", case.name));
    assert_eq!(refusal.identity(), want, "{}: refusal identity", case.name);
}

fn assert_encode_refusal_case(case: &Case, sidecar: &Json, want: &str) {
    assert_eq!(
        sidecar["expect"], "encode-refusal",
        "{}: encode-refusal",
        case.name
    );
    assert_eq!(
        want, "DigestWidth",
        "{}: encode refusal identity",
        case.name
    );
    assert_eq!(
        sidecar["refusal"].as_str().expect("identity"),
        want,
        "{}: sidecar names DigestWidth",
        case.name
    );
    let prev = sidecar["header"]["prev"].as_str().expect("prev");
    assert_ne!(prev.len(), 64, "{}: short prev is not 32 bytes", case.name);
    assert!(
        !bin_path(&case.name).exists(),
        "{}: encode-only has no wire bytes",
        case.name
    );
}

#[test]
fn corpus_matches_the_case_table() {
    let schemas = support::load_schemas();
    let table = case_table();
    assert!(!table.is_empty(), "inventory names batch cases");

    let mut seen: Vec<String> = table.iter().map(|case| case.name.clone()).collect();
    seen.sort();
    assert_eq!(
        seen,
        json_stems_in_batch(),
        "inventory batch roster is the case table"
    );

    for case in &table {
        let codec = codec_for(&schemas, &case.schema);
        let sidecar = sidecar(&case.name);
        assert_eq!(
            sidecar["schema"].as_str().expect("schema"),
            case.schema,
            "{}: schema",
            case.name
        );
        match &case.intent {
            Intent::Ok => assert_ok_case(case, &codec, &sidecar),
            Intent::Refusal(want) => assert_refusal_case(case, &codec, &sidecar, want),
            Intent::EncodeRefusal(want) => assert_encode_refusal_case(case, &sidecar, want),
        }
    }
}
