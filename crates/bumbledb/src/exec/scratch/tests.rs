//! `ScratchRelation` behavior: exact set semantics across both tiers, the
//! charged RAM→LMDB transition, forced spill, oversized-key buckets and
//! disposal hygiene. Gate anchors: Q-DISK / Q-FALLBACK / Q-COLLISION /
//! Q-BUDGET / F-RESOURCE (the scratch halves), QRY-002.

use super::*;
use crate::api::prepared::source::UNBOUNDED_POLICY;
use crate::work::ExecutionPolicy;

fn work() -> WorkContext {
    UNBOUNDED_POLICY.start().expect("unbounded ledger")
}

#[test]
fn insert_if_absent_is_exact_set_semantics_in_ram() {
    let work = work();
    let mut scratch = ScratchRelation::with_default_budget(&work);
    assert!(scratch.insert_if_absent(b"alpha", b"").expect("insert"));
    assert!(!scratch.insert_if_absent(b"alpha", b"").expect("insert"));
    assert!(scratch.insert_if_absent(b"beta", b"").expect("insert"));
    assert_eq!(scratch.len(), 2);
    assert!(!scratch.spilled());
}

#[test]
fn the_spill_transition_preserves_every_entry_and_order() {
    let work = work();
    // A tiny RAM allowance forces the transition mid-stream.
    let mut scratch = ScratchRelation::new(&work, 512);
    for i in 0..64u64 {
        assert!(
            scratch
                .insert_if_absent(&i.to_be_bytes(), &[u8::try_from(i).expect("64 keys")])
                .expect("insert"),
            "fresh key {i}"
        );
    }
    assert!(scratch.spilled(), "512 bytes cannot hold 64 entries");
    assert_eq!(scratch.len(), 64);
    // Exactness survives the tier change: re-inserts are duplicates,
    // lookups return the stored values, iteration is key-ordered/complete.
    for i in 0..64u64 {
        assert!(
            !scratch
                .insert_if_absent(&i.to_be_bytes(), &[u8::try_from(i).expect("64 keys")])
                .expect("insert"),
            "duplicate key {i}"
        );
    }
    let mut out = Vec::new();
    assert!(scratch.get(&7u64.to_be_bytes(), &mut out).expect("get"));
    assert_eq!(out, vec![7u8]);
    let mut seen = Vec::new();
    scratch
        .for_each(&mut |key, value| {
            seen.push((key.to_vec(), value.to_vec()));
            Ok(true)
        })
        .expect("walk");
    assert_eq!(seen.len(), 64);
    assert!(seen.is_sorted_by(|a, b| a.0 < b.0), "key-ordered walk");
}

#[test]
fn forced_spill_before_first_group_matches_ram_answers() {
    // Q-FALLBACK shape: force the disk tier from entry zero; behavior is
    // identical to the RAM tier.
    let work = work();
    let mut ram = ScratchRelation::with_default_budget(&work);
    let mut disk = ScratchRelation::with_default_budget(&work);
    disk.force_spill().expect("forced spill");
    assert!(disk.spilled() && !ram.spilled());
    for key in [&b"one"[..], b"two", b"one", b"three", b"two"] {
        let a = ram.insert_if_absent(key, b"v").expect("ram");
        let b = disk.insert_if_absent(key, b"v").expect("disk");
        assert_eq!(a, b, "tier-independent verdict for {key:?}");
    }
    assert_eq!(ram.len(), disk.len());
}

#[test]
fn oversized_keys_use_exact_buckets_not_hash_verdicts() {
    // Keys beyond the physical LMDB bound: two long keys sharing a long
    // prefix stay distinct; equality is decided by full bytes.
    let work = work();
    let mut scratch = ScratchRelation::new(&work, 0); // spill immediately
    let long_a = vec![0xAB; 600];
    let mut long_b = long_a.clone();
    long_b.push(0x01);
    assert!(scratch.insert_if_absent(&long_a, b"a").expect("insert"));
    assert!(scratch.insert_if_absent(&long_b, b"b").expect("insert"));
    assert!(!scratch.insert_if_absent(&long_a, b"a").expect("insert"));
    let mut out = Vec::new();
    assert!(scratch.get(&long_b, &mut out).expect("get"));
    assert_eq!(out, b"b");
    assert_eq!(scratch.len(), 2);
}

#[test]
fn updates_replace_values_across_both_tiers() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work, 256);
    scratch.put(b"group", b"1").expect("put");
    scratch.put(b"group", b"2").expect("put");
    assert_eq!(scratch.len(), 1, "an upsert is not a second member");
    // Push over the RAM allowance, then update on the disk tier.
    for i in 0..64u64 {
        scratch.put(&i.to_be_bytes(), &[0]).expect("put");
    }
    assert!(scratch.spilled());
    scratch.put(b"group", b"3").expect("put");
    let mut out = Vec::new();
    assert!(scratch.get(b"group", &mut out).expect("get"));
    assert_eq!(out, b"3");
}

#[test]
fn scratch_growth_is_charged_before_it_happens() {
    // Q-BUDGET: a tiny scratch-byte allowance stops the spilled tier with
    // a typed exhaustion, never unreserved growth.
    let context = ExecutionPolicy {
        scratch_bytes: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    let mut scratch = ScratchRelation::new(&context, 0);
    let mut refused = false;
    for i in 0..4096u64 {
        match scratch.insert_if_absent(&i.to_be_bytes(), &[0u8; 64]) {
            Ok(_) => {}
            Err(error) => {
                refused = true;
                let rendered = format!("{error:?}");
                assert!(
                    rendered.contains("ScratchBytes"),
                    "typed scratch exhaustion, got {rendered}"
                );
                break;
            }
        }
    }
    assert!(refused, "4 KiB of scratch cannot absorb 256 KiB of entries");
}

#[test]
fn ram_growth_is_charged_before_it_happens() {
    let context = ExecutionPolicy {
        working_bytes: 4096,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("start");
    // RAM allowance far above the working budget: the working-byte ledger
    // must refuse before the tier ever spills.
    let mut scratch = ScratchRelation::new(&context, usize::MAX);
    let mut refused = false;
    for i in 0..4096u64 {
        match scratch.insert_if_absent(&i.to_be_bytes(), &[0u8; 64]) {
            Ok(_) => {}
            Err(error) => {
                refused = true;
                let rendered = format!("{error:?}");
                assert!(
                    rendered.contains("WorkingBytes"),
                    "typed working exhaustion, got {rendered}"
                );
                break;
            }
        }
    }
    assert!(refused);
    assert!(!scratch.spilled(), "refusal, not an unbudgeted spill");
}

#[test]
fn disposal_removes_the_scratch_directory() {
    let work = work();
    let mut scratch = ScratchRelation::new(&work, 0);
    scratch.insert_if_absent(b"k", b"v").expect("insert");
    assert!(scratch.spilled());
    let path = match &scratch.tier {
        Tier::Lmdb(env) => env.cleanup.0.clone(),
        Tier::Ram { .. } => unreachable!("spilled above"),
    };
    assert!(path.exists(), "environment directory exists while owned");
    drop(scratch);
    assert!(
        !path.exists(),
        "disposal closes the environment then unlinks its own directory"
    );
}

#[test]
fn cancellation_stops_scratch_work_at_a_bounded_quantum() {
    let work = work();
    let mut scratch = ScratchRelation::with_default_budget(&work);
    for i in 0..32u64 {
        scratch
            .insert_if_absent(&i.to_be_bytes(), b"")
            .expect("insert");
    }
    work.cancel();
    let error = scratch
        .insert_if_absent(b"after-cancel", b"")
        .expect_err("cancelled work refuses");
    assert!(format!("{error:?}").contains("Cancelled"));
    let walk = scratch.for_each(&mut |_, _| Ok(true));
    assert!(walk.is_err(), "iteration polls the same ledger");
}
