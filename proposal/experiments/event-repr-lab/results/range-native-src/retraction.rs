//! Fixed-decoder completion over the same raw arena as Essential.
//! Legal product owners repair each face independently. Arbitrary supports
//! retain a global-anchor decoder and exact clipped scoped operations.
use super::carrier::*;
use super::essential_raw::{Arena, Ref, View, axes};
use super::legal::{Domains, equal_value};
use super::observation::{CountPlan, Spectrum};
use super::transfer::{Builder, Packet};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Normalization {
    Staged,
    Equal,
    Ite,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectionGates {
    Joint,
    Active,
    Witness,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectionCompletion {
    Full,
    Needed,
    Local,
    LocalNeeded,
}
impl ProjectionCompletion {
    fn parse(name: &str) -> Self {
        match name {
            "full" => Self::Full,
            "needed" => Self::Needed,
            "local" => Self::Local,
            "local-needed" => Self::LocalNeeded,
            _ => panic!("unknown projection completion policy"),
        }
    }
    fn local(self) -> bool {
        matches!(self, Self::Local | Self::LocalNeeded)
    }
    fn needed(self) -> bool {
        matches!(self, Self::Needed | Self::LocalNeeded)
    }
}

#[derive(Clone)]
struct CountEnvironment {
    // Original environment selector AND the selected legal state domains.
    // These are raw support predicates, never published Event roots.
    supports: [Ref; 8],
    population: u64,
}

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
    partial_normalization: FxHashMap<(Ref, u64), Ref>,
    projected: FxHashMap<(Ref, Ref, u64, u64), Ref>,
    commuting: FxHashMap<Vec<u32>, bool>,
    classifier: super::occupancy::Kernel,
    word_counts: bool,
    factor_counts: bool,
    normalize_mode: Normalization,
    projection_gates: ProjectionGates,
    projection_completion: ProjectionCompletion,
    fused_projection: bool,
    partial_supports: [Option<Ref>; 8],
    count_environments: Vec<CountEnvironment>,
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
            partial_normalization: FxHashMap::default(),
            projected: FxHashMap::default(),
            commuting: FxHashMap::default(),
            classifier: super::occupancy::Kernel::from_env(),
            count_environments: Vec::new(),
            partial_supports: [None; 8],
            fused_projection: match std::env::var("EVENT_LAB_RETRACTION_EXECUTE").as_deref() {
                Ok("fused") => true,
                Ok("staged") | Err(_) => false,
                _ => panic!("unknown projection execution policy"),
            },
            projection_completion: ProjectionCompletion::parse(
                &std::env::var("EVENT_LAB_RETRACTION_COMPLETE").unwrap_or_else(|_| "full".into()),
            ),
            projection_gates: match std::env::var("EVENT_LAB_RETRACTION_PROJECT").as_deref() {
                Ok("active") => ProjectionGates::Active,
                Ok("witness") => ProjectionGates::Witness,
                Ok("joint") | Err(_) => ProjectionGates::Joint,
                _ => panic!("unknown projection gate policy"),
            },
            normalize_mode: match std::env::var("EVENT_LAB_RETRACTION_NORMALIZE").as_deref() {
                Ok("ite") => Normalization::Ite,
                Ok("equal") => Normalization::Equal,
                Ok("staged") | Err(_) => Normalization::Staged,
                _ => panic!("unknown retraction normalization policy"),
            },
            factor_counts: match std::env::var("EVENT_LAB_RETRACTION_FACTOR").as_deref() {
                Ok("faces") => true,
                Ok("joint") | Err(_) => false,
                _ => panic!("unknown retraction factor policy"),
            },
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
        self.partial_normalization.clear();
        self.projected.clear();
        self.commuting.clear();
        self.partial_supports = [None; 8];
    }
    fn normalize(&mut self, r: Ref) -> Ref {
        // Constructors/imports retain the same full, physically ordered path
        // in every projection policy. Input storage is a controlled invariant.
        self.substitute(r, self.nonidentity_axes(), false)
    }
    fn nonidentity_axes(&self) -> u64 {
        ((1u64 << self.dimensions()) - 1) & !self.identity_axes
    }
    fn substitute(&mut self, r: Ref, selected: u64, needed: bool) -> Ref {
        // Simultaneous Shannon substitution: inserted decoder functions are
        // inputs to ITE, never recursively decoded as part of this traversal.
        if self.raw.variables(r) & selected == 0 {
            return r;
        }
        let base = r & !1;
        let full = selected == self.nonidentity_axes();
        let cached = if full {
            self.normalization.get(&base)
        } else {
            self.partial_normalization.get(&(base, selected))
        };
        if let Some(&out) = cached {
            return out ^ (r & 1);
        }
        let vars = self.raw.variables(base);
        let split = if needed { vars & selected } else { vars };
        let v = *self.order.iter().find(|&&v| split >> v & 1 != 0).unwrap();
        let low = self.raw.cofactor(base, v, false);
        let high = self.raw.cofactor(base, v, true);
        let low = self.substitute(low, selected, needed);
        let high = self.substitute(high, selected, needed);
        let selector = if selected >> v & 1 != 0 {
            self.decoder[v as usize]
        } else {
            self.raw.variable(v)
        };
        let out = self.conditional(selector, high, low);
        if full {
            self.normalization.insert(base, out);
        } else {
            self.partial_normalization.insert((base, selected), out);
        }
        out ^ (r & 1)
    }
    fn conditional(&mut self, selector: Ref, high: Ref, low: Ref) -> Ref {
        if self.normalize_mode == Normalization::Ite {
            self.raw.ite(selector, high, low)
        } else if self.normalize_mode == Normalization::Equal && low == high {
            // Legal completion can identify formerly distinct cofactors.
            // ITE(selector, a, a) = a; do not intern its cancelling pieces.
            low
        } else {
            let l = self.raw.apply(4, low, selector);
            let h = self.raw.apply(8, high, selector);
            self.raw.apply(14, l, h)
        }
    }
    fn projection_decoder_axes(&self, hidden: u64) -> u64 {
        let mut selected = self.nonidentity_axes();
        if self.projection_completion.local()
            && self.projection_gates == ProjectionGates::Witness
            && let Some(width) = self.face_width
            && hidden >> (3 * width) == 0
            && !self.count_environments.is_empty()
        {
            // Every face with a hidden coordinate is conservatively repaired.
            // Other faces preserve completion. Resolve environment aliases too.
            let face = (1u64 << width) - 1;
            let mut affected = !((1u64 << (3 * width)) - 1);
            for f in 0..3 {
                let mask = face << (f * width);
                if hidden & mask != 0 {
                    affected |= mask;
                }
            }
            selected &= affected;
        }
        selected
    }
    fn complete_projection(&mut self, r: Ref, hidden: u64) -> Ref {
        let selected = self.projection_decoder_axes(hidden);
        self.substitute(r, selected, self.projection_completion.needed())
    }
    // Source abstraction followed by target substitution. Inserted target
    // selectors never enter this source recursion.
    fn project_pair(
        &mut self,
        mut a: Ref,
        mut b: Ref,
        hidden: u64,
        selected: u64,
        needed: bool,
    ) -> Ref {
        if a == 0 || b == 0 || a == (b ^ 1) {
            return 0;
        }
        if a == b {
            b = 1;
        }
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        let vars = self.raw.variables(a) | self.raw.variables(b);
        let eliminate = hidden & vars;
        let replace = selected & vars & !hidden;
        if eliminate == 0 {
            let joint = self.raw.apply(8, a, b);
            return self.substitute(joint, selected & !hidden, needed);
        }
        if replace == 0 {
            return self.raw.relprod(a, b, eliminate);
        }
        // Existential abstraction does not commute with complement. Keep both
        // signed operands and the active source/decoder masks in the key.
        let key = (a, b, eliminate, replace);
        if let Some(&out) = self.projected.get(&key) {
            return out;
        }
        let split = if needed { eliminate | replace } else { vars };
        let v = *self.order.iter().find(|&&v| split >> v & 1 != 0).unwrap();
        let al = self.raw.cofactor(a, v, false);
        let ah = self.raw.cofactor(a, v, true);
        let bl = self.raw.cofactor(b, v, false);
        let bh = self.raw.cofactor(b, v, true);
        let low = self.project_pair(al, bl, hidden, selected, needed);
        let high = self.project_pair(ah, bh, hidden, selected, needed);
        let out = if eliminate >> v & 1 != 0 {
            self.raw.apply(14, low, high)
        } else {
            let selector = if selected >> v & 1 != 0 {
                self.decoder[v as usize]
            } else {
                self.raw.variable(v)
            };
            self.conditional(selector, high, low)
        };
        self.projected.insert(key, out);
        out
    }
    pub(crate) fn set_projection_execution(&mut self, policy: &str) {
        self.fused_projection = match policy {
            "staged" => false,
            "fused" => true,
            _ => panic!("unknown projection execution policy"),
        };
        self.normalization.clear();
        self.partial_normalization.clear();
        self.projected.clear();
    }
    pub(crate) fn set_projection_completion(&mut self, policy: &str) {
        self.projection_completion = ProjectionCompletion::parse(policy);
        // Test policy changes exercise construction rather than a previous
        // policy's memoized answer. Benchmarks select one policy at creation.
        self.normalization.clear();
        self.partial_normalization.clear();
        self.projected.clear();
    }
    pub(crate) fn check_pair_transform(&mut self) -> usize {
        assert_eq!(self.dimensions(), 3);
        let n = 8usize;
        let second = 0xa6u64;
        let b = self.raw.import(vec![second]);
        let mut roots = Vec::new();
        let mut cases = 0;
        for needed in [false, true] {
            self.normalization.clear();
            self.partial_normalization.clear();
            self.projected.clear();
            let mut index = 0;
            for pattern in 0..256u64 {
                let a = self.raw.import(vec![pattern]);
                for hidden in 0..n as u64 {
                    for mask in 0..n as u64 {
                        let selected = mask & self.nonidentity_axes();
                        let out = self.project_pair(a, b, hidden, selected, needed);
                        if needed {
                            assert_eq!(out, roots[index]);
                        } else {
                            roots.push(out);
                        }
                        index += 1;
                        for w in 0..n as u64 {
                            let mapped =
                                self.decoder.iter().enumerate().fold(w, |value, (v, &r)| {
                                    if selected >> v & 1 == 0 {
                                        value
                                    } else if self.raw.evaluate(r, w) {
                                        value | (1u64 << v)
                                    } else {
                                        value & !(1u64 << v)
                                    }
                                });
                            let expected = (0..n as u64).any(|witness| {
                                let source = (mapped & !hidden) | (witness & hidden);
                                (pattern & second) >> source & 1 != 0
                            });
                            assert_eq!(
                                self.raw.evaluate(out, w),
                                expected,
                                "joint transform pattern {pattern} hidden {hidden} selected {selected} target {w}"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
        cases
    }
    pub(crate) fn check_partial_substitution(&mut self, words: &[u64]) -> usize {
        let input = self.raw.import(words.to_vec());
        let n = 1usize << self.dimensions();
        let mut cases = 0;
        let mut roots = Vec::new();
        for needed in [false, true] {
            self.normalization.clear();
            self.partial_normalization.clear();
            self.projected.clear();
            for mask in 0..n as u64 {
                let selected = mask & self.nonidentity_axes();
                let result = self.substitute(input, selected, needed);
                if needed {
                    assert_eq!(result, roots[mask as usize]);
                } else {
                    roots.push(result);
                }
                assert_eq!(self.substitute(input ^ 1, selected, needed), result ^ 1);
                for w in 0..n as u64 {
                    let decoded = self.decoder.iter().enumerate().fold(w, |value, (v, &r)| {
                        if selected >> v & 1 == 0 {
                            value
                        } else if self.raw.evaluate(r, w) {
                            value | (1u64 << v)
                        } else {
                            value & !(1u64 << v)
                        }
                    });
                    assert_eq!(
                        self.raw.evaluate(result, w),
                        self.raw.evaluate(input, decoded)
                    );
                    cases += 1;
                }
            }
        }
        cases
    }
    pub(crate) fn check_alternate_normalization(&self, words: &[u64], expected: Id) {
        for mode in [
            Normalization::Staged,
            Normalization::Equal,
            Normalization::Ite,
        ] {
            if mode == self.normalize_mode {
                continue;
            }
            let mut other = self.clone();
            other.normalize_mode = mode;
            other.normalization.clear();
            other.partial_normalization.clear();
            other.projected.clear();
            assert_eq!(
                other.import(words),
                expected,
                "normalization policy must preserve canonical identity"
            );
        }
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
    pub(crate) fn set_projection_gates(&mut self, policy: &str) {
        self.projection_gates = match policy {
            "joint" => ProjectionGates::Joint,
            "active" => ProjectionGates::Active,
            "witness" => ProjectionGates::Witness,
            _ => panic!("unknown projection gate policy"),
        };
    }
    // Only constructor-certified product domains qualify. An environment that
    // can change during projection invalidates this per-environment argument.
    fn projection_gate(&mut self, dependencies: u64, hidden: u64) -> Ref {
        assert_eq!(hidden >> self.dimensions(), 0);
        if self.projection_gates == ProjectionGates::Joint {
            return self.support;
        }
        let Some(width) = self.face_width else {
            return self.support;
        };
        if hidden >> (3 * width) != 0 || self.count_environments.is_empty() {
            return self.support;
        }
        let required = if self.projection_gates == ProjectionGates::Witness {
            dependencies & hidden
        } else {
            dependencies
        };
        let face_bits = (1u64 << width) - 1;
        let selected = (0..3).fold(0usize, |m, f| {
            m | if required & (face_bits << (f * width)) != 0 {
                1 << f
            } else {
                0
            }
        });
        if selected == 7 {
            return self.support;
        }
        if let Some(gate) = self.partial_supports[selected] {
            return gate;
        }
        let mut gate = 0;
        for environment in &self.count_environments {
            gate = self.raw.apply(14, gate, environment.supports[selected]);
        }
        self.partial_supports[selected] = Some(gate);
        gate
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
        self.raw.metadata_bytes()
            + (self.order.capacity() + self.decoder.capacity()) * 4
            + std::mem::size_of::<Vec<CountEnvironment>>()
            + self.count_environments.capacity() * std::mem::size_of::<CountEnvironment>()
            + std::mem::size_of_val(&self.partial_supports)
            + std::mem::size_of_val(&self.partial_normalization)
            + std::mem::size_of_val(&self.projected)
    }
    fn cache_bytes(&self) -> usize {
        self.raw.cache_bytes()
            + self.normalization.capacity() * 9
            + self.partial_normalization.capacity() * (std::mem::size_of::<((Ref, u64), Ref)>() + 1)
            + self.projected.capacity() * (std::mem::size_of::<((Ref, Ref, u64, u64), Ref)>() + 1)
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
    pub fn set_factor_counts(&mut self, enabled: bool) {
        self.factor_counts = enabled;
    }
    fn joint_count(&self, support: Ref, event: Ref) -> u64 {
        super::contraction::count(&self.raw, &self.order, support, event, self.word_counts)
    }
    fn observed_count(&self, event: Ref) -> u64 {
        // Full support already has exact structural counts in the raw arena.
        if !self.factor_counts || self.support == 1 || self.count_environments.is_empty() {
            return self.joint_count(self.support, event);
        }
        let width = self
            .face_width
            .expect("count factors require a legal product");
        let face_bits = (1u64 << width) - 1;
        let axes = self.raw.variables(event);
        let selected = (0..3).fold(0usize, |mask, face| {
            mask | if axes & (face_bits << (face * width)) != 0 {
                1 << face
            } else {
                0
            }
        });
        if selected == 7 {
            return self.joint_count(self.support, event);
        }
        let omitted = 3 - selected.count_ones();
        let shift = width * omitted;
        let free_raw = 1u64 << shift;
        let mut total = 0u128;
        for environment in &self.count_environments {
            let count = self.joint_count(environment.supports[selected], event);
            // Both operands ignore every omitted raw face. Divide that exact
            // raw-cube multiplicity out before inserting the legal fibre size.
            assert_eq!(count & (free_raw - 1), 0, "nonintegral raw fibre count");
            total += u128::from(count >> shift) * u128::from(environment.population).pow(omitted);
        }
        u64::try_from(total).expect("legal count exceeds admitted population width")
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
        assert_eq!(mask >> self.dimensions(), 0);
        if self.complete_fibres(mask) {
            return self.raw.exists(a as Ref, mask) as Id;
        }
        let dependencies = self.raw.variables(a as Ref);
        if self.projection_gates == ProjectionGates::Witness && dependencies & mask == 0 {
            // The physical FD is sufficient on arbitrary support; the current
            // legal output itself supplies the existential witness.
            return a;
        }
        let gate = self.projection_gate(dependencies, mask);
        let clipped = self.raw.apply(8, gate, a as Ref);
        if self.fused_projection {
            let selected = self.projection_decoder_axes(mask);
            return self.project_pair(
                clipped,
                1,
                mask,
                selected,
                self.projection_completion.needed(),
            ) as Id;
        }
        let r = self.raw.exists(clipped, mask);
        self.complete_projection(r, mask) as Id
    }
    fn relprod(&mut self, a: Id, b: Id, mask: u64) -> Id {
        if self.complete_fibres(mask) {
            return self.raw.relprod(a as Ref, b as Ref, mask) as Id;
        }
        // The union is a sufficient dependency mask for the joint predicate.
        // It deliberately makes no independence inference between operands.
        let dependencies = self.raw.variables(a as Ref) | self.raw.variables(b as Ref);
        let gate = self.projection_gate(dependencies, mask);
        let a = self.raw.apply(8, gate, a as Ref);
        if self.fused_projection {
            let selected = self.projection_decoder_axes(mask);
            return self.project_pair(
                a,
                b as Ref,
                mask,
                selected,
                self.projection_completion.needed(),
            ) as Id;
        }
        let r = self.raw.relprod(a, b as Ref, mask);
        self.complete_projection(r, mask) as Id
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
            _ => self.observed_count(a as Ref),
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
    fn root_diagnostics(&self, inputs: &[Id], outputs: &[Id]) -> Option<String> {
        let inputs: Vec<_> = inputs.iter().map(|&r| r as Ref).collect();
        let outputs: Vec<_> = outputs.iter().map(|&r| r as Ref).collect();
        let mut infrastructure = self.decoder.clone();
        infrastructure.push(self.support);
        for environment in &self.count_environments {
            infrastructure.extend(environment.supports);
        }
        infrastructure.extend(self.partial_supports.iter().flatten().copied());
        let mut live = infrastructure.clone();
        live.extend(&inputs);
        live.extend(&outputs);
        let census = |roots: &[Ref]| {
            let (records, tables, words) = self.raw.reachable(roots);
            format!("{{\"records\":{records},\"tables\":{tables},\"words\":{words}}}")
        };
        let masks: Vec<_> = outputs.iter().map(|&r| self.raw.variables(r)).collect();
        let sizes: Vec<_> = outputs
            .iter()
            .map(|&r| self.raw.reachable(&[r]).0)
            .collect();
        Some(format!(
            concat!(
                "{{\"arena_records\":{},\"arena_bytes\":{},",
                "\"inputs\":{},\"outputs\":{},\"infrastructure\":{},\"live_union\":{},",
                "\"output_masks\":{:?},\"output_records\":{:?}}}"
            ),
            self.raw.nodes(),
            self.bytes(),
            census(&inputs),
            census(&outputs),
            census(&infrastructure),
            census(&live),
            masks,
            sizes
        ))
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
        let original_selections = selections.clone();
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
            let mut domain_roots = [0; 3];
            for face in 0..3 {
                let base = face * width;
                let legal = d.domain(e).formula(&mut c, base, width) as Ref;
                domain_roots[face as usize] = legal;
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
            let mut supports = [0; 8];
            supports[0] = original_selections[e];
            for mask in 1usize..8 {
                let face = mask.trailing_zeros() as usize;
                supports[mask] = c
                    .raw
                    .apply(8, supports[mask & !(1 << face)], domain_roots[face]);
            }
            c.count_environments.push(CountEnvironment {
                supports,
                population: d.domain(e).cardinality(),
            });
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
