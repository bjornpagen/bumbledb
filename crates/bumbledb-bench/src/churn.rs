//! The charter: degradation curves on BOTH engines — `Kind::Report`-class by
//! design, NEVER a gate; every timed number arrives only via the - [`ops`] —
//! the pure protocol: the mix, the per-cycle plan, and the
pub mod engines;
pub mod lanes;
pub mod ops;
pub mod probes;
pub mod report;
pub mod run;
pub mod verify_end;

#[cfg(test)]
mod tests;
