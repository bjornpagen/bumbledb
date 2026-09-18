//! Raw Boolean normal form, shared by the scoped carrier and standalone checker.
//! The enum/interner layout is an initial correctness implementation; timings
//! must include eventual scoped operations, observation and native Free Join.
use rustc_hash::FxHashMap;
#[path = "essential_words.rs"]
pub(super) mod word_kernels;
#[path = "essential_store.rs"]
mod storage;
use storage::{Body, BorrowedBody, Node, Store};
pub use storage::Statistics as StorageStatistics;

pub type Ref = u32;
pub enum View<'a> {
    Constant(bool),
    Table {
        variables: u64,
        words: &'a [u64],
        complemented: bool,
    },
    Branch {
        variable: u32,
        low: Ref,
        high: Ref,
    },
}
#[derive(Clone)]
pub struct Arena<const K: u32> {
    dimensions: u32,
    rank: Vec<u32>,
    store: Store,
    applications: FxHashMap<(u8, Ref, Ref), Ref>,
    cofactors: FxHashMap<(Ref, u32, bool), Ref>,
    word_kernels: bool,
    derived: bool,
    // Optional structural-cardinality certificate, indexed with nodes. This is
    // assignment counting, never a source-law probability or independence claim.
    counts: Vec<u64>,
}
pub fn axes(mut mask: u64) -> Vec<u32> {
    let mut out = Vec::new();
    while mask != 0 {
        let v = mask.trailing_zeros();
        out.push(v);
        mask &= mask - 1;
    }
    out
}
fn cell(words: &[u64], at: usize) -> bool {
    words[at / 64] >> (at % 64) & 1 != 0
}
fn words(cells: usize, f: impl Fn(usize) -> bool) -> Vec<u64> {
    let mut out = vec![0; cells.div_ceil(64)];
    for i in 0..cells {
        if f(i) {
            out[i / 64] |= 1 << (i % 64);
        }
    }
    out
}
fn cofactor_words(words_in: &[u64], count: u32, position: u32, high: bool) -> Vec<u64> {
    let lower = (1usize << position) - 1;
    words(1usize << (count - 1), |i| {
        let old = (i & lower) | ((high as usize) << position) | ((i >> position) << (position + 1));
        cell(words_in, old)
    })
}
fn scatter(local: usize, coordinates: &[u32]) -> u64 {
    coordinates
        .iter()
        .enumerate()
        .fold(0, |w, (i, &v)| w | (((local >> i) & 1) as u64) << v)
}
impl<const K: u32> Arena<K> {
    pub fn view(&self, r: Ref) -> View<'_> {
        let node = self.store.node(r);
        match node.body {
            BorrowedBody::Constant => View::Constant(r & 1 != 0),
            BorrowedBody::Table(words) => View::Table {
                variables: node.variables,
                words,
                complemented: r & 1 != 0,
            },
            BorrowedBody::Branch {
                variable,
                low,
                high,
            } => View::Branch {
                variable,
                low: low ^ (r & 1),
                high: high ^ (r & 1),
            },
        }
    }
    pub fn node_references(&self) -> impl Iterator<Item = Ref> + '_ {
        (0..self.store.len()).map(|i| i as Ref * 2)
    }
    pub fn nodes(&self) -> usize {
        self.store.len()
    }
    pub fn storage_statistics(&self) -> StorageStatistics {
        self.store.statistics()
    }
    pub fn layout(&self) -> &'static str { self.store.layout() }
    pub fn metadata_bytes(&self) -> usize {
        self.rank.capacity() * 4 + self.counts.capacity() * 8
    }
    pub fn cache_bytes(&self) -> usize {
        self.applications.capacity() * (std::mem::size_of::<((u8, Ref, Ref), Ref)>() + 1)
            + self.cofactors.capacity() * (std::mem::size_of::<((Ref, u32, bool), Ref)>() + 1)
    }
    pub fn bytes(&self) -> usize {
        self.metadata_bytes() + self.cache_bytes() + self.store.statistics().bytes()
    }
    pub fn new(dimensions: u32, order: &[u32]) -> Self {
        assert!(dimensions < 63 && (1..=12).contains(&K));
        assert_eq!(order.len(), dimensions as usize);
        let mut rank = vec![u32::MAX; order.len()];
        for (i, &v) in order.iter().enumerate() {
            assert!(v < dimensions && rank[v as usize] == u32::MAX);
            rank[v as usize] = i as u32;
        }
        Self {
            dimensions,
            rank,
            store: Store::new(std::env::var("EVENT_LAB_ESSENTIAL_LAYOUT").as_deref().unwrap_or("enum")),
            applications: FxHashMap::default(),
            cofactors: FxHashMap::default(),
            word_kernels: match std::env::var("EVENT_LAB_ESSENTIAL_KERNEL").as_deref() {
                Ok("scalar") => false,
                Ok("words" | "derived") | Err(_) => true,
                _ => panic!("unknown essential kernel"),
            },
            derived: std::env::var("EVENT_LAB_ESSENTIAL_KERNEL").as_deref() == Ok("derived"),
            counts: if std::env::var("EVENT_LAB_ESSENTIAL_KERNEL").as_deref() == Ok("derived") {
                vec![0]
            } else {
                Vec::new()
            },
        }
    }
    pub fn variables(&self, r: Ref) -> u64 {
        self.store.variables(r)
    }
    pub fn is_table(&self, r: Ref) -> bool {
        self.store.is_table(r)
    }
    fn top(&self, variables: u64) -> u32 {
        if self.derived {
            let mut remaining = variables;
            let mut best = variables.trailing_zeros();
            while remaining != 0 {
                let v = remaining.trailing_zeros();
                if self.rank[v as usize] < self.rank[best as usize] {
                    best = v;
                }
                remaining &= remaining - 1;
            }
            return best;
        }
        axes(variables)
            .into_iter()
            .min_by_key(|&v| self.rank[v as usize])
            .unwrap()
    }
    fn intern(&mut self, node: Node) -> Ref {
        if let Some(r) = self.store.find(&node) {
            return r;
        }
        assert!(
            self.store.len() < 2_000_000,
            "RESOURCE_CAP: essential raw nodes"
        );
        if self.derived {
            self.counts.push(self.node_count(&node));
        }
        self.store.insert(node)
    }
    pub fn evaluate(&self, r: Ref, world: u64) -> bool {
        let node = self.store.node(r);
        let value = match node.body {
            BorrowedBody::Constant => false,
            BorrowedBody::Table(data) => {
                let mut variables = node.variables;
                let (mut i, mut position) = (0usize, 0);
                while variables != 0 {
                    let v = variables.trailing_zeros();
                    i |= (((world >> v) & 1) as usize) << position;
                    variables &= variables - 1;
                    position += 1;
                }
                cell(data, i)
            }
            BorrowedBody::Branch {
                variable,
                low,
                high,
            } => self.evaluate(
                if world >> variable & 1 == 0 {
                    low
                } else {
                    high
                },
                world,
            ),
        };
        value ^ (r & 1 != 0)
    }
    fn split_words(&self, data: &[u64], count: u32, position: u32, high: bool) -> Vec<u64> {
        if self.word_kernels {
            word_kernels::cofactor(data, count, position, high)
        } else {
            cofactor_words(data, count, position, high)
        }
    }
    // This path is called only below the essential-variable cutoff. By the
    // normal-form invariant each operand is a table or a constant.
    fn aligned(&self, r: Ref, variables: u64) -> Vec<u64> {
        assert_eq!(self.variables(r) & !variables, 0);
        let dimensions = variables.count_ones();
        let mut data = vec![0; (1usize << dimensions).div_ceil(64)];
        if r < 2 {
            if r == 1 {
                data.fill(!0);
            }
            return data;
        }
        let node = self.store.node(r);
        let BorrowedBody::Table(source) = node.body else {
            panic!("table cutoff invariant");
        };
        let count = node.variables.count_ones();
        if count < 6 {
            let mut pattern = source[0];
            let mut cells = 1usize << count;
            while cells < 64 {
                pattern |= pattern << cells;
                cells *= 2;
            }
            data.fill(pattern);
        } else {
            for (i, word) in data.iter_mut().enumerate() {
                *word = source[i % source.len()];
            }
        }
        if r & 1 != 0 {
            for word in &mut data {
                *word = !*word;
            }
        }
        let target = axes(variables);
        let mut permutation: Vec<_> = axes(node.variables)
            .iter()
            .map(|v| target.iter().position(|w| w == v).unwrap() as u32)
            .collect();
        for v in 0..dimensions {
            if !permutation.contains(&v) {
                permutation.push(v);
            }
        }
        word_kernels::permute(&mut data, &permutation);
        data
    }
    // Input bit axis i is the ith set bit of variables. Only this importer is
    // explicitly bounded; the ambient presentation can have sixty coordinates.
    pub fn table(&mut self, mut variables: u64, mut data: Vec<u64>) -> Ref {
        assert!(variables >> self.dimensions == 0);
        assert!(
            variables.count_ones() <= 20,
            "RESOURCE_CAP: explicit local table"
        );
        let mut count = variables.count_ones();
        let cells = 1usize << count;
        assert_eq!(data.len(), cells.div_ceil(64));
        if cells < 64 {
            data[0] &= (1u64 << cells) - 1;
        }
        // Normalization removes all irrelevant axes, including those made
        // irrelevant by an operation. This implementation prioritizes clarity.
        let mut position = 0;
        while position < count {
            let redundant = if self.word_kernels {
                word_kernels::irrelevant(&data, position)
            } else {
                cofactor_words(&data, count, position, false)
                    == cofactor_words(&data, count, position, true)
            };
            if redundant {
                variables &= !(1u64 << axes(variables)[position as usize]);
                data = self.split_words(&data, count, position, false);
                count -= 1;
            } else {
                position += 1;
            }
        }
        if count <= K {
            return self.exact_table(variables, data);
        }
        let v = self.top(variables);
        let pos = axes(variables).iter().position(|&a| a == v).unwrap() as u32;
        let rest = variables & !(1 << v);
        let low = self.table(rest, self.split_words(&data, count, pos, false));
        let high = self.table(rest, self.split_words(&data, count, pos, true));
        self.branch(v, low, high)
    }
    // Private proof boundary: every listed axis is known to be essential.
    // Normalization proves this for arbitrary tables. A bijection preserves it;
    // a reduced ordered branch has exactly {v} union vars(low) union vars(high).
    fn exact_table(&mut self, variables: u64, mut data: Vec<u64>) -> Ref {
        let count = variables.count_ones();
        assert!(count <= K && variables >> self.dimensions == 0);
        let cells = 1usize << count;
        assert_eq!(data.len(), cells.div_ceil(64));
        if cells < 64 {
            data[0] &= (1u64 << cells) - 1;
        }
        if count == 0 {
            return data[0] as Ref;
        }
        let flip = cell(&data, cells - 1) as Ref;
        if flip != 0 {
            for w in &mut data {
                *w = !*w;
            }
            if cells < 64 {
                data[0] &= (1u64 << cells) - 1;
            }
        }
        self.intern(Node {
            variables,
            body: Body::Table(data),
        }) ^ flip
    }
    fn branch(&mut self, variable: u32, mut low: Ref, mut high: Ref) -> Ref {
        assert!(variable < self.dimensions);
        let child_variables = self.variables(low) | self.variables(high);
        assert!(
            axes(child_variables)
                .iter()
                .all(|&v| self.rank[v as usize] > self.rank[variable as usize])
        );
        if low == high {
            return low;
        }
        let variables = child_variables | (1 << variable);
        if variables.count_ones() <= K {
            let names = axes(variables);
            let data = if self.word_kernels {
                let low = self.aligned(low, variables);
                let high = self.aligned(high, variables);
                let position = names.iter().position(|&v| v == variable).unwrap() as u32;
                low.iter()
                    .zip(high)
                    .enumerate()
                    .map(|(i, (a, b))| {
                        let selector = word_kernels::literal_word(position, i);
                        (*a & !selector) | (b & selector)
                    })
                    .collect()
            } else {
                words(1usize << names.len(), |i| {
                    let w = scatter(i, &names);
                    self.evaluate(if w >> variable & 1 != 0 { high } else { low }, w)
                })
            };
            return if self.derived {
                self.exact_table(variables, data)
            } else {
                self.table(variables, data)
            };
        }
        let flip = high & 1;
        low ^= flip;
        high ^= flip;
        self.intern(Node {
            variables,
            body: Body::Branch {
                variable,
                low,
                high,
            },
        }) ^ flip
    }
    pub fn variable(&mut self, coordinate: u32) -> Ref {
        assert!(coordinate < self.dimensions);
        if self.derived {
            self.exact_table(1 << coordinate, vec![2])
        } else {
            self.table(1 << coordinate, vec![2])
        }
    }
    pub fn cofactor(&mut self, r: Ref, variable: u32, high: bool) -> Ref {
        assert!(variable < self.dimensions);
        if self.variables(r) >> variable & 1 == 0 {
            return r;
        }
        if self.derived {
            if let BorrowedBody::Branch {
                variable: v,
                low,
                high: upper,
            } = self.store.node(r).body
            {
                if v == variable {
                    return (if high { upper } else { low }) ^ (r & 1);
                }
            }
            // Cofactoring commutes with complement. Cache the regular root once.
            return self.cofactor_inner(r & !1, variable, high) ^ (r & 1);
        }
        self.cofactor_inner(r, variable, high)
    }
    fn cofactor_inner(&mut self, r: Ref, variable: u32, high: bool) -> Ref {
        if let Some(&out) = self.cofactors.get(&(r, variable, high)) {
            return out;
        }
        let node = self.store.owned(r);
        let out = match node.body {
            Body::Constant => unreachable!(),
            Body::Table(data) => {
                let position = axes(node.variables)
                    .iter()
                    .position(|&v| v == variable)
                    .unwrap() as u32;
                self.table(
                    node.variables & !(1 << variable),
                    self.split_words(&data, node.variables.count_ones(), position, high),
                )
            }
            Body::Branch {
                variable: top,
                low,
                high: upper,
            } => {
                if variable == top {
                    if high { upper } else { low }
                } else {
                    let low = self.cofactor(low, variable, high);
                    let upper = self.cofactor(upper, variable, high);
                    self.branch(top, low, upper)
                }
            }
        } ^ (r & 1);
        self.cofactors.insert((r, variable, high), out);
        out
    }
    pub fn apply(&mut self, mut op: u8, mut a: Ref, mut b: Ref) -> Ref {
        assert!(op < 16);
        let mut flip = 0;
        if self.derived {
            // Absorb edge polarity into the four-bit operation. Every regular
            // raw root is false at the all-one assignment, so bit zero selects
            // the result polarity. Sort operands and transpose the table too.
            if a & 1 != 0 {
                op = ((op & 3) << 2) | (op >> 2);
                a ^= 1;
            }
            if b & 1 != 0 {
                op = ((op & 5) << 1) | ((op & 10) >> 1);
                b ^= 1;
            }
            flip = (op & 1) as Ref;
            if flip != 0 {
                op ^= 15;
            }
            if a > b {
                std::mem::swap(&mut a, &mut b);
                op = (op & 9) | ((op & 2) << 1) | ((op & 4) >> 1);
            }
        }
        self.apply_inner(op, a, b) ^ flip
    }
    fn apply_inner(&mut self, op: u8, a: Ref, b: Ref) -> Ref {
        let unary = |x: Ref, code: u8| match code & 3 {
            0 => 0,
            1 => x ^ 1,
            2 => x,
            _ => 1,
        };
        match op {
            0 => return 0,
            15 => return 1,
            12 => return a,
            10 => return b,
            3 => return a ^ 1,
            5 => return b ^ 1,
            _ if a < 2 => return unary(b, op >> (2 * a)),
            _ if b < 2 => return unary(a, ((op >> b) & 1) | (((op >> (2 + b)) & 1) << 1)),
            _ if a == b => return unary(a, (op & 1) | ((op >> 2) & 2)),
            _ if a == (b ^ 1) => return unary(a, op >> 1),
            _ => {}
        }
        if let Some(&out) = self.applications.get(&(op, a, b)) {
            return out;
        }
        let variables = self.variables(a) | self.variables(b);
        let out = if variables.count_ones() <= K {
            let names = axes(variables);
            let data = if self.word_kernels {
                word_kernels::combine(op, &self.aligned(a, variables), &self.aligned(b, variables))
            } else {
                words(1usize << names.len(), |i| {
                    let w = scatter(i, &names);
                    let cell = 2 * self.evaluate(a, w) as u8 + self.evaluate(b, w) as u8;
                    op >> cell & 1 != 0
                })
            };
            self.table(variables, data)
        } else {
            let v = self.top(variables);
            let al = self.cofactor(a, v, false);
            let ah = self.cofactor(a, v, true);
            let bl = self.cofactor(b, v, false);
            let bh = self.cofactor(b, v, true);
            let low = self.apply(op, al, bl);
            let high = self.apply(op, ah, bh);
            self.branch(v, low, high)
        };
        self.applications.insert((op, a, b), out);
        out
    }
    pub fn exists(&mut self, r: Ref, variables: u64) -> Ref {
        assert!(variables >> self.dimensions == 0);
        self.abstract_rec(r, variables, &mut FxHashMap::default())
    }
    fn abstract_rec(&mut self, r: Ref, mask: u64, memo: &mut FxHashMap<(Ref, u64), Ref>) -> Ref {
        let mask = mask & self.variables(r);
        if mask == 0 {
            return r;
        }
        if let Some(&out) = memo.get(&(r, mask)) {
            return out;
        }
        let node = self.store.owned(r);
        let out = match node.body {
            Body::Constant => r,
            Body::Table(mut data) => {
                let mut variables = node.variables;
                let mut count = variables.count_ones();
                if r & 1 != 0 {
                    for w in &mut data {
                        *w = !*w;
                    }
                }
                for v in axes(mask) {
                    let position = axes(variables).iter().position(|&a| a == v).unwrap() as u32;
                    let low = self.split_words(&data, count, position, false);
                    let high = self.split_words(&data, count, position, true);
                    data = low.iter().zip(high).map(|(a, b)| *a | b).collect();
                    variables &= !(1 << v);
                    count -= 1;
                }
                self.table(variables, data)
            }
            Body::Branch {
                variable,
                low,
                high,
            } => {
                let low = self.abstract_rec(low ^ (r & 1), mask, memo);
                let high = self.abstract_rec(high ^ (r & 1), mask, memo);
                if mask >> variable & 1 != 0 {
                    self.apply(14, low, high)
                } else {
                    self.branch(variable, low, high)
                }
            }
        };
        memo.insert((r, mask), out);
        out
    }
    pub fn relprod(&mut self, a: Ref, b: Ref, mask: u64) -> Ref {
        assert!(mask >> self.dimensions == 0);
        self.product_rec(a, b, mask, &mut FxHashMap::default())
    }
    fn product_rec(
        &mut self,
        a: Ref,
        b: Ref,
        mask: u64,
        memo: &mut FxHashMap<(Ref, Ref, u64), Ref>,
    ) -> Ref {
        if a == 0 || b == 0 || a == (b ^ 1) {
            return 0;
        }
        let variables = self.variables(a) | self.variables(b);
        let mask = mask & variables;
        if mask == 0 {
            return self.apply(8, a, b);
        }
        let key = (a.min(b), a.max(b), mask);
        if let Some(&out) = memo.get(&key) {
            return out;
        }
        let out = if variables.count_ones() <= K {
            let local = self.apply(8, a, b);
            self.exists(local, mask)
        } else {
            let v = self.top(variables);
            let al = self.cofactor(a, v, false);
            let ah = self.cofactor(a, v, true);
            let bl = self.cofactor(b, v, false);
            let bh = self.cofactor(b, v, true);
            let low = self.product_rec(al, bl, mask, memo);
            let high = self.product_rec(ah, bh, mask, memo);
            if mask >> v & 1 != 0 {
                self.apply(14, low, high)
            } else {
                self.branch(v, low, high)
            }
        };
        memo.insert(key, out);
        out
    }
    pub fn rename(&mut self, r: Ref, destinations: &[u32]) -> Ref {
        assert_eq!(destinations.len(), self.dimensions as usize);
        let mut seen = 0u64;
        for &v in destinations {
            assert!(v < self.dimensions && seen >> v & 1 == 0);
            seen |= 1 << v;
        }
        self.rename_rec(r, destinations, &mut FxHashMap::default())
    }
    fn rename_rec(&mut self, r: Ref, destinations: &[u32], memo: &mut FxHashMap<Ref, Ref>) -> Ref {
        if r < 2 {
            return r;
        }
        if let Some(&out) = memo.get(&r) {
            return out;
        }
        let node = self.store.owned(r);
        let out = match node.body {
            Body::Constant => unreachable!(),
            Body::Table(data) => {
                let old = axes(node.variables);
                let variables = old
                    .iter()
                    .fold(0, |mask, &v| mask | (1 << destinations[v as usize]));
                let names = axes(variables);
                let mapped = if self.word_kernels {
                    let permutation: Vec<_> = old
                        .iter()
                        .map(|&v| {
                            names
                                .iter()
                                .position(|&w| w == destinations[v as usize])
                                .unwrap() as u32
                        })
                        .collect();
                    let mut data = data;
                    word_kernels::permute(&mut data, &permutation);
                    data
                } else {
                    words(1usize << names.len(), |i| {
                        let world = scatter(i, &names);
                        let index = old.iter().enumerate().fold(0, |index, (k, &v)| {
                            index | (((world >> destinations[v as usize]) & 1) as usize) << k
                        });
                        cell(&data, index)
                    })
                };
                if self.derived {
                    self.exact_table(variables, mapped)
                } else {
                    self.table(variables, mapped)
                }
            }
            Body::Branch {
                variable,
                low,
                high,
            } => {
                let low = self.rename_rec(low, destinations, memo);
                let high = self.rename_rec(high, destinations, memo);
                let variable = self.variable(destinations[variable as usize]);
                let high = self.apply(8, variable, high);
                let low = self.apply(4, low, variable);
                self.apply(14, low, high)
            }
        } ^ (r & 1);
        memo.insert(r, out);
        out
    }
    pub fn count(&self, r: Ref) -> u64 {
        if self.derived {
            return self.stored_count(r) << (self.dimensions - self.variables(r).count_ones());
        }
        self.count_local(r, &mut FxHashMap::default())
            << (self.dimensions - self.variables(r).count_ones())
    }
    fn stored_count(&self, r: Ref) -> u64 {
        let regular = self.counts[(r / 2) as usize];
        if r & 1 != 0 {
            (1u64 << self.variables(r).count_ones()) - regular
        } else {
            regular
        }
    }
    fn node_count(&self, node: &Node) -> u64 {
        match &node.body {
            Body::Constant => 0,
            Body::Table(words) => words.iter().map(|w| w.count_ones() as u64).sum(),
            Body::Branch { low, high, .. } => {
                let width = node.variables.count_ones();
                (self.stored_count(*low) << (width - 1 - self.variables(*low).count_ones()))
                    + (self.stored_count(*high) << (width - 1 - self.variables(*high).count_ones()))
            }
        }
    }
    fn count_local(&self, r: Ref, memo: &mut FxHashMap<Ref, u64>) -> u64 {
        if let Some(&n) = memo.get(&r) {
            return n;
        }
        let node = self.store.node(r);
        let width = node.variables.count_ones();
        let count = match node.body {
            BorrowedBody::Constant => 0,
            BorrowedBody::Table(words) => words.iter().map(|w| w.count_ones() as u64).sum(),
            BorrowedBody::Branch { low, high, .. } => {
                (self.count_local(low, memo) << (width - 1 - self.variables(low).count_ones()))
                    + (self.count_local(high, memo)
                        << (width - 1 - self.variables(high).count_ones()))
            }
        };
        let count = if r & 1 != 0 {
            (1u64 << width) - count
        } else {
            count
        };
        memo.insert(r, count);
        count
    }
    pub fn import(&mut self, data: Vec<u64>) -> Ref {
        assert!(self.dimensions <= 20);
        self.table((1u64 << self.dimensions) - 1, data)
    }
    pub fn export(&self, r: Ref) -> Vec<u64> {
        assert!(self.dimensions <= 20);
        words(1usize << self.dimensions, |w| self.evaluate(r, w as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slabs_collide_grow_clone_and_preserve_raw_ids() {
        let order: Vec<_> = (0..8).rev().collect();
        let mut control = Arena::<3>::new(8, &order);
        control.store = Store::new("enum");
        let mut trial = Arena::<3>::new(8, &order);
        trial.store = Store::new("slab");
        trial.store.collide_all();
        let mut roots = vec![];
        for seed in 0..48usize {
            let table = words(256, |w| ((w.wrapping_mul(331) ^ (seed * 117)) % 53) < 21);
            let a = control.import(table.clone());
            assert_eq!(a, trial.import(table.clone()));
            assert_eq!(a, trial.import(table));
            assert_eq!(trial.count(a), control.count(a));
            roots.push(a);
            let b = roots[seed / 2];
            for op in [2, 6, 8, 11, 14] {
                assert_eq!(trial.apply(op, a, b), control.apply(op, a, b));
            }
            assert_eq!(trial.exists(a, 0x25), control.exists(a, 0x25));
            assert_eq!(trial.cofactor(a, 3, true), control.cofactor(a, 3, true));
            assert_eq!(trial.relprod(a, b, 0x13), control.relprod(a, b, 0x13));
            assert_eq!(trial.rename(a, &order), control.rename(a, &order));
        }
        assert_eq!(trial.nodes(), control.nodes());
        assert!(trial.nodes() > 100, "exercise slab growth and a long collision chain");
        for r in control.node_references() {
            assert_eq!(control.store.owned(r), trial.store.owned(r));
            assert_eq!(trial.count(r), control.count(r));
        }
        let cloned = trial.clone();
        for r in roots {
            assert_eq!(cloned.export(r), control.export(r));
            assert_eq!(cloned.export(r ^ 1), control.export(r ^ 1));
        }
        let a = control.storage_statistics();
        let b = trial.storage_statistics();
        assert_eq!((a.records, a.tables, a.logical_words), (b.records, b.tables, b.logical_words));
        assert!(b.table_bytes < a.table_bytes, "one resident copy of table words");
    }
    fn exhaustive<const K: u32>() {
        for order in [vec![0, 1, 2, 3], vec![3, 2, 1, 0]] {
            let mut c = Arena::<K>::new(4, &order);
            let literals: Vec<_> = (0..4).map(|v| c.variable(v)).collect();
            for word in 0..65536u64 {
                let r = c.import(vec![word]);
                assert_eq!(c.export(r), vec![word]);
                assert_eq!(c.count(r), word.count_ones() as u64);
                assert_eq!(c.import(vec![word ^ 65535]), r ^ 1);
                if word % 257 != 0 {
                    continue;
                }
                for v in 0..4 {
                    let low = c.cofactor(r, v, false);
                    let high = c.cofactor(r, v, true);
                    let pos = c.apply(8, literals[v as usize], high);
                    let neg = c.apply(4, low, literals[v as usize]);
                    assert_eq!(c.apply(14, pos, neg), r);
                }
                let other = (word.wrapping_mul(331) ^ 0x5397) & 65535;
                let q = c.import(vec![other]);
                for op in 0..16 {
                    let expected = words(16, |w| {
                        op >> (2 * ((word >> w) & 1) + ((other >> w) & 1)) & 1 != 0
                    });
                    let output = c.apply(op, r, q);
                    assert_eq!(output, c.import(expected));
                }
                for mask in 0..16u64 {
                    let expected = words(16, |w| {
                        (0..16usize).any(|v| {
                            v & !(mask as usize) == w & !(mask as usize) && word >> v & 1 != 0
                        })
                    });
                    let output = c.exists(r, mask);
                    assert_eq!(output, c.import(expected));
                }
            }
        }
    }
    #[test]
    fn canonical_small_tables_and_branches() {
        exhaustive::<2>();
        exhaustive::<6>();
    }
    #[test]
    fn noncontiguous_sixty_coordinate_table() {
        let mut c = Arena::<6>::new(60, &(0..60).rev().collect::<Vec<_>>());
        let x = c.variable(0);
        let y = c.variable(59);
        let r = c.apply(6, x, y);
        assert!(c.is_table(r));
        assert_eq!(c.variables(r), 1 | (1u64 << 59));
        assert_eq!(c.exists(r, 1 << 59), 1);
        assert_eq!(c.apply(9, x, y), r ^ 1);
        assert_eq!(c.count(r), 1u64 << 59);
        assert_eq!(c.count(1), 1u64 << 60);
    }
    fn matched<const K: u32>() {
        let mut c = Arena::<K>::new(60, &(0..60).rev().collect::<Vec<_>>());
        let pool = [0, 4, 7, 10, 18, 21, 29, 31, 38, 39, 42, 55, 59];
        let reset = |c: &mut Arena<K>, mode| {
            c.word_kernels = mode != "scalar";
            c.derived = mode == "derived";
            c.applications.clear();
            c.cofactors.clear();
            c.counts.clear();
            if c.derived {
                for i in 0..c.store.len() {
                    c.counts.push(c.node_count(&c.store.owned(i as Ref * 2)));
                }
            }
        };
        for seed in 0..13usize {
            let a_vars = pool[..6].iter().fold(0, |s, &v| s | (1u64 << v));
            let b_vars = (0..6).fold(0, |s, i| s | (1u64 << pool[(i + seed) % pool.len()]));
            let a_data = words(64, |i| (i.wrapping_mul(331) + seed) % 7 < 3);
            let b_data = words(64, |i| (i.wrapping_mul(117) + seed) % 13 < 5);
            reset(&mut c, "scalar");
            let a = c.table(a_vars, a_data.clone());
            let b = c.table(b_vars, b_data.clone());
            reset(&mut c, "words");
            assert_eq!(c.table(a_vars, a_data), a);
            assert_eq!(c.table(b_vars, b_data), b);
            for op in 0..16 {
                reset(&mut c, "scalar");
                let expected = c.apply(op, a, b);
                let population = c.count(expected);
                for mode in ["words", "derived"] {
                    reset(&mut c, mode);
                    assert_eq!(c.apply(op, a, b), expected);
                    assert_eq!(c.count(expected), population);
                    assert_eq!(c.count(expected ^ 1), (1u64 << 60) - population);
                }
            }
            let mask = (a_vars | b_vars) & 0x5555_5555_5555_5555;
            reset(&mut c, "scalar");
            let expected = c.relprod(a, b, mask);
            for mode in ["words", "derived"] {
                reset(&mut c, mode);
                assert_eq!(c.relprod(a, b, mask), expected);
            }
            let map: Vec<_> = (0..60).rev().collect();
            reset(&mut c, "scalar");
            let expected = c.rename(a, &map);
            for mode in ["words", "derived"] {
                reset(&mut c, mode);
                assert_eq!(c.rename(a, &map), expected);
            }
        }
    }
    #[test]
    fn matched_kernels_preserve_resident_ids() {
        matched::<6>();
        matched::<9>();
    }
}
