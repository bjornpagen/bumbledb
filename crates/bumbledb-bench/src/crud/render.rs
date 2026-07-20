//! The crud artifacts, rendered from [`CrudRow`]s. The lane config
//! prose renders VERBATIM from [`DurabilityLane::describe`] — the SAME
//! value that configured the store — so documentation and configuration
//! share one representation and cannot drift. Both artifacts carry
//! every row; nothing here is a gate.

use std::fmt::Write as _;

use crate::duralane::{self, DurabilityLane};
use crate::json::push_str_lit;
use crate::scenarios::json_out::push_stats;

use super::run::CrudRow;

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

/// The human artifact: one `## lane` section per durability lane,
/// opening with the lane's parity-config prose (constructor + every
/// `SQLite` pragma) and closing on the post-state footer.
#[must_use]
pub fn markdown(rows: &[CrudRow], seed: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# crud — the OLTP home turf (report-class; SQLite's strong regime, benched to lose honestly)\n"
    );
    let _ = writeln!(
        out,
        "Seed {seed}. One shared op stream per family, folded by both engines; \
         the read query oracle-gated (value-identical multisets) on every lane \
         before any timed window. ratio = ours p50 / sqlite p50 (lower is \
         better; <1 = bumbledb faster).\n"
    );
    for lane in duralane::ALL {
        let _ = writeln!(out, "## lane {}\n", lane.label());
        let _ = writeln!(out, "{}\n", lane.describe());
        let _ = writeln!(
            out,
            "| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |"
        );
        let _ = writeln!(out, "|---|---|---:|---:|---:|---:|---:|");
        for row in rows.iter().filter(|row| row.lane == lane.label()) {
            let _ = writeln!(
                out,
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
                row.family,
                row.about,
                us(row.ours.p50),
                us(row.theirs.p50),
                row.ratio_p50,
                us(row.ours.p99),
                us(row.theirs.p99),
            );
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "post-state: Doc + Counter value-identical across engines, both lanes. \
         Every row above is report-class, never gated — no budget gate reads a \
         crud number."
    );
    out
}

/// The machine artifact, hand-rolled through [`crate::json`] (the
/// dependency quarantine forbids serde): lanes in report order, each
/// carrying its verbatim config prose and its rows; stats objects
/// byte-shaped like `report.json`'s.
#[must_use]
pub fn json(rows: &[CrudRow], seed: u64) -> String {
    let mut out = String::new();
    let _ = write!(out, "{{\"world\":\"crud\",\"seed\":{seed},\"lanes\":[");
    for (index, lane) in duralane::ALL.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_lane(&mut out, *lane, rows);
    }
    out.push_str("],\"poststate\":\"ok\"}");
    out
}

fn push_lane(out: &mut String, lane: DurabilityLane, rows: &[CrudRow]) {
    out.push_str("{\"lane\":");
    push_str_lit(out, lane.label());
    out.push_str(",\"config\":");
    push_str_lit(out, lane.describe());
    out.push_str(",\"rows\":[");
    for (index, row) in rows
        .iter()
        .filter(|row| row.lane == lane.label())
        .enumerate()
    {
        if index > 0 {
            out.push(',');
        }
        push_row(out, row);
    }
    out.push_str("]}");
}

fn push_row(out: &mut String, row: &CrudRow) {
    out.push_str("{\"family\":");
    push_str_lit(out, row.family);
    out.push_str(",\"about\":");
    push_str_lit(out, row.about);
    out.push_str(",\"ours\":");
    push_stats(out, &row.ours);
    out.push_str(",\"theirs\":");
    push_stats(out, &row.theirs);
    let _ = write!(
        out,
        ",\"ratio_p50\":{:.4},\"work\":{}",
        row.ratio_p50, row.work
    );
    out.push_str(",\"ghz\":");
    match row.ghz {
        Some(g) => {
            let _ = write!(
                out,
                "{{\"pre\":{:.3},\"post\":{:.3},\"retried\":{},\"contaminated\":{}}}",
                g.pre, g.post, g.retried, g.contaminated
            );
        }
        None => out.push_str("null"),
    }
    out.push('}');
}
