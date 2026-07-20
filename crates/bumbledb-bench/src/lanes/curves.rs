//! The curves lane: scale-curve runner plus the cold/warm/memoized
//! warmth panel — REPORT-class ([`crate::lanes`] carries the charter).
//! This packet lands the report schema and the refusal shell; the lane
//! body lands in MET-4-curves-lane, which owns this file afterward and
//! keeps the shape-pin test in sync with any schema extension.

use std::fmt::Write as _;

use crate::harness::Stats;
use crate::report::Provenance;

/// The whole curves report, plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvesReport {
    pub provenance: Provenance,
    pub seed: u64,
    pub samples: u32,
    pub cap_ms: u64,
    pub families: Vec<FamilyCurve>,
}

/// One family's curve across the scale ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyCurve {
    pub name: &'static str,
    pub world: &'static str,
    pub rows: Vec<CurvePoint>,
    pub warmth: Option<Warmth>,
}

/// One (family, scale) point. Absent stats mean the engine never
/// produced a timing for the point (a cap event says why).
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    pub scale: &'static str,
    pub facts: u64,
    pub answers: u64,
    pub ours: Option<Stats>,
    pub theirs: Option<Stats>,
    pub theirs_hand: Option<Stats>,
    pub cap: Option<CapEvent>,
}

/// Where the DNF cap fired: `"gate"` (the oracle pass), `"timing"`
/// (the canonical twin), or `"hand"` (the hand-tuned twin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapEvent {
    pub at: &'static str,
}

/// The cold/warm/memoized panel, both engines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Warmth {
    pub ours_cold: Stats,
    pub ours_warm: Stats,
    pub ours_memoized: Stats,
    pub theirs_cold: Stats,
    pub theirs_warm: Stats,
    pub theirs_memoized: Stats,
}

fn push_point(out: &mut String, point: &CurvePoint) {
    let _ = write!(
        out,
        "{{\"scale\":\"{}\",\"facts\":{},\"answers\":{},\"ours\":",
        point.scale, point.facts, point.answers
    );
    super::push_opt_stats(out, point.ours.as_ref());
    out.push_str(",\"theirs\":");
    super::push_opt_stats(out, point.theirs.as_ref());
    out.push_str(",\"theirs_hand\":");
    super::push_opt_stats(out, point.theirs_hand.as_ref());
    out.push_str(",\"cap\":");
    match point.cap {
        Some(cap) => {
            let _ = write!(out, "{{\"at\":\"{}\"}}", cap.at);
        }
        None => out.push_str("null"),
    }
    out.push('}');
}

fn push_warmth(out: &mut String, warmth: Option<&Warmth>) {
    out.push_str(",\"warmth\":");
    let Some(w) = warmth else {
        out.push_str("null");
        return;
    };
    out.push_str("{\"ours_cold\":");
    super::push_stats(out, &w.ours_cold);
    out.push_str(",\"ours_warm\":");
    super::push_stats(out, &w.ours_warm);
    out.push_str(",\"ours_memoized\":");
    super::push_stats(out, &w.ours_memoized);
    out.push_str(",\"theirs_cold\":");
    super::push_stats(out, &w.theirs_cold);
    out.push_str(",\"theirs_warm\":");
    super::push_stats(out, &w.theirs_warm);
    out.push_str(",\"theirs_memoized\":");
    super::push_stats(out, &w.theirs_memoized);
    out.push('}');
}

/// The machine-consumable curves artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &CurvesReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"seed\":{},\"samples\":{},\"cap_ms\":{},\"families\":[",
        report.seed, report.samples, report.cap_ms
    );
    for (index, family) in report.families.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"name\":\"{}\",\"world\":\"{}\",\"rows\":[",
            family.name, family.world
        );
        for (row_index, point) in family.rows.iter().enumerate() {
            if row_index > 0 {
                out.push(',');
            }
            push_point(&mut out, point);
        }
        out.push(']');
        push_warmth(&mut out, family.warmth.as_ref());
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// The curves lane entry point.
///
/// # Errors
///
/// Always, for now: the lane body lands in MET-4-curves-lane; until
/// then this is the typed refusal naming that packet.
pub fn run(_args: &crate::cli::CurvesArgs) -> Result<i32, String> {
    Err("the curves lane lands in MET-4-curves-lane; this subcommand refuses until then"
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Value;

    fn provenance() -> Provenance {
        Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
        }
    }

    fn stats(base: u64) -> Stats {
        Stats {
            min: base,
            p50: base + 1,
            p90: base + 2,
            p95: base + 3,
            p99: base + 4,
            max: base + 5,
            mean_ns: base + 2,
        }
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = CurvesReport {
            provenance: provenance(),
            seed: 3,
            samples: 16,
            cap_ms: 5000,
            families: vec![FamilyCurve {
                name: "triangle",
                world: "graph",
                rows: vec![
                    CurvePoint {
                        scale: "S",
                        facts: 100_000,
                        answers: 42,
                        ours: Some(stats(100)),
                        theirs: Some(stats(200)),
                        theirs_hand: None,
                        cap: None,
                    },
                    CurvePoint {
                        scale: "M",
                        facts: 1_000_000,
                        answers: 420,
                        ours: Some(stats(300)),
                        theirs: None,
                        theirs_hand: None,
                        cap: Some(CapEvent { at: "timing" }),
                    },
                ],
                warmth: Some(Warmth {
                    ours_cold: stats(10),
                    ours_warm: stats(20),
                    ours_memoized: stats(30),
                    theirs_cold: stats(40),
                    theirs_warm: stats(50),
                    theirs_memoized: stats(60),
                }),
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        assert_eq!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("timestamp"))
                .and_then(Value::as_str),
            Some("2026-07-19T00:00:00Z")
        );
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(3.0));
        assert_eq!(parsed.get("samples").and_then(Value::as_f64), Some(16.0));
        assert_eq!(parsed.get("cap_ms").and_then(Value::as_f64), Some(5000.0));
        let families = parsed
            .get("families")
            .and_then(Value::as_arr)
            .expect("families");
        assert_eq!(families.len(), 1);
        assert_eq!(
            families[0].get("name").and_then(Value::as_str),
            Some("triangle")
        );
        assert_eq!(
            families[0].get("world").and_then(Value::as_str),
            Some("graph")
        );
        let rows = families[0].get("rows").and_then(Value::as_arr).expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("scale").and_then(Value::as_str), Some("S"));
        assert_eq!(rows[0].get("facts").and_then(Value::as_f64), Some(100_000.0));
        assert_eq!(rows[0].get("answers").and_then(Value::as_f64), Some(42.0));
        let ours = rows[0].get("ours").expect("ours");
        assert_eq!(ours.get("p50").and_then(Value::as_f64), Some(101.0));
        let theirs = rows[0].get("theirs").expect("theirs");
        assert_eq!(theirs.get("max").and_then(Value::as_f64), Some(205.0));
        assert_eq!(rows[0].get("theirs_hand"), Some(&Value::Null));
        assert_eq!(rows[0].get("cap"), Some(&Value::Null));
        // The capped point: theirs is null and the cap event says where.
        assert_eq!(rows[1].get("theirs"), Some(&Value::Null));
        assert_eq!(
            rows[1]
                .get("cap")
                .and_then(|c| c.get("at"))
                .and_then(Value::as_str),
            Some("timing")
        );
        let warmth = families[0].get("warmth").expect("warmth");
        assert_eq!(
            warmth
                .get("ours_cold")
                .and_then(|s| s.get("min"))
                .and_then(Value::as_f64),
            Some(10.0)
        );
        assert_eq!(
            warmth
                .get("theirs_memoized")
                .and_then(|s| s.get("mean_ns"))
                .and_then(Value::as_f64),
            Some(62.0)
        );
    }

    #[test]
    fn run_refuses_naming_the_landing_packet() {
        let err = run(&crate::cli::CurvesArgs::default()).unwrap_err();
        assert!(err.contains("MET-4"), "{err}");
    }
}
