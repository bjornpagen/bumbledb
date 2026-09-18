//! Legal-domain workspaces and owner-bound binary relation roles.
//! Scope admission uses containment plus exact cardinality; role admission
//! uses scoped abstraction. Neither certificate is a probability statement.
use super::carrier::*;
use super::relational_core::Algebra;
use rustc_hash::FxHashSet;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug)]
pub enum Domain {
    Below(u64),
    Values(Vec<u64>),
}
impl Domain {
    fn cardinality(&self) -> u64 {
        match self {
            Self::Below(n) => *n,
            Self::Values(v) => v.len() as u64,
        }
    }
    pub fn contains(&self, value: u64) -> bool {
        match self {
            Self::Below(n) => value < *n,
            Self::Values(v) => v.binary_search(&value).is_ok(),
        }
    }
    fn minimum(&self) -> Option<u64> {
        match self {
            Self::Below(0) => None,
            Self::Below(_) => Some(0),
            Self::Values(v) => v.first().copied(),
        }
    }
    fn formula<C: RegionOps>(&self, c: &mut C, base: u32, width: u32) -> Id {
        match self {
            Self::Below(n) => below(c, base, width, *n),
            Self::Values(values) => {
                let mut out = c.empty();
                for &value in values {
                    let atom = equal_value(c, base, width, value);
                    out = c.op(14, out, atom);
                }
                out
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Domains {
    width: u32,
    environment_bits: u32,
    domains: Vec<Domain>,
    population: u64,
}
impl Domains {
    pub fn new(
        width: u32,
        environment_bits: u32,
        mut domains: Vec<Domain>,
    ) -> Result<Self, &'static str> {
        let dimensions = width
            .checked_mul(3)
            .and_then(|n| n.checked_add(environment_bits))
            .ok_or("coordinate overflow")?;
        if dimensions >= 63 || domains.len() as u64 != 1u64 << environment_bits {
            return Err("legal-domain presentation shape");
        }
        let size = 1u64 << width;
        let mut population = 0u128;
        for domain in &mut domains {
            match domain {
                Domain::Below(n) if *n > size => return Err("legal bound outside encoding"),
                Domain::Values(values) => {
                    values.sort_unstable();
                    if values.iter().any(|&v| v >= size) || values.windows(2).any(|p| p[0] == p[1])
                    {
                        return Err("duplicate or out-of-range legal code");
                    }
                }
                _ => {}
            }
            population += u128::from(domain.cardinality()).pow(3);
        }
        if population == 0 || population > u128::from(u64::MAX) {
            return Err("empty or uncountable lab support");
        }
        Ok(Self {
            width,
            environment_bits,
            domains,
            population: population as u64,
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn environment_bits(&self) -> u32 {
        self.environment_bits
    }
    pub fn dimensions(&self) -> u32 {
        3 * self.width + self.environment_bits
    }
    pub fn population(&self) -> u64 {
        self.population
    }
    pub fn domain(&self, environment: usize) -> &Domain {
        &self.domains[environment]
    }
    pub fn environments(&self) -> usize {
        self.domains.len()
    }
    pub fn contains(&self, world: u64) -> bool {
        if world >= 1u64 << self.dimensions() {
            return false;
        }
        let mask = (1u64 << self.width) - 1;
        let domain = &self.domains[(world >> (3 * self.width)) as usize];
        (0..3).all(|face| domain.contains((world >> (face * self.width)) & mask))
    }
    pub fn anchor(&self) -> u64 {
        let (environment, value) = self
            .domains
            .iter()
            .enumerate()
            .find_map(|(e, d)| d.minimum().map(|v| (e, v)))
            .unwrap();
        value
            | (value << self.width)
            | (value << (2 * self.width))
            | ((environment as u64) << (3 * self.width))
    }
    pub fn order(&self, layout: &str) -> Vec<u32> {
        let mut order: Vec<_> = (3 * self.width..self.dimensions()).rev().collect();
        order.extend(face_order(3 * self.width, 3, layout));
        order
    }
    fn face_formula<C: RegionOps>(&self, c: &mut C, face: u32) -> Id {
        let mut out = c.empty();
        for (environment, domain) in self.domains.iter().enumerate() {
            let env = equal_value(c, 3 * self.width, self.environment_bits, environment as u64);
            let values = domain.formula(c, face * self.width, self.width);
            let part = c.op(8, env, values);
            out = c.op(14, out, part);
        }
        out
    }
    pub fn formula<C: RegionOps>(&self, c: &mut C) -> Id {
        assert_eq!(c.dimensions(), self.dimensions());
        let mut out = c.full();
        for face in 0..3 {
            let legal = self.face_formula(c, face);
            out = c.op(8, out, legal);
        }
        out
    }
    pub fn verify<C: RegionOps>(&self, c: &mut C) -> Result<(), &'static str> {
        if c.dimensions() != self.dimensions() {
            return Err("legal workspace dimensions");
        }
        // Scope-relative Full alone is tautological. These predicates test
        // S ⊆ declared product, while the independent product cardinality
        // rules out missing combinations and symmetric coupled supports.
        if c.count(c.full()) != self.population {
            return Err("legal workspace population");
        }
        for face in 0..3 {
            if self.face_formula(c, face) != c.full() {
                return Err("world outside declared domain");
            }
        }
        Ok(())
    }
    pub fn bytes(&self) -> usize {
        self.domains.capacity() * std::mem::size_of::<Domain>()
            + self
                .domains
                .iter()
                .map(|d| match d {
                    Domain::Values(v) => v.capacity() * 8,
                    _ => 0,
                })
                .sum::<usize>()
    }
}

pub fn equal_value<C: RegionOps>(c: &mut C, base: u32, width: u32, value: u64) -> Id {
    assert!(base + width <= c.dimensions() && value < 1u64 << width);
    let mut out = c.full();
    for bit in 0..width {
        let mut literal = c.variable(base + bit);
        if value >> bit & 1 == 0 {
            literal = c.not(literal);
        }
        out = c.op(8, out, literal);
    }
    out
}
pub fn below<C: RegionOps>(c: &mut C, base: u32, width: u32, bound: u64) -> Id {
    assert!(base + width <= c.dimensions() && bound <= 1u64 << width);
    if bound == 1u64 << width {
        return c.full();
    }
    let mut out = c.empty();
    for bit in 0..width {
        let literal = c.variable(base + bit);
        let zero = c.not(literal);
        out = c.op(if bound >> bit & 1 == 0 { 8 } else { 14 }, zero, out);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relation {
    root: Id,
    owner: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    root: Id,
    owner: u64,
}
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
fn owner() -> u64 {
    NEXT_OWNER
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
        .expect("relation owner exhausted")
}

pub struct Checked<C: RegionOps> {
    core: Algebra<C>,
    owner: u64,
    domains: Domains,
    relations: FxHashSet<Id>,
    goals: FxHashSet<Id>,
}
impl<C: RegionOps + Clone> Clone for Checked<C> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            owner: owner(),
            domains: self.domains.clone(),
            relations: self.relations.clone(),
            goals: self.goals.clone(),
        }
    }
}
impl<C: RegionOps> Checked<C> {
    pub fn new(
        c: C,
        domains: &Domains,
        memo: bool,
        gates: &'static str,
    ) -> Result<Self, &'static str> {
        let core = Algebra::try_new_legal(c, domains, memo, "native", gates)?;
        Ok(Self {
            core,
            owner: owner(),
            domains: domains.clone(),
            relations: FxHashSet::default(),
            goals: FxHashSet::default(),
        })
    }
    fn stamp(&mut self, root: Id) -> Relation {
        self.relations.insert(root);
        Relation {
            root,
            owner: self.owner,
        }
    }
    pub fn admit(&mut self, root: Id) -> Result<Relation, &'static str> {
        if !self.relations.contains(&root) {
            let mask = ((1u64 << self.domains.width) - 1) << (2 * self.domains.width);
            if self.core.base.inner.exists(root, mask) != root {
                return Err("Event depends on scratch face");
            }
        }
        Ok(self.stamp(root))
    }
    pub fn goal(&mut self, root: Id) -> Result<Goal, &'static str> {
        if !self.goals.contains(&root) {
            let face = (1u64 << self.domains.width) - 1;
            if self
                .core
                .base
                .inner
                .exists(root, face | (face << (2 * self.domains.width)))
                != root
            {
                return Err("goal depends on an undeclared face");
            }
            self.goals.insert(root);
        }
        Ok(Goal {
            root,
            owner: self.owner,
        })
    }
    pub fn root(&self, r: Relation) -> Result<Id, &'static str> {
        if r.owner != self.owner {
            return Err("foreign relation owner");
        }
        Ok(r.root)
    }
    fn goal_root(&self, g: Goal) -> Result<Id, &'static str> {
        if g.owner != self.owner {
            return Err("foreign goal owner");
        }
        Ok(g.root)
    }
    pub fn empty(&mut self) -> Relation {
        self.stamp(self.core.base.inner.empty())
    }
    pub fn full(&mut self) -> Relation {
        self.stamp(self.core.base.inner.full())
    }
    pub fn boolean(&mut self, op: u8, a: Relation, b: Relation) -> Result<Relation, &'static str> {
        if op > 15 {
            return Err("Boolean operation outside table");
        }
        let a = self.root(a)?;
        let b = self.root(b)?;
        let out = self.core.base.op(op, a, b);
        Ok(self.stamp(out))
    }
    pub fn compose(&mut self, a: Relation, b: Relation) -> Result<Relation, &'static str> {
        let a = self.root(a)?;
        let b = self.root(b)?;
        let out = self.core.compose(a, b);
        Ok(self.stamp(out))
    }
    pub fn converse(&mut self, a: Relation) -> Result<Relation, &'static str> {
        let a = self.root(a)?;
        let out = self.core.rename(a, 0);
        Ok(self.stamp(out))
    }
    pub fn residual(&mut self, a: Relation, b: Relation) -> Result<Relation, &'static str> {
        let a = self.root(a)?;
        let b = self.root(b)?;
        let out = self.core.residual(a, b);
        Ok(self.stamp(out))
    }
    pub fn closure(&mut self, a: Relation) -> Result<(Relation, usize), &'static str> {
        let a = self.root(a)?;
        let (out, steps) = self.core.closure(a);
        Ok((self.stamp(out), steps))
    }
    pub fn modal(&mut self, relation: Relation, goal: Goal) -> Result<(Id, Id), &'static str> {
        Ok((self.may(relation, goal)?, self.must(relation, goal)?))
    }
    pub fn may(&mut self, relation: Relation, goal: Goal) -> Result<Id, &'static str> {
        let r = self.root(relation)?;
        let goal = self.goal_root(goal)?;
        let mask = ((1u64 << self.domains.width) - 1) << self.domains.width;
        Ok(self.core.product(r, goal, mask))
    }
    pub fn must(&mut self, relation: Relation, goal: Goal) -> Result<Id, &'static str> {
        let r = self.root(relation)?;
        let goal = self.goal_root(goal)?;
        let mask = ((1u64 << self.domains.width) - 1) << self.domains.width;
        let neg = self.core.base.inner.not(goal);
        let bad = self.core.product(r, neg, mask);
        let full = self.core.base.inner.full();
        let enabled = self.core.product(r, full, mask);
        Ok(self.core.base.op(4, enabled, bad))
    }
    pub fn union_goals(&mut self, a: Goal, b: Goal) -> Result<Goal, &'static str> {
        let a = self.goal_root(a)?;
        let b = self.goal_root(b)?;
        let out = self.core.base.op(14, a, b);
        self.goals.insert(out);
        Ok(Goal {
            root: out,
            owner: self.owner,
        })
    }
    pub fn carrier(&self) -> &C {
        &self.core.base.inner
    }
    pub fn strategy(&self) -> &'static str {
        self.core.product_strategy()
    }
    pub fn gates(&self) -> &'static str {
        self.core.support_strategy()
    }
    pub fn memo(&mut self, enabled: bool) {
        self.core.base.enabled = enabled;
    }
    pub fn bytes(&self) -> usize {
        self.core.bytes()
            + self.domains.bytes()
            + (self.relations.capacity() + self.goals.capacity()) * 9
    }
}
