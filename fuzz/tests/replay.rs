//! The regression-replay slot (the crucible packet (git ecec1dc3)): every
//! checked-in seed-corpus entry — and any minimized trophy that lands
//! beside them — replays through its runner under plain `cargo test`,
//! so a pinned finding never regresses silently. Empty trophy shelves
//! are fine; the slot exists.

use std::path::Path;

/// Replays every file under `dir` (sorted, deterministic) through one
/// runner; returns how many replayed. A missing directory is zero — the
/// artifact shelves only exist once a finding lands.
fn replay(dir: &str, runner: fn(&[u8])) -> usize {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return 0;
    };
    let mut paths: Vec<_> = entries
        .map(|entry| entry.expect("corpus entry").path())
        .collect();
    paths.sort();
    for path in &paths {
        let bytes = std::fs::read(path).expect("read corpus entry");
        runner(&bytes);
    }
    paths.len()
}

#[test]
fn the_ops_corpus_replays_clean() {
    assert!(
        replay("corpus/ops", bumbledb_fuzz::ops) > 0,
        "the ops seed corpus is checked in"
    );
    replay("trophies/ops", bumbledb_fuzz::ops);
}

#[test]
fn the_theory_corpus_replays_clean() {
    assert!(
        replay("corpus/theory", bumbledb_fuzz::theory) > 0,
        "the theory seed corpus is checked in"
    );
    replay("trophies/theory", bumbledb_fuzz::theory);
}
