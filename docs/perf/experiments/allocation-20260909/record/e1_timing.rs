//! Temporary ordinary-release driver; no allocator or CPU instrumentation.
use std::path::Path;
use std::time::{Duration, Instant};

use bumbledb::schema::ValueType;
use bumbledb::{Answers, Db, ParamId, PreparedQuery, Query};
use bumbledb_bench::clockproxy::{self, GhzStamp};
use bumbledb_bench::compare::{self, Answer};
use bumbledb_bench::corpus_gen::{GenConfig, Scale};
use bumbledb_bench::families::{self, Draw, param_args};
use bumbledb_bench::harness::bench_work;
use bumbledb_bench::schema::Ledger;
use bumbledb_bench::translate::ParamSlot;

const DISCARDED: usize = 2;
const COLD_SAMPLES: usize = 16;
const WARMUPS: usize = 8;
const WARM_SAMPLES: usize = 64;
const MEMO_BATCH: u32 = 16;

fn open(path: &Path) -> Result<Db<Ledger>, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match Db::open(path, Ledger, bench_work()) {
            Ok(db) => return Ok(db),
            Err(bumbledb::Error::EnvironmentLocked) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => return Err(format!("saved database open: {error:?}")),
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
    draw: &Draw,
    out: &mut Answers,
) -> Result<(), String> {
    let args = param_args(draw);
    db.read(bench_work(), |snap| snap.execute(prepared, &args, out))
        .map_err(|e| format!("execute: {e:?}"))
}

fn elapsed(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("duration fits u64")
}

fn check(out: &Answers, types: &[ValueType], expected: &[Answer]) -> Result<(), String> {
    let mut actual = compare::from_answers(out, types);
    actual.sort();
    if actual != expected {
        return Err(format!("SQLite mismatch: {actual:?} != {expected:?}"));
    }
    Ok(())
}

fn report(kind: &str, raw: &[u64], batch: u32, stamp: GhzStamp) {
    let mut samples: Vec<_> = raw.iter().map(|ns| ns / u64::from(batch)).collect();
    let stats = bumbledb_bench::harness::stats(&mut samples);
    println!(
        concat!(
            "{{\"kind\":\"{}\",\"batch\":{},\"raw_ns\":{:?},",
            "\"stats\":{{\"min\":{},\"p50\":{},\"p90\":{},\"p95\":{},\"p99\":{},\"max\":{},\"mean_ns\":{}}},",
            "\"ghz\":{{\"pre\":{},\"post\":{},\"threshold\":{},\"contaminated\":{},\"retried\":{}}}}}"
        ),
        kind,
        batch,
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

struct Case<'a> {
    path: &'a Path,
    query: &'a Query,
    target: &'a Draw,
    alternate: &'a Draw,
    expected: &'a [Answer],
    alternate_expected: &'a [Answer],
    types: &'a [ValueType],
}

fn cold(case: &Case<'_>) -> Result<(), String> {
    let ((first, second), stamp) = clockproxy::stamped(|| {
        let mut first = Vec::with_capacity(COLD_SAMPLES);
        let mut second = Vec::with_capacity(COLD_SAMPLES);
        for round in 0..DISCARDED + COLD_SAMPLES {
            let db = open(case.path)?;
            let mut prepared = prepare(&db, case.query)?;
            let mut out = Answers::new();
            // Match the existing reopen protocol: binding-array construction
            // stays outside both timers, open and preparation are excluded.
            let args = param_args(case.target);
            let start = Instant::now();
            db.read(bench_work(), |snap| {
                snap.execute(&mut prepared, &args, &mut out)
            })
            .map_err(|e| format!("first execute: {e:?}"))?;
            let a = elapsed(start);
            std::hint::black_box(out.len());
            check(&out, case.types, case.expected)?;
            let start = Instant::now();
            db.read(bench_work(), |snap| {
                snap.execute(&mut prepared, &args, &mut out)
            })
            .map_err(|e| format!("second execute: {e:?}"))?;
            let b = elapsed(start);
            std::hint::black_box(out.len());
            check(&out, case.types, case.expected)?;
            if round >= DISCARDED {
                first.push(a);
                second.push(b);
            }
        }
        Ok((first, second))
    })?;
    report("cold", &first, 1, stamp);
    report("second", &second, 1, stamp);
    Ok(())
}

fn rotating(case: &Case<'_>) -> Result<(), String> {
    let db = open(case.path)?;
    let mut prepared = prepare(&db, case.query)?;
    let mut out = Answers::new();
    // The alternate runs outside the timer, preventing a result-cache hit on
    // the timed target. Both exact parameter sets are warmed and memoized.
    let (samples, stamp) = clockproxy::stamped(|| {
        let mut samples = Vec::with_capacity(WARM_SAMPLES);
        for round in 0..WARMUPS + WARM_SAMPLES {
            execute(&db, &mut prepared, case.alternate, &mut out)?;
            check(&out, case.types, case.alternate_expected)?;
            let start = Instant::now();
            execute(&db, &mut prepared, case.target, &mut out)?;
            let ns = elapsed(start);
            std::hint::black_box(out.len());
            check(&out, case.types, case.expected)?;
            if round >= WARMUPS {
                samples.push(ns);
            }
        }
        Ok(samples)
    })?;
    report("rotating", &samples, 1, stamp);
    Ok(())
}

fn memoized(case: &Case<'_>) -> Result<(), String> {
    let db = open(case.path)?;
    let mut prepared = prepare(&db, case.query)?;
    let mut out = Answers::new();
    let (samples, stamp) = clockproxy::stamped(|| {
        for _ in 0..WARMUPS {
            execute(&db, &mut prepared, case.target, &mut out)?;
            check(&out, case.types, case.expected)?;
        }
        let mut samples = Vec::with_capacity(WARM_SAMPLES);
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            for _ in 0..MEMO_BATCH {
                execute(&db, &mut prepared, case.target, &mut out)?;
                std::hint::black_box(out.len());
            }
            samples.push(elapsed(start));
            check(&out, case.types, case.expected)?;
        }
        Ok(samples)
    })?;
    report("memoized", &samples, MEMO_BATCH, stamp);
    Ok(())
}

fn main() -> Result<(), String> {
    const { assert!(!cfg!(feature = "alloc-counter"), "ordinary binary only") };
    bumbledb_bench::boost::engage_from_env()?;
    assert!(
        bumbledb_bench::boost::engaged().is_some(),
        "scheduler boost required"
    );
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("expected DB ORACLE FAMILY DRAW_INDEX".into());
    }
    let family = families::all()
        .iter()
        .find(|f| f.name == args[3])
        .ok_or_else(|| "unknown family".to_owned())?;
    assert!(matches!(family.name, "triangle" | "point"));
    let index: usize = args[4].parse().map_err(|e| format!("draw index: {e}"))?;
    let draws = (family.params)(&GenConfig {
        seed: 1,
        scale: Scale::S,
    });
    assert_eq!(draws.len(), 4);
    assert!(index < draws.len());
    let alternate = (index + 1) % draws.len();
    let query = (family.query)();
    let path = Path::new(&args[1]);
    let (types, expected) = {
        let db = open(path)?;
        let mut prepared = prepare(&db, &query)?;
        let types: Vec<_> = prepared
            .signature()
            .columns
            .iter()
            .map(|c| *c.ty())
            .collect();
        let conn = rusqlite::Connection::open_with_flags(
            &args[2],
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| e.to_string())?;
        let order: Vec<_> = (0..draws[0].len())
            .map(|i| ParamSlot::Whole(ParamId(u16::try_from(i).unwrap())))
            .collect();
        // Use the existing hand-written golden, not an engine-derived answer.
        let mut statement = conn.prepare(family.golden_sql).map_err(|e| e.to_string())?;
        let mut expected = Vec::new();
        let mut out = Answers::new();
        for draw in &draws {
            let mut rows = compare::from_sqlite(&mut statement, &order, draw, &types)?;
            rows.sort();
            execute(&db, &mut prepared, draw, &mut out)?;
            check(&out, &types, &rows)?;
            expected.push(rows);
        }
        (types, expected)
    };
    let numeric: Vec<_> = draws[index]
        .iter()
        .map(|p| match p {
            bumbledb_bench::naive::ParamValue::Scalar(bumbledb::Value::U64(v)) => *v,
            _ => panic!("these families have scalar u64 parameters"),
        })
        .collect();
    println!(
        "{{\"family\":\"{}\",\"draw\":{},\"parameters\":{:?},\"alternate\":{},\"answers\":{},\"oracle_draws_checked\":4}}",
        family.name,
        index,
        numeric,
        alternate,
        expected[index].len()
    );
    let case = Case {
        path,
        query: &query,
        target: &draws[index],
        alternate: &draws[alternate],
        expected: &expected[index],
        alternate_expected: &expected[alternate],
        types: &types,
    };
    clockproxy::warm_up(Duration::from_millis(200));
    cold(&case)?;
    rotating(&case)?;
    memoized(&case)?;
    Ok(())
}
