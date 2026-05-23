//! Sortable primitive encodings.
//!
//! These encodings are file-format decisions: byte-lexical order must match
//! logical order for every ordered primitive used in LMDB keys.

/// An interned dictionary identifier used for strings and bytes in hot keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternId(pub u64);

/// Encoding failure.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EncodingError {
    /// A fixed-width decoder received the wrong number of bytes.
    #[error("expected {expected} bytes, got {actual}")]
    WrongWidth { expected: usize, actual: usize },
}

/// Encodes a boolean as a one-byte ordered value.
pub fn encode_bool(value: bool) -> [u8; 1] {
    [u8::from(value)]
}

/// Decodes a boolean encoded by [`encode_bool`].
pub fn decode_bool(bytes: &[u8]) -> Result<bool, EncodingError> {
    let bytes = exact::<1>(bytes)?;
    Ok(bytes[0] != 0)
}

/// Encodes an enum code as a one-byte ordered value.
pub fn encode_enum(value: u8) -> [u8; 1] {
    [value]
}

/// Decodes an enum code encoded by [`encode_enum`].
pub fn decode_enum(bytes: &[u8]) -> Result<u8, EncodingError> {
    let bytes = exact::<1>(bytes)?;
    Ok(bytes[0])
}

/// Encodes a `u64` in big-endian order.
pub fn encode_u64(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

/// Decodes a `u64` encoded by [`encode_u64`].
pub fn decode_u64(bytes: &[u8]) -> Result<u64, EncodingError> {
    Ok(u64::from_be_bytes(exact::<8>(bytes)?))
}

/// Encodes an `i64` so byte order matches signed numeric order.
pub fn encode_i64(value: i64) -> [u8; 8] {
    ((value as u64) ^ (1u64 << 63)).to_be_bytes()
}

/// Decodes an `i64` encoded by [`encode_i64`].
pub fn decode_i64(bytes: &[u8]) -> Result<i64, EncodingError> {
    Ok((u64::from_be_bytes(exact::<8>(bytes)?) ^ (1u64 << 63)) as i64)
}

/// Encodes an interned value identifier.
pub fn encode_intern_id(value: InternId) -> [u8; 8] {
    encode_u64(value.0)
}

/// Decodes an interned value identifier.
pub fn decode_intern_id(bytes: &[u8]) -> Result<InternId, EncodingError> {
    Ok(InternId(decode_u64(bytes)?))
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N], EncodingError> {
    bytes.try_into().map_err(|_| EncodingError::WrongWidth {
        expected: N,
        actual: bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_encodings_round_trip() -> Result<(), EncodingError> {
        assert!(!decode_bool(&encode_bool(false))?);
        assert!(decode_bool(&encode_bool(true))?);

        for value in [0, 1, u8::MAX] {
            assert_eq!(decode_enum(&encode_enum(value))?, value);
        }

        for value in [0, 1, u64::MAX / 2, u64::MAX] {
            assert_eq!(decode_u64(&encode_u64(value))?, value);
        }

        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert_eq!(decode_i64(&encode_i64(value))?, value);
        }

        let intern = InternId(42);
        assert_eq!(decode_intern_id(&encode_intern_id(intern))?, intern);
        Ok(())
    }

    #[test]
    fn ordered_encodings_sort_like_values() {
        assert_order([-10i64, -1, 0, 1, 10], encode_i64);
        assert_order([0u64, 1, 2, 100, u64::MAX], encode_u64);
    }

    fn assert_order<T, const N: usize>(
        values: impl IntoIterator<Item = T>,
        encode: fn(T) -> [u8; N],
    ) where
        T: Copy + Ord + std::fmt::Debug,
    {
        let mut encoded: Vec<_> = values
            .into_iter()
            .map(|value| (value, encode(value)))
            .collect();
        let mut logical = encoded.clone();

        encoded.sort_by_key(|item| item.1);
        logical.sort_by_key(|item| item.0);

        let encoded_values: Vec<_> = encoded.into_iter().map(|item| item.0).collect();
        let logical_values: Vec<_> = logical.into_iter().map(|item| item.0).collect();
        assert_eq!(encoded_values, logical_values);
    }
}
