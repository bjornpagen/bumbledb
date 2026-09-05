//! Finite measurement input plan for L21. Execution is deferred.
//!
//! Verification: **NotRun**. Timing only on a quiet host after writer freeze.
//! Deterministic counters (visits, owners, roster, census) may run daily.

use super::workloads::{self, Cell};
use super::{Gate, Regime};

/// Named qualification hosts. A container on x86 does not qualify ARM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Host {
    AppleSilicon,
    GravitonArm64,
    X86Node,
}

impl Host {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AppleSilicon => "apple-silicon-macos-arm64",
            Self::GravitonArm64 => "graviton-linux-arm64",
            Self::X86Node => "linux-x86-64-node",
        }
    }

    #[must_use]
    pub const fn toolchain(self) -> &'static str {
        match self {
            Self::AppleSilicon => "nightly-2026-08-15, Apple clang/ld64, rustc host aarch64-apple-darwin",
            Self::GravitonArm64 => "nightly-2026-08-15, Amazon Linux 2023 aarch64, glibc",
            Self::X86Node => "nightly-2026-08-15, Node current LTS, linux-x64 native addon",
        }
    }
}

pub const HOSTS: [Host; 3] = [Host::AppleSilicon, Host::GravitonArm64, Host::X86Node];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warmth {
    Cold,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// LMDB default commit vs SQLite WAL + synchronous=FULL + fullfsync.
    MatchedDurable,
}

/// One executable cell for the night/measure scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptStep {
    pub id: &'static str,
    pub kind: StepKind,
    pub command: &'static str,
    pub hosts: &'static [Host],
    pub warmth: Option<Warmth>,
    pub durability: Durability,
    pub prerequisite: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Oracle / visit / roster / census. Safe as a daily counter.
    Semantic,
    /// Wall-clock. Final qualification only, serialized per host.
    Timing,
    /// Optional research (AEGIS). Absence is NotRun, never a fail.
    Optional,
}

/// Compact default night. Overlapping curves/heap/primerlane/adversarial
/// timing jobs are not in this table.
#[must_use]
pub fn script_steps() -> &'static [ScriptStep] {
    &[
        ScriptStep {
            id: "verify-oracle",
            kind: StepKind::Semantic,
            command: "bumbledb-bench verify --scale S --seed 1",
            hosts: &HOSTS,
            warmth: None,
            durability: Durability::MatchedDurable,
            prerequisite: None,
        },
        ScriptStep {
            id: "scorecard-semantic",
            kind: StepKind::Semantic,
            command: "bumbledb-bench app-perf --plan",
            hosts: &HOSTS,
            warmth: None,
            durability: Durability::MatchedDurable,
            prerequisite: None,
        },
        ScriptStep {
            id: "correspondence-oracles",
            kind: StepKind::Semantic,
            command: "cargo test -p bumbledb-bench correspondence -- --test-threads=1",
            hosts: &HOSTS,
            warmth: None,
            durability: Durability::MatchedDurable,
            prerequisite: Some("final qualification only; judge_final_state, not the planner; NotRun during fanout"),
        },
        ScriptStep {
            id: "three-way-conformance",
            kind: StepKind::Semantic,
            command: "cargo test -p bumbledb-bench three_way_conformance -- --ignored",
            hosts: &HOSTS,
            warmth: None,
            durability: Durability::MatchedDurable,
            prerequisite: Some("elan/lake on PATH; L19 lean.sh no longer invokes cargo tests"),
        },
        ScriptStep {
            id: "storage-census",
            kind: StepKind::Semantic,
            command: "bumbledb-bench storage --scales S,M --seed 1 --out $OUT/storage",
            hosts: &HOSTS,
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: None,
        },
        ScriptStep {
            id: "app-perf-warm",
            kind: StepKind::Timing,
            command: "bumbledb-bench app-perf --regimes warm --out $OUT/app-perf-warm",
            hosts: &HOSTS,
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: Some("quiet host; measure.sh lock; workers not churning"),
        },
        ScriptStep {
            id: "app-perf-cold",
            kind: StepKind::Timing,
            command: "bumbledb-bench app-perf --regimes cold-open,post-write --out $OUT/app-perf-cold",
            hosts: &HOSTS,
            warmth: Some(Warmth::Cold),
            durability: Durability::MatchedDurable,
            prerequisite: Some("quiet host; process restart between cold samples"),
        },
        ScriptStep {
            id: "app-perf-tenants",
            kind: StepKind::Timing,
            command: "bumbledb-bench app-perf --regimes tenant-churn --tenants 8 --out $OUT/app-perf-tenants",
            hosts: &HOSTS,
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: Some("fixed-worker runtime; do not measure parked sessions"),
        },
        ScriptStep {
            id: "large-populated",
            kind: StepKind::Timing,
            command: "bumbledb-bench app-perf --regimes large-result --scale L --out $OUT/large",
            hosts: &[Host::GravitonArm64],
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: Some(
                ">40 GiB allocated blocks (not a sparse map); 8 GiB cgroup memory.max; Linux",
            ),
        },
        ScriptStep {
            id: "hosted-decision",
            kind: StepKind::Timing,
            command: "not-run-here: L08/L10/L11 hosted driver over real S3",
            hosts: &[Host::X86Node, Host::GravitonArm64],
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: Some("real S3/IAM credentials; 1/2/4 writers; record request counts"),
        },
        ScriptStep {
            id: "hash-blake3",
            kind: StepKind::Semantic,
            command: "bumbledb-bench hash-probe --out $OUT/hash-probe",
            hosts: &HOSTS,
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: None,
        },
        ScriptStep {
            id: "hash-aegis-optional",
            kind: StepKind::Optional,
            command: "bumbledb-bench hash-probe --kat $KAT --out $OUT/hash-aegis",
            hosts: &HOSTS,
            warmth: Some(Warmth::Warm),
            durability: Durability::MatchedDurable,
            prerequisite: Some("optional AEGIS KAT file; missing = NotRun"),
        },
    ]
}

/// Hardware / credential holes. Missing is NotRun, not a fabricated pass.
#[must_use]
pub fn hardware_prerequisites() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "apple-silicon-macos-arm64",
            "local M-series Mac; pin nightly-2026-08-15; serialize via scripts/measure.sh",
        ),
        (
            "graviton-linux-arm64",
            "real Graviton instance (not an x86 container); AL2023 aarch64",
        ),
        (
            "linux-x86-64-node",
            "Node host with the packed native addon; event-loop delay column required",
        ),
        (
            "real-s3-iam",
            "hosted cells; emulator runs must be labeled emulator and cannot close G15",
        ),
        (
            "large-populated-disk",
            ">40 GiB allocated blocks + cgroup memory.max; sparse set_len is not this cell",
        ),
    ]
}

/// L21 expected semantic checks (layout/work + L20-owned C-D04/C-D19/C-G03).
/// Timing stays G15-only after freeze. Correspondence uses `judge_final_state`,
/// never the production planner.
#[must_use]
pub fn l21_semantic_checks() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        (
            "D04",
            "compiled-index-locality",
            "count source/group visits while unrelated groups scale; roster uses ProjectionId; compact u64 keys are 19 raw bytes",
        ),
        (
            "C-D04-collision-bytes",
            "exact-bytes-not-fingerprints",
            "unequal canonical encodings stay distinct facts; judge_final_state rejects same-key unequal payloads; neither merge nor wrong delete",
        ),
        (
            "C-D19-cancel",
            "rational-sum-cancel",
            "{1e16, 1, -1e16} exact sum bits from the rational oracle, not host f64 add",
        ),
        (
            "C-D19-mean-once",
            "mean-not-rounded-sum",
            "mean of two MAX_FINITE is MAX_FINITE; sum overflows",
        ),
        (
            "C-D19-merge-not-idemp",
            "merge_not_idempotent",
            "merge one finite partial with itself doubles total and count",
        ),
        (
            "C-G03-mutable-support",
            "untouched-statement-stable",
            "delta outside a statement's mutable consulted rels leaves that statement's judge_final_state citations unchanged",
        ),
        (
            "C-G03-add-wins",
            "changeset-add-wins",
            "same exact fact on both sides of one ChangeSet stays Add; finish/parse refuse a second action",
        ),
        (
            "C-G03-raw-commute",
            "admission-does-not-commute",
            "disjoint child adds sharing a capacity parent: set application commutes; admission of the union rejects",
        ),
        (
            "D08",
            "work-without-output-stops",
            "WorkContext work_units below exploration fails the query; visit count < relation cardinality",
        ),
        (
            "D09",
            "derived-scratch",
            "aggregate/negation/recursion accept Scratch; peak OwnerSnapshot.scratch_bytes bounded; no whole-image resurrection",
        ),
        (
            "D11",
            "pack-logical-order",
            "wide spill [10,20)+[0,15) → [0,20); compare bits with resident pack; no all_claims gather",
        ),
        (
            "D29",
            "tenant-ownership",
            "two owners: paused payload must not hold a runtime-global mutex; retained charge returns to baseline after close cycles",
        ),
        (
            "G05",
            "beyond-ram-and-32gib",
            "enforced resident budget + allocated-block >40 GiB; sparse maps refuse the large cell",
        ),
        (
            "G12",
            "work-queue-scratch",
            "OwnerSnapshot + queue wait + event-loop delay columns; cancellation joins",
        ),
        (
            "G15",
            "measured-envelope",
            "raw distributions and request counts; cold/warm separate; no best-median-only claim",
        ),
        (
            "REVIEW-001",
            "admitted-work",
            "harness work denominator is visitor/used counters, never elapsed>0 or file-exists",
        ),
    ]
}

/// Scorecard cells L21 should expect in evidence (ids only).
#[must_use]
pub fn l21_scorecard_ids() -> Vec<String> {
    workloads::scorecard().into_iter().map(|cell| cell.id).collect()
}

#[must_use]
pub fn cells_for(gate: Gate) -> Vec<Cell> {
    workloads::scorecard()
        .into_iter()
        .filter(|cell| cell.gate == gate)
        .collect()
}

#[must_use]
pub fn cells_for_regime(regime: Regime) -> Vec<Cell> {
    workloads::scorecard()
        .into_iter()
        .filter(|cell| cell.regime == regime)
        .collect()
}

/// Render the plan for `app-perf --plan` / bench-night --plan.
#[must_use]
pub fn render() -> String {
    let mut out = String::from("# L20 scorecard input plan\n\nVerification: NotRun\n\n");
    out.push_str("## Hosts\n\n");
    for host in HOSTS {
        out.push_str(&format!("- `{}` — {}\n", host.label(), host.toolchain()));
    }
    out.push_str("\n## Prerequisites (missing = NotRun)\n\n");
    for (id, note) in hardware_prerequisites() {
        out.push_str(&format!("- `{id}`: {note}\n"));
    }
    out.push_str("\n## Script steps\n\n| id | kind | warmth | command |\n|---|---|---|---|\n");
    for step in script_steps() {
        let warmth = step.warmth.map_or("-", |w| match w {
            Warmth::Cold => "cold",
            Warmth::Warm => "warm",
        });
        out.push_str(&format!(
            "| {} | {:?} | {warmth} | `{}` |\n",
            step.id, step.kind, step.command
        ));
    }
    out.push_str("\n## Scorecard cells\n\n");
    for cell in workloads::scorecard() {
        out.push_str(&format!(
            "- `{}` [{} / {}] oracle: {}\n",
            cell.id,
            cell.gate.label(),
            cell.regime.label(),
            cell.oracle
        ));
    }
    out.push_str("\n## L21 semantic checks\n\n");
    for (gate, id, expect) in l21_semantic_checks() {
        out.push_str(&format!("- {gate} `{id}`: {expect}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_covers_three_hosts_and_keeps_aegis_optional() {
        assert_eq!(HOSTS.len(), 3);
        assert!(
            script_steps()
                .iter()
                .any(|s| s.id == "hash-aegis-optional" && s.kind == StepKind::Optional)
        );
        assert!(
            script_steps()
                .iter()
                .any(|s| s.id == "hosted-decision" && s.prerequisite.is_some())
        );
        assert!(
            !script_steps()
                .iter()
                .any(|s| s.command.contains("primerlane") || s.command.contains("adversarial"))
        );
        assert!(
            script_steps().iter().any(|s| {
                s.id == "correspondence-oracles" && s.kind == StepKind::Semantic
            })
        );
        assert!(
            script_steps().iter().any(|s| {
                s.id == "three-way-conformance" && s.kind == StepKind::Semantic
            })
        );
        assert!(
            !script_steps()
                .iter()
                .any(|s| s.id == "three-way-conformance" && s.kind == StepKind::Timing)
        );
    }

    #[test]
    fn render_names_notrun_and_matched_durability() {
        let text = render();
        assert!(text.contains("NotRun"));
        assert!(text.contains("graviton-linux-arm64"));
        assert!(text.contains("REVIEW-001"));
        assert!(text.contains("C-D04-collision-bytes"));
        assert!(text.contains("C-D19-cancel"));
        assert!(text.contains("C-G03-raw-commute"));
        assert!(text.contains("judge_final_state") || text.contains("exact-bytes-not-fingerprints"));
    }
}
