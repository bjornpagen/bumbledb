use super::*;

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
        "--read-batch",
        "16",
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
            read_batch: std::num::NonZeroU32::new(16),
            out: Some(PathBuf::from("artifacts")),
            i_am_lying: true,
        })
    );
    let err = parse(&argv(&["bench", "--frobnicate"])).unwrap_err();
    assert!(err.contains("--frobnicate"), "{err}");
}

#[test]
fn read_batch_is_positive_bounded_and_defaults_to_auto() {
    let Cmd::Bench(default) = parse(&argv(&["bench"])).expect("default parses") else {
        panic!("bench command");
    };
    assert_eq!(default.read_batch, None);
    for batch in ["1", "16"] {
        let Cmd::Bench(args) =
            parse(&argv(&["bench", "--read-batch", batch])).expect("valid batch")
        else {
            panic!("bench command");
        };
        assert_eq!(
            args.read_batch.map(std::num::NonZeroU32::get),
            batch.parse().ok()
        );
    }
    for batch in ["0", "17", "-1", "1.5", "4294967296"] {
        let err = parse(&argv(&["bench", "--read-batch", batch])).expect_err("invalid batch");
        assert!(err.contains("--read-batch"), "{err}");
    }
    assert!(parse(&argv(&["bench", "--read-batch"])).is_err());
}

#[test]
fn native_profile_keeps_the_corpus_and_rejects_invalid_windows() {
    assert_eq!(
        parse(&argv(&[
            "profile",
            "--family",
            "balance",
            "--scale",
            "M",
            "--seed",
            "7",
            "--dir",
            "corpus",
            "--seconds",
            "30",
            "--out",
            "profile-out"
        ])),
        Ok(Cmd::Profile(ProfileArgs {
            corpus: CorpusArgs {
                scale: Scale::M,
                seed: 7,
                dir: PathBuf::from("corpus")
            },
            family: "balance".to_owned(),
            seconds: 30,
            out: Some(PathBuf::from("profile-out")),
        }))
    );
    let cmd = parse(&argv(&["profile", "--family", "point"])).expect("default window");
    assert!(cmd.runs_measurements());
    assert!(matches!(cmd, Cmd::Profile(ProfileArgs { seconds: 10, .. })));
    for seconds in ["0", "3601", "-1", "1.5", "4294967296"] {
        assert!(
            parse(&argv(&[
                "profile",
                "--family",
                "point",
                "--seconds",
                seconds
            ]))
            .is_err()
        );
    }
    for args in [
        vec!["profile"],
        vec!["profile", "--family", ""],
        vec!["profile", "--family", "point", "--trace"],
        vec!["profile", "--family", "point", "--seconds"],
    ] {
        assert!(parse(&argv(&args)).is_err(), "{args:?}");
    }
}

#[test]
fn storage_parses_its_flags() {
    let cmd = parse(&argv(&[
        "storage",
        "--scales",
        "S,M,L",
        "--seed",
        "7",
        "--dir",
        "/tmp/x",
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
fn writes_parses_its_flags() {
    let cmd = parse(&argv(&[
        "writes",
        "--scale",
        "M",
        "--seed",
        "9",
        "--dir",
        "/tmp/w",
        "--batches",
        "1,10,100,1000",
        "--samples",
        "4",
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
            batches: vec![1, 10, 100, 1000],
            samples: Some(4),
            out: Some(PathBuf::from("artifacts")),
        })
    );

    assert_eq!(
        parse(&argv(&["writes"])),
        Ok(Cmd::Writes(WritesArgs::default()))
    );
    assert_eq!(WritesArgs::default().batches, vec![1, 10, 100, 1000]);

    let err = parse(&argv(&["writes", "--batches", "0"])).unwrap_err();
    assert!(err.contains("--batches"), "{err}");
}

#[test]
fn curves_parses_its_flags() {
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
fn heap_parses_its_flags() {
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
            out: Some(PathBuf::from("y")),
        })
    );
}

#[test]
fn crud_refuses_an_unknown_flag() {
    // `crud` takes no corpus flags; the refusal names both the token and the
    // command.
    let err = parse(&argv(&["crud", "--scale", "S"])).unwrap_err();
    assert!(err.contains("--scale"), "{err}");
    assert!(err.contains("crud"), "{err}");
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
    assert!(text.contains("--boost"), "{text}");
    assert!(text.contains("shared_machine"), "{text}");
}

#[test]
fn the_boost_seam_membership_is_pinned() {
    for tokens in [
        vec!["bench"],
        vec!["scenarios"],
        vec!["crud"],
        vec!["lawful"],
        vec!["storage"],
        vec!["writes"],
        vec!["curves"],
        vec!["heap"],
        vec!["micro"],
        vec!["app-perf"],
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
        "storage",
        "writes",
        "curves",
        "heap",
        "queries",
    ] {
        assert!(text.contains(command), "{command}");
    }
}

#[test]
fn app_perf_parses_regimes_and_refuses_unknown_ones() {
    let cmd = parse(&argv(&[
        "app-perf",
        "--scale",
        "M",
        "--regimes",
        "warm,post-write",
        "--tenants",
        "4",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::AppPerf(AppPerfArgs {
            scale: Scale::M,
            regimes: Some(vec![
                crate::harness::appperf::Regime::Warm,
                crate::harness::appperf::Regime::PostWrite
            ]),
            tenants: 4,
            ..AppPerfArgs::default()
        })
    );
    let err = parse(&argv(&["app-perf", "--regimes", "hosted-contention"])).unwrap_err();
    assert!(err.contains("hosted-contention"), "{err}");
    let err = parse(&argv(&["app-perf", "--tenants", "1"])).unwrap_err();
    assert!(err.contains("at least 2"), "{err}");
}

#[test]
fn micro_parses_levels_and_sizes() {
    use crate::harness::micro::{Levels, MicroArgs};
    assert_eq!(
        parse(&argv(&["micro"])),
        Ok(Cmd::Micro(MicroArgs::default()))
    );
    let cmd = parse(&argv(&[
        "micro",
        "--levels",
        "neon,avx2",
        "--elements",
        "4096",
        "--float-rows",
        "1000",
        "--samples",
        "8",
        "--out",
        "docs/perf/runs/2.0.0/micro.json",
    ]))
    .expect("parses");
    assert_eq!(
        cmd,
        Cmd::Micro(MicroArgs {
            levels: Levels::Named(vec!["neon".to_owned(), "avx2".to_owned()]),
            elements: 4096,
            float_rows: 1000,
            samples: 8,
            out: Some(PathBuf::from("docs/perf/runs/2.0.0/micro.json")),
            ..MicroArgs::default()
        })
    );
    for flags in [
        vec!["--levels", ""],
        vec!["--elements", "0"],
        vec!["--float-rows", "0"],
        vec!["--samples", "0"],
        vec!["--nope"],
    ] {
        let mut args = vec!["micro"];
        args.extend(flags);
        assert!(parse(&argv(&args)).is_err(), "accepted {args:?}");
    }
}

#[test]
fn micro_compare_takes_exactly_two_reports() {
    assert_eq!(
        parse(&argv(&["micro", "--compare", "old.json", "new.json"])),
        Ok(Cmd::MicroCompare {
            old: PathBuf::from("old.json"),
            new: PathBuf::from("new.json"),
        })
    );
    assert!(
        !Cmd::MicroCompare {
            old: PathBuf::new(),
            new: PathBuf::new(),
        }
        .runs_measurements()
    );
    assert!(parse(&argv(&["micro", "--compare", "old.json"])).is_err());
    assert!(parse(&argv(&["micro", "--compare", "a", "b", "c"])).is_err());
}

#[test]
fn global_options_precede_the_command() {
    let (globals, cmd) =
        parse_invocation(&argv(&["--boost", "--jobs", "4", "micro"])).expect("parses");
    assert!(globals.boost);
    assert_eq!(globals.jobs.map(std::num::NonZeroUsize::get), Some(4));
    assert!(matches!(cmd, Cmd::Micro(_)));
    assert_eq!(
        parse_invocation(&argv(&["help"])),
        Ok((Globals::default(), Cmd::Help))
    );
    assert!(parse_invocation(&argv(&["--jobs", "0", "micro"])).is_err());
    assert!(parse_invocation(&argv(&["--jobs"])).is_err());
    assert!(parse(&argv(&["micro", "--boost"])).is_err());
}
