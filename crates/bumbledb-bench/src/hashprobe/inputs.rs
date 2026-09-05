//! The representative probe-input corpus (chapter 41 §"A small physical
//! accounting and hash qualification campaign", item 5).
//!
//! Sizes cover the actual hashing population: short canonical facts
//! (0/8/16/24/32/64/128 bytes), page-ish payloads (1 KiB / 4 KiB) and one
//! snapshot chunk (8 MiB). Buffers are deterministic in the seed so the F3
//! probe binds its result to `(seed, corpus digest)` and any two machines
//! hash byte-identical inputs. Alignment offsets exist because AES/SIMD hash
//! kernels can be sensitive to the base pointer; the probe times each input
//! at every offset instead of assuming the allocator's favor.

/// The representative one-shot sizes. `8 MiB` is the chapter 21 snapshot
/// chunk discussion size; the small sizes bracket real encoded facts.
pub const SIZES: [usize; 10] = [0, 8, 16, 24, 32, 64, 128, 1024, 4096, 8 * 1024 * 1024];

/// Base-pointer offsets exercised per input (0 = allocator-natural). The
/// probe copies into `vec![0; len + offset]` and hashes the subslice.
pub const ALIGN_OFFSETS: [usize; 3] = [0, 1, 3];

/// A realistic mixture protocol: the probe must also time an interleaved
/// stream of short inputs (weighted toward 16–64-byte facts) rather than only
/// same-size batches, because branch predictors and state-copy costs behave
/// differently under mixed lengths.
pub const MIXTURE_WEIGHTS: [(usize, u32); 6] =
    [(8, 1), (16, 4), (24, 4), (32, 4), (64, 2), (128, 1)];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeInput {
    pub name: String,
    pub len: usize,
    pub align_offset: usize,
    /// `bytes[align_offset..align_offset + len]` is the hashed slice.
    pub bytes: Vec<u8>,
}

impl ProbeInput {
    #[must_use]
    pub fn slice(&self) -> &[u8] {
        &self.bytes[self.align_offset..self.align_offset + self.len]
    }
}

/// splitmix64 — deterministic, portable, endian-fixed fill.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fill(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let word = splitmix64(&mut state).to_le_bytes();
        let take = (len - out.len()).min(8);
        out.extend_from_slice(&word[..take]);
    }
    out
}

/// The full deterministic corpus: every size at every alignment offset.
#[must_use]
pub fn corpus(seed: u64) -> Vec<ProbeInput> {
    let mut out = Vec::with_capacity(SIZES.len() * ALIGN_OFFSETS.len());
    for &len in &SIZES {
        for &offset in &ALIGN_OFFSETS {
            let payload = fill(seed ^ (len as u64), len);
            let mut bytes = vec![0u8; offset];
            bytes.extend_from_slice(&payload);
            out.push(ProbeInput {
                name: format!("len{len}+off{offset}"),
                len,
                align_offset: offset,
                bytes,
            });
        }
    }
    out
}

/// The mixed short-fact stream: `count` inputs drawn from
/// [`MIXTURE_WEIGHTS`], deterministic in the seed.
///
/// # Panics
/// On a weight-table invariant violation (draws are bounded by the total).
#[must_use]
pub fn mixture(seed: u64, count: usize) -> Vec<ProbeInput> {
    let total: u32 = MIXTURE_WEIGHTS.iter().map(|(_, w)| *w).sum();
    let mut state = seed ^ 0x4D49_5854_5552_4531; // "MIXTURE1"
    (0..count)
        .map(|index| {
            let draw = u32::try_from(splitmix64(&mut state) % u64::from(total)).expect("bounded");
            let mut cursor = 0u32;
            let mut chosen = MIXTURE_WEIGHTS[0].0;
            for &(len, weight) in &MIXTURE_WEIGHTS {
                cursor += weight;
                if draw < cursor {
                    chosen = len;
                    break;
                }
            }
            ProbeInput {
                name: format!("mix{index}-len{chosen}"),
                len: chosen,
                align_offset: 0,
                bytes: fill(splitmix64(&mut state), chosen),
            }
        })
        .collect()
}

/// Streaming split schedules for one input length: one-shot, a tiny head then
/// the rest, uneven small chunks, and 64 KiB chunks (checkpoint streaming
/// shape). Every schedule sums to `len`; equivalence with the one-shot digest
/// is a hard HASH-01 requirement for both candidates.
#[must_use]
pub fn split_schedules(len: usize) -> Vec<Vec<usize>> {
    let mut schedules = vec![vec![len]];
    if len >= 2 {
        schedules.push(vec![1, len - 1]);
    }
    if len >= 17 {
        schedules.push(vec![7, 9, len - 16]);
    }
    if len > 64 * 1024 {
        let chunk = 64 * 1024;
        let mut plan = Vec::with_capacity(len / chunk + 1);
        let mut rest = len;
        while rest > 0 {
            let take = rest.min(chunk);
            plan.push(take);
            rest -= take;
        }
        schedules.push(plan);
    }
    schedules
}
