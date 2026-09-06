//! Native sampling workloads. These are diagnostics, never timing
//! scores. Filter sampled stacks to `profile_read_window` to exclude opening,
//! preparation, warming, answer fingerprinting, and report serialization.
//! Displaced families retain their explicit foreign stream between reads.
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use bumbledb::schema::Theory;
use bumbledb::{Answers, Db, Query};

use crate::cli::ProfileArgs;
use crate::families::{Draw, param_args};
use crate::harness::bench_work;
use crate::{calendar, families, report};

use super::bench::{stamp_is_fresh, stamp_refusal};
use super::corpus::gen_config;
use super::corpus_paths;

/// Profile a registered read on an existing verified corpus, or a scenario
/// query after its ordinary SQLite oracle gate on a fresh scenario corpus.
/// Each cycle includes every registered parameter draw, including misses.
/// # Errors
/// Refuses unknown families, unverified stores, and output overwrites.
pub fn cmd_profile(args: &ProfileArgs) -> Result<(), String> {
    if cfg!(feature = "alloc-counter") {
        return Err("native profile needs a build without `alloc-counter`".to_owned());
    }
    let cfg = gen_config(&args.corpus);
    let paths = corpus_paths(&args.corpus.dir, cfg);
    let out = args.out.clone().unwrap_or_else(|| {
        Path::new("bench-out").join(format!(
            "profile-{}-{}",
            args.family,
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    let report_path = out.join("workload.json");
    if out.exists() {
        return Err(format!("profile refuses to overwrite {}", out.display()));
    }
    let require_stamp = || {
        if stamp_is_fresh(&paths, cfg) {
            Ok(())
        } else {
            Err(stamp_refusal(&args.corpus))
        }
    };
    let ledger = families::all();
    let scheduling = calendar::families::all();
    let (result, corpus) = if let Some(family) = ledger.iter().find(|f| f.name == args.family) {
        require_stamp()?;
        let db = Db::open(&paths.db, crate::schema::Ledger, bench_work())
            .map_err(|e| format!("profile open: {e:?}"))?;
        (
            profile_query(&db, &(family.query)(), &(family.params)(&cfg), args, || {})?,
            paths.root,
        )
    } else if let Some(family) = scheduling.iter().find(|f| f.name == args.family) {
        require_stamp()?;
        let db = Db::open(&paths.cal_db, calendar::Scheduling, bench_work())
            .map_err(|e| format!("profile open: {e:?}"))?;
        (
            profile_query(&db, &(family.query)(), &(family.params)(&cfg), args, || {})?,
            paths.root,
        )
    } else if let Some(result) = profile_generated(&out.join("corpus"), args)? {
        (result, out.join("corpus"))
    } else if let Some(result) = crate::scenarios::profile(&out.join("corpus"), args)? {
        (result, out.join("corpus"))
    } else {
        return Err(format!(
            "unknown read/scenario profile family `{}`",
            args.family
        ));
    };
    std::fs::create_dir_all(&out).map_err(|e| format!("profile output: {e}"))?;
    let executable = std::env::current_exe().map_err(|e| format!("profile executable: {e}"))?;
    let json = render_report(args, &result, &corpus, &executable);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&report_path)
        .and_then(|mut file| file.write_all(json.as_bytes()))
        .map_err(|e| format!("profile report: {e}"))?;
    println!("profile workload: {}", report_path.display());
    Ok(())
}

fn profile_generated(dir: &Path, args: &ProfileArgs) -> Result<Option<ProfileResult>, String> {
    let cfg = gen_config(&args.corpus);
    let mode = crate::storemode::StoreMode::Durable;
    let fresh = || {
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("profile parent: {e}"))?;
        }
        std::fs::create_dir(dir).map_err(|e| format!("fresh profile corpus: {e}"))
    };
    if let Some(family) = crate::closure::all().iter().find(|f| f.name == args.family) {
        fresh()?;
        let (db, conn) = crate::closure::load_stores(dir, cfg, mode)?;
        let draws = (family.params)(&cfg);
        crate::closure::verify_family(&db, &conn, family, &draws)?;
        return profile_query(&db, &(family.query)(), &draws, args, || {}).map(Some);
    }
    if let Some(family) = crate::displaced::all()
        .iter()
        .find(|f| f.name == args.family)
    {
        fresh()?;
        let (db, conn) = crate::displaced::load_stores(dir, cfg, mode)?;
        crate::displaced::verify_family(&db, &conn, family)?;
        let mut foreign = crate::displaced::ForeignStream::new(family.displace_mib);
        return profile_query(
            &db,
            &(family.query)(),
            &crate::displaced::draws(),
            args,
            || {
                foreign.stream();
            },
        )
        .map(Some);
    }
    Ok(None)
}

#[derive(Debug)]
pub(crate) struct ProfileResult {
    pub(crate) draws: usize,
    pub(crate) cycles: u64,
    pub(crate) rows_per_cycle: u64,
    rows_per_draw: Vec<u64>,
    elapsed: Duration,
    input_digest: String,
    answer_digest: String,
}

fn profile_query<S: Theory>(
    db: &Db<S>,
    query: &Query,
    draws: &[Draw],
    args: &ProfileArgs,
    mut before_read: impl FnMut(),
) -> Result<ProfileResult, String> {
    if draws.is_empty() {
        return Err("profile needs a nonempty registered parameter stream".to_owned());
    }
    let mut prepared = db
        .prepare(query, bench_work())
        .map_err(|e| format!("profile prepare: {e:?}"))?;
    let mut answers = Answers::new();
    let input_digest = input_fingerprint(&(
        bumbledb::schema::fingerprint::fingerprint(db.schema()),
        prepared.rendered_query(),
        draws,
    ));
    let mut digest = bumbledb::digest::Digest::new();
    let mut expected = Vec::with_capacity(draws.len());
    for draw in draws {
        let params = param_args(draw);
        db.read(bench_work(), |snap| {
            snap.execute(&mut prepared, &params, &mut answers)
        })
        .map_err(|e| format!("profile fingerprint: {e:?}"))?;
        expected.push(answers.len() as u64);
        fingerprint_answers(&answers, &mut digest);
    }
    profile_cycles(
        args,
        &expected,
        input_digest,
        crate::corpus_gen::digest_hex(&digest.finalize()),
        |index| {
            before_read();
            let params = param_args(&draws[index]);
            db.read(bench_work(), |snap| {
                snap.execute(&mut prepared, &params, &mut answers)
            })
            .map_err(|e| format!("profile execute: {e:?}"))?;
            Ok(std::hint::black_box(&answers).len() as u64)
        },
    )
}

/// All native read diagnostics share this complete-draw sampling protocol.
/// The caller gates values before entering; each draw's cardinality is checked
/// here, not just their sum (swapped hit/miss counts must not cancel).
pub(crate) fn profile_cycles(
    args: &ProfileArgs,
    expected: &[u64],
    input_digest: String,
    answer_digest: String,
    mut read: impl FnMut(usize) -> Result<u64, String>,
) -> Result<ProfileResult, String> {
    if expected.is_empty() {
        return Err("profile needs a nonempty registered parameter stream".to_owned());
    }
    let mut cycle = || {
        for (index, &rows) in expected.iter().enumerate() {
            if read(index)? != rows {
                return Err(format!("profile answers changed for draw {index}"));
            }
        }
        Ok(())
    };
    // Whole cycles preserve the production draw mix; no synthetic hot key.
    for _ in 0..8 {
        cycle()?;
    }
    eprintln!("profile: {} — starting complete-draw window", args.family);
    let (cycles, elapsed) =
        profile_read_window(Duration::from_secs(u64::from(args.seconds)), &mut cycle)?;
    Ok(ProfileResult {
        draws: expected.len(),
        cycles,
        rows_per_cycle: expected.iter().sum(),
        rows_per_draw: expected.to_vec(),
        elapsed,
        input_digest,
        answer_digest,
    })
}

/// Untimed descriptor/draw identity. Answer equality alone cannot distinguish
/// a changed stream of misses or a different query with the same result.
pub(crate) fn input_fingerprint(input: &impl std::fmt::Debug) -> String {
    let mut digest = bumbledb::digest::Digest::new();
    digest.update(format!("{input:?}").as_bytes());
    crate::corpus_gen::digest_hex(&digest.finalize())
}

fn fingerprint_answers(answers: &Answers, digest: &mut bumbledb::digest::Digest) {
    let mut rows: Vec<_> = answers
        .answers()
        .map(|answer| {
            format!(
                "{:?}",
                (0..answers.arity())
                    .map(|column| answer.get(column))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    rows.sort_unstable();
    digest.update(&(rows.len() as u64).to_le_bytes());
    for row in rows {
        digest.update(&(row.len() as u64).to_le_bytes());
        digest.update(row.as_bytes());
    }
}

// Keep one identifiable sampling boundary; do not outline the engine itself.
#[inline(never)]
fn profile_read_window(
    duration: Duration,
    cycle: &mut impl FnMut() -> Result<(), String>,
) -> Result<(u64, Duration), String> {
    let started = Instant::now();
    let mut cycles = 0;
    loop {
        cycle()?;
        cycles += 1;
        let elapsed = started.elapsed();
        if elapsed >= duration {
            return Ok((cycles, elapsed));
        }
    }
}

fn render_report(
    args: &ProfileArgs,
    result: &ProfileResult,
    corpus: &Path,
    executable: &Path,
) -> String {
    let mut out = String::from(
        "{\n  \"kind\": \"native-profile-workload\",\n  \"protocol\": 1,\n  \"diagnostic_only\": true",
    );
    for (key, value) in [
        ("family", args.family.clone()),
        ("scale", args.corpus.scale.label().to_owned()),
        ("corpus", corpus.display().to_string()),
        ("executable", executable.display().to_string()),
        (
            "binary_fingerprint",
            crate::corpus_gen::digest_hex(&crate::verify::binary_fingerprint()),
        ),
        ("answer_digest", result.answer_digest.clone()),
        ("input_digest", result.input_digest.clone()),
        ("sampling_root", "profile_read_window".to_owned()),
        ("git_rev", report::git_rev(Path::new("."))),
    ] {
        let _ = write!(out, ",\n  \"{key}\": ");
        crate::json::push_str_lit(&mut out, &value);
    }
    let _ = write!(
        out,
        ",\n  \"process_id\": {},\n  \"seed\": {},\n  \"displace_bytes_per_draw\": {},\n  \"requested_seconds\": {},\n  \"elapsed_ns\": {},\n  \"draws_per_cycle\": {},\n  \"cycles\": {},\n  \"rows_per_cycle\": {},\n  \"rows_per_draw\": {:?}\n}}\n",
        std::process::id(),
        args.corpus.seed,
        crate::displaced::all()
            .iter()
            .find(|family| family.name == args.family)
            .map_or(0, |family| family.displace_mib << 20),
        args.seconds,
        result.elapsed.as_nanos(),
        result.draws,
        result.cycles,
        result.rows_per_cycle,
        result.rows_per_draw,
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_cycles_preserve_every_draw_and_refuse_changed_counts_or_errors() {
        let args = ProfileArgs {
            corpus: crate::cli::CorpusArgs::default(),
            family: "protocol-check".to_owned(),
            seconds: 0,
            out: None,
        };
        let expected = [0, 3, 1];
        let mut observed = Vec::new();
        let result = profile_cycles(
            &args,
            &expected,
            "input".to_owned(),
            "digest".to_owned(),
            |index| {
                observed.push(index);
                Ok(expected[index])
            },
        )
        .expect("eight warm cycles and one complete measured cycle");
        assert_eq!(
            (result.draws, result.cycles, result.rows_per_cycle),
            (3, 1, 4)
        );
        assert_eq!(observed, (0..9).flat_map(|_| 0..3).collect::<Vec<_>>());
        assert_eq!(result.rows_per_draw, expected);
        // Same aggregate row count as [1, 0], but each draw is wrong.
        assert_eq!(
            profile_cycles(&args, &[1, 0], String::new(), String::new(), |index| Ok(
                index as u64
            ))
            .unwrap_err(),
            "profile answers changed for draw 0",
        );
        let mut calls = 0;
        assert_eq!(
            profile_cycles(&args, &[1], String::new(), String::new(), |_| {
                calls += 1;
                if calls > 8 {
                    Err("read refused".to_owned())
                } else {
                    Ok(1)
                }
            })
            .unwrap_err(),
            "read refused",
        );
        assert!(
            profile_cycles(&args, &[], String::new(), String::new(), |_| panic!(
                "no draws"
            ))
            .is_err()
        );
    }

    #[test]
    fn sampling_window_finishes_a_cycle_and_propagates_refusal() {
        let mut calls = 0;
        let (cycles, _) = profile_read_window(Duration::ZERO, &mut || {
            calls += 1;
            Ok(())
        })
        .expect("one complete cycle");
        assert_eq!((cycles, calls), (1, 1));
        let error = profile_read_window(Duration::ZERO, &mut || Err("refused".to_owned()))
            .expect_err("failures cannot turn into profile results");
        assert_eq!(error, "refused");
    }
}
