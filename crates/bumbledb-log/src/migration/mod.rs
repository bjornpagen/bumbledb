//! Native canonical migration execution and durable cutover (C11, chapter
//! 22/33). Internal shared-native implementation, not a public Rust SDK.
//!
//! One canonical data family, one executor:
//!
//! - [`plan`] — the finite declarative plan roster, its one JSON spelling
//!   and its canonical frame bytes (the hashed identity). Expressions are
//!   the CORE scalar roster; there is no migration evaluator, callback,
//!   textual parser or executable plan content.
//! - [`manifest`] — the ordered chain, acyclic domain-separated prefix
//!   digests and the plan-set identity a freeze intent cites.
//! - [`compile`] — complete native plan admission: schema binding, total
//!   source/target coverage, explicit destructive acknowledgements, exact
//!   typing; lowering onto `bumbledb::ScalarExpr`.
//! - [`state`] — the private, work-charged ordered-step evaluation with the
//!   core judge at every declared validate boundary.
//! - [`history`] — the authoritative `Applied`/`Baseline` records stored
//!   transactionally beside facts, outside the receipt-retirement keyspace.
//! - [`lock`] — the stable local target namespace: kernel exclusion,
//!   durable pre-genesis tombstones, no-overwrite installation.
//! - [`executor`] — freeze → capture → ordered execution → ONE staged final
//!   target → durable `ReadyToSwitch` → explicit activation; abort fences
//!   the target durably BEFORE thawing the matching source.
//! - [`hosted`] — the same authority transitions over the C07 conditional
//!   store with the three-way certainty grammar (`Unknown` never thaws),
//!   plus the COMPLETE hosted workflow ([`hosted::HostedMigration`],
//!   [`hosted::initialize`]): the staged target's verified checkpoint and
//!   history metadata published under the target's open object epoch, named
//!   by the genesis head's recovery root — S3 is the hosted authority.
//!
//! The TypeScript generator (P10) calls [`crate::schema_file`] and the
//! [`plan`]/[`manifest`] codecs through the native boundary; generation and
//! execution share these exact canonical bytes, so no digest or encoding is
//! ever computed twice in two languages. There is no callback migration,
//! per-file full-incarnation publication, or per-step public root: k small
//! pending plans build one final target and one genesis, and a failed or
//! uncertain migration can neither silently thaw its source, activate its
//! target, nor erase its only evidence.

pub mod compile;
pub mod executor;
mod frame;
pub mod history;
pub mod hosted;
pub(crate) mod json;
pub mod lock;
pub mod manifest;
pub mod plan;
pub mod state;

pub use executor::{
    AbortReport, AbortRequest, ActivateReport, ActivationRef, LocalMigration, MigrateOutcome,
    MigrationError, MigrationStatus, StepInput, SuffixRequest, TargetFence, activate_target,
    fence_target, initialize,
};
pub use frame::{
    PLAN_DIGEST_DOMAIN, PLAN_SET_DIGEST_DOMAIN, PREFIX_DIGEST_DOMAIN, STATE_DIGEST_DOMAIN,
    SYSTEM_DIGEST_DOMAIN,
};
pub use hosted::{HostedCutover, HostedMigration, HostedOutcome};
