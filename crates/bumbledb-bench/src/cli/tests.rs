use super::*;

use crate::duralane::DurabilityLane;

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

#[test]
fn help_and_queries_parse() {
    assert_eq!(parse(&argv(&["help"])), Ok(Cmd::Help));
    assert_eq!(parse(&[]), Ok(Cmd::Help));
    assert_eq!(parse(&argv(&["queries"])), Ok(Cmd::Queries));
}

#[test]
fn gen_parses_the_shared_flags() {
    let cmd = parse(&argv(&[
        "gen", "--scale", "M", "--seed", "7", "--dir", "/tmp/x",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Gen(CorpusArgs {
            scale: Scale::M,
            seed: 7,
            dir: PathBuf::from("/tmp/x"),
        })
    );
    let err = parse(&argv(&["gen", "--scale", "XXL"])).unwrap_err();
    assert!(err.contains("XXL"), "{err}");
}

#[test]
fn verify_parses_cases() {
    let cmd = parse(&argv(&["verify", "--cases", "50"])).expect("parses");
    assert_eq!(
        cmd,
        Cmd::Verify {
            corpus: CorpusArgs::default(),
            cases: 50,
        }
    );
    let err = parse(&argv(&["verify", "--cases"])).unwrap_err();
    assert!(err.contains("--cases"), "{err}");
}

#[test]
fn verify_store_parses_the_shared_flags_and_nothing_else() {
    let cmd = parse(&argv(&[
        "verify-store",
        "--scale",
        "L",
        "--seed",
        "3",
        "--dir",
        "/tmp/y",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::VerifyStore(CorpusArgs {
            scale: Scale::L,
            seed: 3,
            dir: PathBuf::from("/tmp/y"),
        })
    );
    let err = parse(&argv(&["verify-store", "--cases", "5"])).unwrap_err();
    assert!(err.contains("--cases"), "{err}");
}

#[test]
fn bench_parses_every_knob() {
    let cmd = parse(&argv(&[
        "bench",
        "--families",
        "point,containment_walk",
        "--samples",
        "8",
        "--trace",
        "--alloc",
        "--ephemeral",
        "--proxy-per-rep",
        "--out",
        "artifacts",
        "--i-am-lying",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Bench(BenchArgs {
            corpus: CorpusArgs::default(),
            families: Some(vec!["point".to_owned(), "containment_walk".to_owned()]),
            samples: Some(8),
            trace: true,
            alloc: true,
            ephemeral: true,
            proxy_per_rep: true,
            out: Some(PathBuf::from("artifacts")),
            i_am_lying: true,
        })
    );
    let err = parse(&argv(&["bench", "--frobnicate"])).unwrap_err();
    assert!(err.contains("--frobnicate"), "{err}");
    let nosync = parse(&argv(&["bench", "--nosync"])).expect("parses");
    assert_eq!(
        nosync,
        Cmd::Bench(BenchArgs {
            ephemeral: true,
            ..BenchArgs {
                corpus: CorpusArgs::default(),
                families: None,
                samples: None,
                trace: false,
                alloc: false,
                ephemeral: false,
                proxy_per_rep: false,
                out: None,
                i_am_lying: false,
            }
        })
    );
}

#[test]
fn trace_requires_a_family() {
    let cmd = parse(&argv(&["trace", "--family", "skew"])).expect("parses");
    assert_eq!(
        cmd,
        Cmd::Trace {
            corpus: CorpusArgs::default(),
            family: "skew".to_owned(),
        }
    );
    let err = parse(&argv(&["trace"])).unwrap_err();
    assert!(err.contains("--family"), "{err}");
}

#[test]
fn sweep_commit_parses_its_knobs() {
    let cmd = parse(&argv(&["sweep-commit"])).expect("parses bare");
    assert_eq!(cmd, Cmd::SweepCommit(SweepArgs::default()));
    let cmd = parse(&argv(&[
        "sweep-commit",
        "--sizes",
        "4,64",
        "--samples",
        "3",
        "--seed",
        "9",
        "--dir",
        "/tmp/z",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::SweepCommit(SweepArgs {
            sizes: Some(vec![4, 64]),
            samples: Some(3),
            seed: 9,
            dir: PathBuf::from("/tmp/z"),
        })
    );
    let err = parse(&argv(&["sweep-commit", "--sizes", "4,x"])).unwrap_err();
    assert!(err.contains("--sizes"), "{err}");
    let err = parse(&argv(&["sweep-commit", "--scale", "S"])).unwrap_err();
    assert!(err.contains("--scale"), "{err}");
}

#[test]
fn storage_parses_the_lane_flags() {
    let cmd = parse(&argv(&[
        "storage",
        "--scales",
        "S,M,L",
        "--seed",
        "7",
        "--dir",
        "/tmp/x",
        "--churn-dir",
        "/tmp/churn",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Storage(StorageArgs {
            scales: vec![Scale::S, Scale::M, Scale::L],
            seed: 7,
            dir: PathBuf::from("/tmp/x"),
            churn_dir: Some(PathBuf::from("/tmp/churn")),
            out: Some(PathBuf::from("artifacts")),
        })
    );

    assert_eq!(
        parse(&argv(&["storage"])),
        Ok(Cmd::Storage(StorageArgs::default()))
    );
    assert_eq!(StorageArgs::default().scales, vec![Scale::S]);

    let err = parse(&argv(&["storage", "--scales", "S,XXL"])).unwrap_err();
    assert!(err.contains("XXL"), "{err}");
    let err = parse(&argv(&["storage", "--scales", ""])).unwrap_err();
    assert!(err.contains("--scales"), "{err}");
}

#[test]
fn writes_parses_the_lane_flags() {
    let cmd = parse(&argv(&[
        "writes",
        "--scale",
        "M",
        "--seed",
        "9",
        "--dir",
        "/tmp/w",
        "--lanes",
        "durable,nosync",
        "--batches",
        "1,10,100,1000",
        "--samples",
        "4",
        "--trace",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Writes(WritesArgs {
            scale: Scale::M,
            seed: 9,
            dir: PathBuf::from("/tmp/w"),
            lanes: vec![DurabilityLane::Durable, DurabilityLane::Nosync],
            batches: vec![1, 10, 100, 1000],
            samples: Some(4),
            trace: true,
            out: Some(PathBuf::from("artifacts")),
        })
    );

    // fsync shadow lands after every nosync sample), the batch ladder.
    assert_eq!(
        parse(&argv(&["writes"])),
        Ok(Cmd::Writes(WritesArgs::default()))
    );
    assert_eq!(
        WritesArgs::default().lanes,
        vec![DurabilityLane::Nosync, DurabilityLane::Durable]
    );
    assert_eq!(WritesArgs::default().batches, vec![1, 10, 100, 1000]);

    let err = parse(&argv(&["writes", "--batches", "0"])).unwrap_err();
    assert!(err.contains("--batches"), "{err}");

    let err = parse(&argv(&["writes", "--lanes", "durable,paranoid"])).unwrap_err();
    assert!(err.contains("paranoid"), "{err}");
}

#[test]
fn curves_parses_the_lane_flags() {
    let cmd = parse(&argv(&[
        "curves",
        "--scales",
        "S,M",
        "--families",
        "triangle,point",
        "--seed",
        "3",
        "--dir",
        "/tmp/c",
        "--samples",
        "8",
        "--cap-ms",
        "5000",
        "--warmth",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Curves(CurvesArgs {
            scales: vec![Scale::S, Scale::M],
            families: Some(vec!["triangle".to_owned(), "point".to_owned()]),
            seed: 3,
            dir: PathBuf::from("/tmp/c"),
            samples: Some(8),
            cap_ms: 5000,
            warmth: true,
            out: Some(PathBuf::from("artifacts")),
        })
    );

    assert_eq!(
        parse(&argv(&["curves"])),
        Ok(Cmd::Curves(CurvesArgs::default()))
    );
    assert_eq!(CurvesArgs::default().cap_ms, 30_000);
    assert!(!CurvesArgs::default().warmth);
    let err = parse(&argv(&["curves", "--cap-ms", "banana"])).unwrap_err();
    assert!(err.contains("banana"), "{err}");
}

#[test]
fn heap_parses_the_lane_flags() {
    let cmd = parse(&argv(&[
        "heap",
        "--scale",
        "S",
        "--seed",
        "2",
        "--dir",
        "/tmp/h",
        "--samples",
        "8",
        "--prefixes",
        "64,256",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Heap(HeapArgs {
            scale: Scale::S,
            seed: 2,
            dir: PathBuf::from("/tmp/h"),
            samples: Some(8),
            prefixes: vec![64, 256],
            out: Some(PathBuf::from("artifacts")),
        })
    );
    assert_eq!(parse(&argv(&["heap"])), Ok(Cmd::Heap(HeapArgs::default())));
    let err = parse(&argv(&["heap", "--prefixes", "0"])).unwrap_err();
    assert!(err.contains('0'), "{err}");
}

#[test]
fn primerlane_parses_the_lane_flags() {
    let cmd = parse(&argv(&[
        "primerlane",
        "--facts",
        "5000",
        "--relations",
        "6",
        "--seed",
        "2",
        "--dir",
        "/tmp/p",
        "--trace",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Primerlane(PrimerlaneArgs {
            facts: 5000,
            relations: 6,
            seed: 2,
            dir: PathBuf::from("/tmp/p"),
            trace: true,
            alloc: false,
            out: Some(PathBuf::from("artifacts")),
        })
    );
    assert_eq!(
        parse(&argv(&["primerlane"])),
        Ok(Cmd::Primerlane(PrimerlaneArgs::default()))
    );
    let err = parse(&argv(&["primerlane", "--relations", "1"])).unwrap_err();
    assert!(err.contains("at least 2"), "{err}");
    let err = parse(&argv(&["primerlane", "--facts", "0"])).unwrap_err();
    assert!(err.contains("rejects 0"), "{err}");
    let err = parse(&argv(&["primerlane", "--trace", "--alloc"])).unwrap_err();
    assert!(err.contains("mutually exclusive"), "{err}");
}

#[test]
fn churn_parses_its_flags() {
    let cmd = parse(&argv(&[
        "churn",
        "--scale",
        "M",
        "--seed",
        "7",
        "--dir",
        "/tmp/churn",
        "--cycles",
        "100",
        "--sample-every",
        "10",
        "--vacuum-every",
        "20",
        "--analyze-every",
        "25",
        "--runs",
        "steady,delete-heavy",
        "--out",
        "artifacts",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Churn(ChurnArgs {
            corpus: CorpusArgs {
                scale: Scale::M,
                seed: 7,
                dir: PathBuf::from("/tmp/churn"),
            },
            cycles: 100,
            sample_every: 10,
            vacuum_every: 20,
            analyze_every: 25,
            runs: Some(vec!["steady".to_owned(), "delete-heavy".to_owned()]),
            out: Some(PathBuf::from("artifacts")),
        })
    );

    assert_eq!(
        parse(&argv(&["churn"])),
        Ok(Cmd::Churn(ChurnArgs::default()))
    );
    assert_eq!(ChurnArgs::default().cycles, 10_000);
    assert_eq!(ChurnArgs::default().sample_every, 250);
    assert!(ChurnArgs::default().runs.is_none());
}

#[test]
fn churn_rejects_unknown_flags() {
    let err = parse(&argv(&["churn", "--bogus", "x"])).unwrap_err();
    assert!(err.contains("--bogus"), "{err}");
    assert!(err.contains("churn"), "{err}");
}

#[test]
fn crud_parses_its_flags() {
    let cmd = parse(&argv(&[
        "crud",
        "--seed",
        "7",
        "--only",
        "crud_insert,crud_rmw",
        "--samples",
        "9",
        "--dir",
        "x",
        "--out",
        "y",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Crud(ScenarioArgs {
            seed: 7,
            dir: PathBuf::from("x"),
            only: Some(vec!["crud_insert".to_owned(), "crud_rmw".to_owned()]),
            samples: Some(9),
            trace: false,
            alloc: false,
            out: Some(PathBuf::from("y")),
        })
    );
}

#[test]
fn lawful_parses_its_flags() {
    let cmd = parse(&argv(&[
        "lawful",
        "--seed",
        "7",
        "--only",
        "law_insert_legal,law_reject_window",
        "--samples",
        "9",
        "--dir",
        "x",
        "--out",
        "y",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Lawful(ScenarioArgs {
            seed: 7,
            dir: PathBuf::from("x"),
            only: Some(vec![
                "law_insert_legal".to_owned(),
                "law_reject_window".to_owned()
            ]),
            samples: Some(9),
            trace: false,
            alloc: false,
            out: Some(PathBuf::from("y")),
        })
    );
}

#[test]
fn crud_refuses_an_unknown_flag() {

    // precedent); the refusal names both the token and the command.
    let err = parse(&argv(&["crud", "--scale", "S"])).unwrap_err();
    assert!(err.contains("--scale"), "{err}");
    assert!(err.contains("crud"), "{err}");
}

#[test]
fn scenarios_parses_the_trace_and_alloc_flags() {
    let Cmd::Scenarios(args) = parse(&argv(&["scenarios", "--trace"])).expect("parses") else {
        panic!("scenarios");
    };
    assert!(args.trace && !args.alloc);
    let Cmd::Scenarios(args) = parse(&argv(&["scenarios", "--alloc"])).expect("parses") else {
        panic!("scenarios");
    };
    assert!(args.alloc && !args.trace);
}

#[test]
fn the_world_commands_refuse_trace_with_alloc() {
    let err = parse(&argv(&["scenarios", "--trace", "--alloc"])).unwrap_err();
    assert!(err.contains("mutually exclusive"), "{err}");
    assert!(err.contains("scenarios"), "{err}");
}

#[test]
fn the_write_worlds_take_trace_and_refuse_alloc() {
    for cmd in ["crud", "lawful"] {
        let parsed = parse(&argv(&[cmd, "--trace"])).expect("parses");
        let args = match parsed {
            Cmd::Crud(args) | Cmd::Lawful(args) => args,
            other => panic!("{cmd}: {other:?}"),
        };
        assert!(args.trace && !args.alloc, "{cmd}");
        let err = parse(&argv(&[cmd, "--alloc"])).unwrap_err();
        assert!(err.contains("no alloc pass"), "{cmd}: {err}");
        assert!(err.contains(cmd), "{cmd}: {err}");
    }
}

#[test]
fn help_names_the_home_turf_worlds() {
    let text = help();
    assert!(text.contains("crud"), "{text}");
    assert!(text.contains("lawful"), "{text}");
}

#[test]
fn help_names_the_shared_machine_boost_switch() {
    let text = help();
    assert!(text.contains("BUMBLEDB_BENCH_BOOST"), "{text}");
    assert!(text.contains("shared_machine"), "{text}");
}

#[test]
fn the_boost_seam_membership_is_pinned() {
    for tokens in [
        vec!["bench"],
        vec!["trace", "--family", "point"],
        vec!["scenarios"],
        vec!["crud"],
        vec!["lawful"],
        vec!["sweep-commit"],
        vec!["storage"],
        vec!["writes"],
        vec!["curves"],
        vec!["churn"],
        vec!["heap"],
        vec!["primerlane"],
    ] {
        let cmd = parse(&argv(&tokens)).expect("parses");
        assert!(cmd.runs_measurements(), "{tokens:?} runs measurements");
    }
    for tokens in [
        vec!["help"],
        vec!["queries"],
        vec!["gen"],
        vec!["verify"],
        vec!["verify-store"],
        vec!["merge", "some-dir"],
    ] {
        let cmd = parse(&argv(&tokens)).expect("parses");
        assert!(!cmd.runs_measurements(), "{tokens:?} never boosts");
    }
}

#[test]
fn garbage_names_the_offending_token() {
    let err = parse(&argv(&["frobnicate"])).unwrap_err();
    assert!(err.contains("frobnicate"), "{err}");
    let err = parse(&argv(&["help", "me"])).unwrap_err();
    assert!(err.contains("me"), "{err}");
}

#[test]
fn help_text_names_the_binary_and_version() {
    let text = help();
    assert!(text.contains("bumbledb-bench"));
    assert!(text.contains(env!("CARGO_PKG_VERSION")));
    for command in [
        "gen",
        "verify",
        "verify-store",
        "bench",
        "trace",
        "sweep-commit",
        "storage",
        "writes",
        "curves",
        "heap",
        "queries",
    ] {
        assert!(text.contains(command), "{command}");
    }
}
