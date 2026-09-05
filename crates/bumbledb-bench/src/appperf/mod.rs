//! The application performance scorecard (chapter 40; gates APP-FAST,
//! APP-MUTATE, APP-NUMERIC, APP-LARGE, APP-TENANTS, APP-TARGETS, APP-METHOD,
//! APP-MAGIC; audit PERF-003).
//!
//! Authored during F1; every measurement executes only in F3, serialized per
//! host. The module owns:
//!
//! - the frozen workload matrix — families × regimes × gates, with pass
//!   criteria fixed **before** replacement hot paths land ([`workloads`]);
//! - the cost-account vocabulary every cell reports ([`CostAccount`]) —
//!   copies, bytes, allocations, live resources, queue wait, conversion and
//!   event-loop delay are first-class columns, not prose;
//! - the native/bridge/Effect/whole-app layer decomposition and the wire
//!   format the Node-side emitters produce ([`layers`]);
//! - the PERF-003 hosted-commit accounting: requests/bytes/time per terminal
//!   outcome under 1/2/4/8 writers ([`hosted`]);
//! - executable core-side regime runners over the existing ledger corpus
//!   ([`runner`]) — cold open, warm read, post-write first read,
//!   large-result delivery split and local tenant churn.
//!
//! Method (APP-METHOD) is binding: verified results before timing, binding to
//! source/binary/corpus digests, interleaved A/B with baseline controls, raw
//! distributions retained, no-sync reported separately and never as durable
//! evidence, and no invented ratios from timeouts.

pub mod hosted;
pub mod layers;
pub mod runner;
#[cfg(test)]
mod tests;
pub mod workloads;

/// The chapter 40 §8 gate families this module owns evidence for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    AppFast,
    AppMutate,
    AppNumeric,
    AppLarge,
    AppTenants,
    AppTargets,
    AppMethod,
    AppMagic,
}

pub const GATES: [Gate; 8] = [
    Gate::AppFast,
    Gate::AppMutate,
    Gate::AppNumeric,
    Gate::AppLarge,
    Gate::AppTenants,
    Gate::AppTargets,
    Gate::AppMethod,
    Gate::AppMagic,
];

impl Gate {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AppFast => "APP-FAST",
            Self::AppMutate => "APP-MUTATE",
            Self::AppNumeric => "APP-NUMERIC",
            Self::AppLarge => "APP-LARGE",
            Self::AppTenants => "APP-TENANTS",
            Self::AppTargets => "APP-TARGETS",
            Self::AppMethod => "APP-METHOD",
            Self::AppMagic => "APP-MAGIC",
        }
    }
}

/// The measured regimes. Every workload family names which of these it must
/// run under; [`workloads::scorecard`] refuses gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Warm prepared reuse with rotated parameters.
    Warm,
    /// Cold process/first execution after open — activation cost included.
    ColdOpen,
    /// First read after insert/replace/delete: rebuild/copy bytes charged.
    PostWrite,
    /// Selective keyed/direct probes (the image-free entity lookup path).
    Selective,
    /// Large result sets: execution versus delivery split, bounded pages.
    LargeResult,
    /// Many small tenants: activation churn, skewed hot set, eviction release.
    TenantChurn,
    /// Hosted commit under 1/2/4/8 writers, loss injection, checkpoint overlap.
    HostedContention,
    /// Checkpoint/GC/maintenance running beside a live workload.
    Maintenance,
}

pub const REGIMES: [Regime; 8] = [
    Regime::Warm,
    Regime::ColdOpen,
    Regime::PostWrite,
    Regime::Selective,
    Regime::LargeResult,
    Regime::TenantChurn,
    Regime::HostedContention,
    Regime::Maintenance,
];

impl Regime {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::ColdOpen => "cold-open",
            Self::PostWrite => "post-write",
            Self::Selective => "selective",
            Self::LargeResult => "large-result",
            Self::TenantChurn => "tenant-churn",
            Self::HostedContention => "hosted-contention",
            Self::Maintenance => "maintenance",
        }
    }
}

/// The physical cost columns every cell reports beside its latency stats.
/// Missing instrumentation is `None` — a hole in the table, never a zero
/// pretending to be a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CostAccount {
    /// Bytes decoded from storage into values.
    pub bytes_decoded: Option<u64>,
    /// Bytes copied between owned buffers (marshalling, image append, result
    /// publication).
    pub bytes_copied: Option<u64>,
    /// Allocations and allocated bytes inside the measured window.
    pub allocs: Option<u64>,
    pub alloc_bytes: Option<u64>,
    /// Peak resident set over the cell, bytes.
    pub peak_rss: Option<u64>,
    /// Live native resources at cell end (owners, handles, temp LMDBs).
    pub live_resources: Option<u64>,
    /// Queue wait before work started, ns (bridge/host cells).
    pub queue_wait_ns: Option<u64>,
    /// Native→JS conversion time, ns (bridge/Effect/app cells).
    pub conversion_ns: Option<u64>,
    /// Max observed event-loop delay during the cell, ns (Node cells).
    pub event_loop_delay_ns: Option<u64>,
    /// Remote publication accounting (hosted cells): requests, request +
    /// response bytes, retries.
    pub requests: Option<u64>,
    pub request_bytes: Option<u64>,
    pub response_bytes: Option<u64>,
    pub retries: Option<u64>,
    /// On-disk consumption after the cell (store + temporary scratch).
    pub disk_bytes: Option<u64>,
    pub scratch_bytes: Option<u64>,
}

/// Preparation / execution / delivery attribution for one operation. Segments
/// may overlap wall time in some modes; `end_to_end_ns` is always the
/// measured critical path and never the sum of segments (PERF-003's
/// overlap warning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhaseSplit {
    pub prepare_ns: Option<u64>,
    pub execute_ns: Option<u64>,
    pub deliver_ns: Option<u64>,
    pub end_to_end_ns: u64,
}
