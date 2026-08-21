//! The upstream report's component table
//! (proposals/one-representation/10-measurement.md § the output): span
//! names → report components, the ONE mapping every primerlane trace
//! folds through. The accumulation-side points (`MARSHAL_FACTS`,
//! `DYN_PARSE`, `DYN_ENCODE`, `INTERN_PROBE`, `DELTA_APPLY`,
//! `BUILDER_LOAD`) are registered here before their call sites land —
//! the 20/30 lanes wire them; this table is where their totals already
//! have a home, so a wired span changes a row from zero, never the
//! report shape.

use bumbledb::obs::{TraceEvent, TracePoint, names};

/// Component rows in the upstream report's order; each names the spans
/// whose durations it sums. Components 1 (TS fact projection) and
/// 10–12 (the read lanes) are wall-clock phases, not span sums, so they
/// live in the phase table, not here.
pub const COMPONENTS: &[(&str, &[TracePoint])] = &[
    ("JavaScript-to-native marshaling", &[names::MARSHAL_FACTS]),
    ("native batch parsing", &[names::DYN_PARSE]),
    (
        "string ownership and interning",
        &[names::INTERN_PROBE, names::DYN_ENCODE],
    ),
    ("delta apply", &[names::DELTA_APPLY, names::BUILDER_LOAD]),
    (
        "commit judgment",
        &[
            names::JUDGMENT_SOURCE,
            names::JUDGMENT_TARGET,
            names::JUDGMENT_CAPACITIES,
        ],
    ),
    ("dictionary flush", &[names::COUNTERS_FLUSH]),
    (
        "relation and determinant index application",
        &[names::APPLY_INSERTS, names::APPLY_DELETES],
    ),
    (
        "LMDB commit",
        &[names::LMDB_COMMIT, names::PUBLISH_COPY, names::PUBLISH_SYNC],
    ),
];

/// One folded component: recorded events and their summed span time
/// (point events contribute calls, zero duration — `INTERN_PROBE` is an
/// event by decision).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentRow {
    pub name: &'static str,
    pub calls: u64,
    pub total_ns: u64,
}

/// Folds one capture into the component table — every row present,
/// unwired components at zero.
#[must_use]
pub fn totals(events: &[TraceEvent]) -> Vec<ComponentRow> {
    COMPONENTS
        .iter()
        .map(|(name, points)| {
            let mut calls = 0u64;
            let mut total_ns = 0u64;
            for event in events {
                if points.contains(&event.point()) {
                    calls += 1;
                    total_ns += event.dur_ns();
                }
            }
            ComponentRow {
                name,
                calls,
                total_ns,
            }
        })
        .collect()
}
