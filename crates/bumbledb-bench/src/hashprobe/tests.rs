//! Authored F1, executed F3 with the workspace suite.
//!
//! Gate mapping: `roles_*` → HASH-01; `collision_*` → HASH-02;
//! `sizing_*` → HASH-03; `equivalence_*`/`kat_*`/`inputs_*` → HASH-01/04.

use std::collections::BTreeSet;

use super::collision::{self, EngineOps, Judged, Model, Op, Row, WorkBound};
use super::inputs;
use super::kat::{self, KatOutcome};
use super::probe::{self, CANDIDATES, Candidate};
use super::sizing;
use super::{HashRole, role_inventory};

fn close(actual: f64, expected: f64, rel: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * rel,
        "expected {expected:e}, got {actual:e}"
    );
}

// HASH-01 — the inventory itself.

#[test]
fn roles_cover_every_role_once_with_chapter41_widths() {
    let inventory = role_inventory();
    let mut seen = Vec::new();
    for spec in inventory {
        assert!(!seen.contains(&spec.role), "duplicate role {:?}", spec.role);
        seen.push(spec.role);
        match spec.role {
            HashRole::LocalFingerprint | HashRole::ApplicationId => {
                assert_eq!(spec.width_bytes, 16);
            }
            HashRole::AuthoritativeContent => assert_eq!(spec.width_bytes, 32),
            HashRole::TransientRouting => assert_eq!(spec.width_bytes, 8),
        }
    }
    assert_eq!(seen.len(), 4);
}

#[test]
fn roles_never_permit_truncation_and_back_nonadversarial_roles_with_exact_checks() {
    for spec in role_inventory() {
        assert!(
            !spec.truncation_allowed,
            "{:?}: no generic helper may truncate any role's bytes",
            spec.role
        );
        if !spec.adversarial {
            assert!(
                spec.exact_check_backed,
                "{:?}: a non-adversarial hash role must be backed by exact comparison",
                spec.role
            );
        }
    }
}

#[test]
fn roles_only_authoritative_content_carries_the_adversarial_premise() {
    for spec in role_inventory() {
        assert_eq!(
            spec.adversarial,
            matches!(spec.role, HashRole::AuthoritativeContent),
            "{:?}",
            spec.role
        );
    }
}

// HASH-03 — the sizing math reproduces chapter 41's published cells.

#[test]
fn sizing_reproduces_the_chapter41_probability_table() {
    let million = 1_000_000u128;
    let billion = 1_000_000_000u128;
    let trillion = 1_000_000_000_000u128;
    close(sizing::birthday_probability(million, 64), 2.71e-8, 0.01);
    close(sizing::birthday_probability(million, 96), 6.31e-18, 0.01);
    close(sizing::birthday_probability(million, 128), 1.47e-27, 0.01);
    close(sizing::birthday_probability(million, 256), 4.32e-66, 0.01);
    close(sizing::birthday_probability(billion, 64), 0.0267, 0.01);
    close(sizing::birthday_probability(billion, 96), 6.31e-12, 0.01);
    close(sizing::birthday_probability(billion, 128), 1.47e-21, 0.01);
    close(sizing::birthday_probability(trillion, 96), 6.31e-6, 0.01);
    close(sizing::birthday_probability(trillion, 128), 1.47e-15, 0.01);
    // "effectively 100%": one trillion inputs into 64 bits.
    assert!(sizing::birthday_probability(trillion, 64) > 0.999_999);
}

#[test]
fn sizing_required_bits_match_the_published_thresholds() {
    assert_eq!(sizing::required_bits(1_000_000, 1e-15), Some(89));
    assert_eq!(sizing::required_bits(1_000_000_000, 1e-15), Some(109));
    assert_eq!(sizing::required_bits(1_000_000_000_000, 1e-15), Some(129));
    assert_eq!(sizing::required_bytes(1_000_000, 1e-15), Some(12));
    assert_eq!(sizing::required_bytes(1_000_000_000, 1e-15), Some(14));
    assert_eq!(sizing::required_bytes(1_000_000_000_000, 1e-15), Some(17));
    assert_eq!(
        sizing::required_bits(1, 1e-15),
        None,
        "no pair, no collision"
    );
    assert_eq!(
        sizing::required_bits(2, 0.0),
        None,
        "epsilon must be positive"
    );
}

#[test]
fn sizing_fleet_sum_is_not_the_global_domain_square() {
    // One million domains of one million rows each.
    let per_domain = sizing::birthday_probability(1_000_000, 128);
    let fleet = sizing::fleet_probability(1_000_000, per_domain);
    close(fleet, 1.47e-21, 0.01);
    // The one-shared-trillion-domain figure is six orders larger.
    let shared = sizing::birthday_probability(1_000_000_000_000, 128);
    assert!(fleet < shared / 1000.0);
    assert!((sizing::fleet_probability(u64::MAX, 1.0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn sizing_keeps_the_three_probability_models_apart() {
    // Corruption of one chosen message: 2^-b, independent of population.
    close(sizing::corruption_miss_probability(64), 5.421e-20, 0.001);
    // Birthday at the same width with a billion inputs is ~2.7e-2 — the two
    // models differ by ~17 orders of magnitude and must never be swapped.
    assert!(
        sizing::birthday_probability(1_000_000_000, 64)
            > sizing::corruption_miss_probability(64) * 1e15
    );
    // Deliberate search: b/2 generic bits. 16-byte commitment = 64-bit.
    assert_eq!(sizing::generic_collision_resistance_bits(128), 64);
    assert_eq!(sizing::generic_collision_resistance_bits(256), 128);
}

#[test]
fn sizing_uuid_entropy_is_not_128_bits() {
    assert_eq!(sizing::UUID_V4_RANDOM_BITS, 122);
    assert_eq!(sizing::ID128_RANDOM_BITS, 128);
    // Using the 128-bit column for UUIDv4 understates the probability 64x.
    let ratio = sizing::birthday_lambda(1_000_000, 122) / sizing::birthday_lambda(1_000_000, 128);
    close(ratio, 64.0, 1e-9);
}

// HASH-01/04 — corpus and probe structure.

#[test]
fn inputs_cover_the_required_sizes_and_are_deterministic() {
    let corpus = inputs::corpus(7);
    for &len in &inputs::SIZES {
        for &offset in &inputs::ALIGN_OFFSETS {
            assert!(
                corpus
                    .iter()
                    .any(|input| input.len == len && input.align_offset == offset),
                "missing len {len} offset {offset}"
            );
        }
    }
    assert!(
        inputs::SIZES.contains(&(8 * 1024 * 1024)),
        "snapshot chunk size"
    );
    let again = inputs::corpus(7);
    assert_eq!(corpus, again, "same seed, same bytes");
    let other = inputs::corpus(8);
    assert_ne!(corpus, other, "different seed, different bytes");
    for input in &corpus {
        assert_eq!(input.slice().len(), input.len);
    }
}

#[test]
fn inputs_split_schedules_cover_exactly_and_include_streaming_chunks() {
    for &len in &inputs::SIZES {
        let schedules = inputs::split_schedules(len);
        assert!(!schedules.is_empty());
        for schedule in &schedules {
            assert_eq!(schedule.iter().sum::<usize>(), len);
        }
        if len > 64 * 1024 {
            assert!(
                schedules.iter().any(|s| s.len() > 2),
                "bulk inputs need a chunked schedule"
            );
        }
    }
}

#[test]
fn inputs_mixture_is_deterministic_and_short_fact_shaped() {
    let stream = inputs::mixture(3, 512);
    assert_eq!(stream, inputs::mixture(3, 512));
    let allowed: BTreeSet<usize> = inputs::MIXTURE_WEIGHTS
        .iter()
        .map(|(len, _)| *len)
        .collect();
    for input in &stream {
        assert!(
            allowed.contains(&input.len),
            "unexpected length {}",
            input.len
        );
    }
}

#[test]
fn equivalence_streaming_matches_oneshot_for_every_candidate() {
    let corpus = inputs::corpus(11);
    probe::check_equivalence(&corpus).expect("one-shot and streaming digests agree");
}

#[test]
fn equivalence_truncation_is_the_prefix_and_widths_are_pinned() {
    let message = b"the widths are contracts, not tuning knobs";
    let full = probe::digest_oneshot(Candidate::Blake3Full32, message);
    let trunc = probe::digest_oneshot(Candidate::Blake3Trunc16, message);
    assert_eq!(full.len(), 32);
    assert_eq!(trunc.len(), 16);
    assert_eq!(trunc[..], full[..16]);
    for candidate in CANDIDATES {
        assert_eq!(
            probe::digest_oneshot(candidate, message).len(),
            candidate.output_bytes()
        );
    }
}

#[test]
fn equivalence_distinct_inputs_distinct_digests_across_candidates() {
    // Not a collision claim — a smoke check that each candidate actually
    // consumes its input (a stubbed constant function must fail here).
    for candidate in CANDIDATES {
        let a = probe::digest_oneshot(candidate, b"a");
        let b = probe::digest_oneshot(candidate, b"b");
        assert_ne!(a, b, "{} ignored its input", candidate.name());
        let empty = probe::digest_oneshot(candidate, b"");
        assert_ne!(a, empty);
    }
}

#[test]
fn kat_input_rule_is_the_official_byte_cycle() {
    let input = kat::blake3_vector_input(255);
    assert_eq!(input[0], 0);
    assert_eq!(input[250], 250);
    assert_eq!(input[251], 0, "the cycle repeats at 251");
    assert_eq!(input.len(), 255);
}

#[test]
fn kat_missing_file_is_an_error_never_a_pass() {
    let missing = std::path::Path::new("/nonexistent/bumbledb-kat.json");
    assert!(kat::verify_blake3_file(missing).is_err());
}

#[test]
fn kat_self_generated_vector_round_trips_and_mismatch_fails() {
    let dir = std::env::temp_dir().join("bumbledb-bench-kat-selftest");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tempdir");
    // Self-generated file: pins the loader/format, NOT the algorithm (the
    // official upstream vectors do that in F3).
    let input = kat::blake3_vector_input(65);
    let digest = probe::digest_oneshot(Candidate::Blake3Full32, &input);
    let hex = digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    let good = dir.join("good.json");
    std::fs::write(
        &good,
        format!("{{\"blake3\":[{{\"input_len\":65,\"hash\":\"{hex}\"}}]}}"),
    )
    .expect("write");
    assert_eq!(
        kat::verify_blake3_file(&good).expect("parses"),
        KatOutcome::Passed(1)
    );
    let bad = dir.join("bad.json");
    let mut wrong = digest.clone();
    wrong[0] ^= 0xFF;
    let wrong_hex = wrong.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    std::fs::write(
        &bad,
        format!("{{\"blake3\":[{{\"input_len\":65,\"hash\":\"{wrong_hex}\"}}]}}"),
    )
    .expect("write");
    assert!(matches!(
        kat::verify_blake3_file(&bad).expect("parses"),
        KatOutcome::Failed(_)
    ));
    let empty = dir.join("empty.json");
    std::fs::write(&empty, "{\"blake3\":[]}").expect("write");
    assert!(
        kat::verify_blake3_file(&empty).is_err(),
        "zero vectors is not a check"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// HASH-02 — schedule, model and driver logic (the real-engine wiring runs in
// F3 with the fingerprint override; these pin the harness semantics).

#[test]
fn collision_schedule_is_deterministic_and_touches_every_path() {
    let plan = collision::schedule(5, 600, 16);
    assert_eq!(plan, collision::schedule(5, 600, 16));
    let mut inserts = 0;
    let mut deletes = 0;
    let mut contains = 0;
    let mut pairs = 0;
    let mut reopens = 0;
    let mut spills = 0;
    let mut counts = 0;
    let mut long_payloads = 0;
    let mut conflicting_pairs = 0;
    let mut landing_pairs = 0;
    for op in &plan {
        match op {
            Op::Insert(row) => {
                inserts += 1;
                if row.1 > 0 {
                    long_payloads += 1;
                }
            }
            Op::Delete(_) => deletes += 1,
            Op::Contains(_) => contains += 1,
            Op::ConflictingPair(a, b) => {
                pairs += 1;
                if Model::pair_conflicts((*a, *b)) {
                    conflicting_pairs += 1;
                } else {
                    landing_pairs += 1;
                }
            }
            Op::Reopen => reopens += 1,
            Op::ForceSpill => spills += 1,
            Op::CountDistinct => counts += 1,
        }
    }
    assert!(inserts > 0 && deletes > 0 && contains > 0 && pairs > 0 && counts > 0);
    assert!(
        conflicting_pairs > 0 && landing_pairs > 0,
        "both genuine same-key conflicts and distinct-key pairs must occur \
         ({conflicting_pairs} conflicting, {landing_pairs} landing)"
    );
    assert_eq!(reopens, 2, "mid-stream and final reopen");
    assert_eq!(spills, 1);
    assert!(
        long_payloads > 0,
        "long/overflow payload classes must occur"
    );
}

#[test]
fn collision_pair_judgment_is_exact_bytes_not_fingerprints() {
    // Within a command: same key, different payload = conflict; the same row
    // twice normalizes; distinct keys never conflict with each other.
    assert!(Model::pair_conflicts(((1, 0), (1, 2))));
    assert!(!Model::pair_conflicts(((1, 1), (1, 1))));
    assert!(!Model::pair_conflicts(((1, 0), (2, 0))));
    // Complete-final-state judgment against the store: a pair that is clean
    // within itself still rejects when one row collides with a stored
    // different-class row, and nothing lands.
    let mut model = Model::default();
    assert_eq!(model.insert((1, 0)), Judged::Committed);
    assert_eq!(model.insert((1, 2)), Judged::Rejected, "key law vs store");
    assert!(
        model.contains((1, 0)),
        "the rejected insert changed nothing"
    );
    assert_eq!(model.pair((1, 2), (5, 0)), Judged::Rejected);
    assert!(
        !model.contains((5, 0)),
        "a rejected command commits nothing"
    );
    assert_eq!(model.pair((6, 1), (6, 1)), Judged::Committed, "normalizes");
    assert_eq!(model.distinct(), 2);
    // Deleting an absent or different-class row is a committed no-op.
    assert_eq!(model.delete((1, 2)), Judged::Committed);
    assert!(model.contains((1, 0)));
}

/// A reference engine (a second independent model instance) with a per-op
/// probe budget: the driver must accept a lawful engine and reject a lying
/// one.
fn reference_engine_passes_and_divergence_is_caught(lie: bool) -> Result<(), String> {
    let plan = collision::schedule(9, 400, 8);
    // The closures share one store through a RefCell — the driver only needs
    // FnMut, and the reference engine is single-threaded.
    let store: std::cell::RefCell<Model> = std::cell::RefCell::new(Model::default());
    let spilled_cell = std::cell::Cell::new(false);
    let mut insert = |row: Row| {
        let rejected = store.borrow_mut().insert(row) == Judged::Rejected;
        Ok((rejected, store.borrow().distinct()))
    };
    let mut delete = |row: Row| {
        store.borrow_mut().delete(row);
        Ok(store.borrow().distinct() + 1)
    };
    let mut contains = |row: Row| {
        let present = store.borrow().contains(row);
        Ok((
            if lie { !present } else { present },
            store.borrow().distinct(),
        ))
    };
    let mut pair = |a: Row, b: Row| {
        let rejected = store.borrow_mut().pair(a, b) == Judged::Rejected;
        Ok((rejected, store.borrow().distinct()))
    };
    let mut reopen = || Ok(());
    let mut force_spill = || {
        spilled_cell.set(true);
        Ok(())
    };
    let mut count = || Ok((store.borrow().distinct(), store.borrow().distinct()));
    let mut engine = EngineOps {
        insert: &mut insert,
        delete: &mut delete,
        contains: &mut contains,
        conflicting_pair: &mut pair,
        reopen: &mut reopen,
        force_spill: &mut force_spill,
        count_distinct: &mut count,
    };
    let result = collision::drive(
        &plan,
        &mut engine,
        WorkBound {
            probes_per_bucket_row: 2,
            bucket_floor: 4,
        },
    );
    if result.is_ok() {
        assert!(spilled_cell.get(), "the spill point must have been driven");
    }
    result
}

#[test]
fn collision_driver_accepts_a_lawful_engine() {
    reference_engine_passes_and_divergence_is_caught(false).expect("lawful engine passes");
}

#[test]
fn collision_driver_catches_a_lying_engine_with_the_op_index() {
    let err = reference_engine_passes_and_divergence_is_caught(true)
        .expect_err("a lying contains must fail");
    assert!(err.contains("op "), "failure names the op index: {err}");
}

#[test]
fn collision_work_bound_is_linear_in_the_bucket() {
    let bound = WorkBound {
        probes_per_bucket_row: 1,
        bucket_floor: 2,
    };
    assert!(bound.allows(0, 0));
    assert!(bound.allows(102, 100));
    assert!(!bound.allows(103, 100), "superlinear probing is a failure");
}
