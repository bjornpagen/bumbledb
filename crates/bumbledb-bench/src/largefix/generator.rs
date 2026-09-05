//! The streaming, resumable large-fixture generator and its streaming oracle.
//!
//! Determinism law: row `r` of chunk `c` under seed `s` is a pure function of
//! `(s, c, r)`. Population can therefore stop and resume at any chunk
//! boundary, and verification re-derives any chunk independently — the oracle
//! never loads the dataset, it re-generates and compares streams.
//!
//! Row shape: `key = chunk * rows_per_chunk + row` (little-endian, first 8
//! payload bytes) followed by splitmix-filled bytes. The embedded key makes
//! every row self-identifying so a wrong-chunk read is caught by content, not
//! only by count. Mutated rows flip a marker byte through
//! [`mutated_row`] — the F3 boundary schedule replaces, then verifies, rows
//! on both sides of the former 32 GiB ceiling.

/// splitmix64 — local, portable, no production dependence.
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The global row key for `(chunk, row)`.
#[must_use]
pub const fn row_key(chunk: u64, rows_per_chunk: u32, row: u32) -> u64 {
    chunk * rows_per_chunk as u64 + row as u64
}

/// One deterministic row payload of `row_bytes` length.
#[must_use]
pub fn row_payload(
    seed: u64,
    chunk: u64,
    rows_per_chunk: u32,
    row: u32,
    row_bytes: u32,
) -> Vec<u8> {
    let key = row_key(chunk, rows_per_chunk, row);
    let mut state = seed ^ key.rotate_left(17) ^ 0x4C41_5247_4546_4958; // "LARGEFIX"
    let len = row_bytes as usize;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&key.to_le_bytes()[..len.min(8)]);
    while out.len() < len {
        let word = next(&mut state).to_le_bytes();
        let take = (len - out.len()).min(8);
        out.extend_from_slice(&word[..take]);
    }
    out
}

/// The mutation applied by the boundary schedule: same key, marker byte
/// flipped at the last position — a replacement, never an append, so the
/// store's post-mutation read is distinguishable from the original.
#[must_use]
pub fn mutated_row(mut original: Vec<u8>) -> Vec<u8> {
    if let Some(last) = original.last_mut() {
        *last ^= 0xFF;
    }
    original
}

/// Stream one chunk's rows through `emit`, `O(rows_per_chunk` × `row_bytes`)
/// transient memory, no accumulation.
pub fn stream_chunk(
    seed: u64,
    chunk: u64,
    rows_per_chunk: u32,
    row_bytes: u32,
    total_rows: u64,
    emit: &mut dyn FnMut(u64, Vec<u8>),
) {
    for row in 0..rows_per_chunk {
        let key = row_key(chunk, rows_per_chunk, row);
        if key >= total_rows {
            return;
        }
        emit(
            key,
            row_payload(seed, chunk, rows_per_chunk, row, row_bytes),
        );
    }
}

/// Per-chunk checksum for the streaming oracle: BLAKE3 over
/// `key || payload` in row order. Verification recomputes this from the
/// store's read stream and from the generator independently; equality per
/// chunk is the exact check, and no step holds more than one chunk.
#[must_use]
pub fn chunk_checksum(
    seed: u64,
    chunk: u64,
    rows_per_chunk: u32,
    row_bytes: u32,
    total_rows: u64,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    stream_chunk(
        seed,
        chunk,
        rows_per_chunk,
        row_bytes,
        total_rows,
        &mut |key, payload| {
            hasher.update(&key.to_le_bytes());
            hasher.update(&payload);
        },
    );
    *hasher.finalize().as_bytes()
}

/// Fold a stream of `(key, payload)` rows into the same checksum shape as
/// [`chunk_checksum`] — used on the store-read side of the oracle.
pub struct StreamChecksum {
    hasher: blake3::Hasher,
    rows: u64,
}

impl Default for StreamChecksum {
    fn default() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
            rows: 0,
        }
    }
}

impl StreamChecksum {
    pub fn push(&mut self, key: u64, payload: &[u8]) {
        self.hasher.update(&key.to_le_bytes());
        self.hasher.update(payload);
        self.rows += 1;
    }

    #[must_use]
    pub fn finish(self) -> ([u8; 32], u64) {
        (*self.hasher.finalize().as_bytes(), self.rows)
    }
}
