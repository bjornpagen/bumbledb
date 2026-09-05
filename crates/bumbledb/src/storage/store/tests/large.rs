//! E-LARGE / Q-LARGE-STORE / RUN-08 storage-side fixture: an actually
//! populated store beyond the former 32 GiB policy boundary, opened and
//! mutated with bounded working memory. Authored for the dedicated
//! storage-qualified F3 lane (`cargo test -- --ignored` on a runner with
//! ≥ 60 GiB free disk); an ignored-not-run result is `NotRun`, never a pass.
//!
//! Sparse files or a large virtual map alone are NOT this gate: the fixture
//! writes real payload bytes and asserts the populated file crossed the
//! boundary. Co-owned with P14, which measures the same fixture's cost.

use super::*;
use std::fmt::Write as _;

const LARGE_DIR_ENV: &str = "BUMBLEDB_LARGE_STORE_DIR";
const BOUNDARY: u64 = 32 << 30;
/// Per-row payload: 8 MiB of pseudorandom-ish text so page compression or
/// zero-page sparseness cannot fake the population.
const ROW_BYTES: usize = 8 << 20;

fn payload(id: u64) -> String {
    // Deterministic, non-constant bytes: cheap xorshift over the id.
    let mut state = id.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = String::with_capacity(ROW_BYTES);
    while out.len() < ROW_BYTES {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        write!(out, "{state:016x}").expect("string writes are infallible");
    }
    out.truncate(ROW_BYTES);
    out
}

#[test]
#[ignore = "dedicated storage-qualified lane: >40 GiB of real disk; run in F3 with --ignored"]
fn a_populated_store_crosses_the_former_32_gib_boundary_and_reopens() {
    let root = std::path::PathBuf::from(
        std::env::var(LARGE_DIR_ENV).expect("set BUMBLEDB_LARGE_STORE_DIR to a >60 GiB volume"),
    );
    let path = root.join("bumbledb-large-store-fixture");
    assert!(!path.exists(), "stale large fixture; remove it first");
    let store = create_default(&path);
    let mut id = 0u64;
    // Populate past the old boundary plus margin, in bounded batches, with
    // spot exact-lookup checks on both sides of the boundary.
    let mut probes: Vec<u64> = Vec::new();
    while store
        .map_report(&work())
        .expect("report")
        .populated_file_bytes
        < BOUNDARY + (8 << 30)
    {
        let batch: Vec<_> = (0..8)
            .map(|_| {
                id += 1;
                (NOTE, note(id, &payload(id)))
            })
            .collect();
        commit_changes(&store, &change_set(&schema(), &batch, &[]));
        if id.is_multiple_of(512) {
            probes.push(id);
        }
    }
    let report = store.map_report(&work()).expect("report");
    assert!(report.populated_file_bytes > BOUNDARY);
    if let Some(allocated) = report.allocated_disk_bytes {
        // Real blocks, not a sparse file posing as population.
        assert!(allocated > BOUNDARY);
    }
    let total = id;
    drop(store);
    // Reopen the >32 GiB store: no size refusal, exact reads on both sides.
    let store = open_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), total);
    probes.push(1);
    probes.push(total);
    for id in probes {
        assert!(
            snapshot
                .contains(
                    NOTE,
                    crate::canonical::CanonicalRow::encode(
                        schema().relation(NOTE).fields(),
                        &note(id, &payload(id)),
                        &work()
                    )
                    .expect("row")
                    .as_bytes(),
                    &work()
                )
                .expect("probe")
        );
    }
    drop(snapshot);
    // Mutation still works beyond the boundary.
    let commit = commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(total + 1, "beyond"))], &[]),
    );
    assert!(commit.changed);
    drop(store);
    std::fs::remove_dir_all(&path).expect("large fixture cleanup");
}
