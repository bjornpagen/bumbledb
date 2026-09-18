//! Work counters, not timings. Independent raw replay of unique product inputs.
use event_essential_prototype::{Arena, ViewProductStatistics};
use std::collections::BTreeSet;

fn run<const K: u32>(width: u32, layout: &str, fused: bool) {
    let dimensions = 3 * width;
    let chunk = match layout { "face-major" => width, "pair-major" => 2, _ => 1 };
    let mut order = vec![];
    for base in (0..width).step_by(chunk as usize).rev() {
        for face in (0..3).rev() {
            for bit in (base..(base + chunk).min(width)).rev() {
                order.push(face * width + bit);
            }
        }
    }
    let mut input = Arena::<K>::new(dimensions, &order);
    let size = 1usize << width;
    let mut bank = vec![];
    for family in 0..2 {
        for i in 0..8 {
            let mut data = vec![0; (size * size).div_ceil(64)];
            for y in 0..size {
                for x in 0..size {
                    let block = size.min(8);
                    let yes = if family == 0 {
                        x % 7 != i % 7 && x / block == y / block
                            && (y % block == (x + i) % block || y % block == (x + i + 1) % block)
                    } else {
                        x / block == y / block || ((x + i) % size < size / 4 && y % 5 != i % 5)
                    };
                    let cell = x | y << width;
                    if yes { data[cell / 64] |= 1 << (cell % 64); }
                }
            }
            bank.push(input.table((1 << (2 * width)) - 1, data));
        }
    }
    // The clover fixture has 16 groups, two rows per input relation. The
    // third operand duplicates each product; this counter run deduplicates
    // bank-index pairs and does not execute Free Join or grouped union.
    let mut pairs = BTreeSet::new();
    for group in 0..16 { for a in 0..2 { for b in 0..2 {
        pairs.insert(((group * 17 + a * 13) % 8, 8 + (group * 17 + b * 13 + 7) % 8));
    } } }
    let id: Vec<_> = (0..dimensions).collect();
    let map: Vec<_> = (0..dimensions).map(|v| [1, 2, 0][(v / width) as usize] * width + v % width).collect();
    let out: Vec<_> = (0..dimensions).map(|v| [0, 2, 1][(v / width) as usize] * width + v % width).collect();
    let hidden = ((1 << width) - 1) << width;
    let mut sum = [0usize; 17];
    let mut max = [0usize; 17];
    for &(a,b) in &pairs {
        let mut arena = input.clone();
        let (mut actual, stats) = arena.mapped_product_observed(bank[a], &id, bank[b], &map, hidden, if fused { &out } else { &id });
        if !fused { actual = arena.rename(actual, &out); }
        let values = counters(stats);
        for i in 0..17 { sum[i] += values[i]; max[i] = max[i].max(values[i]); }
        let right = arena.rename(bank[b], &map);
        let expected = arena.relprod(bank[a], right, hidden);
        assert_eq!(actual, arena.rename(expected, &out));
    }
    println!("{{\"cutoff\":{K},\"bits\":{dimensions},\"layout\":\"{layout}\",\"fused\":{fused},\"pairs\":{},\"sum\":{:?},\"max\":{:?},\"verified\":true}}", pairs.len(), sum, max);
}
fn counters(s: ViewProductStatistics) -> [usize; 17] {
    [s.subproblems, s.memo_hits, s.local_cubes, s.borrowed_planes,
     s.cofactor_steps, s.permutation_steps, s.broadcast_steps, s.branch_planes,
     s.branch_assignments, s.branch_word_merges, s.plane_cache_hits, s.plane_cache_misses,
     s.plane_cache_evictions, s.plane_cache_bytes_peak, s.canonicalized_views,
     s.canonical_cofactor_steps, s.removed_coordinates]
}
fn main() {
    // Deferred pins can overstate dependence. With z before x in storage,
    // F = if x then y else z still exposes raw z until the cofactor is reduced.
    let mut example = Arena::<1>::new(3, &[2, 1, 0]);
    let f = example.table(7, vec![0xd8]);
    assert_eq!(example.variables(f), 7);
    assert_eq!(example.variables(f) & !1, 6); // conservative y,z mask
    let residual = example.cofactor(f, 0, true);
    assert_eq!(example.variables(residual), 2); // exact y-only mask
    assert_eq!(residual, example.variable(1));
    for width in [4,6] { for layout in ["face-major","bit-major","pair-major"] { for fused in [false,true] {
        run::<6>(width, layout, fused);run::<9>(width, layout, fused);
    } } }
}
