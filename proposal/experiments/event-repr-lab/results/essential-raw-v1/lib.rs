//! Raw Boolean normal form. This is not yet a complete scoped Event carrier.
//! The enum/interner layout is an initial correctness implementation; timings
//! must include eventual scoped operations, observation and native Free Join.
use rustc_hash::FxHashMap;

pub type Ref = u32;
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Body {
    Constant,
    Table(Vec<u64>),
    Branch { variable: u32, low: Ref, high: Ref },
}
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Node {
    variables: u64,
    body: Body,
}
#[derive(Clone)]
pub struct Arena<const K: u32> {
    dimensions: u32,
    rank: Vec<u32>,
    nodes: Vec<Node>,
    unique: FxHashMap<Node, Ref>,
    applications: FxHashMap<(u8, Ref, Ref), Ref>,
    cofactors: FxHashMap<(Ref, u32, bool), Ref>,
}
fn axes(mut mask: u64) -> Vec<u32> {
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
            nodes: vec![Node {
                variables: 0,
                body: Body::Constant,
            }],
            unique: FxHashMap::default(),
            applications: FxHashMap::default(),
            cofactors: FxHashMap::default(),
        }
    }
    pub fn variables(&self, r: Ref) -> u64 {
        self.nodes[(r / 2) as usize].variables
    }
    pub fn is_table(&self, r: Ref) -> bool {
        matches!(self.nodes[(r / 2) as usize].body, Body::Table(_))
    }
    fn top(&self, variables: u64) -> u32 {
        axes(variables)
            .into_iter()
            .min_by_key(|&v| self.rank[v as usize])
            .unwrap()
    }
    fn intern(&mut self, node: Node) -> Ref {
        if let Some(&r) = self.unique.get(&node) {
            return r;
        }
        assert!(
            self.nodes.len() < 2_000_000,
            "RESOURCE_CAP: essential raw nodes"
        );
        let r = (self.nodes.len() as Ref) * 2;
        self.nodes.push(node.clone());
        self.unique.insert(node, r);
        r
    }
    pub fn evaluate(&self, r: Ref, world: u64) -> bool {
        let node = &self.nodes[(r / 2) as usize];
        let value = match &node.body {
            Body::Constant => false,
            Body::Table(data) => {
                let i = axes(node.variables)
                    .iter()
                    .enumerate()
                    .fold(0, |i, (k, &v)| i | (((world >> v) & 1) as usize) << k);
                cell(data, i)
            }
            Body::Branch {
                variable,
                low,
                high,
            } => self.evaluate(
                if world >> variable & 1 == 0 {
                    *low
                } else {
                    *high
                },
                world,
            ),
        };
        value ^ (r & 1 != 0)
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
            let low = cofactor_words(&data, count, position, false);
            let high = cofactor_words(&data, count, position, true);
            if low == high {
                variables &= !(1u64 << axes(variables)[position as usize]);
                data = low;
                count -= 1;
            } else {
                position += 1;
            }
        }
        if count == 0 {
            return data[0] as Ref;
        }
        if count <= K {
            let cells = 1usize << count;
            let flip = cell(&data, cells - 1) as Ref;
            if flip != 0 {
                for w in &mut data {
                    *w = !*w;
                }
                if cells < 64 {
                    data[0] &= (1u64 << cells) - 1;
                }
            }
            return self.intern(Node {
                variables,
                body: Body::Table(data),
            }) ^ flip;
        }
        let v = self.top(variables);
        let pos = axes(variables).iter().position(|&a| a == v).unwrap() as u32;
        let rest = variables & !(1 << v);
        let low = self.table(rest, cofactor_words(&data, count, pos, false));
        let high = self.table(rest, cofactor_words(&data, count, pos, true));
        self.branch(v, low, high)
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
            let data = words(1usize << names.len(), |i| {
                let w = scatter(i, &names);
                self.evaluate(if w >> variable & 1 != 0 { high } else { low }, w)
            });
            return self.table(variables, data);
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
        self.table(1 << coordinate, vec![2])
    }
    pub fn cofactor(&mut self, r: Ref, variable: u32, high: bool) -> Ref {
        assert!(variable < self.dimensions);
        if self.variables(r) >> variable & 1 == 0 {
            return r;
        }
        if let Some(&out) = self.cofactors.get(&(r, variable, high)) {
            return out;
        }
        let node = self.nodes[(r / 2) as usize].clone();
        let out = match node.body {
            Body::Constant => unreachable!(),
            Body::Table(data) => {
                let position = axes(node.variables)
                    .iter()
                    .position(|&v| v == variable)
                    .unwrap() as u32;
                self.table(
                    node.variables & !(1 << variable),
                    cofactor_words(&data, node.variables.count_ones(), position, high),
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
    pub fn apply(&mut self, op: u8, a: Ref, b: Ref) -> Ref {
        assert!(op < 16);
        if let Some(&out) = self.applications.get(&(op, a, b)) {
            return out;
        }
        let variables = self.variables(a) | self.variables(b);
        let out = if variables.count_ones() <= K {
            let names = axes(variables);
            let data = words(1usize << names.len(), |i| {
                let w = scatter(i, &names);
                let cell = 2 * self.evaluate(a, w) as u8 + self.evaluate(b, w) as u8;
                op >> cell & 1 != 0
            });
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
    pub fn exists(&mut self, mut r: Ref, variables: u64) -> Ref {
        assert!(variables >> self.dimensions == 0);
        for v in axes(variables & self.variables(r)) {
            let low = self.cofactor(r, v, false);
            let high = self.cofactor(r, v, true);
            r = self.apply(14, low, high);
        }
        r
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
    fn exhaustive<const K: u32>() {
        for order in [vec![0, 1, 2, 3], vec![3, 2, 1, 0]] {
            let mut c = Arena::<K>::new(4, &order);
            let literals: Vec<_> = (0..4).map(|v| c.variable(v)).collect();
            for word in 0..65536u64 {
                let r = c.import(vec![word]);
                assert_eq!(c.export(r), vec![word]);
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
    }
}
