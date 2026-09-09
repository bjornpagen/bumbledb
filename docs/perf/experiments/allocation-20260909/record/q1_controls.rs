use super::*;
use std::path::Path;
use std::time::{Duration, Instant};
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1_control_cases.rs"]
mod cases;
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1-source/crates/bumbledb-bench/src/schema.rs"]
mod saved_schema;

fn open(path: &Path, work: &crate::WorkContext) -> crate::Db<saved_schema::Ledger> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match crate::Db::open(path, saved_schema::Ledger, work.clone()) {
            Ok(db) => return db,
            Err(crate::Error::EnvironmentLocked) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) => panic!("open saved database: {e:?}"),
        }
    }
}
fn sink(label: &str, value: &EitherSink) {
    match value {
        EitherSink::Projection(s) => s.q1_control_report(label),
        EitherSink::Aggregate(s) => s.q1_control_report(label),
        EitherSink::Computed(s) => sink(label, &s.inner),
    }
}
fn owners(label: &str, p: &PreparedQuery<saved_schema::Ledger>, out: &Answers) {
    println!(
        "ANSWER {label} rows={} arity={} cells=({}, {}, {}) text=({}, {}) blob=({}, {})",
        out.len(),
        out.arity(),
        out.cells.len(),
        out.cells.capacity(),
        std::mem::size_of::<Cell>(),
        out.text.len(),
        out.text.capacity(),
        out.blob.len(),
        out.blob.capacity()
    );
    sink(&format!("{label}/main"), &p.sink);
    for (i, interior) in p.pipeline.interiors().iter().enumerate() {
        sink(&format!("{label}/interior{i}"), &interior.sink);
    }
    if let PreparedPipeline::Reach { driver, .. } = &p.pipeline {
        driver.sink.q1_control_report(&format!("{label}/rec"));
    }
}
fn check(out: &Answers, expected: &[Vec<i64>]) {
    let mut actual: Vec<Vec<i64>> = (0..out.len())
        .map(|row| {
            (0..out.arity())
                .map(|col| match out.get(row, col) {
                    AnswerValue::U64(v) => i64::try_from(v).unwrap(),
                    AnswerValue::I64(v) => v,
                    value => panic!("unexpected {value:?}"),
                })
                .collect()
        })
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
}
fn execute(
    label: &str,
    db: &crate::Db<saved_schema::Ledger>,
    p: &mut PreparedQuery<saved_schema::Ledger>,
    out: &mut Answers,
    n: u64,
    expected: &[Vec<i64>],
    work: &crate::WorkContext,
) {
    let args = [BindValue::U64(n)];
    crate::alloc_counter::reset();
    db.read(work.clone(), |s| s.execute(p, &args, out)).unwrap();
    let counter = crate::alloc_counter::snapshot().window;
    check(out, expected);
    println!("ALLOC {label} {counter:?}");
    owners(label, p, out);
    println!("PASS {label}");
}
#[test]
#[ignore = "bounded saved-corpus allocation controls; no timing or CPU trace"]
fn saved_controls() {
    assert!(cfg!(feature = "alloc-counter"));
    let path = std::env::var_os("BUMBLEDB_Q1_DB").unwrap();
    let path = Path::new(&path);
    let oracle = std::env::var_os("BUMBLEDB_Q1_ORACLE").unwrap();
    let oracle = Path::new(&oracle);
    let work = crate::WorkContext::new();
    let mut windows = 0;
    let mut count = 0;
    for &(name, draws, sql) in cases::CASES {
        let query = cases::query(name);
        let small = cases::expected(oracle, sql, 0);
        for &n in draws {
            let expected = cases::expected(oracle, sql, n);
            let db = open(path, &work);
            let mut out = Answers::new();
            let label = format!("{name}:{n}:prepare");
            crate::alloc_counter::reset();
            let mut p = db.prepare(&query, work.clone()).unwrap();
            let counter = crate::alloc_counter::snapshot().window;
            println!("ALLOC {label} {counter:?}");
            owners(&label, &p, &out);
            windows += 1;
            for (phase, bound, answer) in [
                ("cold", n, &expected),
                ("warm", n, &expected),
                ("small", 0, &small),
                ("return", n, &expected),
            ] {
                execute(
                    &format!("{name}:{n}:{phase}"),
                    &db,
                    &mut p,
                    &mut out,
                    bound,
                    answer,
                    &work,
                );
                windows += 1;
            }
            let label = format!("{name}:{n}:release");
            crate::alloc_counter::reset();
            p.release_memory();
            let counter = crate::alloc_counter::snapshot().window;
            check(&out, &expected);
            println!("ALLOC {label} {counter:?}");
            owners(&label, &p, &out);
            windows += 1;
            for phase in ["refill", "warm_refill"] {
                execute(
                    &format!("{name}:{n}:{phase}"),
                    &db,
                    &mut p,
                    &mut out,
                    n,
                    &expected,
                    &work,
                );
                windows += 1;
            }
            count += 1;
        }
    }
    println!("PASS controls cases={count} windows={windows}");
}
