//! The store-free colt fixture — the unchecked-gather interior's
//! standing referee (the fuzz corpus replay and the ASAN lane died
//! with the 2026-07-20 hard-delete): `TransientImage::refill` builds a
//! real `RelationImage` from encoded word rows with zero storage
//! involved, so this module runs under the Miri lane
//! (`scripts/miri.sh`) — the one dynamic checker that can see an
//! out-of-bounds `get_unchecked` in `gather_segment`'s interior, which
//! the LMDB-backed fixtures beside it keep out of Miri's reach.

use super::*;
use crate::image::TransientImage;
use bumbledb_theory::TypeDesc;

/// A store-free view over `(k, v)` u64 rows.
fn synthetic_view(rows: &[(u64, u64)]) -> View {
    let words: Vec<[u64; 2]> = rows.iter().map(|&(k, v)| [k, v]).collect();
    let mut slot = TransientImage::default();
    let image = slot.refill(
        &[TypeDesc::U64, TypeDesc::U64],
        words.len(),
        words.iter().map(|row| &row[..]),
    );
    apply(&image, &[], &[], Vec::new())
}

/// Force, probe, and drain over a synthetic image agree with a naive
/// model — every gather shape crossed: the forced-map iter at level 0,
/// singleton rows, and multi-chunk position chains through
/// `gather_segment`'s unchecked interior at the suffix.
#[test]
fn store_free_gathers_match_a_naive_model() {
    // 500 positions over 40 keys: per-key fanouts of 12–13 exercise
    // singleton-to-chunk promotion; key 0 gets 200 extra duplicates so
    // one chain crosses chunk boundaries.
    let mut rows: Vec<(u64, u64)> = (0..500).map(|i| (i % 40, i)).collect();
    rows.extend((500..700).map(|i| (0, i)));
    let view = synthetic_view(&rows);
    let mut colt = Colt::new(view, &[], vec![vec![0], vec![1]]);
    let root = Colt::root();
    for key in 0..40u64 {
        let child = colt.get(root, 0, &[key]).expect("every key exists");
        let entries = drain(&mut colt, child, 1);
        let mut got: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
        got.sort_unstable();
        let want: Vec<u64> = rows
            .iter()
            .filter(|(k, _)| *k == key)
            .map(|&(_, v)| v)
            .collect();
        assert_eq!(got, want, "key {key}");
    }
    assert!(colt.get(root, 0, &[40]).is_none(), "absent keys miss");

    // The root drain (forced-map iteration) yields each key once.
    let keys = drain(&mut colt, root, 0);
    assert_eq!(keys.len(), 40);
}
