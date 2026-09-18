//! Ordered six-bit decisions whose outgoing selectors are disjoint u64 sets.
//! Equal continuations share one selector; the final block is a truth table.
use super::carrier::*;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Default, Debug, Hash, PartialEq, Eq)]
#[repr(C)]
struct Edge {
    selector: u64,
    child: u32,
}
#[derive(Clone, Copy)]
#[repr(C)]
struct Branch {
    level: u32,
    start: u32,
    len: u32,
    next: u32,
}
#[derive(Clone)]
struct GroupMap {
    target: Option<usize>,
    local: Option<Permutation>,
}

#[derive(Clone)]
pub struct Block64 {
    n: usize,
    groups: Vec<Vec<u32>>,
    remaining: Vec<u32>,
    cells: Vec<usize>,
    masks: Vec<u64>,
    tail: usize,
    contiguous_tail: bool,
    branches: Vec<Branch>,
    edges: Vec<Edge>,
    unique: FxHashMap<u64, u32>,
    leaves: Vec<u64>,
    leaf_unique: FxHashMap<u64, u32>,
    apply: FxHashMap<(u8, u32, u32), u32>,
    support: u32,
    anchor: usize,
    population: u64,
}
impl Block64 {
    fn transport_raw(&self,r:u32,out:&mut super::transfer::Builder,memo:&mut FxHashMap<u32,u32>)->u32 {
        if r<2 {return r;}
        let base=r&!1;
        if let Some(&v)=memo.get(&base) {return v^(r&1);}
        let v=if r&2==0 {
            out.table(self.groups[self.tail].clone(),vec![self.leaves[(r>>2) as usize]])
        } else {
            let node=self.branches[(r>>2) as usize];
            let edges=&self.edges[node.start as usize..(node.start+node.len) as usize];
            let children=(0..self.cells[node.level as usize]).map(|i| {
                let child=edges.iter().find(|e|e.selector>>i&1!=0).unwrap().child;
                self.transport_raw(child,out,memo)
            }).collect();
            out.split(self.groups[node.level as usize].clone(),children)
        };
        memo.insert(base,v);v^(r&1)
    }
    pub fn symbolic(nbits: u32, order: Vec<u32>) -> Self {
        validate_order(nbits, &order);
        // Partial block first, so every full low block contains six coordinates.
        let first = if nbits == 0 {
            0
        } else {
            (nbits as usize - 1) % 6 + 1
        };
        let mut groups: Vec<Vec<u32>> = vec![order[..first].iter().rev().copied().collect()];
        groups.extend(
            order[first..]
                .chunks(6)
                .map(|g| g.iter().rev().copied().collect()),
        );
        let tail = groups.len() - 1;
        let mut remaining = vec![0; groups.len() + 1];
        for i in (0..groups.len()).rev() {
            remaining[i] = remaining[i + 1] + groups[i].len() as u32;
        }
        let cells: Vec<_> = groups.iter().map(|g| 1usize << g.len()).collect();
        let masks = cells
            .iter()
            .map(|&n| if n == 64 { !0 } else { (1u64 << n) - 1 })
            .collect();
        let contiguous_tail =
            groups[tail].len() == 6 && groups[tail].iter().enumerate().all(|(i, &c)| i as u32 == c);
        Self {
            n: 1usize << nbits,
            groups,
            remaining,
            cells,
            masks,
            tail,
            contiguous_tail,
            branches: vec![],
            edges: vec![],
            unique: FxHashMap::default(),
            leaves: vec![0],
            leaf_unique: FxHashMap::from_iter([(0, 0)]),
            apply: FxHashMap::default(),
            support: 1,
            anchor: 0,
            population: 1u64 << nbits,
        }
    }
    fn capacity(&self, extra: usize) {
        assert!(
            self.branches.len() + self.edges.len() + self.leaves.len() + extra < 2_000_000,
            "RESOURCE_CAP: block records including selector edges"
        );
    }
    fn leaf(&mut self, mut word: u64) -> u32 {
        word &= self.masks[self.tail];
        let flip = ((word >> (self.cells[self.tail] - 1)) & 1) as u32;
        if flip != 0 {
            word ^= self.masks[self.tail];
        }
        if let Some(&r) = self.leaf_unique.get(&word) {
            return r ^ flip;
        }
        self.capacity(1);
        let r = (self.leaves.len() as u32) << 2;
        self.leaves.push(word);
        self.leaf_unique.insert(word, r);
        r ^ flip
    }
    fn word(&self, r: u32) -> u64 {
        debug_assert_eq!(r & 2, 0);
        self.leaves[(r >> 2) as usize] ^ if r & 1 == 0 { 0 } else { self.masks[self.tail] }
    }
    fn level(&self, r: u32) -> usize {
        if r & 2 == 0 {
            self.tail
        } else {
            self.branches[(r >> 2) as usize].level as usize
        }
    }
    fn mk(&mut self, level: usize, edges: &mut [Edge]) -> u32 {
        debug_assert!(level < self.tail && !edges.is_empty());
        let high = 1u64 << (self.cells[level] - 1);
        let flip = edges.iter().find(|e| e.selector & high != 0).unwrap().child & 1;
        for edge in edges.iter_mut() {
            edge.child ^= flip;
        }
        edges.sort_unstable_by_key(|e| e.child);
        let mut len = 0;
        for i in 0..edges.len() {
            if len != 0 && edges[len - 1].child == edges[i].child {
                edges[len - 1].selector |= edges[i].selector;
            } else {
                edges[len] = edges[i];
                len += 1;
            }
        }
        let edges = &edges[..len];
        debug_assert_eq!(
            edges.iter().fold(0, |s, e| s | e.selector),
            self.masks[level]
        );
        debug_assert_eq!(
            edges.iter().map(|e| e.selector.count_ones()).sum::<u32>(),
            self.cells[level] as u32
        );
        if len == 1 {
            return edges[0].child ^ flip;
        }
        let h = hash(&(level, edges));
        let prior = self.unique.get(&h).copied().unwrap_or(u32::MAX);
        let mut i = prior;
        while i != u32::MAX {
            let node = self.branches[i as usize];
            if node.level as usize == level
                && &self.edges[node.start as usize..(node.start + node.len) as usize] == edges
            {
                return (i << 2 | 2) ^ flip;
            }
            i = node.next;
        }
        self.capacity(len + 1);
        let i = self.branches.len() as u32;
        self.branches.push(Branch {
            level: level as u32,
            start: self.edges.len() as u32,
            len: len as u32,
            next: prior,
        });
        self.edges.extend_from_slice(edges);
        self.unique.insert(h, i);
        (i << 2 | 2) ^ flip
    }
    fn mk_children(&mut self, level: usize, children: &[u32; 64]) -> u32 {
        let mut edges = [Edge::default(); 64];
        for i in 0..self.cells[level] {
            edges[i] = Edge {
                selector: 1u64 << i,
                child: children[i],
            };
        }
        self.mk(level, &mut edges[..self.cells[level]])
    }
    fn branches_at(&self, r: u32, level: usize, out: &mut [Edge; 64]) -> usize {
        if self.level(r) != level || r & 2 == 0 {
            out[0] = Edge {
                selector: self.masks[level],
                child: r,
            };
            return 1;
        }
        let node = self.branches[(r >> 2) as usize];
        let len = node.len as usize;
        out[..len].copy_from_slice(&self.edges[node.start as usize..node.start as usize + len]);
        for edge in &mut out[..len] {
            edge.child ^= r & 1;
        }
        len
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
        let out = if level == self.tail {
            self.leaf(binary(op, self.word(a), self.word(b)))
        } else {
            let (mut ae, mut be, mut out) = (
                [Edge::default(); 64],
                [Edge::default(); 64],
                [Edge::default(); 64],
            );
            let al = self.branches_at(a, level, &mut ae);
            let bl = self.branches_at(b, level, &mut be);
            let mut len = 0;
            for a in &ae[..al] {
                for b in &be[..bl] {
                    let selector = a.selector & b.selector;
                    if selector != 0 {
                        out[len] = Edge {
                            selector,
                            child: self.raw_op(op, a.child, b.child),
                        };
                        len += 1;
                    }
                }
            }
            self.mk(level, &mut out[..len])
        };
        self.apply.insert((op, a, b), out);
        out
    }
    fn local(&self, level: usize, world: usize) -> usize {
        self.groups[level]
            .iter()
            .enumerate()
            .fold(0, |a, (i, &c)| a | (((world >> c) & 1) << i))
    }
    fn eval(&self, mut r: u32, world: usize) -> bool {
        while r & 2 != 0 {
            let node = self.branches[(r >> 2) as usize];
            let cell = 1u64 << self.local(node.level as usize, world);
            r = self.edges[node.start as usize..(node.start + node.len) as usize]
                .iter()
                .find(|e| e.selector & cell != 0)
                .unwrap()
                .child
                ^ (r & 1);
        }
        self.word(r) >> self.local(self.tail, world) & 1 != 0
    }
    fn build(&mut self, bits: &[u64], level: usize, world: usize) -> u32 {
        if level == self.tail && self.contiguous_tail {
            return self.leaf(bits.get(world / 64).copied().unwrap_or(0));
        }
        let mut children = [0; 64];
        let mut word = 0;
        for a in 0..self.cells[level] {
            let w = self.groups[level]
                .iter()
                .enumerate()
                .fold(world, |w, (i, &c)| w | (((a >> i) & 1) << c));
            if level == self.tail {
                if w < self.n && at(bits, w) {
                    word |= 1u64 << a;
                }
            } else {
                children[a] = self.build(bits, level + 1, w);
            }
        }
        if level == self.tail {
            self.leaf(word)
        } else {
            self.mk_children(level, &children)
        }
    }
    fn seal(&mut self, raw: u32) -> Id {
        let flip = self.eval(raw, self.anchor) as u32;
        let root = self.raw_op(8, self.support, raw ^ flip);
        (root as Id) << 1 | flip as Id
    }
    fn true_root(&mut self, event: Id) -> u32 {
        let root = (event >> 1) as u32;
        if event & 1 == 0 {
            root
        } else {
            self.raw_op(4, self.support, root)
        }
    }
    fn local_mask(&self, level: usize, mask: u64) -> u64 {
        self.groups[level]
            .iter()
            .enumerate()
            .fold(0, |m, (i, &c)| m | (((mask >> c) & 1) << i))
    }
    fn reduce_leaf(&mut self, word: u64, mask: u64) -> u32 {
        let mut words = [word];
        abstract_dense(
            &mut words,
            self.cells[self.tail],
            self.local_mask(self.tail, mask),
        );
        self.leaf(words[0])
    }
    fn quantify(&mut self, level: usize, edges: &mut [Edge], mask: u64) -> u32 {
        let local = self.local_mask(level, mask);
        if local == 0 {
            return self.mk(level, edges);
        }
        if local == (1u64 << self.groups[level].len()) - 1 {
            return edges.iter().fold(0, |r, e| self.raw_op(14, r, e.child));
        }
        let mut children = [0; 64];
        for edge in edges {
            let mut selected = [edge.selector];
            abstract_dense(&mut selected, self.cells[level], local);
            let mut bits = selected[0];
            while bits != 0 {
                let i = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                children[i] = self.raw_op(14, children[i], edge.child);
            }
        }
        self.mk_children(level, &children)
    }
    fn abstract_rec(&mut self, r: u32, mask: u64, memo: &mut FxHashMap<u32, u32>) -> u32 {
        if r < 2 {
            return r;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let level = self.level(r);
        let out = if level == self.tail {
            self.reduce_leaf(self.word(r), mask)
        } else {
            let mut edges = [Edge::default(); 64];
            let len = self.branches_at(r, level, &mut edges);
            for edge in &mut edges[..len] {
                edge.child = self.abstract_rec(edge.child, mask, memo);
            }
            self.quantify(level, &mut edges[..len], mask)
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
        if let Some(&out) = memo.get(&key) {
            return out;
        }
        let level = self.level(a).min(self.level(b));
        let out = if level == self.tail {
            self.reduce_leaf(self.word(a) & self.word(b), mask)
        } else {
            let (mut ae, mut be, mut out) = (
                [Edge::default(); 64],
                [Edge::default(); 64],
                [Edge::default(); 64],
            );
            let al = self.branches_at(a, level, &mut ae);
            let bl = self.branches_at(b, level, &mut be);
            let mut len = 0;
            for a in &ae[..al] {
                for b in &be[..bl] {
                    let selector = a.selector & b.selector;
                    if selector != 0 {
                        out[len] = Edge {
                            selector,
                            child: self.product_rec(a.child, b.child, mask, memo),
                        };
                        len += 1;
                    }
                }
            }
            self.quantify(level, &mut out[..len], mask)
        };
        memo.insert(key, out);
        out
    }
    fn group_predicate(&mut self, level: usize, word: u64) -> u32 {
        let word = word & self.masks[level];
        if word == 0 {
            return 0;
        }
        if word == self.masks[level] {
            return 1;
        }
        if level == self.tail {
            return self.leaf(word);
        }
        self.mk(
            level,
            &mut [
                Edge {
                    selector: self.masks[level] ^ word,
                    child: 0,
                },
                Edge {
                    selector: word,
                    child: 1,
                },
            ],
        )
    }
    fn raw_literal(&mut self, coordinate: u32) -> u32 {
        let level = self
            .groups
            .iter()
            .position(|g| g.contains(&coordinate))
            .unwrap();
        let local = self.groups[level]
            .iter()
            .position(|&c| c == coordinate)
            .unwrap();
        let word = (0..self.cells[level])
            .filter(|&i| i >> local & 1 != 0)
            .fold(0, |w, i| w | 1u64 << i);
        self.group_predicate(level, word)
    }
    pub fn literal(&mut self, coordinate: u32) -> Id {
        let raw = self.raw_literal(coordinate);
        self.seal(raw)
    }
    fn group_maps(&self, map: &Permutation) -> Vec<GroupMap> {
        self.groups
            .iter()
            .map(|group| {
                let mapped: Vec<_> = group
                    .iter()
                    .map(|&c| map.destinations[c as usize])
                    .collect();
                let target = self
                    .groups
                    .iter()
                    .position(|g| g.len() == mapped.len() && mapped.iter().all(|c| g.contains(c)));
                let local = target.map(|target| {
                    Permutation::new(
                        mapped
                            .iter()
                            .map(|c| {
                                self.groups[target].iter().position(|v| v == c).unwrap() as u32
                            })
                            .collect(),
                    )
                    .unwrap()
                });
                GroupMap { target, local }
            })
            .collect()
    }
    fn mapped_word(word: u64, map: &GroupMap) -> u64 {
        let mut words = [word];
        map.local.as_ref().unwrap().dense(&mut words);
        words[0]
    }
    fn permute_preserving(
        &mut self,
        r: u32,
        maps: &[GroupMap],
        memo: &mut FxHashMap<u32, u32>,
    ) -> u32 {
        if r < 2 {
            return r;
        }
        if r & 1 != 0 {
            return self.permute_preserving(r ^ 1, maps, memo) ^ 1;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let level = self.level(r);
        let out = if level == self.tail {
            self.leaf(Self::mapped_word(self.word(r), &maps[level]))
        } else {
            let mut edges = [Edge::default(); 64];
            let len = self.branches_at(r, level, &mut edges);
            for edge in &mut edges[..len] {
                edge.selector = Self::mapped_word(edge.selector, &maps[level]);
                edge.child = self.permute_preserving(edge.child, maps, memo);
            }
            self.mk(level, &mut edges[..len])
        };
        memo.insert(r, out);
        out
    }
    fn select(&mut self, condition: u32, low: u32, high: u32) -> u32 {
        if low == high {
            return low;
        }
        let a = self.raw_op(2, condition, low);
        let b = self.raw_op(8, condition, high);
        self.raw_op(14, a, b)
    }
    fn selector_shannon(
        &mut self,
        level: usize,
        word: u64,
        start: usize,
        size: usize,
        literals: &[u32],
    ) -> u32 {
        if size == 1 {
            return ((word >> start) & 1) as u32;
        }
        let low = self.selector_shannon(level, word, start, size / 2, literals);
        let high = self.selector_shannon(level, word, start + size / 2, size / 2, literals);
        let coord = self.groups[level][size.trailing_zeros() as usize - 1] as usize;
        self.select(literals[coord], low, high)
    }
    fn mapped_selector(
        &mut self,
        level: usize,
        word: u64,
        maps: &[GroupMap],
        literals: &[u32],
        cache: &mut FxHashMap<(usize, u64), u32>,
    ) -> u32 {
        if let Some(&out) = cache.get(&(level, word)) {
            return out;
        }
        let out = if let Some(target) = maps[level].target {
            self.group_predicate(target, Self::mapped_word(word, &maps[level]))
        } else {
            self.selector_shannon(level, word, 0, self.cells[level], literals)
        };
        cache.insert((level, word), out);
        out
    }
    fn permute_rec(
        &mut self,
        r: u32,
        maps: &[GroupMap],
        literals: &[u32],
        memo: &mut FxHashMap<u32, u32>,
        selectors: &mut FxHashMap<(usize, u64), u32>,
    ) -> u32 {
        if r < 2 {
            return r;
        }
        if r & 1 != 0 {
            return self.permute_rec(r ^ 1, maps, literals, memo, selectors) ^ 1;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let level = self.level(r);
        let out = if level == self.tail {
            self.mapped_selector(level, self.word(r), maps, literals, selectors)
        } else {
            let mut edges = [Edge::default(); 64];
            let len = self.branches_at(r, level, &mut edges);
            let mut out = 0;
            for edge in &edges[..len] {
                let child = self.permute_rec(edge.child, maps, literals, memo, selectors);
                let selector =
                    self.mapped_selector(level, edge.selector, maps, literals, selectors);
                let term = self.raw_op(8, selector, child);
                out = self.raw_op(14, out, term);
            }
            out
        };
        memo.insert(r, out);
        out
    }
    fn count_raw(&self, r: u32, from: usize, memo: &mut FxHashMap<(u32, usize), u64>) -> u64 {
        if let Some(&count) = memo.get(&(r, from)) {
            return count;
        }
        let level = self.level(r);
        let count = if level == self.tail {
            self.word(r).count_ones() as u64
        } else {
            let node = self.branches[(r >> 2) as usize];
            self.edges[node.start as usize..(node.start + node.len) as usize]
                .iter()
                .map(|edge| {
                    edge.selector.count_ones() as u64
                        * self.count_raw(edge.child ^ (r & 1), level + 1, memo)
                })
                .sum()
        } << (self.remaining[from] - self.remaining[level]);
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
        let table = plan.table(&self.groups[level]);
        let mut out = if level == self.tail {
            table.contract(plan, &[self.word(r)], None)
        } else {
            let mut out = plan.zero();
            let node = self.branches[(r >> 2) as usize];
            for edge in &self.edges[node.start as usize..(node.start + node.len) as usize] {
                let child = self.spectrum_raw(edge.child ^ (r & 1), level + 1, plan, memo);
                for (shift, count) in table.terms(0, edge.selector) {
                    plan.add_shifted(&mut out, &child, shift, count);
                }
            }
            out
        };
        plan.smooth(&mut out, self.groups[from..level].iter().flatten().copied());
        memo.insert((r, from), out.clone());
        out
    }
}
impl Carrier for Block64 {
    fn dimensions(&self)->u32 {self.n.next_power_of_two().trailing_zeros()}
    fn full_space(nbits:u32,order:Vec<u32>)->Result<Self,&'static str> {Ok(Self::symbolic(nbits,order))}
    fn variable(&mut self,coordinate:u32)->Id {self.literal(coordinate)}
    fn packet(&self,names:&[u64],roots:&[Id])->super::transfer::Packet {
        assert_eq!(names.len(),self.dimensions() as usize);
        let order=self.groups.iter().flat_map(|g|g.iter().rev().copied()).collect();
        let mut out=super::transfer::Builder::new(names.to_vec(),order);
        let mut memo=FxHashMap::default();
        let support=self.transport_raw(self.support,&mut out,&mut memo);
        let roots=roots.iter().map(|&r|self.transport_raw((r>>1) as u32,&mut out,&mut memo)^((r&1) as u32)).collect();
        out.finish(support,roots)
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        assert_eq!(plan.group_of.len() as u32, self.remaining[0]);
        for group in &self.groups {
            plan.prepare_table(group);
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
    const NAME: &'static str = "block64";
    fn new(support: &[u64], n: usize) -> Self {
        Self::with_order(
            support,
            n,
            (0..n.next_power_of_two().trailing_zeros()).rev().collect(),
        )
    }
    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
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
    fn import(&mut self, bits: &[u64]) -> Id {
        let raw = self.build(bits, 0, 0);
        self.seal(raw)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        if let Some(out) = trivial(op, a, b) {
            return out;
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
        let raw = self.raw_op(code, (a >> 1) as u32, (b >> 1) as u32);
        (raw as Id) << 1 | flip as Id
    }
    fn not(&mut self, event: Id) -> Id {
        event ^ 1
    }
    fn empty(&self) -> Id {
        0
    }
    fn full(&self) -> Id {
        1
    }
    fn export(&self, event: Id) -> Vec<u64> {
        bits(self.n, |w| {
            self.eval(self.support, w) && (self.eval((event >> 1) as u32, w) ^ (event & 1 != 0))
        })
    }
    fn exists(&mut self, event: Id, mask: u64) -> Id {
        let raw = self.true_root(event);
        let raw = self.abstract_rec(raw, mask, &mut FxHashMap::default());
        self.seal(raw)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let a = self.true_root(a);
        let b = self.true_root(b);
        let raw = self.product_rec(a, b, mask, &mut FxHashMap::default());
        self.seal(raw)
    }
    fn permute(&mut self, event: Id, map: &Permutation) -> Id {
        assert_eq!(self.n, 1usize << map.destinations.len());
        let raw = self.true_root(event);
        let maps = self.group_maps(map);
        let raw = if maps.iter().enumerate().all(|(i, m)| m.target == Some(i)) {
            self.permute_preserving(raw, &maps, &mut FxHashMap::default())
        } else {
            let literals: Vec<_> = map
                .destinations
                .iter()
                .map(|&c| self.raw_literal(c))
                .collect();
            self.permute_rec(
                raw,
                &maps,
                &literals,
                &mut FxHashMap::default(),
                &mut FxHashMap::default(),
            )
        };
        self.seal(raw)
    }
    fn count(&self, event: Id) -> u64 {
        let count = self.count_raw((event >> 1) as u32, 0, &mut FxHashMap::default());
        if event & 1 == 0 {
            count
        } else {
            self.population - count
        }
    }
    fn bytes(&self) -> usize {
        self.branches.capacity() * 16
            + self.edges.capacity() * 16
            + self.unique.capacity() * 24
            + self.leaves.capacity() * 8
            + self.leaf_unique.capacity() * 24
            + self.apply.capacity() * 24
            + self.groups.capacity() * 24
            + self.groups.iter().map(|g| g.capacity() * 4).sum::<usize>()
            + self.remaining.capacity() * 4
            + self.cells.capacity() * 8
            + self.masks.capacity() * 8
    }
    fn nodes(&self) -> usize {
        self.branches.len() + self.leaves.len()
    }
}
