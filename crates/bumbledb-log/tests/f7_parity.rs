//! Lane 7's Rust half of the cross-language parity goldens: the v:3
//! inventory under `conformance/v3` is consumed from disk as the oracle
//! — the sidecar is parsed, the binary is decoded against it, and the
//! batch is re-encoded byte for byte. The TypeScript suite consumes the
//! identical files, so any drift between the two codecs or braid
//! derivations lands here or there as a typed disagreement.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb_log::apply::{Applied, ApplyRefusal, ChainCause, apply};
use bumbledb_log::braids::braids;
use bumbledb_log::codec::Codec;
use bumbledb_log::sidecar::{Chain, ChainEntry};
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

fn sidecar(section: &str, stem: &str) -> Json {
    let text = std::fs::read_to_string(
        support::corpus_dir()
            .join(section)
            .join(format!("{stem}.json")),
    )
    .expect("sidecar readable");
    serde_json::from_str(&text).expect("sidecar parses")
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
    let codec = Codec::new(&descriptor, support::corpus_fingerprint(&schema));
    assert_eq!(
        fixture["fingerprint"].as_str().expect("fingerprint"),
        support::hex(codec.fingerprint()),
        "the carried fingerprint is the corpus's own synthetic hash"
    );
    (schema, codec)
}

fn parse_braid_text(codec: &Codec, text: &str) -> bumbledb_log::braids::BraidId {
    let hex = text
        .strip_prefix('c')
        .unwrap_or_else(|| panic!("{text}: braid spelling"));
    let raw = u32::from_str_radix(hex, 16).expect("braid hex");
    codec.braids().parse(raw).expect("schema braid")
}

fn digest32(text: &str) -> [u8; 32] {
    support::unhex(text).try_into().expect("32-byte digest")
}

fn cause_name(cause: &ChainCause) -> &'static str {
    match cause {
        ChainCause::Slot { .. } => "slot",
        ChainCause::Prev { .. } => "prev",
        ChainCause::Timestamp { .. } => "timestamp",
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path =
        std::env::temp_dir().join(format!("bdb-log-f7-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Every inventory batch golden, decoded from disk against its sidecar:
/// ok fixtures must yield the sidecar's header and ops and re-encode to
/// the identical bytes; refusal fixtures must carry the sidecar's
/// typed identity; encode-only fixtures name `DigestWidth` and have no
/// wire bytes.
#[test]
fn batch_corpus_decodes_recomputes_and_reencodes() {
    let roster = inventory();
    let mut fixtures = Vec::new();
    for stem in stems(&roster["batch_ok"]) {
        fixtures.push((stem.clone(), sidecar("batch", &stem)));
    }
    for stem in stems(&roster["batch_refusal"]) {
        fixtures.push((stem.clone(), sidecar("batch", &stem)));
    }
    assert!(!fixtures.is_empty(), "the batch corpus is populated");
    for (stem, fixture) in fixtures {
        let (_, codec) = codec_of(&fixture);
        match fixture["expect"].as_str().expect("expect") {
            "ok" => {
                let bytes = corpus_bytes("batch", &stem);
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
                let reencoded = codec
                    .encode(&batch.header, &batch.ops)
                    .expect("ok fixture re-encodes");
                assert_eq!(reencoded, bytes, "{stem}: byte-exact re-encode");
            }
            "refusal" => {
                let bytes = corpus_bytes("batch", &stem);
                let refusal = codec
                    .decode(&bytes)
                    .expect_err("refusal fixture must refuse");
                assert_eq!(
                    refusal.identity(),
                    fixture["refusal"].as_str().expect("refusal identity"),
                    "{stem}: refusal identity"
                );
            }
            "encode-refusal" => {
                assert_eq!(
                    fixture["refusal"].as_str().expect("refusal identity"),
                    "DigestWidth",
                    "{stem}: encode refusal identity"
                );
                assert!(
                    !support::corpus_dir()
                        .join("batch")
                        .join(format!("{stem}.bin"))
                        .exists(),
                    "{stem}: encode-only has no wire bytes"
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
        "TrailingBytes",
    ];
    let mut refused = std::collections::BTreeSet::new();
    let mut encode_refused = std::collections::BTreeSet::new();
    let mut kinds = std::collections::BTreeSet::new();
    let mut arms = std::collections::BTreeSet::new();
    let mut scalars = std::collections::BTreeSet::new();
    let inventory = inventory();
    for stem in stems(&inventory["batch_ok"])
        .into_iter()
        .chain(stems(&inventory["batch_refusal"]))
    {
        let fixture = sidecar("batch", &stem);
        match fixture["expect"].as_str().expect("expect") {
            "refusal" => {
                refused.insert(fixture["refusal"].as_str().expect("identity").to_string());
            }
            "encode-refusal" => {
                encode_refused.insert(fixture["refusal"].as_str().expect("identity").to_string());
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
    assert!(
        encode_refused.contains("DigestWidth"),
        "corpus encode-refuses DigestWidth"
    );
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
    let dir = support::corpus_dir().join("braids");
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

/// The chain corpus from the v:3 inventory, driven whole through
/// `apply` over a fresh store: the advance fixture lands `Advanced` at
/// generation one, and every mismatch fixture surfaces `ChainMismatch`
/// carrying the sidecar's cause name, the fetched braid, the slot, and
/// the writer — the identity the TypeScript `verifyChain` is held to
/// over the same files.
#[test]
fn chain_corpus_pins_the_three_causes() {
    let schemas = support::load_schemas();
    let mut mismatch_names = std::collections::BTreeSet::new();
    for stem in stems(&inventory()["chain"]) {
        let fixture = sidecar("chain", &stem);
        let (_, codec) = codec_of(&fixture);
        let bytes = corpus_bytes("chain", &stem);
        let batch = codec
            .decode(&bytes)
            .unwrap_or_else(|err| panic!("{stem}: chain golden decodes, got {}", err.identity()));
        let again = codec.encode(&batch.header, &batch.ops).expect("re-encode");
        assert_eq!(again, bytes, "{stem}: byte-exact re-encode");

        let fetched = parse_braid_text(&codec, fixture["braid"].as_str().expect("braid"));
        let slot: u64 = fixture["slot"]
            .as_str()
            .expect("slot")
            .parse()
            .expect("u64");
        let position = ChainEntry {
            g: fixture["chain"]["g"]
                .as_str()
                .expect("g")
                .parse()
                .expect("u64"),
            prev: digest32(fixture["chain"]["prev"].as_str().expect("prev")),
            ts: fixture["chain"]["ts"]
                .as_str()
                .expect("ts")
                .parse()
                .expect("u64"),
        };
        // The chain discipline refuses before the store is touched, so
        // every mismatch case runs over a kitchen-theory scratch store;
        // only the advance case — itself a kitchen batch — writes to it.
        // (The booking fixture schema is codec vocabulary, not an
        // engine-admissible theory: its containment target carries no
        // backing key statement.)
        let db = bumbledb::Db::create(&temp_root(&stem).join("db"), schemas["kitchen"].clone())
            .expect("create")
            .expect("theory admits empty store");
        let mut chain = Chain::genesis(codec.braids());
        chain.entries_mut().insert(fetched, position);
        let applied =
            apply(&db, &mut chain, &codec, fetched, slot, &bytes).expect("apply infrastructure");
        match fixture["expect"].as_str().expect("expect") {
            "ok" => {
                assert_eq!(
                    applied,
                    Applied::Advanced { generation: 1 },
                    "{stem}: the clean chain advances"
                );
            }
            "chainMismatch" => {
                let Applied::Refused(ApplyRefusal::ChainMismatch {
                    cause,
                    braid: refused_braid,
                    slot: refused_slot,
                    writer,
                }) = applied
                else {
                    panic!("{stem}: expected a chain mismatch, got {applied:?}");
                };
                assert_eq!(
                    cause_name(&cause),
                    fixture["cause"].as_str().expect("cause"),
                    "{stem}: cause"
                );
                assert_eq!(refused_braid, fetched, "{stem}: fetched braid");
                assert_eq!(refused_slot, slot, "{stem}: slot");
                assert_eq!(
                    writer.to_string(),
                    fixture["writer"].as_str().expect("writer"),
                    "{stem}: writer"
                );
                mismatch_names.insert(fixture["cause"].as_str().expect("cause").to_string());
            }
            other => panic!("{stem}: unknown expect {other}"),
        }
    }

    assert_eq!(
        mismatch_names.into_iter().collect::<Vec<_>>(),
        ["prev", "slot", "timestamp"],
        "all three proved causes are pinned"
    );
}
