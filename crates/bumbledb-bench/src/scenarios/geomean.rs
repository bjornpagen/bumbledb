use super::QueryReport;

/// Geometric mean of the p50 ratios (the honest cross-query summary:
/// ratios multiply, so the geomean is the scale-free center).
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
pub fn geomean(reports: &[&QueryReport]) -> f64 {
    if reports.is_empty() {
        return 1.0;
    }
    let log_sum: f64 = reports.iter().map(|r| r.ratio_p50.max(1e-9).ln()).sum();
    (log_sum / reports.len() as f64).exp()
}
