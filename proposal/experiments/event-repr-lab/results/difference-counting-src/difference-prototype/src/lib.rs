//! A matched Shannon / positive-Davio / negative-Davio raw normal-form probe.
//! One fixed basis per semantic coordinate. No scoped Event or speed claim yet.
use rustc_hash::{FxHashMap, FxHashSet};

pub type Ref = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis { Shannon, Positive, Negative }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel { Coefficients, Cofactors }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Node { variables: u64, edges: [Ref; 2] }
const _: () = assert!(std::mem::size_of::<Node>() == 16);

#[derive(Clone)]
pub struct Arena {
    order: Vec<u32>,
    rank: Vec<usize>,
    basis: Vec<Basis>,
    kernel: Kernel,
    nodes: Vec<Node>,
    unique: FxHashMap<Node, Ref>,
    applications: FxHashMap<(u8, Ref, Ref), Ref>,
    cofactors: FxHashMap<(Ref, u32, bool), Ref>,
    projections: FxHashMap<(Ref, u64), Ref>,
    pub apply_misses: usize,
    pub cofactor_misses: usize,
    pub projection_misses: usize,
}

fn trivial(op: u8, a: Ref, b: Ref) -> Option<Ref> {
    let unary = |x, code: u8| match code & 3 { 0 => 0, 1 => x ^ 1, 2 => x, _ => 1 };
    Some(match op {
        0 => 0, 15 => 1, 12 => a, 10 => b, 3 => a ^ 1, 5 => b ^ 1,
        _ if a < 2 => unary(b, op >> (2 * a)),
        _ if b < 2 => unary(a, ((op >> b) & 1) | (((op >> (2 + b)) & 1) << 1)),
        _ if a == b => unary(a, (op & 1) | ((op >> 2) & 2)),
        _ if a == (b ^ 1) => unary(a, op >> 1),
        _ => return None,
    })
}

impl Arena {
    pub fn new(order: Vec<u32>, basis: Vec<Basis>) -> Self {
        Self::with_kernel(order,basis,Kernel::Coefficients)
    }
    pub fn with_kernel(order: Vec<u32>, basis: Vec<Basis>, kernel: Kernel) -> Self {
        let n = order.len();
        assert!(n < 63 && basis.len() == n);
        let mut rank = vec![usize::MAX; n];
        for (i, &v) in order.iter().enumerate() {
            assert!((v as usize) < n && rank[v as usize] == usize::MAX);
            rank[v as usize] = i;
        }
        Self { order, rank, basis, kernel, nodes: vec![Node { variables: 0, edges: [0, 0] }],
            unique: FxHashMap::default(), applications: FxHashMap::default(),
            cofactors: FxHashMap::default(), projections: FxHashMap::default(),
            apply_misses: 0, cofactor_misses: 0, projection_misses: 0 }
    }
    pub fn variables(&self, r: Ref) -> u64 { self.nodes[(r / 2) as usize].variables }
    fn top(&self, mask: u64) -> u32 {
        *self.order.iter().find(|&&v| mask >> v & 1 != 0).expect("nonconstant")
    }
    fn mk(&mut self, v: u32, mut edges: [Ref; 2]) -> Ref {
        let b = self.basis[v as usize];
        if (b == Basis::Shannon && edges[0] == edges[1]) || (b != Basis::Shannon && edges[1] == 0) {
            return edges[0];
        }
        let children = self.variables(edges[0]) | self.variables(edges[1]);
        assert!(children >> v & 1 == 0);
        debug_assert!(self.order.iter().take(self.rank[v as usize]).all(|&w| children >> w & 1 == 0));
        let flip = edges[0] & 1;
        edges[0] ^= flip;
        if b == Basis::Shannon { edges[1] ^= flip; }
        let node = Node { variables: children | (1 << v), edges };
        let r = if let Some(&r) = self.unique.get(&node) { r } else {
            assert!(self.nodes.len() < 2_000_000, "RESOURCE_CAP: coefficient nodes");
            let r = (self.nodes.len() as Ref) * 2;
            self.nodes.push(node); self.unique.insert(node, r); r
        };
        r ^ flip
    }
    // The returned pair uses the selected coordinate's fixed basis, even when
    // that coordinate is absent from this operand.
    fn coefficients(&self, r: Ref, v: u32) -> [Ref; 2] {
        let b = self.basis[v as usize];
        if self.variables(r) >> v & 1 == 0 {
            return [r, if b == Basis::Shannon { r } else { 0 }];
        }
        assert_eq!(self.top(self.variables(r)), v);
        let mut e = self.nodes[(r / 2) as usize].edges;
        e[0] ^= r & 1;
        if b == Basis::Shannon { e[1] ^= r & 1; }
        e
    }
    pub fn variable(&mut self, v: u32) -> Ref {
        let e = match self.basis[v as usize] {
            Basis::Shannon | Basis::Positive => [0, 1], Basis::Negative => [1, 1],
        };
        self.mk(v, e)
    }
    pub fn apply(&mut self, mut op: u8, mut a: Ref, mut b: Ref) -> Ref {
        assert!(op < 16);
        if let Some(r) = trivial(op, a, b) { return r; }
        if a > b {
            std::mem::swap(&mut a, &mut b);
            op = (op & 9) | ((op & 2) << 1) | ((op & 4) >> 1);
        }
        if let Some(&r) = self.applications.get(&(op, a, b)) { return r; }
        self.apply_misses += 1;
        let v = self.top(self.variables(a) | self.variables(b));
        // Control: expand ordinary cofactors for nonlinear operations, retaining
        // the native linear coefficient operation for XOR/affine truth tables.
        // Inputs and the fixed-basis canonical normal form are unchanged.
        if self.kernel == Kernel::Cofactors && self.basis[v as usize] != Basis::Shannon && op.count_ones() & 1 != 0 {
            let a0=self.cofactor(a,v,false);let a1=self.cofactor(a,v,true);
            let b0=self.cofactor(b,v,false);let b1=self.cofactor(b,v,true);
            let low=self.apply(op,a0,b0);let high=self.apply(op,a1,b1);
            let out=self.from_cofactors(v,low,high);
            self.applications.insert((op,a,b),out);
            return out;
        }
        let [f, d] = self.coefficients(a, v);
        let [g, e] = self.coefficients(b, v);
        let base = self.apply(op, f, g);
        let second = if self.basis[v as usize] == Basis::Shannon {
            self.apply(op, d, e)
        } else {
            // Every binary truth function is c0 XOR cA*a XOR cB*b XOR cAB*a*b.
            // For a=f+t*d, b=g+t*e and Boolean t, t*t=t.
            let ca = ((op >> 2) ^ op) & 1 != 0;
            let cb = ((op >> 1) ^ op) & 1 != 0;
            let cab = op.count_ones() & 1 != 0;
            let mut delta = 0;
            if ca { delta = self.apply(6, delta, d); }
            if cb { delta = self.apply(6, delta, e); }
            if cab {
                let fe = self.apply(8, f, e);
                let gd = self.apply(8, g, d);
                let de = self.apply(8, d, e);
                delta = self.apply(6, delta, fe);
                delta = self.apply(6, delta, gd);
                delta = self.apply(6, delta, de);
            }
            delta
        };
        let out = self.mk(v, [base, second]);
        self.applications.insert((op, a, b), out);
        out
    }
    fn from_cofactors(&mut self, v: u32, lo: Ref, hi: Ref) -> Ref {
        if self.basis[v as usize] == Basis::Shannon { return self.mk(v, [lo, hi]); }
        let delta = self.apply(6, lo, hi);
        let base = if self.basis[v as usize] == Basis::Positive { lo } else { hi };
        self.mk(v, [base, delta])
    }
    pub fn cofactor(&mut self, r: Ref, variable: u32, high: bool) -> Ref {
        if self.variables(r) >> variable & 1 == 0 { return r; }
        let regular = r & !1;
        if let Some(&out) = self.cofactors.get(&(regular, variable, high)) { return out ^ (r & 1); }
        self.cofactor_misses += 1;
        let v = self.top(self.variables(regular));
        let e = self.coefficients(regular, v);
        let out = if v == variable {
            match self.basis[v as usize] {
                Basis::Shannon => e[high as usize],
                Basis::Positive if !high => e[0],
                Basis::Negative if high => e[0],
                _ => self.apply(6, e[0], e[1]),
            }
        } else {
            let a = self.cofactor(e[0], variable, high);
            let b = self.cofactor(e[1], variable, high);
            self.mk(v, [a, b])
        };
        self.cofactors.insert((regular, variable, high), out);
        out ^ (r & 1)
    }
    pub fn exists(&mut self, r: Ref, hidden: u64) -> Ref {
        let hidden = hidden & self.variables(r);
        if hidden == 0 { return r; }
        if let Some(&out) = self.projections.get(&(r, hidden)) { return out; }
        self.projection_misses += 1;
        let v = self.top(self.variables(r));
        let out = if hidden >> v & 1 != 0 && self.basis[v as usize] != Basis::Shannon {
            let [a, d] = self.coefficients(r, v);
            let either = self.apply(14, a, d);
            self.exists(either, hidden & !(1 << v))
        } else {
            // Existential abstraction is not linear over XOR. Remaining
            // coefficients cannot be projected independently.
            let lo = self.cofactor(r, v, false);
            let hi = self.cofactor(r, v, true);
            let lo = self.exists(lo, hidden & !(1 << v));
            let hi = self.exists(hi, hidden & !(1 << v));
            if hidden >> v & 1 != 0 { self.apply(14, lo, hi) }
            else { self.from_cofactors(v, lo, hi) }
        };
        self.projections.insert((r, hidden), out);
        out
    }
    pub fn substitute(&mut self, r: Ref, selectors: &[Ref]) -> Ref {
        assert_eq!(selectors.len(), self.order.len());
        fn visit(c: &mut Arena, r: Ref, selectors: &[Ref], memo: &mut FxHashMap<Ref, Ref>) -> Ref {
            if r < 2 { return r; }
            let regular = r & !1;
            if let Some(&out) = memo.get(&regular) { return out ^ (r & 1); }
            let v = c.top(c.variables(regular));
            let [a, b] = c.coefficients(regular, v);
            let a = visit(c, a, selectors, memo);
            let b = visit(c, b, selectors, memo);
            let x = selectors[v as usize];
            let out = match c.basis[v as usize] {
                Basis::Shannon => {
                    let low = c.apply(8, x ^ 1, a);
                    let high = c.apply(8, x, b);
                    c.apply(14, low, high)
                }
                Basis::Positive | Basis::Negative => {
                    let select = if c.basis[v as usize] == Basis::Positive { x } else { x ^ 1 };
                    let delta = c.apply(8, select, b);
                    c.apply(6, a, delta)
                }
            };
            memo.insert(regular, out);
            out ^ (r & 1)
        }
        visit(self, r, selectors, &mut FxHashMap::default())
    }
    pub fn evaluate(&self, r: Ref, world: u64) -> bool {
        if r < 2 { return r != 0; }
        let v = self.top(self.variables(r));
        let [a, b] = self.coefficients(r, v);
        let x = world >> v & 1 != 0;
        match self.basis[v as usize] {
            Basis::Shannon => self.evaluate(if x { b } else { a }, world),
            Basis::Positive => self.evaluate(a, world) ^ (x && self.evaluate(b, world)),
            Basis::Negative => self.evaluate(a, world) ^ (!x && self.evaluate(b, world)),
        }
    }
    /// Build a sparse GF(2) polynomial directly in an all-positive basis.
    /// A term is a set of coordinates; duplicate terms cancel, including 1.
    /// This visits at most order.len() times the normalized term count plus
    /// leaves before interning. No truth table or sequence of Apply calls.
    pub fn from_anf(&mut self, mut terms: Vec<u64>) -> Ref {
        assert!(self.basis.iter().all(|&b| b == Basis::Positive));
        assert!(terms.iter().all(|&t| t >> self.order.len() == 0));
        terms.sort_unstable();
        let mut normalized = Vec::new();
        let mut i = 0;
        while i < terms.len() {
            let mut j = i+1;
            while j < terms.len() && terms[j] == terms[i] { j += 1; }
            if (j-i) & 1 != 0 { normalized.push(terms[i]); }
            i = j;
        }
        fn visit(c: &mut Arena, terms: Vec<u64>) -> Ref {
            if terms.is_empty() { return 0; }
            let variables = terms.iter().fold(0, |m, &t| m | t);
            if variables == 0 { return 1; }
            let v = c.top(variables);
            let mut base = Vec::new(); let mut delta = Vec::new();
            for term in terms {
                if term >> v & 1 == 0 { base.push(term); }
                else { delta.push(term & !(1 << v)); }
            }
            let a = visit(c,base); let d = visit(c,delta);
            c.mk(v,[a,d])
        }
        visit(self,normalized)
    }
    /// Exact population on the FULL raw Boolean cube, with one weight per code.
    /// Not a count of decoded legal worlds or a designated correlated law.
    /// Ordinary cofactor expansion may construct many new coefficient nodes.
    pub fn raw_uniform_count(&mut self, root: Ref) -> u64 {
        fn level(c: &Arena, r: Ref) -> usize {
            if r < 2 { c.order.len() } else { c.rank[c.top(c.variables(r)) as usize] }
        }
        fn visit(c: &mut Arena, r: Ref, memo: &mut FxHashMap<Ref,u64>) -> u64 {
            if r < 2 { return r as u64; }
            let top = level(c,r);
            let regular = r & !1;
            let count = if let Some(&n) = memo.get(&regular) { n } else {
                let v = c.order[top];
                let lo = c.cofactor(regular,v,false);
                let hi = c.cofactor(regular,v,true);
                let a = visit(c,lo,memo) << (level(c,lo)-top-1);
                let b = visit(c,hi,memo) << (level(c,hi)-top-1);
                memo.insert(regular,a+b); a+b
            };
            if r & 1 != 0 { (1u64 << (c.order.len()-top))-count } else { count }
        }
        let top = level(self,root);
        visit(self,root,&mut FxHashMap::default()) << top
    }
    pub fn import(&mut self, truth: &[bool]) -> Ref {
        assert_eq!(truth.len(), 1 << self.order.len());
        fn visit(c: &mut Arena, t: &[bool], remaining: &[u32]) -> Ref {
            if remaining.is_empty() { return t[0] as Ref; }
            let v = *c.order.iter().find(|v| remaining.contains(v)).unwrap();
            let p = remaining.iter().position(|&w| w == v).unwrap();
            let insert = |w: usize, bit: usize| (w & ((1 << p)-1)) | (bit << p) | ((w >> p) << (p+1));
            let lo: Vec<_> = (0..t.len()/2).map(|i| t[insert(i, 0)]).collect();
            let hi: Vec<_> = (0..t.len()/2).map(|i| t[insert(i, 1)]).collect();
            let rest: Vec<_> = remaining.iter().copied().filter(|&w| w != v).collect();
            let (a, b) = if c.basis[v as usize] == Basis::Shannon {
                (lo, hi)
            } else {
                let delta: Vec<_> = lo.iter().zip(&hi).map(|(a,b)| a ^ b).collect();
                (if c.basis[v as usize] == Basis::Positive { lo } else { hi }, delta)
            };
            let a = visit(c, &a, &rest); let b = visit(c, &b, &rest);
            c.mk(v, [a, b])
        }
        let vars: Vec<_> = (0..self.order.len() as u32).collect();
        visit(self, truth, &vars)
    }
    pub fn nodes(&self) -> usize { self.nodes.len() }
    pub fn reachable(&self, roots: &[Ref]) -> usize {
        let mut seen = FxHashSet::default(); let mut pending = roots.to_vec();
        while let Some(r) = pending.pop() {
            if r < 2 || !seen.insert(r / 2) { continue; }
            pending.extend(self.nodes[(r / 2) as usize].edges);
        }
        seen.len()
    }
    pub fn bytes(&self) -> usize {
        self.nodes.capacity() * 16 + self.unique.capacity() * (16 + 4 + 1)
            + self.applications.capacity() * (std::mem::size_of::<((u8,Ref,Ref),Ref)>() + 1)
            + self.cofactors.capacity() * (std::mem::size_of::<((Ref,u32,bool),Ref)>() + 1)
            + self.projections.capacity() * (std::mem::size_of::<((Ref,u64),Ref)>() + 1)
            + self.order.capacity()*4 + self.rank.capacity()*std::mem::size_of::<usize>()
            + self.basis.capacity()*std::mem::size_of::<Basis>()
    }
}

#[cfg(test)]
mod tests;
