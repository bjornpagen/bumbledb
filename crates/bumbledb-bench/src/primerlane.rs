//! The Primer-shaped attribution lane
//! (proposals/one-representation/10-measurement.md): the write-path rig
//! the one-representation set measures against — REPORT-class
//! ([`crate::lanes`] carries the charter; no budget gate ever reads a
//! primerlane number).
//!
//! The corpus is a config-driven synthetic GENERATOR ([`corpus`]) —
//! never a data file: a parameterized roster of ordinary relations
//! (a representative slice of Primer's 39), mixed arities 2–8, every
//! relation fresh-keyed, str columns fed by a Zipf-distributed
//! vocabulary plus a long-tail novel population, a containment chain,
//! and one capacity statement. Identical config ⇒ identical rows,
//! forever (the [`crate::corpus_gen`] law).
//!
//! The lanes ([`run`]), one function per arm — the seams the later
//! waves extend:
//!
//! - *builder lane*: `InstanceBuilder::load_dyn` per relation →
//!   `admit` → `Db::from_instance` (Primer's persist path —
//!   `HeapMutation`, then the durable publish);
//! - *delta lane*: `Db::create`, seed half the corpus, then one
//!   `db.write` + `insert_dyn` of the other half (the incremental path
//!   — `StoreMutation`, the full commit pipeline);
//! - *read lane*: full `scan` decode per relation (the regression
//!   baseline). The `count` read lane (40-exact-count.md) and the
//!   accepted-collection arm (20-accepted-collection.md) land beside
//!   it as their own functions.
//!
//! Output: one table with wall time per phase (std `Instant`); under
//! the obs build, `--alloc` adds per-phase allocation windows
//! ([`bumbledb::alloc_counter`]) and `--trace` adds one capture over
//! the lanes with the span totals folded into the upstream report's
//! component table ([`components`]) — mutually exclusive passes, the
//! obs doctrine.

pub mod components;
pub mod corpus;
pub mod report;
pub mod run;
#[cfg(test)]
mod tests;

pub use run::run;

/// The corpus identity: relation count + total facts + seed. Everything
/// else derives ([`corpus`] owns the derivations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimerConfig {
    /// Ordinary relation count (≥ 2 — the containment chain and the
    /// capacity statement need a parent and a child).
    pub relations: u32,
    /// Total facts across the roster, split by the skew weights.
    pub facts: u64,
    pub seed: u64,
}
