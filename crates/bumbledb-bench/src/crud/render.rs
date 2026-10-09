use std::fmt::Write as _;

use crate::json::push_str_lit;
use crate::scenarios::json_out::push_stats;

use super::run::CrudRow;

fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

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
    let _ = writeln!(out, "{}\n", crate::sqlite_run::DURABILITY);
    let _ = writeln!(
        out,
        "| family | about | ours p50 (µs) | sqlite p50 (µs) | ratio | ours p99 (µs) | sqlite p99 (µs) |"
    );
    let _ = writeln!(out, "|---|---|---:|---:|---:|---:|---:|");
    for row in rows {
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
    let _ = writeln!(
        out,
        "\npost-state: Doc + Counter value-identical across engines. Every row \
         above is report-class, never gated."
    );

    out
}

#[must_use]
pub fn json(rows: &[CrudRow], seed: u64) -> String {
    let mut out = String::new();
    let _ = write!(out, "{{\"world\":\"crud\",\"seed\":{seed},\"provenance\":");
    crate::report::push_provenance(
        &mut out,
        &crate::report::provenance(std::path::Path::new(".")),
    );
    out.push_str(",\"config\":");
    push_str_lit(&mut out, crate::sqlite_run::DURABILITY);
    out.push_str(",\"rows\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_row(&mut out, row);
    }
    out.push_str("],\"poststate\":\"ok\"}");
    out
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
    out.push('}');
}
