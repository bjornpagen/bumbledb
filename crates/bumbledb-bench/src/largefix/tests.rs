//! Authored F1, executed F3. Gate mapping: everything here → APP-LARGE /
//! G05 (fixture-plan admission, generator determinism/resumability, sparse
//! refusal). Population itself happens only in F3 on fitting hardware.

use super::enforce;
use super::generator::{self, StreamChecksum};
use super::{BeyondRamPlan, FORMER_CEILING_BYTES, LargeStorePlan, MIN_POPULATED_BYTES};

#[test]
fn plan_default_clears_both_thresholds_and_the_arithmetic_holds() {
    let plan = LargeStorePlan::default_f3();
    plan.check().expect("the default plan is admissible");
    assert!(plan.target_payload_bytes > MIN_POPULATED_BYTES.min(FORMER_CEILING_BYTES));
    // rows × row_bytes covers the target.
    assert!(plan.rows() * u64::from(plan.row_bytes) >= plan.target_payload_bytes);
    assert!(plan.chunks() * u64::from(plan.rows_per_chunk) >= plan.rows());
    // The recorded allowance is a small fraction of the data.
    assert!(plan.memory_allowance_bytes * 4 < plan.target_payload_bytes);
}

#[test]
fn plan_refuses_sub_minimum_and_ceiling_dodging_fixtures() {
    let mut plan = LargeStorePlan::default_f3();
    plan.target_payload_bytes = 20 << 30;
    assert!(plan.check().is_err(), "below the 40 GiB minimum");
    let mut plan = LargeStorePlan::default_f3();
    plan.memory_allowance_bytes = plan.target_payload_bytes;
    assert!(
        plan.check().is_err(),
        "allowance must sit far below the data"
    );
    let mut plan = LargeStorePlan::default_f3();
    plan.row_bytes = 0;
    assert!(plan.check().is_err());
}

#[test]
fn plan_boundary_chunks_straddle_the_former_ceiling() {
    let plan = LargeStorePlan::default_f3();
    let (before, after) = plan.boundary_chunks();
    assert!(before < after);
    let boundary_row = FORMER_CEILING_BYTES / u64::from(plan.row_bytes);
    let before_last_row = (before + 1) * u64::from(plan.rows_per_chunk) - 1;
    let after_first_row = after * u64::from(plan.rows_per_chunk);
    assert!(
        before_last_row < boundary_row || before * u64::from(plan.rows_per_chunk) < boundary_row,
        "the before-chunk holds pre-ceiling rows"
    );
    assert!(
        after_first_row > boundary_row,
        "the after-chunk sits past the ceiling"
    );
    assert!(after < plan.chunks());
}

#[test]
fn beyond_ram_plan_requires_a_real_multiple() {
    let plan = BeyondRamPlan::default_f3();
    plan.check().expect("default admissible");
    assert_eq!(plan.target_payload_bytes(), (2 << 30) * 8);
    assert!(plan.rows() > 0);
    let mut small = plan;
    small.data_multiple = 2;
    assert!(small.check().is_err(), "2x data is not beyond-RAM");
}

#[test]
fn generator_is_deterministic_and_self_identifying() {
    let a = generator::row_payload(7, 3, 100, 42, 64);
    let b = generator::row_payload(7, 3, 100, 42, 64);
    assert_eq!(a, b);
    assert_eq!(a.len(), 64);
    let key = generator::row_key(3, 100, 42);
    assert_eq!(a[..8], key.to_le_bytes(), "rows embed their key");
    assert_ne!(
        generator::row_payload(7, 3, 100, 43, 64)[8..],
        a[8..],
        "different rows differ beyond the key"
    );
    assert_ne!(generator::row_payload(8, 3, 100, 42, 64)[8..], a[8..]);
}

#[test]
fn generator_streams_chunks_resumably_and_respects_total_rows() {
    let mut all = Vec::new();
    for chunk in 0..3u64 {
        generator::stream_chunk(5, chunk, 10, 16, 25, &mut |key, payload| {
            all.push((key, payload));
        });
    }
    assert_eq!(all.len(), 25, "the final chunk is truncated at total_rows");
    for (index, (key, _)) in all.iter().enumerate() {
        assert_eq!(*key, index as u64, "keys are dense and ordered");
    }
    // Resumption: chunk 1 alone equals the middle slice.
    let mut middle = Vec::new();
    generator::stream_chunk(5, 1, 10, 16, 25, &mut |key, payload| {
        middle.push((key, payload));
    });
    assert_eq!(middle[..], all[10..20]);
}

#[test]
fn oracle_checksums_match_between_generator_and_stream_fold() {
    let expected = generator::chunk_checksum(9, 2, 8, 32, 100);
    let mut fold = StreamChecksum::default();
    generator::stream_chunk(9, 2, 8, 32, 100, &mut |key, payload| {
        fold.push(key, &payload);
    });
    let (got, rows) = fold.finish();
    assert_eq!(got, expected);
    assert_eq!(rows, 8);
    // A single mutated row changes the checksum — the oracle sees content,
    // not counts.
    let mut fold = StreamChecksum::default();
    generator::stream_chunk(9, 2, 8, 32, 100, &mut |key, payload| {
        if key == 17 {
            fold.push(key, &generator::mutated_row(payload));
        } else {
            fold.push(key, &payload);
        }
    });
    let (tampered, _) = fold.finish();
    assert_ne!(tampered, expected);
}

#[test]
fn mutated_row_is_a_replacement_with_the_same_key() {
    let original = generator::row_payload(1, 0, 10, 3, 32);
    let mutated = generator::mutated_row(original.clone());
    assert_eq!(mutated.len(), original.len());
    assert_eq!(mutated[..8], original[..8], "the key survives");
    assert_ne!(mutated, original);
}

#[test]
fn enforce_rejects_sparse_files_and_accepts_written_ones() {
    let dir = std::env::temp_dir().join("bumbledb-bench-largefix-tests");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tempdir");
    // Sparse: 8 MiB length, ~0 allocated.
    let sparse = dir.join("sparse.mdb");
    let file = std::fs::File::create(&sparse).expect("create");
    file.set_len(8 << 20).expect("set_len");
    drop(file);
    let err = enforce::assert_populated(&sparse, 8 << 20).expect_err("sparse must refuse");
    assert!(err.contains("sparse"), "refusal names the mechanism: {err}");
    // Written: 1 MiB of real bytes passes a 1 MiB minimum.
    let written = dir.join("written.mdb");
    std::fs::write(&written, vec![0x5Au8; 1 << 20]).expect("write");
    enforce::assert_populated(&written, 1 << 20).expect("written data passes");
    // Too short refuses on length before blocks.
    let err = enforce::assert_populated(&written, 2 << 20).expect_err("short refuses");
    assert!(err.contains("below"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn enforce_memory_bound_is_linux_cgroup_or_not_applicable() {
    match enforce::read_cgroup_memory_max() {
        Ok(evidence) => {
            // On a bounded Linux runner the value is a real limit.
            assert!(evidence.memory_max_bytes > 0);
            assert_eq!(evidence.source, "cgroup-v2 memory.max");
        }
        Err(reason) => {
            // Unlimited/absent cgroup or non-Linux target: a typed refusal,
            // never a silent pass.
            assert!(
                reason.contains("cgroup")
                    || reason.contains("NotApplicable")
                    || reason.contains("memory.max"),
                "refusal names the mechanism: {reason}"
            );
        }
    }
    assert!(enforce::FORBIDDEN_ENFORCEMENT.contains("RLIMIT_AS"));
}
