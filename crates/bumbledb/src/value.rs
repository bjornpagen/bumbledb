//! The one literal-value sum — the definition lives in `bumbledb-theory`
//! (`docs/architecture/30-dependencies.md`: dependencies and queries share
//! one representation). This module is the facade half of the split: the
//! public path `bumbledb::value::Value` stays valid forever, while internal
//! engine code imports `bumbledb_theory::Value` directly (the facade is
//! API, never an internal crutch — `docs/architecture/70-api.md`).
//! Hosts depend on this crate alone; the theory crate is not API.

// Dormant by design: after the internal import sweep nothing in-crate
// resolves through this path (internal code names the theory crate), but
// the path itself is pinned — deleting it would silently break any
// referrer the next refactor adds expecting the facade contract.
#[expect(
    unused_imports,
    reason = "the facade path is contract, not plumbing — kept compiling \
              with zero internal users"
)]
pub use bumbledb_theory::Value;
