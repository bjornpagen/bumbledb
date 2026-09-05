use bumbledb::Db;

use crate::fixture::TempDir;

use super::{
    ID_BASE, Mass, child_fact_bytes, grind_children, hash_rank_word, load, model_fact_fingerprint,
    pin_hash_model, run_with_floor, shuffled_ranks, slab, world,
};
use crate::corpus_gen::Rng;

#[test]
fn the_hash_model_matches_the_engine() {
    let dir = TempDir::new("sweep-pin");
    let db = Db::create(dir.path(), world::WindowedWorld)
        .expect("create")
        .expect("accepted");
    load(&db, Mass::unit()).expect("load the unit mass");
    pin_hash_model(&db).expect("the sweep's hash model matches the engine");
}

#[test]
fn grading_engineers_the_sorted_probe_order() {
    let parents: Vec<u64> = (0..8).map(|i| i * 3 + 1).collect();
    let ranks: Vec<u64> = (0..8).collect();
    let mut next_id = ID_BASE;
    let children = grind_children(&parents, &ranks, &mut next_id);
    assert_eq!(children.len(), parents.len());
    let hashes: Vec<[u8; super::MODEL_FINGERPRINT_LEN]> = children
        .iter()
        .map(|&(id, parent)| model_fact_fingerprint(&child_fact_bytes(id, parent, 0)))
        .collect();
    for (i, pair) in hashes.windows(2).enumerate() {
        assert!(
            pair[0] < pair[1],
            "hash order must ascend with parent order at {i}"
        );
    }
    for (child, &parent) in children.iter().zip(&parents) {
        assert_eq!(child.1, parent, "the grind never re-pairs parents");
    }

    assert_eq!(slab(0, 8), 0);
    assert_eq!(slab(u64::MAX, 8), 7);
    let probe = model_fact_fingerprint(&child_fact_bytes(1, 2, 0));
    assert_eq!(
        hash_rank_word(&probe),
        u64::from_be_bytes(probe[..8].try_into().expect("8 bytes"))
    );
}

#[test]
fn shuffled_ranks_are_a_seed_stable_permutation() {
    let mut rng = Rng::new(7);
    let ranks = shuffled_ranks(16, &mut rng);
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..16).collect::<Vec<u64>>());
    let mut rng = Rng::new(7);
    assert_eq!(ranks, shuffled_ranks(16, &mut rng), "seed-stable");
}

#[cfg(feature = "obs")]
#[test]
fn a_tiny_sweep_prints_one_row_per_size() {
    let dir = TempDir::new("sweep-smoke");
    let table = run_with_floor(dir.path(), &[2, 4], 2, 1, 64).expect("smoke sweep");
    assert!(table.contains("size"), "{table}");
    assert!(table.contains("sorted/delta"), "{table}");
    let rows: Vec<&str> = table
        .lines()
        .filter(|line| line.trim_start().starts_with('2') || line.trim_start().starts_with('4'))
        .collect();
    assert_eq!(rows.len(), 2, "one row per swept size:\n{table}");

    for row in rows {
        assert_eq!(row.matches('|').count(), 3, "four column groups: {row}");
    }
}

#[cfg(not(feature = "obs"))]
#[test]
fn a_plain_build_refuses_with_the_obs_remedy() {
    let dir = TempDir::new("sweep-plain");
    let err = run_with_floor(dir.path(), &[2], 1, 1, 64).unwrap_err();
    assert!(err.contains("--features obs"), "{err}");
}

/// The refusal ladder: empty sizes, a zero size, and an out-of-range sample
/// count each name their remedy without touching a store.
#[test]
fn the_knob_refusals_name_their_remedies() {
    let dir = TempDir::new("sweep-refusals");
    let err = run_with_floor(dir.path(), &[], 2, 1, 64).unwrap_err();
    assert!(err.contains("--sizes"), "{err}");
    let err = run_with_floor(dir.path(), &[0], 2, 1, 64).unwrap_err();
    assert!(err.contains("positive"), "{err}");
    let err = run_with_floor(dir.path(), &[4], 0, 1, 64).unwrap_err();
    assert!(err.contains("--samples"), "{err}");
    let err = run_with_floor(dir.path(), &[4], super::MAX_SAMPLES + 1, 1, 64).unwrap_err();
    assert!(err.contains("--samples"), "{err}");
}
