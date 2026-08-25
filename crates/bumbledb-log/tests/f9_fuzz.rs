//! Lane 9 — fuzz. The v:3 inventory's storm recipe (`fuzz/storm.json`)
//! drives the mutation harness over the listed goldens: no panic, no
//! overflow, every refusal a named identity with any carried offset in
//! bounds, and trailing bytes landing at the accepted prefix's end.
//! An accepted mutant is a canonical fixpoint.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use bumbledb_log::codec::{Codec, DecodeError};
use bumbledb_log::manifest::{Checkpoint, CheckpointError, Manifest, ManifestError};
use bumbledb_log::sidecar::{Chain, SidecarError};
use serde_json::Value as Json;

fn v3() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("v3")
}

fn read_json(path: &Path) -> Json {
    serde_json::from_str(&std::fs::read_to_string(path).expect("json readable")).expect("json")
}

fn storm() -> Json {
    read_json(&v3().join("fuzz/storm.json"))
}

fn inventory() -> Json {
    read_json(&v3().join("inventory.json"))
}

fn hex_u64(text: &str) -> u64 {
    let text = text.strip_prefix("0x").unwrap_or(text);
    u64::from_str_radix(text, 16).expect("hex u64")
}

fn storm_seed(storm: &Json, name: &str) -> u64 {
    hex_u64(storm["prng"]["seeds"][name].as_str().expect("seed"))
}

fn storm_iters(storm: &Json, name: &str) -> usize {
    usize::try_from(storm["prng"][name].as_u64().expect("iters")).expect("iters fit usize")
}

fn codec_named(name: &str) -> Codec {
    Codec::new(&support::schema(name), support::corpus_fingerprint(name))
}

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound.max(1)).expect("bound fits u64"))
            .expect("index fits usize")
    }

    fn byte(&mut self) -> u8 {
        u8::try_from(self.next() % 256).expect("byte")
    }
}

struct BatchGolden {
    name: String,
    codec: Codec,
    bytes: Vec<u8>,
}

fn batch_goldens(storm: &Json) -> Vec<BatchGolden> {
    storm["goldens"]["batch"]
        .as_array()
        .expect("batch goldens")
        .iter()
        .map(|item| {
            let rel = item.as_str().expect("rel");
            let sidecar = read_json(&v3().join(format!("{rel}.json")));
            assert_eq!(sidecar["expect"], "ok", "{rel}: storm lists ok goldens");
            let schema = sidecar["schema"].as_str().expect("schema");
            BatchGolden {
                name: rel.to_string(),
                codec: codec_named(schema),
                bytes: std::fs::read(v3().join(format!("{rel}.bin"))).expect("bin"),
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
enum DocKind {
    Manifest,
    Checkpoint,
    Sidecar,
}

struct DocGolden {
    name: String,
    kind: DocKind,
    codec: Option<Codec>,
    bytes: Vec<u8>,
}

fn doc_goldens(storm: &Json, section: &str, kind: DocKind) -> Vec<DocGolden> {
    storm["goldens"][section]
        .as_array()
        .expect("document goldens")
        .iter()
        .map(|item| {
            let rel = item.as_str().expect("rel");
            let sidecar = read_json(&v3().join(format!("{rel}.json")));
            assert_eq!(sidecar["expect"], "ok", "{rel}: storm lists ok goldens");
            let codec = sidecar
                .get("schema")
                .and_then(Json::as_str)
                .map(codec_named);
            DocGolden {
                name: rel.to_string(),
                kind,
                codec,
                bytes: std::fs::read(v3().join(format!("{rel}.bin"))).expect("bin"),
            }
        })
        .collect()
}

/// The general mutation operator set the storm recipe names: byte
/// flips, a truncation, a chunk insertion, a chunk deletion, or a
/// hostile all-ones u32 spliced over four bytes.
fn storm_mutant(prng: &mut XorShift, bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    match prng.next() % 6 {
        0 | 1 => {
            for _ in 0..=prng.below(4) {
                if out.is_empty() {
                    break;
                }
                let at = prng.below(out.len());
                out[at] ^= u8::try_from(prng.next() % 255 + 1).expect("byte");
            }
        }
        2 => {
            let len = prng.below(out.len() + 1);
            out.truncate(len);
        }
        3 => {
            let at = prng.below(out.len() + 1);
            let chunk: Vec<u8> = (0..=prng.below(16)).map(|_| prng.byte()).collect();
            out.splice(at..at, chunk);
        }
        4 => {
            if !out.is_empty() {
                let at = prng.below(out.len());
                let end = (at + 1 + prng.below(16)).min(out.len());
                out.drain(at..end);
            }
        }
        _ => {
            if out.len() >= 4 {
                let at = prng.below(out.len() - 3);
                out[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            }
        }
    }
    out
}

fn assert_typed_in_bounds(name: &str, refusal: &DecodeError, len: usize) {
    assert!(!refusal.identity().is_empty(), "{name}: named identity");
    match refusal {
        DecodeError::Truncated { offset } => {
            assert!(*offset <= len, "{name}: truncation offset in bounds");
        }
        DecodeError::TrailingBytes { at } => {
            assert!(*at <= len, "{name}: trailing offset in bounds");
        }
        _ => {}
    }
}

fn assert_manifest_in_bounds(name: &str, error: ManifestError, len: usize) {
    assert!(!error.identity().is_empty(), "{name}: named identity");
    if let ManifestError::Malformed { at } = error {
        assert!(at <= len, "{name}: offset in bounds");
    }
}

fn assert_checkpoint_in_bounds(name: &str, error: CheckpointError, len: usize) {
    assert!(!error.identity().is_empty(), "{name}: named identity");
    if let CheckpointError::Malformed { at } = error {
        assert!(at <= len, "{name}: offset in bounds");
    }
}

fn assert_sidecar_in_bounds(name: &str, error: SidecarError, len: usize) {
    assert!(!error.identity().is_empty(), "{name}: named identity");
    if let SidecarError::Malformed { at } = error {
        assert!(at <= len, "{name}: offset in bounds");
    }
}

fn soup_from(prng: &mut XorShift, golden: &[u8], alphabet: &[u8], extra_cap: usize) -> Vec<u8> {
    let cut = prng.below(golden.len() + 1);
    let mut soup = golden[..cut].to_vec();
    for _ in 0..prng.below(extra_cap) {
        let byte = if alphabet.is_empty() || prng.next().is_multiple_of(4) {
            prng.byte()
        } else {
            alphabet[prng.below(alphabet.len())]
        };
        soup.push(byte);
    }
    soup
}

fn parse_doc(golden: &DocGolden, bytes: &[u8]) -> Result<(), DocRefusal> {
    match golden.kind {
        DocKind::Manifest => Manifest::parse(bytes)
            .map(|_| ())
            .map_err(DocRefusal::Manifest),
        DocKind::Checkpoint => {
            Checkpoint::parse(bytes, golden.codec.as_ref().expect("schema").braids())
                .map(|_| ())
                .map_err(DocRefusal::Checkpoint)
        }
        DocKind::Sidecar => Chain::parse(bytes, golden.codec.as_ref().expect("schema").braids())
            .map(|_| ())
            .map_err(DocRefusal::Sidecar),
    }
}

fn render_doc(golden: &DocGolden, bytes: &[u8]) -> Vec<u8> {
    match golden.kind {
        DocKind::Manifest => Manifest::parse(bytes).expect("accepted").render(),
        DocKind::Checkpoint => {
            Checkpoint::parse(bytes, golden.codec.as_ref().expect("schema").braids())
                .expect("accepted")
                .render()
        }
        DocKind::Sidecar => Chain::parse(bytes, golden.codec.as_ref().expect("schema").braids())
            .expect("accepted")
            .render(),
    }
}

enum DocRefusal {
    Manifest(ManifestError),
    Checkpoint(CheckpointError),
    Sidecar(SidecarError),
}

fn assert_doc_refusal(name: &str, refusal: DocRefusal, len: usize) {
    match refusal {
        DocRefusal::Manifest(error) => assert_manifest_in_bounds(name, error, len),
        DocRefusal::Checkpoint(error) => assert_checkpoint_in_bounds(name, error, len),
        DocRefusal::Sidecar(error) => assert_sidecar_in_bounds(name, error, len),
    }
}

// ---------------------------------------------------------------------
// The inventory recipe is load-bearing.
// ---------------------------------------------------------------------

#[test]
fn storm_recipe_names_the_v3_ok_goldens() {
    let storm = storm();
    let roster = inventory();
    assert_eq!(roster["version"], 3);
    assert_eq!(storm["prng"]["name"], "XorShift64");
    let goldens = batch_goldens(&storm);
    assert!(!goldens.is_empty(), "the storm lists batch goldens");
    for golden in &goldens {
        golden.codec.decode(&golden.bytes).unwrap_or_else(|err| {
            panic!("{}: ok golden decodes, got {}", golden.name, err.identity())
        });
    }
    for section in ["manifest", "checkpoint", "sidecar"] {
        let kind = match section {
            "manifest" => DocKind::Manifest,
            "checkpoint" => DocKind::Checkpoint,
            _ => DocKind::Sidecar,
        };
        let docs = doc_goldens(&storm, section, kind);
        assert!(!docs.is_empty(), "the storm lists {section} goldens");
        for golden in &docs {
            parse_doc(golden, &golden.bytes)
                .unwrap_or_else(|_| panic!("{}: ok document parses", golden.name));
            assert_eq!(
                render_doc(golden, &golden.bytes),
                golden.bytes,
                "{}: inventory golden is a fixpoint",
                golden.name
            );
        }
    }
}

// ---------------------------------------------------------------------
// The batch decoder.
// ---------------------------------------------------------------------

/// Header widths summed: magic 4, version 2, flags 2, fingerprint 32,
/// braid 4, `braid_gen` 8, prev 32, writer 8, timestamp 8.
const OP_COUNT_AT: usize = 100;

#[test]
fn byte_soup_over_the_batch_decoder_types_every_refusal_in_bounds() {
    let storm = storm();
    let mut prng = XorShift(storm_seed(&storm, "batch_soup"));
    let cuts: Vec<usize> = storm["prng"]["batch_soup_cuts"]
        .as_array()
        .expect("cuts")
        .iter()
        .map(|n| usize::try_from(n.as_u64().expect("cut")).expect("cut fits"))
        .collect();
    let iters = storm_iters(&storm, "batch_soup_iters");
    for golden in batch_goldens(&storm) {
        for _ in 0..iters {
            let cut = cuts[prng.below(cuts.len())].min(golden.bytes.len());
            let mut soup = golden.bytes[..cut].to_vec();
            for _ in 0..prng.below(300) {
                soup.push(prng.byte());
            }
            if let Err(refusal) = golden.codec.decode(&soup) {
                assert_typed_in_bounds(&golden.name, &refusal, soup.len());
            }
        }
    }
}

#[test]
fn golden_mutation_storm_over_the_batch_decoder_never_panics() {
    let storm = storm();
    let mut prng = XorShift(storm_seed(&storm, "batch_storm"));
    let iters = storm_iters(&storm, "batch_storm_iters");
    for golden in batch_goldens(&storm) {
        for _ in 0..iters {
            let mutated = storm_mutant(&mut prng, &golden.bytes);
            match golden.codec.decode(&mutated) {
                Ok(batch) => {
                    let again = golden
                        .codec
                        .encode(&batch.header, &batch.ops)
                        .expect("accepted mutant re-encodes");
                    assert_eq!(
                        again, mutated,
                        "{}: accepted mutant is a fixpoint",
                        golden.name
                    );
                }
                Err(refusal) => assert_typed_in_bounds(&golden.name, &refusal, mutated.len()),
            }
        }
    }
}

#[test]
fn hostile_counts_refuse_truncated() {
    let storm = storm();
    for golden in batch_goldens(&storm) {
        let batch = golden
            .codec
            .decode(&golden.bytes)
            .expect("ok golden decodes");

        let mut ops_bomb = golden.bytes.clone();
        ops_bomb[OP_COUNT_AT..OP_COUNT_AT + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let refusal = golden
            .codec
            .decode(&ops_bomb)
            .expect_err("an unbacked op count refuses");
        assert_eq!(
            refusal.identity(),
            "Truncated",
            "{}: unbacked op count is Truncated",
            golden.name
        );
        assert_typed_in_bounds(&golden.name, &refusal, ops_bomb.len());

        if !batch.ops.is_empty() {
            let mut rows_bomb = golden.bytes.clone();
            rows_bomb[109..113].copy_from_slice(&u32::MAX.to_le_bytes());
            let refusal = golden
                .codec
                .decode(&rows_bomb)
                .expect_err("an unbacked row count refuses");
            assert_eq!(
                refusal.identity(),
                "Truncated",
                "{}: unbacked row count is Truncated",
                golden.name
            );
            assert_typed_in_bounds(&golden.name, &refusal, rows_bomb.len());
        }
    }
}

#[test]
fn trailing_bytes_refuse_at_the_exact_sequential_boundary() {
    let storm = storm();
    for golden in batch_goldens(&storm) {
        for extra in [1usize, 7, 64] {
            let mut padded = golden.bytes.clone();
            padded.extend(std::iter::repeat_n(0xAA, extra));
            assert_eq!(
                golden.codec.decode(&padded),
                Err(DecodeError::TrailingBytes {
                    at: golden.bytes.len()
                }),
                "{}: the parse consumes exactly the batch and no more",
                golden.name
            );
        }
        let mut doubled = golden.bytes.clone();
        doubled.extend_from_slice(&golden.bytes);
        assert_eq!(
            golden.codec.decode(&doubled),
            Err(DecodeError::TrailingBytes {
                at: golden.bytes.len()
            }),
            "{}: concatenation refuses at the boundary",
            golden.name
        );
    }
}

#[test]
fn every_proper_prefix_refuses_truncated() {
    let storm = storm();
    for golden in batch_goldens(&storm) {
        for len in 0..golden.bytes.len() {
            let refusal = golden
                .codec
                .decode(&golden.bytes[..len])
                .expect_err("a proper prefix always refuses");
            assert_eq!(
                refusal.identity(),
                "Truncated",
                "{}: prefix {len} is Truncated",
                golden.name
            );
            assert_typed_in_bounds(&golden.name, &refusal, len);
        }
    }
}

// ---------------------------------------------------------------------
// Manifest, checkpoint, and sidecar — the same storm over inventory
// document goldens.
// ---------------------------------------------------------------------

#[test]
fn byte_soup_over_the_document_parsers_refuses_in_bounds() {
    let storm = storm();
    let alphabet = storm["alphabet"].as_str().expect("alphabet").as_bytes();
    let iters = storm_iters(&storm, "document_soup_iters");
    let mut prng = XorShift(storm_seed(&storm, "document_soup"));
    let mut goldens = doc_goldens(&storm, "manifest", DocKind::Manifest);
    goldens.extend(doc_goldens(&storm, "checkpoint", DocKind::Checkpoint));
    for _ in 0..iters {
        let pick = prng.below(goldens.len());
        let golden = &goldens[pick];
        let soup = soup_from(&mut prng, &golden.bytes, alphabet, 120);
        if let Err(refusal) = parse_doc(golden, &soup) {
            assert_doc_refusal(&golden.name, refusal, soup.len());
        }
    }
}

#[test]
fn sidecar_byte_soup_refuses_in_bounds() {
    let storm = storm();
    let alphabet = storm["alphabet"].as_str().expect("alphabet").as_bytes();
    let iters = storm_iters(&storm, "document_soup_iters");
    let mut prng = XorShift(storm_seed(&storm, "sidecar_soup"));
    let goldens = doc_goldens(&storm, "sidecar", DocKind::Sidecar);
    for _ in 0..iters {
        let pick = prng.below(goldens.len());
        let golden = &goldens[pick];
        let soup = soup_from(&mut prng, &golden.bytes, alphabet, 120);
        if let Err(refusal) = parse_doc(golden, &soup) {
            assert_doc_refusal(&golden.name, refusal, soup.len());
        }
    }
}

fn document_storm(section: &str, kind: DocKind, seed_name: &str) {
    let storm = storm();
    let mut prng = XorShift(storm_seed(&storm, seed_name));
    let iters = storm_iters(&storm, "document_storm_iters");
    for golden in doc_goldens(&storm, section, kind) {
        parse_doc(&golden, &golden.bytes).unwrap_or_else(|_| {
            panic!("{}: inventory golden parses", golden.name);
        });
        assert_eq!(
            render_doc(&golden, &golden.bytes),
            golden.bytes,
            "{}: the golden is a fixpoint",
            golden.name
        );
        for _ in 0..iters {
            let mutated = storm_mutant(&mut prng, &golden.bytes);
            match parse_doc(&golden, &mutated) {
                Ok(()) => assert_eq!(
                    render_doc(&golden, &mutated),
                    mutated,
                    "{}: an accepted document is canonical",
                    golden.name
                ),
                Err(refusal) => assert_doc_refusal(&golden.name, refusal, mutated.len()),
            }
        }
    }
}

#[test]
fn manifest_mutation_storm_holds_the_canonical_fixpoint() {
    document_storm("manifest", DocKind::Manifest, "manifest_storm");
}

#[test]
fn checkpoint_mutation_storm_holds_the_canonical_fixpoint() {
    document_storm("checkpoint", DocKind::Checkpoint, "checkpoint_storm");
}

#[test]
fn sidecar_mutation_storm_holds_the_canonical_fixpoint() {
    document_storm("sidecar", DocKind::Sidecar, "sidecar_storm");
}

// ---------------------------------------------------------------------
// Materialised prefixes: the inventory's named refusals, no PRNG.
// ---------------------------------------------------------------------

fn stems(value: &Json) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item.as_str().expect("stem").to_string())
        .collect()
}

#[test]
fn materialised_fuzz_refuses_by_the_named_identity() {
    let roster = inventory();
    for rel in stems(&roster["fuzz_materialised"]) {
        let sidecar = read_json(&v3().join(format!("{rel}.json")));
        let bytes = std::fs::read(v3().join(format!("{rel}.bin"))).expect("fuzz bytes");
        match sidecar["expect"].as_str().expect("expect") {
            "ok" => {
                if rel.contains("/batch/") {
                    let codec = codec_named(sidecar["schema"].as_str().expect("schema"));
                    let batch = codec.decode(&bytes).expect("accepted mutant decodes");
                    let again = codec
                        .encode(&batch.header, &batch.ops)
                        .expect("accepted mutant re-encodes");
                    assert_eq!(again, bytes, "{rel}: accepted mutant is a fixpoint");
                } else if rel.contains("manifest_") {
                    let parsed = Manifest::parse(&bytes).expect("accepted manifest");
                    assert_eq!(parsed.render(), bytes, "{rel}: manifest fixpoint");
                } else if rel.contains("checkpoint_") {
                    let kitchen = codec_named("kitchen");
                    let parsed =
                        Checkpoint::parse(&bytes, kitchen.braids()).expect("accepted checkpoint");
                    assert_eq!(parsed.render(), bytes, "{rel}: checkpoint fixpoint");
                } else if rel.contains("sidecar_") {
                    let kitchen = codec_named("kitchen");
                    let parsed = Chain::parse(&bytes, kitchen.braids()).expect("accepted sidecar");
                    assert_eq!(parsed.render(), bytes, "{rel}: sidecar fixpoint");
                } else {
                    panic!("{rel}: unknown fuzz kind");
                }
            }
            "refusal" => {
                let identity = sidecar["refusal"].as_str().expect("named refusal");
                if rel.contains("/batch/") {
                    let schema = sidecar
                        .get("schema")
                        .and_then(Json::as_str)
                        .unwrap_or("kitchen");
                    let codec = codec_named(schema);
                    let refusal = codec.decode(&bytes).expect_err("{rel}: prefix refuses");
                    assert_eq!(refusal.identity(), identity, "{rel}: refusal identity");
                    assert_typed_in_bounds(&rel, &refusal, bytes.len());
                    continue;
                }
                if rel.contains("manifest_") {
                    let error = Manifest::parse(&bytes).expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    assert_manifest_in_bounds(&rel, error, bytes.len());
                    continue;
                }
                let kitchen = codec_named("kitchen");
                if rel.contains("checkpoint_") {
                    let error = Checkpoint::parse(&bytes, kitchen.braids())
                        .expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    assert_checkpoint_in_bounds(&rel, error, bytes.len());
                    continue;
                }
                if rel.contains("sidecar_") {
                    let error =
                        Chain::parse(&bytes, kitchen.braids()).expect_err("{rel}: prefix refuses");
                    assert_eq!(error.identity(), identity, "{rel}: refusal identity");
                    assert_sidecar_in_bounds(&rel, error, bytes.len());
                    continue;
                }
                panic!("{rel}: unknown fuzz kind");
            }
            other => panic!("{rel}: unknown expect {other}"),
        }
    }
}
