//! The v:3 conformance inventory: every ok golden decodes and
//! re-encodes byte-identically, every `r_*` golden refuses under the
//! sidecar's typed identity, and every materialised fuzz prefix is a
//! typed in-bounds refusal or a canonical fixpoint.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb_log::apply::{Applied, ApplyRefusal, ChainCause, apply};
use bumbledb_log::codec::{Codec, DecodeError, MAGIC, VERSION};
use bumbledb_log::manifest::{
    Checkpoint, CheckpointError, DOC_VERSION, Head, Manifest, ManifestError,
};
use bumbledb_log::sidecar::{Chain, ChainEntry, Pending, SidecarError};
use serde_json::Value as Json;

fn v3() -> PathBuf {
    support::corpus_dir()
}

fn inventory() -> Json {
    serde_json::from_str(&std::fs::read_to_string(v3().join("inventory.json")).expect("inventory"))
        .expect("inventory parses")
}

fn schemas() -> BTreeMap<String, bumbledb::SchemaDescriptor> {
    support::load_schemas()
}

fn codec_named(schemas: &BTreeMap<String, bumbledb::SchemaDescriptor>, name: &str) -> Codec {
    Codec::new(&schemas[name], support::corpus_fingerprint(name))
}

fn read_json(path: &Path) -> Json {
    serde_json::from_str(&std::fs::read_to_string(path).expect("json readable")).expect("json")
}

fn stems(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item.as_str().expect("stem").to_string())
        .collect()
}

fn assert_decimal_string(label: &str, value: &Json) {
    let text = value.as_str().unwrap_or_else(|| {
        panic!("{label}: target API carries a decimal string, not a JSON number")
    });
    text.parse::<u64>()
        .unwrap_or_else(|_| panic!("{label}: decimal u64"));
}

fn assert_lowercase_hex(label: &str, text: &str) {
    assert!(
        !text.is_empty() && text.len().is_multiple_of(2),
        "{label}: even-length hex"
    );
    assert!(
        text.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "{label}: lowercase hex"
    );
}

fn stems_under(rel: &str, extension: &str) -> BTreeSet<String> {
    let root = v3().join(rel);
    let mut stems = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("section present") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some(extension) {
                continue;
            }
            let rel_path = path
                .strip_prefix(v3())
                .expect("under v3")
                .with_extension("");
            stems.insert(rel_path.to_string_lossy().replace('\\', "/"));
        }
    }
    stems
}

fn json_stems_under(rel: &str) -> BTreeSet<String> {
    stems_under(rel, "json")
}

fn decimal_u64(label: &str, value: &Json) -> u64 {
    assert_decimal_string(label, value);
    value.as_str().expect("decimal").parse().expect("u64")
}

fn optional_digest(value: &Json) -> Option<[u8; 32]> {
    match value {
        Json::Null => None,
        other => Some(digest32(other.as_str().expect("digest"))),
    }
}

fn expected_manifest(value: &Json) -> Manifest {
    Manifest {
        fingerprint: digest32(value["fingerprint"].as_str().expect("fingerprint")),
        checkpoint: optional_digest(&value["checkpoint"]),
    }
}

fn expected_checkpoint(codec: &Codec, value: &Json) -> Checkpoint {
    let mut braids = BTreeMap::new();
    for (name, head) in value["braids"].as_object().expect("braids") {
        assert_decimal_string(&format!("{name}.g"), &head["g"]);
        assert_decimal_string(&format!("{name}.ts"), &head["ts"]);
        braids.insert(
            parse_braid_text(codec, name),
            Head {
                g: decimal_u64(&format!("{name}.g"), &head["g"]),
                hash: digest32(head["hash"].as_str().expect("hash")),
                ts: decimal_u64(&format!("{name}.ts"), &head["ts"]),
            },
        );
    }
    assert_decimal_string("writer", &value["writer"]);
    Checkpoint {
        braids,
        catalog: digest32(value["catalog"].as_str().expect("catalog")),
        writer: decimal_u64("writer", &value["writer"]),
        prev: optional_digest(&value["prev"]),
    }
}

fn expected_sidecar(codec: &Codec, value: &Json) -> Chain {
    let mut entries = BTreeMap::new();
    for (name, entry) in value["chain"].as_object().expect("chain") {
        entries.insert(
            parse_braid_text(codec, name),
            ChainEntry {
                g: decimal_u64(&format!("{name}.g"), &entry["g"]),
                prev: digest32(entry["prev"].as_str().expect("prev")),
                ts: decimal_u64(&format!("{name}.ts"), &entry["ts"]),
            },
        );
    }
    match &value["pending"] {
        Json::Null => Chain::Settled { entries },
        pending => {
            assert_decimal_string("pending.gen", &pending["gen"]);
            assert_lowercase_hex(
                "pending.bytes",
                pending["bytes"].as_str().expect("pending hex"),
            );
            Chain::Pending {
                entries,
                batch: Pending {
                    braid: parse_braid_text(codec, pending["braid"].as_str().expect("braid")),
                    slot: decimal_u64("pending.gen", &pending["gen"]),
                    bytes: support::unhex(pending["bytes"].as_str().expect("bytes")),
                },
            }
        }
    }
}

fn parse_braid_text(codec: &Codec, text: &str) -> bumbledb_log::braids::BraidId {
    let hex = text
        .strip_prefix('c')
        .unwrap_or_else(|| panic!("{text}: braid spelling"));
    let raw = u32::from_str_radix(hex, 16).expect("braid hex");
    codec.braids().parse(raw).expect("schema braid")
}

fn digest32(text: &str) -> [u8; 32] {
    assert_lowercase_hex("digest", text);
    support::unhex(text).try_into().expect("32-byte digest")
}

fn assert_decode_offset(name: &str, refusal: &DecodeError, len: usize) {
    if let DecodeError::Truncated { offset } | DecodeError::TrailingBytes { at: offset } = refusal {
        assert!(*offset <= len, "{name}: offset {offset} in bounds of {len}");
    }
}

fn temp_root(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path =
        std::env::temp_dir().join(format!("bdb-log-v3-{tag}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

fn cause_name(cause: &ChainCause) -> &'static str {
    match cause {
        ChainCause::Slot { .. } => "slot",
        ChainCause::Prev { .. } => "prev",
        ChainCause::Timestamp { .. } => "timestamp",
    }
}

#[test]
fn inventory_is_the_v3_roster() {
    let roster = inventory();
    assert_eq!(roster["version"], 3);
    assert_eq!(VERSION, 3);
    assert_eq!(DOC_VERSION, 3);
    assert_eq!(MAGIC, *b"BDBL");

    let batch_ok = stems(&roster["batch_ok"]);
    let batch_refusal = stems(&roster["batch_refusal"]);
    assert!(!batch_ok.is_empty(), "ok batch goldens");
    assert!(!batch_refusal.is_empty(), "refusal batch goldens");
    for stem in &batch_ok {
        assert!(stem.starts_with("ok_"), "{stem}: ok stem");
    }
    for stem in &batch_refusal {
        assert!(stem.starts_with("r_"), "{stem}: refusal stem");
    }

    let listed_batch: BTreeSet<String> = batch_ok
        .into_iter()
        .chain(batch_refusal)
        .map(|stem| format!("batch/{stem}"))
        .collect();
    assert_eq!(
        listed_batch,
        json_stems_under("batch"),
        "inventory batch roster matches the goldens"
    );
    assert!(
        v3().join("batch/r_encode_short_prev.json").exists(),
        "encode-only short prev sidecar"
    );
    assert!(
        !v3().join("batch/r_encode_short_prev.bin").exists(),
        "short prev is unconstructible as [u8; 32]"
    );

    let listed_docs: BTreeSet<String> = stems(&roster["documents"]).into_iter().collect();
    assert_eq!(
        listed_docs,
        json_stems_under("documents"),
        "inventory document roster matches the goldens"
    );
    assert_eq!(
        listed_docs,
        stems_under("documents", "bin"),
        "every document golden is binary"
    );
    assert!(
        !v3().join("chain.json").exists()
            && json_stems_under("chain")
                .iter()
                .all(|stem| stem != "chain/chain"),
        "the local sidecar name is chain, not chain.json"
    );

    let listed_chain: BTreeSet<String> = stems(&roster["chain"])
        .into_iter()
        .map(|stem| format!("chain/{stem}"))
        .collect();
    assert_eq!(
        listed_chain,
        json_stems_under("chain"),
        "inventory chain roster matches the goldens"
    );

    let listed_fuzz: BTreeSet<String> = stems(&roster["fuzz_materialised"]).into_iter().collect();
    let mut on_disk_fuzz = json_stems_under("fuzz");
    on_disk_fuzz.remove("fuzz/storm");
    assert_eq!(
        listed_fuzz, on_disk_fuzz,
        "inventory fuzz roster matches the prefixes"
    );
    assert_eq!(
        roster["fuzz_storm"].as_str().expect("storm path"),
        "fuzz/storm.json"
    );
}

#[test]
fn inventory_batch_ok_decodes_and_reencodes() {
    let schemas = schemas();
    for stem in stems(&inventory()["batch_ok"]) {
        let sidecar = read_json(&v3().join("batch").join(format!("{stem}.json")));
        let bytes = std::fs::read(v3().join("batch").join(format!("{stem}.bin"))).expect("bin");
        assert_eq!(sidecar["expect"], "ok", "{stem}");
        assert_eq!(&bytes[..4], MAGIC, "{stem}: magic");
        assert_eq!(
            u16::from_le_bytes([bytes[4], bytes[5]]),
            VERSION,
            "{stem}: wire version 3"
        );
        let schema = sidecar["schema"].as_str().expect("schema");
        let codec = codec_named(&schemas, schema);
        assert_eq!(
            sidecar["fingerprint"].as_str().expect("fp"),
            support::hex(codec.fingerprint()),
            "{stem}: fingerprint"
        );
        for field in ["braidGen", "writer", "timestamp"] {
            assert_decimal_string(&format!("{stem}.header.{field}"), &sidecar["header"][field]);
        }
        assert_lowercase_hex(
            &format!("{stem}.header.prev"),
            sidecar["header"]["prev"].as_str().expect("prev"),
        );
        let batch = codec
            .decode(&bytes)
            .unwrap_or_else(|err| panic!("{stem}: ok golden decodes, got {}", err.identity()));
        assert_eq!(
            support::render_header(&batch.header),
            sidecar["header"],
            "{stem}: header"
        );
        assert_eq!(
            support::render_ops(&batch.ops),
            sidecar["ops"],
            "{stem}: ops"
        );
        let again = codec.encode(&batch.header, &batch.ops).expect("re-encode");
        assert_eq!(again, bytes, "{stem}: byte-exact re-encode");
    }
}

#[test]
fn inventory_batch_refusal_matches_the_named_identity() {
    let schemas = schemas();
    let mut mismatches = Vec::new();
    for stem in stems(&inventory()["batch_refusal"]) {
        let sidecar = read_json(&v3().join("batch").join(format!("{stem}.json")));
        let expect = sidecar["expect"].as_str().expect("expect");
        if expect == "encode-refusal" {
            assert_eq!(sidecar["refusal"], "DigestWidth", "{stem}");
            let prev = sidecar["header"]["prev"].as_str().expect("prev");
            assert_ne!(prev.len(), 64, "{stem}: short prev is not 32 bytes");
            assert!(
                !v3().join("batch").join(format!("{stem}.bin")).exists(),
                "{stem}: encode-only has no wire bytes"
            );
            continue;
        }
        assert_eq!(expect, "refusal", "{stem}");
        let bytes = std::fs::read(v3().join("batch").join(format!("{stem}.bin"))).expect("bin");
        let schema = sidecar["schema"].as_str().expect("schema");
        let codec = codec_named(&schemas, schema);
        let refusal = codec
            .decode(&bytes)
            .expect_err("{stem}: refusal golden must refuse");
        let want = sidecar["refusal"].as_str().expect("identity");
        if refusal.identity() != want {
            mismatches.push(format!("{stem}: got {}, want {want}", refusal.identity()));
        }
        assert_decode_offset(&stem, &refusal, bytes.len());
    }
    assert!(
        mismatches.is_empty(),
        "typed refusals:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn inventory_documents_parse_and_rerender() {
    let schemas = schemas();
    for rel in stems(&inventory()["documents"]) {
        let sidecar = read_json(&v3().join(format!("{rel}.json")));
        let bytes = std::fs::read(v3().join(format!("{rel}.bin"))).expect("document bytes");
        if let Some(hex) = sidecar.get("hex").and_then(Json::as_str) {
            assert_eq!(
                support::unhex(hex),
                bytes,
                "{rel}: inventory hex is the golden"
            );
        }
        assert_ne!(
            bytes.first().copied(),
            Some(b'{'),
            "{rel}: document golden is binary, not JSON"
        );
        let kind = sidecar["kind"].as_str().expect("kind");
        match sidecar["expect"].as_str().expect("expect") {
            "ok" => match kind {
                "manifest" => {
                    let parsed = Manifest::parse(&bytes).expect("manifest parses");
                    assert_eq!(parsed.render(), bytes, "{rel}: manifest fixpoint");
                    assert_eq!(
                        parsed,
                        expected_manifest(&sidecar["value"]),
                        "{rel}: manifest value"
                    );
                }
                "checkpoint" => {
                    let schema = sidecar["schema"].as_str().expect("schema");
                    let codec = codec_named(&schemas, schema);
                    let parsed =
                        Checkpoint::parse(&bytes, codec.braids()).expect("checkpoint parses");
                    assert_eq!(parsed.render(), bytes, "{rel}: checkpoint fixpoint");
                    assert_eq!(
                        parsed,
                        expected_checkpoint(&codec, &sidecar["value"]),
                        "{rel}: checkpoint value"
                    );
                }
                "sidecar" => {
                    let schema = sidecar["schema"].as_str().expect("schema");
                    let codec = codec_named(&schemas, schema);
                    let parsed = Chain::parse(&bytes, codec.braids()).expect("sidecar parses");
                    assert_eq!(parsed.render(), bytes, "{rel}: sidecar fixpoint");
                    assert_eq!(
                        parsed,
                        expected_sidecar(&codec, &sidecar["value"]),
                        "{rel}: sidecar value"
                    );
                }
                other => panic!("{rel}: unknown kind {other}"),
            },
            "refusal" => {
                let want = sidecar["refusal"].as_str().expect("refusal");
                let got = match kind {
                    "manifest" => Manifest::parse(&bytes)
                        .expect_err("manifest refuses")
                        .identity()
                        .to_string(),
                    "checkpoint" => {
                        let schema = sidecar["schema"].as_str().expect("schema");
                        let codec = codec_named(&schemas, schema);
                        Checkpoint::parse(&bytes, codec.braids())
                            .expect_err("checkpoint refuses")
                            .identity()
                            .to_string()
                    }
                    "sidecar" => {
                        let schema = sidecar["schema"].as_str().expect("schema");
                        let codec = codec_named(&schemas, schema);
                        Chain::parse(&bytes, codec.braids())
                            .expect_err("sidecar refuses")
                            .identity()
                            .to_string()
                    }
                    other => panic!("{rel}: unknown kind {other}"),
                };
                assert_eq!(got, want, "{rel}: document refusal");
            }
            other => panic!("{rel}: unknown expect {other}"),
        }
    }
}

#[test]
fn inventory_fuzz_prefixes_refuse_by_name() {
    let schemas = schemas();
    for rel in stems(&inventory()["fuzz_materialised"]) {
        let sidecar = read_json(&v3().join(format!("{rel}.json")));
        let bytes = std::fs::read(v3().join(format!("{rel}.bin"))).expect("fuzz bytes");
        let want = sidecar["refusal"].as_str();
        match sidecar["expect"].as_str().expect("expect") {
            "ok" => {
                if rel.contains("/batch/") {
                    let schema = sidecar["schema"].as_str().expect("schema");
                    let codec = codec_named(&schemas, schema);
                    let batch = codec.decode(&bytes).expect("accepted mutant decodes");
                    let again = codec
                        .encode(&batch.header, &batch.ops)
                        .expect("accepted mutant re-encodes");
                    assert_eq!(again, bytes, "{rel}: accepted mutant is a fixpoint");
                } else if rel.contains("manifest_") {
                    let parsed = Manifest::parse(&bytes).expect("accepted manifest");
                    assert_eq!(parsed.render(), bytes, "{rel}: manifest fixpoint");
                } else if rel.contains("checkpoint_") {
                    let kitchen = codec_named(&schemas, "kitchen");
                    let parsed =
                        Checkpoint::parse(&bytes, kitchen.braids()).expect("accepted checkpoint");
                    assert_eq!(parsed.render(), bytes, "{rel}: checkpoint fixpoint");
                } else if rel.contains("sidecar_") {
                    let kitchen = codec_named(&schemas, "kitchen");
                    let parsed = Chain::parse(&bytes, kitchen.braids()).expect("accepted sidecar");
                    assert_eq!(parsed.render(), bytes, "{rel}: sidecar fixpoint");
                } else {
                    panic!("{rel}: unknown fuzz kind");
                }
            }
            "refusal" => {
                let identity = want.expect("named refusal");
                if rel.contains("/batch/") {
                    let schema = sidecar
                        .get("schema")
                        .and_then(Json::as_str)
                        .unwrap_or("kitchen");
                    let codec = codec_named(&schemas, schema);
                    let refusal = codec.decode(&bytes).expect_err("{rel}: prefix refuses");
                    assert_eq!(refusal.identity(), identity, "{rel}: refusal identity");
                    assert_decode_offset(&rel, &refusal, bytes.len());
                    continue;
                }
                if rel.contains("manifest_") {
                    let error = Manifest::parse(&bytes).expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    if let ManifestError::Malformed { at } = error {
                        assert!(at <= bytes.len(), "{rel}: offset in bounds");
                    }
                    continue;
                }
                let kitchen = codec_named(&schemas, "kitchen");
                if rel.contains("checkpoint_") {
                    let error = Checkpoint::parse(&bytes, kitchen.braids())
                        .expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    if let CheckpointError::Malformed { at } = error {
                        assert!(at <= bytes.len(), "{rel}: offset in bounds");
                    }
                    continue;
                }
                if rel.contains("sidecar_") {
                    let error =
                        Chain::parse(&bytes, kitchen.braids()).expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    if let SidecarError::Malformed { at } = error {
                        assert!(at <= bytes.len(), "{rel}: offset in bounds");
                    }
                    continue;
                }
                panic!("{rel}: unknown fuzz kind");
            }
            other => panic!("{rel}: unknown expect {other}"),
        }
    }
}

#[test]
fn inventory_chain_goldens_decode_and_verify() {
    let schemas = schemas();
    for stem in stems(&inventory()["chain"]) {
        let sidecar = read_json(&v3().join("chain").join(format!("{stem}.json")));
        let bytes = std::fs::read(v3().join("chain").join(format!("{stem}.bin"))).expect("bin");
        let schema = sidecar["schema"].as_str().expect("schema");
        let codec = codec_named(&schemas, schema);
        assert_eq!(
            sidecar["fingerprint"].as_str().expect("fp"),
            support::hex(codec.fingerprint()),
            "{stem}: fingerprint"
        );
        assert_decimal_string(&format!("{stem}.slot"), &sidecar["slot"]);
        assert_decimal_string(&format!("{stem}.chain.g"), &sidecar["chain"]["g"]);
        assert_decimal_string(&format!("{stem}.chain.ts"), &sidecar["chain"]["ts"]);
        let batch = codec
            .decode(&bytes)
            .unwrap_or_else(|err| panic!("{stem}: chain golden decodes, got {}", err.identity()));
        let again = codec.encode(&batch.header, &batch.ops).expect("re-encode");
        assert_eq!(again, bytes, "{stem}: byte-exact re-encode");

        let fetched = parse_braid_text(&codec, sidecar["braid"].as_str().expect("braid"));
        let slot: u64 = sidecar["slot"]
            .as_str()
            .expect("slot")
            .parse()
            .expect("u64");
        let position = ChainEntry {
            g: sidecar["chain"]["g"]
                .as_str()
                .expect("g")
                .parse()
                .expect("u64"),
            prev: digest32(sidecar["chain"]["prev"].as_str().expect("prev")),
            ts: sidecar["chain"]["ts"]
                .as_str()
                .expect("ts")
                .parse()
                .expect("u64"),
        };
        let db = bumbledb::Db::create(&temp_root(&stem).join("db"), schemas["kitchen"].clone())
            .expect("create")
            .expect("theory admits empty store");
        let mut chain = Chain::genesis(codec.braids());
        chain.entries_mut().insert(fetched, position);
        let applied =
            apply(&db, &mut chain, &codec, fetched, slot, &bytes).expect("apply infrastructure");
        match sidecar["expect"].as_str().expect("expect") {
            "ok" => {
                assert_eq!(
                    applied,
                    Applied::Advanced { generation: 1 },
                    "{stem}: chain advances"
                );
            }
            "chainMismatch" => {
                let Applied::Refused(ApplyRefusal::ChainMismatch {
                    cause,
                    braid,
                    slot: refused_slot,
                    writer,
                }) = applied
                else {
                    panic!("{stem}: expected chain mismatch, got {applied:?}");
                };
                assert_eq!(
                    cause_name(&cause),
                    sidecar["cause"].as_str().expect("cause"),
                    "{stem}: cause"
                );
                assert_eq!(braid, fetched, "{stem}: fetched braid");
                assert_eq!(refused_slot, slot, "{stem}: slot");
                assert_eq!(
                    writer.to_string(),
                    sidecar["writer"].as_str().expect("writer"),
                    "{stem}: writer"
                );
            }
            other => panic!("{stem}: unknown expect {other}"),
        }
    }
}

#[test]
fn inventory_storm_recipe_is_pinned() {
    let storm = read_json(&v3().join("fuzz/storm.json"));
    assert_eq!(storm["prng"]["name"], "XorShift64");
    assert_ne!(
        storm["goldens"]["batch"].as_array().expect("batch").len(),
        0
    );
    assert_ne!(storm["operators"].as_array().expect("operators").len(), 0);
}
