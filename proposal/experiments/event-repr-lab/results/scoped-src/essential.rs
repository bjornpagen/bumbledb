//! Scoped Event carrier over the essential-coordinate table normal form.
//! Physical loops are deliberately straightforward in this first matched build.
use super::carrier::*;
use super::essential_raw::{Arena, Ref, View, axes};
use super::observation::{CountPlan, Spectrum};
use super::transfer::{Builder, Packet};
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub struct Essential<const K: u32> {
    raw: Arena<K>,
    n: usize,
    order: Vec<u32>,
    support: Ref,
    anchor: u64,
    population: u64,
    classifier: super::occupancy::Kernel,
}
impl<const K: u32> Essential<K> {
    pub fn symbolic(dimensions: u32, order: Vec<u32>) -> Self {
        Self {
            raw: Arena::new(dimensions, &order),
            n: 1usize << dimensions,
            order,
            support: 1,
            anchor: 0,
            population: 1u64 << dimensions,
            classifier: super::occupancy::Kernel::from_env(),
        }
    }
    pub fn literal(&mut self, coordinate: u32) -> Id {
        let raw = self.raw.variable(coordinate);
        self.seal(raw)
    }
    fn seal(&mut self, raw: Ref) -> Id {
        let selected = self.raw.apply(8, self.support, raw);
        let flip = self.raw.evaluate(selected, self.anchor);
        let representative = if flip {
            self.raw.apply(4, self.support, selected)
        } else {
            selected
        };
        (representative as Id) << 1 | flip as Id
    }
    fn true_root(&mut self, event: Id) -> Ref {
        let root = (event >> 1) as Ref;
        if event & 1 == 0 {
            root
        } else {
            self.raw.apply(4, self.support, root)
        }
    }
    fn reorder_words(coordinates: &[u32], target: &[u32], words: &[u64]) -> Vec<u64> {
        let map = coordinates
            .iter()
            .map(|v| target.iter().position(|c| c == v).unwrap() as u32)
            .collect();
        let mut out = words.to_vec();
        Permutation::new(map).unwrap().dense(&mut out);
        out
    }
    fn transport_raw(&self, r: Ref, out: &mut Builder, memo: &mut FxHashMap<Ref, Ref>) -> Ref {
        if r < 2 {
            return r;
        }
        let base = r & !1;
        if let Some(&value) = memo.get(&base) {
            return value ^ (r & 1);
        }
        let value = match self.raw.view(base) {
            View::Constant(_) => unreachable!(),
            View::Table {
                variables,
                words,
                complemented,
            } => {
                assert!(!complemented);
                let coordinates = axes(variables);
                let target: Vec<_> = self
                    .order
                    .iter()
                    .rev()
                    .copied()
                    .filter(|v| variables >> v & 1 != 0)
                    .collect();
                let words = Self::reorder_words(&coordinates, &target, words);
                out.table(target, words)
            }
            View::Branch {
                variable,
                low,
                high,
            } => {
                let low = self.transport_raw(low, out, memo);
                let high = self.transport_raw(high, out, memo);
                out.split(vec![variable], vec![low, high])
            }
        };
        memo.insert(base, value);
        value ^ (r & 1)
    }
    // Coefficients count assignments over exactly this function's essential
    // coordinates. Missing coordinates are smoothed by the caller, not deleted
    // from their shared parameter groups or interpreted as independence.
    fn spectrum_local(
        &self,
        r: Ref,
        plan: &CountPlan,
        memo: &mut FxHashMap<Ref, Spectrum>,
    ) -> Spectrum {
        if let Some(value) = memo.get(&r) {
            return value.clone();
        }
        let value = match self.raw.view(r) {
            View::Constant(false) => plan.zero(),
            View::Constant(true) => plan.one(),
            View::Table {
                variables,
                words,
                complemented,
            } => {
                let coords = axes(variables);
                let cells = 1usize << coords.len();
                let masks: Vec<_> = (0..words.len())
                    .map(|i| {
                        if cells >= (i + 1) * 64 {
                            !0
                        } else {
                            (1u64 << (cells - i * 64)) - 1
                        }
                    })
                    .collect();
                if let Some(table) = plan.table_if_prepared(&coords) {
                    table.contract(plan, words, complemented.then_some(masks.as_slice()))
                } else {
                    // A projection after plan preparation can create a new
                    // subset of axes. Exact bounded fallback, no stale plan.
                    let mut value = plan.zero();
                    for i in 0..cells {
                        if at(words, i) ^ complemented {
                            let index = coords
                                .iter()
                                .enumerate()
                                .filter(|&(k, _)| i >> k & 1 != 0)
                                .map(|(_, &v)| plan.strides[plan.group_of[v as usize]])
                                .sum::<usize>();
                            value[index] += 1;
                        }
                    }
                    value
                }
            }
            View::Branch {
                variable,
                low,
                high,
            } => {
                let mut value = plan.zero();
                let vars = self.raw.variables(r) & !(1 << variable);
                for (bit, child) in [low, high].into_iter().enumerate() {
                    let mut part = self.spectrum_local(child, plan, memo);
                    plan.smooth(&mut part, axes(vars & !self.raw.variables(child)));
                    for (shift, count) in plan.table(&[variable]).terms(0, 1 << bit) {
                        plan.add_shifted(&mut value, &part, shift, count);
                    }
                }
                value
            }
        };
        memo.insert(r, value.clone());
        value
    }
    fn spectrum_full(
        &self,
        r: Ref,
        plan: &CountPlan,
        memo: &mut FxHashMap<Ref, Spectrum>,
    ) -> Spectrum {
        let mut value = self.spectrum_local(r, plan, memo);
        plan.smooth(
            &mut value,
            (0..self.dimensions()).filter(|v| self.raw.variables(r) >> v & 1 == 0),
        );
        value
    }
}
impl<const K: u32> RegionOps for Essential<K> {
    const VIEW_PRODUCT: bool = true;
    const DIRECT_SIGNATURE: bool = true;
    fn direct_signature(&self, a: Id, b: Id) -> Option<u8> {
        let raw = super::occupancy::classify_with(
            self.classifier,
            &self.raw,
            &self.order,
            self.support,
            (a >> 1) as Ref,
            (b >> 1) as Ref,
        )
        .signature;
        Some(super::signature::orient(
            raw,
            (2 * (a & 1) + (b & 1)) as u8,
            false,
        ))
    }
    const NAME: &'static str = match K {
        6 => "essential64",
        9 => "essential512",
        _ => "essential-small",
    };
    fn dimensions(&self) -> u32 {
        self.order.len() as u32
    }
    fn variable(&mut self, coordinate: u32) -> Id {
        self.literal(coordinate)
    }
    fn import(&mut self, input: &[u64]) -> Id {
        assert!(
            self.dimensions() <= 20,
            "RESOURCE_CAP: explicit essential import"
        );
        let mut data = input.to_vec();
        data.resize((1usize << self.dimensions()).div_ceil(64), 0);
        let raw = self.raw.import(data);
        self.seal(raw)
    }
    fn import_table(&mut self, coordinates: &[u32], input: &[u64]) -> Option<Id> {
        if coordinates.len() > 20 {
            return None;
        }
        let mut variables = 0u64;
        for &v in coordinates {
            if v >= self.dimensions() || variables >> v & 1 != 0 {
                return None;
            }
            variables |= 1 << v;
        }
        let words = Self::reorder_words(coordinates, &axes(variables), input);
        let raw = self.raw.table(variables, words);
        Some(self.seal(raw))
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        assert!(op < 16);
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
        // Zero-preserving truth table keeps the representative outside support
        // and at the legal anchor false, exactly as in the anchored baseline.
        let raw = self.raw.apply(code, (a >> 1) as Ref, (b >> 1) as Ref);
        (raw as Id) << 1 | flip as Id
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
        assert!(
            self.dimensions() <= 20,
            "RESOURCE_CAP: explicit essential export"
        );
        bits(self.n, |w| {
            self.raw.evaluate(self.support, w as u64)
                && (self.raw.evaluate((a >> 1) as Ref, w as u64) ^ (a & 1 != 0))
        })
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        let raw = self.true_root(a);
        let result = self.raw.exists(raw, mask);
        self.seal(result)
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let a = self.true_root(a);
        let b = self.true_root(b);
        let result = self.raw.relprod(a, b, mask);
        self.seal(result)
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        assert_eq!(map.destinations.len(), self.dimensions() as usize);
        let a = self.true_root(a);
        let result = self.raw.rename(a, &map.destinations);
        self.seal(result)
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
        let mut a = self.true_root(a);
        let b = self.true_root(b);
        // A scoped permutation clips to S. Move the common witness gate S
        // back through operand A's map, then restore the intermediate S after
        // output renaming. This preserves the *staged scoped operations* even
        // for non-invariant support; it does not certify faithful transport or
        // grant the higher-level full-product relation algebra.
        let output_support = if self.support == 1 {
            1
        } else {
            let dimensions = self.dimensions() as usize;
            for map in [am, bm, output] {
                assert_eq!(map.destinations.len(), dimensions);
                let mut seen = 0u64;
                for &v in &map.destinations {
                    assert!((v as usize) < dimensions && seen >> v & 1 == 0);
                    seen |= 1 << v;
                }
            }
            assert_eq!(mask >> dimensions, 0);
            let mut inverse = vec![0; dimensions];
            for (i, &v) in am.destinations.iter().enumerate() {
                inverse[v as usize] = i as u32;
            }
            let witness_gate = self.raw.rename(self.support, &inverse);
            a = self.raw.apply(8, a, witness_gate);
            self.raw.rename(self.support, &output.destinations)
        };
        let result = self.raw.mapped_product(
            a,
            &am.destinations,
            b,
            &bm.destinations,
            mask,
            &output.destinations,
        );
        let result = self.raw.apply(8, output_support, result);
        Some(self.seal(result))
    }
    fn count(&self, a: Id) -> u64 {
        let part = self.raw.count((a >> 1) as Ref);
        if a & 1 == 0 {
            part
        } else {
            self.population - part
        }
    }
    fn bytes(&self) -> usize {
        self.raw.bytes() + self.order.capacity() * 4
    }
    fn memory_stats(&self) -> Option<MemoryStats> {
        let s = self.raw.storage_statistics();
        Some(MemoryStats {
            layout: self.raw.layout(),
            record_bytes: s.record_bytes,
            table_bytes: s.table_bytes,
            interner_bytes: s.interner_bytes,
            metadata_bytes: self.raw.metadata_bytes() + self.order.capacity() * 4,
            cache_bytes: self.raw.cache_bytes(),
            records: s.records,
            tables: s.tables,
            logical_words: s.logical_words,
        })
    }
    fn nodes(&self) -> usize {
        self.raw.nodes()
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> Packet {
        assert_eq!(names.len(), self.order.len());
        let mut out = Builder::new(names.to_vec(), self.order.clone());
        let mut memo = FxHashMap::default();
        let support = self.transport_raw(self.support, &mut out, &mut memo);
        let roots = roots
            .iter()
            .map(|&r| self.transport_raw((r >> 1) as Ref, &mut out, &mut memo) ^ (r & 1) as Ref)
            .collect();
        out.finish(support, roots)
    }
    fn prepare_spectrum(&self, plan: &mut CountPlan) {
        assert_eq!(plan.group_of.len(), self.order.len());
        for v in 0..self.dimensions() {
            plan.prepare_table(&[v]);
        }
        for r in self.raw.node_references() {
            if let View::Table { variables, .. } = self.raw.view(r) {
                plan.prepare_table(&axes(variables));
            }
        }
    }
    fn spectrum(&self, event: Id, plan: &CountPlan) -> Spectrum {
        let mut memo = FxHashMap::default();
        let value = self.spectrum_full((event >> 1) as Ref, plan, &mut memo);
        if event & 1 == 0 {
            value
        } else {
            plan.subtract(self.spectrum_full(self.support, plan, &mut memo), &value)
        }
    }
}
impl<const K: u32> Carrier for Essential<K> {
    fn full_space(dimensions: u32, order: Vec<u32>) -> Result<Self, &'static str> {
        Ok(Self::symbolic(dimensions, order))
    }
    fn new(support: &[u64], n: usize) -> Self {
        Self::with_order(
            support,
            n,
            (0..n.next_power_of_two().trailing_zeros()).rev().collect(),
        )
    }
    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        let mut out = Self::symbolic(n.next_power_of_two().trailing_zeros(), order);
        assert_eq!(support.len(), n.div_ceil(64));
        let mut data = support.to_vec();
        if n % 64 != 0 {
            *data.last_mut().unwrap() &= (1u64 << (n % 64)) - 1;
        }
        out.population = data.iter().map(|w| w.count_ones() as u64).sum();
        out.anchor = data
            .iter()
            .enumerate()
            .find_map(|(i, &w)| (w != 0).then(|| (i * 64 + w.trailing_zeros() as usize) as u64))
            .expect("nonempty support");
        data.resize((1usize << out.dimensions()).div_ceil(64), 0);
        out.support = out.raw.import(data);
        out.n = n;
        out
    }
}
