//! An open-addressed map over inline u64 word tuples (docs/architecture/30-execution.md): the sink
//! machinery's seen-sets and group maps. Rebuilt by docs/perf/ PRD 06 as
//! a tag-byte-controlled single-probe-line map: a control byte per slot
//! (0 = empty, else `0x80 | top-7-hash-bits`) means a probe step
//! usually touches ONE ctrl line, key words load only on a tag match
//! (~1/128 of collisions falsely), and values are uninitialized until
//! occupied — no `Option` in the slot array.
//!
//! Geometry and probe shape follow the measured law (docs/silicon/03,
//! bumblebench exps 01/02): these maps are MISS-heavy by construction —
//! a seen-set's first sight of every distinct key is a miss — and misses
//! cost more than hits in open addressing (walk length plus a
//! mispredicted exit branch). Two consequences, built in:
//!
//! - **33% max load** (was 50%): dropping load factor shortens the walks
//!   that misses pay for (measured miss cost fell 9.2 → 2.8 ns between
//!   38% and 5% load); the {50, 33, 25}% ledger sweep picked 33% —
//!   most of the walk win at 1.5× the memory.
//! - **Branchless window probing**: the ctrl bytes are scanned eight at
//!   a time with SWAR masks — one well-predicted exit branch per window
//!   instead of one branch per slot (measured 4.6× at hit-rate 0). The
//!   ctrl slab carries a `WINDOW-1`-byte mirror of its first bytes so a
//!   window read never wraps.
//!
//! Growth stays rehash-double with insertion order preserved (the dense
//! rule: iteration *and clearing* walk `O(len)`, never `O(capacity)`).
//!
#![allow(unsafe_code)] // 00-product unsafe policy: this module is allowlisted
#![allow(clippy::inline_always)]
// docs/silicon/03/04: the probe path's
// inlining is load-bearing (per-element call ceremony was measured cost)
// and machine-checked by scripts/check-asm.sh, not trusted to attributes.
//! `unsafe` per the 00-product policy (this module is allowlisted): the
//! `MaybeUninit` reads are gated by ctrl-byte occupancy, and the probe
//! indices are masked to the power-of-two capacity — both invariants
//! stated at the sites. `V: Copy` keeps the uninitialized-slot story
//! drop-free (both users store `()` and `usize`).

use std::mem::MaybeUninit;

/// Ctrl bytes scanned per probe step (one SWAR word).
const WINDOW: usize = 8;

/// Fixed-arity word-tuple keys mapping to `V`. No tombstones (insert-only).
#[derive(Debug)]
pub struct WordMap<V> {
    arity: usize,
    /// One control byte per slot (0 = empty, else `0x80 | tag7(hash)`),
    /// plus a `WINDOW - 1`-byte mirror of the first bytes at the tail so
    /// window loads never wrap (`ctrl.len() == capacity + WINDOW - 1`).
    ctrl: Vec<u8>,
    /// `capacity * arity` key words.
    keys: Vec<u64>,
    /// One value per slot, initialized exactly when its ctrl byte is set.
    values: Vec<MaybeUninit<V>>,
    /// Occupied slot indices in insertion order — docs/architecture/30-execution.md dense
    /// rule, extended to the sink maps: iteration *and clearing* walk
    /// O(len), never O(capacity), so one hot execution's high-water
    /// cannot tax every later execution's finalize and reset.
    dense: Vec<u32>,
    len: usize,
}

/// The presizing clamp (docs/perf/ PRD 06): hints are planner estimates —
/// trusted enough to kill rehash storms, capped so a wild estimate cannot
/// balloon a sink.
const HINT_CAP: usize = 1 << 21;

/// Max load as `len × LOAD_DEN ≤ capacity` — 3 = 33% (docs/silicon/03,
/// justified by the {50, 33, 25}% family-ledger sweep recorded in that
/// PRD's Result: 50% loses badly on spread (+28%), 25% costs triangle
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
mod hash;
mod new;
mod probe;

#[cfg(test)]
mod tests;
