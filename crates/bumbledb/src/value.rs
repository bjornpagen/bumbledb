//! The one literal-value sum — the definition lives in `bumbledb-theory`
//! . This module is the facade half of the split: the
//! public path `bumbledb::value::Value` stays valid forever, while internal
//! engine code imports `bumbledb_theory::Value` directly.
//! Hosts depend on this crate alone; the theory crate is not API.

// Dormant by design: after the internal import sweep nothing in-crate
#[expect(
    unused_imports,
    reason = "the facade path is contract, not plumbing — kept compiling \
              with zero internal users"
)]
pub use bumbledb_theory::Value;
