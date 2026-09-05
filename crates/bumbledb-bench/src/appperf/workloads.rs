//! Compact chapter-40 scorecard. Six families, no cartesian ritual.
//!
//! Corpora: ledger (native-ledger-shaped), calendar/temporal (intervals),
//! rings/points (joins/keys). Existing verified oracles stay; this table
//! names which cell they serve.

use super::{Gate, Regime};

/// The six qualification families (chapter 40).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Exact key hit/miss, selective Free Join, fanout/existence, anti-join,
    /// named-stage reuse, small positive recursion, migration field arithmetic.
    ResidentRead,
    /// Insert/replace/delete/no-change/rejection then the prepared read.
    MutationRead,
    /// Exact sum/mean, intervals, Pack. Bits before timing.
    NumericInterval,
    /// >RAM text/derived/results and actual >32 GiB populated data.
    Nonresident,
    /// Many idle tenants, open storms, LRU, retained snapshots, noisy neighbor.
    TenantLifecycle,
    /// 1/2/4 contenders, full named-decision cost, checkpoint under writes.
    HostedLifecycle,
}

pub const FAMILIES: [Family; 6] = [
    Family::ResidentRead,
    Family::MutationRead,
    Family::NumericInterval,
    Family::Nonresident,
    Family::TenantLifecycle,
    Family::HostedLifecycle,
];

impl Family {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ResidentRead => "resident-read",
            Self::MutationRead => "mutation-read",
            Self::NumericInterval => "numeric-interval",
            Self::Nonresident => "nonresident",
            Self::TenantLifecycle => "tenant-lifecycle",
            Self::HostedLifecycle => "hosted-lifecycle",
        }
    }

    #[must_use]
    pub const fn corpus(self) -> &'static str {
        match self {
            Self::ResidentRead => "ledger + rings/points (verified families)",
            Self::MutationRead => "ledger POSTING_TAG delta + calendar booking",
            Self::NumericInterval => "corpus-float + temporal Pack",
            Self::Nonresident => "largefix populated + enforced RAM",
            Self::TenantLifecycle => "tiny ledger × N tenants",
            Self::HostedLifecycle => "object-crud over successor log / real S3",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub id: String,
    pub family: Family,
    pub regime: Regime,
    pub gate: Gate,
    pub oracle: &'static str,
    pub extra_outputs: &'static [&'static str],
    pub criterion: &'static str,
}

fn cell(
    family: Family,
    regime: Regime,
    gate: Gate,
    oracle: &'static str,
    extra_outputs: &'static [&'static str],
    criterion: &'static str,
) -> Cell {
    Cell {
        id: format!("{}/{}", family.label(), regime.label()),
        family,
        regime,
        gate,
        oracle,
        extra_outputs,
        criterion,
    }
}

/// Frozen compact matrix. Structure tests refuse missing families/gates
/// and refuse a cartesian explosion (one cell per needed regime only).
#[must_use]
pub fn scorecard() -> Vec<Cell> {
    vec![
        cell(
            Family::ResidentRead,
            Regime::Warm,
            Gate::AppFast,
            "naive/SQLite verified result sets before timing",
            &[
                "source_visits",
                "group_visits",
                "prepared-reuse",
                "owner snapshot",
            ],
            "warm keyed probe and selective Free Join keep visit counts local; \
             not victory over a deliberately slow fallback",
        ),
        cell(
            Family::ResidentRead,
            Regime::Selective,
            Gate::AppFast,
            "keyed oracle rows + CompiledTheory::consume_visits",
            &["probe path", "source_visits", "index roster"],
            "direct typed AST reaches the image-free lookup; existence-only \
             suffixes stop after the first sufficient witness",
        ),
        cell(
            Family::ResidentRead,
            Regime::ColdOpen,
            Gate::AppFast,
            "same verified results as warm",
            &["open work_units", "bytes decoded", "peak RSS"],
            "activation plus first read is counted; a per-user database is not \
             evaluated only after all users are warm",
        ),
        cell(
            Family::MutationRead,
            Regime::PostWrite,
            Gate::AppMutate,
            "post-state comparison against the independent write model",
            &[
                "first-read rebuild bytes",
                "copy bytes",
                "invalidated relation versions",
                "judge groups/rows",
            ],
            "first read after insert/replace/delete reports rebuild/copy bytes; \
             no persisted ID-allocation work remains",
        ),
        cell(
            Family::NumericInterval,
            Regime::Warm,
            Gate::AppNumeric,
            "independent bit/rational reduction fixtures (never the production accumulator)",
            &[
                "exact-bits agreement",
                "accumulator owner bytes",
                "single+many groups",
            ],
            "sum and mean alone/together, distinct-witness and dedup-required: \
             bits agree before any timing",
        ),
        cell(
            Family::NumericInterval,
            Regime::Selective,
            Gate::AppNumeric,
            "dense float-interval endpoint oracle + temporal Pack naive",
            &["interval kernel path", "Pack logical merge", "length errors"],
            "dense intervals and Pack agree with the endpoint/sweep oracle; \
             D11 order is logical, not insertion-token",
        ),
        cell(
            Family::Nonresident,
            Regime::LargeResult,
            Gate::AppLarge,
            "streaming chunk-checksum oracle (crate::largefix)",
            &[
                "forced-spill transitions",
                "allocated_disk_bytes",
                "scratch owner",
                "virtual_map_bytes (separate)",
            ],
            ">RAM and >40 GiB populated execution stays correct; a huge sparse \
             map with tiny contents is not this cell",
        ),
        cell(
            Family::TenantLifecycle,
            Regime::TenantChurn,
            Gate::AppTenants,
            "per-tenant post-state checks",
            &[
                "native bytes released",
                "owner snapshot baseline",
                "queue/event-loop tail",
                "live_transactions",
            ],
            "eviction releases native bytes; no runtime-global mutex covers \
             scratch/conversion; fixed workers, not parked sessions",
        ),
        cell(
            Family::TenantLifecycle,
            Regime::Maintenance,
            Gate::AppTenants,
            "post-maintenance state equality",
            &["checkpoint completion", "bytes reread", "request counts"],
            "checkpoint/GC beside writes makes progress without relation-sized \
             stalls or lost maintenance progress",
        ),
        cell(
            Family::HostedLifecycle,
            Regime::HostedContention,
            Gate::AppTenants,
            "independent history model receipts",
            &[
                "requests per terminal outcome",
                "bytes per terminal outcome",
                "retries",
                "1/2/4 writers",
            ],
            "complete named-decision cost is counted, not one winning PUT; \
             missing S3 credentials = NotRun",
        ),
        cell(
            Family::ResidentRead,
            Regime::Warm,
            Gate::AppTargets,
            "identical verified answers on every named host",
            &["host-local calibration", "unsupported envelope"],
            "Apple Silicon, real Graviton ARM64 and x86 Node give identical \
             answers; M2 constants are not inherited",
        ),
        cell(
            Family::ResidentRead,
            Regime::Warm,
            Gate::AppMethod,
            "baseline/baseline control comparisons",
            &[
                "raw distributions",
                "admitted work denominators",
                "cold/warm split",
                "interleaved A/B",
            ],
            "record raw samples and request counts, not just best times; \
             timeouts stay timeouts",
        ),
        cell(
            Family::ResidentRead,
            Regime::Warm,
            Gate::AppMagic,
            "structural tests per constant (appperf::constants)",
            &["constant class", "owner lane", "crossover host"],
            "every high-impact constant is a representation bound, host policy, \
             or measured crossover; AEGIS stays optional",
        ),
    ]
}
