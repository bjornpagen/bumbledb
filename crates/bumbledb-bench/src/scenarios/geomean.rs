use super::{LaneOutcome, QueryReport};

/// Geometric mean of the primary-lane p50 ratios (the honest cross-query
/// summary: ratios multiply, so the geomean is the scale-free center).
/// A DNF primary lane contributes nothing — a censored ratio is not a
/// ratio, so exceeded-cap queries are excluded (and counted by
/// [`dnf_count`]).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
pub fn geomean(reports: &[&QueryReport]) -> f64 {
    let ratios: Vec<f64> = reports.iter().filter_map(|r| r.primary_ratio()).collect();
    if ratios.is_empty() {
        return 1.0;
    }
    let log_sum: f64 = ratios.iter().map(|r| r.max(1e-9).ln()).sum();
    (log_sum / ratios.len() as f64).exp()
}

/// How many reports carry at least one exceeded-cap lane — the DNFs the
/// renderers count beside every geomean (honesty in both directions:
/// excluded, never hidden).
#[must_use]
pub fn dnf_count(reports: &[&QueryReport]) -> usize {
    reports
        .iter()
        .filter(|r| {
            r.lanes
                .iter()
                .any(|lane| matches!(lane.outcome, LaneOutcome::ExceededCap { .. }))
        })
        .count()
}
