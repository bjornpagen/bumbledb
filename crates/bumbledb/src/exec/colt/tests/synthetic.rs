//! with the 2026-07-20 hard-delete): `TransientImage::refill` builds a
//! the LMDB-backed fixtures beside it keep out of Miri's reach.

use super::*;
use crate::image::TransientImage;
use bumbledb_theory::schema::ValueType;

fn synthetic_view(rows: &[(u64, u64)]) -> View {
    let words: Vec<[u64; 2]> = rows.iter().map(|&(k, v)| [k, v]).collect();
    let mut slot = TransientImage::default();
    let image = slot.refill(
        &[ValueType::U64, ValueType::U64],
        words.len(),
        words.iter().map(|row| &row[..]),
    );
    apply(&image, &[], &[], Vec::new())
}

#[test]
fn store_free_gathers_match_a_naive_model() {
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

    let keys = drain(&mut colt, root, 0);
    assert_eq!(keys.len(), 40);
}
