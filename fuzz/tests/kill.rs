//! The WRITEMAP commit-window kill sweep's deterministic lane
//! (`fuzz/src/kill.rs` — random-timing SIGKILL against a child's commit
//! loop, filling the one window the crashpoint sweep cannot cut:
//! `mdb_txn_commit`'s interior, where `MDB_WRITEMAP` changes the write
//! pattern). Two lanes: the ephemeral store (`WRITEMAP|NOSYNC`, the
//! surface under test) and the durable store as the control; both
//! assert the same four-point corpse invariant — reopen, `verify_store`
//! green, a complete batch prefix (all-or-nothing, no third state), and
//! a working post-recovery commit.
//!
//! The smoke (~30 kills) runs in `scripts/check.sh`. The long lane is
//! `#[ignore]`d; run it per kind on the ramdisk
//! (`docs/architecture/60-validation.md` § the fuzzing charter):
//!
//! ```sh
//! export BUMBLEDB_SCRATCH_DIR="$(scripts/ramdisk.sh path || scripts/ramdisk.sh create)"
//! cargo test --manifest-path fuzz/Cargo.toml --test kill -- --ignored \
//!     --test-threads=1 --nocapture random_kills
//! ```
//!
//! `BUMBLEDB_KILL_ROUNDS` overrides the 2,000-round default;
//! `BUMBLEDB_KILL_SEED` pins the delay sequence (printed per session,
//! carried in every failure). Sessions are recorded in
//! `fuzz/SESSIONS.md`.

use bumbledb::StoreKind;

/// The child body: creates its store from the steering environment and
/// commits batches until the parent's SIGKILL lands. Run only via the
/// sweep parents below; a bare `--ignored` sweep is a no-op.
#[test]
#[ignore = "kill-child body; spawned by the sweep parents"]
fn kill_child() {
    bumbledb_fuzz::kill::child_entry();
}

/// The smoke: both kinds, ~30 kills total — enough to prove the harness
/// end-to-end per commit, not a statistical session (the long lane is).
#[test]
fn random_kills_recover_on_both_kinds_smoke() {
    let seed = bumbledb_fuzz::kill::session_seed();
    bumbledb_fuzz::kill::sweep(StoreKind::Durable, 15, seed);
    bumbledb_fuzz::kill::sweep(StoreKind::Ephemeral, 15, seed ^ 1);
}

/// The long ephemeral lane: >= 2,000 random-timing kills against
/// `WRITEMAP|NOSYNC` commit loops.
#[test]
#[ignore = "long kill session; see the module doc for the invocation"]
fn random_kills_recover_on_an_ephemeral_store_long() {
    bumbledb_fuzz::kill::sweep(
        StoreKind::Ephemeral,
        bumbledb_fuzz::kill::long_rounds(),
        bumbledb_fuzz::kill::session_seed(),
    );
}

/// The long durable control lane: the same session shape on the
/// default-flag store.
#[test]
#[ignore = "long kill session; see the module doc for the invocation"]
fn random_kills_recover_on_a_durable_store_long() {
    bumbledb_fuzz::kill::sweep(
        StoreKind::Durable,
        bumbledb_fuzz::kill::long_rounds(),
        bumbledb_fuzz::kill::session_seed(),
    );
}
