use super::carrier::*;
use roaring::RoaringBitmap;
use rustc_hash::FxHashMap;

pub trait Set: Clone + PartialEq {
    const NAME: &'static str;
    fn from_bits(b: &[u64]) -> Self;
    fn raw_op(op: u8, a: &Self, b: &Self) -> Self;
    fn len(&self) -> usize;
    fn contains(&self, w: u32) -> bool;
    fn fingerprint(&self) -> u64;
    fn bytes(&self) -> usize;
    fn write(&self, out: &mut [u64]);
}

#[derive(Clone, PartialEq)]
pub struct Dense(Vec<u64>);
impl Set for Dense {
    const NAME: &'static str = "dense";
    fn from_bits(b: &[u64]) -> Self {
        Self(b.to_vec())
    }
    fn raw_op(op: u8, a: &Self, b: &Self) -> Self {
        Self(
            a.0.iter()
                .zip(&b.0)
                .map(|(&x, &y)| binary(op, x, y))
                .collect(),
        )
    }
    fn len(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }
    fn contains(&self, w: u32) -> bool {
        at(&self.0, w as usize)
    }
    fn fingerprint(&self) -> u64 {
        hash(&self.0)
    }
    fn bytes(&self) -> usize {
        24 + 8 * self.0.capacity()
    }
    fn write(&self, out: &mut [u64]) {
        out.copy_from_slice(&self.0);
    }
}

#[derive(Clone, PartialEq)]
pub struct Sparse(Vec<u32>);
impl Set for Sparse {
    const NAME: &'static str = "sparse";
    fn from_bits(b: &[u64]) -> Self {
        let mut a = Vec::new();
        for (i, &word) in b.iter().enumerate() {
            let mut w = word;
            while w != 0 {
                a.push((64 * i + w.trailing_zeros() as usize) as u32);
                w &= w - 1;
            }
        }
        Self(a)
    }
    fn raw_op(op: u8, a: &Self, b: &Self) -> Self {
        if op == 0 {
            return Self(vec![]);
        }
        if op == 12 {
            return a.clone();
        }
        if op == 10 {
            return b.clone();
        }
        let mut o = Vec::with_capacity(match op {
            8 => a.0.len().min(b.0.len()),
            4 => a.0.len(),
            2 => b.0.len(),
            _ => a.0.len() + b.0.len(),
        });
        let (mut i, mut j) = (0, 0);
        while i < a.0.len() || j < b.0.len() {
            let x = a.0.get(i).copied().unwrap_or(u32::MAX);
            let y = b.0.get(j).copied().unwrap_or(u32::MAX);
            let v = x.min(y);
            let ai = x == v;
            let bj = y == v;
            if op >> ((ai as u8) * 2 + bj as u8) & 1 != 0 {
                o.push(v);
            }
            i += ai as usize;
            j += bj as usize;
        }
        Self(o)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn contains(&self, w: u32) -> bool {
        self.0.binary_search(&w).is_ok()
    }
    fn fingerprint(&self) -> u64 {
        hash(&self.0)
    }
    fn bytes(&self) -> usize {
        24 + 4 * self.0.capacity()
    }
    fn write(&self, out: &mut [u64]) {
        for &w in &self.0 {
            out[w as usize / 64] |= 1 << (w % 64);
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Roaring(RoaringBitmap);
impl Set for Roaring {
    const NAME: &'static str = "roaring";
    fn from_bits(b: &[u64]) -> Self {
        Self(Sparse::from_bits(b).0.into_iter().collect())
    }
    fn raw_op(op: u8, a: &Self, b: &Self) -> Self {
        Self(match op {
            0 => RoaringBitmap::new(),
            2 => &b.0 - &a.0,
            4 => &a.0 - &b.0,
            6 => &a.0 ^ &b.0,
            8 => &a.0 & &b.0,
            10 => b.0.clone(),
            12 => a.0.clone(),
            14 => &a.0 | &b.0,
            _ => unreachable!(),
        })
    }
    fn len(&self) -> usize {
        self.0.len() as usize
    }
    fn contains(&self, w: u32) -> bool {
        self.0.contains(w)
    }
    // Summary hash is only a bucket selector; complete bitmap equality resolves collisions.
    fn fingerprint(&self) -> u64 {
        hash(&(
            self.0.len(),
            self.0.min(),
            self.0.max(),
            self.0.select((self.0.len() / 2) as u32),
        ))
    }
    fn bytes(&self) -> usize {
        let s = self.0.statistics();
        24 + s.n_containers as usize * 32
            + (s.n_bytes_array_containers + s.n_bytes_bitset_containers) as usize
    }
    fn write(&self, out: &mut [u64]) {
        for w in self.0.iter() {
            out[w as usize / 64] |= 1 << (w % 64);
        }
    }
}

// Sorted disjoint half-open runs; this asks how far Allen-like region storage goes.
#[derive(Clone, PartialEq)]
pub struct Runs(Vec<(u32, u32)>);
impl Set for Runs {
    const NAME: &'static str = "runs";
    fn from_bits(b: &[u64]) -> Self {
        let mut o = Vec::new();
        let mut start = None;
        for i in 0..b.len() * 64 {
            if at(b, i) {
                if start.is_none() {
                    start = Some(i as u32);
                }
            } else if let Some(s) = start.take() {
                o.push((s, i as u32));
            }
        }
        if let Some(s) = start {
            o.push((s, (b.len() * 64) as u32));
        }
        Self(o)
    }
    fn raw_op(op: u8, a: &Self, b: &Self) -> Self {
        if op == 0 {
            return Self(vec![]);
        }
        if op == 12 {
            return a.clone();
        }
        if op == 10 {
            return b.clone();
        }
        let (mut i, mut j) = (0, 0);
        let (mut av, mut bv) = (false, false);
        let mut start = 0;
        let mut out = Vec::new();
        let point = |r: &Self, k: usize| {
            if k >= r.0.len() * 2 {
                u32::MAX
            } else if k % 2 == 0 {
                r.0[k / 2].0
            } else {
                r.0[k / 2].1
            }
        };
        while i < a.0.len() * 2 || j < b.0.len() * 2 {
            let x = point(a, i);
            let y = point(b, j);
            let p = x.min(y);
            let old = op >> ((av as u8) * 2 + bv as u8) & 1 != 0;
            if x == p {
                av = !av;
                i += 1;
            }
            if y == p {
                bv = !bv;
                j += 1;
            }
            let new = op >> ((av as u8) * 2 + bv as u8) & 1 != 0;
            if !old && new {
                start = p;
            }
            if old && !new {
                out.push((start, p));
            }
        }
        Self(out)
    }
    fn len(&self) -> usize {
        self.0.iter().map(|&(a, b)| (b - a) as usize).sum()
    }
    fn contains(&self, w: u32) -> bool {
        let i = self.0.partition_point(|&(a, _)| a <= w);
        i > 0 && w < self.0[i - 1].1
    }
    fn fingerprint(&self) -> u64 {
        hash(&self.0)
    }
    fn bytes(&self) -> usize {
        24 + 8 * self.0.capacity()
    }
    fn write(&self, out: &mut [u64]) {
        for &(a, b) in &self.0 {
            for w in a..b {
                out[w as usize / 64] |= 1 << (w % 64);
            }
        }
    }
}

#[derive(Clone)]
pub struct Finite<B: Set> {
    n: usize,
    support: B,
    support_bits: Vec<u64>,
    population: usize,
    pivot: u32,
    sets: Vec<B>,
    links: Vec<u32>,
    index: FxHashMap<u64, u32>,
}
impl<B: Set> Finite<B> {
    fn intern(&mut self, mut value: B) -> Id {
        let c = value.len();
        let flip =
            c * 2 > self.population || (c * 2 == self.population && value.contains(self.pivot));
        if flip {
            value = B::raw_op(4, &self.support, &value);
        }
        let h = value.fingerprint();
        let prior = self.index.get(&h).copied().unwrap_or(u32::MAX);
        let mut at = prior;
        while at != u32::MAX {
            if self.sets[at as usize] == value {
                return (at as Id) * 2 + flip as Id;
            }
            at = self.links[at as usize];
        }
        assert!(
            self.sets.len() < 1_000_000,
            "RESOURCE_CAP: finite region count"
        );
        let id = self.sets.len() as u32;
        self.sets.push(value);
        self.links.push(prior);
        self.index.insert(h, id);
        (id as Id) * 2 + flip as Id
    }
}
impl<B: Set> Carrier for Finite<B> {
    const NAME: &'static str = B::NAME;
    fn new(support: &[u64], n: usize) -> Self {
        let s = B::from_bits(support);
        let population = s.len();
        assert!(population > 0);
        let pivot = (0..n as u32).find(|&w| s.contains(w)).unwrap();
        let mut a = Self {
            n,
            support: s,
            support_bits: support.to_vec(),
            population,
            pivot,
            sets: vec![],
            links: vec![],
            index: FxHashMap::default(),
        };
        assert_eq!(a.intern(B::from_bits(&vec![0; support.len()])), 0);
        a
    }
    fn import(&mut self, b: &[u64]) -> Id {
        let r = B::from_bits(b);
        let r = B::raw_op(8, &r, &self.support);
        self.intern(r)
    }
    #[inline]
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        if a == b {
            return match ((op & 1) != 0, (op & 8) != 0) {
                (false, false) => 0,
                (true, true) => 1,
                (false, true) => a,
                (true, false) => a ^ 1,
            };
        }
        let mut code = 0u8;
        for cell in 0..4 {
            let orig = cell ^ (((a & 1) as u8) * 2) ^ ((b & 1) as u8);
            code |= ((op >> orig) & 1) << cell;
        }
        let flip = code & 1;
        if flip != 0 {
            code ^= 15;
        }
        if code == 0 {
            return flip as Id;
        }
        if code == 12 {
            return (a & !1) ^ flip as Id;
        }
        if code == 10 {
            return (b & !1) ^ flip as Id;
        }
        let raw = B::raw_op(
            code,
            &self.sets[(a / 2) as usize],
            &self.sets[(b / 2) as usize],
        );
        self.intern(raw) ^ flip as Id
    }
    fn not(&mut self, a: Id) -> Id {
        a ^ 1
    }
    fn empty(&self) -> Id {
        0
    }
    fn full(&self) -> Id {
        1
    }
    fn export(&self, a: Id) -> Vec<u64> {
        let mut b = vec![0; self.support_bits.len()];
        self.sets[(a / 2) as usize].write(&mut b);
        if a & 1 != 0 {
            for (v, s) in b.iter_mut().zip(&self.support_bits) {
                *v ^= *s;
            }
        }
        b
    }
    fn exists(&mut self, a: Id, mask: u32) -> Id {
        let mut b = self.export(a);
        abstract_dense(&mut b, self.n, mask);
        self.import(&b)
    }
    fn bytes(&self) -> usize {
        self.sets.iter().map(Set::bytes).sum::<usize>()
            + self.links.capacity() * 4
            + self.index.capacity() * 24
            + self.support.bytes()
            + self.support_bits.capacity() * 8
    }
    fn nodes(&self) -> usize {
        self.sets.len()
    }
    fn count(&self, a: Id) -> u64 {
        let n = self.sets[(a / 2) as usize].len();
        if a & 1 != 0 {
            (self.population - n) as u64
        } else {
            n as u64
        }
    }
}
