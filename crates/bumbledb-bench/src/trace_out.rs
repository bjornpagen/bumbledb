//! Trace export (docs/benchmarks/17): every captured run becomes a
//! Chrome Trace Format artifact (Perfetto / `chrome://tracing`) plus a
//! terminal flame summary — where-the-time-goes without leaving the
//! repo. Hand-rolled JSON, per the dependency quarantine.

use std::io::Write;
use std::path::{Path, PathBuf};

use bumbledb::obs::{Category, TraceEvent};

/// Splits one capture into (engine, harness) event streams — the
/// harness's own spans export under a separate tid so tool overhead is
/// honestly separated.
#[must_use]
pub fn split_harness(events: Vec<TraceEvent>) -> (Vec<TraceEvent>, Vec<TraceEvent>) {
    events
        .into_iter()
        .partition(|event| event.cat != Category::Harness)
}

fn write_event(out: &mut impl Write, event: &TraceEvent, tid: u32) -> std::io::Result<()> {
    // Names are `&'static str` constants from `obs::names` — ASCII by
    // registry discipline (asserted in tests), so no escaping machinery.
    debug_assert!(
        !event.name.contains('"') && !event.name.contains('\\'),
        "trace names never need escaping"
    );
    #[allow(clippy::cast_precision_loss)] // ns fit f64 exactly for ~104 days
    let ts = event.start_ns as f64 / 1000.0;
    if event.dur_ns == 0 {
        write!(
            out,
            "{{\"name\":\"{}\",\"cat\":\"{}\",\"ph\":\"i\",\"ts\":{ts:.3},\"s\":\"t\",\
             \"pid\":1,\"tid\":{tid},\"args\":{{\"a0\":{},\"a1\":{}}}}}",
            event.name,
            event.cat.label(),
            event.a0,
            event.a1,
        )
    } else {
        #[allow(clippy::cast_precision_loss)]
        let dur = event.dur_ns as f64 / 1000.0;
        write!(
            out,
            "{{\"name\":\"{}\",\"cat\":\"{}\",\"ph\":\"X\",\"ts\":{ts:.3},\"dur\":{dur:.3},\
             \"pid\":1,\"tid\":{tid},\"args\":{{\"a0\":{},\"a1\":{}}}}}",
            event.name,
            event.cat.label(),
            event.a0,
            event.a1,
        )
    }
}

/// Emits the Chrome Trace Event Format: one JSON array of complete
/// events (`ph: "X"`, timestamps and durations in microseconds with
/// three decimals) and instant events (`ph: "i"`) for zero-duration
/// points. Engine events carry `tid` 1, harness events `tid` 2; file
/// order is start-time order (spans record at drop, so the capture
/// arrives in end order and is re-sorted here).
///
/// # Errors
///
/// Writer errors verbatim.
pub fn write_chrome(
    events: &[TraceEvent],
    harness: &[TraceEvent],
    out: &mut impl Write,
) -> std::io::Result<()> {
    let mut all: Vec<(&TraceEvent, u32)> = events
        .iter()
        .map(|e| (e, 1))
        .chain(harness.iter().map(|e| (e, 2)))
        .collect();
    all.sort_by_key(|(e, _)| e.start_ns);
    out.write_all(b"[\n")?;
    for (index, (event, tid)) in all.iter().enumerate() {
        if index > 0 {
            out.write_all(b",\n")?;
        }
        write_event(out, event, *tid)?;
    }
    out.write_all(b"\n]\n")
}

/// Writes one capture to `<trace_dir>/<stem>.json`, creating the
/// directory (the bench integration: `<family>.warm.json`,
/// `<family>.cold.json`, one per traced sample).
///
/// # Errors
///
/// I/O errors verbatim.
pub fn write_trace_file(
    trace_dir: &Path,
    stem: &str,
    events: &[TraceEvent],
    harness: &[TraceEvent],
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(trace_dir)?;
    let path = trace_dir.join(format!("{stem}.json"));
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path)?);
    write_chrome(events, harness, &mut file)?;
    file.flush()?;
    Ok(path)
}

/// One aggregated span name in the flame summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameRow {
    pub name: &'static str,
    pub calls: u64,
    pub total_ns: u64,
    /// Total minus the durations of *directly* nested children.
    pub self_ns: u64,
    pub p50_ns: u64,
    pub max_ns: u64,
}

/// The terminal where-the-time-goes table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlameSummary {
    /// Sorted by self time, descending.
    pub rows: Vec<FlameRow>,
    /// `max(end) - min(start)` over every event.
    pub wall_ns: u64,
}

/// How many rows the render keeps.
const RENDER_ROWS: usize = 24;

impl FlameSummary {
    /// Aggregates a capture by span name. Containment is a stack sweep:
    /// spans re-sorted by `(start, -end)` and walked, so each span's
    /// duration is charged to its *direct* parent's child time; point
    /// events count as calls with zero duration.
    #[must_use]
    pub fn compute(events: &[TraceEvent]) -> Self {
        let mut spans: Vec<&TraceEvent> = events.iter().filter(|e| e.dur_ns > 0).collect();
        spans.sort_by_key(|e| (e.start_ns, std::cmp::Reverse(e.start_ns + e.dur_ns)));
        let mut child_ns = vec![0u64; spans.len()];
        let mut stack: Vec<usize> = Vec::new();
        for (index, event) in spans.iter().enumerate() {
            while let Some(&top) = stack.last() {
                if spans[top].start_ns + spans[top].dur_ns <= event.start_ns {
                    stack.pop();
                } else {
                    break;
                }
            }
            if let Some(&parent) = stack.last() {
                child_ns[parent] += event.dur_ns;
            }
            stack.push(index);
        }

        let mut by_name: std::collections::BTreeMap<&'static str, (Vec<u64>, u64)> =
            std::collections::BTreeMap::new();
        for (index, event) in spans.iter().enumerate() {
            let entry = by_name.entry(event.name).or_default();
            entry.0.push(event.dur_ns);
            entry.1 += event.dur_ns - child_ns[index];
        }
        for event in events.iter().filter(|e| e.dur_ns == 0) {
            by_name.entry(event.name).or_default().0.push(0);
        }

        let mut rows: Vec<FlameRow> = by_name
            .into_iter()
            .map(|(name, (mut durs, self_ns))| {
                let stats = crate::harness::stats(&mut durs);
                FlameRow {
                    name,
                    calls: durs.len() as u64,
                    total_ns: durs.iter().sum(),
                    self_ns,
                    p50_ns: stats.p50,
                    max_ns: stats.max,
                }
            })
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse((row.self_ns, row.name)));

        let wall_ns = match (
            events.iter().map(|e| e.start_ns).min(),
            events.iter().map(|e| e.start_ns + e.dur_ns).max(),
        ) {
            (Some(start), Some(end)) => end - start,
            _ => 0,
        };
        Self { rows, wall_ns }
    }

    /// The aligned text table: top [`RENDER_ROWS`] rows by self time,
    /// microseconds with three decimals, plus the total-wall line.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        #[allow(clippy::cast_precision_loss)]
        let us = |ns: u64| ns as f64 / 1000.0;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{:<24} {:>7} {:>12} {:>12} {:>12} {:>12}",
            "span", "calls", "total_us", "self_us", "p50_us", "max_us"
        );
        for row in self.rows.iter().take(RENDER_ROWS) {
            let _ = writeln!(
                out,
                "{:<24} {:>7} {:>12.3} {:>12.3} {:>12.3} {:>12.3}",
                row.name,
                row.calls,
                us(row.total_ns),
                us(row.self_ns),
                us(row.p50_ns),
                us(row.max_ns),
            );
        }
        let _ = writeln!(out, "total wall {:.3} us", us(self.wall_ns));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(name: &'static str, cat: Category, start_ns: u64, dur_ns: u64, a0: u64) -> TraceEvent {
        TraceEvent {
            name,
            cat,
            start_ns,
            dur_ns,
            a0,
            a1: 0,
        }
    }

    #[test]
    fn the_chrome_writer_is_golden_and_structurally_valid() {
        let engine = vec![
            span("prepare", Category::Prepare, 1000, 2500, 0),
            span("execute", Category::Execute, 4000, 10000, 7),
            span("join", Category::Execute, 5000, 8000, 0),
            span("cache_hit", Category::Cache, 6000, 0, 3),
        ];
        let harness = vec![span("sample", Category::Harness, 900, 15000, 0)];
        let mut out = Vec::new();
        write_chrome(&engine, &harness, &mut out).expect("writes");
        let text = String::from_utf8(out).expect("utf-8");
        let expected = "[\n\
            {\"name\":\"sample\",\"cat\":\"harness\",\"ph\":\"X\",\"ts\":0.900,\"dur\":15.000,\"pid\":1,\"tid\":2,\"args\":{\"a0\":0,\"a1\":0}},\n\
            {\"name\":\"prepare\",\"cat\":\"prepare\",\"ph\":\"X\",\"ts\":1.000,\"dur\":2.500,\"pid\":1,\"tid\":1,\"args\":{\"a0\":0,\"a1\":0}},\n\
            {\"name\":\"execute\",\"cat\":\"execute\",\"ph\":\"X\",\"ts\":4.000,\"dur\":10.000,\"pid\":1,\"tid\":1,\"args\":{\"a0\":7,\"a1\":0}},\n\
            {\"name\":\"join\",\"cat\":\"execute\",\"ph\":\"X\",\"ts\":5.000,\"dur\":8.000,\"pid\":1,\"tid\":1,\"args\":{\"a0\":0,\"a1\":0}},\n\
            {\"name\":\"cache_hit\",\"cat\":\"cache\",\"ph\":\"i\",\"ts\":6.000,\"s\":\"t\",\"pid\":1,\"tid\":1,\"args\":{\"a0\":3,\"a1\":0}}\n\
            ]\n";
        assert_eq!(text, expected);

        // Structural validity: balanced brackets, one object per event,
        // ts monotone nondecreasing in file order.
        assert_eq!(text.matches('{').count(), text.matches('}').count());
        assert_eq!(text.matches("\"name\":").count(), 5);
        let ts: Vec<f64> = text
            .lines()
            .filter_map(|line| {
                let start = line.find("\"ts\":")? + 5;
                let rest = &line[start..];
                let end = rest.find(',')?;
                rest[..end].parse().ok()
            })
            .collect();
        assert_eq!(ts.len(), 5);
        assert!(ts.windows(2).all(|w| w[0] <= w[1]), "{ts:?}");
    }

    #[test]
    fn every_registered_name_is_escape_free_ascii() {
        // The writer relies on the registry discipline instead of
        // escaping machinery.
        assert!(FlameSummary::compute(&[]).rows.is_empty());
        let names = [
            bumbledb::obs::names::PREPARE,
            bumbledb::obs::names::EXECUTE,
            bumbledb::obs::names::JOIN,
            bumbledb::obs::names::VIEW_BUILD,
            bumbledb::obs::names::VIEW_MEMO_HIT,
            bumbledb::obs::names::SAMPLE,
            bumbledb::obs::names::TOUCH,
        ];
        for name in names {
            assert!(
                name.is_ascii() && !name.contains('"') && !name.contains('\\'),
                "{name}"
            );
        }
    }

    #[test]
    fn the_flame_summary_computes_exact_self_time() {
        // Outer 100 us containing inner 60 us: outer self = 40 us.
        let events = vec![
            span("outer", Category::Execute, 0, 100_000, 0),
            span("inner", Category::Execute, 10_000, 60_000, 0),
        ];
        let summary = FlameSummary::compute(&events);
        assert_eq!(summary.wall_ns, 100_000);
        assert_eq!(summary.rows.len(), 2);
        let inner = &summary.rows[0];
        assert_eq!(
            (inner.name, inner.total_ns, inner.self_ns),
            ("inner", 60_000, 60_000),
            "inner leads by self time"
        );
        let outer = &summary.rows[1];
        assert_eq!(
            (outer.name, outer.total_ns, outer.self_ns),
            ("outer", 100_000, 40_000)
        );

        // Only DIRECT children are subtracted: grandchildren charge the
        // middle span, not the outer one.
        let nested = vec![
            span("outer", Category::Execute, 0, 100_000, 0),
            span("middle", Category::Execute, 10_000, 60_000, 0),
            span("leaf", Category::Execute, 20_000, 30_000, 0),
        ];
        let summary = FlameSummary::compute(&nested);
        let by_name = |name: &str| {
            summary
                .rows
                .iter()
                .find(|row| row.name == name)
                .expect("row")
                .self_ns
        };
        assert_eq!(by_name("outer"), 40_000);
        assert_eq!(by_name("middle"), 30_000);
        assert_eq!(by_name("leaf"), 30_000);
    }

    #[test]
    fn the_table_render_is_golden() {
        let events = vec![
            span("outer", Category::Execute, 0, 100_000, 0),
            span("inner", Category::Execute, 10_000, 60_000, 0),
        ];
        let summary = FlameSummary::compute(&events);
        let expected = "span                       calls     total_us      self_us       p50_us       max_us\n\
                        inner                          1       60.000       60.000       60.000       60.000\n\
                        outer                          1      100.000       40.000      100.000      100.000\n\
                        total wall 100.000 us\n";
        assert_eq!(summary.render(), expected);
    }

    /// A real captured S-scale `fk_walk` trace: the expected spans appear
    /// and the summary wall tracks the execute span within 5%.
    #[cfg(feature = "obs")]
    #[test]
    fn a_real_fk_walk_capture_summarizes_to_the_execute_span() {
        use crate::gen::{GenConfig, Scale};
        use crate::harness::Rotation;

        let dir = std::env::temp_dir().join("bumbledb-bench-trace-out");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::S,
        };
        let db = bumbledb::Db::create(&dir.join("db"), crate::schema::schema()).expect("create");
        crate::corpus::load_bumbledb(&db, cfg).expect("load");

        let family = crate::families::all()
            .iter()
            .find(|f| f.name == "fk_walk")
            .expect("registered");
        let mut prepared = db.prepare(&(family.query)()).expect("prepare");
        let mut rotation = Rotation::new((family.params)(&cfg));
        let mut buffer = bumbledb::ResultBuffer::new();
        let mut run = || {
            let params = rotation.next_set().to_vec();
            db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
                .map_err(|e| format!("{e:?}"))?;
            Ok(buffer.len() as u64)
        };
        // Warm first — the traced sample is a warm one.
        for _ in 0..4 {
            run().expect("warm");
        }
        let (_, events) = crate::harness::traced_sample(&mut run).expect("traced");
        let (engine, harness) = split_harness(events);
        let names: std::collections::HashSet<&str> =
            engine.iter().map(|event| event.name).collect();
        assert!(names.contains("execute"), "{names:?}");
        assert!(names.contains("join"), "{names:?}");
        assert!(
            names.contains("view_build") || names.contains("view_memo_hit"),
            "{names:?}"
        );
        assert_eq!(harness.len(), 1, "the sample span");

        let summary = FlameSummary::compute(&engine);
        let execute = summary
            .rows
            .iter()
            .find(|row| row.name == "execute")
            .expect("execute row");
        let wall = summary.wall_ns;
        assert!(
            wall.abs_diff(execute.total_ns) * 20 <= execute.total_ns,
            "wall {wall} vs execute {} exceeds 5%",
            execute.total_ns
        );

        // And it exports.
        let path = write_trace_file(&dir.join("trace"), "fk_walk.warm", &engine, &harness)
            .expect("export");
        let text = std::fs::read_to_string(path).expect("read back");
        assert!(text.starts_with("[\n") && text.ends_with("\n]\n"));
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
