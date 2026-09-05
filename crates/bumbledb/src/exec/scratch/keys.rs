//! Ordered fixed-word keys, named maps, and early-stoppable visitors for
//! the one scratch substrate (C2). Wide exact keys stay full-byte compared;
//! fingerprint buckets may collide and never merge distinct tuples.

use crate::error::Result;

/// Named map on one [`super::ScratchRelation`] / one `ScratchEnv`.
/// A claim cursor, a group-header get, the insertion-order log, and text
/// forward/reverse share this roster — never a second environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScratchMapId {
    /// Default exact map. Pack claims (`ScratchClaimKey`) live here.
    Default = 0,
    /// Pack: exact group-head bytes → token (big-endian u64).
    GroupToToken = 1,
    /// Pack: token (big-endian u64) → exact group-head bytes.
    TokenToGroup = 2,
    /// Insertion-order log (`seq → row/key bytes`) for SpillSet / projection
    /// drain watermarks. Same env as [`Self::Default`]; not a second relation.
    OrderLog = 3,
    /// Exact text → token for nonresident forward lookup. Same env as
    /// [`Self::TextReverse`]; not a second `ScratchRelation`.
    TextForward = 4,
    /// Exact token → text bytes for nonresident reverse lookup. Same env
    /// as [`Self::TextForward`]; not a second `ScratchRelation`.
    TextReverse = 5,
}

impl ScratchMapId {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::GroupToToken,
        Self::TokenToGroup,
        Self::OrderLog,
        Self::TextForward,
        Self::TextReverse,
    ];
    pub const COUNT: usize = 6;

    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn lmdb_name(self) -> &'static str {
        match self {
            Self::Default => "scratch",
            Self::GroupToToken => "group_to_token",
            Self::TokenToGroup => "token_to_group",
            Self::OrderLog => "order_log",
            Self::TextForward => "text_forward",
            Self::TextReverse => "text_reverse",
        }
    }
}

/// Ordered fixed-width word key. Encoded big-endian so byte order equals
/// unsigned word order. Inline under [`super::MAX_INLINE_KEY`] for every
/// `WORDS` that fits `WORDS * 8 <= 400`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScratchWordKey<const WORDS: usize> {
    words: [u64; WORDS],
}

impl<const WORDS: usize> ScratchWordKey<WORDS> {
    pub const BYTE_LEN: usize = WORDS * 8;

    #[must_use]
    pub const fn new(words: [u64; WORDS]) -> Self {
        Self { words }
    }

    #[must_use]
    pub const fn words(self) -> [u64; WORDS] {
        self.words
    }

    /// Write the ordered encoding into `out`. `out` must be at least
    /// [`Self::BYTE_LEN`] bytes.
    pub fn write(self, out: &mut [u8]) {
        debug_assert!(out.len() >= Self::BYTE_LEN);
        for (index, word) in self.words.iter().enumerate() {
            let start = index * 8;
            out[start..start + 8].copy_from_slice(&word.to_be_bytes());
        }
    }

    #[must_use]
    pub fn encode(self) -> Vec<u8> {
        let mut out = vec![0u8; Self::BYTE_LEN];
        self.write(&mut out);
        out
    }

    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTE_LEN {
            return None;
        }
        let mut words = [0u64; WORDS];
        for (index, word) in words.iter_mut().enumerate() {
            let start = index * 8;
            *word = u64::from_be_bytes(bytes[start..start + 8].try_into().ok()?);
        }
        Some(Self { words })
    }
}

/// Pack claim: 3×big-endian u64 `(token, start, end)`. Always 24 bytes,
/// which is ≤ [`super::MAX_INLINE_KEY`], so claim walks are exact key
/// order. This is not a mode tag — 0xFE payload inference is deleted.
pub type ScratchClaimKey = ScratchWordKey<3>;

/// Ordered 4-word exact key for colliding-wide heads that still fit
/// inline. Not Pack-mode inference; oversized heads use [`ScratchExactKey`].
pub type ScratchWideClaimKey = ScratchWordKey<4>;

/// Exact arbitrary key. Equality is the full byte string; oversized keys
/// share fingerprint buckets and are compared exactly (forced collisions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScratchExactKey<'a> {
    bytes: &'a [u8],
}

impl<'a> ScratchExactKey<'a> {
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    #[must_use]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }
}

/// Outcome of one exact get. Absence is not an error; I/O, cancelled
/// work, and admission refuse as [`crate::error::Error`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchProbe<T> {
    Hit(T),
    Miss,
}

impl<T> ScratchProbe<T> {
    #[must_use]
    pub const fn is_hit(&self) -> bool {
        matches!(self, Self::Hit(_))
    }

    #[must_use]
    pub const fn is_miss(&self) -> bool {
        matches!(self, Self::Miss)
    }
}

/// Early-stoppable fallible visit. `Ok(false)` stops the walk.
pub type ScratchVisit<'v> = &'v mut dyn FnMut(&[u8], &[u8]) -> Result<bool>;

/// Early-stoppable fallible visitor over exact key/value bytes.
pub trait ScratchVisitor {
    /// # Errors
    /// Callback or ledger failure. `Ok(false)` is a clean early stop.
    fn visit(&mut self, key: &[u8], value: &[u8]) -> Result<bool>;
}

impl<F> ScratchVisitor for F
where
    F: FnMut(&[u8], &[u8]) -> Result<bool>,
{
    fn visit(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
        self(key, value)
    }
}
