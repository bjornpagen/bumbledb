//! Census walker over a live [`bumbledb::store::OwnedSnapshot`].
//!
//! Separates live payload, pages, free blocks, map extent and allocated
//! disk. RSS and image/cache retention are not this walk.

use std::path::Path;

use bumbledb::store::{MapReport, OwnedSnapshot, StorePageStats};
use bumbledb::{WorkContext, start_operation};

use super::Namespace;
use super::census::{CensusSource, EntrySize, PageStats};
use crate::harness::bench_policy;

/// One coherent snapshot plus the map report taken with it.
pub struct StoreCensusSource<'a> {
    snapshot: &'a OwnedSnapshot,
    work: &'a WorkContext,
    map: MapReport,
}

impl<'a> StoreCensusSource<'a> {
    /// # Errors
    /// Storage or stopped work.
    pub fn open(snapshot: &'a OwnedSnapshot, work: &'a WorkContext, map: MapReport) -> Self {
        Self {
            snapshot,
            work,
            map,
        }
    }

    #[must_use]
    pub fn map(&self) -> MapReport {
        self.map
    }
}

impl CensusSource for StoreCensusSource<'_> {
    fn walk(&mut self, visit: &mut dyn FnMut(EntrySize)) -> Result<(), String> {
        self.snapshot
            .entry_census(self.work, &mut |is_meta, tag, key_len, value_len| {
                visit(EntrySize {
                    namespace: Namespace::from_census_tag(is_meta, tag),
                    key_bytes: key_len as u64,
                    value_bytes: value_len as u64,
                });
                Ok(())
            })
            .map_err(|error| format!("entry census: {error:?}"))
    }

    fn page_stats(&mut self) -> Result<PageStats, String> {
        let stats = self
            .snapshot
            .page_stats()
            .map_err(|error| format!("page stats: {error:?}"))?;
        Ok(from_store_pages(stats))
    }
}

#[must_use]
pub fn from_store_pages(stats: StorePageStats) -> PageStats {
    PageStats {
        page_size: stats.page_size,
        depth: stats.depth,
        branch_pages: stats.branch_pages,
        leaf_pages: stats.leaf_pages,
        overflow_pages: stats.overflow_pages,
        entries: stats.entries,
        free_pages: stats.free_pages,
    }
}

/// Distinct physical quantities for one store. Never collapse these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSplit {
    pub live_raw_bytes: u64,
    pub used_page_bytes: u64,
    pub free_page_bytes: u64,
    pub populated_file_bytes: u64,
    pub allocated_disk_bytes: Option<u64>,
    pub virtual_map_bytes: u64,
    pub live_transactions: u64,
}

impl PhysicalSplit {
    #[must_use]
    pub fn from_parts(
        live_raw_bytes: u64,
        pages: PageStats,
        map: MapReport,
        allocated_disk_bytes: Option<u64>,
    ) -> Self {
        Self {
            live_raw_bytes,
            used_page_bytes: pages.used_page_bytes(),
            free_page_bytes: pages.free_page_bytes(),
            populated_file_bytes: map.populated_file_bytes,
            allocated_disk_bytes,
            virtual_map_bytes: map.virtual_map_bytes,
            live_transactions: map.live_transactions,
        }
    }
}

/// Mint a census work context. Not a product default.
/// # Errors
pub fn census_work() -> Result<WorkContext, String> {
    start_operation(bench_policy()).map_err(|error| format!("census work: {error:?}"))
}

/// `data.mdb` beside a store directory.
#[must_use]
pub fn data_mdb(store_dir: &Path) -> std::path::PathBuf {
    store_dir.join("data.mdb")
}
