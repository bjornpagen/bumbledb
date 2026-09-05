//! The frozen application scorecard (chapter 40 §6): representative schemas,
//! populations, parameter schedules and pass criteria fixed **before** the
//! first replacement hot path lands. These are benchmark inputs, not tenant
//! product caps. Fixtures use actual successor encoded bytes and 16-byte
//! identity widths — an all-u64 fixture that omits the new layout's cost is
//! not a matched workload.

use super::{Gate, Regime};

/// The six application families of the scorecard table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Keyed profile/attempt lookup; assignment → attempt → mastery joins;
    /// small per-course count/sum/mean; insert then immediately refreshed
    /// results.
    StudentLearning,
    /// Point membership, overlap/exclusion, coverage, packed availability;
    /// integer time plus separate continuous F64 range fixtures; booking
    /// accept/reject and consequence reads.
    Scheduling,
    /// Neighborhood, two-hop, mutual edges, cyclic/triangle joins, bounded
    /// linear reachability; varied selectivity and frontier width/depth.
    Graph,
    /// Small insert, keyed replacement, deletion, batch write,
    /// read/modify/write condition, no-change and rejected update;
    /// app-owned 16-byte IDs chosen before sealing.
    ObjectCrud,
    /// Repeated short labels and unique text churn; inline payload sizes,
    /// exact key probes, long-key collisions/range fallback,
    /// delete/export/reopen.
    TextRich,
    /// Many unopened/idle databases, skewed hot users, sequential activation,
    /// prepared reuse, pressure eviction with actual native release;
    /// concurrent small queries beside one large query.
    TenantFleet,
}

pub const FAMILIES: [Family; 6] = [
    Family::StudentLearning,
    Family::Scheduling,
    Family::Graph,
    Family::ObjectCrud,
    Family::TextRich,
    Family::TenantFleet,
];

impl Family {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StudentLearning => "student-learning",
            Self::Scheduling => "scheduling",
            Self::Graph => "graph",
            Self::ObjectCrud => "object-crud",
            Self::TextRich => "text-rich",
            Self::TenantFleet => "tenant-fleet",
        }
    }
}

/// One scorecard cell: a family under a regime, owned by a gate, with its
/// verification and reporting obligations spelled out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub id: String,
    pub family: Family,
    pub regime: Regime,
    pub gate: Gate,
    /// Every cell verifies complete result sets/errors/final states against
    /// the independent oracle before timing (APP-METHOD rule 1).
    pub oracle: &'static str,
    /// What the cell must report beyond p50/p95/p99 (chapter 40 §6's
    /// per-cell reporting law is global; this names cell-specific extras).
    pub extra_outputs: &'static [&'static str],
    /// Pass criterion, frozen now. `report-only` cells feed the P00 cost
    /// decision without a numeric gate of their own.
    pub criterion: &'static str,
}

/// Regimes every family must cover (chapter 40 §6: warm reuse, first
/// execution after open, first read after mutation, memory-pressure /
/// displacement variants).
pub const CORE_REGIMES: [Regime; 4] = [
    Regime::Warm,
    Regime::ColdOpen,
    Regime::PostWrite,
    Regime::LargeResult,
];

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

/// The complete frozen matrix. Structure tests assert: every family covers
/// [`CORE_REGIMES`]; every gate has at least one cell; selective probes,
/// tenant churn, hosted contention and maintenance each exist; numeric cells
/// carry exact-bits oracles.
#[must_use]
#[expect(clippy::too_many_lines, reason = "the scorecard is one frozen table")]
pub fn scorecard() -> Vec<Cell> {
    let mut cells = Vec::new();
    // Core regimes for every family.
    for family in FAMILIES {
        let gate = match family {
            Family::ObjectCrud => Gate::AppMutate,
            Family::TenantFleet => Gate::AppTenants,
            _ => Gate::AppFast,
        };
        cells.push(cell(
            family,
            Regime::Warm,
            gate,
            "naive/SQLite verified result sets before timing",
            &[
                "prepared-reuse hit rate",
                "allocations",
                "retained capacity",
            ],
            "retains the agreed warm fast-path budget on direct probe, preferred Free Join \
             and forced cursor paths — not victory over a deliberately slow fallback",
        ));
        cells.push(cell(
            family,
            Regime::ColdOpen,
            gate,
            "same verified results as warm",
            &[
                "open time",
                "first-execution time",
                "bytes decoded",
                "peak RSS",
            ],
            "activation plus first read fits the per-user activation budget; a per-user \
             database is not evaluated only after all users are warm",
        ));
        cells.push(cell(
            family,
            Regime::PostWrite,
            Gate::AppMutate,
            "post-state comparison against the independent write model",
            &[
                "first-read rebuild bytes",
                "copy bytes",
                "ID-allocation work (must be absent)",
            ],
            "first read after insert/replace/delete reports rebuild/copy bytes separately; \
             no persisted ID-allocation work remains",
        ));
        cells.push(cell(
            family,
            Regime::LargeResult,
            Gate::AppFast,
            "streaming exact-check oracle",
            &[
                "execute vs deliver split",
                "page pull cadence",
                "scratch bytes",
            ],
            "complete owned results only; delivery is bounded pages, cap failure keeps the \
             sealed backing available",
        ));
    }
    // Selective probes (APP-FAST's direct-probe leg).
    for family in [
        Family::StudentLearning,
        Family::ObjectCrud,
        Family::TextRich,
    ] {
        cells.push(cell(
            family,
            Regime::Selective,
            Gate::AppFast,
            "keyed oracle rows",
            &[
                "probe path taken (classifier evidence)",
                "per-probe allocations",
            ],
            "a direct typed AST reaches the image-free lookup path without a generic join \
             setup toll",
        ));
    }
    // Numeric cells (APP-NUMERIC): exact float sum/mean and dense intervals.
    cells.push(cell(
        Family::StudentLearning,
        Regime::Warm,
        Gate::AppNumeric,
        "independent bit/rational reduction fixtures (never the production accumulator)",
        &[
            "exact-bits agreement",
            "accumulator state bytes charged",
            "single+many groups",
        ],
        "sum and mean alone/together, single and high-cardinality groups, distinct-witness \
         and dedup-required inputs: bits agree and full numerical state is charged",
    ));
    cells.push(cell(
        Family::Scheduling,
        Regime::Warm,
        Gate::AppNumeric,
        "dense float-interval endpoint oracle",
        &[
            "interval kernel path",
            "length-overflow and unbounded-measure errors",
        ],
        "dense float interval kernels agree with the endpoint oracle including adjacency, \
         rays and gap cases; errors are distinct, not NaN results",
    ));
    // Beyond-memory (APP-LARGE) — the fixtures live in crate::largefix.
    cells.push(cell(
        Family::Graph,
        Regime::LargeResult,
        Gate::AppLarge,
        "streaming chunk-checksum oracle (crate::largefix)",
        &[
            "forced-spill transitions",
            "bounded RAM/scratch",
            "fallback work reported",
        ],
        ">RAM and >40 GiB populated execution stays correct through the same public \
         semantics; report fallback work and usability, not merely a successful open",
    ));
    // Tenant fleet (APP-TENANTS).
    cells.push(cell(
        Family::TenantFleet,
        Regime::TenantChurn,
        Gate::AppTenants,
        "per-tenant post-state checks",
        &[
            "activation p99",
            "native bytes released on eviction",
            "filesystem space",
            "queue/event-loop tail under a noisy neighbor",
        ],
        "per-user activation/churn and mixed-size noisy neighbors hold the queue and \
         event-loop tail budgets; eviction actually releases native bytes",
    ));
    // Hosted commit (PERF-003 / APP-TENANTS + APP-METHOD shared evidence).
    cells.push(cell(
        Family::ObjectCrud,
        Regime::HostedContention,
        Gate::AppTenants,
        "independent history model receipts (P04/P11 traces)",
        &[
            "requests per terminal outcome",
            "bytes per terminal outcome",
            "retry/loss recovery term",
            "queue/judgment/publication/catch-up/commit split",
        ],
        "1/2/4/8 writers on one history vs independent histories, same-key vs disjoint-key: \
         complete named-decision cost is counted, not one winning PUT",
    ));
    // Maintenance overlap.
    cells.push(cell(
        Family::TenantFleet,
        Regime::Maintenance,
        Gate::AppTenants,
        "post-maintenance state equality",
        &[
            "checkpoint completion time under writes",
            "bytes reread",
            "stall distribution",
        ],
        "checkpoint/GC beside a live workload makes progress without relation-sized stalls \
         or lost maintenance progress",
    ));
    // Targets and method are cross-cutting cells.
    cells.push(cell(
        Family::ObjectCrud,
        Regime::Warm,
        Gate::AppTargets,
        "identical verified answers on every named target",
        &[
            "target-local calibration",
            "unsupported host-resource envelope",
        ],
        "fresh artifacts on named Apple Silicon, Graviton ARM and x86-64 Node targets give \
         identical answers; M2 constants are not inherited",
    ));
    cells.push(cell(
        Family::ObjectCrud,
        Regime::Warm,
        Gate::AppMethod,
        "baseline/baseline control comparisons",
        &[
            "interleaved A/B arms",
            "raw distributions",
            "ambient/clock flags",
            "work denominators",
        ],
        "binary/data-bound verification, baseline controls, interleaving, real work counts \
         and truthful unrun/drift/timeout reporting",
    ));
    cells.push(cell(
        Family::ObjectCrud,
        Regime::Warm,
        Gate::AppMagic,
        "structural invariant tests per constant (docs/perf/magic-number-review.md)",
        &[
            "constant class",
            "owner",
            "sweep result",
            "fallback correctness",
        ],
        "every high-impact constant classified and owned; hardware crossovers measured; \
         no public knob explosion",
    ));
    cells
}
