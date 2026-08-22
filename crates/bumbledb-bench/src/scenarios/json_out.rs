use std::fmt::Write as _;
use std::path::Path;

use super::{LaneOutcome, QueryReport};
use crate::harness::{Protocol, Stats};
use crate::{json, report};

pub(crate) fn push_stats(out: &mut String, stats: &Stats) {
    let _ = write!(
        out,
        "{{\"min\":{},\"p50\":{},\"p90\":{},\"p95\":{},\"p99\":{},\"max\":{},\"mean_ns\":{}}}",
        stats.min, stats.p50, stats.p90, stats.p95, stats.p99, stats.max, stats.mean_ns
    );
}

fn push_lane(out: &mut String, lane: &super::LaneReport) {
    out.push_str("{\"lane\":");
    json::push_str_lit(out, lane.lane);
    match &lane.outcome {
        LaneOutcome::Timed { stats, ratio_p50 } => {
            out.push_str(",\"outcome\":\"timed\",\"stats\":");
            push_stats(out, stats);
            let _ = write!(out, ",\"ratio_p50\":{ratio_p50:.4}");
        }
        LaneOutcome::ExceededCap { cap } => {
            let _ = write!(out, ",\"outcome\":\"exceeded_cap\",\"cap_ms\":{}", cap.0);
        }
    }
    out.push('}');
}

fn push_query(out: &mut String, r: &QueryReport) {
    out.push_str("{\"scenario\":");
    json::push_str_lit(out, r.scenario);
    out.push_str(",\"name\":");
    json::push_str_lit(out, r.name);
    out.push_str(",\"about\":");
    json::push_str_lit(out, r.about);
    let _ = write!(out, ",\"answers\":{}", r.answers);
    out.push_str(",\"ours\":");
    push_stats(out, &r.ours);
    out.push_str(",\"lanes\":[");
    for (index, lane) in r.lanes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_lane(out, lane);
    }
    out.push(']');
    if let Some(alloc) = &r.alloc {
        let _ = write!(
            out,
            ",\"alloc\":{{\"allocs\":{},\"deallocs\":{},\"alloc_bytes\":{},\"dealloc_bytes\":{}}}",
            alloc.allocs, alloc.deallocs, alloc.alloc_bytes, alloc.dealloc_bytes
        );
    }
    if let Some(flame) = &r.flame {
        out.push_str(",\"flame\":");
        json::push_str_lit(out, flame);
    }
    out.push('}');
}

#[must_use]
pub fn to_json(reports: &[QueryReport], proto: Protocol, seed: u64) -> String {
    let mut out = String::new();
    out.push_str("{\"timestamp\":");
    json::push_str_lit(&mut out, &report::timestamp_iso8601());
    out.push_str(",\"git_rev\":");
    json::push_str_lit(&mut out, &report::git_rev(Path::new(".")));
    let _ = write!(
        &mut out,
        ",\"seed\":{seed},\"warmups\":{},\"samples\":{}",
        proto.warmups, proto.samples
    );
    out.push_str(",\"queries\":[");
    for (index, r) in reports.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_query(&mut out, r);
    }
    out.push_str("]}");
    out
}
