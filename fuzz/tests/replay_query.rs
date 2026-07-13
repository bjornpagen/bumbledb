//! The regression-replay slot for PRD 13's targets (the `tests/replay.rs`
//! pattern): every checked-in seed-corpus entry — and any minimized
//! trophy that lands beside them — replays through its runner under
//! plain `cargo test`, so a pinned finding never regresses silently.

use std::path::Path;

/// Replays every file under `dir` (sorted, deterministic) through one
/// runner; returns how many replayed. A missing directory is zero — the
/// trophy shelves only exist once a finding lands.
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
fn the_query_corpus_replays_clean() {
    assert!(
        replay("corpus/query", bumbledb_fuzz::query::run) > 0,
        "the query seed corpus is checked in"
    );
    replay("trophies/query", bumbledb_fuzz::query::run);
}

#[test]
fn the_rewrites_corpus_replays_clean() {
    assert!(
        replay("corpus/rewrites", bumbledb_fuzz::rewrites::run) > 0,
        "the rewrites seed corpus is checked in"
    );
    replay("trophies/rewrites", bumbledb_fuzz::rewrites::run);
}
