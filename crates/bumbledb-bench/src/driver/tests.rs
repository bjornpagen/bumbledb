use super::bench::{obs_missing, stamp_refusal};
use super::*;
use crate::cli::{BenchArgs, CorpusArgs};
use crate::gen::Scale;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-bench-driver-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

/// The marker short-circuits regeneration (the counter hook).
#[test]
fn the_digest_directory_is_reused() {
    let dir = scratch("reuse");
    let mut loads = 0;
    let mut loader = |paths: &CorpusPaths| {
        loads += 1;
        std::fs::create_dir_all(&paths.db).map_err(|e| e.to_string())
    };
    let first = ensure_corpus_with(&dir, CFG, &mut loader).expect("first");
    let second = ensure_corpus_with(&dir, CFG, &mut loader).expect("second");
    assert_eq!(first, second);
    assert_eq!(loads, 1, "the marker short-circuits regeneration");
    // A different seed keys a different directory.
    let other = corpus_paths(
        &dir,
        GenConfig {
            seed: 2,
            scale: Scale::S,
        },
    );
    assert_ne!(first.root, other.root);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_refusal_messages_substitute_the_flags() {
    let corpus = CorpusArgs {
        scale: Scale::M,
        seed: 9,
        dir: PathBuf::from("/tmp/corpora"),
    };
    assert_eq!(
        stamp_refusal(&corpus),
        "bench refuses: no fresh verify stamp for this corpus.\n\
         run first: bumbledb-bench verify --scale M --seed 9 --dir /tmp/corpora\n\
         (or pass --i-am-lying to run unverified — the report will say so)"
    );
    assert_eq!(
        obs_missing("--alloc"),
        "--alloc needs an obs build; run:\n\
         cargo run -p bumbledb-bench --features obs --release -- …"
    );
}

/// An unstamped bench refuses; the message names verify.
#[test]
fn bench_refuses_without_a_stamp() {
    let dir = scratch("refuse");
    let args = BenchArgs {
        corpus: CorpusArgs {
            scale: Scale::S,
            seed: 1,
            dir: dir.clone(),
        },
        families: Some(vec!["point".to_owned()]),
        samples: Some(8),
        trace: false,
        alloc: false,
        proxy_per_rep: false,
        out: Some(dir.join("out")),
        i_am_lying: false,
    };
    let err = cmd_bench(&args).unwrap_err();
    assert!(err.contains("bumbledb-bench verify"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(not(feature = "obs"))]
#[test]
fn alloc_without_obs_names_the_cargo_invocation() {
    let args = BenchArgs {
        corpus: CorpusArgs::default(),
        families: None,
        samples: None,
        trace: false,
        alloc: true,
        proxy_per_rep: false,
        out: None,
        i_am_lying: false,
    };
    let err = cmd_bench(&args).unwrap_err();
    assert!(
        err.contains("cargo run -p bumbledb-bench --features obs --release"),
        "{err}"
    );
}

/// The sweeper's full CLI pipeline on a clean store: gen, then the
/// driver fn (no spawned process) — an empty report and exit code 0.
#[test]
fn verify_store_exits_zero_on_a_clean_corpus() {
    let dir = scratch("verify-store-clean");
    let corpus = CorpusArgs {
        scale: Scale::S,
        seed: 1,
        dir: dir.clone(),
    };
    cmd_gen(&corpus).expect("gen");
    assert_eq!(cmd_verify_store(&corpus).expect("verify-store"), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without a generated corpus the sweeper refuses; the message names gen.
#[test]
fn verify_store_refusal_names_gen() {
    let dir = scratch("verify-store-missing");
    let corpus = CorpusArgs {
        scale: Scale::S,
        seed: 1,
        dir: dir.clone(),
    };
    let err = cmd_verify_store(&corpus).unwrap_err();
    assert!(err.contains("bumbledb-bench gen"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The suite's own integration point (unit-scale by S's size):
/// gen → verify → bench --families point --samples 8, three
/// artifacts, PARTIAL verdict, and the UNVERIFIED override branding.
/// (Re-armed by PRD 06: the naive model seeds closed-relation
/// extensions, so the vocabulary containments commit on both oracles.)
#[test]
fn the_full_sequence_runs_at_s() {
    let dir = scratch("e2e");
    let corpus = CorpusArgs {
        scale: Scale::S,
        seed: 1,
        dir: dir.clone(),
    };
    cmd_gen(&corpus).expect("gen");
    let paths = corpus_paths(&dir, CFG);
    assert!(paths.db.join("data.mdb").exists(), "compacted store");
    assert!(
        !paths.root.join("db-load").exists(),
        "no load-scratch residue (docs/architecture/50-storage.md)"
    );
    assert_eq!(cmd_verify(&corpus, 25).expect("verify"), 0);

    let out = dir.join("out");
    let args = BenchArgs {
        corpus: corpus.clone(),
        families: Some(vec!["point".to_owned()]),
        samples: Some(8),
        trace: false,
        alloc: false,
        proxy_per_rep: false,
        out: Some(out.clone()),
        i_am_lying: false,
    };
    let code = cmd_bench(&args).expect("bench");
    assert!(code == 0 || code == 1, "a gate verdict, not a refusal");
    let mut names: Vec<String> = std::fs::read_dir(&out)
        .expect("read out")
        .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext == "md" || ext == "json")
        })
        .collect();
    names.sort();
    assert_eq!(names, ["QUERIES.md", "report.json", "report.md"]);
    let md = std::fs::read_to_string(out.join("report.md")).expect("read");
    assert!(md.contains("PARTIAL — filtered run"), "{md}");
    assert!(!md.contains("UNVERIFIED"), "verified run");
    assert!(
        md.contains("(families + 25 randomized cases)"),
        "the provenance shows how much evidence earned the stamp: {md}"
    );

    // The override path brands the report.
    let lying = BenchArgs {
        families: Some(vec!["point".to_owned()]),
        out: Some(dir.join("lying-out")),
        i_am_lying: true,
        ..args.clone()
    };
    // Invalidate the stamp by changing the recorded case count.
    std::fs::write(paths.root.join(super::CASES_FILE), "26").expect("tamper");
    cmd_bench(&lying).expect("bench --i-am-lying");
    let md = std::fs::read_to_string(dir.join("lying-out").join("report.md")).expect("read");
    assert!(md.contains("UNVERIFIED"), "{md}");
    let _ = std::fs::remove_dir_all(&dir);
}
