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
    for command in ["gen", "verify", "verify-store", "bench", "trace", "queries"] {
        assert!(text.contains(command), "{command}");
    }
}
