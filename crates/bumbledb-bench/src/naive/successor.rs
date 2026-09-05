//! P11's successor semantic models (chapters 10/12/13; audit ASS-001 and
//! ASS-002 assurance routing).
//!
//! Two independent models live here, beside the existing [`super::NaiveDb`]
//! reference judge:
//!
//! * [`admission`] — mutable-consulted-relation support, the one-command
//!   normalization/tie rule, and the union/commutation counterexamples that
//!   bound what the raw-delta algebra licenses (Lean twins:
//!   `lean/Bumbledb/Txn/Support.lean`, countermodels in chapter 02).
//! * [`staged`] — the independent staged relation-expression evaluator:
//!   acyclic stages with aggregate/computed outputs consumed downstream,
//!   producer-error propagation, inline/materialize equivalence, and the
//!   frozen-finite-domain recursion fence (Lean twin:
//!   `lean/Bumbledb/Query/Stages.lean`).
//!
//! Neither model calls production equality, arithmetic, hashing or
//! transition helpers as an oracle: schema DESCRIPTORS are consumed as data
//! (declarations, per chapter 62's reuse rule), judgments and evaluations
//! are recomputed here, and float expectations come from the independent
//! bit/rational oracle (`crate::verify::f64_oracle`).

pub mod admission;
pub mod staged;
