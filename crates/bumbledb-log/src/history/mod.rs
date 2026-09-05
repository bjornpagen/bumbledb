//! The internal successor history machine's value vocabulary.
//!
//! One authority record and its legal transitions ([`authority`]), pure
//! current-admission guards ([`admission`]), sealed canonical commands and
//! receipt/outcome framing ([`command`]), immutable decision/genesis records
//! ([`decision`]) and retained receipt rows ([`receipt`]). The executable
//! change payload, scalar results, rejection evidence and atomic
//! facts/receipt storage belong to the core; this module frames identity,
//! precondition and outcome metadata around the core's canonical bytes and
//! never interprets them with a second codec.
//!
//! `SchemaId` aliases the core's canonical schema fingerprint. This Rust
//! surface is internal shared-native implementation, not a public log SDK,
//! and its physical bytes remain provisional until the F3 format freeze.

pub mod admission;
pub mod authority;
pub mod command;
pub mod decision;
mod frame;
pub mod locator;
pub mod receipt;
mod types;

pub use bumbledb::SchemaFingerprint as SchemaId;
pub use frame::FrameError;
pub use locator::{
    ChainVisitor, OBJECT_REF_OPTION_ABSENT_BYTES, OBJECT_REF_OPTION_PRESENT_BYTES,
    OBJECT_REF_WIRE_BYTES, object_ref_option_bytes, parent_locator_field_bytes,
    receive_limits_for_object, validate_parent_locator, validate_recovery_locators,
    validate_tip_locator, walk_decision_chain, walk_decision_chain_collect,
};
pub use types::*;
