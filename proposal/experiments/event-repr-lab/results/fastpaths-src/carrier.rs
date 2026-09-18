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
pub fn abstract_dense(b: &mut [u64], n: usize, mask: u32) {
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
    fn import(&mut self, b: &[u64]) -> Id;
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id;
    fn not(&mut self, a: Id) -> Id;
    fn empty(&self) -> Id;
    fn full(&self) -> Id;
    fn export(&self, a: Id) -> Vec<u64>;
    fn exists(&mut self, a: Id, mask: u32) -> Id;
    fn bytes(&self) -> usize;
    fn nodes(&self) -> usize;
    fn count(&self, a: Id) -> u64 {
        self.export(a).iter().map(|w| w.count_ones() as u64).sum()
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u32) -> Id {
        let v = self.op(8, a, b);
        self.exists(v, mask)
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
