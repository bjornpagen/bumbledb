//! Read-only occupancy classification over the actual essential-table arena.
//! This is a raw finite-binary prototype, not a public scoped Event API.
use super::essential_raw as raw;
use raw::{Arena, Ref, View};
use rustc_hash::FxHashMap;
#[path = "occupancy_words.rs"]
mod words;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel {
    Scalar,
    Words,
}
impl Kernel {
    pub fn from_env() -> Self {
        match std::env::var("EVENT_LAB_OCCUPANCY_KERNEL").as_deref() {
            Ok("scalar") | Err(_) => Self::Scalar,
            Ok("words") => Self::Words,
            _ => panic!("unknown occupancy kernel"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Restricted {
    root: Ref,
    pinned: u64,
    values: u64,
}
impl Restricted {
    fn new(root: Ref) -> Self {
        Self {
            root,
            pinned: 0,
            values: 0,
        }
    }
    fn remaining<const K: u32>(self, arena: &Arena<K>) -> u64 {
        arena.variables(self.root) & !self.pinned
    }
    fn cofactor<const K: u32>(self, arena: &Arena<K>, variable: u32, high: bool) -> Self {
        let bit = 1u64 << variable;
        if self.remaining(arena) & bit == 0 {
            return self;
        }
        match arena.view(self.root) {
            View::Constant(_) => unreachable!(),
            View::Branch {
                variable: top,
                low,
                high: upper,
            } => {
                // The chosen coordinate is earliest in the manager's order.
                assert_eq!(
                    variable, top,
                    "classifier must use the arena's physical order"
                );
                assert_eq!(self.pinned, 0);
                Self::new(if high { upper } else { low })
            }
            View::Table { .. } => {
                let next = Self {
                    root: self.root,
                    pinned: self.pinned | bit,
                    values: self.values | if high { bit } else { 0 },
                };
                if next.remaining(arena) == 0 {
                    Self::new(arena.evaluate(next.root, next.values) as Ref)
                } else {
                    next
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct Observation {
    pub signature: u8,
    pub subproblems: usize,
    pub local_assignments: usize,
    pub local_cubes: usize,
    pub word_batches: usize,
    pub borrowed_planes: usize,
    pub aligned_planes: usize,
    pub constant_planes: usize,
    pub alignment_allocations: usize,
    pub memo_entries: usize,
}

/// Classify raw predicates inside support. `order` is the arena's fixed order.
/// Nodes are borrowed immutably; memoized views are temporary, never Event IDs.
/// The empty-support result is zero at this raw boundary; scoped publication
/// separately requires a nonempty admitted universe.
pub fn classify<const K: u32>(
    arena: &Arena<K>,
    order: &[u32],
    support: Ref,
    a: Ref,
    b: Ref,
) -> Observation {
    classify_with(Kernel::from_env(), arena, order, support, a, b)
}

pub fn classify_with<const K: u32>(
    kernel: Kernel,
    arena: &Arena<K>,
    order: &[u32],
    support: Ref,
    a: Ref,
    b: Ref,
) -> Observation {
    fn visit<const K: u32, const WORDS: bool>(
        arena: &Arena<K>,
        order: &[u32],
        views: [Restricted; 3],
        memo: &mut FxHashMap<[Restricted; 3], u8>,
        stats: &mut Observation,
    ) -> u8 {
        if views[0].root == 0 {
            return 0;
        }
        if let Some(&signature) = memo.get(&views) {
            return signature;
        }
        stats.subproblems += 1;
        let variables = views.iter().fold(0, |mask, v| mask | v.remaining(arena));
        let signature = if variables.count_ones() <= K {
            stats.local_cubes += 1;
            if WORDS {
                words::classify(arena, views, variables, stats)
            } else {
                // Retained per-assignment control; it creates no resident nodes.
                let axes = raw::axes(variables);
                let mut found = 0;
                for assignment in 0..(1usize << axes.len()) {
                    stats.local_assignments += 1;
                    let world = axes
                        .iter()
                        .enumerate()
                        .fold(0, |w, (i, &v)| w | (((assignment >> i) & 1) as u64) << v);
                    let truth = views.map(|v| arena.evaluate(v.root, world | v.values));
                    if truth[0] {
                        found |= 1 << (2 * truth[1] as u8 + truth[2] as u8);
                        if found == 15 {
                            break;
                        }
                    }
                }
                found
            }
        } else {
            let variable = *order.iter().find(|&&v| variables >> v & 1 != 0).unwrap();
            let low = visit::<K, WORDS>(
                arena,
                order,
                views.map(|v| v.cofactor(arena, variable, false)),
                memo,
                stats,
            );
            if low == 15 {
                low
            } else {
                low | visit::<K, WORDS>(
                    arena,
                    order,
                    views.map(|v| v.cofactor(arena, variable, true)),
                    memo,
                    stats,
                )
            }
        };
        // Every completed result is exact. A full signature is exact even if
        // exploration stopped early; no other partial signature is cached.
        memo.insert(views, signature);
        signature
    }
    let mut memo = FxHashMap::default();
    let mut result = Observation::default();
    let inputs = [support, a, b].map(Restricted::new);
    result.signature = match kernel {
        Kernel::Scalar => visit::<K, false>(arena, order, inputs, &mut memo, &mut result),
        Kernel::Words => visit::<K, true>(arena, order, inputs, &mut memo, &mut result),
    };
    result.memo_entries = memo.len();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn both<const K: u32>(
        arena: &Arena<K>,
        order: &[u32],
        support: Ref,
        a: Ref,
        b: Ref,
    ) -> Observation {
        let scalar = classify_with(Kernel::Scalar, arena, order, support, a, b);
        let words = classify_with(Kernel::Words, arena, order, support, a, b);
        assert_eq!(scalar.signature, words.signature);
        assert_eq!(scalar.subproblems, words.subproblems);
        assert_eq!(scalar.memo_entries, words.memo_entries);
        assert_eq!(scalar.local_cubes, words.local_cubes);
        assert_eq!(words.local_assignments, 0);
        assert_eq!(scalar.word_batches, 0);
        words
    }
    fn exhaustive<const K: u32>() {
        let mut seen = 0u16;
        for order in [vec![0, 1], vec![1, 0]] {
            let mut c = Arena::<K>::new(2, &order);
            let roots: Vec<_> = (0..16).map(|word| c.import(vec![word])).collect();
            let before = (c.nodes(), c.bytes());
            for support in 1..16usize {
                for a in 0..16usize {
                    for b in 0..16usize {
                        let expected =
                            (0..4).filter(|w| support >> w & 1 != 0).fold(0, |sig, w| {
                                sig | (1 << (2 * ((a >> w) & 1) + ((b >> w) & 1)))
                            });
                        let sig = both(&c, &order, roots[support], roots[a], roots[b]).signature;
                        assert_eq!(sig, expected);
                        seen |= 1 << sig;
                        let swapped = (sig & 9) | ((sig & 2) << 1) | ((sig & 4) >> 1);
                        assert_eq!(
                            both(&c, &order, roots[support], roots[b], roots[a]).signature,
                            swapped
                        );
                        assert_eq!(
                            both(&c, &order, roots[support], roots[a] ^ 1, roots[b]).signature,
                            ((sig << 2) | (sig >> 2)) & 15
                        );
                        assert_eq!(
                            both(&c, &order, roots[support], roots[a], roots[b] ^ 1).signature,
                            ((sig & 5) << 1) | ((sig & 10) >> 1)
                        );
                    }
                }
            }
            assert_eq!((c.nodes(), c.bytes()), before);
            // Outside support is not a witness for the all-false cell.
            assert_eq!(both(&c, &order, roots[1], roots[1], roots[1]).signature, 8);
        }
        assert_eq!(
            seen, 0xfffe,
            "all fifteen nonempty-support classes are exercised"
        );
    }
    #[test]
    fn exact_signatures_and_symmetries_with_no_resident_growth() {
        exhaustive::<1>();
        exhaustive::<2>();
    }
    fn scattered<const K: u32>() {
        let axes = [0, 3, 8, 16, 23, 29, 37, 42, 55, 59];
        let mask = axes.iter().fold(0, |m, &v| m | (1u64 << v));
        for order in [(0..60).collect::<Vec<_>>(), (0..60).rev().collect()] {
            let mut c = Arena::<K>::new(60, &order);
            let mut tables = [vec![0u64; 16], vec![0u64; 16], vec![0u64; 16]];
            let mut expected = 0;
            for i in 0..1024usize {
                let truth = [
                    i % 5 != 0,
                    (i * 331 + 17) % 13 < 6,
                    (i * 331 + 17) % 13 < 6 && i % 7 < 4,
                ];
                for (t, bit) in tables.iter_mut().zip(truth) {
                    if bit {
                        t[i / 64] |= 1u64 << (i % 64);
                    }
                }
                if truth[0] {
                    expected |= 1 << (2 * truth[1] as u8 + truth[2] as u8);
                }
            }
            let roots = tables.map(|t| c.table(mask, t));
            let before = (c.nodes(), c.bytes());
            assert_eq!(expected, 13); // B is a proper nonempty subset of A.
            let result = both(&c, &order, roots[0], roots[1], roots[2]);
            assert_eq!(result.signature, expected);
            assert!(result.subproblems > 1);
            println!(
                "OCCUPANCY_WORDS K={K} order_first={} stats={result:?}",
                order[0]
            );
            assert_eq!((c.nodes(), c.bytes()), before);
        }
    }
    #[test]
    fn noncontiguous_sixty_coordinate_support_and_views() {
        scattered::<6>();
        scattered::<9>();
    }
}
