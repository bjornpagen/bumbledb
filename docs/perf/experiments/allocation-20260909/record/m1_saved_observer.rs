use super::Colt;
use std::cell::Cell;
use std::collections::BTreeMap;

thread_local! {
    static CLONES: Cell<(usize, usize, usize)> = const { Cell::new((0, 0, 0)) };
}

pub(crate) fn reset_clones() {
    CLONES.set((0, 0, 0));
}

pub(crate) fn clones() -> (usize, usize, usize) {
    CLONES.get()
}

pub(crate) fn record_clone(colt: &Colt) {
    let arena = colt.ctrl.len() + 8 * colt.buckets.len() + 4 * colt.dense.len();
    let live: usize = colt.maps.iter().map(|m| {
        8 * m.nbuckets + 8 * m.nbuckets * m.stride() + 4 * m.len as usize
    }).sum();
    let retired = arena.checked_sub(live).unwrap();
    CLONES.update(|(count, bytes, dead)| (count + 1, bytes + arena, dead + retired));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub rows: usize,
    pub map_count: usize,
    pub owners: [(usize, usize); 3],
    pub table_bytes: usize,
    pub arena_bytes: usize,
    pub capacity_bytes: usize,
    pub all_pool_retained: usize,
    pub histogram: BTreeMap<(usize, usize, usize), usize>,
}

impl Colt {
    pub(crate) fn m1_saved_snapshot(&self) -> Snapshot {
        let mut ranges: [Vec<(usize, usize)>; 3] = std::array::from_fn(|_| Vec::new());
        let owners = [(self.ctrl.len(), self.ctrl.capacity()),
                      (self.buckets.len(), self.buckets.capacity()),
                      (self.dense.len(), self.dense.capacity())];
        let mut table_bytes = 0;
        let mut histogram = BTreeMap::new();
        for m in &self.maps {
            let counts = [8 * m.nbuckets, m.nbuckets * m.stride(), m.len as usize];
            let starts = [m.ctrl_start, m.bucket_start, m.dense_start];
            for i in 0..3 {
                assert!(starts[i] + counts[i] <= owners[i].0);
                if counts[i] != 0 {
                    ranges[i].push((starts[i], starts[i] + counts[i]));
                }
            }
            table_bytes += counts[0] + 8 * counts[1] + 4 * counts[2];
            *histogram.entry((m.arity, m.nbuckets, m.len as usize)).or_default() += 1;
        }
        for ranges in &mut ranges {
            ranges.sort_unstable();
            assert!(ranges.windows(2).all(|pair| pair[0].1 <= pair[1].0));
        }
        let arena_bytes = owners[0].0 + 8 * owners[1].0 + 4 * owners[2].0;
        assert!(arena_bytes >= table_bytes);
        Snapshot {
            rows: self.view.len(), map_count: self.maps.len(), owners, table_bytes,
            arena_bytes, capacity_bytes: owners[0].1 + 8 * owners[1].1 + 4 * owners[2].1,
            all_pool_retained: self.retained_bytes(), histogram,
        }
    }
}
