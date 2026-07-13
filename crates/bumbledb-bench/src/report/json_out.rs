use std::fmt::Write as _;

use super::{GhzReport, ReadFamilyReport, RunReport, Stats, WriteFamilyReport};

use crate::json;

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
            let _ = write!(
                out,
                ",\"emitted\":{},\"absorbed\":{}}}",
                exec.emitted, exec.absorbed
            );
        }
        None => out.push_str("null"),
    }
    push_ghz(out, family.ghz);
    out.push_str(",\"p50_norm\":");
    match family.p50_norm {
        Some(v) => {
            let _ = write!(out, "{v}");
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_ghz(out: &mut String, ghz: Option<GhzReport>) {
    out.push_str(",\"ghz\":");
    match ghz {
        Some(g) => {
            let _ = write!(
                out,
                "{{\"pre\":{:.3},\"post\":{:.3},\"retried\":{},\"contaminated\":{}}}",
                g.pre, g.post, g.retried, g.contaminated
            );
        }
        None => out.push_str("null"),
    }
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
    push_ghz(out, family.ghz);
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
        ",\"budget_gates\":{},\"partial\":{},\"all_win\":{},\"budget_ok\":{}",
        report.budget_gates,
        report.partial,
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
