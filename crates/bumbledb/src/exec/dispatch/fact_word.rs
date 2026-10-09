//! The single-row operand shape of the key-probe path: one decoded field as column
//! words. Decoding lives in the canonical walker (`image/canon.rs`); this type is
//! the span-shaped view consumers dispatch on.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FactOperand {
    Word(u64),
    Pair(u64, u64),

    Block { words: [u64; 8], count: u8 },
}
