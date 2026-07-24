use super::stamp_value::stamp_value_with;
use super::*;
use crate::corpus_gen::Scale;

fn scratch(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("bumbledb-bench-verify-{tag}"))
}

fn cfg(tag: &str) -> VerifyConfig {
    VerifyConfig {
        corpus_gen: GenConfig {
            seed: 1,
            scale: Scale::S,
        },
        // 25 randomized cases: the debug-build engine takes seconds per
        // heavy random shape, so 25 keeps the full three-lane test
        // inside a unit-test budget (release verify runs use
        // DEFAULT_RANDOM_CASES).
        random_cases: 25,
        out_dir: scratch(tag),
    }
}

#[test]
fn the_stamp_tracks_every_ingredient() {
    let base = cfg("stamp");
    let baseline = stamp_value(&base);
    assert_eq!(baseline, stamp_value(&base), "deterministic");
    let mut seed = base.clone();
    seed.corpus_gen.seed = 2;
    assert_ne!(stamp_value(&seed), baseline, "seed is an ingredient");
    let mut cases = base.clone();
    cases.random_cases = 51;
    assert_ne!(stamp_value(&cases), baseline, "case count is an ingredient");
}

/// The stamp is bound to the binary that
/// earned it. The fingerprint ingredient is blake3 of the running
/// executable, and flipping it flips the stamp — a stamp computed
/// under any other fingerprint is rejected.
#[test]
fn the_stamp_is_bound_to_the_binary() {
    let base = cfg("stamp-binary");
    // The fingerprint is exactly blake3 of the running executable.
    let exe = std::env::current_exe().expect("exe");
    let bytes = std::fs::read(exe).expect("read");
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(&bytes);
    assert_eq!(binary_fingerprint(), digest.finalize());

    // Flipping the fingerprint flips the stamp...
    let mut foreign = binary_fingerprint();
    foreign[0] ^= 0xFF;
    let foreign_stamp = stamp_value_with(&base, &foreign);
    assert_ne!(foreign_stamp, stamp_value(&base));

    // ...and stamp_matches rejects a stamp another binary earned.
    std::fs::create_dir_all(&base.out_dir).expect("dir");
    let path = base.out_dir.join("verify.stamp");
    std::fs::write(&path, &foreign_stamp).expect("write");
    assert!(!stamp_matches(&base, &path));
    std::fs::write(&path, stamp_value(&base)).expect("write");
    assert!(stamp_matches(&base, &path), "this binary's stamp accepts");
    let _ = std::fs::remove_dir_all(&base.out_dir);
}

/// One side erroring where the other answers is a mismatch
/// bundle with an `ERROR:` artifact — never a panic, never a stamp.
#[test]
fn divergence_by_error_is_a_bundle_not_a_panic() {
    let mut config = cfg("error-divergence");
    config.random_cases = 0;
    let failure = run_with_sql_override(&config, |family| {
        (family == "point").then(|| "SELECT this is not sql".to_owned())
    })
    .expect_err("must fail");
    assert!(!failure.bundles.is_empty());
    let theirs = std::fs::read_to_string(failure.bundles[0].join("theirs.txt")).expect("artifact");
    assert!(theirs.starts_with("ERROR:"), "{theirs}");
    let ours = std::fs::read_to_string(failure.bundles[0].join("ours.txt")).expect("artifact");
    assert!(
        ours.contains("answer(s)"),
        "the engine's answers render: {ours}"
    );
    let mismatch =
        std::fs::read_to_string(failure.bundles[0].join("mismatch.txt")).expect("artifact");
    assert!(mismatch.contains("divergence by error"), "{mismatch}");
    assert!(
        !config.out_dir.join("verify.stamp").exists(),
        "no stamp on failure"
    );
    let _ = std::fs::remove_dir_all(&config.out_dir);
}

#[test]
fn stamp_matches_accepts_and_rejects() {
    let base = cfg("stamp-match");
    std::fs::create_dir_all(&base.out_dir).expect("dir");
    let path = base.out_dir.join("verify.stamp");
    assert!(!stamp_matches(&base, &path), "missing file rejects");
    std::fs::write(&path, stamp_value(&base)).expect("write");
    assert!(stamp_matches(&base, &path));
    std::fs::write(&path, "not a stamp").expect("write");
    assert!(!stamp_matches(&base, &path));
    let _ = std::fs::remove_dir_all(&base.out_dir);
}

/// A deliberately wrong SQL for one family fails the run with full
/// arbitration bundles.
#[test]
fn a_wrong_oracle_fails_with_a_bundle() {
    let mut config = cfg("mismatch");
    config.random_cases = 0;
    let failure = run_with_sql_override(&config, |family| {
        (family == "point").then(|| {
            // Off-by-one: the wrong posting's values on every hit.
            "SELECT DISTINCT t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0 \
             WHERE t0.\"id\" = ?1 + 1"
                .to_owned()
        })
    })
    .expect_err("must fail");
    assert!(!failure.bundles.is_empty());
    assert!(failure.to_string().contains("mismatch"));
    for name in [
        "query.txt",
        "query.sql",
        "params.txt",
        "mismatch.txt",
        "golden.sql",
    ] {
        let content = std::fs::read_to_string(failure.bundles[0].join(name)).expect("artifact");
        assert!(!content.is_empty(), "{name} must have content");
    }
    assert!(
        !config.out_dir.join("verify.stamp").exists(),
        "no stamp on failure"
    );
    let _ = std::fs::remove_dir_all(&config.out_dir);
}

/// The full oracle at S: families + 25 randomized cases agree, and the
/// stamp lands. (Re-armed by PRD 06: the naive model seeds
/// closed-relation extensions, so the vocabulary containments commit on
/// both oracles.)
#[test]
fn a_full_verify_at_s_succeeds() {
    let config = cfg("full");
    let report = run(&config).expect("verify succeeds");
    // README.md's published oracle count ("N-case differential oracle";
    // "N cases" under Measurement discipline) is the COMPLETED count of
    // the default release run — this config differs from it in
    // random_cases alone, and only the randomized lane consumes that
    // knob, at exactly four draws per query. Projecting this run to the
    // defaults must therefore land on the README's number (the ca7fc313
    // precedent: the README is trued to the stamp) — any roster or lane
    // change fails HERE until README.md is trued with it.
    assert_eq!(
        report.cases + u64::from(DEFAULT_RANDOM_CASES - config.random_cases) * 4,
        2_876,
        "README.md's oracle case count must equal the default run's completed count"
    );
    let stamp_path = config.out_dir.join("verify.stamp");
    assert!(stamp_matches(&config, &stamp_path));
    // A different config must not accept this stamp.
    let mut other = config.clone();
    other.random_cases += 1;
    assert!(!stamp_matches(&other, &stamp_path));
    let _ = std::fs::remove_dir_all(&config.out_dir);
}
