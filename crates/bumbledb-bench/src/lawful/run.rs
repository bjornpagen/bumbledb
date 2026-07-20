//! The lawful orchestration: device honesty first, then per durability
//! lane — load the twin pair, run the untimed window setup (only when
//! `law_reject_window` is selected), time the selected families in
//! registry order (one clock-proxy stamp bracketing each family's
//! engine+`SQLite` pair), then the post-state fold over ALL FIVE
//! ordinary relations (rejections must have committed nothing; legal
//! lanes must have landed identically on both twins).
//!
//! **No oracle gate runs here: this world has no queries** — law 3 of
//! the standing run binds queries, and the lawful world declares none.
//! The write families' oracle is the post-state fold
//! ([`crate::poststate`]) plus LAW-1's naive verdict parity (the
//! differential test over the full law roster, verdicts and citations
//! compared whole).
//!
//! REPORT-class, never gated: the artifacts land and nothing else — no
//! budget gate ever reads a lawful row (the standing report-class law).

use std::path::Path;

use crate::clockproxy;
use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::harness::{Protocol, Stats};
use crate::poststate;
use crate::report::GhzReport;

use super::{LawFamily, LawSizes, families, ids, lanes, load, render, schema};

/// One lawful report row — the crud row's fields, defined locally (the
/// worlds stay independent; nothing here imports from `crud`).
#[derive(Debug, Clone, PartialEq)]
pub struct LawRow {
    pub family: &'static str,
    pub lane: &'static str,
    pub about: &'static str,
    pub ours: Stats,
    pub theirs: Stats,
    /// ours p50 / theirs p50 (below 1.0 means we are faster) — the
    /// read-family ratio convention.
    pub ratio_p50: f64,
    /// The measured samples' summed work (identical on both engines by
    /// the shared-stream representation, asserted per family).
    pub work: u64,
    pub ghz: GhzReport,
}

/// The timed entry point at the standing bench scale: delegates to
/// [`run_with`] under `LawSizes::of(Scale::S)`.
///
/// # Errors
///
/// Everything [`run_with`] refuses, verbatim.
pub fn run(
    dir: &Path,
    seed: u64,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    run_with(dir, seed, LawSizes::of(Scale::S), samples, only)
}

/// The full lawful run: returns `(markdown, json)` — the two artifacts
/// as strings ([`render`]), produced only after every lane's post-state
/// fold passed. `samples` overrides each registered protocol's sample
/// count (warmups stay registered); `only` selects families by
/// registry name (unknown names are refused before anything loads).
///
/// # Errors
///
/// The device-honesty refusal (the timed lawful lanes are fsync-bound);
/// an unknown `--only` name; loader, runner, and post-state failures,
/// stringified with the lane named.
pub fn run_with(
    dir: &Path,
    seed: u64,
    sizes: LawSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    // Device honesty FIRST, before creating anything: every legal
    // sample is one durable commit, so a RAM-backed volume would report
    // a number physics never signed.
    crate::devhonesty::assert_disk_backed(dir, "the timed lawful lanes")
        .map_err(|refusal| refusal.to_string())?;
    if let Some(names) = only {
        for name in names {
            if !families().iter().any(|family| family.name == name.as_str()) {
                return Err(format!(
                    "unknown lawful family: {name} (the registry names: {})",
                    families()
                        .iter()
                        .map(|family| family.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
    let selected =
        move |name: &str| only.is_none_or(|names| names.iter().any(|n| n.as_str() == name));
    let mut rows = Vec::new();
    for lane in duralane::ALL {
        rows.extend(run_lane(lane, dir, seed, sizes, samples, &selected)?);
    }
    Ok((render::markdown(seed, &rows), render::json(seed, &rows)))
}

/// The registered protocol with the sample override applied (warmups
/// stay registered — the override trims measurement, never readiness).
fn proto_of(family: &LawFamily, samples: Option<u32>) -> Protocol {
    Protocol {
        warmups: family.protocol.warmups,
        samples: samples.unwrap_or(family.protocol.samples),
    }
}

/// The driver's stamp→report conversion, local twin (every lane module
/// keeps its own private copy).
fn ghz_report(stamp: clockproxy::GhzStamp) -> GhzReport {
    GhzReport {
        pre: stamp.pre,
        post: stamp.post,
        retried: stamp.retried,
        contaminated: stamp.contaminated(),
    }
}

/// The read-family ratio convention: ours p50 over theirs p50.
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn ratio(ours: u64, theirs: u64) -> f64 {
    ours as f64 / theirs.max(1) as f64
}

/// One durability lane, whole: load, window setup, the selected
/// families in registry order, the post-state fold.
fn run_lane(
    lane: DurabilityLane,
    dir: &Path,
    seed: u64,
    sizes: LawSizes,
    samples: Option<u32>,
    selected: &dyn Fn(&str) -> bool,
) -> Result<Vec<LawRow>, String> {
    eprintln!("bench: lawful {} — loading the twin pair", lane.label());
    let (db, conn) = load::load_stores(&dir.join(lane.label()), seed, sizes, lane)?;
    let mut ours_cursor = lanes::LawCursor::at_base(sizes);
    let mut theirs_cursor = lanes::LawCursor::at_base(sizes);
    // The untimed window setup — run ONLY when the window rejection is
    // selected (both engines, cursors advanced in lockstep, before any
    // timing starts).
    if selected("law_reject_window") {
        lanes::fill_window_target_engine(&db, sizes, &mut ours_cursor)?;
        lanes::fill_window_target_sqlite(&conn, sizes, &mut theirs_cursor)?;
    }
    // ONE legal op stream covering both legal families, sliced in
    // registry order — the per-task n counters continue across the
    // slice boundary, so no (task, n) key is ever minted twice.
    let count_for = |name: &str| -> usize {
        families()
            .iter()
            .find(|family| family.name == name)
            .filter(|_| selected(name))
            .map_or(0, |family| {
                let proto = proto_of(family, samples);
                usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small")
            })
    };
    let n_attempt = count_for("law_commit_attempt");
    let n_cluster = count_for("law_commit_cluster");
    let stream = lanes::attempt_ops(sizes, n_attempt + n_cluster);
    let (attempt_stream, cluster_stream) = stream.split_at(n_attempt);

    let mut rows = Vec::new();
    for family in families() {
        if !selected(family.name) {
            continue;
        }
        let proto = proto_of(family, samples);
        eprintln!("bench: lawful {} — {}", lane.label(), family.name);
        let ((ours, theirs), stamp) = clockproxy::stamped(|| {
            Ok(match family.name {
                "law_commit_attempt" => (
                    lanes::commit_attempt_engine(&db, proto, attempt_stream, &mut ours_cursor)?,
                    lanes::commit_attempt_sqlite(&conn, proto, attempt_stream, &mut theirs_cursor)?,
                ),
                "law_commit_cluster" => (
                    lanes::commit_cluster_engine(&db, proto, cluster_stream, &mut ours_cursor)?,
                    lanes::commit_cluster_sqlite(&conn, proto, cluster_stream, &mut theirs_cursor)?,
                ),
                "law_reject_key" => (
                    lanes::reject_key_engine(&db, proto)?,
                    lanes::reject_key_sqlite(&conn, proto)?,
                ),
                "law_reject_containment" => (
                    lanes::reject_containment_engine(&db, proto, sizes)?,
                    lanes::reject_containment_sqlite(&conn, proto, sizes)?,
                ),
                "law_reject_window" => (
                    lanes::reject_window_engine(&db, proto)?,
                    lanes::reject_window_sqlite(&conn, proto)?,
                ),
                "law_reject_scope" => (
                    lanes::reject_scope_engine(&db, proto)?,
                    lanes::reject_scope_sqlite(&conn, proto)?,
                ),
                other => return Err(format!("unregistered lawful family: {other}")),
            })
        })?;
        if ours.work != theirs.work {
            return Err(format!(
                "{}: the twins' work diverges — engine {}, sqlite {}",
                family.name, ours.work, theirs.work
            ));
        }
        rows.push(LawRow {
            family: family.name,
            lane: lane.label(),
            about: family.about,
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50: ratio(ours.stats.p50, theirs.stats.p50),
            work: ours.work,
            ghz: ghz_report(stamp),
        });
    }

    // The post-state fold over ALL FIVE ordinary relations: rejections
    // must have committed nothing; legal lanes must have landed
    // identically — the run's oracle (this world has no queries).
    for rel in [
        ids::TASK,
        ids::ATTEMPT,
        ids::VERDICT,
        ids::STEER,
        ids::STEER_SCOPE,
    ] {
        let relation = schema().relation(rel);
        let ours = poststate::engine_rows(&db, rel)?;
        let theirs = poststate::sqlite_rows(&conn, relation)?;
        poststate::assert_identical("lawful", relation.name(), ours, theirs)
            .map_err(|e| format!("{} lane: {e}", lane.label()))?;
    }
    drop((db, conn));
    Ok(rows)
}
