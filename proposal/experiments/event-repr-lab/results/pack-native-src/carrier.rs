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
pub fn abstract_dense(b: &mut [u64], n: usize, mask: u64) {
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

// Retained carrier capacities, sampled outside query timers. Hash estimates
// include tuple payload and one control byte per usable entry, not all allocator
// overhead. External query memos and temporary traversal scratch are separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryStats {
    pub layout: &'static str,
    pub record_bytes: usize,
    pub table_bytes: usize,
    pub interner_bytes: usize,
    pub metadata_bytes: usize,
    pub cache_bytes: usize,
    pub records: usize,
    pub tables: usize,
    pub logical_words: usize,
}
impl MemoryStats {
    pub fn bytes(self) -> usize {
        self.record_bytes
            + self.table_bytes
            + self.interner_bytes
            + self.metadata_bytes
            + self.cache_bytes
    }
}
pub fn memory_json(stats: Option<MemoryStats>) -> String {
    let Some(s) = stats else {
        return "null".into();
    };
    format!(
        "{{\"layout\":\"{}\",\"record_bytes\":{},\"table_bytes\":{},\"interner_bytes\":{},\"metadata_bytes\":{},\"cache_bytes\":{},\"records\":{},\"tables\":{},\"logical_words\":{},\"total_bytes_est\":{}}}",
        s.layout,
        s.record_bytes,
        s.table_bytes,
        s.interner_bytes,
        s.metadata_bytes,
        s.cache_bytes,
        s.records,
        s.tables,
        s.logical_words,
        s.bytes()
    )
}

// Mutable region operations can borrow an arena without cloning or replacing it.
pub trait RegionOps: Sized {
    const NAME: &'static str;
    const BIT_COMPLEMENT: bool = true;
    const DIRECT_SIGNATURE: bool = false;
    const VIEW_PRODUCT: bool = false;

    // Read-only support-relative Venn occupancy, if the carrier has a native
    // traversal. None is a missing capability, not a negative relationship.
    fn direct_signature(&self, _a: Id, _b: Id) -> Option<u8> {
        None
    }

    fn dimensions(&self) -> u32;

    // Support-relative equality of named binary coordinates. Symbolic carriers
    // inherit a formula constructor; finite carriers can fill one table directly.
    fn diagonal(&mut self, pairs: &[(u32, u32)]) -> Result<Id, &'static str> {
        symbolic_diagonal(self, pairs)
    }

    fn variable(&mut self, coordinate: u32) -> Id {
        assert!(coordinate < self.dimensions() && self.dimensions() <= 20);
        self.import(&bits(1usize << self.dimensions(), |w| {
            w >> coordinate & 1 != 0
        }))
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> super::transfer::Packet {
        assert_eq!(names.len(), self.dimensions() as usize);
        assert!(names.len() <= 20, "RESOURCE_CAP: explicit transfer table");
        let coords: Vec<_> = (0..self.dimensions()).collect();
        let mut out =
            super::transfer::Builder::new(names.to_vec(), coords.iter().rev().copied().collect());
        let table = |out: &mut super::transfer::Builder, id| {
            let mut words = self.export(id);
            words.resize((1usize << names.len()).div_ceil(64), 0);
            out.table(coords.clone(), words)
        };
        let support = table(&mut out, self.full());
        let roots = roots.iter().map(|&r| table(&mut out, r)).collect();
        out.finish(support, roots)
    }

    fn import(&mut self, b: &[u64]) -> Id;
    // Exact table substitution. Local table axis i names coordinates[i].
    // None requests the generic cofactor importer; no coordinate is eliminated here.
    fn import_table(&mut self, coordinates: &[u32], words: &[u64]) -> Option<Id> {
        import_full_table(self, coordinates, words)
    }
    fn import_table_words(&mut self, coordinates: &[u32], words: &[u64]) -> Option<Id> {
        self.import_table(coordinates, words)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id;
    fn not(&mut self, a: Id) -> Id;
    fn empty(&self) -> Id;
    fn full(&self) -> Id;
    fn export(&self, a: Id) -> Vec<u64>;
    fn exists(&mut self, a: Id, mask: u64) -> Id;
    // Pull back through a bijective coordinate renaming, then intersect support.
    // Laws commuting with relative complement require support preservation.
    fn permute(&mut self, a: Id, map: &Permutation) -> Id;
    fn bytes(&self) -> usize;
    fn memory_stats(&self) -> Option<MemoryStats> {
        None
    }
    // Optional laboratory census, never called inside measured query phases.
    fn root_diagnostics(&self, _inputs: &[Id], _outputs: &[Id]) -> Option<String> {
        None
    }
    fn nodes(&self) -> usize;
    fn count(&self, a: Id) -> u64 {
        self.export(a).iter().map(|w| w.count_ones() as u64).sum()
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        plan.prepare_table(&(0..plan.group_of.len() as u32).collect::<Vec<_>>());
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum;
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let v = self.op(8, a, b);
        self.exists(v, mask)
    }
    // Optional fusion of the exact staged scoped operations:
    // permute A/B (each clips), relprod (clips), output permute (clips).
    // A carrier may require additional support capabilities. This operation
    // alone grants neither faithful transport nor full-product relation laws.
    // None is a missing capability, never an empty mathematical result.
    fn view_product(
        &mut self,
        _a: Id,
        _am: &Permutation,
        _b: Id,
        _bm: &Permutation,
        _mask: u64,
        _output: &Permutation,
    ) -> Option<Id> {
        None
    }
    // Exact staged product, available only after the carrier itself verifies
    // support preservation for the required maps. A caller cannot supply an
    // unchecked "independent" or "preserving" flag. None means no certificate.
    fn preserving_product(
        &mut self,
        _a: Id,
        _am: &Permutation,
        _b: Id,
        _bm: &Permutation,
        _mask: u64,
        _output: &Permutation,
    ) -> Option<Id> {
        None
    }
}

// Construction and benchmark snapshot capability are separate from query operations.
pub trait Carrier: RegionOps + Clone {
    fn new(support: &[u64], n: usize) -> Self;

    fn legal_space(domains: &super::legal::Domains, order: Vec<u32>) -> Result<Self, &'static str> {
        let dimensions = domains.dimensions();
        if dimensions > 20 {
            return Err("RESOURCE_CAP: explicit legal support constructor");
        }
        let n = 1usize << dimensions;
        Ok(Self::with_order(
            &bits(n, |w| domains.contains(w as u64)),
            n,
            order,
        ))
    }

    fn full_space(nbits: u32, order: Vec<u32>) -> Result<Self, &'static str> {
        if nbits > 20 {
            return Err("RESOURCE_CAP: explicit transfer proof space");
        }
        let n = 1usize << nbits;
        Ok(Self::with_order(&bits(n, |_| true), n, order))
    }

    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        validate_order(n.next_power_of_two().trailing_zeros(), &order);
        Self::new(support, n)
    }
}

impl<C: RegionOps> RegionOps for &mut C {
    const NAME: &'static str = C::NAME;
    const BIT_COMPLEMENT: bool = C::BIT_COMPLEMENT;
    const DIRECT_SIGNATURE: bool = C::DIRECT_SIGNATURE;
    const VIEW_PRODUCT: bool = C::VIEW_PRODUCT;
    fn direct_signature(&self, a: Id, b: Id) -> Option<u8> {
        (**self).direct_signature(a, b)
    }
    fn dimensions(&self) -> u32 {
        (**self).dimensions()
    }
    fn diagonal(&mut self, pairs: &[(u32, u32)]) -> Result<Id, &'static str> {
        (**self).diagonal(pairs)
    }
    fn variable(&mut self, coordinate: u32) -> Id {
        (**self).variable(coordinate)
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> super::transfer::Packet {
        (**self).packet(names, roots)
    }
    fn import(&mut self, words: &[u64]) -> Id {
        (**self).import(words)
    }
    fn import_table(&mut self, coordinates: &[u32], words: &[u64]) -> Option<Id> {
        (**self).import_table(coordinates, words)
    }
    fn import_table_words(&mut self, coordinates: &[u32], words: &[u64]) -> Option<Id> {
        (**self).import_table_words(coordinates, words)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        (**self).op(op, a, b)
    }
    fn not(&mut self, a: Id) -> Id {
        (**self).not(a)
    }
    fn empty(&self) -> Id {
        (**self).empty()
    }
    fn full(&self) -> Id {
        (**self).full()
    }
    fn export(&self, a: Id) -> Vec<u64> {
        (**self).export(a)
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        (**self).exists(a, mask)
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        (**self).permute(a, map)
    }
    fn bytes(&self) -> usize {
        (**self).bytes()
    }
    fn memory_stats(&self) -> Option<MemoryStats> {
        (**self).memory_stats()
    }
    fn nodes(&self) -> usize {
        (**self).nodes()
    }
    fn count(&self, a: Id) -> u64 {
        (**self).count(a)
    }
    fn prepare_spectrum(&self, plan: &mut super::observation::CountPlan) {
        (**self).prepare_spectrum(plan)
    }
    fn spectrum(
        &self,
        event: Id,
        plan: &super::observation::CountPlan,
    ) -> super::observation::Spectrum {
        (**self).spectrum(event, plan)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        (**self).relprod(a, b, mask)
    }
    fn view_product(
        &mut self,
        a: Id,
        am: &Permutation,
        b: Id,
        bm: &Permutation,
        mask: u64,
        output: &Permutation,
    ) -> Option<Id> {
        (**self).view_product(a, am, b, bm, mask, output)
    }
    fn preserving_product(
        &mut self,
        a: Id,
        am: &Permutation,
        b: Id,
        bm: &Permutation,
        mask: u64,
        output: &Permutation,
    ) -> Option<Id> {
        (**self).preserving_product(a, am, b, bm, mask, output)
    }
}

pub fn validate_diagonal(dimensions: u32, pairs: &[(u32, u32)]) -> Result<(), &'static str> {
    // Check all inputs before returning even a trivial diagonal.
    if pairs
        .iter()
        .any(|&(a, b)| a >= dimensions || b >= dimensions)
    {
        return Err("diagonal coordinate outside presentation");
    }
    Ok(())
}

pub fn symbolic_diagonal<C: RegionOps>(
    c: &mut C,
    pairs: &[(u32, u32)],
) -> Result<Id, &'static str> {
    validate_diagonal(c.dimensions(), pairs)?;
    let mut out = c.full();
    for &(a, b) in pairs {
        let a = c.variable(a);
        let b = c.variable(b);
        let eq = c.op(9, a, b);
        out = c.op(8, out, eq);
    }
    Ok(out)
}

pub fn table_diagonal<C: RegionOps>(c: &mut C, pairs: &[(u32, u32)]) -> Result<Id, &'static str> {
    validate_diagonal(c.dimensions(), pairs)?;
    if c.dimensions() > 20 {
        return Err("RESOURCE_CAP: enumerated diagonal");
    }
    Ok(c.import(&bits(1usize << c.dimensions(), |w| {
        pairs.iter().all(|&(a, b)| ((w >> a) ^ (w >> b)) & 1 == 0)
    })))
}

pub fn import_full_table<C: RegionOps>(
    c: &mut C,
    coordinates: &[u32],
    words: &[u64],
) -> Option<Id> {
    if coordinates.len() != c.dimensions() as usize || coordinates.len() > 20 {
        return None;
    }
    assert_eq!(words.len(), (1usize << coordinates.len()).div_ceil(64));
    if coordinates.iter().enumerate().all(|(i, &v)| i as u32 == v) {
        return Some(c.import(words));
    }
    let permutation = Permutation::new(coordinates.to_vec()).ok()?;
    let mut mapped = words.to_vec();
    permutation.dense(&mut mapped);
    Some(c.import(&mapped))
}

pub fn validate_order(nbits: u32, order: &[u32]) {
    assert!(nbits < 63 && order.len() == nbits as usize);
    let mut seen = 0u64;
    for &coord in order {
        assert!(coord < nbits && seen >> coord & 1 == 0);
        seen |= 1 << coord;
    }
    assert_eq!(seen, (1u64 << nbits) - 1);
}

pub fn face_order(nbits: u32, faces: u32, layout: &str) -> Vec<u32> {
    assert!(faces > 0 && nbits % faces == 0);
    let width = nbits / faces;
    let chunk = match layout {
        "face-major" => width.max(1),
        "bit-major" => 1,
        "pair-major" => 2,
        _ => panic!("unknown face layout {layout}"),
    };
    let mut order = vec![];
    for base in (0..width).step_by(chunk as usize).rev() {
        for face in (0..faces).rev() {
            for bit in (base..(base + chunk).min(width)).rev() {
                order.push(face * width + bit);
            }
        }
    }
    validate_order(nbits, &order);
    order
}

#[derive(Clone)]
pub struct Permutation {
    // Old coordinate i becomes coordinate destinations[i].
    pub destinations: Vec<u32>,
    swaps: Vec<(u32, u32)>,
}
impl Permutation {
    pub fn bytes(&self) -> usize {
        self.destinations.capacity() * 4 + self.swaps.capacity() * std::mem::size_of::<(u32, u32)>()
    }
    pub fn new(destinations: Vec<u32>) -> Result<Self, &'static str> {
        if destinations.len() >= 63 {
            return Err("coordinate limit");
        }
        let mut seen = 0u64;
        for &c in &destinations {
            if c as usize >= destinations.len() || seen >> c & 1 != 0 {
                return Err("not a coordinate permutation");
            }
            seen |= 1 << c;
        }
        let mut labels: Vec<_> = (0..destinations.len() as u32).collect();
        let mut swaps = vec![];
        for (old, &target) in destinations.iter().enumerate() {
            let from = labels.iter().position(|&v| v == old as u32).unwrap();
            if from != target as usize {
                swaps.push(((from as u32).min(target), (from as u32).max(target)));
                labels.swap(from, target as usize);
            }
        }
        Ok(Self {
            destinations,
            swaps,
        })
    }
    pub fn source_world(&self, destination: usize) -> usize {
        self.destinations
            .iter()
            .enumerate()
            .fold(0, |w, (old, &new)| w | (((destination >> new) & 1) << old))
    }
    pub fn dense(&self, words: &mut [u64]) {
        assert_eq!(
            words.len(),
            (1usize << self.destinations.len()).div_ceil(64)
        );
        for &(i, j) in &self.swaps {
            if j < 6 {
                let distance = (1 << j) - (1 << i);
                let mask = (0..64)
                    .filter(|p| p >> i & 1 != 0 && p >> j & 1 == 0)
                    .fold(0u64, |m, p| m | (1 << p));
                for w in words.iter_mut() {
                    let t = (*w ^ (*w >> distance)) & mask;
                    *w ^= t ^ (t << distance);
                }
            } else if i < 6 {
                let stride = 1usize << (j - 6);
                let shift = 1 << i;
                let mask = (0..64)
                    .filter(|p| p >> i & 1 == 0)
                    .fold(0u64, |m, p| m | (1 << p));
                for base in (0..words.len()).step_by(stride * 2) {
                    let (low, high) = words[base..base + 2 * stride].split_at_mut(stride);
                    for (a, b) in low.iter_mut().zip(high) {
                        let t = ((*a >> shift) ^ *b) & mask;
                        *a ^= t << shift;
                        *b ^= t;
                    }
                }
            } else {
                let a = 1usize << (i - 6);
                let b = 1usize << (j - 6);
                for p in 0..words.len() {
                    if p & a != 0 && p & b == 0 {
                        words.swap(p, p ^ a ^ b);
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Memo<C: RegionOps> {
    pub inner: C,
    pub enabled: bool,
    pub cache: FxHashMap<(u8, Id, Id), Id>,
    pub hits: u64,
    pub misses: u64,
}
impl<C: RegionOps> Memo<C> {
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

pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
