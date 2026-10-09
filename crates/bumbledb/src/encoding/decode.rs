//! Decoders for the order-preserving scalar words of query images.
use super::I64_SIGN_BIT;
use crate::error::CorruptionError;

/// Decodes big-endian U64 bytes.
#[cfg(test)]
#[must_use]
pub(crate) const fn decode_u64(bytes: [u8; 8]) -> u64 {
    u64::from_be_bytes(bytes)
}

/// Decodes sign-flipped big-endian I64 bytes.
#[must_use]
pub(crate) const fn decode_i64(bytes: [u8; 8]) -> i64 {
    (u64::from_be_bytes(bytes) ^ I64_SIGN_BIT).cast_signed()
}

/// Decodes physical total-order F64 bytes, rejecting noncanonical zero and
/// NaN representations instead of normalizing corrupt stored data.
/// # Errors
/// [`CorruptionError::NonCanonicalF64`] for a noncanonical order key.
pub(crate) const fn decode_f64(bytes: [u8; 8]) -> Result<bumbledb_theory::F64, CorruptionError> {
    match bumbledb_theory::F64::from_order_bytes(bytes) {
        Ok(value) => Ok(value),
        Err(_) => Err(CorruptionError::NonCanonicalF64(bytes)),
    }
}
