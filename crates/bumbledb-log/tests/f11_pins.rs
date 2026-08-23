//! F11 — performance pins: recorded, not asserted. Every harness here
//! is ignored by default; run one with `--ignored --nocapture` and it
//! prints `PIN f11` lines carrying the measured figures with their
//! attribution. Nothing gates on a value — the numbers ride the release
//! receipt, and the only assertions are shape checks proving each
//! harness measured the path it names (the loser counters fired, the
//! verdicts matched). The S3 smoke is credential-gated and skips
//! loudly.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod lane_e_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Barrier;
use std::time::{Duration, Instant};

use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::braids::BraidId;
use bumbledb_log::footprint::footprint;
use bumbledb_log::intersect::{LoserDecision, intersect};
use bumbledb_log::manifest::{Manifest, ckpt_mdb_key, log_key, manifest_key};
use bumbledb_log::replica::{Opened, Provenance, Refreshed, Replica};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Counters, Error, Options, Writer, WriterOpened};
use lane_e_support::{
    Competitor, NOTE, RECIPE, RacingStore, TestLog, VENUE, codec, insert, kitchen_braid,
    note_braid, note_row, recipe_row, temp_dir, theory, venue_braid,
};

type FsWriter = Writer<SchemaDescriptor, FsStore>;

fn open_writer(root: PathBuf, dir: &Path, writer_id: u64) -> FsWriter {
    match Writer::open(
        FsStore::new(root),
        "",
        dir,
        theory(),
        Options::new(writer_id),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn venue_row(id: u64) -> Box<[Value]> {
    Box::from([Value::U64(id)])
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e6
}

/// Sorts in place and prints one `PIN` line of order statistics.
fn report(label: &str, samples: &mut [Duration]) {
    samples.sort_unstable();
    let n = samples.len();
    let at = |q: f64| samples[((n - 1) as f64 * q).round() as usize];
    let mean = samples.iter().sum::<Duration>() / u32::try_from(n).expect("sample count fits");
    println!(
        "PIN f11 {label}: n={n} min={:.1}us p50={:.1}us p90={:.1}us p99={:.1}us mean={:.1}us",
        micros(samples[0]),
        micros(at(0.5)),
        micros(at(0.9)),
        micros(at(0.99)),
        micros(mean),
    );
}

fn median(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

/// xorshift64* — decorrelation is the whole requirement here.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Zipfian over ranks 0..n via the precomputed CDF; theta 0 is uniform.
struct Zipf {
    cdf: Vec<f64>,
}

impl Zipf {
    fn new(n: usize, theta: f64) -> Self {
        let weights: Vec<f64> = (1..=n).map(|rank| (rank as f64).powf(-theta)).collect();
        let total: f64 = weights.iter().sum();
        let mut acc = 0.0;
        let cdf = weights
            .into_iter()
            .map(|w| {
                acc += w / total;
                acc
            })
            .collect();
        Self { cdf }
    }

    fn sample(&self, rng: &mut Rng) -> u64 {
        let u = rng.unit();
        self.cdf.partition_point(|c| *c < u) as u64
    }
}

fn timed_note<S: ObjectStore + 'static>(writer: &Writer<SchemaDescriptor, S>, id: u64) -> Duration {
    let start = Instant::now();
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(id, "pin")]);
            Ok(())
        })
        .expect("note commit");
    let elapsed = start.elapsed();
    assert!(matches!(outcome, Commit::Accepted { .. }));
    elapsed
}

/// Publishes one checkpoint now (one extra note commit crosses the
/// cadence) and restores the huge cadence, so a later discard re-seeds
/// from a near-tip checkpoint — the production shape for loss costs.
fn force_checkpoint<S: ObjectStore + 'static>(writer: &Writer<SchemaDescriptor, S>, id: u64) {
    writer.set_checkpoint_cadence(1, 1);
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(id, "ckpt")]);
            Ok(())
        })
        .expect("cadence commit");
    assert!(matches!(outcome, Commit::Accepted { .. }));
    writer.quiesce();
    writer.set_checkpoint_cadence(u64::MAX, u64::MAX);
}

fn braid_tip(store: &FsStore, braid: BraidId) -> u64 {
    let mut g = 0;
    while store
        .get(&log_key("", braid, g + 1))
        .expect("probe slot")
        .is_some()
    {
        g += 1;
    }
    g
}

/// The gated S3 smoke: no credentials means no lane, and the skip is
/// loud rather than a silent green.
#[test]
fn s3_smoke_gated_skips_loudly_without_credentials() {
    let required = [
        "BUMBLEDB_S3_SMOKE_BUCKET",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
    ];
    let missing: Vec<&str> = required
        .iter()
        .filter(|key| std::env::var(key).is_err())
        .copied()
        .collect();
    assert!(
        !missing.is_empty(),
        "S3 credentials are present but this crate carries no S3Store yet; \
         the smoke lane cannot run — wire it here the day the store lands"
    );
    eprintln!(
        "SKIPPED f11 S3 smoke: credential-gated lane not run (missing {missing:?}); \
         the FsStore pins are the recorded floor"
    );
}

/// Per-braid commit latency floor on `FsStore`: one writer, sequential
/// single-insert commits, one series per braid shape — statement-free
/// (note), key + containment (recipe), keyed parent (venue).
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_per_braid_commit_latency_floor() {
    let root = temp_dir("f11_latency");
    let writer = open_writer(root.clone(), &root.join("w"), 11);

    for i in 0..8 {
        let _ = timed_note(&writer, 90_000 + i);
    }
    let mut note_samples: Vec<Duration> = (0..128).map(|i| timed_note(&writer, i)).collect();
    report(
        "commit latency floor, note braid (statement-free)",
        &mut note_samples,
    );

    let mut recipe_samples: Vec<Duration> = (0..128)
        .map(|i| {
            let start = Instant::now();
            let outcome = writer
                .commit(|batch| {
                    batch.insert(RECIPE, [recipe_row(i, "pin")]);
                    Ok(())
                })
                .expect("recipe commit");
            let elapsed = start.elapsed();
            assert!(matches!(outcome, Commit::Accepted { .. }));
            elapsed
        })
        .collect();
    report(
        "commit latency floor, kitchen braid (key + containment)",
        &mut recipe_samples,
    );

    let mut venue_samples: Vec<Duration> = (0..128)
        .map(|i| {
            let start = Instant::now();
            let outcome = writer
                .commit(|batch| {
                    batch.insert(VENUE, [venue_row(i)]);
                    Ok(())
                })
                .expect("venue commit");
            let elapsed = start.elapsed();
            assert!(matches!(outcome, Commit::Accepted { .. }));
            elapsed
        })
        .collect();
    report(
        "commit latency floor, venue braid (keyed capacity parent)",
        &mut venue_samples,
    );
}

/// Loss costs: the no-loss baseline, a disjoint loss end to end (the
/// attribution line names whether the fast path or a re-judgment
/// carried it), a conflict loss re-judging to the serial rejection —
/// the discard + re-open shape that was also the old baseline's cost
/// for every loss — and the pure fast-path components (one
/// intersection + one slot PUT) measured alone.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_loss_costs_disjoint_vs_conflict_vs_discard() {
    // Baseline: no losses anywhere.
    let base_root = temp_dir("f11_loss_base");
    let base_writer = open_writer(base_root.clone(), &base_root.join("w"), 11);
    for i in 0..8 {
        let _ = timed_note(&base_writer, 90_000 + i);
    }
    let mut base_samples: Vec<Duration> = (0..32).map(|i| timed_note(&base_writer, i)).collect();
    report("loss cost baseline (no loss)", &mut base_samples);
    let base = median(&mut base_samples);

    // Disjoint loss: the racing store plants a chain-valid competitor
    // note on every armed slot attempt, so every measured commit pays
    // exactly one live disjoint loss before landing.
    let root = temp_dir("f11_loss_disjoint");
    let braid = note_braid(&codec());
    let (racing, handle) = RacingStore::new(root.clone(), "", braid, 0, Competitor::Notes);
    let writer = match Writer::open(racing, "", &root.join("w"), theory(), Options::new(12))
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    let _ = timed_note(&writer, 90_001);
    let before = writer.counters();
    let mut disjoint_samples: Vec<Duration> = (0..16)
        .map(|i| {
            force_checkpoint(&writer, 80_000 + i);
            handle.seed_from(root.clone());
            handle.arm(1);
            timed_note(&writer, 100 + i)
        })
        .collect();
    let after = writer.counters();
    assert!(after.disjoint_verdicts >= before.disjoint_verdicts + 16);
    assert!(after.republishes >= before.republishes + 16);
    let disjoint_re_judged = after.re_judgments - before.re_judgments;
    report("disjoint loss, end to end", &mut disjoint_samples);
    println!(
        "PIN f11 disjoint loss attribution: 16 losses, {disjoint_re_judged} re-judgments — \
         zero means the republish-without-re-judgment fast path carried them; nonzero means \
         the gate routed them through discard + re-judge",
    );
    let disjoint = median(&mut disjoint_samples);

    // Conflict loss: a planted winner takes the loser's slot with the
    // same recipe determinant; the loser re-judges to the serial FD
    // rejection.
    let croot = temp_dir("f11_loss_conflict");
    let cwriter = open_writer(croot.clone(), &croot.join("w"), 13);
    let _ = timed_note(&cwriter, 90_002);
    let mut log = TestLog::attach(croot.clone(), "");
    let kitchen = kitchen_braid(&codec());
    let mut conflict_samples: Vec<Duration> = (0..16)
        .map(|i| {
            force_checkpoint(&cwriter, 80_000 + i);
            log.publish(kitchen, &[insert(RECIPE, recipe_row(i, "winner"))], i + 1);
            let start = Instant::now();
            let outcome = cwriter
                .commit(|batch| {
                    batch.insert(RECIPE, [recipe_row(i, "loser")]);
                    Ok(())
                })
                .expect("conflict commit");
            let elapsed = start.elapsed();
            assert!(
                matches!(outcome, Commit::Rejected(_)),
                "the re-judgment produces the serial FD rejection"
            );
            elapsed
        })
        .collect();
    report(
        "conflict loss, end to end (discard + re-open + re-judge to rejection)",
        &mut conflict_samples,
    );
    let conflict = median(&mut conflict_samples);

    // Fast-path attribution: what the lifted gate would spend — one
    // pure intersection plus one slot PUT (the local apply already
    // happened before the loss was known).
    let fixture = codec();
    let loser_ops = vec![insert(NOTE, note_row(1, "loser"))];
    let winner_ops = vec![insert(NOTE, note_row(2, "winner"))];
    let loser_fp = footprint(fixture.vocabulary(), &loser_ops).expect("footprint");
    let start = Instant::now();
    for _ in 0..10_000 {
        let decision = intersect(
            fixture.vocabulary(),
            &loser_fp,
            &loser_ops,
            &winner_ops,
            &BTreeMap::new(),
        )
        .expect("intersect");
        assert_eq!(decision, LoserDecision::Disjoint);
    }
    let intersect_each = start.elapsed() / 10_000;

    let put_root = temp_dir("f11_loss_put");
    let put_store = FsStore::new(put_root);
    let bytes = log.encode(note_braid(&fixture), &loser_ops, 1).1;
    let mut put_samples: Vec<Duration> = (0..256)
        .map(|i| {
            let key = format!("bench/{i}");
            let start = Instant::now();
            put_store.put_create(&key, &bytes).expect("put");
            start.elapsed()
        })
        .collect();
    report(
        "slot PUT alone (FsStore put_create, fsynced)",
        &mut put_samples,
    );
    let put = median(&mut put_samples);

    println!(
        "PIN f11 loss cost summary: baseline={:.1}us; disjoint-loss={:.1}us (extra {:.1}us); \
         conflict-loss={:.1}us (extra {:.1}us — the discard + re-open + re-judge shape, \
         which is also what every disjoint loss paid under the old discard-everything \
         baseline); pure fast-path components: intersect={:.2}us + slot PUT={:.1}us = {:.1}us",
        micros(base),
        micros(disjoint),
        micros(disjoint.saturating_sub(base)),
        micros(conflict),
        micros(conflict.saturating_sub(base)),
        micros(intersect_each),
        micros(put),
        micros(intersect_each + put),
    );
}

/// Group-commit throughput as braids multiply: eight committers per
/// braid over one writer, one/two/three active braids; log-slot counts
/// give the realized batch packing.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_group_commit_throughput_by_braid_count() {
    const THREADS_PER_BRAID: usize = 8;
    const PER_THREAD: u64 = 64;
    let fixture = codec();
    let braids = [
        note_braid(&fixture),
        kitchen_braid(&fixture),
        venue_braid(&fixture),
    ];
    for braid_count in 1..=3usize {
        let root = temp_dir("f11_group");
        let writer = open_writer(root.clone(), &root.join("w"), 21);
        let barrier = Barrier::new(braid_count * THREADS_PER_BRAID + 1);
        let elapsed = std::thread::scope(|scope| {
            let writer = &writer;
            let barrier = &barrier;
            let mut handles = Vec::new();
            for b in 0..braid_count {
                for t in 0..THREADS_PER_BRAID {
                    handles.push(scope.spawn(move || {
                        barrier.wait();
                        for i in 0..PER_THREAD {
                            let id = ((b as u64) << 32) | ((t as u64) << 16) | i;
                            let outcome = match b {
                                0 => writer.commit(|batch| {
                                    batch.insert(NOTE, [note_row(id, "g")]);
                                    Ok(())
                                }),
                                1 => writer.commit(|batch| {
                                    batch.insert(RECIPE, [recipe_row(id, "g")]);
                                    Ok(())
                                }),
                                _ => writer.commit(|batch| {
                                    batch.insert(VENUE, [venue_row(id)]);
                                    Ok(())
                                }),
                            }
                            .expect("group commit");
                            assert!(matches!(outcome, Commit::Accepted { .. }));
                        }
                    }));
                }
            }
            barrier.wait();
            let start = Instant::now();
            for handle in handles {
                handle.join().expect("join committer");
            }
            start.elapsed()
        });
        let total = (braid_count * THREADS_PER_BRAID) as u64 * PER_THREAD;
        let store = FsStore::new(root);
        let slots: u64 = braids[..braid_count]
            .iter()
            .map(|braid| braid_tip(&store, *braid))
            .sum();
        println!(
            "PIN f11 group commit, one writer: braids={braid_count} committers={} \
             commits={total} elapsed={:.2}s throughput={:.0}/s slots={slots} \
             avg_batch={:.1} commits/slot",
            braid_count * THREADS_PER_BRAID,
            elapsed.as_secs_f64(),
            total as f64 / elapsed.as_secs_f64(),
            total as f64 / slots as f64,
        );
    }

    // Braids multiply throughput across writers: one writer per braid,
    // each with its own committers, over one shared prefix — the drains
    // never race because the braids never share a slot.
    for braid_count in 1..=3usize {
        let root = temp_dir("f11_group_fleet");
        let writers: Vec<FsWriter> = (0..braid_count)
            .map(|b| open_writer(root.clone(), &root.join(format!("w{b}")), 70 + b as u64))
            .collect();
        let barrier = Barrier::new(braid_count * THREADS_PER_BRAID + 1);
        let elapsed = std::thread::scope(|scope| {
            let barrier = &barrier;
            let mut handles = Vec::new();
            for (b, writer) in writers.iter().enumerate() {
                for t in 0..THREADS_PER_BRAID {
                    handles.push(scope.spawn(move || {
                        barrier.wait();
                        for i in 0..PER_THREAD {
                            let id = ((b as u64) << 32) | ((t as u64) << 16) | i;
                            let outcome = match b {
                                0 => writer.commit(|batch| {
                                    batch.insert(NOTE, [note_row(id, "g")]);
                                    Ok(())
                                }),
                                1 => writer.commit(|batch| {
                                    batch.insert(RECIPE, [recipe_row(id, "g")]);
                                    Ok(())
                                }),
                                _ => writer.commit(|batch| {
                                    batch.insert(VENUE, [venue_row(id)]);
                                    Ok(())
                                }),
                            }
                            .expect("fleet commit");
                            assert!(matches!(outcome, Commit::Accepted { .. }));
                        }
                    }));
                }
            }
            barrier.wait();
            let start = Instant::now();
            for handle in handles {
                handle.join().expect("join committer");
            }
            start.elapsed()
        });
        let total = (braid_count * THREADS_PER_BRAID) as u64 * PER_THREAD;
        let store = FsStore::new(root);
        let slots: u64 = braids[..braid_count]
            .iter()
            .map(|braid| braid_tip(&store, *braid))
            .sum();
        println!(
            "PIN f11 group commit, writer per braid: braids={braid_count} commits={total} \
             elapsed={:.2}s throughput={:.0}/s slots={slots} avg_batch={:.1} commits/slot",
            elapsed.as_secs_f64(),
            total as f64 / elapsed.as_secs_f64(),
            total as f64 / slots as f64,
        );
    }
}

/// Cold open against the log alone vs seeding from a checkpoint, with
/// the history spread across all three braids so replay walks them
/// round-robin; the checkpoint object's size rides the same line.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_cold_open_vs_checkpoint_size() {
    const PER_BRAID: u64 = 60;
    let root = temp_dir("f11_cold");
    let writer = open_writer(root.clone(), &root.join("w"), 31);
    for i in 0..PER_BRAID {
        let _ = timed_note(&writer, i);
        let outcome = writer
            .commit(|batch| {
                batch.insert(RECIPE, [recipe_row(i, "cold")]);
                Ok(())
            })
            .expect("recipe commit");
        assert!(matches!(outcome, Commit::Accepted { .. }));
        let outcome = writer
            .commit(|batch| {
                batch.insert(VENUE, [venue_row(i)]);
                Ok(())
            })
            .expect("venue commit");
        assert!(matches!(outcome, Commit::Accepted { .. }));
    }

    let start = Instant::now();
    let replayed = match Replica::open(
        FsStore::new(root.clone()),
        "",
        &root.join("r_cold"),
        theory(),
    )
    .expect("open replica")
    {
        Opened::Ready(replica) => replica,
        Opened::Refused(refusal) => panic!("cold open refused: {refusal:?}"),
    };
    let cold = start.elapsed();
    let cold_slots: u64 = replayed.vector().values().sum();
    assert_eq!(cold_slots, PER_BRAID * 3);

    force_checkpoint(&writer, 90_000);
    let store = FsStore::new(root.clone());
    let manifest = Manifest::parse(
        &store
            .get(&manifest_key(""))
            .expect("manifest get")
            .expect("manifest exists")
            .bytes,
    )
    .expect("manifest parses");
    let digest = manifest.checkpoint.expect("checkpoint published");
    let ckpt_bytes = store
        .get(&ckpt_mdb_key("", &digest))
        .expect("ckpt get")
        .expect("ckpt object exists")
        .bytes
        .len() as u64;

    let start = Instant::now();
    let seeded = match Replica::open(
        FsStore::new(root.clone()),
        "",
        &root.join("r_seed"),
        theory(),
    )
    .expect("open replica")
    {
        Opened::Ready(replica) => replica,
        Opened::Refused(refusal) => panic!("seeded open refused: {refusal:?}"),
    };
    let warm = start.elapsed();
    assert_eq!(seeded.provenance(), Provenance::Checkpoint);
    let seeded_slots: u64 = seeded.vector().values().sum();

    println!(
        "PIN f11 cold open: full-replay open of {cold_slots} slots across 3 braids = {:.1}ms \
         ({:.2}ms/slot); checkpoint-seeded open at {seeded_slots} slots = {:.1}ms; \
         checkpoint object = {ckpt_bytes} bytes",
        cold.as_secs_f64() * 1e3,
        cold.as_secs_f64() * 1e3 / cold_slots as f64,
        warm.as_secs_f64() * 1e3,
    );
}

/// Idle probe cost: a replica at the tip pays one 404 GET per braid per
/// pass, plus the conditional manifest poll on every heartbeat pass.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_idle_probe_cost_per_pass() {
    let root = temp_dir("f11_idle");
    let writer = open_writer(root.clone(), &root.join("w"), 41);
    for i in 0..4 {
        let _ = timed_note(&writer, i);
    }
    let mut replica = match Replica::open(FsStore::new(root.clone()), "", &root.join("r"), theory())
        .expect("open replica")
    {
        Opened::Ready(replica) => replica,
        Opened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };

    let mut idle: Vec<Duration> = Vec::new();
    let mut heartbeat: Vec<Duration> = Vec::new();
    for pass in 1..=64u64 {
        let start = Instant::now();
        let refreshed = replica.refresh().expect("refresh");
        let elapsed = start.elapsed();
        assert!(matches!(refreshed, Refreshed::Vector(_)));
        if pass.is_multiple_of(16) {
            heartbeat.push(elapsed);
        } else {
            idle.push(elapsed);
        }
    }
    report("idle probe pass (3 braid 404 probes)", &mut idle);
    report(
        "idle probe pass with heartbeat (adds the conditional manifest poll)",
        &mut heartbeat,
    );
}

fn sum_counters(tallies: &[Tally]) -> Counters {
    let mut total = Counters {
        re_judgments: 0,
        republishes: 0,
        subsumptions: 0,
        disjoint_verdicts: 0,
    };
    for tally in tallies {
        total.re_judgments += tally.counters.re_judgments;
        total.republishes += tally.counters.republishes;
        total.subsumptions += tally.counters.subsumptions;
        total.disjoint_verdicts += tally.counters.disjoint_verdicts;
    }
    total
}

struct Tally {
    accepted: u64,
    rejected: u64,
    contended: u64,
    counters: Counters,
}

/// The contention curve: four writers over one `FsStore` prefix insert
/// recipes whose determinant is drawn Zipfian from 4096 keys, skew 0
/// to 0.999. Throughput, the re-judge rate, and the verdict mix ride
/// one line per skew; rejections at high skew are the serial FD
/// verdicts the algebra exists to produce.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_contention_curve_zipfian_hot_key() {
    const WRITERS: usize = 4;
    const PER_WRITER: u64 = 30;
    const KEYS: usize = 4096;
    for theta in [0.0, 0.5, 0.9, 0.99, 0.999] {
        let root = temp_dir("f11_curve");
        let writers: Vec<FsWriter> = (0..WRITERS)
            .map(|w| {
                let writer = open_writer(root.clone(), &root.join(format!("w{w}")), 100 + w as u64);
                writer.set_checkpoint_cadence(32, u64::MAX);
                writer
            })
            .collect();
        let zipf = Zipf::new(KEYS, theta);
        let barrier = Barrier::new(WRITERS + 1);
        let (elapsed, tallies) = std::thread::scope(|scope| {
            let zipf = &zipf;
            let barrier = &barrier;
            let handles: Vec<_> = writers
                .into_iter()
                .enumerate()
                .map(|(w, writer)| {
                    scope.spawn(move || {
                        barrier.wait();
                        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(w as u64 + 1));
                        let mut tally = Tally {
                            accepted: 0,
                            rejected: 0,
                            contended: 0,
                            counters: writer.counters(),
                        };
                        for i in 0..PER_WRITER {
                            let key = zipf.sample(&mut rng) + 1;
                            let title = format!("w{w}-{i}");
                            match writer.commit(|batch| {
                                batch.insert(RECIPE, [recipe_row(key, &title)]);
                                Ok(())
                            }) {
                                Ok(Commit::Accepted { .. }) => tally.accepted += 1,
                                Ok(Commit::Rejected(_)) => tally.rejected += 1,
                                Err(Error::Contention { .. }) => tally.contended += 1,
                                Err(error) => panic!("commit failed: {error}"),
                            }
                        }
                        writer.quiesce();
                        tally.counters = writer.counters();
                        tally
                    })
                })
                .collect();
            barrier.wait();
            let start = Instant::now();
            let tallies: Vec<Tally> = handles
                .into_iter()
                .map(|handle| handle.join().expect("join writer"))
                .collect();
            (start.elapsed(), tallies)
        });
        let totals = sum_counters(&tallies);
        let accepted: u64 = tallies.iter().map(|t| t.accepted).sum();
        let rejected: u64 = tallies.iter().map(|t| t.rejected).sum();
        let contended: u64 = tallies.iter().map(|t| t.contended).sum();
        let commits = accepted + rejected + contended;
        println!(
            "PIN f11 contention curve: theta={theta} writers={WRITERS} keys={KEYS} \
             commits={commits} elapsed={:.2}s decided/s={:.0} accepted={accepted} \
             rejected={rejected} contended={contended} re_judge_rate={:.3} \
             re_judgments={} republishes={} subsumed={} disjoint={}",
            elapsed.as_secs_f64(),
            commits as f64 / elapsed.as_secs_f64(),
            totals.re_judgments as f64 / commits as f64,
            totals.re_judgments,
            totals.republishes,
            totals.subsumptions,
            totals.disjoint_verdicts,
        );
    }
}

/// The conflict ratio on a deterministic alternating two-writer
/// workload: identical rows race into subsumption, distinct notes into
/// disjoint losses (republished through the gated re-judgment), hot
/// recipe determinants into serial rejections.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_conflict_ratio_mixed_workload() {
    const ROUNDS: u64 = 24;
    let root = temp_dir("f11_ratio");
    let a = open_writer(root.clone(), &root.join("wa"), 51);
    let b = open_writer(root.clone(), &root.join("wb"), 52);
    let mut rejected = 0u64;
    let mut accepted = 0u64;
    let mut commit = |writer: &FsWriter, relation, row: Box<[Value]>| {
        let outcome = writer
            .commit(|batch| {
                batch.insert(relation, [row.clone()]);
                Ok(())
            })
            .expect("ratio commit");
        match outcome {
            Commit::Accepted { .. } => accepted += 1,
            Commit::Rejected(_) => rejected += 1,
        }
    };
    for i in 0..ROUNDS {
        commit(&a, NOTE, note_row(i, "dup"));
        commit(&b, NOTE, note_row(i, "dup"));
        commit(&a, NOTE, note_row(10_000 + i, "solo-a"));
        commit(&b, NOTE, note_row(20_000 + i, "solo-b"));
        commit(&a, RECIPE, recipe_row(i, "first"));
        commit(&b, RECIPE, recipe_row(i, "second"));
    }
    let counters = sum_counters(&[
        Tally {
            accepted: 0,
            rejected: 0,
            contended: 0,
            counters: a.counters(),
        },
        Tally {
            accepted: 0,
            rejected: 0,
            contended: 0,
            counters: b.counters(),
        },
    ]);
    let commits = accepted + rejected;
    assert_eq!(commits, ROUNDS * 6);
    assert!(counters.subsumptions > 0, "the dup lane subsumed");
    assert!(
        counters.disjoint_verdicts > 0,
        "the solo lane lost disjoint"
    );
    assert!(rejected > 0, "the hot lane rejected serially");
    println!(
        "PIN f11 conflict ratio: commits={commits} accepted={accepted} rejected={rejected} \
         subsumed={} ({:.3}) republished={} ({:.3}) re_judged={} ({:.3}) rejected_share={:.3}",
        counters.subsumptions,
        counters.subsumptions as f64 / commits as f64,
        counters.republishes,
        counters.republishes as f64 / commits as f64,
        counters.re_judgments,
        counters.re_judgments as f64 / commits as f64,
        rejected as f64 / commits as f64,
    );
}

/// The crossover behind the sixteen-loss bound: measure a resident
/// writer's single-braid group-commit throughput and the cost of one
/// live disjoint loss, then record the loss rate at which loss
/// resolution alone consumes the writer — and what a full run to the
/// bound stalls for.
#[test]
#[ignore = "measurement harness: run with --ignored --nocapture to record the pins"]
fn pin_crossover_point_for_the_loss_bound() {
    // Component 1: resident group-commit throughput on one braid.
    const THREADS: usize = 8;
    const PER_THREAD: u64 = 48;
    let groot = temp_dir("f11_cross_group");
    let gwriter = open_writer(groot.clone(), &groot.join("w"), 61);
    let barrier = Barrier::new(THREADS + 1);
    let elapsed = std::thread::scope(|scope| {
        let writer = &gwriter;
        let barrier = &barrier;
        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                scope.spawn(move || {
                    barrier.wait();
                    for i in 0..PER_THREAD {
                        let _ = timed_note(writer, ((t as u64) << 16) | i);
                    }
                })
            })
            .collect();
        barrier.wait();
        let start = Instant::now();
        for handle in handles {
            handle.join().expect("join committer");
        }
        start.elapsed()
    });
    let throughput = (THREADS as u64 * PER_THREAD) as f64 / elapsed.as_secs_f64();

    // Component 2: the live-loss cost on the same braid shape.
    let root = temp_dir("f11_cross_loss");
    let braid = note_braid(&codec());
    let (racing, handle) = RacingStore::new(root.clone(), "", braid, 0, Competitor::Notes);
    let writer = match Writer::open(racing, "", &root.join("w"), theory(), Options::new(62))
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    let mut base_samples: Vec<Duration> = (0..12).map(|i| timed_note(&writer, i)).collect();
    let base = median(&mut base_samples);
    let mut loss_samples: Vec<Duration> = (0..12)
        .map(|i| {
            force_checkpoint(&writer, 80_000 + i);
            handle.seed_from(root.clone());
            handle.arm(1);
            timed_note(&writer, 100 + i)
        })
        .collect();
    let loss = median(&mut loss_samples);
    let loss_cost = loss.saturating_sub(base);
    let crossover = 1.0 / loss_cost.as_secs_f64();
    println!(
        "PIN f11 crossover: group-commit capacity {throughput:.0} commits/s on one braid; \
         one live disjoint loss costs {:.1}ms over the {:.1}ms no-loss commit; a live-loss \
         rate above {crossover:.0}/s spends the writer entirely on loss resolution \
         (loss-rate x cost >= 1); a full run to the 16-loss bound stalls {:.0}ms before \
         Err::Contention — the recorded basis for the bound and for resident mode on hot \
         braids",
        loss_cost.as_secs_f64() * 1e3,
        base.as_secs_f64() * 1e3,
        16.0 * loss_cost.as_secs_f64() * 1e3,
    );
}
