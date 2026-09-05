//! Elastic map sizing. A mapped address range is not resident RAM and not a
//! database-size limit; there is no 32 GiB policy constant anywhere in the
//! successor store. The map starts with generous virtual headroom (resize is
//! ordinary but uncommon) and grows geometrically, page-aligned, under the
//! exclusive transaction gate.
//!
//! Virtual map extent, populated file bytes, live b-tree pages and allocated
//! disk blocks are separately reported quantities ([`MapReport`]); none of
//! them is a RAM admission test.

/// Growth/rounding alignment: a multiple of every supported page size
/// (4 KiB, 16 KiB, 64 KiB), so `mdb_env_set_mapsize` always receives a
/// page-aligned value.
pub(crate) const MAP_ALIGN: u64 = 1 << 20;

/// Default initial virtual extent: 4 GiB. Sparse virtual reservation on
/// 64-bit targets; the populated file grows lazily from zero.
const DEFAULT_INITIAL_MAP: u64 = 4 << 30;

/// Map growth policy. These are backend/tuning parameters, not semantic
/// database-size limits; `max_map_bytes` exists for hosts with explicit
/// address-space budgets and defaults to unbounded (platform-limited).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPolicy {
    /// Virtual extent used at open when the populated file is smaller.
    pub initial_map_bytes: u64,
    /// Optional hard ceiling on the virtual map. `None` means the platform
    /// address space is the only limit. This is a host budget, never a
    /// product default.
    pub max_map_bytes: Option<u64>,
}

impl Default for MapPolicy {
    fn default() -> Self {
        Self {
            initial_map_bytes: DEFAULT_INITIAL_MAP,
            max_map_bytes: None,
        }
    }
}

impl MapPolicy {
    /// The map extent to request at open: the configured initial extent, or
    /// the existing populated file plus one geometric step of headroom,
    /// whichever is larger. Page-aligned.
    pub(crate) fn open_map_bytes(&self, populated_file_bytes: u64) -> u64 {
        let mut wanted = self.initial_map_bytes.max(MAP_ALIGN);
        if populated_file_bytes > wanted / 2 {
            wanted = wanted.max(populated_file_bytes.saturating_mul(2));
        }
        let aligned = align_up(wanted);
        match self.max_map_bytes {
            // Never shrink below the populated file: LMDB refuses to map a
            // file larger than the map. A misconfigured ceiling surfaces as
            // an open failure, not silent truncation.
            Some(ceiling) => aligned
                .min(align_up(ceiling))
                .max(align_up(populated_file_bytes)),
            None => aligned,
        }
    }

    /// The next geometric extent after `current`, at least covering
    /// `needed_hint` when one is known. `None` when no growth is possible
    /// within the policy/platform bounds — the typed
    /// `MapGrowthExhausted` refusal, never a wrap or silent clamp to the
    /// current size.
    pub(crate) fn grown_map_bytes(&self, current: u64, needed_hint: Option<u64>) -> Option<u64> {
        let doubled = current.checked_mul(2)?;
        let wanted = align_up(doubled.max(needed_hint.unwrap_or(0)));
        match self.max_map_bytes {
            Some(ceiling) => {
                let ceiling = align_up(ceiling);
                if current >= ceiling {
                    None
                } else {
                    Some(wanted.min(ceiling))
                }
            }
            None => Some(wanted),
        }
    }
}

pub(crate) fn align_up(bytes: u64) -> u64 {
    bytes
        .checked_add(MAP_ALIGN - 1)
        .map_or(u64::MAX - (u64::MAX % MAP_ALIGN), |n| n - (n % MAP_ALIGN))
}

/// Distinct physical quantities, reported separately (chapter 31). Mixed
/// namespaces share pages; no fictional per-namespace page attribution is
/// invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapReport {
    /// Current virtual map extent (`mdb_env_info.me_mapsize`).
    pub virtual_map_bytes: u64,
    /// Length of `data.mdb` — populated file bytes, possibly sparse.
    pub populated_file_bytes: u64,
    /// Live branch/leaf/overflow pages across the store databases.
    pub non_free_page_bytes: u64,
    /// Actually allocated disk blocks where the platform reports them
    /// (POSIX `st_blocks`); `None` elsewhere. Distinguishes sparse virtual
    /// reservation from consumed disk.
    pub allocated_disk_bytes: Option<u64>,
    /// LMDB page size for this environment.
    pub page_size: u32,
    /// Live gated transactions (snapshots and the writer) at report time.
    pub live_transactions: u64,
}
