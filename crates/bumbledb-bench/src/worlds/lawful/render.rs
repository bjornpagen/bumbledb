use std::fmt::Write as _;

use crate::json::push_str_lit;

use super::enforcement;
use super::run::LawRow;

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
    if !rows.is_empty() {
        let _ = writeln!(
            out,
            "\n## families\n\n{}\n",
            crate::harness::sqlite_run::DURABILITY
        );
        out.push_str(
            "| family | ours p50 µs | sqlite p50 µs | ratio p50 (ours/sqlite) | work | about |\n\
             |---|---:|---:|---:|---:|---|\n",
        );
        for row in rows {
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
         (`WriteOutcome::Rejected`, the complete violation set decoded); on SQLite, the \
         constraint failure — UNIQUE, FK, or a trigger's `RAISE(ABORT)` — plus the \
         `ROLLBACK`. No rejected sample commits anything on either engine (the \
         post-state fold certifies it).\n",
    );

    out
}

fn push_row(out: &mut String, row: &LawRow) {
    out.push_str("{\"family\":");
    push_str_lit(out, row.family);
    out.push_str(",\"about\":");
    push_str_lit(out, row.about);
    out.push_str(",\"ours\":");
    crate::harness::lanes::push_stats(out, &row.ours);
    out.push_str(",\"theirs\":");
    crate::harness::lanes::push_stats(out, &row.theirs);
    let _ = write!(
        out,
        ",\"ratio_p50\":{:.4},\"work\":{}",
        row.ratio_p50, row.work
    );
    out.push('}');
}

/// The machine artifact, emitted only after the post-state comparison passes.
#[must_use]
pub fn json(seed: u64, rows: &[LawRow]) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\"world\":\"lawful\",\"seed\":{seed},\"provenance\":"
    );
    crate::harness::report::push_provenance(
        &mut out,
        &crate::harness::report::provenance(std::path::Path::new(".")),
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
    out.push_str("],\"rows\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    out.push_str("],\"poststate\":\"ok\"}");
    out
}
