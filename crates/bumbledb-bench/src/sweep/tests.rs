use bumbledb::Db;

use crate::fixture::TempDir;

use super::{
    ID_BASE, Mass, child_fact_bytes, grind_children, hash_rank_word, load, model_fact_hash,
    pin_hash_model, run_with_floor, shuffled_ranks, slab, world,
};
use crate::corpus_gen::Rng;

/// The grading's foundation, pinned against the live engine: the
/// canonical child encoding the sweep models is exactly the engine's —
/// observable because `Violations::seal`'s stable sort keeps the
/// first-discovered witness, and the source-side scan discovers in
/// target-key order since the W8 sort landed (the pin's expected
/// witness is the key-least probe; its hash-least sibling is a
/// checked-different fact, so a revert to delta-order discovery trips
/// it too). If this fails, [`super::run`] refuses at every store setup
/// with the same message; the fixture keeps the pin in the plain test
/// suite where an encoding or order change trips it immediately.
#[test]
fn the_hash_model_matches_the_engine() {
    let dir = TempDir::new("sweep-pin");
    let db = Db::ephemeral(dir.path(), world::WindowedWorld).expect("ephemeral");
    load(&db, Mass::unit()).expect("load the unit mass");
    pin_hash_model(&db).expect("the sweep's hash model matches the engine");
}

/// Grading lands each child in its parent's rank slab: the sorted arm's
/// hashes ascend with parent order (delta order = key order), and the
/// per-parent pairing survives the grind.
#[test]
fn grading_engineers_the_sorted_probe_order() {
    let parents: Vec<u64> = (0..8).map(|i| i * 3 + 1).collect();
    let ranks: Vec<u64> = (0..8).collect();
    let mut next_id = ID_BASE;
    let children = grind_children(&parents, &ranks, &mut next_id);
    assert_eq!(children.len(), parents.len());
    let hashes: Vec<[u8; 32]> = children
        .iter()
        .map(|&(id, parent)| model_fact_hash(&child_fact_bytes(id, parent, 0)))
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
    // The slab partition is exact and order-preserving at the edges.
    assert_eq!(slab(0, 8), 0);
    assert_eq!(slab(u64::MAX, 8), 7);
    let probe = model_fact_hash(&child_fact_bytes(1, 2, 0));
    assert_eq!(
        hash_rank_word(&probe),
        u64::from_be_bytes(probe[..8].try_into().expect("8 bytes"))
    );
}

/// The delta arm's rank permutation covers 0..k and is seed-stable —
/// both arms replay identical draws run-over-run.
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

/// One tiny end-to-end sweep (obs builds only — the spans need the
/// trace seam): the table prints one row per size with both arms.
/// Structure only, never a timing claim.
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
    // The delta arm's probe-order label is engineered, not measured —
    // both arms must have produced numbers (columns present).
    for row in rows {
        assert_eq!(row.matches('|').count(), 3, "four column groups: {row}");
    }
}

/// Without the obs build the judgment spans are invisible — the lane
/// refuses with the rebuild remedy instead of printing zeros.
#[cfg(not(feature = "obs"))]
#[test]
fn a_plain_build_refuses_with_the_obs_remedy() {
    let dir = TempDir::new("sweep-plain");
    let err = run_with_floor(dir.path(), &[2], 1, 1, 64).unwrap_err();
    assert!(err.contains("--features obs"), "{err}");
}

/// The refusal ladder: empty sizes, a zero size, and an out-of-range
/// sample count each name their remedy without touching a store.
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
