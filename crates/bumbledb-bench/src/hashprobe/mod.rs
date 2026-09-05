//! The hash qualification campaign (chapter 41; gates HASH-01..HASH-04).
//!
//! This module is authored during F1 and **executes only in F3**, before the
//! physical format freeze (C12). It owns:
//!
//! - the role/width/domain inventory that HASH-01 checks (this file),
//! - the reproducible sizing math for HASH-03 ([`sizing`]),
//! - the representative canonical-input corpus ([`inputs`]),
//! - the BLAKE3-full / BLAKE3-truncated-16 / AEGIS-128L-MAC candidate probe
//!   with one-shot/streaming equivalence and timing ([`probe`], HASH-04),
//! - known-answer-test loading with typed refusal when vectors are absent
//!   ([`kat`], HASH-01 — a missing vector file is `Err`, never a silent pass),
//! - the forced-collision workload schedules for HASH-02 ([`collision`]).
//!
//! Nothing here selects a persisted format. The selected default remains
//! 16-byte exact-checked local fingerprints (initially the first 16 bytes of
//! domain-separated BLAKE3) plus 32-byte authoritative BLAKE3. AEGIS is a
//! measured candidate that can revise the *local fingerprint* algorithm before
//! format freeze; it is never a replacement for authoritative commitments and
//! never a per-CPU format variation. Hardware implementations of the selected
//! algorithm must return identical bytes on every supported target.

pub mod collision;
pub mod inputs;
pub mod kat;
pub mod probe;
pub mod sizing;
#[cfg(test)]
mod tests;

/// The distinct jobs hashes do in the successor. One role, one width, one
/// domain — a generic helper must never truncate an authoritative digest or
/// widen a routing hash into a commitment (HASH-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashRole {
    /// Per-fact local membership fingerprint: 16 bytes, exact canonical-byte
    /// comparison always follows. A collision adds lookup work, never merges
    /// two facts. Accidental-collision budget is per lookup namespace
    /// (tenant × relation), not one global trillion-row domain.
    LocalFingerprint,
    /// Authoritative content identity: 32-byte BLAKE3 over schemas, commands,
    /// receipts, decision chains, snapshots, migration plans and remote
    /// objects. Carries the explicit ~128-bit generic collision-resistance
    /// premise; truncation is forbidden everywhere.
    AuthoritativeContent,
    /// Transient 64-bit routing hashes inside fixed-word in-memory tables,
    /// always followed by full-key comparison with bounded work. Never
    /// persisted, never a commitment, never cryptographic.
    TransientRouting,
    /// Application-owned 16-byte Id128. Not a content hash and not proof of
    /// database-issued uniqueness; duplicate IDs follow ordinary schema laws.
    ApplicationId,
}

/// One row of the HASH-01 inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleSpec {
    pub role: HashRole,
    pub width_bytes: u32,
    /// The selected algorithm family, or `None` where the bytes are not a
    /// hash at all (application IDs are caller-supplied identity bytes).
    pub algorithm: Option<&'static str>,
    /// Whether truncating this value below `width_bytes` is ever legal.
    pub truncation_allowed: bool,
    /// Whether exact full-byte comparison backs every equality decision made
    /// through this value.
    pub exact_check_backed: bool,
    /// The collision domain the sizing math must use ([`sizing`]).
    pub collision_domain: &'static str,
    /// Adversarial stance: `true` when deliberate collision search is part of
    /// the threat model (generic collision resistance ≈ width/2 bits).
    pub adversarial: bool,
}

/// The complete role inventory. HASH-01's checker asserts this roster, and
/// [`tests`] pin its invariants (no truncatable authoritative digest, every
/// non-adversarial role backed by exact comparison, widths match chapter 41).
#[must_use]
pub const fn role_inventory() -> [RoleSpec; 4] {
    [
        RoleSpec {
            role: HashRole::LocalFingerprint,
            width_bytes: 16,
            algorithm: Some(
                "BLAKE3 (domain-separated, first 16 bytes); AEGIS-128L MAC is the measured candidate",
            ),
            truncation_allowed: false,
            exact_check_backed: true,
            collision_domain: "per lookup namespace (tenant x relation), lifetime of live facts",
            adversarial: false,
        },
        RoleSpec {
            role: HashRole::AuthoritativeContent,
            width_bytes: 32,
            algorithm: Some("BLAKE3"),
            truncation_allowed: false,
            exact_check_backed: false,
            collision_domain: "every distinct object generated over the retention lifetime",
            adversarial: true,
        },
        RoleSpec {
            role: HashRole::TransientRouting,
            width_bytes: 8,
            algorithm: Some("existing in-memory routing hash (non-cryptographic)"),
            truncation_allowed: false,
            exact_check_backed: true,
            collision_domain: "one transient table build",
            adversarial: false,
        },
        RoleSpec {
            role: HashRole::ApplicationId,
            width_bytes: 16,
            algorithm: None,
            truncation_allowed: false,
            exact_check_backed: true,
            collision_domain: "application-chosen; UUIDv4 carries 122 random bits, not 128",
            adversarial: false,
        },
    ]
}
