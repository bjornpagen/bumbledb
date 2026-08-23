//! Lane 9 — fuzz. Deterministic seeded harnesses as ordinary tests: the
//! batch decoder over arbitrary byte soup, hostile wire counts, and
//! golden mutations — never a panic, never an overflow, every refusal
//! typed with any carried offset in bounds, and the trailing-bytes
//! refusal landing at the exact end of the accepted prefix (the
//! offset-free sequential proof: the parser knows where a batch ends
//! from the bytes alone, no offset table to poison). The same harness
//! shape runs over the manifest and checkpoint parsers, where an
//! accepted mutant must be a canonical fixpoint (parse-then-render
//! reproduces the exact input bytes), and over the footprint
//! recomputation comparator, where a mutated footprint section must
//! land `FootprintMismatch` or a typed decode refusal — never
//! acceptance, never a state change.

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, StatementId, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Db, Value};
use bumbledb_log::apply::{Applied, ApplyRefusal, FootprintCause, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, DecodeError, MAGIC, Op, OpKind, VERSION};
use bumbledb_log::footprint::{CapacityMode, ContainmentMode, Entry, FootprintError};
use bumbledb_log::manifest::{Checkpoint, CheckpointError, Head, Manifest, ManifestError, hex32};
use bumbledb_log::sidecar::Chain;

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

fn goldens() -> Vec<(String, Codec, Vec<u8>, bool)> {
    let schemas = support::load_schemas();
    let dir = support::corpus_dir().join("batch");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("corpus present") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let sidecar: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("sidecar")).expect("json");
        let schema = sidecar["schema"].as_str().expect("schema").to_owned();
        let fingerprint: [u8; 32] = support::unhex(sidecar["fingerprint"].as_str().expect("hex"))
            .try_into()
            .expect("32 bytes");
        let codec = Codec::new(&schemas[&schema], fingerprint).expect("fixture vocabulary");
        let bytes = std::fs::read(path.with_extension("bin")).expect("bin");
        let ok = sidecar["expect"].as_str() == Some("ok");
        out.push((schema, codec, bytes, ok));
    }
    assert!(!out.is_empty(), "the corpus feeds the harness");
    out
}

/// Every refusal is a value; any byte offset it carries points inside
/// the input it refused.
fn assert_typed_in_bounds(name: &str, refusal: &DecodeError, len: usize) {
    assert!(!refusal.identity().is_empty(), "{name}: typed identity");
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

/// The general mutation operator set the storms share: byte flips, a
/// truncation, a chunk insertion, a chunk deletion, or a hostile
/// all-ones u32 spliced over four bytes.
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

// ---------------------------------------------------------------------
// The batch decoder.
// ---------------------------------------------------------------------

/// Header widths summed: magic 4, version 2, flags 2, fingerprint 32,
/// braid 4, `braid_gen` 8, prev 32, writer 8, timestamp 8.
const OP_COUNT_AT: usize = 100;

#[test]
fn byte_soup_over_the_batch_decoder_types_every_refusal_in_bounds() {
    let mut prng = XorShift(0xf9f9_0001_d00d_feed);
    // Prefix cuts at the header's field boundaries, so the soup reaches
    // past each sequential gate instead of dying at the magic.
    let cuts = [0usize, 4, 8, 40, 44, 100, 104];
    for (name, codec, bytes, _) in goldens() {
        for _ in 0..700 {
            let cut = cuts[prng.below(cuts.len())].min(bytes.len());
            let mut soup = bytes[..cut].to_vec();
            for _ in 0..prng.below(300) {
                soup.push(prng.byte());
            }
            if let Err(refusal) = codec.decode(&soup) {
                assert_typed_in_bounds(&name, &refusal, soup.len());
            }
        }
    }
}

#[test]
fn golden_mutation_storm_over_the_batch_decoder_never_panics() {
    let mut prng = XorShift(0xf9f9_0002_cafe_b0ba);
    for (name, codec, bytes, _) in goldens() {
        for _ in 0..2_000 {
            let mutated = storm_mutant(&mut prng, &bytes);
            if let Err(refusal) = codec.decode(&mutated) {
                assert_typed_in_bounds(&name, &refusal, mutated.len());
            }
        }
    }
}

#[test]
fn hostile_counts_cannot_force_allocation_or_overflow() {
    for (name, codec, bytes, ok) in goldens() {
        if !ok {
            continue;
        }
        let batch = codec.decode(&bytes).expect("golden decodes");

        let mut ops_bomb = bytes.clone();
        ops_bomb[OP_COUNT_AT..OP_COUNT_AT + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let refusal = codec
            .decode(&ops_bomb)
            .expect_err("an unbacked op count refuses");
        assert_typed_in_bounds(&name, &refusal, ops_bomb.len());

        if !batch.ops.is_empty() {
            // First op: kind at 104, relation at 105, row count at 109.
            let mut rows_bomb = bytes.clone();
            rows_bomb[109..113].copy_from_slice(&u32::MAX.to_le_bytes());
            let refusal = codec
                .decode(&rows_bomb)
                .expect_err("an unbacked row count refuses");
            assert_typed_in_bounds(&name, &refusal, rows_bomb.len());
        }

        let section = section_len(&batch.footprint);
        let fp_count_at = bytes.len() - section;
        let mut fp_bomb = bytes.clone();
        fp_bomb[fp_count_at..fp_count_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let refusal = codec
            .decode(&fp_bomb)
            .expect_err("an unbacked footprint count refuses");
        assert_typed_in_bounds(&name, &refusal, fp_bomb.len());
    }
}

#[test]
fn trailing_bytes_refuse_at_the_exact_sequential_boundary() {
    let goldens = goldens();
    for (name, codec, bytes, ok) in &goldens {
        if !ok {
            continue;
        }
        for extra in [1usize, 7, 64] {
            let mut padded = bytes.clone();
            padded.extend(std::iter::repeat_n(0xAA, extra));
            assert_eq!(
                codec.decode(&padded),
                Err(DecodeError::TrailingBytes { at: bytes.len() }),
                "{name}: the parse consumes exactly the batch and no more"
            );
        }
        // A second whole batch appended is still trailing garbage at
        // the first batch's exact end — nothing seeks past the cursor.
        let mut doubled = bytes.clone();
        doubled.extend_from_slice(bytes);
        assert_eq!(
            codec.decode(&doubled),
            Err(DecodeError::TrailingBytes { at: bytes.len() }),
            "{name}: concatenation refuses at the boundary"
        );
    }
}

#[test]
fn every_proper_prefix_refuses_with_offsets_in_bounds() {
    for (name, codec, bytes, ok) in goldens() {
        if !ok {
            continue;
        }
        for len in 0..bytes.len() {
            let refusal = codec
                .decode(&bytes[..len])
                .expect_err("a proper prefix always refuses");
            assert_typed_in_bounds(&name, &refusal, len);
        }
    }
}

// ---------------------------------------------------------------------
// The manifest and checkpoint parsers, same harness shape.
// ---------------------------------------------------------------------

fn manifest_goldens() -> Vec<Manifest> {
    vec![
        Manifest {
            fingerprint: [0x3c; 32],
            checkpoint: None,
        },
        Manifest {
            fingerprint: [0x3c; 32],
            checkpoint: Some([0x5a; 32]),
        },
    ]
}

fn checkpoint_goldens(codec: &Codec) -> Vec<Checkpoint> {
    let braids: Vec<BraidId> = codec.braids().components().keys().copied().collect();
    let heads: BTreeMap<BraidId, Head> = braids
        .iter()
        .enumerate()
        .map(|(index, braid)| {
            let fill = u8::try_from(index + 1).expect("small index");
            (
                *braid,
                Head {
                    g: 80 - u64::try_from(index).expect("small index") * 37,
                    hash: [fill; 32],
                    ts: 1_000 + u64::try_from(index).expect("small index"),
                },
            )
        })
        .collect();
    vec![
        Checkpoint {
            braids: heads.clone(),
            catalog: [0xab; 32],
            writer: 12_345,
            prev: None,
        },
        Checkpoint {
            braids: heads,
            catalog: [0xab; 32],
            writer: 12_345,
            prev: Some([0xcd; 32]),
        },
    ]
}

fn assert_manifest_refusal_in_bounds(error: ManifestError, len: usize) {
    match error {
        ManifestError::Malformed { at } => assert!(at <= len, "offset in bounds"),
        ManifestError::Version { .. } => {}
    }
}

fn assert_checkpoint_refusal_in_bounds(error: CheckpointError, len: usize) {
    match error {
        CheckpointError::Malformed { at } => assert!(at <= len, "offset in bounds"),
        CheckpointError::UnknownBraid { .. } | CheckpointError::BraidSet => {}
    }
}

#[test]
fn byte_soup_over_the_manifest_and_checkpoint_parsers_refuses_in_bounds() {
    let codec = fixture_codec();
    let mut prng = XorShift(0xf9f9_0003_beef_5eed);
    let alphabet = b"0123456789abcdef\"{}:,nulvig ";
    let manifest_golden = manifest_goldens().remove(1).render();
    let checkpoint_golden = checkpoint_goldens(&codec).remove(1).render();
    for _ in 0..12_000 {
        let (golden, manifest_side) = if prng.next().is_multiple_of(2) {
            (&manifest_golden, true)
        } else {
            (&checkpoint_golden, false)
        };
        let cut = prng.below(golden.len() + 1);
        let mut soup = golden[..cut].to_vec();
        for _ in 0..prng.below(120) {
            let byte = if prng.next().is_multiple_of(4) {
                prng.byte()
            } else {
                alphabet[prng.below(alphabet.len())]
            };
            soup.push(byte);
        }
        if manifest_side {
            if let Err(error) = Manifest::parse(&soup) {
                assert_manifest_refusal_in_bounds(error, soup.len());
            }
        } else if let Err(error) = Checkpoint::parse(&soup, codec.braids()) {
            assert_checkpoint_refusal_in_bounds(error, soup.len());
        }
    }
}

#[test]
fn manifest_mutation_storm_holds_the_canonical_fixpoint() {
    let mut prng = XorShift(0xf9f9_0004_0dd5_0f00);
    for golden in manifest_goldens() {
        let bytes = golden.render();
        assert_eq!(
            Manifest::parse(&bytes),
            Ok(golden),
            "the golden round-trips"
        );
        for _ in 0..6_000 {
            let mutated = storm_mutant(&mut prng, &bytes);
            match Manifest::parse(&mutated) {
                Ok(parsed) => assert_eq!(
                    parsed.render(),
                    mutated,
                    "an accepted document is canonical — parse-then-render is the identity"
                ),
                Err(error) => assert_manifest_refusal_in_bounds(error, mutated.len()),
            }
        }
    }
}

#[test]
fn checkpoint_mutation_storm_holds_the_canonical_fixpoint() {
    let codec = fixture_codec();
    let mut prng = XorShift(0xf9f9_0005_ca11_ab1e);
    for golden in checkpoint_goldens(&codec) {
        let bytes = golden.render();
        assert_eq!(
            Checkpoint::parse(&bytes, codec.braids()),
            Ok(golden),
            "the golden round-trips"
        );
        for _ in 0..6_000 {
            let mutated = storm_mutant(&mut prng, &bytes);
            match Checkpoint::parse(&mutated, codec.braids()) {
                Ok(parsed) => assert_eq!(
                    parsed.render(),
                    mutated,
                    "an accepted document is canonical — parse-then-render is the identity"
                ),
                Err(error) => assert_checkpoint_refusal_in_bounds(error, mutated.len()),
            }
        }
    }
}

fn braid_entry(braid: &str, head: &Head) -> String {
    format!(
        "\"{braid}\":{{\"g\":{},\"hash\":\"{}\",\"ts\":{}}}",
        head.g,
        hex32(&head.hash),
        head.ts
    )
}

fn checkpoint_doc(braids_body: &str) -> Vec<u8> {
    format!(
        "{{\"braids\":{{{braids_body}}},\"catalog\":\"{}\",\"writer\":12345,\"prev\":null}}",
        hex32(&[0xab; 32])
    )
    .into_bytes()
}

#[test]
fn checkpoint_braid_map_deviations_from_the_canon_refuse() {
    let codec = fixture_codec();
    let golden = checkpoint_goldens(&codec).remove(0);
    let entries: Vec<(String, Head)> = golden
        .braids
        .iter()
        .map(|(braid, head)| (braid.to_string(), *head))
        .collect();
    assert_eq!(entries.len(), 2, "the fixture theory carries two braids");

    let canonical = checkpoint_doc(&format!(
        "{},{}",
        braid_entry(&entries[0].0, &entries[0].1),
        braid_entry(&entries[1].0, &entries[1].1)
    ));
    assert_eq!(
        Checkpoint::parse(&canonical, codec.braids()),
        Ok(golden),
        "the canonical order parses"
    );

    // The same two facts in swapped order are non-canonical bytes of
    // the same value — a strict template walk refuses them, because a
    // content-addressed document with two accepted spellings would be
    // two digests for one checkpoint.
    let swapped = checkpoint_doc(&format!(
        "{},{}",
        braid_entry(&entries[1].0, &entries[1].1),
        braid_entry(&entries[0].0, &entries[0].1)
    ));
    assert!(
        matches!(
            Checkpoint::parse(&swapped, codec.braids()),
            Err(CheckpointError::Malformed { .. })
        ),
        "a re-ordered braid map refuses"
    );

    let duplicated = checkpoint_doc(&format!(
        "{},{}",
        braid_entry(&entries[0].0, &entries[0].1),
        braid_entry(&entries[0].0, &entries[0].1)
    ));
    assert!(
        matches!(
            Checkpoint::parse(&duplicated, codec.braids()),
            Err(CheckpointError::Malformed { .. })
        ),
        "a duplicated braid entry refuses"
    );

    let subset = checkpoint_doc(&braid_entry(&entries[0].0, &entries[0].1));
    assert_eq!(
        Checkpoint::parse(&subset, codec.braids()),
        Err(CheckpointError::BraidSet),
        "a missing braid is set drift"
    );

    let unknown = checkpoint_doc(&braid_entry("c00000007", &entries[0].1));
    assert_eq!(
        Checkpoint::parse(&unknown, codec.braids()),
        Err(CheckpointError::UnknownBraid { got: 7 }),
        "a braid the decomposition never mints refuses"
    );
}

#[test]
fn manifest_targeted_deviations_refuse_where_the_template_breaks() {
    let fingerprint = hex32(&[0x3c; 32]);
    let version_three =
        format!("{{\"v\":3,\"fingerprint\":\"{fingerprint}\",\"checkpoint\":null}}");
    assert_eq!(
        Manifest::parse(version_three.as_bytes()),
        Err(ManifestError::Version { got: 3 }),
        "a well-formed foreign version is the typed version refusal"
    );

    let deviants: [String; 6] = [
        format!("{{\"v\":02,\"fingerprint\":\"{fingerprint}\",\"checkpoint\":null}}"),
        format!(
            "{{\"v\":18446744073709551616,\"fingerprint\":\"{fingerprint}\",\"checkpoint\":null}}"
        ),
        format!(
            "{{\"v\":2,\"fingerprint\":\"{}\",\"checkpoint\":null}}",
            fingerprint.to_uppercase()
        ),
        format!("{{\"v\":2, \"fingerprint\":\"{fingerprint}\",\"checkpoint\":null}}"),
        format!("{{\"v\":2,\"fingerprint\":\"{fingerprint}\",\"checkpoint\":\"\"}}"),
        format!("{{\"v\":2,\"fingerprint\":\"{fingerprint}\",\"checkpoint\":null}}\n"),
    ];
    for bytes in &deviants {
        assert!(
            matches!(
                Manifest::parse(bytes.as_bytes()),
                Err(ManifestError::Malformed { .. })
            ),
            "the canon has one spelling: {bytes}"
        );
    }
}

// ---------------------------------------------------------------------
// The footprint recomputation comparator.
// ---------------------------------------------------------------------

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const VENUE: RelationId = RelationId(2);
const BOOKING: RelationId = RelationId(3);

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("f9_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// Two braids covering all four footprint classes: recipe+step under a
/// key and a containment, venue+booking under a weighted capacity whose
/// bound is the venue's own cap field.
fn fixture_theory() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::from([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "recipe".into(),
                fields: vec![field("id", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "step".into(),
                fields: vec![
                    field("recipe", ValueType::U64),
                    field("name", ValueType::String),
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "venue".into(),
                fields: vec![field("id", ValueType::U64), field("cap", ValueType::U64)],
                extension: None,
            },
            RelationDescriptor {
                name: "booking".into(),
                fields: vec![field("venue", ValueType::U64), field("qty", ValueType::U64)],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RECIPE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(STEP, &[0]),
                target: side(RECIPE, &[0]),
            },
            StatementDescriptor::Functionality {
                relation: VENUE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(VENUE, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::TargetField(FieldId(1))),
                source: side(BOOKING, &[0]),
            },
        ],
    }
}

fn fixture_codec() -> Codec {
    let descriptor = fixture_theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint).expect("fixture vocabulary")
}

fn fresh_db(tag: &str) -> Db<SchemaDescriptor> {
    let dir = temp_dir(tag).join("db");
    Db::create(&dir, fixture_theory())
        .expect("create")
        .expect("theory admits empty store")
}

fn header(codec: &Codec, braid: BraidId) -> BatchHeader {
    BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid,
        braid_gen: 1,
        prev: [0u8; 32],
        writer: 9,
        timestamp: 500,
    }
}

/// A first slot on the kitchen braid touching F, K, and C entries.
fn kitchen_batch(codec: &Codec) -> (BraidId, Vec<u8>, Vec<Entry>) {
    let braid = codec.braids().braid_of(RECIPE).expect("recipe braid");
    let ops = [
        Op {
            kind: OpKind::Insert,
            relation: RECIPE,
            rows: vec![Box::from([Value::U64(1)])],
        },
        Op {
            kind: OpKind::Insert,
            relation: STEP,
            rows: vec![Box::from([Value::U64(1), Value::String("chop".into())])],
        },
    ];
    let bytes = codec.encode(&header(codec, braid), &ops).expect("encode");
    let entries = codec.decode(&bytes).expect("golden decodes").footprint;
    (braid, bytes, entries)
}

/// A first slot on the venue braid touching F and both W modes.
fn venue_batch(codec: &Codec) -> (BraidId, Vec<u8>, Vec<Entry>) {
    let braid = codec.braids().braid_of(VENUE).expect("venue braid");
    let ops = [
        Op {
            kind: OpKind::Insert,
            relation: VENUE,
            rows: vec![Box::from([Value::U64(1), Value::U64(100)])],
        },
        Op {
            kind: OpKind::Insert,
            relation: BOOKING,
            rows: vec![Box::from([Value::U64(1), Value::U64(5)])],
        },
    ];
    let bytes = codec.encode(&header(codec, braid), &ops).expect("encode");
    let entries = codec.decode(&bytes).expect("golden decodes").footprint;
    (braid, bytes, entries)
}

/// The wire bytes of one footprint entry, mirroring the codec's
/// class/mode numbering so forged sections stay decodable.
fn encode_entry(entry: &Entry) -> Vec<u8> {
    let mut out = Vec::new();
    match entry {
        Entry::Fact { fid, mode } => {
            out.push(1);
            out.extend_from_slice(fid);
            out.push(match mode {
                OpKind::Insert => 1,
                OpKind::Delete => 2,
            });
        }
        Entry::Key { statement, key } => {
            out.push(2);
            out.extend_from_slice(&statement.0.to_le_bytes());
            out.extend_from_slice(key);
        }
        Entry::Containment {
            statement,
            key,
            mode,
        } => {
            out.push(3);
            out.extend_from_slice(&statement.0.to_le_bytes());
            out.extend_from_slice(key);
            out.push(match mode {
                ContainmentMode::Need => 1,
                ContainmentMode::SupportAdd => 2,
                ContainmentMode::SupportRemove => 3,
            });
        }
        Entry::Capacity {
            statement,
            key,
            mode,
        } => {
            out.push(4);
            out.extend_from_slice(&statement.0.to_le_bytes());
            out.extend_from_slice(key);
            match mode {
                CapacityMode::ChildDelta(delta) => {
                    out.push(1);
                    out.extend_from_slice(&delta.to_le_bytes());
                }
                CapacityMode::ParentAdd => out.push(2),
                CapacityMode::ParentRemove => out.push(3),
            }
        }
    }
    out
}

fn section_len(entries: &[Entry]) -> usize {
    4 + entries
        .iter()
        .map(|entry| encode_entry(entry).len())
        .sum::<usize>()
}

/// Rebuilds the batch with a replacement footprint section: everything
/// up to the section's count field kept byte-identical, the new entries
/// spliced behind a corrected count.
fn with_section(bytes: &[u8], original: &[Entry], entries: &[Entry]) -> Vec<u8> {
    let mut out = bytes[..bytes.len() - section_len(original)].to_vec();
    out.extend_from_slice(
        &u32::try_from(entries.len())
            .expect("few entries")
            .to_le_bytes(),
    );
    for entry in entries {
        out.extend_from_slice(&encode_entry(entry));
    }
    out
}

/// A mutated footprint section may fail the section parse (typed decode
/// refusal) or reach the comparator and mismatch — it must never be
/// accepted and never touch state.
fn assert_comparator_refusal(name: &str, applied: &Applied) {
    match applied {
        Applied::Refused(ApplyRefusal::Decode(error)) => {
            assert!(!error.identity().is_empty(), "{name}: typed identity");
        }
        Applied::Refused(ApplyRefusal::FootprintMismatch { .. }) => {}
        other => panic!("{name}: a mutated footprint section must refuse, got {other:?}"),
    }
}

fn apply_mutant(db: &Db<SchemaDescriptor>, codec: &Codec, braid: BraidId, bytes: &[u8]) -> Applied {
    let mut chain = Chain::genesis(codec.braids());
    apply(db, &mut chain, codec, braid, 1, bytes, 0).expect("store plumbing")
}

#[test]
fn footprint_section_byte_storm_never_reaches_acceptance() {
    let codec = fixture_codec();
    let db = fresh_db("fp_storm");
    let mut prng = XorShift(0xf9f9_0006_ba5e_ba11);
    let fixtures = [kitchen_batch(&codec), venue_batch(&codec)];

    for (braid, bytes, entries) in &fixtures {
        let start = bytes.len() - section_len(entries);
        for _ in 0..4_000 {
            let mut mutated = bytes.clone();
            match prng.next() % 5 {
                0 | 1 => {
                    for _ in 0..=prng.below(3) {
                        let at = start + prng.below(mutated.len() - start);
                        mutated[at] ^= u8::try_from(prng.next() % 255 + 1).expect("byte");
                    }
                }
                2 => {
                    let len = start + prng.below(mutated.len() - start);
                    mutated.truncate(len);
                }
                3 => {
                    let at = start + prng.below(mutated.len() - start + 1);
                    let chunk: Vec<u8> = (0..=prng.below(8)).map(|_| prng.byte()).collect();
                    mutated.splice(at..at, chunk);
                }
                _ => {
                    let at = start + prng.below(mutated.len() - start);
                    let end = (at + 1 + prng.below(8)).min(mutated.len());
                    mutated.drain(at..end);
                }
            }
            let applied = apply_mutant(&db, &codec, *braid, &mutated);
            assert_comparator_refusal("section storm", &applied);
        }
    }

    // No mutant reached the engine: the storm store is still at birth,
    // and the unmutated fixtures now apply as ordinary first slots.
    assert_eq!(db.generation().expect("generation").value(), 0);
    let mut chain = Chain::genesis(codec.braids());
    for (braid, bytes, _) in &fixtures {
        let applied = apply(&db, &mut chain, &codec, *braid, 1, bytes, 0).expect("store plumbing");
        assert!(
            matches!(applied, Applied::Advanced { .. }),
            "the clean fixture applies: {applied:?}"
        );
    }
    assert_eq!(db.generation().expect("generation").value(), 2);
}

#[test]
fn structured_footprint_forgeries_land_their_exact_refusals() {
    let codec = fixture_codec();
    let db = fresh_db("fp_forge");
    let (kitchen_braid, kitchen_bytes, kitchen_entries) = kitchen_batch(&codec);
    let (venue_braid, venue_bytes, venue_entries) = venue_batch(&codec);

    let diverged = |name: &str, braid: BraidId, bytes: &[u8]| {
        let applied = apply_mutant(&db, &codec, braid, bytes);
        assert!(
            matches!(
                applied,
                Applied::Refused(ApplyRefusal::FootprintMismatch {
                    cause: FootprintCause::Diverged,
                    ..
                })
            ),
            "{name}: the comparator convicts the carried section, got {applied:?}"
        );
    };

    // A dropped entry: still sorted, still decodable, no longer true.
    let mut dropped = kitchen_entries.clone();
    dropped.pop();
    diverged(
        "dropped entry",
        kitchen_braid,
        &with_section(&kitchen_bytes, &kitchen_entries, &dropped),
    );

    // The empty section: the cheapest possible understatement.
    diverged(
        "empty section",
        kitchen_braid,
        &with_section(&kitchen_bytes, &kitchen_entries, &[]),
    );

    // A forged extra entry appended at the sort order's ceiling.
    let forged = Entry::Capacity {
        statement: StatementId(u16::MAX),
        key: [0xff; 32],
        mode: CapacityMode::ParentRemove,
    };
    let mut padded = kitchen_entries.clone();
    padded.push(forged);
    diverged(
        "forged entry",
        kitchen_braid,
        &with_section(&kitchen_bytes, &kitchen_entries, &padded),
    );

    // The lying winner's exact shape: the child delta understated, and
    // separately nudged by one — the payload is not identity, so both
    // sections decode and both must fail the recompute.
    for (name, forged_delta) in [("understated delta", 0i64), ("nudged delta", 6)] {
        let tampered: Vec<Entry> = venue_entries
            .iter()
            .map(|entry| match entry {
                Entry::Capacity {
                    statement,
                    key,
                    mode: CapacityMode::ChildDelta(_),
                } => Entry::Capacity {
                    statement: *statement,
                    key: *key,
                    mode: CapacityMode::ChildDelta(forged_delta),
                },
                other => *other,
            })
            .collect();
        assert_ne!(tampered, venue_entries, "the tamper changed the section");
        diverged(
            name,
            venue_braid,
            &with_section(&venue_bytes, &venue_entries, &tampered),
        );
    }

    // Order violations die in the decoder, before the comparator runs.
    let mut swapped = kitchen_entries.clone();
    swapped.swap(0, 1);
    assert_eq!(
        apply_mutant(
            &db,
            &codec,
            kitchen_braid,
            &with_section(&kitchen_bytes, &kitchen_entries, &swapped)
        ),
        Applied::Refused(ApplyRefusal::Decode(DecodeError::UnsortedFootprint {
            index: 1
        })),
    );
    let mut doubled = kitchen_entries.clone();
    doubled.insert(1, doubled[0]);
    assert_eq!(
        apply_mutant(
            &db,
            &codec,
            kitchen_braid,
            &with_section(&kitchen_bytes, &kitchen_entries, &doubled)
        ),
        Applied::Refused(ApplyRefusal::Decode(DecodeError::DuplicateFootprintEntry {
            index: 1
        })),
    );

    assert_eq!(
        db.generation().expect("generation").value(),
        0,
        "no forgery touched state"
    );
}

#[test]
fn unrecomputable_ops_land_footprint_mismatch_never_ub() {
    let codec = fixture_codec();
    let db = fresh_db("fp_overflow");
    let braid = codec.braids().braid_of(VENUE).expect("venue braid");

    // Hand-built bytes our encoder refuses to produce: two bookings
    // whose weight sum leaves i64, under an empty carried section. The
    // decode is clean — the recompute itself is the tripwire.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(codec.fingerprint());
    bytes.extend_from_slice(&braid.raw().to_le_bytes());
    bytes.extend_from_slice(&1u64.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 32]);
    bytes.extend_from_slice(&9u64.to_le_bytes());
    bytes.extend_from_slice(&500u64.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&BOOKING.0.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    for qty in [u64::MAX, u64::MAX - 1] {
        bytes.push(1);
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&qty.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());

    assert!(codec.decode(&bytes).is_ok(), "the poison batch decodes");
    let applied = apply_mutant(&db, &codec, braid, &bytes);
    assert!(
        matches!(
            applied,
            Applied::Refused(ApplyRefusal::FootprintMismatch {
                cause: FootprintCause::Unrecomputable(FootprintError::DeltaOverflow { .. }),
                ..
            })
        ),
        "recomputation refusal is the typed unrecomputable arm, got {applied:?}"
    );
    assert_eq!(db.generation().expect("generation").value(), 0);
}
