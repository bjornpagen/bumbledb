//! PERF-003: hosted commit cost is the complete named-decision path, not one
//! winning PUT.
//!
//! Accounting identity (audit 40-performance.md):
//! `commit latency = queue + record/encode + local durable preparation +
//! judgment/apply + object round trips + settlement`, plus a variable
//! recovery/retry term. Segments can overlap in some modes, so
//! [`CommitCostSample::end_to_end_ns`] is measured on its own and the
//! summariser never reports a sum of segments as the latency.
//!
//! The successor history machine is P04/P05 wave-B work; this module is the
//! accounting/schedule side, complete now, with the driver seam
//! ([`HostedDriver`]) the F3 wiring implements over the real log. Nothing
//! here fabricates a hosted result: without a driver there is no report.

use super::PhaseSplit;

/// Terminal outcomes per C06: decided (accepted), decided (rejected with
/// receipts), decided (no-change stamp), or outcome-unknown. All four are
/// terminal for cost purposes; unknown carries its resolution cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutcome {
    Accepted,
    Rejected,
    NoChange,
    Unknown,
}

pub const OUTCOMES: [TerminalOutcome; 4] = [
    TerminalOutcome::Accepted,
    TerminalOutcome::Rejected,
    TerminalOutcome::NoChange,
    TerminalOutcome::Unknown,
];

impl TerminalOutcome {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::NoChange => "no-change",
            Self::Unknown => "unknown",
        }
    }
}

/// One submitted command's complete cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitCostSample {
    pub outcome: TerminalOutcome,
    pub phases: PhaseSplit,
    /// Segment attribution beyond prepare/execute/deliver: queue wait,
    /// local durable preparation, judgment/apply, remote publication,
    /// catch-up and settlement — each `None` when uninstrumented.
    pub queue_ns: Option<u64>,
    pub local_durable_ns: Option<u64>,
    pub judgment_apply_ns: Option<u64>,
    pub publication_ns: Option<u64>,
    pub catch_up_ns: Option<u64>,
    pub settlement_ns: Option<u64>,
    /// Object-store accounting: every request on the path to the terminal
    /// outcome, including losing attempts, retries and resolution reads.
    pub requests: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub retries: u64,
}

/// Requests/bytes/time per terminal outcome — the PERF-003 deliverable shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutcomeCost {
    pub commands: u64,
    pub total_ns: u64,
    pub total_requests: u64,
    pub total_request_bytes: u64,
    pub total_response_bytes: u64,
    pub total_retries: u64,
}

impl OutcomeCost {
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "reporting arithmetic")]
    pub fn requests_per_command(&self) -> f64 {
        if self.commands == 0 {
            0.0
        } else {
            self.total_requests as f64 / self.commands as f64
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "reporting arithmetic")]
    pub fn bytes_per_command(&self) -> f64 {
        if self.commands == 0 {
            0.0
        } else {
            (self.total_request_bytes + self.total_response_bytes) as f64 / self.commands as f64
        }
    }
}

/// Aggregate samples per terminal outcome. Indexed by [`OUTCOMES`] order.
///
/// # Panics
/// On a sample whose outcome is not in [`OUTCOMES`] (a fixture invariant).
#[must_use]
pub fn per_terminal_summary(samples: &[CommitCostSample]) -> [OutcomeCost; 4] {
    let mut out = [OutcomeCost::default(); 4];
    for sample in samples {
        let index = OUTCOMES
            .iter()
            .position(|&o| o == sample.outcome)
            .expect("OUTCOMES is total");
        let cell = &mut out[index];
        cell.commands += 1;
        cell.total_ns += sample.phases.end_to_end_ns;
        cell.total_requests += sample.requests;
        cell.total_request_bytes += sample.request_bytes;
        cell.total_response_bytes += sample.response_bytes;
        cell.total_retries += sample.retries;
    }
    out
}

/// Contention key modes: same-key writers collide on judgment; disjoint-key
/// writers collide only on the HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    SameKey,
    DisjointKeys,
}

/// Whether writers share one history or write genuinely independent
/// histories (the audit's "one braid versus independent braids" question,
/// restated for the successor's single-HEAD histories).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryMode {
    SharedHistory,
    IndependentHistories,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentionCell {
    pub writers: u32,
    pub key_mode: KeyMode,
    pub history_mode: HistoryMode,
    pub checkpoint_active: bool,
    /// Inject response loss on a fixed fraction of publications so the
    /// recovery/retry term is measured, not modeled.
    pub loss_injection: bool,
}

pub const WRITER_COUNTS: [u32; 4] = [1, 2, 4, 8];

/// The full contention schedule: writers × key mode × history mode ×
/// checkpoint × loss. Independent-histories cells skip the same-key mode
/// (distinct histories cannot share a key domain).
#[must_use]
pub fn contention_schedule() -> Vec<ContentionCell> {
    let mut cells = Vec::new();
    for &writers in &WRITER_COUNTS {
        for key_mode in [KeyMode::SameKey, KeyMode::DisjointKeys] {
            for history_mode in [
                HistoryMode::SharedHistory,
                HistoryMode::IndependentHistories,
            ] {
                if history_mode == HistoryMode::IndependentHistories && key_mode == KeyMode::SameKey
                {
                    continue;
                }
                for checkpoint_active in [false, true] {
                    for loss_injection in [false, true] {
                        cells.push(ContentionCell {
                            writers,
                            key_mode,
                            history_mode,
                            checkpoint_active,
                            loss_injection,
                        });
                    }
                }
            }
        }
    }
    cells
}

/// The seam the F3 wiring implements over the real hosted log (C06/C07/C08).
/// One call runs one contention cell and returns every command's complete
/// cost sample. No emulator shortcut: `backend` names what actually served
/// the requests, and emulator green is recorded as emulator green.
pub trait HostedDriver {
    /// # Errors
    fn run_cell(
        &mut self,
        cell: ContentionCell,
        commands_per_writer: u32,
    ) -> Result<Vec<CommitCostSample>, String>;

    fn backend(&self) -> &'static str;
}

/// Sanity law for any driver result: every sample's segment attribution,
/// where present, stays at or below the measured end-to-end critical path
/// per segment (segments may overlap each other, but no single segment can
/// exceed the whole), and unknown outcomes still carry their request cost.
///
/// # Errors
pub fn check_samples(samples: &[CommitCostSample]) -> Result<(), String> {
    for (index, sample) in samples.iter().enumerate() {
        let end = sample.phases.end_to_end_ns;
        for (name, segment) in [
            ("queue", sample.queue_ns),
            ("local-durable", sample.local_durable_ns),
            ("judgment-apply", sample.judgment_apply_ns),
            ("publication", sample.publication_ns),
            ("catch-up", sample.catch_up_ns),
            ("settlement", sample.settlement_ns),
        ] {
            if let Some(segment) = segment
                && segment > end
            {
                return Err(format!(
                    "sample {index}: segment {name} ({segment} ns) exceeds the end-to-end \
                     critical path ({end} ns) — a summed-timer artifact"
                ));
            }
        }
        if sample.outcome == TerminalOutcome::Unknown && sample.requests == 0 {
            return Err(format!(
                "sample {index}: an unknown outcome with zero requests measured nothing"
            ));
        }
        if sample.retries > 0 && sample.requests == 0 {
            return Err(format!("sample {index}: retries without requests"));
        }
    }
    Ok(())
}
