//! Fact identity: the full 32-byte blake3 of the canonical fact bytes.

/// Fact identity: the full 32-byte blake3 of the canonical fact bytes.
///
/// Never truncated (v5 truncated to 16 bytes — post-mortem §00). Hash
/// equality is treated as fact equality; collisions are an accepted axiom
/// recorded in `docs/architecture/10-data-model.md`.
#[must_use]
pub fn fact_hash(fact_bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(fact_bytes).as_bytes()
}
