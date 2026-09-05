//! Foundational successor history values and bounded framing.
//!
//! This is not the old braided protocol, a second fact codec, or a working
//! LocalHistory authority. The executable change, scalar result, complete
//! rejection codec, and atomic facts/receipt storage belong to the core. Until
//! the full authority exists, low-level framing returns explicitly
//! **unverified** borrowed envelopes. `command::Command` uses the core's one
//! checked `ChangeSet` and bounded hashing, without a second fact payload.
//!
//! `SchemaId` temporarily aliases the core's existing fingerprint. These
//! frames are implementation fixtures, not a successor format qualification or
//! a promise that the old schema hash survives the new core format. This Rust
//! surface is internal shared-native implementation, not a public log SDK.

pub mod admission;
pub mod command;
mod types;

pub use bumbledb::SchemaFingerprint as SchemaId;
pub use types::*;
