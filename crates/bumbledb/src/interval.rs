//! The host-facing Interval value — the checked type lives in
//! `bumbledb-theory` (parse, don't validate: a held [`Interval`] always
//! satisfies `start < end`); this module re-exports it and keeps the
//! engine-only half behind: the coalescing segment sweep and the
//! order-based overlap index, which are commit/exec machinery, not
//! theory.

pub(crate) mod overlap;
pub(crate) mod sweep;

pub use bumbledb_theory::Interval;
