use std::fmt::Write as _;

use crate::duralane;
use crate::json::push_str_lit;

use super::enforcement;
use super::run::LawRow;

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn us(ns: u64) -> String {
    format!("{:.3}", ns as f64 / 1000.0)
}

#[must_use]
pub fn markdown(seed: u64, rows: &[LawRow]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# lawful — the integrity home turf (report-class)\n");
    let _ = writeln!(
        out,
        "seed {seed}. This world has no queries — the write families' oracle is the \
         post-state fold over all five ordinary relations plus the naive verdict-parity \
         test. Every row below is REPORT-class, never gated.\n"
    );
    out.push_str("## the enforcement map\n\n");
    out.push_str("| law | statement notation | sqlite enforcement |\n|---|---|---|\n");
    for row in enforcement::MAP {
        let _ = writeln!(
            out,
            "| {} | `{}` | `{}` |",
            row.law, row.notation, row.sqlite
        );
    }
    for lane in duralane::ALL {
        let lane_rows: Vec<&LawRow> = rows.iter().filter(|row| row.lane == lane.label()).collect();
        if lane_rows.is_empty() {
            continue;
        }
        let _ = writeln!(out, "\n## lane `{}`\n\n{}\n", lane.label(), lane.describe());
        out.push_str(
            "| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |\n\
             |---|---:|---:|---:|---:|---|\n",
        );
        for row in lane_rows {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {:.4} | {} | {} |",
                row.family,
                us(row.ours.p50),
                us(row.theirs.p50),
                row.ratio_p50,
                row.work,
                row.about
            );
        }
    }
    out.push_str(
        "\n### rejection latency\n\nThe `law_reject_*` rows price a REFUSED commit \
         round-trip: on the engine, the full dependency judgment plus the abort \
         (`Admission::Rejected`, the complete violation set decoded); on SQLite, the \
         constraint failure — UNIQUE, FK, or a trigger's `RAISE(ABORT)` — plus the \
         `ROLLBACK`. No rejected sample commits anything on either engine (the \
         post-state fold certifies it).\n",
    );

    if rows.iter().any(|row| row.flame.is_some()) {
        let _ = writeln!(out, "\n## Flame summaries (per family, --trace)\n");
        for row in rows {
            if let Some(flame) = &row.flame {
                let _ = writeln!(out, "### {} / {}\n", row.lane, row.family);
                let _ = writeln!(out, "```text\n{flame}```\n");
            }
        }
    }
    out
}

fn push_row(out: &mut String, row: &LawRow) {
    out.push_str("{\"family\":");
    push_str_lit(out, row.family);
    out.push_str(",\"lane\":");
    push_str_lit(out, row.lane);
    out.push_str(",\"about\":");
    push_str_lit(out, row.about);
    out.push_str(",\"ours\":");
    crate::lanes::push_stats(out, &row.ours);
    out.push_str(",\"theirs\":");
    crate::lanes::push_stats(out, &row.theirs);
    let _ = write!(
        out,
        ",\"ratio_p50\":{:.4},\"work\":{}",
        row.ratio_p50, row.work
    );
    crate::lanes::push_ghz(out, Some(row.ghz));
    if let Some(flame) = &row.flame {
        out.push_str(",\"flame\":");
        push_str_lit(out, flame);
    }
    out.push('}');
}

/// The machine artifact — emitted only after every lane's post-state
#[must_use]
pub fn json(seed: u64, rows: &[LawRow]) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\"world\":\"lawful\",\"seed\":{seed},\"provenance\":"
    );
    crate::report::push_provenance(
        &mut out,
        &crate::report::provenance(std::path::Path::new(".")),
    );
    out.push_str(",\"enforcement\":[");
    for (index, row) in enforcement::MAP.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"law\":");
        push_str_lit(&mut out, row.law);
        out.push_str(",\"notation\":");
        push_str_lit(&mut out, row.notation);
        out.push_str(",\"sqlite\":");
        push_str_lit(&mut out, row.sqlite);
        out.push('}');
    }
    out.push_str("],\"lanes\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    out.push_str("],\"poststate\":\"ok\"}");
    out
}
