//! T8 pin — judgment source probes in key-sorted vs arrival order.
//!
//! The commit path's source-side judgment runs one serial LMDB B-tree
//! descent per containment edge (`storage/commit/judgment.rs:
//! check_source`). The descents are dependent — no manufactured MLP is
//! available through the LMDB API — so the one honest lever is probe
//! ORDER: key-sorted probes revisit the same upper pages and walk the
//! leaf level monotonically; arrival order (the delta's fact-hash
//! order) is a random walk over the target tree.
//!
//! Interleaved A/B inside ONE process (m2max.method.interleaved-ab):
//! arm A = the shipped key-sorted worklist, arm B = arrival order
//! through `bumbledb::with_probe_sort_disabled` (the `trace`
//! test-support switch). Fresh random parent draws per repetition
//! (m2max.predict.tage-memorizes-benchmarks), pair order alternated
//! (ABBA) to cancel drift, and the measured span is the engine's own
//! `judgment_source` trace span with its traced probe count
//! (m2max.method.attribution-count-error: the divisor is the span's
//! a0, never an assumed count). Ephemeral store: no fsync floor to
//! drown the µs-scale judgment (`docs/reports/ramdisk-phase-r.md`).
//!
//! Run: `cargo run --release -p bumbledb-bench --features obs
//!       --example judgment_probe_order`

// Measurement arithmetic: ns and counts far below 2^52 — the f64 casts
// are exact at every magnitude this pin produces.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::PathBuf;
use std::time::Instant;

use bumbledb::{Db, RelationId, Value, obs};
use bumbledb_bench::corpus_gen::Rng;

/// The pin world: one containment, `Child(parent) <= Parent(id)` — every
/// child insert is exactly one scalar source probe into Parent's key tree.
mod world {
    bumbledb::schema! {
        pub ProbeWorld;

        relation Parent {
            id: u64 as ParentId, fresh,
            kind: u64,
        }
        relation Child {
            id: u64 as ChildId, fresh,
            parent: u64 as ParentId,
        }

        Child(parent) <= Parent(id);
    }
}

const PARENT: RelationId = RelationId(0);

/// One captured `judgment_source` span: (`dur_ns`, probes).
fn judged_span(events: &[obs::TraceEvent]) -> (u64, u64) {
    let event = events
        .iter()
        .find(|e| e.name == obs::names::JUDGMENT_SOURCE)
        .expect("the commit records a judgment_source span");
    (event.dur_ns, event.a0)
}

/// One measured commit: K child inserts under fresh random parents —
/// returns (span ns, probes) and the inserted facts for cleanup.
fn run_commit(
    db: &Db<world::ProbeWorld>,
    rng: &mut Rng,
    parents: u64,
    k: usize,
) -> ((u64, u64), Vec<world::Child>) {
    let draws: Vec<u64> = (0..k).map(|_| rng.range(parents)).collect();
    let mut inserted = Vec::with_capacity(k);
    obs::start_capture();
    db.write(|tx| {
        for &p in &draws {
            let id: world::ChildId = tx.alloc()?;
            let child = world::Child {
                id,
                parent: world::ParentId(p),
            };
            tx.insert(&child)?;
            inserted.push(child);
        }
        Ok(())
    })
    .expect("pin commit");
    let events = obs::finish_capture();
    (judged_span(&events), inserted)
}

/// The untimed cleanup: delete the commit's children so the store's
/// mass is identical for every repetition of both arms.
fn cleanup(db: &Db<world::ProbeWorld>, inserted: &[world::Child]) {
    db.write(|tx| {
        for child in inserted {
            tx.delete(child)?;
        }
        Ok(())
    })
    .expect("pin cleanup");
}

fn median(xs: &mut [f64]) -> f64 {
    xs.sort_unstable_by(|a, b| a.partial_cmp(b).expect("finite"));
    xs[xs.len() / 2]
}

/// One (parents, commit-size) configuration: `pairs` interleaved A/B
/// repetitions, ABBA order, per-pair per-probe ratio distribution.
fn pin_config(db: &Db<world::ProbeWorld>, parents: u64, k: usize, pairs: usize) {
    let mut rng = Rng::new(0x0717_0001 ^ parents ^ (k as u64) << 32);
    // Warm both arms (mmap residency, allocator, first-commit paths).
    for _ in 0..4 {
        let (_, ins) = run_commit(db, &mut rng, parents, k);
        cleanup(db, &ins);
        let (_, ins) = bumbledb::with_probe_sort_disabled(|| run_commit(db, &mut rng, parents, k));
        cleanup(db, &ins);
    }

    let mut sorted_ns: Vec<f64> = Vec::with_capacity(pairs);
    let mut arrival_ns: Vec<f64> = Vec::with_capacity(pairs);
    let mut ratios: Vec<f64> = Vec::with_capacity(pairs);
    for pair in 0..pairs {
        let mut arm = |sorted: bool| -> f64 {
            let ((ns, probes), ins) = if sorted {
                run_commit(db, &mut rng, parents, k)
            } else {
                bumbledb::with_probe_sort_disabled(|| run_commit(db, &mut rng, parents, k))
            };
            cleanup(db, &ins);
            assert_eq!(probes as usize, k, "one probe per inserted child");
            ns as f64 / probes as f64
        };
        // ABBA: even pairs A-first, odd pairs B-first.
        let (a, b) = if pair % 2 == 0 {
            let a = arm(true);
            let b = arm(false);
            (a, b)
        } else {
            let b = arm(false);
            let a = arm(true);
            (a, b)
        };
        sorted_ns.push(a);
        arrival_ns.push(b);
        ratios.push(b / a);
    }
    let (p10, p50, p90) = {
        ratios.sort_unstable_by(|a, b| a.partial_cmp(b).expect("finite"));
        (
            ratios[ratios.len() / 10],
            ratios[ratios.len() / 2],
            ratios[ratios.len() * 9 / 10],
        )
    };
    println!(
        "parents={parents:>8} k={k:>5} pairs={pairs} \
         sorted_med={:>8.1} ns/probe  arrival_med={:>8.1} ns/probe  \
         ratio(arrival/sorted) p10={p10:.3} p50={p50:.3} p90={p90:.3}",
        median(&mut sorted_ns),
        median(&mut arrival_ns),
    );
}

/// The family-shape guard: bench-world commits are ONE insert per
/// commit — the sort must be invisible there. Whole-commit wall time
/// (ephemeral, no fsync floor), blocks per arm, interleaved.
fn family_guard(scratch: &std::path::Path, parents: u64, blocks: usize, per_block: usize) {
    let dir = scratch.join(format!("probe-order-family-p{parents}"));
    let db = Db::ephemeral(&dir, world::ProbeWorld).expect("ephemeral");
    db.bulk_load_dyn(
        PARENT,
        (0..parents).map(|i| vec![Value::U64(i), Value::U64(0)]),
    )
    .expect("seed parents");
    let mut rng = Rng::new(0x0717_00FF);
    let block = |sorted: bool, rng: &mut Rng| -> f64 {
        let run = |rng: &mut Rng| {
            let start = Instant::now();
            for _ in 0..per_block {
                let p = rng.range(parents);
                db.write(|tx| {
                    let id: world::ChildId = tx.alloc()?;
                    tx.insert(&world::Child {
                        id,
                        parent: world::ParentId(p),
                    })
                })
                .expect("family commit");
            }
            start.elapsed().as_secs_f64() * 1e9 / per_block as f64
        };
        if sorted {
            run(rng)
        } else {
            bumbledb::with_probe_sort_disabled(|| run(rng))
        }
    };
    // Warm.
    block(true, &mut rng);
    block(false, &mut rng);
    let mut ratios = Vec::with_capacity(blocks);
    let mut a_ns = Vec::with_capacity(blocks);
    let mut b_ns = Vec::with_capacity(blocks);
    for i in 0..blocks {
        let (a, b) = if i % 2 == 0 {
            let a = block(true, &mut rng);
            let b = block(false, &mut rng);
            (a, b)
        } else {
            let b = block(false, &mut rng);
            let a = block(true, &mut rng);
            (a, b)
        };
        a_ns.push(a);
        b_ns.push(b);
        ratios.push(b / a);
    }
    println!(
        "family-shape (k=1 commits, wall): parents={parents} blocks={blocks}x{per_block} \
         sorted_med={:>9.1} ns/commit  arrival_med={:>9.1} ns/commit  ratio p50={:.4}",
        median(&mut a_ns),
        median(&mut b_ns),
        median(&mut ratios),
    );
}

fn main() {
    let scratch = std::env::var_os("BUMBLEDB_SCRATCH_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("t8-judgment-probe-order");
    // The ephemeral store's WRITEMAP file is the full 4 GiB map: at most
    // one store fits the 5 GiB ramdisk, so each tier's store is removed
    // before the next opens (and any stale run's leftovers first).
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("scratch");

    // The regime sweep: bench-world tree (L1/L2-resident) up to a
    // DRAM-tier parent tree, commit sizes from family-real to bulk.
    // One store per tier (the K sweep leaves the mass unchanged — every
    // measured commit's children are deleted before the next).
    for &parents in &[4_096u64, 65_536, 1_048_576, 4_194_304] {
        let dir = scratch.join(format!("probe-order-p{parents}"));
        {
            let db = Db::ephemeral(&dir, world::ProbeWorld).expect("ephemeral");
            db.bulk_load_dyn(
                PARENT,
                (0..parents).map(|i| vec![Value::U64(i), Value::U64(0)]),
            )
            .expect("seed parents");
            for &k in &[16usize, 64, 256, 1024, 4096] {
                pin_config(&db, parents, k, 60);
            }
        }
        std::fs::remove_dir_all(&dir).expect("tier teardown");
    }
    family_guard(&scratch, 4_096, 40, 256);
    let _ = std::fs::remove_dir_all(&scratch);
}
