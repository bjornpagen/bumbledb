//! The churn report artifact — the degradation curve as data. A churn
//! run is a TIME SERIES (cycle → sample per lane), a new shape rather
//! than a retrofit of [`crate::report::RunReport`]'s scalar-median rows;
//! the JSON key names are NORMATIVE and frozen behind `churn_schema: 1`
//! (the viz planner charts this schema). `Kind::Report`-class by the
//! charter — nothing here gates, and nothing here times.
//!
//! Engine-specific counters are the sum type [`Counters`]: a lane
//! carrying the wrong engine's counters is unrepresentable in memory.
//! The JSON face renders the sum as four documented nullable fields
//! (`generation`, `id_high_water`, `freelist_count`, `page_count`) with
//! the OTHER engine's pair as `null`, so the viz layer needs no
//! tagged-union parsing. The writer is hand-rolled through
//! [`crate::json`] (the dependency quarantine) and pinned by a parse
//! round-trip against the crate's own [`crate::json::parse`].

use std::fmt::Write as _;
use std::path::Path;

use crate::churn::ops::Mix;
use crate::churn::probes::ProbeSample;
use crate::json;
use crate::report::Provenance;

/// The whole churn run, plain data — everything the renderers print.
#[derive(Debug, Clone, PartialEq)]
pub struct ChurnReport {
    pub provenance: Provenance,
    pub config: ConfigReport,
    pub runs: Vec<RunSeries>,
}

/// The run's schedule, as printed: the corpus identity plus the cycle,
/// sampling, and `SQLite` maintenance strides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigReport {
    pub scale: &'static str,
    pub seed: u64,
    pub cycles: u64,
    pub sample_every: u64,
    pub vacuum_every: u64,
    pub analyze_every: u64,
}

/// One mix's series: the named run, its mix, the working-set size, and
/// the per-lane time series.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSeries {
    pub name: String,
    pub mix: Mix,
    pub working_set: u64,
    pub lanes: Vec<LaneSeries>,
}

/// One lane's time series — the degradation curve for one engine
/// configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneSeries {
    pub lane: String,
    pub engine: Engine,
    pub samples: Vec<SamplePoint>,
}

/// Which engine a lane drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Bumbledb,
    Sqlite,
}

impl Engine {
    /// The JSON/markdown spelling.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bumbledb => "bumbledb",
            Self::Sqlite => "sqlite",
        }
    }
}

/// One point on the curve: the probe readings at this cycle plus the
/// write-side and store-size observables.
#[derive(Debug, Clone, PartialEq)]
pub struct SamplePoint {
    pub cycle: u64,
    pub probes: Vec<ProbeSample>,
    pub commits_per_sec: f64,
    pub maintenance_ns: u64,
    pub disk_bytes: u64,
    pub counters: Counters,
}

/// The engine-specific counters, a sum — a lane carrying the wrong
/// engine's counters is unrepresentable. `generation` and
/// `id_high_water` are the never-reissue law's observables (both burn
/// monotonically under churn; the series shows whether anything
/// degrades with the burn). `freelist_count`/`page_count` are `SQLite`'s
/// own PRAGMA counters (freelist growth is the delete-heavy lane's
/// story). The JSON face renders the sum as four flat nullable fields,
/// the other engine's pair `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counters {
    Ours {
        generation: u64,
        id_high_water: u64,
    },
    Sqlite {
        freelist_count: u64,
        page_count: u64,
    },
}

fn push_sample(out: &mut String, sample: &SamplePoint) {
    let _ = write!(out, "{{\"cycle\":{},\"probes\":[", sample.cycle);
    for (index, probe) in sample.probes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        json::push_str_lit(out, &probe.name);
        let _ = write!(
            out,
            ",\"p50_ns\":{},\"answers\":{}}}",
            probe.p50_ns, probe.answers
        );
    }
    let _ = write!(
        out,
        "],\"commits_per_sec\":{:.2},\"maintenance_ns\":{},\"disk_bytes\":{}",
        sample.commits_per_sec, sample.maintenance_ns, sample.disk_bytes
    );
    match sample.counters {
        Counters::Ours {
            generation,
            id_high_water,
        } => {
            let _ = write!(
                out,
                ",\"generation\":{generation},\"id_high_water\":{id_high_water},\
                 \"freelist_count\":null,\"page_count\":null"
            );
        }
        Counters::Sqlite {
            freelist_count,
            page_count,
        } => {
            let _ = write!(
                out,
                ",\"generation\":null,\"id_high_water\":null,\
                 \"freelist_count\":{freelist_count},\"page_count\":{page_count}"
            );
        }
    }
    out.push('}');
}

fn push_lane(out: &mut String, lane: &LaneSeries) {
    out.push_str("{\"lane\":");
    json::push_str_lit(out, &lane.lane);
    let _ = write!(out, ",\"engine\":\"{}\",\"samples\":[", lane.engine.label());
    for (index, sample) in lane.samples.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_sample(out, sample);
    }
    out.push_str("]}");
}

fn push_run(out: &mut String, run: &RunSeries) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &run.name);
    let _ = write!(
        out,
        ",\"mix\":{{\"churn\":{},\"updates\":{},\"growth\":{}}},\"working_set\":{},\"lanes\":[",
        run.mix.churn, run.mix.updates, run.mix.growth, run.working_set
    );
    for (index, lane) in run.lanes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_lane(out, lane);
    }
    out.push_str("]}");
}

/// The machine-consumable artifact — every field, hand-rolled, frozen
/// behind `"churn_schema":1`. The [`Counters`] sum renders as four flat
/// nullable fields (the other engine's pair `null`) — the documented
/// JSON face of the sum type.
#[must_use]
pub fn to_json(report: &ChurnReport) -> String {
    let mut out = String::new();
    out.push_str("{\"churn_schema\":1,\"provenance\":");
    crate::report::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"config\":{{\"scale\":\"{}\",\"seed\":{},\"cycles\":{},\"sample_every\":{},\
         \"vacuum_every\":{},\"analyze_every\":{}}}",
        report.config.scale,
        report.config.seed,
        report.config.cycles,
        report.config.sample_every,
        report.config.vacuum_every,
        report.config.analyze_every
    );
    out.push_str(",\"runs\":[");
    for (index, run) in report.runs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_run(&mut out, run);
    }
    out.push_str("]}");
    out
}

/// One markdown table row: probes space-joined as `name=p50`, the
/// counters sum printing its own pair and `-` for the other engine's
/// columns.
fn markdown_row(out: &mut String, sample: &SamplePoint) {
    let mut probes = String::new();
    for (index, probe) in sample.probes.iter().enumerate() {
        if index > 0 {
            probes.push(' ');
        }
        let _ = write!(probes, "{}={}", probe.name, probe.p50_ns);
    }
    let (generation, id_high_water, freelist, pages) = match sample.counters {
        Counters::Ours {
            generation,
            id_high_water,
        } => (
            generation.to_string(),
            id_high_water.to_string(),
            "-".to_owned(),
            "-".to_owned(),
        ),
        Counters::Sqlite {
            freelist_count,
            page_count,
        } => (
            "-".to_owned(),
            "-".to_owned(),
            freelist_count.to_string(),
            page_count.to_string(),
        ),
    };
    let _ = writeln!(
        out,
        "| {} | {probes} | {:.2} | {} | {} | {generation} | {id_high_water} | {freelist} | {pages} |",
        sample.cycle, sample.commits_per_sec, sample.maintenance_ns, sample.disk_bytes
    );
}

/// The human artifact — one table per lane, one row per sample point.
#[must_use]
pub fn to_markdown(report: &ChurnReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# churn — degradation over cycles\n");
    let p = &report.provenance;
    let _ = writeln!(out, "- crate version: {}", p.crate_version);
    let _ = writeln!(out, "- engine rev: {}", p.git_rev);
    let _ = writeln!(out, "- timestamp: {}", p.timestamp);
    let _ = writeln!(out, "- host: {}", p.host);
    if let Some(shared) = &p.shared {
        let _ = writeln!(out, "- shared machine: {}", shared.describe());
    }
    let c = &report.config;
    let _ = writeln!(
        out,
        "- config: scale {}, seed {}, {} cycles, sample every {}, vacuum every {}, \
         analyze every {}",
        c.scale, c.seed, c.cycles, c.sample_every, c.vacuum_every, c.analyze_every
    );
    for run in &report.runs {
        let _ = writeln!(
            out,
            "\n## run {} (churn={} updates={} growth={}, working set {})",
            run.name, run.mix.churn, run.mix.updates, run.mix.growth, run.working_set
        );
        for lane in &run.lanes {
            let _ = writeln!(out, "\n### {} ({})\n", lane.lane, lane.engine.label());
            let _ = writeln!(
                out,
                "| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | \
                 freelist | pages |"
            );
            let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
            for sample in &lane.samples {
                markdown_row(&mut out, sample);
            }
        }
    }
    out
}

/// Writes exactly two artifacts into `out_dir`: `churn-report` with
/// the `json` extension ([`to_json`]) and with the `md` extension
/// ([`to_markdown`]). Renders never write outside `out_dir`;
/// publishing a run is a human copy (the `report.rs` law).
///
/// # Errors
///
/// I/O errors verbatim.
pub fn write_artifacts(report: &ChurnReport, out_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("churn-report.json"), to_json(report))?;
    std::fs::write(out_dir.join("churn-report.md"), to_markdown(report))
}

#[cfg(test)]
mod tests {
    use crate::churn::ops::STEADY;
    use crate::json::Value;

    use super::*;

    /// The three registry probes at one sample point (the names are the
    /// probe registry's — `crate::churn::probes::all`).
    fn probe_samples(base: u64) -> Vec<ProbeSample> {
        vec![
            ProbeSample {
                name: "churn_point".to_owned(),
                p50_ns: base + 420,
                answers: 1,
            },
            ProbeSample {
                name: "churn_balance".to_owned(),
                p50_ns: base + 9_000,
                answers: 1,
            },
            ProbeSample {
                name: "churn_window".to_owned(),
                p50_ns: base + 15_000,
                answers: 20,
            },
        ]
    }

    fn sample(cycle: u64, counters: Counters) -> SamplePoint {
        SamplePoint {
            cycle,
            probes: probe_samples(cycle * 10),
            commits_per_sec: 195.2,
            maintenance_ns: cycle * 100,
            disk_bytes: 77_955_072 + cycle,
            counters,
        }
    }

    /// The in-memory synthetic fixture: provenance hand-built (never
    /// from git), one steady run, one lane per engine, two samples each.
    fn fixture() -> ChurnReport {
        ChurnReport {
            provenance: Provenance {
                crate_version: "0.0.0-fixture".to_owned(),
                git_rev: "deadbeef".to_owned(),
                timestamp: "2026-07-20T00:00:00Z".to_owned(),
                host: "fixture-host".to_owned(),
                shared: None,
            },
            config: ConfigReport {
                scale: "Tiny",
                seed: 1,
                cycles: 6,
                sample_every: 3,
                vacuum_every: 2,
                analyze_every: 3,
            },
            runs: vec![RunSeries {
                name: "steady".to_owned(),
                mix: STEADY,
                working_set: 1024,
                lanes: vec![
                    LaneSeries {
                        lane: "ours-durable".to_owned(),
                        engine: Engine::Bumbledb,
                        samples: vec![
                            sample(
                                3,
                                Counters::Ours {
                                    generation: 4,
                                    id_high_water: 1_312,
                                },
                            ),
                            sample(
                                6,
                                Counters::Ours {
                                    generation: 7,
                                    id_high_water: 1_600,
                                },
                            ),
                        ],
                    },
                    LaneSeries {
                        lane: "sqlite-bare".to_owned(),
                        engine: Engine::Sqlite,
                        samples: vec![
                            sample(
                                3,
                                Counters::Sqlite {
                                    freelist_count: 2,
                                    page_count: 40,
                                },
                            ),
                            sample(
                                6,
                                Counters::Sqlite {
                                    freelist_count: 5,
                                    page_count: 40,
                                },
                            ),
                        ],
                    },
                ],
            }],
        }
    }

    /// The schema pin: the hand-rolled emission parses back through the
    /// crate's own parser, with the counters sum rendered as the four
    /// flat fields — the other engine's pair PRESENT and `null`.
    #[test]
    fn churn_report_json_round_trips_through_the_parser() {
        let text = to_json(&fixture());
        let parsed = crate::json::parse(&text).expect("our own emission parses");
        assert_eq!(
            parsed.get("churn_schema").and_then(Value::as_f64),
            Some(1.0)
        );
        let runs = parsed
            .get("runs")
            .and_then(Value::as_arr)
            .expect("runs array");
        assert_eq!(runs.len(), 1);
        let lanes = runs[0]
            .get("lanes")
            .and_then(Value::as_arr)
            .expect("lanes array");
        assert_eq!(lanes.len(), 2);

        let ours = &lanes[0];
        assert_eq!(
            ours.get("lane").and_then(Value::as_str),
            Some("ours-durable")
        );
        assert_eq!(ours.get("engine").and_then(Value::as_str), Some("bumbledb"));
        let ours_sample = &ours
            .get("samples")
            .and_then(Value::as_arr)
            .expect("samples")[0];
        assert_eq!(ours_sample.get("cycle").and_then(Value::as_f64), Some(3.0));
        assert!(
            ours_sample
                .get("generation")
                .and_then(Value::as_f64)
                .is_some(),
            "an ours sample carries its generation as a number"
        );
        let freelist = ours_sample
            .get("freelist_count")
            .expect("the other engine's key EXISTS on the JSON face");
        assert!(
            freelist.as_f64().is_none(),
            "…and it is the null variant, not a number"
        );
        assert_eq!(freelist, &Value::Null);

        let sqlite = &lanes[1];
        assert_eq!(sqlite.get("engine").and_then(Value::as_str), Some("sqlite"));
        let sqlite_sample = &sqlite
            .get("samples")
            .and_then(Value::as_arr)
            .expect("samples")[0];
        assert!(
            sqlite_sample
                .get("freelist_count")
                .and_then(Value::as_f64)
                .is_some(),
            "a sqlite sample carries its freelist_count as a number"
        );
        let generation = sqlite_sample
            .get("generation")
            .expect("the other engine's key EXISTS on the JSON face");
        assert_eq!(generation, &Value::Null);

        let probes = ours_sample
            .get("probes")
            .and_then(Value::as_arr)
            .expect("probes array");
        assert_eq!(
            probes[0].get("name").and_then(Value::as_str),
            Some("churn_point")
        );
        let p50 = probes[0]
            .get("p50_ns")
            .and_then(Value::as_f64)
            .expect("p50_ns is a number");
        assert!(p50 > 0.0);
    }

    /// The human artifact names every run, lane, and column.
    #[test]
    fn churn_report_markdown_names_every_lane_and_column() {
        let text = to_markdown(&fixture());
        assert!(text.contains("## run steady"), "{text}");
        assert!(text.contains("### ours-durable (bumbledb)"), "{text}");
        assert!(text.contains("### sqlite-bare (sqlite)"), "{text}");
        assert!(
            text.contains(
                "| cycle | probes (p50 ns) | commits/s | maint ns | disk bytes | gen | id-hw | \
                 freelist | pages |"
            ),
            "{text}"
        );
    }

    /// Both artifacts land in `out_dir` and the JSON one parses.
    #[test]
    fn churn_report_artifacts_land_in_out_dir() {
        let dir = std::env::temp_dir().join("bumbledb-bench-churn-report-artifacts");
        let _ = std::fs::remove_dir_all(&dir);
        write_artifacts(&fixture(), &dir).expect("artifacts write");
        let stem = dir.join("churn-report");
        let json_path = stem.with_extension("json");
        let md_path = stem.with_extension("md");
        assert!(json_path.exists(), "the JSON artifact exists");
        assert!(md_path.exists(), "the markdown artifact exists");
        let text = std::fs::read_to_string(&json_path).expect("readable");
        crate::json::parse(&text).expect("the written artifact parses");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
