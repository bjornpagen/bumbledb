//! Lane 7's Rust half of the cross-language parity goldens: the corpus
//! under `conformance/corpus` is consumed from disk as the oracle — the
//! sidecar is parsed, the binary is decoded against it, the footprint
//! is recomputed as the pure function, and the batch is re-encoded byte
//! for byte. The TypeScript suite consumes the identical files, so any
//! drift between the two codecs, footprint functions, or braid
//! derivations lands here or there as a typed disagreement.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb_log::braids::braids;
use bumbledb_log::codec::Codec;
use bumbledb_log::footprint::footprint;
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
