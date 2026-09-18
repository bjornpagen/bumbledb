use super::carrier::*;
use rustc_hash::FxHashMap;

#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Node<const N: usize> {
    var: u32,
    children: [u32; N],
}

#[derive(Clone)]
pub struct Diagram<const N: usize, const PAIRED: bool> {
    n: usize,
    order: Vec<[u32; 2]>,
    nodes: Vec<Node<N>>,
    unique: FxHashMap<Node<N>, u32>,
    apply: FxHashMap<(u8, u32, u32), u32>,
    support: u32,
    pairs: Vec<(u32, u32)>,
    pair_index: FxHashMap<(u32, u32), u32>,
}
impl<const N: usize, const P: bool> Diagram<N, P> {
    fn transport_raw(
        &self,
        r: u32,
        out: &mut super::transfer::Builder,
        memo: &mut FxHashMap<u32, u32>,
    ) -> u32 {
        if r < 2 {
            return r;
        }
        let base = r & !1;
        if let Some(&v) = memo.get(&base) {
            return v ^ (r & 1);
        }
        let node = self.nodes[(r / 2) as usize];
        let semantic: Vec<_> = self.order[node.var as usize]
            .iter()
            .copied()
            .enumerate()
            .take(N.trailing_zeros() as usize)
            .filter(|&(_, c)| c < self.n.next_power_of_two().trailing_zeros())
            .collect();
        let children = (0..1usize << semantic.len())
            .map(|i| {
                let old = semantic
                    .iter()
                    .enumerate()
                    .fold(0, |v, (k, &(position, _))| v | (((i >> k) & 1) << position));
                self.transport_raw(node.children[old], out, memo)
            })
            .collect();
        let v = out.split(semantic.iter().map(|&(_, c)| c).collect(), children);
        memo.insert(base, v);
        v ^ (r & 1)
    }
    fn raw_packet(&self, names: &[u64], roots: &[u32]) -> super::transfer::Packet {
        assert_eq!(
            names.len(),
            self.n.next_power_of_two().trailing_zeros() as usize
        );
        let order = self
            .order
            .iter()
            .flat_map(|g| g[..N.trailing_zeros() as usize].iter().rev().copied())
            .filter(|&c| c < (names.len() as u32))
            .collect();
        let mut out = super::transfer::Builder::new(names.to_vec(), order);
        let mut memo = FxHashMap::default();
        let support = self.transport_raw(self.support, &mut out, &mut memo);
        let roots = roots
            .iter()
            .map(|&r| self.transport_raw(r, &mut out, &mut memo))
            .collect();
        out.finish(support, roots)
    }
    // Compatibility constructor: each input names the lowest semantic bit in
    // one contiguous binary/four-way decision, in decision order.
    pub fn symbolic(nbits: u32, order: Vec<u32>) -> Self {
        let step = N.trailing_zeros();
        assert_eq!(order.len() as u32, nbits.div_ceil(step));
        let flat = order
            .into_iter()
            .flat_map(|base| (base..base + step).rev())
            .filter(|&c| c < nbits)
            .collect();
        Self::symbolic_order(nbits, flat)
    }
    pub fn symbolic_order(nbits: u32, mut order: Vec<u32>) -> Self {
        assert!(N == 2 || N == 4);
        validate_order(nbits, &order);
        let step = N.trailing_zeros() as usize;
        let padded = order.len() % step != 0;
        if padded {
            order.insert(0, nbits);
        }
        let groups = order
            .chunks(step)
            .map(|chunk| {
                let mut group = [u32::MAX; 2];
                for (i, &coord) in chunk.iter().rev().enumerate() {
                    group[i] = coord;
                }
                group
            })
            .collect();
        let mut out = Self {
            n: 1usize << nbits,
            order: groups,
            nodes: vec![Node {
                var: u32::MAX,
                children: [0; N],
            }],
            unique: FxHashMap::default(),
            apply: FxHashMap::default(),
            support: 1,
            pairs: vec![(0, 1)],
            pair_index: FxHashMap::from_iter([((0, 1), 0)]),
        };
        if padded {
            out.support = out.raw_literal(nbits) ^ 1;
            out.pairs = vec![(0, out.support)];
            out.pair_index = FxHashMap::from_iter([((0, out.support), 0)]);
        }
        out
    }
    fn mk(&mut self, var: u32, mut children: [u32; N]) -> u32 {
        if children.iter().all(|&r| r == children[0]) {
            return children[0];
        }
        let flip = children[N - 1] & 1;
        if flip != 0 {
            for r in &mut children {
                *r ^= 1;
            }
        }
        let node = Node { var, children };
        if let Some(&r) = self.unique.get(&node) {
            return r ^ flip;
        }
        assert!(
            self.nodes.len() < 2_000_000,
            "RESOURCE_CAP: decision diagram nodes"
        );
        let r = (self.nodes.len() as u32) * 2;
        self.nodes.push(node);
        self.unique.insert(node, r);
        r ^ flip
    }
    fn var(&self, r: u32) -> u32 {
        if r < 2 {
            u32::MAX
        } else {
            self.nodes[(r / 2) as usize].var
        }
    }
    fn child(&self, r: u32, var: u32, c: usize) -> u32 {
        if self.var(r) == var {
            self.nodes[(r / 2) as usize].children[c] ^ (r & 1)
        } else {
            r
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
        let v = self.var(a).min(self.var(b));
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            *c = self.raw_op(op, self.child(a, v, i), self.child(b, v, i));
        }
        let r = self.mk(v, children);
        self.apply.insert((op, a, b), r);
        r
    }
    fn root(&self, id: Id) -> u32 {
        if P {
            let (a, b) = self.pairs[(id / 2) as usize];
            if id & 1 == 0 { a } else { b }
        } else {
            id as u32
        }
    }
    fn seal(&mut self, raw: u32) -> Id {
        let t = self.raw_op(8, self.support, raw);
        if !P {
            return t as Id;
        }
        let f = self.raw_op(8, self.support, t ^ 1);
        let key = (t.min(f), t.max(f));
        let id = if let Some(&i) = self.pair_index.get(&key) {
            i
        } else {
            let i = self.pairs.len() as u32;
            self.pairs.push(key);
            self.pair_index.insert(key, i);
            i
        };
        id as Id * 2 + (t > f) as Id
    }
    fn build(&mut self, b: &[u64], level: usize, prefix: usize) -> u32 {
        if level == self.order.len() {
            return (prefix < self.n && at(b, prefix)) as u32;
        }
        let group = self.order[level];
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            let w = (0..N.trailing_zeros() as usize)
                .fold(prefix, |w, bit| w | (((i >> bit) & 1) << group[bit]));
            *c = self.build(b, level + 1, w);
        }
        self.mk(level as u32, children)
    }
    fn eval(&self, mut r: u32, w: usize) -> bool {
        while r >= 2 {
            let node = self.nodes[(r / 2) as usize];
            let group = self.order[node.var as usize];
            let c = (0..N.trailing_zeros() as usize)
                .fold(0, |c, bit| c | (((w >> group[bit]) & 1) << bit));
            r = node.children[c] ^ (r & 1);
        }
        r != 0
    }
    fn quantify_children(&mut self, v: u32, old: [u32; N], mask: u64) -> u32 {
        let group = self.order[v as usize];
        let local = (0..N.trailing_zeros() as usize)
            .fold(0, |m, bit| m | (((mask >> group[bit]) as usize & 1) << bit));
        if local == 0 {
            return self.mk(v, old);
        }
        let mut out = [0; N];
        for (i, r) in out.iter_mut().enumerate() {
            for (j, &c) in old.iter().enumerate() {
                if i & !local == j & !local {
                    *r = self.raw_op(14, *r, c);
                }
            }
        }
        self.mk(v, out)
    }
    fn abstract_rec(&mut self, r: u32, mask: u64, memo: &mut FxHashMap<u32, u32>) -> u32 {
        if r < 2 {
            return r;
        }
        if let Some(&v) = memo.get(&r) {
            return v;
        }
        let var = self.var(r);
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            *c = self.abstract_rec(self.child(r, var, i), mask, memo);
        }
        let out = self.quantify_children(var, children, mask);
        memo.insert(r, out);
        out
    }
    fn relprod_rec(
        &mut self,
        a: u32,
        b: u32,
        mask: u64,
        memo: &mut FxHashMap<(u32, u32), u32>,
    ) -> u32 {
        if a == 0 || b == 0 {
            return 0;
        }
        if a < 2 && b < 2 {
            return a & b;
        }
        let key = (a.min(b), a.max(b));
        if let Some(&r) = memo.get(&key) {
            return r;
        }
        let var = self.var(a).min(self.var(b));
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            *c = self.relprod_rec(self.child(a, var, i), self.child(b, var, i), mask, memo);
        }
        let r = self.quantify_children(var, children, mask);
        memo.insert(key, r);
        r
    }
    fn raw_literal(&mut self, coordinate: u32) -> u32 {
        let step = N.trailing_zeros() as usize;
        let var = self
            .order
            .iter()
            .position(|p| p[..step].contains(&coordinate))
            .unwrap();
        let bit = self.order[var][..step]
            .iter()
            .position(|&c| c == coordinate)
            .unwrap();
        self.mk(var as u32, std::array::from_fn(|i| ((i >> bit) & 1) as u32))
    }
    pub fn literal(&mut self, coordinate: u32) -> Id {
        let raw = self.raw_literal(coordinate);
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
    fn permute_rec(&mut self, r: u32, literals: &[u32], memo: &mut FxHashMap<u32, u32>) -> u32 {
        if r < 2 {
            return r;
        }
        if r & 1 != 0 {
            return self.permute_rec(r ^ 1, literals, memo) ^ 1;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let node = self.nodes[(r / 2) as usize];
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            *c = self.permute_rec(node.children[i], literals, memo);
        }
        let group = self.order[node.var as usize];
        let mut size = N;
        for bit in 0..N.trailing_zeros() as usize {
            for j in 0..size / 2 {
                children[j] = self.select(
                    literals[group[bit] as usize],
                    children[2 * j],
                    children[2 * j + 1],
                );
            }
            size /= 2;
        }
        memo.insert(r, children[0]);
        children[0]
    }
    fn permute_raw(&mut self, r: u32, map: &Permutation) -> u32 {
        assert_eq!(self.n, 1usize << map.destinations.len());
        // An odd semantic width in a four-way diagram has a padded high bit.
        // It remains fixed; it is not a coordinate the caller may rename.
        let internal_bits = self.order.len() as u32 * N.trailing_zeros();
        let literals: Vec<_> = (0..internal_bits)
            .map(|old| self.raw_literal(map.destinations.get(old as usize).copied().unwrap_or(old)))
            .collect();
        self.permute_rec(r, &literals, &mut FxHashMap::default())
    }
    fn count_raw(&self, r: u32, level: u32, memo: &mut FxHashMap<(u32, u32), u64>) -> u64 {
        if r == 0 {
            return 0;
        }
        if r == 1 {
            return (N as u64).pow(self.order.len() as u32 - level);
        }
        if let Some(&v) = memo.get(&(r, level)) {
            return v;
        }
        let var = self.var(r);
        let sum = (0..N)
            .map(|i| self.count_raw(self.child(r, var, i), var + 1, memo))
            .sum::<u64>();
        let out = sum * (N as u64).pow(var - level);
        memo.insert((r, level), out);
        out
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
        let level = if r < 2 {
            self.order.len()
        } else {
            self.var(r) as usize
        };
        let mut out = if r == 1 { plan.one() } else { plan.zero() };
        let step = N.trailing_zeros() as usize;
        if r >= 2 {
            let table = plan.table(&self.order[level][..step]);
            for i in 0..N {
                let child =
                    self.spectrum_raw(self.child(r, level as u32, i), level + 1, plan, memo);
                for (shift, count) in table.terms(0, 1u64 << i) {
                    plan.add_shifted(&mut out, &child, shift, count);
                }
            }
        }
        plan.smooth(
            &mut out,
            self.order[from..level]
                .iter()
                .flat_map(|g| g[..step].iter().copied()),
        );
        memo.insert((r, from), out.clone());
        out
    }
}
impl<const N: usize, const P: bool> RegionOps for Diagram<N, P> {
    const BIT_COMPLEMENT: bool = P;
    fn dimensions(&self) -> u32 {
        self.n.next_power_of_two().trailing_zeros()
    }

    fn variable(&mut self, coordinate: u32) -> Id {
        self.literal(coordinate)
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> super::transfer::Packet {
        self.raw_packet(
            names,
            &roots.iter().map(|&r| self.root(r)).collect::<Vec<_>>(),
        )
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        assert_eq!(
            plan.group_of.len() as u32,
            self.n.next_power_of_two().trailing_zeros()
        );
        for group in &self.order {
            plan.prepare_table(&group[..N.trailing_zeros() as usize]);
        }
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum {
        self.spectrum_raw(self.root(event), 0, plan, &mut FxHashMap::default())
    }
    const NAME: &'static str = if N == 4 {
        "mdd4-pair"
    } else if P {
        "bdd-pair"
    } else {
        "bdd-root"
    };

    fn import(&mut self, b: &[u64]) -> Id {
        let raw = self.build(b, 0, 0);
        self.seal(raw)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        let r = self.raw_op(op, self.root(a), self.root(b));
        self.seal(r)
    }
    fn not(&mut self, a: Id) -> Id {
        if P {
            a ^ 1
        } else {
            let r = self.root(a) ^ 1;
            self.seal(r)
        }
    }
    fn empty(&self) -> Id {
        0
    }
    fn full(&self) -> Id {
        if P { 1 } else { self.support as Id }
    }
    fn export(&self, a: Id) -> Vec<u64> {
        bits(self.n, |w| self.eval(self.root(a), w))
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        let r = self.abstract_rec(self.root(a), mask, &mut FxHashMap::default());
        self.seal(r)
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        let r = self.permute_raw(self.root(a), map);
        self.seal(r)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let r = self.relprod_rec(self.root(a), self.root(b), mask, &mut FxHashMap::default());
        self.seal(r)
    }
    fn bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node<N>>()
            + self.unique.capacity() * (std::mem::size_of::<Node<N>>() + 16)
            + self.apply.capacity() * 24
            + self.pairs.capacity() * 8
            + self.pair_index.capacity() * 24
            + self.order.capacity() * 8
    }
    fn nodes(&self) -> usize {
        self.nodes.len()
    }
    fn count(&self, a: Id) -> u64 {
        self.count_raw(self.root(a), 0, &mut FxHashMap::default())
    }
}

impl<const N: usize, const P: bool> Carrier for Diagram<N, P> {
    fn full_space(nbits: u32, order: Vec<u32>) -> Result<Self, &'static str> {
        Ok(Self::symbolic_order(nbits, order))
    }

    fn new(support: &[u64], n: usize) -> Self {
        let nbits = n.next_power_of_two().trailing_zeros();
        Self::with_order(support, n, (0..nbits).rev().collect())
    }

    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        let mut d = Self::symbolic_order(n.next_power_of_two().trailing_zeros(), order);
        d.n = n;
        let s = d.build(support, 0, 0);
        assert!(s != 0);
        d.support = s;
        d.pairs = vec![(0, s)];
        d.pair_index = FxHashMap::from_iter([((0, s), 0)]);
        d
    }
}

// One representative per complementary pair: the side that excludes an anchor
// world in support. Canonicality follows from the fixed BDD order and anchor.
// The event polarity is distinct from the raw BDD's complemented-edge bit.
#[derive(Clone)]
pub struct Anchored {
    inner: Diagram<2, false>,
    anchor: usize,
    population: u64,
}
impl Anchored {
    fn seal(&mut self, raw: u32) -> Id {
        let flip = self.inner.eval(raw, self.anchor) as u32;
        let root = self.inner.raw_op(8, self.inner.support, raw ^ flip);
        (root as Id) * 2 + flip as Id
    }
    fn true_root(&mut self, a: Id) -> u32 {
        let raw = (a / 2) as u32;
        if a & 1 == 0 {
            raw
        } else {
            self.inner.raw_op(4, self.inner.support, raw)
        }
    }
    pub fn symbolic(nbits: u32, order: Vec<u32>) -> Self {
        Self {
            inner: Diagram::symbolic(nbits, order),
            anchor: 0,
            population: 1u64 << nbits,
        }
    }
    pub fn literal(&mut self, coordinate: u32) -> Id {
        let raw = self.inner.literal(coordinate) as u32;
        self.seal(raw)
    }
}
impl RegionOps for Anchored {
    fn dimensions(&self) -> u32 {
        self.inner.n.next_power_of_two().trailing_zeros()
    }

    fn variable(&mut self, coordinate: u32) -> Id {
        self.literal(coordinate)
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> super::transfer::Packet {
        self.inner.raw_packet(
            names,
            &roots
                .iter()
                .map(|&r| ((r / 2) as u32) ^ ((r & 1) as u32))
                .collect::<Vec<_>>(),
        )
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        self.inner.prepare_spectrum(plan);
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum {
        let mut memo = FxHashMap::default();
        let part = self
            .inner
            .spectrum_raw((event >> 1) as u32, 0, plan, &mut memo);
        if event & 1 == 0 {
            part
        } else {
            plan.subtract(
                self.inner
                    .spectrum_raw(self.inner.support, 0, plan, &mut memo),
                &part,
            )
        }
    }
    const NAME: &'static str = "bdd-anchored";

    fn import(&mut self, b: &[u64]) -> Id {
        let raw = self.inner.build(b, 0, 0);
        self.seal(raw)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        if let Some(r) = trivial(op, a, b) {
            return r;
        }
        let mut code = 0;
        for cell in 0..4 {
            let original = cell ^ ((a & 1) as u8 * 2) ^ (b & 1) as u8;
            code |= ((op >> original) & 1) << cell;
        }
        let flip = code & 1;
        if flip != 0 {
            code ^= 15;
        }
        // Both representatives vanish outside support and at the anchor.
        // code(0,0)=0, so the result does too; no support mask or pair needed.
        let raw = self.inner.raw_op(code, (a / 2) as u32, (b / 2) as u32);
        (raw as Id) * 2 + flip as Id
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
        bits(self.inner.n, |w| {
            self.inner.eval(self.inner.support, w)
                && (self.inner.eval((a / 2) as u32, w) ^ (a & 1 != 0))
        })
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        let root = self.true_root(a);
        let raw = self
            .inner
            .abstract_rec(root, mask, &mut FxHashMap::default());
        self.seal(raw)
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        let root = self.true_root(a);
        let raw = self.inner.permute_raw(root, map);
        self.seal(raw)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let a = self.true_root(a);
        let b = self.true_root(b);
        let raw = self
            .inner
            .relprod_rec(a, b, mask, &mut FxHashMap::default());
        self.seal(raw)
    }
    fn count(&self, a: Id) -> u64 {
        let n = self
            .inner
            .count_raw((a / 2) as u32, 0, &mut FxHashMap::default());
        if a & 1 == 0 { n } else { self.population - n }
    }
    fn bytes(&self) -> usize {
        self.inner.bytes() + 16
    }
    fn nodes(&self) -> usize {
        self.inner.nodes()
    }
}

impl Carrier for Anchored {
    fn full_space(nbits: u32, order: Vec<u32>) -> Result<Self, &'static str> {
        Ok(Self::symbolic(nbits, order))
    }

    fn new(support: &[u64], n: usize) -> Self {
        let nbits = n.next_power_of_two().trailing_zeros();
        Self::with_order(support, n, (0..nbits).rev().collect())
    }

    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        let anchor = support
            .iter()
            .enumerate()
            .find_map(|(i, &w)| (w != 0).then(|| i * 64 + w.trailing_zeros() as usize))
            .unwrap();
        Self {
            inner: Diagram::with_order(support, n, order),
            anchor,
            population: support.iter().map(|w| w.count_ones() as u64).sum(),
        }
    }
}
