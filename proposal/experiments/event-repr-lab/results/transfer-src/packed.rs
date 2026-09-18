//! Canonical ordered diagrams whose terminal values are exact truth tables.
//! Fixed prefix/tail decomposition; support-relative anchored public handles.
use super::carrier::*;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(C, align(16))]
struct Branch {
    level: u32,
    low: u32,
    high: u32,
}

#[derive(Clone)]
pub struct Packed<const W: usize> {
    n: usize,
    order: Vec<u32>,
    tail: Vec<u32>,
    prefix: usize,
    cells: usize,
    masks: [u64; W],
    contiguous_tail: bool,
    // Lab control: both algorithms have identical representation and identity.
    local_permutation: bool,
    branches: Vec<Branch>,
    unique: FxHashMap<Branch, u32>,
    leaves: Vec<[u64; W]>,
    leaf_unique: FxHashMap<[u64; W], u32>,
    apply: FxHashMap<(u8, u32, u32), u32>,
    support: u32,
    anchor: usize,
    population: u64,
}

impl<const W: usize> Packed<W> {
    fn transport_raw(&self,r:u32,out:&mut super::transfer::Builder,memo:&mut FxHashMap<u32,u32>)->u32 {
        if r<2 {return r;}
        let base=r&!1;
        if let Some(&v)=memo.get(&base) {return v^(r&1);}
        let v=if r&2==0 {
            out.table(self.tail.clone(),self.leaves[(r>>2) as usize][..self.cells.div_ceil(64)].to_vec())
        } else {
            let node=self.branches[(r>>2) as usize];
            let lo=self.transport_raw(node.low,out,memo);
            let hi=self.transport_raw(node.high,out,memo);
            out.split(vec![self.order[node.level as usize]],vec![lo,hi])
        };
        memo.insert(base,v);v^(r&1)
    }
    // Raw reference: bit 0 complements; bit 1 selects branch vs terminal table.
    // Tables 0/1 denote global false/true. Branch indices use the upper 30 bits.
    pub fn symbolic(nbits: u32, order: Vec<u32>) -> Self {
        assert!(matches!(W, 1 | 4 | 8 | 64));
        assert!(nbits < 63 && order.len() == nbits as usize);
        let mut covered = 0u64;
        for &k in &order {
            assert!(k < nbits && covered >> k & 1 == 0);
            covered |= 1 << k;
        }
        assert_eq!(covered, (1u64 << nbits) - 1);
        let cut = nbits.min(6 + W.ilog2()) as usize;
        let prefix = order.len() - cut;
        let tail: Vec<_> = order[prefix..].iter().rev().copied().collect();
        let cells = 1usize << cut;
        let masks = std::array::from_fn(|i| {
            let valid = cells.saturating_sub(i * 64).min(64);
            if valid == 64 { !0 } else { (1u64 << valid) - 1 }
        });
        let contiguous_tail = cut >= 6 && tail.iter().enumerate().all(|(i, &c)| i as u32 == c);
        let local_permutation = match std::env::var("EVENT_LAB_PACKED_MAP").as_deref() {
            Ok("recursive") => false,
            Ok("local") | Err(std::env::VarError::NotPresent) => true,
            _ => panic!("invalid EVENT_LAB_PACKED_MAP"),
        };
        Self {
            n: 1usize << nbits,
            order,
            tail,
            prefix,
            cells,
            masks,
            contiguous_tail,
            local_permutation,
            branches: vec![],
            unique: FxHashMap::default(),
            leaves: vec![[0; W]],
            leaf_unique: FxHashMap::from_iter([([0; W], 0)]),
            apply: FxHashMap::default(),
            support: 1,
            anchor: 0,
            population: 1u64 << nbits,
        }
    }
    pub fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        let mut c = Self::symbolic(n.next_power_of_two().trailing_zeros(), order);
        c.n = n;
        c.anchor = support
            .iter()
            .enumerate()
            .find_map(|(i, &w)| (w != 0).then(|| i * 64 + w.trailing_zeros() as usize))
            .unwrap();
        c.population = support.iter().map(|w| w.count_ones() as u64).sum();
        c.support = c.build(support, 0, 0);
        assert_ne!(c.support, 0);
        c
    }
    fn check_capacity(&self) {
        assert!(
            self.branches.len() + self.leaves.len() < 2_000_000,
            "RESOURCE_CAP: packed diagram records"
        );
    }
    fn leaf(&mut self, mut words: [u64; W]) -> u32 {
        for (w, &m) in words.iter_mut().zip(&self.masks) {
            *w &= m;
        }
        let bit = self.cells - 1;
        let flip = ((words[bit / 64] >> (bit % 64)) & 1) as u32;
        if flip != 0 {
            for (w, &m) in words.iter_mut().zip(&self.masks) {
                *w ^= m;
            }
        }
        if let Some(&r) = self.leaf_unique.get(&words) {
            return r ^ flip;
        }
        self.check_capacity();
        let r = (self.leaves.len() as u32) << 2;
        self.leaves.push(words);
        self.leaf_unique.insert(words, r);
        r ^ flip
    }
    fn mk(&mut self, level: u32, mut low: u32, mut high: u32) -> u32 {
        if low == high {
            return low;
        }
        let flip = high & 1;
        low ^= flip;
        high ^= flip;
        let node = Branch { level, low, high };
        if let Some(&r) = self.unique.get(&node) {
            return r ^ flip;
        }
        self.check_capacity();
        let r = ((self.branches.len() as u32) << 2) | 2;
        self.branches.push(node);
        self.unique.insert(node, r);
        r ^ flip
    }
    fn level(&self, r: u32) -> usize {
        if r & 2 == 0 {
            self.prefix
        } else {
            self.branches[(r >> 2) as usize].level as usize
        }
    }
    fn child(&self, r: u32, level: usize, high: bool) -> u32 {
        if self.level(r) != level {
            return r;
        }
        let n = self.branches[(r >> 2) as usize];
        (if high { n.high } else { n.low }) ^ (r & 1)
    }
    fn words(&self, r: u32) -> [u64; W] {
        debug_assert_eq!(r & 2, 0);
        let raw = self.leaves[(r >> 2) as usize];
        if r & 1 == 0 {
            raw
        } else {
            std::array::from_fn(|i| raw[i] ^ self.masks[i])
        }
    }
    fn combine(op: u8, a: [u64; W], b: [u64; W]) -> [u64; W] {
        #[inline]
        fn words<const OP: u8, const W: usize>(a: [u64; W], b: [u64; W]) -> [u64; W] {
            std::array::from_fn(|i| binary(OP, a[i], b[i]))
        }
        match op {
            0 => words::<0, W>(a, b),
            1 => words::<1, W>(a, b),
            2 => words::<2, W>(a, b),
            3 => words::<3, W>(a, b),
            4 => words::<4, W>(a, b),
            5 => words::<5, W>(a, b),
            6 => words::<6, W>(a, b),
            7 => words::<7, W>(a, b),
            8 => words::<8, W>(a, b),
            9 => words::<9, W>(a, b),
            10 => words::<10, W>(a, b),
            11 => words::<11, W>(a, b),
            12 => words::<12, W>(a, b),
            13 => words::<13, W>(a, b),
            14 => words::<14, W>(a, b),
            15 => words::<15, W>(a, b),
            _ => unreachable!(),
        }
    }
    fn raw_op(&mut self, mut op: u8, mut a: u32, mut b: u32) -> u32 {
        if let Some(r) = trivial(op, a as Id, b as Id) {
            return r as u32;
        }
        if a > b {
            std::mem::swap(&mut a, &mut b);
            op = (op & 9) | ((op & 2) << 1) | ((op & 4) >> 1);
        }
        if let Some(&r) = self.apply.get(&(op, a, b)) {
            return r;
        }
        let level = self.level(a).min(self.level(b));
        let r = if level == self.prefix {
            self.leaf(Self::combine(op, self.words(a), self.words(b)))
        } else {
            let low = self.raw_op(op, self.child(a, level, false), self.child(b, level, false));
            let high = self.raw_op(op, self.child(a, level, true), self.child(b, level, true));
            self.mk(level as u32, low, high)
        };
        self.apply.insert((op, a, b), r);
        r
    }
    fn tail_index(&self, w: usize) -> usize {
        self.tail
            .iter()
            .enumerate()
            .fold(0, |i, (k, &c)| i | (((w >> c) & 1) << k))
    }
    fn eval(&self, mut r: u32, w: usize) -> bool {
        while r & 2 != 0 {
            let node = self.branches[(r >> 2) as usize];
            r = (if w >> self.order[node.level as usize] & 1 == 0 {
                node.low
            } else {
                node.high
            }) ^ (r & 1);
        }
        let i = self.tail_index(w);
        ((self.leaves[(r >> 2) as usize][i / 64] >> (i % 64)) & 1) != (r & 1) as u64
    }
    fn build(&mut self, b: &[u64], level: usize, prefix: usize) -> u32 {
        if level == self.prefix {
            let mut words = [0; W];
            if self.contiguous_tail {
                for (i, w) in words.iter_mut().enumerate() {
                    *w = b.get(prefix / 64 + i).copied().unwrap_or(0);
                }
            } else {
                for i in 0..self.cells {
                    let w = self
                        .tail
                        .iter()
                        .enumerate()
                        .fold(prefix, |w, (k, &c)| w | (((i >> k) & 1) << c));
                    if w < self.n && at(b, w) {
                        words[i / 64] |= 1 << (i % 64);
                    }
                }
            }
            return self.leaf(words);
        }
        let low = self.build(b, level + 1, prefix);
        let high = self.build(b, level + 1, prefix | (1usize << self.order[level]));
        self.mk(level as u32, low, high)
    }
    fn seal(&mut self, raw: u32) -> Id {
        let flip = self.eval(raw, self.anchor) as u32;
        let r = self.raw_op(8, self.support, raw ^ flip);
        (r as Id) << 1 | flip as Id
    }
    fn true_root(&mut self, a: Id) -> u32 {
        let r = (a >> 1) as u32;
        if a & 1 == 0 {
            r
        } else {
            self.raw_op(4, self.support, r)
        }
    }
    fn raw_literal(&mut self, coord: u32) -> u32 {
        let level = self.order.iter().position(|&v| v == coord).unwrap();
        if level < self.prefix {
            self.mk(level as u32, 0, 1)
        } else {
            let k = self.tail.iter().position(|&v| v == coord).unwrap();
            let mut words = [0; W];
            for i in 0..self.cells {
                if i >> k & 1 != 0 {
                    words[i / 64] |= 1 << (i % 64);
                }
            }
            self.leaf(words)
        }
    }
    pub fn literal(&mut self, coord: u32) -> Id {
        let raw = self.raw_literal(coord);
        self.seal(raw)
    }
    fn select(&mut self, condition: u32, low: u32, high: u32) -> u32 {
        if low == high {
            return low;
        }
        let a = self.raw_op(2, condition, low);
        let b = self.raw_op(8, condition, high);
        self.raw_op(14, a, b)
    }
    fn permute_table(
        &mut self,
        words: &[u64; W],
        start: usize,
        size: usize,
        literals: &[u32],
    ) -> u32 {
        if size == 1 {
            return ((words[start / 64] >> (start % 64)) & 1) as u32;
        }
        let low = self.permute_table(words, start, size / 2, literals);
        let high = self.permute_table(words, start + size / 2, size / 2, literals);
        let coord = self.tail[size.trailing_zeros() as usize - 1] as usize;
        self.select(literals[coord], low, high)
    }
    fn permute_rec(
        &mut self,
        r: u32,
        literals: &[u32],
        tail_map: Option<&Permutation>,
        memo: &mut FxHashMap<u32, u32>,
    ) -> u32 {
        if r < 2 {
            return r;
        }
        if r & 1 != 0 {
            return self.permute_rec(r ^ 1, literals, tail_map, memo) ^ 1;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let out = if r & 2 == 0 {
            if let Some(map) = tail_map {
                let mut words = self.words(r);
                map.dense(&mut words[..self.cells.div_ceil(64)]);
                self.leaf(words)
            } else {
                self.permute_table(&self.words(r), 0, self.cells, literals)
            }
        } else {
            let node = self.branches[(r >> 2) as usize];
            let low = self.permute_rec(node.low, literals, tail_map, memo);
            let high = self.permute_rec(node.high, literals, tail_map, memo);
            self.select(
                literals[self.order[node.level as usize] as usize],
                low,
                high,
            )
        };
        memo.insert(r, out);
        out
    }
    fn reduce_leaf(&mut self, mut words: [u64; W], mask: u64) -> u32 {
        let local = self
            .tail
            .iter()
            .enumerate()
            .fold(0, |m, (k, &c)| m | (((mask >> c) & 1) << k));
        abstract_dense(&mut words[..self.cells.div_ceil(64)], self.cells, local);
        self.leaf(words)
    }
    fn abstract_rec(&mut self, r: u32, mask: u64, memo: &mut FxHashMap<u32, u32>) -> u32 {
        if r < 2 {
            return r;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let level = self.level(r);
        let out = if level == self.prefix {
            self.reduce_leaf(self.words(r), mask)
        } else {
            let low = self.abstract_rec(self.child(r, level, false), mask, memo);
            let high = self.abstract_rec(self.child(r, level, true), mask, memo);
            if mask >> self.order[level] & 1 != 0 {
                self.raw_op(14, low, high)
            } else {
                self.mk(level as u32, low, high)
            }
        };
        memo.insert(r, out);
        out
    }
    fn product_rec(
        &mut self,
        a: u32,
        b: u32,
        mask: u64,
        memo: &mut FxHashMap<(u32, u32), u32>,
    ) -> u32 {
        if a == 0 || b == 0 {
            return 0;
        }
        let key = (a.min(b), a.max(b));
        if let Some(&r) = memo.get(&key) {
            return r;
        }
        let level = self.level(a).min(self.level(b));
        let out = if level == self.prefix {
            self.reduce_leaf(Self::combine(8, self.words(a), self.words(b)), mask)
        } else {
            let low = self.product_rec(
                self.child(a, level, false),
                self.child(b, level, false),
                mask,
                memo,
            );
            let high = self.product_rec(
                self.child(a, level, true),
                self.child(b, level, true),
                mask,
                memo,
            );
            if mask >> self.order[level] & 1 != 0 {
                self.raw_op(14, low, high)
            } else {
                self.mk(level as u32, low, high)
            }
        };
        memo.insert(key, out);
        out
    }
    fn count_raw(&self, r: u32, from: usize, memo: &mut FxHashMap<(u32, usize), u64>) -> u64 {
        if let Some(&n) = memo.get(&(r, from)) {
            return n;
        }
        let level = self.level(r);
        let count = if level == self.prefix {
            self.words(r)
                .iter()
                .map(|w| w.count_ones() as u64)
                .sum::<u64>()
                << (self.prefix - from)
        } else {
            (self.count_raw(self.child(r, level, false), level + 1, memo)
                + self.count_raw(self.child(r, level, true), level + 1, memo))
                << (level - from)
        };
        memo.insert((r, from), count);
        count
    }
    fn spectrum_raw(
        &self,
        r: u32,
        from: usize,
        plan: &super::observation::CountPlan,
        memo: &mut FxHashMap<(u32, usize), super::observation::Spectrum>,
    ) -> super::observation::Spectrum {
        if let Some(out) = memo.get(&(r, from)) {
            return out.clone();
        }
        let level = self.level(r);
        let mut out = if level == self.prefix {
            plan.table(&self.tail)
                .contract(plan, &self.words(r)[..self.cells.div_ceil(64)], None)
        } else {
            let mut out = plan.zero();
            let table = plan.table(&[self.order[level]]);
            for (i, high) in [false, true].into_iter().enumerate() {
                let child = self.spectrum_raw(self.child(r, level, high), level + 1, plan, memo);
                for (shift, count) in table.terms(0, 1 << i) {
                    plan.add_shifted(&mut out, &child, shift, count);
                }
            }
            out
        };
        plan.smooth(&mut out, self.order[from..level].iter().copied());
        memo.insert((r, from), out.clone());
        out
    }
}
impl<const W: usize> Carrier for Packed<W> {
    fn dimensions(&self)->u32 {self.n.next_power_of_two().trailing_zeros()}
    fn full_space(nbits:u32,order:Vec<u32>)->Result<Self,&'static str> {Ok(Self::symbolic(nbits,order))}
    fn variable(&mut self,coordinate:u32)->Id {self.literal(coordinate)}
    fn packet(&self,names:&[u64],roots:&[Id])->super::transfer::Packet {
        assert_eq!(names.len(),self.dimensions() as usize);
        let mut out=super::transfer::Builder::new(names.to_vec(),self.order.clone());
        let mut memo=FxHashMap::default();
        let support=self.transport_raw(self.support,&mut out,&mut memo);
        let roots=roots.iter().map(|&r|self.transport_raw((r>>1) as u32,&mut out,&mut memo)^((r&1) as u32)).collect();
        out.finish(support,roots)
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        assert_eq!(plan.group_of.len(), self.order.len());
        plan.prepare_table(&self.tail);
        for &coord in &self.order[..self.prefix] {
            plan.prepare_table(&[coord]);
        }
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum {
        let mut memo = FxHashMap::default();
        let part = self.spectrum_raw((event >> 1) as u32, 0, plan, &mut memo);
        if event & 1 == 0 {
            part
        } else {
            plan.subtract(self.spectrum_raw(self.support, 0, plan, &mut memo), &part)
        }
    }
    const NAME: &'static str = match W {
        1 => "packed64",
        4 => "packed256",
        8 => "packed512",
        64 => "packed4096",
        _ => "invalid-packed",
    };
    fn new(support: &[u64], n: usize) -> Self {
        let nbits = n.next_power_of_two().trailing_zeros();
        Self::with_order(support, n, (0..nbits).rev().collect())
    }
    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        Packed::with_order(support, n, order)
    }
    fn import(&mut self, b: &[u64]) -> Id {
        let r = self.build(b, 0, 0);
        self.seal(r)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        if let Some(r) = trivial(op, a, b) {
            return r;
        }
        let mut code = 0;
        for cell in 0..4 {
            let old = cell ^ ((a & 1) as u8 * 2) ^ (b & 1) as u8;
            code |= ((op >> old) & 1) << cell;
        }
        let flip = code & 1;
        if flip != 0 {
            code ^= 15;
        }
        let r = self.raw_op(code, (a >> 1) as u32, (b >> 1) as u32);
        (r as Id) << 1 | flip as Id
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
        bits(self.n, |w| {
            self.eval(self.support, w) && (self.eval((a >> 1) as u32, w) ^ (a & 1 != 0))
        })
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        let r = self.true_root(a);
        let out = self.abstract_rec(r, mask, &mut FxHashMap::default());
        self.seal(out)
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        assert_eq!(self.n, 1usize << map.destinations.len());
        let root = self.true_root(a);
        let literals: Vec<_> = map
            .destinations
            .iter()
            .map(|&k| self.raw_literal(k))
            .collect();
        // A global bijection that keeps the tail's coordinate set invariant
        // induces a bijection of the table's local axes. Prefix changes still
        // use canonical selection; maps crossing the cut retain the general path.
        let tail_map = if self.local_permutation {
            self.tail
                .iter()
                .map(|&old| {
                    self.tail
                        .iter()
                        .position(|&c| c == map.destinations[old as usize])
                        .map(|k| k as u32)
                })
                .collect::<Option<Vec<_>>>()
                .map(|destinations| Permutation::new(destinations).unwrap())
        } else {
            None
        };
        let raw = self.permute_rec(
            root,
            &literals,
            tail_map.as_ref(),
            &mut FxHashMap::default(),
        );
        self.seal(raw)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let a = self.true_root(a);
        let b = self.true_root(b);
        let out = self.product_rec(a, b, mask, &mut FxHashMap::default());
        self.seal(out)
    }
    fn count(&self, a: Id) -> u64 {
        let n = self.count_raw((a >> 1) as u32, 0, &mut FxHashMap::default());
        if a & 1 == 0 { n } else { self.population - n }
    }
    fn bytes(&self) -> usize {
        self.branches.capacity() * 16
            + self.unique.capacity() * 32
            + self.leaves.capacity() * W * 8
            + self.leaf_unique.capacity() * (W * 8 + 16)
            + self.apply.capacity() * 24
            + self.order.capacity() * 4
            + self.tail.capacity() * 4
    }
    fn nodes(&self) -> usize {
        self.branches.len() + self.leaves.len()
    }
}
