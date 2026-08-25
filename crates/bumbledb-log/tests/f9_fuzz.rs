//! Lane 9 — fuzz. Deterministic seeded harnesses as ordinary tests: the
//! batch decoder over arbitrary byte soup, hostile wire counts, and
//! golden mutations — never a panic, never an overflow, every refusal
//! typed with any carried offset in bounds, and the trailing-bytes
//! refusal landing at the exact end of the accepted prefix (the
//! offset-free sequential proof: the parser knows where a batch ends
//! from the bytes alone, no offset table to poison). The same harness
//! shape runs over the manifest, checkpoint, and chain-sidecar
//! parsers, where an accepted mutant must be a canonical fixpoint
//! (parse-then-render reproduces the exact input bytes).

#[path = "lane_a_support/mod.rs"]
mod support;

use std::collections::BTreeMap;

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{Codec, DecodeError};
use bumbledb_log::manifest::{hex32, Checkpoint, CheckpointError, Head, Manifest, ManifestError};
use bumbledb_log::sidecar::{Chain, ChainEntry, Pending, SidecarError};

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
        let codec = Codec::new(&schemas[&schema], fingerprint);
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
        CheckpointError::UnknownBraid { .. }
        | CheckpointError::BraidSet
        | CheckpointError::Version { .. }
        | CheckpointError::Overflow => {}
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
// The fixture theory behind the checkpoint and sidecar goldens.
// ---------------------------------------------------------------------

const RECIPE: RelationId = RelationId(0);
const STEP: RelationId = RelationId(1);
const VENUE: RelationId = RelationId(2);
const BOOKING: RelationId = RelationId(3);

/// Two braids: recipe+step under a key and a containment, venue+booking
/// under a weighted capacity whose bound is the venue's own cap field.
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
    Codec::new(&descriptor, fingerprint)
}

// ---------------------------------------------------------------------
// The chain sidecar parser, same harness shape.
// ---------------------------------------------------------------------

fn chain_goldens(codec: &Codec) -> Vec<Chain> {
    let mut advanced = Chain::genesis(codec.braids());
    for (index, entry) in advanced.entries_mut().values_mut().enumerate() {
        let fill = u8::try_from(index + 1).expect("small index");
        *entry = ChainEntry {
            g: 40 + u64::try_from(index).expect("small index") * 3,
            prev: [fill; 32],
            ts: 9_000 + u64::try_from(index).expect("small index"),
        };
    }
    let braid = *advanced.entries().keys().next().expect("a braid exists");
    let Chain::Settled { entries } = advanced.clone() else {
        panic!("genesis is Settled");
    };
    let pending = Chain::Pending {
        entries,
        batch: Pending {
            braid,
            slot: 41,
            bytes: vec![0xde, 0xad, 0xbe, 0xef],
        },
    };
    vec![Chain::genesis(codec.braids()), advanced, pending]
}

fn assert_sidecar_refusal_in_bounds(error: SidecarError, len: usize) {
    match error {
        SidecarError::Malformed { at } => assert!(at <= len, "offset in bounds"),
        SidecarError::Version { .. }
        | SidecarError::UnknownBraid { .. }
        | SidecarError::Overflow => {}
    }
}

#[test]
fn sidecar_mutation_storm_holds_the_canonical_fixpoint() {
    let codec = fixture_codec();
    let mut prng = XorShift(0xf9f9_0007_0000_0aaa);
    for golden in chain_goldens(&codec) {
        let bytes = golden.render();
        assert_eq!(
            Chain::parse(&bytes, codec.braids()),
            Ok(golden),
            "the golden round-trips"
        );
        for _ in 0..6_000 {
            let mutated = storm_mutant(&mut prng, &bytes);
            match Chain::parse(&mutated, codec.braids()) {
                Ok(parsed) => assert_eq!(
                    parsed.render(),
                    mutated,
                    "an accepted document is canonical — parse-then-render is the identity"
                ),
                Err(error) => assert_sidecar_refusal_in_bounds(error, mutated.len()),
            }
        }
    }
}

#[test]
fn sidecar_byte_soup_refuses_in_bounds() {
    let codec = fixture_codec();
    let mut prng = XorShift(0xf9f9_0008_0000_0bbb);
    let alphabet = b"0123456789abcdef\"{}:,nulvig ";
    let golden = chain_goldens(&codec).remove(2).render();
    for _ in 0..12_000 {
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
        if let Err(error) = Chain::parse(&soup, codec.braids()) {
            assert_sidecar_refusal_in_bounds(error, soup.len());
        }
    }
}
