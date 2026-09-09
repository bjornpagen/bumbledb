//! Open-addressed word tuples for seen sets and aggregate groups.
//! Control bytes gate key reads; generation stamps distinguish live slots
//! from retained stale tags. The dense list contains only initialized live
//! values. `V: Copy` permits recycling and rehashing without drop state.
//! Unsafe value access is confined to the entry, rehash and iteration sites
//! that establish or consume those invariants.
#![allow(clippy::inline_always)]
use std::mem::MaybeUninit;

/// Ctrl bytes scanned per probe step (one SWAR word).
const WINDOW: usize = 8;

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,

    ctrl: Vec<u8>,

    keys: Vec<u64>,

    values: Vec<MaybeUninit<V>>,

    stamps: Vec<u8>,

    /// `clear` resets control bytes before a generation value is reused.
    generation: u8,

    stale: usize,

    dense: Vec<u32>,
    len: usize,
}

#[cfg(test)]
const HINT_CAP: usize = 1 << 21;

/// Max load as `len × LOAD_DEN ≤ capacity` — 3 = 33% (justified by
/// the measured {50, 33, 25}% family-ledger
/// sweep: 50% loses badly on spread (+28%), 25% costs triangle
/// +7%; 33% is best-or-near-best everywhere. Misses pay for walks, and
/// these maps are miss-heavy).
const LOAD_DEN: usize = 3;

impl<V: Copy> WordMap<V> {
    #[cfg(test)]
    pub(crate) fn retained_bytes_for_test(&self) -> usize {
        self.ctrl.capacity()
            + self.keys.capacity() * size_of::<u64>()
            + self.values.capacity() * size_of::<V>()
            + self.stamps.capacity()
            + self.dense.capacity() * size_of::<u32>()
    }

    /// Distinct entries before another growth would exceed the u32 slot index.
    /// This is a representation bound, not an execution or memory allowance.
    pub(crate) fn remaining_rows(&self) -> usize {
        let maximum = usize::try_from((u64::from(u32::MAX) + 1) / LOAD_DEN as u64)
            .expect("u32 slot cardinality fits supported targets");
        maximum - self.len
    }

    #[must_use]
    pub(crate) const fn arity(&self) -> usize {
        self.arity
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }
}

mod clear;
mod entry;
mod grow;
mod new;
mod probe;

use super::swar::{ctrl_tag, eq_byte_mask, hash_core, hash_words, zero_byte_mask};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1_control_wordmap.rs"]
mod q1_control;

#[cfg(test)]
#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q2_payload_regression.rs"]
mod q2_payload_regression;
