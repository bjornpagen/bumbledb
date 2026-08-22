use bumbledb::obs::{Category, TraceEvent};

use super::{FlameRow, FlameSummary, RENDER_ROWS, containment};

impl FlameSummary {
    /// must not see them; `render_phase_table` does).
    #[must_use]
    pub fn compute(events: &[TraceEvent]) -> Self {
        let sweep = containment::sweep(events);
        let mut by_name: std::collections::BTreeMap<&'static str, (Vec<u64>, u64)> =
            std::collections::BTreeMap::new();
        for (index, event) in sweep.spans.iter().enumerate() {
            let entry = by_name.entry(event.name()).or_default();
            entry.0.push(event.dur_ns());
            entry.1 += event.dur_ns() - sweep.child_ns[index];
        }
        for event in events
            .iter()
            .filter(|e| matches!(e, TraceEvent::Point { .. } if e.cat() != Category::Phase))
        {
            by_name.entry(event.name()).or_default().0.push(0);
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
            events.iter().map(|e| e.start_ns()).min(),
            events.iter().map(|e| e.start_ns() + e.dur_ns()).max(),
        ) {
            (Some(start), Some(end)) => end - start,
            _ => 0,
        };
        Self { rows, wall_ns }
    }

    #[must_use]
    pub fn render(&self) -> String {
        self.render_top(RENDER_ROWS)
    }

    #[must_use]
    pub fn render_top(&self, rows: usize) -> String {
        use std::fmt::Write as _;
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let us = |ns: u64| ns as f64 / 1000.0;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{:<24} {:>7} {:>12} {:>12} {:>12} {:>12}",
            "span", "calls", "total_us", "self_us", "p50_us", "max_us"
        );
        for row in self.rows.iter().take(rows) {
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
