//! The PRD-22 stress harness (test-only): loops bulk loads against
//! throwaway stores under synthetic CPU/IO contention, hunting the
//! chunk-commit-boundary EINVAL observed once on 2026-07-10
//! (`driver::tests::bench_refuses_without_a_stamp`, failing inside
//! `ensure_corpus` with `BulkLoad { committed: 65536, error:
//! Lmdb(Io(EINVAL)) }` alongside a concurrent cargo build).
//!
//! Mechanism under test: LMDB's commit durability boundary. On macOS,
//! `mdb_txn_commit` issues `fcntl(fd, F_FULLFSYNC)` (`lmdb-master-sys`
//! `mdb.c:171`, `MDB_FDATASYNC`) and surfaces the errno raw — no
//! `fsync(2)` fallback (`mdb_env_sync0`, `mdb.c:2915`), unlike `SQLite`'s
//! `unixSync`. The contention threads replicate the observed conditions:
//! concurrent flush-to-media traffic on the same volume plus CPU
//! saturation.
//!
//! Iterations come from `BUMBLEDB_STRESS_ITERS` (default 100 — the
//! PRD's floor). The reproduction budget the PRD names (500 iterations
//! under worst-case contention) is this harness run with
//! `BUMBLEDB_STRESS_ITERS=500` next to a live `cargo build`.
//!
//! Repro outcome, recorded honestly: the 500-iteration budget (1500
//! chunk commits under three sync-storm writers, three CPU spinners,
//! and a live release rebuild loop, 2026-07-10, pre-fix) did **not**
//! reproduce the EINVAL. The fix — the typed `CommitSync` boundary plus
//! the bounded observable retry in `storage/commit/write.rs` — stands
//! on the prime suspect's documented semantics per the PRD's direction:
//! the corpus volume is APFS (supports `F_FULLFSYNC`; the same test
//! passed on rerun there, eliminating the capability branch), leaving
//! the transient-under-pressure class.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::{Db, Value};

use crate::schema::{Ledger, ids};

/// Facts per iteration: two full bulk chunks plus a remainder, so every
/// iteration crosses three chunk-commit boundaries — the observed
/// failure site (`committed: 65536` = exactly 16 chunks).
const FACTS: u64 = 2 * 4096 + 512;

fn iterations() -> u64 {
    std::env::var("BUMBLEDB_STRESS_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

/// `Holder` rows synthesized directly: the one dependency-free relation,
/// so a prefix of any length commits cleanly (no containment sources).
fn rows() -> impl Iterator<Item = Vec<Value>> {
    (0..FACTS).map(|i| {
        vec![
            Value::U64(i),
            Value::String(format!("holder-{i}").into_bytes().into_boxed_slice()),
        ]
    })
}

/// Flush-to-media contention: write a few MiB and `sync_all` (which is
/// `fcntl(F_FULLFSYNC)` on macOS — the same drive-cache flush the LMDB
/// commit competes with), in the same temp volume the stores live on.
fn io_pressure(dir: std::path::PathBuf, stop: &Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let stop = Arc::clone(stop);
    std::thread::spawn(move || {
        use std::io::Write as _;
        let path = dir.join("pressure");
        let payload = vec![0xA5u8; 1 << 22];
        while !stop.load(Ordering::Relaxed) {
            let Ok(mut file) = std::fs::File::create(&path) else {
                return;
            };
            let _ = file.write_all(&payload);
            let _ = file.sync_all();
        }
        let _ = std::fs::remove_file(&path);
    })
}

/// CPU contention: saturate cores so the fsync path also fights for
/// scheduling, as it did next to the observed concurrent build.
fn cpu_pressure(stop: &Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let stop = Arc::clone(stop);
    std::thread::spawn(move || {
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        while !stop.load(Ordering::Relaxed) {
            for _ in 0..1 << 16 {
                x = std::hint::black_box(x.wrapping_mul(0x2545_F491_4F6C_DD1D).rotate_left(23));
            }
        }
    })
}

/// N bulk loads against fresh temp stores under synthetic contention:
/// every chunk commit must land (transient durability faults absorbed by
/// the bounded commit-boundary retry), never a `Lmdb(Io(...))` escape.
#[test]
fn bulk_load_survives_commit_pressure() {
    let iters = iterations();
    let root = std::env::temp_dir().join("bumbledb-bench-stress");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("stress root");

    let stop = Arc::new(AtomicBool::new(false));
    let mut pressure = Vec::new();
    for worker in 0..3 {
        let dir = root.join(format!("io-{worker}"));
        std::fs::create_dir_all(&dir).expect("pressure dir");
        pressure.push(io_pressure(dir, &stop));
    }
    for _ in 0..3 {
        pressure.push(cpu_pressure(&stop));
    }

    for i in 0..iters {
        let dir = root.join(format!("db-{i}"));
        let db = Db::create(&dir, Ledger).expect("create store");
        let loaded = db
            .bulk_load_dyn(ids::HOLDER, rows())
            .unwrap_or_else(|e| panic!("iteration {i}: {e}"));
        assert_eq!(loaded, FACTS, "iteration {i}: short load");
        drop(db);
        std::fs::remove_dir_all(&dir).expect("scratch teardown");
    }

    stop.store(true, Ordering::Relaxed);
    for handle in pressure {
        handle.join().expect("pressure thread");
    }
    let _ = std::fs::remove_dir_all(&root);
}
