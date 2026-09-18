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
    order: Vec<u32>,
    nodes: Vec<Node<N>>,
    unique: FxHashMap<Node<N>, u32>,
    apply: FxHashMap<(u8, u32, u32), u32>,
    support: u32,
    pairs: Vec<(u32, u32)>,
    pair_index: FxHashMap<(u32, u32), u32>,
}
impl<const N: usize, const P: bool> Diagram<N, P> {
    pub fn symbolic(nbits: u32, order: Vec<u32>) -> Self {
        assert!(N == 2 || N == 4);
        let step = N.trailing_zeros();
        assert_eq!(order.len() as u32, nbits.div_ceil(step));
        let mut covered = 0u64;
        for &k in &order {
            let m = ((1u64 << step) - 1) << k;
            assert_eq!(covered & m, 0);
            covered |= m;
        }
        assert_eq!(covered, (1u64 << (step * order.len() as u32)) - 1);
        Self {
            n: 1usize << nbits,
            order,
            nodes: vec![Node {
                var: u32::MAX,
                children: [0; N],
            }],
            unique: FxHashMap::default(),
            apply: FxHashMap::default(),
            support: 1,
            pairs: vec![(0, 1)],
            pair_index: FxHashMap::from_iter([((0, 1), 0)]),
        }
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
        match op {
            0 => return 0,
            15 => return 1,
            12 => return a,
            10 => return b,
            3 => return a ^ 1,
            5 => return b ^ 1,
            _ => {}
        }
        if a == b {
            return match (op & 1, op & 8) {
                (0, 0) => 0,
                (0, _) => a,
                (_, 0) => a ^ 1,
                _ => 1,
            };
        }
        if a < 2 && b < 2 {
            return ((op >> (2 * a + b)) & 1) as u32;
        }
        if op == 8 {
            if a == 0 || b == 0 {
                return 0;
            }
            if a == 1 {
                return b;
            }
            if b == 1 {
                return a;
            }
            if a == b ^ 1 {
                return 0;
            }
        }
        if op == 14 {
            if a == 1 || b == 1 {
                return 1;
            }
            if a == 0 {
                return b;
            }
            if b == 0 {
                return a;
            }
            if a == b ^ 1 {
                return 1;
            }
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
        let shift = self.order[level];
        let mut children = [0; N];
        for (i, c) in children.iter_mut().enumerate() {
            *c = self.build(b, level + 1, prefix | (i << shift));
        }
        self.mk(level as u32, children)
    }
    fn eval(&self, mut r: u32, w: usize) -> bool {
        while r >= 2 {
            let node = self.nodes[(r / 2) as usize];
            let c = (w >> self.order[node.var as usize]) & (N - 1);
            r = node.children[c] ^ (r & 1);
        }
        r != 0
    }
    fn quantify_children(&mut self, v: u32, old: [u32; N], mask: u32) -> u32 {
        let local = ((mask >> self.order[v as usize]) as usize) & (N - 1);
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
    fn abstract_rec(&mut self, r: u32, mask: u32, memo: &mut FxHashMap<u32, u32>) -> u32 {
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
        mask: u32,
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
    pub fn literal(&mut self, coordinate: u32) -> Id {
        let step = N.trailing_zeros();
        let var = self
            .order
            .iter()
            .position(|&p| coordinate >= p && coordinate < p + step)
            .unwrap();
        let bit = coordinate - self.order[var];
        let raw = self.mk(var as u32, std::array::from_fn(|i| ((i >> bit) & 1) as u32));
        self.seal(raw)
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
}
impl<const N: usize, const P: bool> Carrier for Diagram<N, P> {
    const NAME: &'static str = if N == 4 {
        "mdd4-pair"
    } else if P {
        "bdd-pair"
    } else {
        "bdd-root"
    };
    fn new(support: &[u64], n: usize) -> Self {
        let nbits = n.next_power_of_two().trailing_zeros();
        let step = N.trailing_zeros();
        let order = (0..nbits.div_ceil(step)).rev().map(|i| i * step).collect();
        let mut d = Self::symbolic(nbits, order);
        d.n = n;
        let s = d.build(support, 0, 0);
        assert!(s != 0);
        d.support = s;
        d.pairs = vec![(0, s)];
        d.pair_index = FxHashMap::from_iter([((0, s), 0)]);
        d
    }
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
    fn exists(&mut self, a: Id, mask: u32) -> Id {
        let r = self.abstract_rec(self.root(a), mask, &mut FxHashMap::default());
        self.seal(r)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u32) -> Id {
        let r = self.relprod_rec(self.root(a), self.root(b), mask, &mut FxHashMap::default());
        self.seal(r)
    }
    fn bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node<N>>()
            + self.unique.capacity() * (std::mem::size_of::<Node<N>>() + 16)
            + self.apply.capacity() * 24
            + self.pairs.capacity() * 8
            + self.pair_index.capacity() * 24
    }
    fn nodes(&self) -> usize {
        self.nodes.len()
    }
    fn count(&self, a: Id) -> u64 {
        self.count_raw(self.root(a), 0, &mut FxHashMap::default())
    }
}
