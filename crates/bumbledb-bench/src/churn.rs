//! The churn harness — long-lived high-churn performance over time
//! (the degradation lane the write families cannot see): a steady-state
//! working set of postings churned by a configured mix, applied with
//! identical logical operations to a bumbledb store and its `SQLite`
//! mirrors, with three-way end-state verification.
//!
//! The charter: degradation curves on BOTH engines — `Kind::Report`-class
//! by design, NEVER a gate; every timed number arrives only via the
//! owner's night session, and nothing in this module times anything.
//! The never-reissue law means the id space burns monotonically under
//! churn — an aborted mint burns like a committed one — and the lane
//! watches whether anything degrades with the burn.
//!
//! The layers:
//!
//! - [`ops`] — the pure protocol: the mix, the per-cycle plan, and the
//!   `LiveSet` model. A cycle's operations are a pure function of
//!   `(seed, cycle, live_len)` — no wall clock, no engine state — so
//!   determinism and resumability are properties of the type, not a
//!   discipline.
//! - [`engines`] — the twin stores and the per-cycle appliers: one
//!   `db.write` per cycle on ours, one transaction per cycle on each
//!   mirror, the identical logical operations.
//! - [`lanes`] — the lane registry, pure data: a [`lanes::RunSpec`]
//!   structurally carries exactly ONE ours lane (the id-minter) plus
//!   its `SQLite` twins; the five mandated lanes are three rows.
//! - [`probes`] — the pinned read probes whose per-sample p50 is the
//!   degradation curve's y-axis: exact IR, stationary draws, and the
//!   per-sample oracle gate carried by type.
//! - [`report`] — the time-series report artifact (`churn_schema: 1`):
//!   cycle → sample per lane, hand-rolled JSON + markdown, pinned by a
//!   parse round-trip.
//! - [`run`] — the per-run driver loop: lockstep twins, per-lane
//!   wall-time windows, maintenance charged into the maintained lane's
//!   own series, the probe sampler with its riding oracle gate.
//! - [`verify_end`] — the three-way end gate: model vs engine vs
//!   `SQLite` posting multisets, then the store sweeper.

pub mod engines;
pub mod lanes;
pub mod ops;
pub mod probes;
pub mod report;
pub mod run;
pub mod verify_end;

#[cfg(test)]
mod tests;
