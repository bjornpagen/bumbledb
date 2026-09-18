//! Exact finite conditional-iid source experiment, separate from Event identity.
//! Coefficients count assignments by successes in each named parameter group.
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{One, Zero};
use rustc_hash::FxHashMap;

pub type Spectrum = Vec<u64>;

#[derive(Clone)]
pub struct TablePlan {
    // Per physical word: disjoint masks, grouped by global coefficient index.
    masks: Vec<Vec<(usize, u64)>>,
}
impl TablePlan {
    pub fn terms(&self, word: usize, selector: u64) -> impl Iterator<Item = (usize, u64)> + '_ {
        self.masks[word].iter().filter_map(move |&(bin, mask)| {
            let count = (selector & mask).count_ones() as u64;
            (count != 0).then_some((bin, count))
        })
    }
    pub fn contract(&self, plan: &CountPlan, words: &[u64], xor: Option<&[u64]>) -> Spectrum {
        let mut out = plan.zero();
        for (i, &word) in words.iter().enumerate() {
            let word = word ^ xor.map_or(0, |s| s[i]);
            for (bin, count) in self.terms(i, word) {
                out[bin] += count;
            }
        }
        out
    }
    fn bytes(&self) -> usize {
        self.masks.capacity() * 24 + self.masks.iter().map(|v| v.capacity() * 16).sum::<usize>()
    }
}

#[derive(Clone)]
pub struct CountPlan {
    pub group_of: Vec<usize>,
    pub degrees: Vec<usize>,
    pub strides: Vec<usize>,
    pub bins: usize,
    tables: FxHashMap<Vec<u32>, TablePlan>,
}
impl CountPlan {
    pub fn new(group_of: Vec<usize>) -> Self {
        assert!(!group_of.is_empty() && group_of.len() < 63);
        let groups = 1 + *group_of.iter().max().unwrap();
        let mut degrees = vec![0; groups];
        for &g in &group_of {
            degrees[g] += 1;
        }
        assert!(
            degrees.iter().all(|&d| d != 0),
            "parameter groups must be contiguous and nonempty"
        );
        let mut bins = 1usize;
        let strides = degrees
            .iter()
            .map(|&d| {
                let stride = bins;
                bins = bins.checked_mul(d + 1).expect("coefficient size overflow");
                assert!(bins <= 1_000_000, "RESOURCE_CAP: coefficient table");
                stride
            })
            .collect();
        Self {
            group_of,
            degrees,
            strides,
            bins,
            tables: FxHashMap::default(),
        }
    }
    pub fn zero(&self) -> Spectrum {
        vec![0; self.bins]
    }
    pub fn one(&self) -> Spectrum {
        let mut out = self.zero();
        out[0] = 1;
        out
    }
    pub fn index(&self, assignment: usize) -> usize {
        self.group_of
            .iter()
            .enumerate()
            .filter(|&(c, _)| assignment >> c & 1 != 0)
            .map(|(_, &g)| self.strides[g])
            .sum()
    }
    pub fn prepare_table(&mut self, coords: &[u32]) {
        if self.tables.contains_key(coords) {
            return;
        }
        assert!(
            coords.len() <= 20,
            "RESOURCE_CAP: explicit observation table"
        );
        let cells = 1usize << coords.len();
        let mut masks = Vec::with_capacity(cells.div_ceil(64));
        for base in (0..cells).step_by(64) {
            let mut word = FxHashMap::<usize, u64>::default();
            for local in base..(base + 64).min(cells) {
                let mut bin = 0;
                let mut legal = true;
                for (i, &coord) in coords.iter().enumerate() {
                    if local >> i & 1 != 0 {
                        if let Some(&g) = self.group_of.get(coord as usize) {
                            bin += self.strides[g];
                        } else {
                            // MDD padding is a fixed false coordinate, not a draw.
                            legal = false;
                        }
                    }
                }
                if legal {
                    *word.entry(bin).or_default() |= 1u64 << (local - base);
                }
            }
            let mut terms: Vec<_> = word.into_iter().collect();
            terms.sort_unstable_by_key(|&(bin, _)| bin);
            masks.push(terms);
        }
        self.tables.insert(coords.to_vec(), TablePlan { masks });
    }
    pub fn table(&self, coords: &[u32]) -> &TablePlan {
        self.tables
            .get(coords)
            .expect("observation plan was not prepared")
    }
    pub fn table_if_prepared(&self, coords: &[u32]) -> Option<&TablePlan> {
        self.tables.get(coords)
    }
    pub fn add_shifted(&self, out: &mut Spectrum, child: &Spectrum, shift: usize, count: u64) {
        // Caller combines disjoint coordinate blocks. Thus indices cannot carry
        // into another parameter axis or exceed the source's declared degrees.
        for (i, &v) in child.iter().enumerate() {
            if v != 0 {
                out[i + shift] += v * count;
            }
        }
    }
    pub fn smooth(&self, out: &mut Spectrum, coords: impl IntoIterator<Item = u32>) {
        for coord in coords {
            if let Some(&g) = self.group_of.get(coord as usize) {
                let stride = self.strides[g];
                for i in (0..self.bins - stride).rev() {
                    if (i / stride) % (self.degrees[g] + 1) < self.degrees[g] {
                        out[i + stride] += out[i];
                    }
                }
            }
        }
    }
    pub fn subtract(&self, mut total: Spectrum, part: &Spectrum) -> Spectrum {
        for (a, &b) in total.iter_mut().zip(part) {
            *a = a
                .checked_sub(b)
                .expect("region is not contained in support");
        }
        total
    }
    pub fn oracle(&self, bits: &[u64], n: usize) -> Spectrum {
        let mut out = self.zero();
        for w in 0..n {
            if bits[w / 64] >> (w % 64) & 1 != 0 {
                out[self.index(w)] += 1;
            }
        }
        out
    }
    pub fn bytes(&self) -> usize {
        self.group_of.capacity() * 8
            + self.degrees.capacity() * 8
            + self.strides.capacity() * 8
            + self.tables.capacity() * 56
            + self
                .tables
                .iter()
                .map(|(k, v)| k.capacity() * 4 + v.bytes())
                .sum::<usize>()
    }
}

// One common denominator lets contraction use exact integer multiply-adds.
// These are weights of the declared source law, never per-row Event weights.
#[derive(Clone)]
pub struct ExactLaw {
    pub numerators: Vec<BigUint>,
    pub denominator: BigUint,
}
fn rising(start: u32, n: usize) -> BigUint {
    (0..n).fold(BigUint::one(), |v, i| v * (start as u64 + i as u64))
}
impl ExactLaw {
    fn product(plan: &CountPlan, groups: Vec<(Vec<BigUint>, BigUint)>) -> Self {
        assert_eq!(groups.len(), plan.degrees.len());
        let denominator = groups
            .iter()
            .map(|(_, d)| d)
            .fold(BigUint::one(), |v, d| v * d);
        let numerators = (0..plan.bins)
            .map(|bin| {
                groups
                    .iter()
                    .enumerate()
                    .fold(BigUint::one(), |v, (g, (weights, _))| {
                        v * &weights[(bin / plan.strides[g]) % (plan.degrees[g] + 1)]
                    })
            })
            .collect();
        Self {
            numerators,
            denominator,
        }
    }
    pub fn beta(plan: &CountPlan, priors: &[(u32, u32)]) -> Self {
        assert_eq!(priors.len(), plan.degrees.len());
        Self::product(
            plan,
            priors
                .iter()
                .zip(&plan.degrees)
                .map(|(&(a, b), &n)| {
                    assert!(a != 0 && b != 0);
                    (
                        (0..=n).map(|k| rising(a, k) * rising(b, n - k)).collect(),
                        rising(a.checked_add(b).unwrap(), n),
                    )
                })
                .collect(),
        )
    }
    pub fn point(plan: &CountPlan, probabilities: &[(u32, u32)]) -> Self {
        assert_eq!(probabilities.len(), plan.degrees.len());
        Self::product(
            plan,
            probabilities
                .iter()
                .zip(&plan.degrees)
                .map(|(&(a, b), &n)| {
                    assert!(b != 0 && a <= b);
                    (
                        (0..=n)
                            .map(|k| {
                                BigUint::from(a).pow(k as u32)
                                    * BigUint::from(b - a).pow((n - k) as u32)
                            })
                            .collect(),
                        BigUint::from(b).pow(n as u32),
                    )
                })
                .collect(),
        )
    }
    pub fn numerator(&self, spectrum: &Spectrum) -> BigUint {
        assert_eq!(spectrum.len(), self.numerators.len());
        spectrum
            .iter()
            .zip(&self.numerators)
            .filter(|(c, _)| **c != 0)
            .fold(BigUint::zero(), |v, (&count, weight)| v + weight * count)
    }
    pub fn probability(&self, spectrum: &Spectrum) -> BigRational {
        BigRational::new(
            BigInt::from(self.numerator(spectrum)),
            BigInt::from(self.denominator.clone()),
        )
    }
    pub fn conditional(&self, numerator: &Spectrum, evidence: &Spectrum) -> Option<BigRational> {
        let d = self.numerator(evidence);
        if d.is_zero() {
            None
        } else {
            Some(BigRational::new(
                BigInt::from(self.numerator(numerator)),
                BigInt::from(d),
            ))
        }
    }
    pub fn bytes(&self) -> usize {
        self.numerators.capacity() * std::mem::size_of::<BigUint>()
            + self
                .numerators
                .iter()
                .map(|n| n.bits().div_ceil(8) as usize)
                .sum::<usize>()
            + self.denominator.bits().div_ceil(8) as usize
    }
}
