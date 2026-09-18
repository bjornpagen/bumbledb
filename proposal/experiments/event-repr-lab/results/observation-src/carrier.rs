use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};

pub type Id = u64;
pub fn hash<T: Hash + ?Sized>(x: &T) -> u64 {
    let mut h = FxHasher::default();
    x.hash(&mut h);
    h.finish()
}
pub fn bits(n: usize, f: impl Fn(usize) -> bool) -> Vec<u64> {
    let mut b = vec![0; n.div_ceil(64)];
    for i in 0..n {
        if f(i) {
            b[i / 64] |= 1 << (i % 64);
        }
    }
    b
}
pub fn at(b: &[u64], w: usize) -> bool {
    b[w / 64] >> (w % 64) & 1 != 0
}
pub fn binary(op: u8, a: u64, b: u64) -> u64 {
    match op {
        0 => 0,
        2 => !a & b,
        4 => a & !b,
        6 => a ^ b,
        8 => a & b,
        10 => b,
        12 => a,
        14 => a | b,
        _ => !binary(op ^ 15, a, b),
    }
}
// Identical constant/equal/complementary-input rules for every canonical carrier.
#[inline]
pub fn trivial(op: u8, a: Id, b: Id) -> Option<Id> {
    let unary = |x: Id, code: u8| match code & 3 {
        0 => 0,
        1 => x ^ 1,
        2 => x,
        _ => 1,
    };
    Some(match op {
        0 => 0,
        15 => 1,
        12 => a,
        10 => b,
        3 => a ^ 1,
        5 => b ^ 1,
        _ if a < 2 => unary(b, op >> (2 * a)),
        _ if b < 2 => unary(a, ((op >> b) & 1) | (((op >> (2 + b)) & 1) << 1)),
        _ if a == b => unary(a, (op & 1) | ((op >> 2) & 2)),
        _ if a == (b ^ 1) => unary(a, op >> 1),
        _ => return None,
    })
}
pub fn abstract_dense(b: &mut [u64], n: usize, mask: u64) {
    assert!(n.is_power_of_two());
    for k in 0..n.trailing_zeros() {
        if mask >> k & 1 == 0 {
            continue;
        }
        if k < 6 {
            let stride = 1usize << k;
            let mut low = 0u64;
            for p in (0..64).step_by(2 * stride) {
                low |= ((1u64 << stride) - 1) << p;
            }
            for w in b.iter_mut() {
                let a = (*w | (*w >> stride)) & low;
                *w = a | (a << stride);
            }
        } else {
            let stride = 1usize << (k - 6);
            for base in (0..b.len()).step_by(stride * 2) {
                for j in 0..stride {
                    let v = b[base + j] | b[base + j + stride];
                    b[base + j] = v;
                    b[base + j + stride] = v;
                }
            }
        }
    }
    if n < 64 {
        b[0] &= (1u64 << n) - 1;
    }
}

pub trait Carrier: Clone {
    const NAME: &'static str;
    fn new(support: &[u64], n: usize) -> Self;
    // Finite containers retain the semantic world-bit enumeration. Diagram
    // managers override this method to choose their internal decision order.
    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        validate_order(n.next_power_of_two().trailing_zeros(), &order);
        Self::new(support, n)
    }
    fn import(&mut self, b: &[u64]) -> Id;
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id;
    fn not(&mut self, a: Id) -> Id;
    fn empty(&self) -> Id;
    fn full(&self) -> Id;
    fn export(&self, a: Id) -> Vec<u64>;
    fn exists(&mut self, a: Id, mask: u64) -> Id;
    // Pull back through a bijective coordinate renaming, then intersect support.
    // Laws commuting with relative complement require support preservation.
    fn permute(&mut self, a: Id, map: &Permutation) -> Id;
    fn bytes(&self) -> usize;
    fn nodes(&self) -> usize;
    fn count(&self, a: Id) -> u64 {
        self.export(a).iter().map(|w| w.count_ones() as u64).sum()
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        plan.prepare_table(&(0..plan.group_of.len() as u32).collect::<Vec<_>>());
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum;
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let v = self.op(8, a, b);
        self.exists(v, mask)
    }
}

pub fn validate_order(nbits: u32, order: &[u32]) {
    assert!(nbits < 63 && order.len() == nbits as usize);
    let mut seen = 0u64;
    for &coord in order {
        assert!(coord < nbits && seen >> coord & 1 == 0);
        seen |= 1 << coord;
    }
    assert_eq!(seen, (1u64 << nbits) - 1);
}

pub fn face_order(nbits: u32, faces: u32, layout: &str) -> Vec<u32> {
    assert!(faces > 0 && nbits % faces == 0);
    let width = nbits / faces;
    let chunk = match layout {
        "face-major" => width.max(1),
        "bit-major" => 1,
        "pair-major" => 2,
        _ => panic!("unknown face layout {layout}"),
    };
    let mut order = vec![];
    for base in (0..width).step_by(chunk as usize).rev() {
        for face in (0..faces).rev() {
            for bit in (base..(base + chunk).min(width)).rev() {
                order.push(face * width + bit);
            }
        }
    }
    validate_order(nbits, &order);
    order
}

#[derive(Clone)]
pub struct Permutation {
    // Old coordinate i becomes coordinate destinations[i].
    pub destinations: Vec<u32>,
    swaps: Vec<(u32, u32)>,
}
impl Permutation {
    pub fn new(destinations: Vec<u32>) -> Result<Self, &'static str> {
        if destinations.len() >= 63 {
            return Err("coordinate limit");
        }
        let mut seen = 0u64;
        for &c in &destinations {
            if c as usize >= destinations.len() || seen >> c & 1 != 0 {
                return Err("not a coordinate permutation");
            }
            seen |= 1 << c;
        }
        let mut labels: Vec<_> = (0..destinations.len() as u32).collect();
        let mut swaps = vec![];
        for (old, &target) in destinations.iter().enumerate() {
            let from = labels.iter().position(|&v| v == old as u32).unwrap();
            if from != target as usize {
                swaps.push(((from as u32).min(target), (from as u32).max(target)));
                labels.swap(from, target as usize);
            }
        }
        Ok(Self {
            destinations,
            swaps,
        })
    }
    pub fn source_world(&self, destination: usize) -> usize {
        self.destinations
            .iter()
            .enumerate()
            .fold(0, |w, (old, &new)| w | (((destination >> new) & 1) << old))
    }
    pub fn dense(&self, words: &mut [u64]) {
        assert_eq!(
            words.len(),
            (1usize << self.destinations.len()).div_ceil(64)
        );
        for &(i, j) in &self.swaps {
            if j < 6 {
                let distance = (1 << j) - (1 << i);
                let mask = (0..64)
                    .filter(|p| p >> i & 1 != 0 && p >> j & 1 == 0)
                    .fold(0u64, |m, p| m | (1 << p));
                for w in words.iter_mut() {
                    let t = (*w ^ (*w >> distance)) & mask;
                    *w ^= t ^ (t << distance);
                }
            } else if i < 6 {
                let stride = 1usize << (j - 6);
                let shift = 1 << i;
                let mask = (0..64)
                    .filter(|p| p >> i & 1 == 0)
                    .fold(0u64, |m, p| m | (1 << p));
                for base in (0..words.len()).step_by(stride * 2) {
                    let (low, high) = words[base..base + 2 * stride].split_at_mut(stride);
                    for (a, b) in low.iter_mut().zip(high) {
                        let t = ((*a >> shift) ^ *b) & mask;
                        *a ^= t << shift;
                        *b ^= t;
                    }
                }
            } else {
                let a = 1usize << (i - 6);
                let b = 1usize << (j - 6);
                for p in 0..words.len() {
                    if p & a != 0 && p & b == 0 {
                        words.swap(p, p ^ a ^ b);
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Memo<C: Carrier> {
    pub inner: C,
    pub enabled: bool,
    pub cache: FxHashMap<(u8, Id, Id), Id>,
    pub hits: u64,
    pub misses: u64,
}
impl<C: Carrier> Memo<C> {
    pub fn new(inner: C, enabled: bool) -> Self {
        Self {
            inner,
            enabled,
            cache: FxHashMap::default(),
            hits: 0,
            misses: 0,
        }
    }
    #[inline]
    pub fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        if self.enabled {
            if let Some(&r) = self.cache.get(&(op, a, b)) {
                self.hits += 1;
                return r;
            }
        }
        self.misses += 1;
        let r = self.inner.op(op, a, b);
        if self.enabled {
            self.cache.insert((op, a, b), r);
        }
        r
    }
    #[inline]
    pub fn expression(&mut self, a: Id, b: Id, c: Id) -> Id {
        let bc = self.op(14, b, c);
        self.op(4, a, bc)
    }
    pub fn bytes(&self) -> usize {
        self.inner.bytes() + self.cache.capacity() * 40
    }
}

pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
