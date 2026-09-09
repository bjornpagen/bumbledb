use super::{Dir, OverlapCache};
use crate::alloc_counter::{AllocWindow, snapshot};
use std::cell::Cell;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProbeCosts {
    pub calls: u64,
    pub allocs: u64,
    pub deallocs: u64,
    pub alloc_bytes: u64,
    pub dealloc_bytes: u64,
}

thread_local! {
    static COSTS: Cell<ProbeCosts> = Cell::new(ProbeCosts::default());
    static QUERIES: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

pub(crate) fn reset_costs() {
    COSTS.set(ProbeCosts::default());
    QUERIES.set((0, 0));
}

pub(crate) fn record_query(len: usize) {
    let (flat, tree) = QUERIES.get();
    QUERIES.set(if len <= super::FLAT_SWEEP_CEILING {
        (flat + 1, tree)
    } else {
        (flat, tree + 1)
    });
}

pub(crate) fn query_counts() -> (u64, u64) {
    QUERIES.get()
}

pub(crate) fn costs() -> ProbeCosts {
    COSTS.get()
}

pub(crate) struct ProbeAllocationWindow(AllocWindow);

impl ProbeAllocationWindow {
    pub(crate) fn begin() -> Self {
        Self(snapshot().window)
    }
}

impl Drop for ProbeAllocationWindow {
    fn drop(&mut self) {
        let after = snapshot().window;
        COSTS.with(|cell| {
            let mut sum = cell.get();
            sum.calls += 1;
            sum.allocs += after.allocs - self.0.allocs;
            sum.deallocs += after.deallocs - self.0.deallocs;
            sum.alloc_bytes += after.alloc_bytes - self.0.alloc_bytes;
            sum.dealloc_bytes += after.dealloc_bytes - self.0.dealloc_bytes;
            cell.set(sum);
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheSnapshot {
    pub owners: Vec<(&'static str, usize, usize, usize)>,
    pub tallied: usize,
    pub groups: Vec<(Vec<u64>, usize, usize, usize)>,
}

impl OverlapCache {
    pub(crate) fn saved_structure_snapshot(&self) -> CacheSnapshot {
        let owners = vec![
            (
                "table",
                self.table.len(),
                self.table.capacity(),
                size_of::<u32>(),
            ),
            (
                "dirs",
                self.dirs.len(),
                self.dirs.capacity(),
                size_of::<Dir>(),
            ),
            (
                "keys",
                self.keys.len(),
                self.keys.capacity(),
                size_of::<u64>(),
            ),
            (
                "starts",
                self.starts.len(),
                self.starts.capacity(),
                size_of::<u64>(),
            ),
            (
                "positions",
                self.positions.len(),
                self.positions.capacity(),
                size_of::<u32>(),
            ),
            (
                "tree",
                self.tree.len(),
                self.tree.capacity(),
                size_of::<u64>(),
            ),
            (
                "triples",
                self.triples.len(),
                self.triples.capacity(),
                size_of::<(u64, u64, u32)>(),
            ),
        ];
        let mut groups = Vec::new();
        let mut tallied = 0;
        for dir in &self.dirs {
            match *dir {
                Dir::Tallied { .. } => tallied += 1,
                Dir::Built {
                    key_start,
                    key_len,
                    len,
                    tree_base,
                    p,
                    ..
                } => {
                    groups.push((
                        self.keys[key_start as usize..(key_start + key_len) as usize].to_vec(),
                        len as usize,
                        tree_base as usize,
                        p.get() as usize,
                    ));
                }
            }
        }
        CacheSnapshot {
            owners,
            tallied,
            groups,
        }
    }
}
