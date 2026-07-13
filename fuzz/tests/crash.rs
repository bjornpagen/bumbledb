//! The crash target's deterministic lane (the crucible packet (git ecec1dc3)):
//! the crashpoint sweep — every named commit-pipeline point × the small
//! ops-prefix matrix, each combination killed and recovered at least
//! once under plain `cargo test`, never left to fuzzer luck — plus the
//! regression-replay slot for the crash seed corpus and any trophies.
//!
//! The child is THIS binary re-entered on the ignored `crash_child`
//! body (the `crates/bumbledb/tests/crash.rs` precedent), env-var
//! steered by the harness's spawn.

use std::path::Path;

/// The child body: rebuilds its scenario from the steering environment
/// (sweep cell or replay input), commits the prefix, arms the drawn
/// crashpoint, runs the victim, and dies inside the engine's hook. Run
/// only via the parents below; a bare `--ignored` sweep is a no-op.
#[test]
#[ignore = "crash-child body; spawned by the sweep and replay parents"]
fn crash_child() {
    bumbledb_fuzz::crash::child_entry();
}

/// The sweep: every crashpoint fires on every matrix cell's victim (the
/// generator's co-located test pins that the victims lie on every
/// point's path), and every corpse recovers per the table's side —
/// reopen, `verify_store`, all-or-nothing contents, victim replay.
#[test]
fn every_crashpoint_recovers_across_the_prefix_matrix() {
    for cell in 0..bumbledb_fuzz::crash::MATRIX_CELLS {
        for index in 0..bumbledb_fuzz::crash::point_count() {
            bumbledb_fuzz::crash::sweep(cell, index);
        }
    }
}

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
fn the_crash_corpus_replays_clean() {
    assert!(
        replay("corpus/crash", bumbledb_fuzz::crash::replay) > 0,
        "the crash seed corpus is checked in"
    );
    replay("trophies/crash", bumbledb_fuzz::crash::replay);
}
