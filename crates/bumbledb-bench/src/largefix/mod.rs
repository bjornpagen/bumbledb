//! APP-LARGE / G05 large-data fixtures: **authored now, populated only in
//! F3** (chapter 70 §G05; chapter 62 P02/P14 shared fixture obligation).
//!
//! Two distinct gates that must never be merged:
//!
//! 1. **Beyond-RAM** ([`BeyondRamPlan`]): the *workload's data* substantially
//!    exceeds resident memory, enforced with a real memory bound (Linux
//!    cgroup v2 `memory.max` on an isolated runner). An address-space limit
//!    is explicitly forbidden — it breaks legitimate sparse LMDB maps and
//!    proves nothing about residency.
//! 2. **Beyond-32-GiB populated store** ([`LargeStorePlan`]): the physical
//!    database file crosses the deleted 32 GiB ceiling with *actually
//!    populated* pages — [`enforce::assert_populated`] rejects sparse files
//!    and large empty maps by allocated-block accounting. The practical
//!    minimum fixture is > 40 GiB with an explicitly recorded smaller memory
//!    allowance, grown, reopened, mutated and checked on both sides of the
//!    boundary, then checkpointed/restored under bounded memory.
//!
//! The generator ([`generator`]) is streaming and resumable: O(1) memory per
//! chunk, deterministic in `(seed, chunk_index)`, with per-chunk checksums so
//! the exact-check oracle never needs the whole dataset in RAM.

pub mod enforce;
pub mod generator;
#[cfg(test)]
mod tests;

/// The deleted ceiling. Fixtures must comfortably clear it.
pub const FORMER_CEILING_BYTES: u64 = 32 << 30;

/// The chapter 70 minimum populated fixture.
pub const MIN_POPULATED_BYTES: u64 = 40 << 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LargeStorePlan {
    pub seed: u64,
    /// Target populated payload bytes (before store overhead) — at least
    /// [`MIN_POPULATED_BYTES`].
    pub target_payload_bytes: u64,
    /// Encoded row payload width the generator emits.
    pub row_bytes: u32,
    /// Rows per generated chunk (one chunk = one bounded commit).
    pub rows_per_chunk: u32,
    /// The explicitly recorded memory allowance the run must respect
    /// (RSS/cgroup), far below the data size.
    pub memory_allowance_bytes: u64,
}

impl LargeStorePlan {
    /// The default F3 plan: > 40 GiB payload from 4 KiB rows, populated in
    /// 4096-row (16 MiB) chunks, under an 8 GiB recorded allowance.
    #[must_use]
    pub const fn default_f3() -> Self {
        Self {
            seed: 1,
            target_payload_bytes: 41 << 30,
            row_bytes: 4096,
            rows_per_chunk: 4096,
            memory_allowance_bytes: 8 << 30,
        }
    }

    #[must_use]
    pub fn rows(&self) -> u64 {
        self.target_payload_bytes
            .div_ceil(u64::from(self.row_bytes))
    }

    #[must_use]
    pub fn chunks(&self) -> u64 {
        self.rows().div_ceil(u64::from(self.rows_per_chunk))
    }

    /// Plan-level admission: refuse a plan that cannot satisfy the gate
    /// before a single byte is generated.
    ///
    /// # Errors
    pub fn check(&self) -> Result<(), String> {
        if self.target_payload_bytes < MIN_POPULATED_BYTES {
            return Err(format!(
                "target {} B is below the {} B populated-fixture minimum",
                self.target_payload_bytes, MIN_POPULATED_BYTES
            ));
        }
        if self.target_payload_bytes <= FORMER_CEILING_BYTES {
            return Err("the fixture must cross the former 32 GiB ceiling".to_owned());
        }
        if self.memory_allowance_bytes >= self.target_payload_bytes / 4 {
            return Err(format!(
                "memory allowance {} B is not meaningfully below the data size {} B",
                self.memory_allowance_bytes, self.target_payload_bytes
            ));
        }
        if self.row_bytes == 0 || self.rows_per_chunk == 0 {
            return Err("degenerate row/chunk geometry".to_owned());
        }
        Ok(())
    }

    /// The mutation/check schedule around the boundary: rows referenced on
    /// both sides of the former ceiling, by chunk index. Deterministic so the
    /// F3 driver and the manifest agree on which chunks were touched.
    #[must_use]
    pub fn boundary_chunks(&self) -> (u64, u64) {
        let boundary_row = FORMER_CEILING_BYTES / u64::from(self.row_bytes);
        let boundary_chunk = boundary_row / u64::from(self.rows_per_chunk);
        (
            boundary_chunk.saturating_sub(1),
            (boundary_chunk + 1).min(self.chunks() - 1),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeyondRamPlan {
    pub seed: u64,
    /// The enforced memory bound (cgroup v2 `memory.max`).
    pub memory_max_bytes: u64,
    /// Dataset payload as a multiple of the bound — at least
    /// [`Self::MIN_DATA_MULTIPLE`].
    pub data_multiple: u32,
    pub row_bytes: u32,
    pub rows_per_chunk: u32,
}

impl BeyondRamPlan {
    pub const MIN_DATA_MULTIPLE: u32 = 4;

    /// The default F3 plan: 2 GiB memory.max, 8× data.
    #[must_use]
    pub const fn default_f3() -> Self {
        Self {
            seed: 1,
            memory_max_bytes: 2 << 30,
            data_multiple: 8,
            row_bytes: 512,
            rows_per_chunk: 32_768,
        }
    }

    #[must_use]
    pub const fn target_payload_bytes(&self) -> u64 {
        self.memory_max_bytes * self.data_multiple as u64
    }

    #[must_use]
    pub fn rows(&self) -> u64 {
        self.target_payload_bytes()
            .div_ceil(u64::from(self.row_bytes))
    }

    /// # Errors
    pub fn check(&self) -> Result<(), String> {
        if self.data_multiple < Self::MIN_DATA_MULTIPLE {
            return Err(format!(
                "data must be at least {}x the memory bound, got {}x",
                Self::MIN_DATA_MULTIPLE,
                self.data_multiple
            ));
        }
        if self.memory_max_bytes == 0 || self.row_bytes == 0 || self.rows_per_chunk == 0 {
            return Err("degenerate plan geometry".to_owned());
        }
        Ok(())
    }
}

/// What each large lane must record (chapter 70 G05): correctness plus the
/// resource envelope, never only a successful open.
pub const REQUIRED_OBSERVATIONS: [&str; 9] = [
    "page-faults",
    "rss-or-cgroup-peak",
    "file-length",
    "allocated-blocks",
    "virtual-map-size",
    "io-bytes",
    "result-correctness-streaming-oracle",
    "cancellation-latency",
    "temporary-disk-cleanup",
];
