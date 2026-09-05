//! The single-row operand shape shared by the key-probe path and the
//! cursor fallback: one decoded field as column words. The decoding itself
//! lives in the one canonical walker (`image/canon.rs`); this type is the
//! span-shaped view consumers dispatch on.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactOperand {
    Word(u64),
    Pair(u64, u64),

    Block { words: [u64; 8], count: u8 },
}
