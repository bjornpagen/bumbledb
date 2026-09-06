use std::path::Path;

use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::harness::{Protocol, Stats};
use crate::report::GhzReport;
use crate::{clockproxy, poststate};

use super::{LawFamily, LawSizes, families, ids, lanes, load, render, schema};

#[derive(Debug, Clone, PartialEq)]
pub struct LawRow {
    pub family: &'static str,
    pub lane: &'static str,
    pub about: &'static str,
    pub ours: Stats,
    pub theirs: Stats,

    pub ratio_p50: f64,

    pub work: u64,
    pub ghz: GhzReport,
}

/// # Errors
pub fn run(
    dir: &Path,
    seed: u64,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    run_with(dir, seed, LawSizes::of(Scale::S), samples, only)
}

/// The full lawful run: returns `(markdown, json)` only after every lane's
/// post-state comparison passes. `samples` overrides measured samples, not
/// warmups. `only` selects registry names; unknown names refuse before loading.
/// # Errors
/// The device-honesty refusal (the timed lawful lanes are fsync-bound); an
/// unknown `--only` name; loader, runner, and post-state failures, stringified
/// with the lane named.
pub fn run_with(
    dir: &Path,
    seed: u64,
    sizes: LawSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    // Every lane is durable; refuse RAM-backed targets before creating anything.
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

/// The registered protocol with the sample override applied (warmups stay
/// registered — the override trims measurement, never readiness).
fn proto_of(family: &LawFamily, samples: Option<u32>) -> Protocol {
    Protocol {
        warmups: family.protocol.warmups,
        samples: samples.unwrap_or(family.protocol.samples),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn ratio(ours: u64, theirs: u64) -> f64 {
    ours as f64 / theirs.max(1) as f64
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per registered family: the registry IS the run order"
)]
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

    // Saturate the rejection target on both engines before any timing.
    if selected("law_reject_window") {
        lanes::fill_window_target_engine(&db, sizes, &mut ours_cursor)?;
        lanes::fill_window_target_sqlite(&conn, sizes, &mut theirs_cursor)?;
    }

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
        let (ours, theirs, stamp) = match family.name {
            "law_commit_attempt" => {
                let timed = attempt_stream;
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::commit_attempt_engine(&db, proto, timed, &mut ours_cursor)?,
                        lanes::commit_attempt_sqlite(&conn, proto, timed, &mut theirs_cursor)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            "law_commit_cluster" => {
                let timed = cluster_stream;
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::commit_cluster_engine(&db, proto, timed, &mut ours_cursor)?,
                        lanes::commit_cluster_sqlite(&conn, proto, timed, &mut theirs_cursor)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            "law_reject_key" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::reject_key_engine(&db, proto)?,
                        lanes::reject_key_sqlite(&conn, proto)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            "law_reject_containment" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::reject_containment_engine(&db, proto, sizes)?,
                        lanes::reject_containment_sqlite(&conn, proto, sizes)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            "law_reject_window" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::reject_window_engine(&db, proto)?,
                        lanes::reject_window_sqlite(&conn, proto)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            "law_reject_scope" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::reject_scope_engine(&db, proto)?,
                        lanes::reject_scope_sqlite(&conn, proto)?,
                    ))
                })?;
                (ours, theirs, stamp)
            }
            other => return Err(format!("unregistered lawful family: {other}")),
        };
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
            ghz: stamp.into(),
        });
    }

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
