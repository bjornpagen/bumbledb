//! Sealed univariate parameter scopes. Guard codes are logical cells of one
//! actual real parameter, not additional sampled coordinates.
use std::sync::Arc;

use crate::arena::{Operation, Ref};
use crate::{
    BoolOp4, Capacity, Control, Error, Event, ExactArithmetic, FunctionLimits, LawLimits,
    ParameterCell, ParameterCodecLimits, ParameterDomain, ParameterRegion, RealWitness, Result,
    Space,
};

mod expectation;
mod family;
mod measure;
mod refinement;
mod revision;
pub(crate) mod wire;
pub use expectation::ParameterExpectationObservation;
pub use family::{FamilyFunction, FamilyFunctionPiece};
pub use measure::{ParameterDensityPiece, ParameterProbabilityObservation};
pub use refinement::ParameterRefinement;
pub use revision::{
    ParameterConditioning, ParameterJeffrey, ParameterLikelihood, ParameterRestriction,
    ParameterRevisedSource,
};

#[derive(Debug, Clone, Copy)]
pub struct ParameterSourceLimits {
    pub parameters: ParameterCodecLimits,
    pub functions: FunctionLimits,
    pub laws: LawLimits,
    pub fibres: usize,
    pub steps: usize,
}
impl Default for ParameterSourceLimits {
    fn default() -> Self {
        Self {
            parameters: ParameterCodecLimits::default(),
            functions: FunctionLimits::default(),
            laws: LawLimits::default(),
            fibres: 65_536,
            steps: 1_000_000,
        }
    }
}

/// The truth of `region` selects this logical coordinate. The remaining
/// coordinates in the source are finite outcome bits.
#[derive(Debug, Clone)]
pub struct ParameterGuard {
    pub coordinate: u8,
    pub region: ParameterRegion,
}

/// A semantic world: one exact parameter assignment and a finite outcome code.
/// Guard bits are absent from `outcomes`; they are determined by `parameter`.
#[derive(Debug, Clone)]
pub struct ParameterWorld {
    pub parameter: RealWitness,
    pub outcomes: u64,
}

/// Cardinality of actual worlds, distinct from the finite logical presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldCardinality {
    Finite(u64),
    Continuum,
}

#[derive(Debug, Clone)]
pub(crate) struct Fibre {
    pub code: u64,
    pub region: ParameterRegion,
    pub canonical: Arc<[u8]>,
}
#[derive(Debug, Clone)]
pub(crate) struct Context {
    pub domain: ParameterDomain,
    pub domain_bytes: Arc<[u8]>,
    pub guards: Arc<[ParameterGuard]>,
    pub mask: u64,
    pub fibres: Arc<[Fibre]>,
    pub canonical: Arc<[u8]>,
    pub law: Option<Arc<measure::Law>>,
}

pub(super) struct Budget {
    limits: ParameterSourceLimits,
    steps: usize,
}
impl Budget {
    pub(super) fn new(limits: ParameterSourceLimits, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        Ok(Self { limits, steps: 0 })
    }
    pub(super) fn step(&mut self, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        if self.steps >= self.limits.steps {
            return Err(Error::Capacity(Capacity::ParameterSourceSteps));
        }
        self.steps += 1;
        Ok(())
    }
    fn extent(&mut self, n: usize, control: &dyn Control) -> Result<()> {
        self.step(control)?;
        if n > self.limits.fibres {
            return Err(Error::Capacity(Capacity::ParameterSourceCells));
        }
        Ok(())
    }
}

impl Space {
    /// Interpret declared coordinates as exact guards of one shared parameter.
    /// The input must be unmeasured and not already parameterized. Impossible
    /// guard codes are removed. Every admitted parameter must retain a legal
    /// outcome fibre; missing fibres refuse instead of shrinking the domain.
    /// # Errors
    /// Incompatible declarations, missing fibres, capacities or cancellation.
    pub fn with_parameters(
        &self,
        domain: ParameterDomain,
        guards: &[ParameterGuard],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        if self.0.parameter.is_some() || self.is_measured() {
            return Err(Error::ParameterBinding);
        }
        if guards.len() > usize::from(self.dimensions()) {
            return Err(Error::InvalidOrder);
        }
        let domain_bytes: Arc<[u8]> = domain.to_bytes(limits.parameters, work)?.into();
        let mut retained = Vec::new();
        retained.try_reserve_exact(guards.len())?;
        let mut mask = 0u64;
        for guard in guards {
            budget.step(control)?;
            if guard.coordinate >= self.dimensions() {
                return Err(Error::InvalidCoordinate(guard.coordinate));
            }
            let bit = 1u64 << guard.coordinate;
            if mask & bit != 0 {
                return Err(Error::InvalidOrder);
            }
            mask |= bit;
            let region = guard.region.apply(
                BoolOp4::AND,
                domain.region(),
                limits.parameters.region,
                work,
            )?;
            retained.push(ParameterGuard {
                coordinate: guard.coordinate,
                region,
            });
        }
        retained.sort_unstable_by_key(|g| g.coordinate);
        budget.extent(1, control)?;
        let mut parts = vec![(0u64, domain.region().clone())];
        for guard in &retained {
            let mut next = Vec::new();
            for (code, region) in parts {
                for yes in [false, true] {
                    budget.step(control)?;
                    let predicate = if yes {
                        guard.region.clone()
                    } else {
                        guard.region.complement()
                    };
                    let part =
                        region.apply(BoolOp4::AND, &predicate, limits.parameters.region, work)?;
                    if !part.is_empty() {
                        budget.extent(next.len() + 1, control)?;
                        next.try_reserve(1)?;
                        next.push((code | (u64::from(yes) << guard.coordinate), part));
                    }
                }
            }
            parts = next;
        }
        parts.sort_unstable_by_key(|(code, _)| *code);
        let mut fibres = Vec::new();
        fibres.try_reserve_exact(parts.len())?;
        let mut feasible = self.empty();
        for (code, region) in parts {
            budget.step(control)?;
            if count_root(self, self.0.support, mask, code, control)? == 0 {
                return Err(Error::EmptyParameterFibre);
            }
            feasible = feasible.apply(BoolOp4::OR, &cube(self, mask, code, control)?, control)?;
            let canonical = region.to_bytes(limits.parameters, work)?.into();
            fibres.push(Fibre {
                code,
                region,
                canonical,
            });
        }
        let context = Context {
            canonical: wire::encode_context(&domain, &retained, limits, work)?.into(),
            domain,
            domain_bytes,
            guards: retained.into(),
            mask,
            fibres: fibres.into(),
            law: None,
        };
        let restricted = self.restrict(&feasible, control)?;
        restricted.with_parameter_context(Arc::new(context), control)
    }

    #[must_use]
    pub fn parameter_domain(&self) -> Option<&ParameterDomain> {
        self.0.parameter.as_ref().map(|p| &p.domain)
    }
    #[must_use]
    pub fn parameter_guards(&self) -> Option<&[ParameterGuard]> {
        self.0.parameter.as_ref().map(|p| p.guards.as_ref())
    }
    #[must_use]
    pub fn guard_coordinates(&self) -> u64 {
        self.0.parameter.as_ref().map_or(0, |p| p.mask)
    }
    #[must_use]
    pub fn outcome_coordinates(&self) -> u64 {
        ((1u64 << self.dimensions()) - 1) & !self.guard_coordinates()
    }
    pub(crate) fn parameter_bytes(&self) -> Option<&[u8]> {
        self.0.parameter.as_ref().map(|p| p.canonical.as_ref())
    }

    /// Verify that a finite readout map also preserves the same real parameter.
    /// Each source parameter cell must equal one whole target cell. The source
    /// domain may be a subset; missing target cells are unreachable. Different
    /// partitions require explicit `ParameterRefinement` first.
    pub(crate) fn parameter_map(
        &self,
        target: &Self,
        readouts: &[Event],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        let control = work.control();
        let (a, b) = match (&self.0.parameter, &target.0.parameter) {
            (None, None) => return Ok(()),
            (Some(a), Some(b)) => (a, b),
            _ => return Err(Error::ParameterScopeMismatch),
        };
        let mut budget = Budget::new(limits, control)?;
        budget.extent(a.fibres.len(), control)?;
        budget.extent(b.fibres.len(), control)?;
        if a.domain.parameter() != b.domain.parameter() {
            return Err(Error::ParameterScopeMismatch);
        }
        if a.domain_bytes != b.domain_bytes
            && !a
                .domain
                .region()
                .included(b.domain.region(), limits.parameters.region, work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        let mut by_region = std::collections::HashMap::new();
        by_region.try_reserve(b.fibres.len())?;
        for fibre in b.fibres.iter() {
            budget.step(control)?;
            by_region.insert(fibre.canonical.as_ref(), fibre.code);
        }
        let mut matching = Vec::new();
        matching.try_reserve_exact(a.fibres.len())?;
        for fibre in a.fibres.iter() {
            budget.step(control)?;
            let target = by_region
                .get(fibre.canonical.as_ref())
                .ok_or(Error::ParameterRefinementRequired)?;
            matching.push((fibre.code, *target));
        }
        for guard in b.guards.iter() {
            let mut expected = self.empty();
            for &(source, target) in &matching {
                budget.step(control)?;
                if target & (1u64 << guard.coordinate) != 0 {
                    expected = expected.apply(
                        BoolOp4::OR,
                        &cube(self, a.mask, source, control)?,
                        control,
                    )?;
                }
            }
            if !expected.equivalent(&readouts[usize::from(guard.coordinate)])? {
                return Err(Error::ParameterGuardMismatch);
            }
        }
        Ok(())
    }
}

impl Context {
    pub(crate) fn without_law(&self) -> Arc<Self> {
        Arc::new(Self {
            law: None,
            ..self.clone()
        })
    }

    pub(crate) fn restricted(
        &self,
        space: &Space,
        support: Ref,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Arc<Self>> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        let mut fibres = Vec::new();
        budget.extent(self.fibres.len(), control)?;
        fibres.try_reserve_exact(self.fibres.len())?;
        let mut region = ParameterRegion::empty(self.domain.parameter());
        for fibre in self.fibres.iter() {
            budget.step(control)?;
            if count_root(space, support, self.mask, fibre.code, control)? != 0 {
                region =
                    region.apply(BoolOp4::OR, &fibre.region, limits.parameters.region, work)?;
                fibres.push(fibre.clone());
            }
        }
        let domain = ParameterDomain::new(region)?;
        Ok(Arc::new(Self {
            domain_bytes: domain.to_bytes(limits.parameters, work)?.into(),
            canonical: wire::encode_context(&domain, &self.guards, limits, work)?.into(),
            domain,
            fibres: fibres.into(),
            law: None,
            ..self.clone()
        }))
    }
}

impl Event {
    /// Decode with explicit parameter/guard/function-law budgets, sharing one
    /// arithmetic allowance across every embedded domain, guard and density.
    /// # Errors
    /// Malformed or noncanonical input, source violations, capacities or cancellation.
    pub fn from_bytes_with_parameter_limits(
        bytes: &[u8],
        order: Option<&[u8]>,
        events: crate::Limits,
        parameters: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        if bytes.get(..5) == Some(b"BEVT\x03") {
            wire::decode(bytes, order, events, parameters, work)
        } else {
            Self::from_bytes_with_arithmetic(bytes, order, events, parameters.laws, work)
        }
    }
    /// Actual world cardinality. An inhabited open parameter cell contributes a
    /// continuum; isolated algebraic points each contribute their outcome count.
    /// # Errors
    /// Count overflow, graph capacities or cancellation.
    pub fn world_cardinality(&self, control: &dyn Control) -> Result<WorldCardinality> {
        let space = self.space();
        let Some(context) = &space.0.parameter else {
            return self.atom_count(control).map(WorldCardinality::Finite);
        };
        let mut total = Some(0u64);
        let mut infinite = false;
        for fibre in context.fibres.iter() {
            control.checkpoint()?;
            let count = count_root(&space, self.root, context.mask, fibre.code, control)?;
            if count == 0 {
                continue;
            }
            let mut points = 0u64;
            for (cell, included) in fibre.region.cells() {
                control.checkpoint()?;
                if included {
                    match cell {
                        ParameterCell::Point(_) => points += 1,
                        ParameterCell::Open { .. } => infinite = true,
                    }
                }
            }
            total = total
                .and_then(|total| count.checked_mul(points).and_then(|n| total.checked_add(n)));
        }
        Ok(if infinite {
            WorldCardinality::Continuum
        } else {
            WorldCardinality::Finite(total.ok_or(Error::Capacity(Capacity::WorldCount))?)
        })
    }

    /// Test a real parameter and an outcome code, deriving every guard bit.
    /// # Errors
    /// Missing/illegal parameter, supplied guard bits, graph/solver limits or cancellation.
    pub fn contains_parameter(
        &self,
        world: &ParameterWorld,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<bool> {
        let space = self.space();
        let context = space.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        if world.outcomes & !space.outcome_coordinates() != 0 {
            return Err(Error::IllegalWorld(world.outcomes));
        }
        if !context
            .domain
            .region()
            .contains(&world.parameter, limits.parameters.region, work)?
        {
            return Err(Error::IllegalParameter);
        }
        let mut code = world.outcomes;
        for guard in context.guards.iter() {
            if guard
                .region
                .contains(&world.parameter, limits.parameters.region, work)?
            {
                code |= 1u64 << guard.coordinate;
            }
        }
        self.contains(code)
    }

    /// A semantic witness rather than merely a finite logical-cell code.
    /// # Errors
    /// Missing parameter, graph/solver limits or cancellation.
    pub fn parameter_witness(
        &self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ParameterWorld>> {
        let space = self.space();
        let context = space.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let Some(code) = self.witness(work.control())? else {
            return Ok(None);
        };
        let fibre = context
            .fibres
            .iter()
            .find(|f| f.code == code & context.mask)
            .ok_or(Error::FunctionInvariant)?;
        let parameter = fibre
            .region
            .witness(limits.parameters.region, work)?
            .ok_or(Error::FunctionInvariant)?;
        Ok(Some(ParameterWorld {
            parameter,
            outcomes: code & space.outcome_coordinates(),
        }))
    }
}

fn cube(space: &Space, mask: u64, code: u64, control: &dyn Control) -> Result<Event> {
    let mut out = space.full();
    for coordinate in 0..space.dimensions() {
        if mask & (1u64 << coordinate) != 0 {
            let bit = space.coordinate(coordinate, control)?;
            let bit = if code & (1u64 << coordinate) == 0 {
                bit.complement()
            } else {
                bit
            };
            out = out.apply(BoolOp4::AND, &bit, control)?;
        }
    }
    Ok(out)
}

pub(super) fn count_root(
    space: &Space,
    root: Ref,
    mask: u64,
    code: u64,
    control: &dyn Control,
) -> Result<u64> {
    let mut arena = space.0.lock()?;
    let mut op = Operation::new(&mut arena, control)?;
    let mut legal = op.apply(BoolOp4::AND, space.0.support, root)?;
    for coordinate in 0..space.dimensions() {
        if mask & (1u64 << coordinate) != 0 {
            legal = op.cofactor(legal, coordinate, code & (1u64 << coordinate) != 0)?;
        }
    }
    let absent = (op.arena.mask() & !mask) & !op.arena.variables(legal);
    let result = op.count(legal)? << absent.count_ones();
    control.checkpoint()?;
    Ok(result)
}
