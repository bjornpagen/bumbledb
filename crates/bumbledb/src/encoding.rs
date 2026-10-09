//! Order-preserving scalar words for query images and keys: byte order is
//! value order. Stored rows use [`crate::canonical`].
mod decode;
mod encode;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use decode::decode_u64;
pub use decode::{decode_f64, decode_i64};
pub use encode::{encode_bool, encode_f64, encode_i64, encode_u64};

/// A query-image text token: the dense id a prepared query's process-scoped
/// interner mints per distinct text; stored rows own their text inline. Ids
/// allocate from 0; [`InternId::SENTINEL`] (`u64::MAX`) is never minted and is
/// the miss token on query-word paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternId(u64);

impl InternId {
    pub const SENTINEL: Self = Self(u64::MAX);

    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn is_sentinel(self) -> bool {
        self.0 == Self::SENTINEL.0
    }
}

/// The widest `bytes<N>` field, in bytes.
pub const MAX_FIXED_BYTES: usize = bumbledb_theory::schema::MAX_FIXED_BYTES as usize;

/// The word count of a `bytes<len>` value's padded encoding: `⌈len/8⌉`.
#[must_use]
pub const fn fixed_bytes_words(len: u16) -> usize {
    (len as usize).div_ceil(8)
}

/// One `bytes<N>` value: the raw bytes inline in a fixed 64-byte buffer
/// (`Copy`, borrow-free), zero-padded to whole words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedBytesValue {
    bytes: [u8; MAX_FIXED_BYTES],
    len: u8,
}

impl FixedBytesValue {
    /// # Panics
    /// On a width outside `1..=64`.
    #[must_use]
    pub fn new(raw: &[u8]) -> Self {
        assert!(
            !raw.is_empty() && raw.len() <= MAX_FIXED_BYTES,
            "bytes<N> widths are 1..=64"
        );
        let mut bytes = [0u8; MAX_FIXED_BYTES];
        bytes[..raw.len()].copy_from_slice(raw);
        Self {
            bytes,
            len: u8::try_from(raw.len()).expect("len <= 64"),
        }
    }

    #[must_use]
    pub fn padded(&self) -> &[u8] {
        &self.bytes[..fixed_bytes_words(u16::from(self.len)) * 8]
    }
}

const I64_SIGN_BIT: u64 = 1 << 63;
