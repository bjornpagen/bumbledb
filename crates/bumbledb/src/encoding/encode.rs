//! Order-preserving scalar words: byte order equals value order.
use super::I64_SIGN_BIT;

/// Encodes a Bool as its canonical single byte.
#[must_use]
pub(crate) const fn encode_bool(value: bool) -> u8 {
    value as u8
}

/// Encodes a U64 as big-endian bytes (lexicographic order = numeric order).
#[must_use]
pub(crate) const fn encode_u64(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

/// Encodes an I64 as sign-flipped big-endian bytes: flipping the sign bit
/// biases the value so lexicographic byte order equals numeric order.
#[must_use]
pub(crate) const fn encode_i64(value: i64) -> [u8; 8] {
    (value.cast_unsigned() ^ I64_SIGN_BIT).to_be_bytes()
}

/// Encodes a canonical F64 as total-order bytes for physical facts and query
/// keys. Wire payloads instead use [`bumbledb_theory::F64::to_be_bytes`].
#[must_use]
pub(crate) const fn encode_f64(value: bumbledb_theory::F64) -> [u8; 8] {
    value.to_order_bytes()
}
