use crate::EncodedRef;

/// Owned fixed-width encoded value used by query iterators.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EncodedOwned {
    /// One-byte encoded value.
    One([u8; 1]),
    /// Eight-byte encoded value.
    Eight([u8; 8]),
    /// Sixteen-byte encoded value.
    Sixteen([u8; 16]),
}

impl EncodedOwned {
    /// Copies an encoded reference into an owned value.
    pub fn from_ref(value: EncodedRef<'_>) -> Self {
        match value {
            EncodedRef::One(bytes) => EncodedOwned::One(*bytes),
            EncodedRef::Eight(bytes) => EncodedOwned::Eight(*bytes),
            EncodedRef::Sixteen(bytes) => EncodedOwned::Sixteen(*bytes),
        }
    }

    /// Returns this value as encoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            EncodedOwned::One(bytes) => &bytes[..],
            EncodedOwned::Eight(bytes) => &bytes[..],
            EncodedOwned::Sixteen(bytes) => &bytes[..],
        }
    }

    /// Returns this owned value as a borrowed encoded reference.
    pub fn as_ref(&self) -> EncodedRef<'_> {
        match self {
            EncodedOwned::One(bytes) => EncodedRef::One(bytes),
            EncodedOwned::Eight(bytes) => EncodedRef::Eight(bytes),
            EncodedOwned::Sixteen(bytes) => EncodedRef::Sixteen(bytes),
        }
    }
}

/// Linear iterator over encoded keys at one trie depth.
pub trait LinearIter {
    /// Returns the current key.
    fn key(&self) -> Option<EncodedRef<'_>>;
    /// Advances to the next distinct key.
    fn next(&mut self);
    /// Seeks to the first key greater than or equal to `target`.
    fn seek(&mut self, target: EncodedRef<'_>);
    /// Returns true when the iterator is exhausted.
    fn at_end(&self) -> bool;
}

/// Trie iterator used by the Free Join runtime.
pub trait TrieIter: LinearIter {
    /// Opens the next depth.
    fn open(&mut self);
    /// Moves back to the parent depth.
    fn up(&mut self);
}
