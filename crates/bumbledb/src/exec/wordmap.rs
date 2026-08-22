//! An open-addressed map over inline u64 word tuples: the sink
//! machinery's seen-sets and group maps. A tag-byte-controlled
//! single-probe-line map: a control byte per slot
//! (0 = empty, else `0x80 | top-7-hash-bits`) means a probe step
//! usually touches ONE ctrl line, key words load only on a tag match
//! (~1/128 of collisions falsely), and values are uninitialized until
#![allow(unsafe_code)] // 00-product unsafe policy: this module is allowlisted
#![allow(clippy::inline_always)]
//! `MaybeUninit` reads are gated by ctrl-byte occupancy, and the probe
//! indices are masked to the power-of-two capacity — both invariants
//! stated at the sites. `V: Copy` keeps the uninitialized-slot story
//! `unsafe` per the 00-product policy (this module is allowlisted): the
//! drop-free (both users store `Copy` values).
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

    /// forces the physical reset before a stamp value is ever reused,
    generation: u8,

    stale: usize,

    dense: Vec<u32>,
    len: usize,
}

const HINT_CAP: usize = 1 << 21;

/// Max load as `len × LOAD_DEN ≤ capacity` — 3 = 33% (justified by
/// the measured {50, 33, 25}% family-ledger
/// sweep: 50% loses badly on spread (+28%), 25% costs triangle
/// +7%; 33% is best-or-near-best everywhere. Misses pay for walks, and
/// these maps are miss-heavy).
const LOAD_DEN: usize = 3;

impl<V: Copy> WordMap<V> {
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
