//! SPACE-01: the live-byte census over a populated store.
//!
//! Four distinct measurements, reported as themselves and never conflated:
//!
//! 1. **Live key/value bytes per namespace** — walked entry by entry through
//!    the [`CensusSource`] the F3 wiring provides over the successor store's
//!    read snapshot (C04; the walker itself is a P02 interface request — see
//!    the P14 packet file). Chapter 41's arithmetic model predicts these
//!    numbers, so model-versus-walk divergence is itself a finding.
//! 2. **LMDB page statistics** — page size, depth, branch/leaf/overflow
//!    pages, entries and freelist pages, store-wide. Mixed namespaces share
//!    pages; there is no honest exact per-namespace page attribution without
//!    an actual method, so none is invented.
//! 3. **File length** (`data.mdb` stat — the existing storage-lane measure).
//! 4. **OS-allocated blocks** — `st_blocks * 512` on Unix, which differs from
//!    file length for sparse files. This is the measure that keeps a sparse
//!    fixture from impersonating populated data (also used by
//!    [`crate::largefix`]).
//!
//! Resident memory (images, tries, plans, results) is a separate axis owned
//! by the heap lane and the APP-TENANTS cells; a census of `data.mdb` says
//! nothing about RSS and is never reported as if it did.

use std::path::Path;

use super::{NAMESPACES, Namespace};

/// One walked entry: which namespace, and its exact key/value byte lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntrySize {
    pub namespace: Namespace,
    pub key_bytes: u64,
    pub value_bytes: u64,
}

/// What the F3 wiring implements over one coherent read snapshot of the
/// successor store. Implementations must walk **live** entries only (one
/// snapshot, no dirty pages) and classify by the successor's actual tag
/// roster. Requested from P02 as part of C04's snapshot cursor handoff.
pub trait CensusSource {
    /// # Errors
    fn walk(&mut self, visit: &mut dyn FnMut(EntrySize)) -> Result<(), String>;

    /// LMDB statistics for the whole environment: `(page_size, depth,
    /// branch_pages, leaf_pages, overflow_pages, entries, free_pages)`.
    ///
    /// # Errors
    fn page_stats(&mut self) -> Result<PageStats, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageStats {
    pub page_size: u64,
    pub depth: u64,
    pub branch_pages: u64,
    pub leaf_pages: u64,
    pub overflow_pages: u64,
    pub entries: u64,
    pub free_pages: u64,
}

impl PageStats {
    /// Bytes held by used pages (branch + leaf + overflow).
    #[must_use]
    pub const fn used_page_bytes(&self) -> u64 {
        (self.branch_pages + self.leaf_pages + self.overflow_pages) * self.page_size
    }

    /// Bytes parked on the freelist — copy-on-write high water retained by
    /// the file but holding no live entry.
    #[must_use]
    pub const fn free_page_bytes(&self) -> u64 {
        self.free_pages * self.page_size
    }
}

/// Per-namespace accumulated census.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NamespaceCensus {
    pub entries: u64,
    pub key_bytes: u64,
    pub value_bytes: u64,
}

impl NamespaceCensus {
    #[must_use]
    pub const fn raw_bytes(&self) -> u64 {
        self.key_bytes + self.value_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreCensus {
    /// Indexed by [`NAMESPACES`] order.
    pub per_namespace: [NamespaceCensus; NAMESPACES.len()],
    pub pages: PageStats,
    pub file_bytes: u64,
    pub allocated_bytes: u64,
}

impl StoreCensus {
    /// # Panics
    /// Never: `NAMESPACES` is total over [`Namespace`].
    #[must_use]
    pub fn namespace(&self, namespace: Namespace) -> NamespaceCensus {
        let index = NAMESPACES
            .iter()
            .position(|&n| n == namespace)
            .expect("NAMESPACES is total");
        self.per_namespace[index]
    }

    /// Sum of live raw key/value bytes across namespaces. Always ≤ used page
    /// bytes; the difference is node/slot/page overhead plus fill slack —
    /// the number the report attributes explicitly instead of hiding.
    #[must_use]
    pub fn live_raw_bytes(&self) -> u64 {
        self.per_namespace
            .iter()
            .map(NamespaceCensus::raw_bytes)
            .sum()
    }

    /// Page overhead + occupancy slack: used page bytes minus live raw bytes
    /// (saturating: an inconsistent walk must not underflow into nonsense).
    #[must_use]
    pub fn page_overhead_bytes(&self) -> u64 {
        self.pages
            .used_page_bytes()
            .saturating_sub(self.live_raw_bytes())
    }
}

/// Run the census: walk entries, take page stats, stat the file, read the
/// allocated blocks.
///
/// # Errors
/// # Panics
pub fn run(source: &mut dyn CensusSource, data_file: &Path) -> Result<StoreCensus, String> {
    let mut per_namespace = [NamespaceCensus::default(); NAMESPACES.len()];
    source.walk(&mut |entry| {
        let index = NAMESPACES
            .iter()
            .position(|&n| n == entry.namespace)
            .expect("NAMESPACES is total");
        let cell = &mut per_namespace[index];
        cell.entries += 1;
        cell.key_bytes += entry.key_bytes;
        cell.value_bytes += entry.value_bytes;
    })?;
    let pages = source.page_stats()?;
    let file_bytes = std::fs::metadata(data_file)
        .map(|meta| meta.len())
        .map_err(|e| format!("stat {}: {e}", data_file.display()))?;
    let allocated_bytes = allocated_bytes(data_file)?;
    Ok(StoreCensus {
        per_namespace,
        pages,
        file_bytes,
        allocated_bytes,
    })
}

/// OS-allocated bytes: `st_blocks` are 512-byte units on Unix regardless of
/// the filesystem block size. On non-Unix targets this returns the file
/// length with an explicit note — never a silently different meaning.
///
/// # Errors
pub fn allocated_bytes(path: &Path) -> Result<u64, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        std::fs::metadata(path)
            .map(|meta| meta.blocks() * 512)
            .map_err(|e| format!("stat {}: {e}", path.display()))
    }
    #[cfg(not(unix))]
    {
        Err(format!(
            "allocated-block accounting is Unix-only; {} cannot be measured on this target \
             (record NotApplicable, not a length)",
            path.display()
        ))
    }
}
