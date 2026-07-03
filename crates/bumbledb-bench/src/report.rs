//! The report (docs/benchmarks/18): one run → one self-contained,
//! versionable artifact — comparison tables, gate verdicts, budget
//! checks, allocation and execution statistics, flame summaries, and
//! full provenance. The thing a human reads before making (or refusing)
//! the claim. Renders never write outside `out_dir`; the human copies
//! artifacts into the repo when publishing.

use std::fmt::Write as _;
use std::path::Path;

use crate::families::{self, Kind};
use crate::harness::Stats;
use crate::json;

/// Where the numbers came from. The engine git rev is read at *runtime*
/// (`git rev-parse HEAD` from the repo dir, "unknown" outside one) —
/// a build script would freeze the rev at compile time and lie after a
/// rebase; runtime resolution names the tree the binary actually ran in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub crate_version: String,
    pub git_rev: String,
    /// ISO-8601 UTC, hand-formatted.
    pub timestamp: String,
    pub host: String,
}

/// Resolves provenance from the environment (best-effort fields fall
/// back to "unknown").
#[must_use]
pub fn provenance(repo_dir: &Path) -> Provenance {
    Provenance {
        crate_version: env!("CARGO_PKG_VERSION").to_owned(),
        git_rev: git_rev(repo_dir),
        timestamp: timestamp_iso8601(),
        host: host_description(),
    }
}

fn command_line(program: &str, args: &[&str], dir: Option<&Path>) -> Option<String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_owned())
}

/// The engine git rev at runtime; "unknown" outside a repo.
#[must_use]
pub fn git_rev(repo_dir: &Path) -> String {
    command_line("git", &["rev-parse", "HEAD"], Some(repo_dir))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Best-effort host description (`sysctl -n machdep.cpu.brand_string`).
#[must_use]
pub fn host_description() -> String {
    command_line("sysctl", &["-n", "machdep.cpu.brand_string"], None)
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Civil-from-days (Howard Hinnant's algorithm) — hand-rolled ISO-8601.
fn civil(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).expect("epoch days fit");
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The current UTC time, ISO-8601.
#[must_use]
pub fn timestamp_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil(secs)
}

/// The run's configuration, as printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
}

/// Family gate verdicts. `Win` ⇔ ours p50 strictly < theirs p50 (a tie
/// is a loss — the claim is "faster", not "not slower").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Win,
    Loss,
    ReportOnly,
}

impl Verdict {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Win => "WIN",
            Self::Loss => "LOSS",
            Self::ReportOnly => "report",
        }
    }
}

/// The gate rule, pinned here.
#[must_use]
pub fn verdict(kind: Kind, ours_p50: u64, theirs_p50: u64) -> Verdict {
    match kind {
        Kind::Report => Verdict::ReportOnly,
        Kind::Gate => {
            if ours_p50 < theirs_p50 {
                Verdict::Win
            } else {
                Verdict::Loss
            }
        }
    }
}

/// The warm p99 budget (`00-product.md`): 10 ms, inclusive.
pub const P99_BUDGET_NS: u64 = 10_000_000;

/// Budget check — `≤` passes at the boundary exactly.
#[must_use]
pub fn within_budget(p99_ns: u64) -> bool {
    p99_ns <= P99_BUDGET_NS
}

/// Allocation window numbers, feature-independent plain data (the CLI
/// converts from `AllocSnapshot` when the obs build ran one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocReport {
    pub allocs: u64,
    pub deallocs: u64,
    pub alloc_bytes: u64,
    pub dealloc_bytes: u64,
}

/// The execution digest: the planner-honesty numbers a human scans.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecDigest {
    /// The worst per-node estimate-vs-actual factor.
    pub worst_estimate_factor: f64,
    /// Condensed cover histogram (e.g. `n0:t0x256 n1:t1x255/t2x1`).
    pub covers: String,
    pub emits: u64,
}

/// One read family's comparison row.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Stats,
    pub ratio_p50: f64,
    pub verdict: Verdict,
    pub alloc: Option<AllocReport>,
    pub exec: Option<ExecDigest>,
    pub p99_within_budget: bool,
}

/// One write/cold family's row (`theirs` absent for cold — no `SQLite`
/// mirror exists).
#[derive(Debug, Clone, PartialEq)]
pub struct WriteFamilyReport {
    pub name: String,
    pub ours: Stats,
    pub theirs: Option<Stats>,
    pub facts_per_sec: Option<f64>,
}

/// Store-level numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreNumbers {
    pub db_bytes: u64,
    pub sqlite_bytes: u64,
    pub cache_images: u64,
    pub cache_bytes: u64,
}

/// One traced family's rendered flame table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameEmbed {
    pub name: String,
    pub table: String,
}

/// The whole run, plain data — everything the renderers print.
#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    pub provenance: Provenance,
    pub config: RunConfig,
    pub corpus_digest: String,
    pub verify_stamp: String,
    /// The budget gates at scale L; at S/M it prints as informational.
    pub budget_gates: bool,
    pub reads: Vec<ReadFamilyReport>,
    pub writes: Vec<WriteFamilyReport>,
    pub store: StoreNumbers,
    pub flames: Vec<FlameEmbed>,
}

impl RunReport {
    /// ALL-WIN ⇔ every gated read family wins.
    #[must_use]
    pub fn all_win(&self) -> bool {
        self.reads
            .iter()
            .all(|family| family.verdict != Verdict::Loss)
    }

    /// Every gated family's warm p99 within [`P99_BUDGET_NS`].
    #[must_use]
    pub fn budget_ok(&self) -> bool {
        self.reads
            .iter()
            .filter(|family| family.verdict != Verdict::ReportOnly)
            .all(|family| family.p99_within_budget)
    }
}

#[allow(clippy::cast_precision_loss)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

fn markdown_header(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "# bumbledb bench report\n");
    let _ = writeln!(out, "## Provenance\n");
    let p = &report.provenance;
    let _ = writeln!(out, "- crate version: {}", p.crate_version);
    let _ = writeln!(out, "- engine rev: {}", p.git_rev);
    let _ = writeln!(out, "- timestamp: {}", p.timestamp);
    let _ = writeln!(out, "- host: {}", p.host);
    let _ = writeln!(
        out,
        "- config: scale {}, seed {}, {} samples",
        report.config.scale, report.config.seed, report.config.samples
    );
    let _ = writeln!(out, "- corpus digest: `{}`", report.corpus_digest);
    let _ = writeln!(out, "- verify stamp: `{}`\n", report.verify_stamp);

    let _ = writeln!(out, "## Gate verdict\n");
    if report.all_win() {
        let _ = writeln!(
            out,
            "ALL-WIN — every gated read family beats SQLite on p50."
        );
    } else {
        let losing: Vec<&str> = report
            .reads
            .iter()
            .filter(|family| family.verdict == Verdict::Loss)
            .map(|family| family.name.as_str())
            .collect();
        let _ = writeln!(out, "FAIL — losing families: {}.", losing.join(", "));
    }
    let budget = if report.budget_ok() { "PASS" } else { "FAIL" };
    let scope = if report.budget_gates {
        "gating at scale L"
    } else {
        "informational below scale L"
    };
    let _ = writeln!(out, "p99 budget (<= 10 ms warm): {budget} ({scope}).\n");
}

fn markdown_family_tables(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "## Read families\n");
    let _ = writeln!(
        out,
        "| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for family in &report.reads {
        let _ = writeln!(
            out,
            "| {} | {:.1} / {:.1} / {:.1} | {:.1} / {:.1} / {:.1} | {:.2} | {} |",
            family.name,
            us(family.ours.p50),
            us(family.ours.p95),
            us(family.ours.p99),
            us(family.theirs.p50),
            us(family.theirs.p95),
            us(family.theirs.p99),
            family.ratio_p50,
            family.verdict.label(),
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Write families\n");
    let _ = writeln!(
        out,
        "| family | ours p50 (us) | sqlite p50 (us) | facts/sec |"
    );
    let _ = writeln!(out, "|---|---|---|---|");
    for family in &report.writes {
        let theirs = family
            .theirs
            .map_or_else(|| "-".to_owned(), |stats| format!("{:.1}", us(stats.p50)));
        let throughput = family
            .facts_per_sec
            .map_or_else(|| "-".to_owned(), |v| format!("{v:.0}"));
        let _ = writeln!(
            out,
            "| {} | {:.1} | {theirs} | {throughput} |",
            family.name,
            us(family.ours.p50),
        );
    }
    let _ = writeln!(out);
}

fn markdown_diagnostics(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "## Allocations\n");
    let mut any_window = false;
    for family in &report.reads {
        let Some(alloc) = family.alloc else { continue };
        if !any_window {
            let _ = writeln!(
                out,
                "| family | allocs | deallocs | alloc bytes | dealloc bytes |"
            );
            let _ = writeln!(out, "|---|---|---|---|---|");
            any_window = true;
        }
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            family.name, alloc.allocs, alloc.deallocs, alloc.alloc_bytes, alloc.dealloc_bytes,
        );
    }
    if any_window {
        let _ = writeln!(out);
    } else {
        let _ = writeln!(out, "(not captured — run with the alloc window)\n");
    }

    let _ = writeln!(out, "## Execution digests\n");
    let _ = writeln!(out, "| family | worst est/actual | covers | emits |");
    let _ = writeln!(out, "|---|---|---|---|");
    for family in &report.reads {
        if let Some(exec) = &family.exec {
            let _ = writeln!(
                out,
                "| {} | {:.2} | {} | {} |",
                family.name, exec.worst_estimate_factor, exec.covers, exec.emits,
            );
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Store\n");
    let _ = writeln!(out, "- bumbledb file: {} bytes", report.store.db_bytes);
    let _ = writeln!(out, "- sqlite file: {} bytes", report.store.sqlite_bytes);
    let _ = writeln!(
        out,
        "- image cache: {} images, {} bytes\n",
        report.store.cache_images, report.store.cache_bytes
    );

    let _ = writeln!(out, "## Flame summaries\n");
    if report.flames.is_empty() {
        let _ = writeln!(out, "(none captured — run with --trace)");
    } else {
        for flame in &report.flames {
            let _ = writeln!(out, "### {}\n", flame.name);
            let _ = writeln!(out, "```text\n{}```\n", flame.table);
        }
    }
}

/// The markdown artifact.
#[must_use]
pub fn to_markdown(report: &RunReport) -> String {
    let mut out = String::new();
    markdown_header(&mut out, report);
    markdown_family_tables(&mut out, report);
    markdown_diagnostics(&mut out, report);
    out
}

fn push_read_family(out: &mut String, family: &ReadFamilyReport) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &family.name);
    out.push_str(",\"ours\":");
    push_stats(out, &family.ours);
    out.push_str(",\"theirs\":");
    push_stats(out, &family.theirs);
    let _ = write!(
        out,
        ",\"ratio_p50\":{:.4},\"verdict\":\"{}\",\"p99_within_budget\":{}",
        family.ratio_p50,
        family.verdict.label(),
        family.p99_within_budget
    );
    out.push_str(",\"alloc\":");
    match family.alloc {
        Some(alloc) => {
            let _ = write!(
                out,
                "{{\"allocs\":{},\"deallocs\":{},\"alloc_bytes\":{},\"dealloc_bytes\":{}}}",
                alloc.allocs, alloc.deallocs, alloc.alloc_bytes, alloc.dealloc_bytes
            );
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"exec\":");
    match &family.exec {
        Some(exec) => {
            let _ = write!(
                out,
                "{{\"worst_estimate_factor\":{:.4},\"covers\":",
                exec.worst_estimate_factor
            );
            json::push_str_lit(out, &exec.covers);
            let _ = write!(out, ",\"emits\":{}}}", exec.emits);
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_write_family(out: &mut String, family: &WriteFamilyReport) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &family.name);
    out.push_str(",\"ours\":");
    push_stats(out, &family.ours);
    out.push_str(",\"theirs\":");
    match &family.theirs {
        Some(stats) => push_stats(out, stats),
        None => out.push_str("null"),
    }
    out.push_str(",\"facts_per_sec\":");
    match family.facts_per_sec {
        Some(v) => {
            let _ = write!(out, "{v:.2}");
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_stats(out: &mut String, stats: &Stats) {
    let _ = write!(
        out,
        "{{\"min\":{},\"p50\":{},\"p90\":{},\"p95\":{},\"p99\":{},\"max\":{},\"mean_ns\":{}}}",
        stats.min, stats.p50, stats.p90, stats.p95, stats.p99, stats.max, stats.mean_ns
    );
}

/// The machine-consumable artifact — every field, hand-rolled.
#[must_use]
pub fn to_json(report: &RunReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":{\"crate_version\":");
    json::push_str_lit(&mut out, &report.provenance.crate_version);
    out.push_str(",\"git_rev\":");
    json::push_str_lit(&mut out, &report.provenance.git_rev);
    out.push_str(",\"timestamp\":");
    json::push_str_lit(&mut out, &report.provenance.timestamp);
    out.push_str(",\"host\":");
    json::push_str_lit(&mut out, &report.provenance.host);
    let _ = write!(
        out,
        "}},\"config\":{{\"scale\":\"{}\",\"seed\":{},\"samples\":{}}}",
        report.config.scale, report.config.seed, report.config.samples
    );
    out.push_str(",\"corpus_digest\":");
    json::push_str_lit(&mut out, &report.corpus_digest);
    out.push_str(",\"verify_stamp\":");
    json::push_str_lit(&mut out, &report.verify_stamp);
    let _ = write!(
        out,
        ",\"budget_gates\":{},\"all_win\":{},\"budget_ok\":{}",
        report.budget_gates,
        report.all_win(),
        report.budget_ok()
    );

    out.push_str(",\"reads\":[");
    for (index, family) in report.reads.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_read_family(&mut out, family);
    }

    out.push_str("],\"writes\":[");
    for (index, family) in report.writes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_write_family(&mut out, family);
    }

    let _ = write!(
        out,
        "],\"store\":{{\"db_bytes\":{},\"sqlite_bytes\":{},\"cache_images\":{},\"cache_bytes\":{}}}",
        report.store.db_bytes,
        report.store.sqlite_bytes,
        report.store.cache_images,
        report.store.cache_bytes
    );

    out.push_str(",\"flames\":[");
    for (index, flame) in report.flames.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        json::push_str_lit(&mut out, &flame.name);
        out.push_str(",\"table\":");
        json::push_str_lit(&mut out, &flame.table);
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// Writes exactly three artifacts into `out_dir`: `report.md`,
/// `report.json`, and `QUERIES.md` (the versioned query list from the
/// family registry). The tool never writes into `docs/` — publishing a
/// run is a human copy.
///
/// # Errors
///
/// I/O errors verbatim.
pub fn write_artifacts(report: &RunReport, out_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("report.md"), to_markdown(report))?;
    std::fs::write(out_dir.join("report.json"), to_json(report))?;
    std::fs::write(out_dir.join("QUERIES.md"), families::render_queries_md())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(p50: u64) -> Stats {
        Stats {
            min: p50 / 2,
            p50,
            p90: p50 * 2,
            p95: p50 * 3,
            p99: p50 * 4,
            max: p50 * 5,
            mean_ns: p50,
        }
    }

    fn fixture() -> RunReport {
        RunReport {
            provenance: Provenance {
                crate_version: "0.1.0".to_owned(),
                git_rev: "unknown".to_owned(),
                timestamp: "2026-01-01T00:00:00Z".to_owned(),
                host: "test-host".to_owned(),
            },
            config: RunConfig {
                scale: "S",
                seed: 1,
                samples: 256,
            },
            corpus_digest: "cafe".to_owned(),
            verify_stamp: "beef".to_owned(),
            budget_gates: false,
            reads: vec![ReadFamilyReport {
                name: "point".to_owned(),
                ours: stats(10_000),
                theirs: stats(20_000),
                ratio_p50: 0.5,
                verdict: Verdict::Win,
                alloc: Some(AllocReport {
                    allocs: 0,
                    deallocs: 0,
                    alloc_bytes: 0,
                    dealloc_bytes: 0,
                }),
                exec: Some(ExecDigest {
                    worst_estimate_factor: 1.0,
                    covers: "n0:t0x256".to_owned(),
                    emits: 256,
                }),
                p99_within_budget: true,
            }],
            writes: vec![WriteFamilyReport {
                name: "commit_single".to_owned(),
                ours: stats(100_000),
                theirs: Some(stats(120_000)),
                facts_per_sec: None,
            }],
            store: StoreNumbers {
                db_bytes: 1024,
                sqlite_bytes: 2048,
                cache_images: 3,
                cache_bytes: 4096,
            },
            flames: vec![],
        }
    }

    #[test]
    fn the_markdown_is_golden() {
        let expected = "\
# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: unknown
- timestamp: 2026-01-01T00:00:00Z
- host: test-host
- config: scale S, seed 1, 256 samples
- corpus digest: `cafe`
- verify stamp: `beef`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 10.0 / 30.0 / 40.0 | 20.0 / 60.0 / 80.0 | 0.50 | WIN |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 100.0 | 120.0 | - |

## Allocations

| family | allocs | deallocs | alloc bytes | dealloc bytes |
|---|---|---|---|---|
| point | 0 | 0 | 0 | 0 |

## Execution digests

| family | worst est/actual | covers | emits |
|---|---|---|---|
| point | 1.00 | n0:t0x256 | 256 |

## Store

- bumbledb file: 1024 bytes
- sqlite file: 2048 bytes
- image cache: 3 images, 4096 bytes

## Flame summaries

(none captured — run with --trace)
";
        assert_eq!(to_markdown(&fixture()), expected);

        // The losing render names the family and fails the gate.
        let mut failing = fixture();
        failing.reads[0].verdict = Verdict::Loss;
        let md = to_markdown(&failing);
        assert!(md.contains("FAIL — losing families: point."), "{md}");
        assert!(!failing.all_win());
    }

    #[test]
    fn the_json_is_structurally_sound() {
        let text = to_json(&fixture());
        assert_eq!(text.matches('{').count(), text.matches('}').count());
        assert_eq!(text.matches('[').count(), text.matches(']').count());
        for key in [
            "\"provenance\":",
            "\"config\":",
            "\"corpus_digest\":\"cafe\"",
            "\"verify_stamp\":\"beef\"",
            "\"all_win\":true",
            "\"budget_ok\":true",
            "\"reads\":[",
            "\"writes\":[",
            "\"ratio_p50\":0.5000",
            "\"verdict\":\"WIN\"",
            "\"facts_per_sec\":null",
            "\"store\":{\"db_bytes\":1024",
            "\"flames\":[]",
        ] {
            assert!(text.contains(key), "missing {key} in {text}");
        }
    }

    #[test]
    fn verdict_and_budget_logic_is_table_tested() {
        assert_eq!(verdict(Kind::Gate, 10, 11), Verdict::Win);
        assert_eq!(verdict(Kind::Gate, 10, 10), Verdict::Loss, "a tie loses");
        assert_eq!(verdict(Kind::Gate, 11, 10), Verdict::Loss);
        assert_eq!(verdict(Kind::Report, 1, 100), Verdict::ReportOnly);
        assert!(within_budget(P99_BUDGET_NS), "the boundary passes on <=");
        assert!(!within_budget(P99_BUDGET_NS + 1));
    }

    #[test]
    fn write_artifacts_creates_exactly_the_three_files() {
        let dir = std::env::temp_dir().join("bumbledb-bench-report");
        let _ = std::fs::remove_dir_all(&dir);
        write_artifacts(&fixture(), &dir).expect("writes");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .expect("read dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .into_string()
                    .expect("utf-8")
            })
            .collect();
        names.sort();
        assert_eq!(names, ["QUERIES.md", "report.json", "report.md"]);
        let queries = std::fs::read_to_string(dir.join("QUERIES.md")).expect("read");
        assert_eq!(queries, families::render_queries_md());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_timestamp_formatter_matches_known_epochs() {
        assert_eq!(civil(0), "1970-01-01T00:00:00Z");
        assert_eq!(civil(86_399), "1970-01-01T23:59:59Z");
        // 2026-07-01T12:30:05Z.
        assert_eq!(civil(1_782_909_005), "2026-07-01T12:30:05Z");
    }
}
