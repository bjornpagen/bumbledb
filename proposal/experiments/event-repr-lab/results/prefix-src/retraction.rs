//! Fixed-decoder completion over the same raw arena as Essential.
//! Legal product owners repair each face independently. Arbitrary supports
//! retain a global-anchor decoder and exact clipped scoped operations.
use super::carrier::*;
use super::essential_raw::{Arena, Ref, View, axes};
use super::legal::{Domains, equal_value};
use super::observation::{CountPlan, Spectrum};
use super::transfer::{Builder, Packet};
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub struct Retraction<const K: u32, const PREFIX: bool = false> {
    raw: Arena<K>,
    order: Vec<u32>,
    n: usize,
    support: Ref,
    population: u64,
    decoder: Vec<Ref>,
    identity_axes: u64,
    face_width: Option<u32>,
    normalization: FxHashMap<Ref, Ref>,
    commuting: FxHashMap<Vec<u32>, bool>,
    classifier: super::occupancy::Kernel,
    word_counts: bool,
}
impl<const K: u32, const PREFIX: bool> Retraction<K, PREFIX> {
    pub fn symbolic(dimensions: u32, order: Vec<u32>) -> Self {
        let mut raw = Arena::new(dimensions, &order);
        let decoder = (0..dimensions).map(|v| raw.variable(v)).collect();
        Self {
            raw,
            order,
            n: 1usize << dimensions,
            support: 1,
            population: 1u64 << dimensions,
            decoder,
            identity_axes: (1u64 << dimensions) - 1,
            face_width: None,
            normalization: FxHashMap::default(),
            commuting: FxHashMap::default(),
            classifier: super::occupancy::Kernel::from_env(),
            word_counts: match std::env::var("EVENT_LAB_RETRACTION_COUNT").as_deref() {
                Ok("words") | Err(_) => true,
                Ok("scalar") => false,
                _ => panic!("unknown retraction count kernel"),
            },
        }
    }
    fn install(&mut self, decoder: Vec<Ref>) {
        assert_eq!(decoder.len(), self.order.len());
        self.identity_axes = decoder.iter().enumerate().fold(0, |m, (v, &r)| {
            m | if r == self.raw.variable(v as u32) {
                1u64 << v
            } else {
                0
            }
        });
        self.decoder = decoder;
        self.normalization.clear();
        self.commuting.clear();
    }
    fn normalize(&mut self, r: Ref) -> Ref {
        // Simultaneous Shannon substitution: inserted decoder functions are
        // inputs to ITE, never recursively decoded as part of this traversal.
        if self.raw.variables(r) & !self.identity_axes == 0 {
            return r;
        }
        let base = r & !1;
        if let Some(&out) = self.normalization.get(&base) {
            return out ^ (r & 1);
        }
        let vars = self.raw.variables(base);
        let v = *self.order.iter().find(|&&v| vars >> v & 1 != 0).unwrap();
        let low = self.raw.cofactor(base, v, false);
        let high = self.raw.cofactor(base, v, true);
        let low = self.normalize(low);
        let high = self.normalize(high);
        let selector = self.decoder[v as usize];
        let l = self.raw.apply(4, low, selector);
        let h = self.raw.apply(8, high, selector);
        let out = self.raw.apply(14, l, h);
        self.normalization.insert(base, out);
        out ^ (r & 1)
    }
    pub(crate) fn complete_fibres(&self, mask: u64) -> bool {
        assert_eq!(mask >> self.dimensions(), 0);
        if self.support == 1 || mask == 0 {
            return true;
        }
        let Some(width) = self.face_width else {
            return false;
        };
        if mask >> (3 * width) != 0 {
            return false;
        }
        let face = (1u64 << width) - 1;
        (0..3).all(|k| {
            let selected = (mask >> (k * width)) & face;
            if PREFIX {
                // The semantic decoder order is high-to-low, independently of
                // physical arena order. Only low-bit suffixes have this proof.
                selected & (selected + 1) == 0
            } else {
                selected == 0 || selected == face
            }
        })
    }
    fn commutes(&mut self, map: &Permutation) -> bool {
        validate_order(self.dimensions(), &map.destinations);
        if self.support == 1
            || map
                .destinations
                .iter()
                .enumerate()
                .all(|(v, &w)| v == w as usize)
        {
            return true;
        }
        if let Some(&out) = self.commuting.get(map.destinations.as_slice()) {
            return out;
        }
        // Check rho(f(w)) = f(rho(w)) coordinate by coordinate. This also
        // implies support invariance for the permutation; a weaker support
        // certificate would not justify operating on completed roots directly.
        let out = (0..self.dimensions() as usize).all(|v| {
            self.raw.rename(self.decoder[v], &map.destinations)
                == self.decoder[map.destinations[v] as usize]
        });
        self.commuting.insert(map.destinations.clone(), out);
        out
    }
    fn metadata_bytes(&self) -> usize {
        self.raw.metadata_bytes() + (self.order.capacity() + self.decoder.capacity()) * 4
    }
    fn cache_bytes(&self) -> usize {
        self.raw.cache_bytes()
            + self.normalization.capacity() * 9
            + self.commuting.capacity() * (std::mem::size_of::<(Vec<u32>, bool)>() + 1)
            + self
                .commuting
                .keys()
                .map(|m| m.capacity() * 4)
                .sum::<usize>()
    }
    pub fn physical_axes(&self, event: Id) -> u64 {
        self.raw.variables(event as Ref)
    }
    pub fn set_word_counts(&mut self, enabled: bool) {
        self.word_counts = enabled;
    }
    pub fn decode_world(&self, w: u64) -> u64 {
        self.decoder.iter().enumerate().fold(0, |out, (v, &r)| {
            out | ((self.raw.evaluate(r, w) as u64) << v)
        })
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
                let coords = axes(variables);
                let target: Vec<_> = self
                    .order
                    .iter()
                    .rev()
                    .copied()
                    .filter(|v| variables >> v & 1 != 0)
                    .collect();
                let mut words = words.to_vec();
                Permutation::new(
                    coords
                        .iter()
                        .map(|v| target.iter().position(|c| c == v).unwrap() as u32)
                        .collect(),
                )
                .unwrap()
                .dense(&mut words);
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
}
impl<const K: u32, const PREFIX: bool> RegionOps for Retraction<K, PREFIX> {
    const NAME: &'static str = match (K, PREFIX) {
        (6, false) => "retraction64",
        (9, false) => "retraction512",
        (_, false) => "retraction-small",
        (6, true) => "prefix64",
        (9, true) => "prefix512",
        (_, true) => "prefix-small",
    };
    const VIEW_PRODUCT: bool = true;
    const DIRECT_SIGNATURE: bool = true;
    fn dimensions(&self) -> u32 {
        self.order.len() as u32
    }
    fn variable(&mut self, coordinate: u32) -> Id {
        self.decoder[coordinate as usize] as Id
    }
    fn import(&mut self, input: &[u64]) -> Id {
        assert!(
            self.dimensions() <= 20,
            "RESOURCE_CAP: explicit retraction import"
        );
        let mut words = input.to_vec();
        words.resize((1usize << self.dimensions()).div_ceil(64), 0);
        let r = self.raw.import(words);
        self.normalize(r) as Id
    }
    fn import_table(&mut self, coords: &[u32], input: &[u64]) -> Option<Id> {
        if coords.len() > 20 || input.len() != (1usize << coords.len()).div_ceil(64) {
            return None;
        }
        let mut vars = 0;
        for &v in coords {
            if v >= self.dimensions() || vars >> v & 1 != 0 {
                return None;
            }
            vars |= 1u64 << v;
        }
        let target = axes(vars);
        let mut words = input.to_vec();
        Permutation::new(
            coords
                .iter()
                .map(|v| target.iter().position(|c| c == v).unwrap() as u32)
                .collect(),
        )
        .unwrap()
        .dense(&mut words);
        let r = self.raw.table(vars, words);
        Some(self.normalize(r) as Id)
    }
    fn op(&mut self, op: u8, a: Id, b: Id) -> Id {
        assert!(op < 16);
        self.raw.apply(op, a as Ref, b as Ref) as Id
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
            "RESOURCE_CAP: explicit retraction export"
        );
        bits(self.n, |w| {
            self.raw.evaluate(self.support, w as u64) && self.raw.evaluate(a as Ref, w as u64)
        })
    }
    fn exists(&mut self, a: Id, mask: u64) -> Id {
        if self.complete_fibres(mask) {
            return self.raw.exists(a as Ref, mask) as Id;
        }
        let clipped = self.raw.apply(8, self.support, a as Ref);
        let r = self.raw.exists(clipped, mask);
        self.normalize(r) as Id
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        if self.complete_fibres(mask) {
            return self.raw.relprod(a as Ref, b as Ref, mask) as Id;
        }
        let a = self.raw.apply(8, self.support, a as Ref);
        let r = self.raw.relprod(a, b as Ref, mask);
        self.normalize(r) as Id
    }
    fn permute(&mut self, a: Id, map: &Permutation) -> Id {
        if self.commutes(map) {
            return self.raw.rename(a as Ref, &map.destinations) as Id;
        }
        let clipped = self.raw.apply(8, self.support, a as Ref);
        let r = self.raw.rename(clipped, &map.destinations);
        self.normalize(r) as Id
    }
    fn view_product(
        &mut self,
        a: Id,
        am: &Permutation,
        b: Id,
        bm: &Permutation,
        mask: u64,
        out: &Permutation,
    ) -> Option<Id> {
        if let Some(r) = self.preserving_product(a, am, b, bm, mask, out) {
            return Some(r);
        }
        let a = self.permute(a, am);
        let b = self.permute(b, bm);
        let r = self.relprod(a, b, mask);
        Some(self.permute(r, out))
    }
    fn preserving_product(
        &mut self,
        a: Id,
        am: &Permutation,
        b: Id,
        bm: &Permutation,
        mask: u64,
        out: &Permutation,
    ) -> Option<Id> {
        for map in [am, bm, out] {
            validate_order(self.dimensions(), &map.destinations);
        }
        if self.complete_fibres(mask)
            && self.commutes(am)
            && self.commutes(bm)
            && self.commutes(out)
        {
            return Some(self.raw.mapped_product(
                a as Ref,
                &am.destinations,
                b as Ref,
                &bm.destinations,
                mask,
                &out.destinations,
            ) as Id);
        }
        // Preserve the existing capability contract for arbitrary scoped owners.
        // Support preservation can admit the exact staged fallback; it cannot
        // admit the direct completed-root kernel without the stronger checks.
        if [am, out]
            .iter()
            .any(|m| self.raw.rename(self.support, &m.destinations) != self.support)
        {
            return None;
        }
        let a = self.permute(a, am);
        let b = self.permute(b, bm);
        let r = self.relprod(a, b, mask);
        Some(self.permute(r, out))
    }
    fn count(&self, a: Id) -> u64 {
        match a {
            0 => 0,
            1 => self.population,
            _ => super::contraction::count(
                &self.raw,
                &self.order,
                self.support,
                a as Ref,
                self.word_counts,
            ),
        }
    }
    fn direct_signature(&self, a: Id, b: Id) -> Option<u8> {
        // Surjectivity reflects every occupancy cell. No support gate is needed
        // here, but this theorem does not extend to assignment multiplicities.
        Some(
            super::occupancy::classify_with(
                self.classifier,
                &self.raw,
                &self.order,
                1,
                a as Ref,
                b as Ref,
            )
            .signature,
        )
    }
    fn bytes(&self) -> usize {
        self.raw.storage_statistics().bytes() + self.metadata_bytes() + self.cache_bytes()
    }
    fn nodes(&self) -> usize {
        self.raw.nodes()
    }
    fn memory_stats(&self) -> Option<MemoryStats> {
        let s = self.raw.storage_statistics();
        Some(MemoryStats {
            layout: self.raw.layout(),
            record_bytes: s.record_bytes,
            table_bytes: s.table_bytes,
            interner_bytes: s.interner_bytes,
            metadata_bytes: self.metadata_bytes(),
            cache_bytes: self.cache_bytes(),
            records: s.records,
            tables: s.tables,
            logical_words: s.logical_words,
        })
    }
    fn packet(&self, names: &[u64], roots: &[Id]) -> Packet {
        assert_eq!(names.len(), self.order.len());
        let mut out = Builder::new(names.to_vec(), self.order.clone());
        let mut memo = FxHashMap::default();
        let support = self.transport_raw(self.support, &mut out, &mut memo);
        let roots = roots
            .iter()
            .map(|&r| self.transport_raw(r as Ref, &mut out, &mut memo))
            .collect();
        out.finish(support, roots)
    }
    fn prepare_spectrum(&self, plan: &mut CountPlan) {
        assert_eq!(plan.group_of.len(), self.order.len());
    }
    fn spectrum(&self, event: Id, plan: &CountPlan) -> Spectrum {
        super::contraction::spectrum(&self.raw, &self.order, self.support, event as Ref, plan)
    }
}
impl<const K: u32, const PREFIX: bool> Carrier for Retraction<K, PREFIX> {
    fn full_space(dimensions: u32, order: Vec<u32>) -> Result<Self, &'static str> {
        Ok(Self::symbolic(dimensions, order))
    }
    fn legal_space(d: &Domains, order: Vec<u32>) -> Result<Self, &'static str> {
        let mut c = Self::symbolic(d.dimensions(), order);
        c.support = d.formula(&mut c) as Ref;
        c.population = d.population();
        assert_eq!(c.raw.count(c.support), c.population);
        let width = d.width();
        let env_base = 3 * width;
        let anchor_env = (d.anchor() >> env_base) as usize;
        let mut valid_env = 0;
        let mut selections = vec![0; d.environments()];
        for (e, selection) in selections.iter_mut().enumerate() {
            if d.domain(e).minimum().is_some() {
                *selection = equal_value(&mut c, env_base, d.environment_bits(), e as u64) as Ref;
                valid_env = c.raw.apply(14, valid_env, *selection);
            }
        }
        selections[anchor_env] = c.raw.apply(14, selections[anchor_env], valid_env ^ 1);
        let mut decoder = vec![0; d.dimensions() as usize];
        for (e, &selection) in selections.iter().enumerate() {
            let Some(anchor) = d.domain(e).minimum() else {
                continue;
            };
            for bit in 0..d.environment_bits() {
                if e >> bit & 1 != 0 {
                    let v = (env_base + bit) as usize;
                    decoder[v] = c.raw.apply(14, decoder[v], selection);
                }
            }
            for face in 0..3 {
                let base = face * width;
                let legal = d.domain(e).formula(&mut c, base, width) as Ref;
                let mut residual = legal;
                for step in 0..width {
                    let bit = if PREFIX { width - 1 - step } else { step };
                    let v = (base + bit) as usize;
                    let raw = c.raw.variable(v as u32);
                    let repaired = if PREFIX {
                        // Feasibility of the current raw bit after repairing
                        // higher bits. An impossible choice must flip; the
                        // incoming repaired prefix always has a completion.
                        let lower = ((1u64 << bit) - 1) << base;
                        let keep = c.raw.exists(residual, lower);
                        let chosen = c.raw.apply(9, raw, keep);
                        // Simultaneous substitution of this chosen bit only.
                        let low = c.raw.cofactor(residual, v as u32, false);
                        let high = c.raw.cofactor(residual, v as u32, true);
                        let l = c.raw.apply(4, low, chosen);
                        let h = c.raw.apply(8, high, chosen);
                        residual = c.raw.apply(14, l, h);
                        chosen
                    } else if anchor >> bit & 1 == 0 {
                        c.raw.apply(8, legal, raw)
                    } else {
                        c.raw.apply(14, legal ^ 1, raw)
                    };
                    let part = c.raw.apply(8, selection, repaired);
                    decoder[v] = c.raw.apply(14, decoder[v], part);
                }
                if PREFIX {
                    assert_eq!(residual, 1, "prefix decoder must land in its domain");
                }
            }
        }
        c.install(decoder);
        c.face_width = Some(width);
        Ok(c)
    }
    fn new(support: &[u64], n: usize) -> Self {
        Self::with_order(
            support,
            n,
            (0..n.next_power_of_two().trailing_zeros()).rev().collect(),
        )
    }
    fn with_order(support: &[u64], n: usize, order: Vec<u32>) -> Self {
        let mut c = Self::symbolic(n.next_power_of_two().trailing_zeros(), order);
        assert_eq!(support.len(), n.div_ceil(64));
        let mut words = support.to_vec();
        if n % 64 != 0 {
            *words.last_mut().unwrap() &= (1u64 << (n % 64)) - 1;
        }
        c.population = words.iter().map(|w| w.count_ones() as u64).sum();
        let anchor = words
            .iter()
            .enumerate()
            .find_map(|(i, &w)| (w != 0).then(|| (i * 64 + w.trailing_zeros() as usize) as u64))
            .expect("nonempty support");
        words.resize((1usize << c.dimensions()).div_ceil(64), 0);
        c.support = c.raw.import(words);
        c.n = n;
        let decoder = (0..c.dimensions())
            .map(|v| {
                let r = c.raw.variable(v);
                if anchor >> v & 1 == 0 {
                    c.raw.apply(8, c.support, r)
                } else {
                    c.raw.apply(14, c.support ^ 1, r)
                }
            })
            .collect();
        c.install(decoder);
        c
    }
}
