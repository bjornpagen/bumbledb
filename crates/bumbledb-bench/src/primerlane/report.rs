//! The primerlane report artifact: plain data, hand-rolled JSON +
//! markdown (the dependency quarantine) — the before/after evidence
//! table 80-acceptance.md attaches to every condition (gate G4).

use std::fmt::Write as _;

use crate::report::Provenance;

/// One measured phase: wall time by std `Instant`, rows processed, and
/// (under `--alloc`) the phase's allocation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseRow {
    pub name: &'static str,
    pub wall_ns: u64,
    pub rows: u64,
    pub alloc: Option<PhaseAlloc>,
}

/// One phase's allocation window ([`bumbledb::alloc_counter`]): window
/// events and bytes, plus the absolute peak-live high-water read at
/// phase end (peak is process-monotone; the per-phase reading is the
/// high-water as of that phase's close).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseAlloc {
    pub allocs: u64,
    pub deallocs: u64,
    pub alloc_bytes: u64,
    pub dealloc_bytes: u64,
    pub peak_live_bytes: u64,
}

/// The whole primerlane report, plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct PrimerlaneReport {
    pub provenance: Provenance,
    pub relations: u32,
    pub facts: u64,
    pub seed: u64,
    pub phases: Vec<PhaseRow>,
}

fn push_phase(out: &mut String, row: &PhaseRow) {
    let _ = write!(
        out,
        "{{\"name\":\"{}\",\"wall_ns\":{},\"rows\":{},\"alloc\":",
        row.name, row.wall_ns, row.rows
    );
    match &row.alloc {
        Some(alloc) => {
            let _ = write!(
                out,
                "{{\"allocs\":{},\"deallocs\":{},\"alloc_bytes\":{},\"dealloc_bytes\":{},\"peak_live_bytes\":{}}}",
                alloc.allocs,
                alloc.deallocs,
                alloc.alloc_bytes,
                alloc.dealloc_bytes,
                alloc.peak_live_bytes
            );
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

/// The machine-consumable primerlane artifact.
#[must_use]
pub fn to_json(report: &PrimerlaneReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    crate::report::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"relations\":{},\"facts\":{},\"seed\":{},\"phases\":[",
        report.relations, report.facts, report.seed
    );
    for (i, row) in report.phases.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_phase(&mut out, row);
    }
    out.push(']');
    out.push('}');
    out
}

/// The human table.
#[must_use]
pub fn to_markdown(report: &PrimerlaneReport) -> String {
    let alloc = report.phases.iter().any(|row| row.alloc.is_some());
    let mut out = String::new();
    out.push_str("# Primer-shaped attribution lane\n\n");
    let _ = writeln!(
        out,
        "relations {} · facts {} · seed {}\n",
        report.relations, report.facts, report.seed
    );
    if alloc {
        out.push_str(
            "| phase | wall ms | rows | ns/row | allocs | deallocs | alloc MiB | peak live MiB |\n\
             | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
        );
    } else {
        out.push_str("| phase | wall ms | rows | ns/row |\n| --- | ---: | ---: | ---: |\n");
    }
    for row in &report.phases {
        #[allow(
            clippy::cast_precision_loss,
            reason = "printed milliseconds and ratios; mantissa loss is below the table's decimals"
        )]
        let (ms, ns_per_row) = (
            row.wall_ns as f64 / 1e6,
            if row.rows == 0 {
                0.0
            } else {
                row.wall_ns as f64 / row.rows as f64
            },
        );
        let _ = write!(
            out,
            "| {} | {ms:.1} | {} | {ns_per_row:.0} |",
            row.name, row.rows
        );
        if let Some(a) = &row.alloc {
            #[allow(
                clippy::cast_precision_loss,
                reason = "printed MiB; mantissa loss is below the table's decimals"
            )]
            let (alloc_mib, peak_mib) = (
                a.alloc_bytes as f64 / f64::from(1 << 20),
                a.peak_live_bytes as f64 / f64::from(1 << 20),
            );
            let _ = write!(
                out,
                " {} | {} | {alloc_mib:.1} | {peak_mib:.1} |",
                a.allocs, a.deallocs
            );
        } else if alloc {
            out.push_str(" — | — | — | — |");
        }
        out.push('\n');
    }
    out
}
