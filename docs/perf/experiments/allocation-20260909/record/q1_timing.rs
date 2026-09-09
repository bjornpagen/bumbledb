//! Ordinary Q1 timing: paired prepare/first/combined, second, and rollover-length warm runs.
use std::path::Path;
use std::time::{Duration, Instant};

use bumbledb::{AnswerValue, Answers, BindValue, Db, PreparedQuery, Query, Value};
use bumbledb_bench::clockproxy::{self, GhzStamp};
use bumbledb_bench::corpus_gen::{GenConfig, Scale};
use bumbledb_bench::families;
use bumbledb_bench::harness::bench_work;
use bumbledb_bench::naive::ParamValue;
use bumbledb_bench::schema::Ledger;

#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1_control_cases.rs"]
#[allow(
    clippy::missing_panics_doc,
    clippy::unreadable_literal,
    reason = "frozen allocation fixtures are shared verbatim; not a public API"
)]
mod controls;

const COLD_DISCARDED: usize = 2;
const COLD_SAMPLES: usize = 16;
const WARMUPS: usize = 8;
// Cross a full u8 generation cycle even for tiny same-target maps. No
// timed batching: retain the individual periodic-clear tail, not its average.
const WARM_SAMPLES: usize = 320;

fn open(path: &Path) -> Result<Db<Ledger>, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match Db::open(path, Ledger, bench_work()) {
            Ok(db) => return Ok(db),
            Err(bumbledb::Error::EnvironmentLocked) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => return Err(format!("saved DB open: {error:?}")),
        }
    }
}

fn prepare(db: &Db<Ledger>, query: &Query) -> Result<PreparedQuery<Ledger>, String> {
    db.prepare(query, bench_work())
        .map_err(|e| format!("prepare: {e:?}"))
}

fn execute(
    db: &Db<Ledger>,
    prepared: &mut PreparedQuery<Ledger>,
    args: &[BindValue<'_>],
    out: &mut Answers,
) -> Result<(), String> {
    db.read(bench_work(), |snap| snap.execute(prepared, args, out))
        .map_err(|e| format!("execute: {e:?}"))
}

fn ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("duration fits u64")
}

fn check(out: &Answers, expected: &[Vec<i64>]) -> Result<(), String> {
    if !(1..=2).contains(&out.arity()) {
        return Err("focused numeric fixtures have one or two columns".into());
    }
    // Verification is outside the timer. Avoid a heap allocation per row
    // when checking the large controls, while still checking exact values.
    let mut actual: Vec<[i64; 2]> = (0..out.len())
        .map(|row| {
            let mut words = [0; 2];
            for (col, word) in words[..out.arity()].iter_mut().enumerate() {
                *word = match out.get(row, col) {
                    AnswerValue::U64(v) => i64::try_from(v).expect("saved numeric result fits i64"),
                    AnswerValue::I64(v) => v,
                    other => panic!("unexpected numeric answer {other:?}"),
                };
            }
            words
        })
        .collect();
    actual.sort_unstable();
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(expected)
            .any(|(a, b)| a[..out.arity()] != **b)
    {
        return Err(format!(
            "SQL result mismatch: {} actual vs {} expected rows",
            actual.len(),
            expected.len()
        ));
    }
    Ok(())
}

fn report(kind: &str, raw: &[u64], stamp: GhzStamp) {
    let mut sorted = raw.to_vec();
    let stats = bumbledb_bench::harness::stats(&mut sorted);
    println!(
        concat!(
            "{{\"kind\":\"{}\",\"batch\":1,\"raw_ns\":{:?},",
            "\"stats\":{{\"min\":{},\"p50\":{},\"p90\":{},\"p95\":{},\"p99\":{},\"max\":{},\"mean_ns\":{}}},",
            "\"ghz\":{{\"pre\":{},\"post\":{},\"threshold\":{},\"contaminated\":{},\"retried\":{}}}}}"
        ),
        kind,
        raw,
        stats.min,
        stats.p50,
        stats.p90,
        stats.p95,
        stats.p99,
        stats.max,
        stats.mean_ns,
        stamp.pre,
        stamp.post,
        stamp.threshold,
        stamp.contaminated(),
        stamp.retried
    );
}

struct Input {
    query: Query,
    draws: Vec<Vec<Value>>,
    expected: Vec<Vec<Vec<i64>>>,
}

fn input(family: &str, oracle: &Path) -> Result<Input, String> {
    if let Some(name) = family.strip_prefix("saved_") {
        let entry = families::all()
            .iter()
            .find(|f| f.name == name)
            .ok_or("unknown saved family")?;
        if !matches!(name, "triangle" | "point" | "range") {
            return Err("saved family outside focused scope".into());
        }
        let draws: Vec<Vec<Value>> = (entry.params)(&GenConfig {
            seed: 1,
            scale: Scale::S,
        })
        .into_iter()
        .map(|draw| {
            draw.into_iter()
                .map(|value| match value {
                    ParamValue::Scalar(v) => v,
                    ParamValue::Set(_) => panic!("scalar saved family"),
                })
                .collect()
        })
        .collect();
        let conn = rusqlite::Connection::open_with_flags(
            oracle,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;
        let mut statement = conn.prepare(entry.golden_sql).map_err(|e| e.to_string())?;
        let width = statement.column_count();
        let mut expected = Vec::new();
        for draw in &draws {
            let numeric: Vec<i64> = draw
                .iter()
                .map(|v| match v {
                    Value::U64(v) => i64::try_from(*v).unwrap(),
                    Value::I64(v) => *v,
                    _ => panic!("saved numeric params"),
                })
                .collect();
            let mut rows: Vec<Vec<i64>> = statement
                .query_map(rusqlite::params_from_iter(numeric), |row| {
                    (0..width).map(|i| row.get(i)).collect()
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            rows.sort_unstable();
            expected.push(rows);
        }
        Ok(Input {
            query: (entry.query)(),
            draws,
            expected,
        })
    } else {
        let (_, draws, sql) = controls::CASES
            .iter()
            .find(|(name, ..)| *name == family)
            .ok_or("unknown control")?;
        Ok(Input {
            query: controls::query(family),
            draws: draws.iter().map(|n| vec![Value::U64(*n)]).collect(),
            expected: draws
                .iter()
                .map(|n| controls::expected(oracle, sql, *n))
                .collect(),
        })
    }
}

struct Case<'a> {
    path: &'a Path,
    input: &'a Input,
    target: usize,
    alternate: usize,
}

fn cold(case: &Case<'_>) -> Result<(), String> {
    let args = families::bind_values(&case.input.draws[case.target]);
    let ((preparation, first, combined, second), stamp) = clockproxy::stamped(|| {
        let mut preparation = Vec::with_capacity(COLD_SAMPLES);
        let mut first = Vec::with_capacity(COLD_SAMPLES);
        let mut combined = Vec::with_capacity(COLD_SAMPLES);
        let mut second = Vec::with_capacity(COLD_SAMPLES);
        for round in 0..COLD_DISCARDED + COLD_SAMPLES {
            let db = open(case.path)?;
            let mut out = Answers::new();
            // Open, AST/argument construction and empty Answers construction
            // are excluded. The joint interval includes both operations and
            // the small intervening timestamp reads; never summed percentiles.
            let start = Instant::now();
            let mut p = prepare(&db, &case.input.query)?;
            let prepare_ns = ns(start);
            let exec_start = Instant::now();
            execute(&db, &mut p, &args, &mut out)?;
            let first_ns = ns(exec_start);
            let combined_ns = ns(start);
            std::hint::black_box(out.len());
            check(&out, &case.input.expected[case.target])?;
            let start = Instant::now();
            execute(&db, &mut p, &args, &mut out)?;
            let second_ns = ns(start);
            std::hint::black_box(out.len());
            check(&out, &case.input.expected[case.target])?;
            if round >= COLD_DISCARDED {
                preparation.push(prepare_ns);
                first.push(first_ns);
                combined.push(combined_ns);
                second.push(second_ns);
            }
        }
        Ok((preparation, first, combined, second))
    })?;
    for (kind, raw) in [
        ("prepare", preparation),
        ("first", first),
        ("combined", combined),
        ("second", second),
    ] {
        report(kind, &raw, stamp);
    }
    Ok(())
}

fn warm(case: &Case<'_>, alternating: bool) -> Result<(), String> {
    let db = open(case.path)?;
    let mut p = prepare(&db, &case.input.query)?;
    let mut out = Answers::new();
    let target = families::bind_values(&case.input.draws[case.target]);
    let other = families::bind_values(&case.input.draws[case.alternate]);
    let (samples, stamp) = clockproxy::stamped(|| {
        let mut samples = Vec::with_capacity(WARM_SAMPLES);
        for round in 0..WARMUPS + WARM_SAMPLES {
            if alternating {
                execute(&db, &mut p, &other, &mut out)?;
                check(&out, &case.input.expected[case.alternate])?;
            }
            let start = Instant::now();
            execute(&db, &mut p, &target, &mut out)?;
            let elapsed = ns(start);
            std::hint::black_box(out.len());
            check(&out, &case.input.expected[case.target])?;
            if round >= WARMUPS {
                samples.push(elapsed);
            }
        }
        Ok(samples)
    })?;
    report(
        if alternating {
            "alternating"
        } else {
            "same_target"
        },
        &samples,
        stamp,
    );
    Ok(())
}

fn main() -> Result<(), String> {
    const { assert!(!cfg!(feature = "alloc-counter"), "ordinary binary only") };
    bumbledb_bench::boost::engage_from_env()?;
    assert!(
        bumbledb_bench::boost::engaged().is_some(),
        "verified scheduler boost required"
    );
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("expected DB ORACLE FAMILY DRAW_INDEX".into());
    }
    let target: usize = args[4].parse().map_err(|e| format!("draw index: {e}"))?;
    let input = input(&args[3], Path::new(&args[2]))?;
    if target >= input.draws.len() {
        return Err("draw outside fixture".into());
    }
    let alternate = (target + 1) % input.draws.len();
    let numeric: Vec<i64> = input.draws[target]
        .iter()
        .map(|v| match v {
            Value::U64(v) => i64::try_from(*v).unwrap(),
            Value::I64(v) => *v,
            _ => panic!("numeric params"),
        })
        .collect();
    let kinds: Vec<_> = input.draws[target]
        .iter()
        .map(|v| match v {
            Value::U64(_) => "u64",
            Value::I64(_) => "i64",
            _ => unreachable!(),
        })
        .collect();
    println!(
        "{{\"family\":\"{}\",\"draw\":{},\"parameters\":{:?},\"parameter_types\":{:?},\"alternate\":{},\"answers\":{},\"alternate_answers\":{},\"sql_draws\":{},\"warmups\":{},\"warm_samples\":{}}}",
        args[3],
        target,
        numeric,
        kinds,
        alternate,
        input.expected[target].len(),
        input.expected[alternate].len(),
        input.draws.len(),
        WARMUPS,
        WARM_SAMPLES
    );
    let case = Case {
        path: Path::new(&args[1]),
        input: &input,
        target,
        alternate,
    };
    clockproxy::warm_up(Duration::from_millis(200));
    cold(&case)?;
    warm(&case, false)?;
    warm(&case, true)?;
    Ok(())
}
